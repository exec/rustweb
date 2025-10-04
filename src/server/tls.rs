#![allow(dead_code)]

use crate::config::SslConfig;
use anyhow::{Context, Result};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

#[derive(Clone)]
pub struct TlsManager {
    acceptor: Option<TlsAcceptor>,
}

impl TlsManager {
    pub fn new(ssl_config: Option<&SslConfig>) -> Result<Self> {
        let acceptor = if let Some(ssl) = ssl_config {
            Some(Self::create_tls_acceptor(ssl)?)
        } else {
            None
        };

        Ok(Self { acceptor })
    }

    pub fn get_acceptor(&self) -> Option<&TlsAcceptor> {
        self.acceptor.as_ref()
    }

    fn create_tls_acceptor(ssl_config: &SslConfig) -> Result<TlsAcceptor> {
        // Check if certificates exist, generate if needed
        use std::path::Path;
        let cert_exists = Path::new(&ssl_config.certificate).exists();
        let key_exists = Path::new(&ssl_config.private_key).exists();

        if !cert_exists || !key_exists {
            if ssl_config.auto_generate_self_signed {
                // Use blocking certificate generation
                Self::generate_cert_blocking(&ssl_config.certificate, &ssl_config.private_key)?;
            } else {
                return Err(anyhow::anyhow!(
                    "SSL certificates not found at {} and {}, and auto-generation is disabled",
                    ssl_config.certificate,
                    ssl_config.private_key
                ));
            }
        }
        // Load certificates
        let cert_file = File::open(&ssl_config.certificate).with_context(|| {
            format!(
                "Failed to open certificate file: {}",
                ssl_config.certificate
            )
        })?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain: Vec<_> = certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse certificate file")?;

        // Load private key
        let key_file = File::open(&ssl_config.private_key).with_context(|| {
            format!(
                "Failed to open private key file: {}",
                ssl_config.private_key
            )
        })?;
        let mut key_reader = BufReader::new(key_file);
        let keys: Vec<_> = pkcs8_private_keys(&mut key_reader)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse private key file")?;

        if keys.is_empty() {
            return Err(anyhow::anyhow!("No private keys found in key file"));
        }

        let private_key = keys.into_iter().next().unwrap();

        // Configure TLS
        let mut config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, private_key.into())
            .context("Failed to build TLS configuration")?;

        // Set ALPN protocols - prioritize HTTP/2, fallback to HTTP/1.1
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(TlsAcceptor::from(Arc::new(config)))
    }

    pub fn supports_http2(&self) -> bool {
        self.acceptor.is_some()
    }

    fn generate_cert_blocking(cert_path: &str, key_path: &str) -> Result<()> {
        use std::fs;
        use std::path::Path;
        use std::process::Command;
        use tracing::{info, warn};

        // Create directory if it doesn't exist
        if let Some(cert_dir) = Path::new(cert_path).parent() {
            fs::create_dir_all(cert_dir).with_context(|| {
                format!(
                    "Failed to create certificate directory: {}",
                    cert_dir.display()
                )
            })?;
        }

        info!("Generating self-signed SSL certificate");
        warn!("SSL certificates not found, generating self-signed certificates");

        // Use openssl command to generate certificate
        let output = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-keyout",
                key_path,
                "-out",
                cert_path,
                "-days",
                "365",
                "-nodes",
                "-subj",
                "/C=US/ST=Auto/L=Auto/O=RustWeb/OU=Auto/CN=localhost",
            ])
            .output()
            .context("Failed to execute openssl command. Please ensure openssl is installed.")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to generate SSL certificate: {}",
                error_msg
            ));
        }

        // Set appropriate permissions on the private key (600)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(key_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(key_path, perms)?;
        }

        info!("Successfully generated self-signed SSL certificate");
        warn!("⚠️  Using self-signed certificate. For production, replace with a proper certificate from a CA.");

        Ok(())
    }
}
