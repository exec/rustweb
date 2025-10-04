use anyhow::Result;
use serde_json::json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;
use uuid::Uuid;

pub struct AccessLogger {
    file: Option<Arc<Mutex<std::fs::File>>>,
    format: AccessLogFormat,
}

#[derive(Clone)]
pub enum AccessLogFormat {
    Json,
    CommonLog,
    Combined,
}

#[derive(Debug)]
pub struct LogEntry {
    pub request_id: Uuid,
    pub remote_addr: String,
    pub method: String,
    pub uri: String,
    pub status: u16,
    pub response_size: usize,
    pub duration_ms: f64,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AccessLogger {
    pub fn new(log_path: Option<&str>, format: AccessLogFormat) -> Result<Self> {
        let file = if let Some(path) = log_path {
            let file = OpenOptions::new().create(true).append(true).open(path)?;
            Some(Arc::new(Mutex::new(file)))
        } else {
            None
        };

        Ok(Self { file, format })
    }

    pub async fn log(&self, entry: LogEntry) {
        let log_line = self.format_entry(&entry);

        if let Some(ref file) = self.file {
            let mut file_guard = file.lock().await;
            if let Err(e) = writeln!(file_guard, "{}", log_line) {
                error!("Failed to write access log: {}", e);
            }
            if let Err(e) = file_guard.flush() {
                error!("Failed to flush access log: {}", e);
            }
        } else {
            // Log to stdout if no file specified
            println!("{}", log_line);
        }
    }

    fn format_entry(&self, entry: &LogEntry) -> String {
        match self.format {
            AccessLogFormat::Json => json!({
                "timestamp": entry.timestamp.to_rfc3339(),
                "request_id": entry.request_id.to_string(),
                "remote_addr": entry.remote_addr,
                "method": entry.method,
                "uri": entry.uri,
                "status": entry.status,
                "response_size": entry.response_size,
                "duration_ms": entry.duration_ms,
                "user_agent": entry.user_agent,
                "referer": entry.referer
            })
            .to_string(),
            AccessLogFormat::CommonLog => {
                format!(
                    "{} - - [{}] \"{} {} HTTP/1.1\" {} {}",
                    entry.remote_addr,
                    entry.timestamp.format("%d/%b/%Y:%H:%M:%S %z"),
                    entry.method,
                    entry.uri,
                    entry.status,
                    entry.response_size
                )
            }
            AccessLogFormat::Combined => {
                format!(
                    "{} - - [{}] \"{} {} HTTP/1.1\" {} {} \"{}\" \"{}\"",
                    entry.remote_addr,
                    entry.timestamp.format("%d/%b/%Y:%H:%M:%S %z"),
                    entry.method,
                    entry.uri,
                    entry.status,
                    entry.response_size,
                    entry.referer.as_deref().unwrap_or("-"),
                    entry.user_agent.as_deref().unwrap_or("-")
                )
            }
        }
    }
}

pub struct LogRotator {
    base_path: String,
    max_size: u64,
    max_files: usize,
}

impl LogRotator {
    pub fn new(base_path: String, max_size: u64, max_files: usize) -> Self {
        Self {
            base_path,
            max_size,
            max_files,
        }
    }

    pub async fn should_rotate(&self) -> bool {
        if let Ok(metadata) = tokio::fs::metadata(&self.base_path).await {
            metadata.len() > self.max_size
        } else {
            false
        }
    }

    pub async fn rotate(&self) -> Result<()> {
        // Rotate existing files
        for i in (1..self.max_files).rev() {
            let old_path = format!("{}.{}", self.base_path, i);
            let new_path = format!("{}.{}", self.base_path, i + 1);

            if Path::new(&old_path).exists() {
                tokio::fs::rename(&old_path, &new_path).await?;
            }
        }

        // Move current to .1
        if Path::new(&self.base_path).exists() {
            let rotated_path = format!("{}.1", self.base_path);
            tokio::fs::rename(&self.base_path, &rotated_path).await?;
        }

        Ok(())
    }
}
