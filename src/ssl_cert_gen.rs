use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

pub struct CertificateGenerator;

impl CertificateGenerator {
    pub async fn ensure_certificates_exist(cert_path: &str, key_path: &str, auto_generate: bool) -> Result<()> {
        let cert_exists = Path::new(cert_path).exists();
        let key_exists = Path::new(key_path).exists();

        if cert_exists && key_exists {
            info!("SSL certificates found at {} and {}", cert_path, key_path);
            return Ok(());
        }

        if !auto_generate {
            return Err(anyhow::anyhow!(
                "SSL certificates not found at {} and {}, and auto-generation is disabled",
                cert_path, key_path
            ));
        }

        warn!("SSL certificates not found, generating self-signed certificates");
        Self::generate_self_signed_cert(cert_path, key_path).await
    }

    async fn generate_self_signed_cert(cert_path: &str, key_path: &str) -> Result<()> {
        // Create directory if it doesn't exist
        if let Some(cert_dir) = Path::new(cert_path).parent() {
            fs::create_dir_all(cert_dir)
                .with_context(|| format!("Failed to create certificate directory: {}", cert_dir.display()))?;
        }

        info!("Generating self-signed SSL certificate");
        
        // Use openssl command to generate certificate
        let output = Command::new("openssl")
            .args([
                "req", "-x509", "-newkey", "rsa:2048",
                "-keyout", key_path,
                "-out", cert_path,
                "-days", "365",
                "-nodes",
                "-subj", "/C=US/ST=Auto/L=Auto/O=RustWeb/OU=Auto/CN=localhost"
            ])
            .output()
            .context("Failed to execute openssl command. Please ensure openssl is installed.")?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to generate SSL certificate: {}", error_msg));
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

    // Alternative pure-Rust implementation (for when openssl isn't available)
    #[allow(dead_code)]
    async fn generate_self_signed_cert_rust(cert_path: &str, key_path: &str) -> Result<()> {
        // This would use rcgen crate to generate certificates entirely in Rust
        // For now, we'll rely on openssl command as it's more universally available
        // and produces more standard certificates
        
        info!("Pure Rust certificate generation not yet implemented");
        info!("Please install openssl or provide existing certificates");
        
        Err(anyhow::anyhow!("Certificate generation requires openssl command"))
    }

    pub fn validate_certificate_files(cert_path: &str, key_path: &str) -> Result<()> {
        // Basic validation - check if files exist and are readable
        if !Path::new(cert_path).exists() {
            return Err(anyhow::anyhow!("Certificate file not found: {}", cert_path));
        }

        if !Path::new(key_path).exists() {
            return Err(anyhow::anyhow!("Private key file not found: {}", key_path));
        }

        // Check if files are readable
        fs::read_to_string(cert_path)
            .with_context(|| format!("Cannot read certificate file: {}", cert_path))?;

        fs::read_to_string(key_path)
            .with_context(|| format!("Cannot read private key file: {}", key_path))?;

        Ok(())
    }

    pub fn create_default_ssl_directories() -> Result<()> {
        let ssl_dir = "/etc/rustweb/ssl";
        
        // Try to create the directory, but don't fail if we can't (might not have permissions)
        match fs::create_dir_all(ssl_dir) {
            Ok(_) => {
                info!("Created SSL directory: {}", ssl_dir);
                Ok(())
            }
            Err(e) => {
                warn!("Could not create SSL directory {}: {}. You may need to run with sudo or create it manually.", ssl_dir, e);
                // For development, fall back to local directory
                let local_ssl_dir = "./ssl";
                fs::create_dir_all(local_ssl_dir)
                    .with_context(|| format!("Failed to create local SSL directory: {}", local_ssl_dir))?;
                info!("Created local SSL directory: {}", local_ssl_dir);
                Ok(())
            }
        }
    }
}