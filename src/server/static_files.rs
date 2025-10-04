use crate::server::response::{ErrorResponse, ResponseBuilder};
use anyhow::Result;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use mime_guess::MimeGuess;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, warn};

pub struct StaticFileHandler;

impl StaticFileHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn serve_file(
        &self,
        req: &Request<Incoming>,
        document_root: &str,
        index_files: &[String],
    ) -> Result<Response<Full<Bytes>>> {
        if req.method() != Method::GET && req.method() != Method::HEAD {
            return Ok(ErrorResponse::method_not_allowed().build());
        }

        let request_path = req.uri().path();
        let sanitized_path = self.sanitize_path(request_path)?;

        let full_path = Path::new(document_root).join(&sanitized_path);

        debug!("Serving static file: {}", full_path.display());

        if !self.is_safe_path(&full_path, document_root)? {
            warn!("Attempted path traversal attack: {}", request_path);
            return Ok(ErrorResponse::forbidden().build());
        }

        let metadata = match fs::metadata(&full_path).await {
            Ok(meta) => meta,
            Err(_) => return Ok(ErrorResponse::not_found().build()),
        };

        let file_path = if metadata.is_dir() {
            match self.find_index_file(&full_path, index_files).await {
                Some(index_path) => index_path,
                None => return Ok(ErrorResponse::forbidden().build()),
            }
        } else {
            full_path
        };

        self.serve_single_file(req, &file_path).await
    }

    async fn serve_single_file(
        &self,
        req: &Request<Incoming>,
        file_path: &Path,
    ) -> Result<Response<Full<Bytes>>> {
        let metadata = match fs::metadata(file_path).await {
            Ok(meta) => meta,
            Err(_) => return Ok(ErrorResponse::not_found().build()),
        };

        if metadata.is_dir() {
            return Ok(ErrorResponse::forbidden().build());
        }

        let mime_type = MimeGuess::from_path(file_path)
            .first()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM);

        if let Some(if_none_match) = req.headers().get("if-none-match") {
            let etag = self.generate_etag(&metadata);
            if if_none_match.to_str().unwrap_or("") == etag {
                return Ok(ResponseBuilder::new(StatusCode::NOT_MODIFIED).build());
            }
        }

        if let Some(if_modified_since) = req.headers().get("if-modified-since") {
            if let Ok(since_time) =
                httpdate::parse_http_date(if_modified_since.to_str().unwrap_or(""))
            {
                if let Ok(modified_time) = metadata.modified() {
                    if modified_time <= since_time {
                        return Ok(ResponseBuilder::new(StatusCode::NOT_MODIFIED).build());
                    }
                }
            }
        }

        let content = if req.method() == Method::HEAD {
            Bytes::new()
        } else {
            match fs::read(file_path).await {
                Ok(content) => Bytes::from(content),
                Err(_) => return Ok(ErrorResponse::internal_server_error().build()),
            }
        };

        let etag = self.generate_etag(&metadata);
        let last_modified = self.format_last_modified(&metadata);

        let response = ResponseBuilder::new(StatusCode::OK)
            .header_string("content-type", mime_type.to_string())
            .header_string("content-length", content.len().to_string())
            .header_string("etag", etag)
            .header_string("last-modified", last_modified)
            .header("accept-ranges", "bytes")
            .body(content)
            .build();

        Ok(response)
    }

    fn sanitize_path(&self, path: &str) -> Result<String> {
        let decoded =
            urlencoding::decode(path).map_err(|_| anyhow::anyhow!("Invalid URL encoding"))?;

        let path = decoded.trim_start_matches('/');

        if path.contains("..") || path.contains('\0') {
            return Err(anyhow::anyhow!("Invalid path"));
        }

        Ok(path.to_string())
    }

    fn is_safe_path(&self, requested_path: &Path, document_root: &str) -> Result<bool> {
        let canonical_requested = requested_path
            .canonicalize()
            .unwrap_or_else(|_| requested_path.to_path_buf());

        let canonical_root = Path::new(document_root)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(document_root));

        Ok(canonical_requested.starts_with(canonical_root))
    }

    async fn find_index_file(&self, dir_path: &Path, index_files: &[String]) -> Option<PathBuf> {
        for index_file in index_files {
            let index_path = dir_path.join(index_file);
            if let Ok(metadata) = fs::metadata(&index_path).await {
                if metadata.is_file() {
                    return Some(index_path);
                }
            }
        }
        None
    }

    fn generate_etag(&self, metadata: &std::fs::Metadata) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }
        format!("\"{}\"", hasher.finish())
    }

    fn format_last_modified(&self, metadata: &std::fs::Metadata) -> String {
        match metadata.modified() {
            Ok(time) => httpdate::fmt_http_date(time),
            Err(_) => httpdate::fmt_http_date(std::time::SystemTime::now()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_path() {
        let handler = StaticFileHandler::new();

        assert_eq!(handler.sanitize_path("/index.html").unwrap(), "index.html");
        assert_eq!(
            handler.sanitize_path("/css/style.css").unwrap(),
            "css/style.css"
        );
        assert!(handler.sanitize_path("/../etc/passwd").is_err());
        assert!(handler.sanitize_path("/file\0.txt").is_err());
    }
}
