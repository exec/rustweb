#![allow(dead_code)]

use crate::server::request_handler::RequestHandler;
use anyhow::Result;
use bytes::Bytes;
use h2::server::{self, SendResponse};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tracing::{debug, error, info};

pub struct Http2Handler {
    request_handler: Arc<RequestHandler>,
}

impl Http2Handler {
    pub fn new(request_handler: Arc<RequestHandler>) -> Self {
        Self { request_handler }
    }

    pub async fn handle_connection(
        &self,
        stream: TlsStream<TcpStream>,
        client_addr: std::net::SocketAddr,
    ) -> Result<()> {
        let mut connection = server::handshake(stream).await?;

        info!("HTTP/2 connection established from {}", client_addr);

        while let Some(request) = connection.accept().await {
            let (request, mut respond) = request?;

            let handler = self.request_handler.clone();
            let addr = client_addr;

            tokio::spawn(async move {
                match Self::handle_h2_request(handler, request, addr).await {
                    Ok(response) => {
                        if let Err(e) = Self::send_h2_response(&mut respond, response).await {
                            error!("Failed to send HTTP/2 response: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("HTTP/2 request handling failed: {}", e);
                        let error_response = Response::builder()
                            .status(500)
                            .body(Full::new(Bytes::from("Internal Server Error")))
                            .unwrap();

                        let _ = Self::send_h2_response(&mut respond, error_response).await;
                    }
                }
            });
        }

        debug!("HTTP/2 connection closed for {}", client_addr);
        Ok(())
    }

    async fn handle_h2_request(
        handler: Arc<RequestHandler>,
        request: Request<h2::RecvStream>,
        client_addr: std::net::SocketAddr,
    ) -> Result<Response<Full<Bytes>>> {
        // Convert H2 request to hyper request
        let (parts, mut body) = request.into_parts();

        // Collect the body
        let mut body_bytes = Vec::new();
        while let Some(chunk) = body.data().await {
            let chunk = chunk?;
            body_bytes.extend_from_slice(&chunk);
            let _ = body.flow_control().release_capacity(chunk.len());
        }

        // For HTTP/2, we'll handle the request directly using the same logic as RequestHandler
        // but adapted for HTTP/2 specifics
        Self::handle_h2_request_direct(handler, &parts, &body_bytes, client_addr).await
    }

    async fn handle_h2_request_direct(
        _handler: Arc<RequestHandler>,
        parts: &http::request::Parts,
        _body_bytes: &[u8],
        _client_addr: std::net::SocketAddr,
    ) -> Result<Response<Full<Bytes>>> {
        let path = parts.uri.path();
        let method = &parts.method;

        // Basic HTTP/2 request handling - serve static files for GET requests
        if method == hyper::Method::GET {
            // For simplicity, we'll serve from the default document root
            let document_root = "./www";

            let file_path = if path == "/" {
                format!("{}/index.html", document_root)
            } else {
                format!("{}{}", document_root, path)
            };

            // Security: Prevent directory traversal
            if path.contains("..") {
                return Ok(Response::builder()
                    .status(403)
                    .header("content-type", "text/html")
                    .header("server", "RustWeb/0.1.0 HTTP/2")
                    .body(Full::new(Bytes::from(
                        "<!DOCTYPE html><html><head><title>403 Forbidden</title></head><body><h1>Forbidden</h1><p>Access denied</p></body></html>"
                    )))
                    .unwrap());
            }

            // Try to serve the file
            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    let content_type = Self::get_content_type(&file_path);
                    Ok(Response::builder()
                        .status(200)
                        .header("content-type", content_type)
                        .header("server", "RustWeb/0.1.0 HTTP/2")
                        .header("content-length", content.len())
                        .body(Full::new(Bytes::from(content)))
                        .unwrap())
                }
                Err(_) => {
                    // Return HTTP/2 specific 404
                    Ok(Response::builder()
                        .status(404)
                        .header("content-type", "text/html")
                        .header("server", "RustWeb/0.1.0 HTTP/2")
                        .body(Full::new(Bytes::from(
                            "<!DOCTYPE html><html><head><title>404 Not Found</title></head><body><h1>Not Found</h1><p>The requested resource was not found on this server.</p><hr><small>RustWeb Server HTTP/2</small></body></html>"
                        )))
                        .unwrap())
                }
            }
        } else {
            // Method not allowed for HTTP/2
            Ok(Response::builder()
                .status(405)
                .header("content-type", "text/html")
                .header("server", "RustWeb/0.1.0 HTTP/2")
                .header("allow", "GET")
                .body(Full::new(Bytes::from(
                    "<!DOCTYPE html><html><head><title>405 Method Not Allowed</title></head><body><h1>Method Not Allowed</h1><p>The method is not allowed for this resource.</p></body></html>"
                )))
                .unwrap())
        }
    }

    fn get_content_type(file_path: &str) -> &'static str {
        if file_path.ends_with(".html") || file_path.ends_with(".htm") {
            "text/html; charset=utf-8"
        } else if file_path.ends_with(".css") {
            "text/css"
        } else if file_path.ends_with(".js") {
            "application/javascript"
        } else if file_path.ends_with(".json") {
            "application/json"
        } else if file_path.ends_with(".png") {
            "image/png"
        } else if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
            "image/jpeg"
        } else if file_path.ends_with(".gif") {
            "image/gif"
        } else if file_path.ends_with(".svg") {
            "image/svg+xml"
        } else if file_path.ends_with(".ico") {
            "image/x-icon"
        } else {
            "application/octet-stream"
        }
    }

    async fn send_h2_response(
        respond: &mut SendResponse<Bytes>,
        response: Response<Full<Bytes>>,
    ) -> Result<()> {
        let (parts, body) = response.into_parts();

        // Send response headers
        let response = Response::from_parts(parts, ());
        let mut stream = respond.send_response(response, false)?;

        // Send body
        let body_bytes = body.collect().await?.to_bytes();
        stream.send_data(body_bytes, true)?;

        Ok(())
    }

    pub async fn handle_plain_connection(
        &self,
        _stream: TcpStream,
        client_addr: std::net::SocketAddr,
    ) -> Result<()> {
        // For plain HTTP/2 connections (h2c - HTTP/2 over cleartext)
        // This requires HTTP/1.1 upgrade mechanism
        info!("HTTP/2 cleartext connection attempt from {}", client_addr);

        // For now, we'll just reject plain HTTP/2 connections
        // In a full implementation, we'd handle the HTTP/1.1 upgrade

        Ok(())
    }
}

// HTTP/2 Server Push support (future enhancement)
#[allow(dead_code)]
pub struct Http2PushHandler {
    // Track resources that should be pushed
    push_resources: Vec<String>,
}

#[allow(dead_code)]
impl Default for Http2PushHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl Http2PushHandler {
    pub fn new() -> Self {
        Self {
            push_resources: Vec::new(),
        }
    }

    pub fn add_push_resource(&mut self, path: String) {
        self.push_resources.push(path);
    }

    // Server push implementation would go here
    // This would analyze HTML responses and push CSS/JS resources
}
