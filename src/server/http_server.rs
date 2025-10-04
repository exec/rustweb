use crate::config::Config;
use crate::server::request_handler::RequestHandler;
use crate::server::tls::TlsManager;
use crate::server::http2::Http2Handler;
use crate::server::http3::Http3Server;
use anyhow::{Context, Result};
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tracing::{error, info, warn};
use futures::future::join_all;
use signal_hook::consts::{SIGTERM, SIGINT, SIGQUIT};
use signal_hook_tokio::Signals;

pub struct HttpServer {
    config: Arc<Config>,
    handler: Arc<RequestHandler>,
    tls_manager: TlsManager,
}

impl HttpServer {
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let handler = Arc::new(RequestHandler::new(config.clone())?);
        
        // Initialize TLS for any virtual host that has SSL config
        let ssl_config = config.virtual_hosts.values()
            .find_map(|vhost| vhost.ssl.as_ref());
        let tls_manager = TlsManager::new(ssl_config)?;
        
        Ok(Self {
            config,
            handler,
            tls_manager,
        })
    }

    pub async fn run(self) -> Result<()> {
        let addresses = self.config.listen_addresses()?;
        
        if addresses.is_empty() {
            return Err(anyhow::anyhow!("No listen addresses configured"));
        }

        info!("Starting RustWeb server");

        // Start HTTP/3 server if enabled
        if self.config.server.enable_http3 {
            let http3_server = Http3Server::new(
                self.config.clone(),
                self.handler.clone(),
                Arc::new(self.tls_manager.clone()),
            );
            
            if let Err(e) = http3_server.start().await {
                error!("Failed to start HTTP/3 server: {}", e);
            }
        }

        let mut listeners = Vec::new();
        for addr in &addresses {
            let listener = TcpListener::bind(*addr).await
                .with_context(|| format!("Failed to bind to {}", addr))?;
            info!("Listening on {}", addr);
            listeners.push(listener);
        }

        let server_tasks = listeners.into_iter().enumerate().map(|(i, listener)| {
            let handler = self.handler.clone();
            let config = self.config.clone();
            let tls_manager = self.tls_manager.clone();
            let addr = addresses[i];
            
            tokio::spawn(async move {
                // Determine if this should be an HTTPS listener
                let is_https = addr.port() == 8443 || addr.port() == 443;
                Self::serve_listener(listener, handler, config, tls_manager, is_https).await
            })
        });

        tokio::select! {
            results = join_all(server_tasks) => {
                for result in results {
                    if let Err(e) = result {
                        error!("Server task failed: {}", e);
                    }
                }
            }
            _ = self.wait_for_signal() => {
                info!("Received shutdown signal, stopping server");
            }
        }

        info!("Server stopped");
        Ok(())
    }

    async fn serve_listener(
        listener: TcpListener,
        handler: Arc<RequestHandler>,
        config: Arc<Config>,
        tls_manager: TlsManager,
        is_https: bool,
    ) -> Result<()> {
        loop {
            let (stream, addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            let handler = handler.clone();
            let config = config.clone();
            let tls_manager = tls_manager.clone();

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, handler, config, tls_manager, is_https).await {
                    error!("Connection error from {}: {}", addr, e);
                }
            });
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        addr: std::net::SocketAddr,
        handler: Arc<RequestHandler>,
        config: Arc<Config>,
        tls_manager: TlsManager,
        is_https: bool,
    ) -> Result<()> {
        stream.set_nodelay(config.server.tcp_nodelay)?;
        
        if is_https {
            // Handle HTTPS connection with HTTP/2 support
            if let Some(acceptor) = tls_manager.get_acceptor() {
                let tls_stream = acceptor.accept(stream).await
                    .map_err(|e| anyhow::anyhow!("TLS handshake failed: {}", e))?;
                
                // Check ALPN negotiation to determine protocol
                let (_, session) = tls_stream.get_ref();
                let alpn_protocol = session.alpn_protocol();
                
                match alpn_protocol {
                    Some(b"h2") => {
                        // Use HTTP/2
                        info!("Negotiated HTTP/2 for connection from {}", addr);
                        let http2_handler = Http2Handler::new(handler);
                        http2_handler.handle_connection(tls_stream, addr).await?;
                    }
                    Some(b"http/1.1") | None => {
                        // Use HTTP/1.1
                        info!("Using HTTP/1.1 for connection from {}", addr);
                        let io = TokioIo::new(tls_stream);
                        let service = hyper::service::service_fn(move |req| {
                            let handler = handler.clone();
                            let addr = addr;
                            async move {
                                handler.handle_request(req, addr).await
                            }
                        });

                        http1::Builder::new()
                            .keep_alive(true)
                            .serve_connection(io, service)
                            .await
                            .map_err(|e| anyhow::anyhow!("HTTPS connection error: {}", e))?;
                    }
                    Some(protocol) => {
                        warn!("Unsupported ALPN protocol: {:?}", std::str::from_utf8(protocol));
                        return Err(anyhow::anyhow!("Unsupported ALPN protocol"));
                    }
                }
            } else {
                return Err(anyhow::anyhow!("HTTPS requested but no TLS acceptor configured"));
            }
        } else {
            // Handle HTTP connection
            let io = TokioIo::new(stream);
            let service = hyper::service::service_fn(move |req| {
                let handler = handler.clone();
                let addr = addr;
                async move {
                    handler.handle_request(req, addr).await
                }
            });

            http1::Builder::new()
                .keep_alive(true)
                .serve_connection(io, service)
                .await
                .map_err(|e| anyhow::anyhow!("HTTP connection error: {}", e))?;
        }

        Ok(())
    }

    async fn wait_for_signal(&self) {
        #[cfg(unix)]
        {
            use futures::stream::StreamExt;
            let mut signals = Signals::new(&[SIGTERM, SIGINT, SIGQUIT])
                .expect("Failed to register signal handlers");

            while let Some(signal) = signals.next().await {
                match signal {
                    SIGTERM | SIGINT | SIGQUIT => {
                        info!("Received signal {}, initiating graceful shutdown", signal);
                        break;
                    }
                    _ => {}
                }
            }
        }
        
        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
            info!("Received Ctrl-C, initiating graceful shutdown");
        }
    }
}