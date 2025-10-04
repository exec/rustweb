use crate::config::Config;
use anyhow::Result;
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression as GzCompression};
use http_body_util::{BodyExt, Full};
use hyper::body::Body;
use hyper::{body::Incoming, Request, Response};
use std::io::Write;
use std::sync::Arc;

pub struct CompressionHandler {
    config: Arc<Config>,
}

impl CompressionHandler {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub async fn compress_response(
        &self,
        response: Response<Full<Bytes>>,
        request: &Request<Incoming>,
    ) -> Result<Response<Full<Bytes>>> {
        let accept_encoding = request
            .headers()
            .get("accept-encoding")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        self.compress_response_with_encoding(response, accept_encoding)
            .await
    }

    pub async fn compress_response_with_encoding(
        &self,
        response: Response<Full<Bytes>>,
        accept_encoding: &str,
    ) -> Result<Response<Full<Bytes>>> {
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if !self.should_compress(content_type, &response) {
            return Ok(response);
        }

        let body_bytes = response.body().clone().collect().await?.to_bytes();

        if body_bytes.len() < self.config.compression.min_compress_size {
            return Ok(response);
        }

        if self.config.compression.enable_brotli && accept_encoding.contains("br") {
            if let Ok(compressed) = self.compress_brotli(&body_bytes) {
                let (parts, _) = response.into_parts();
                let mut response = Response::from_parts(parts, Full::new(compressed));
                response
                    .headers_mut()
                    .insert("content-encoding", "br".parse().unwrap());
                if let Some(exact_size) = response.body().size_hint().exact() {
                    response
                        .headers_mut()
                        .insert("content-length", exact_size.to_string().parse().unwrap());
                }
                return Ok(response);
            }
        }

        if self.config.compression.enable_gzip && accept_encoding.contains("gzip") {
            if let Ok(compressed) = self.compress_gzip(&body_bytes) {
                let compressed_len = compressed.len();
                let (parts, _) = response.into_parts();
                let mut response = Response::from_parts(parts, Full::new(compressed));
                response
                    .headers_mut()
                    .insert("content-encoding", "gzip".parse().unwrap());
                response.headers_mut().insert(
                    "content-length",
                    compressed_len.to_string().parse().unwrap(),
                );
                return Ok(response);
            }
        }

        if self.config.compression.enable_zstd && accept_encoding.contains("zstd") {
            if let Ok(compressed) = self.compress_zstd(&body_bytes) {
                let compressed_len = compressed.len();
                let (parts, _) = response.into_parts();
                let mut response = Response::from_parts(parts, Full::new(compressed));
                response
                    .headers_mut()
                    .insert("content-encoding", "zstd".parse().unwrap());
                response.headers_mut().insert(
                    "content-length",
                    compressed_len.to_string().parse().unwrap(),
                );
                return Ok(response);
            }
        }

        Ok(response)
    }

    fn should_compress(&self, content_type: &str, response: &Response<Full<Bytes>>) -> bool {
        if response.status().is_redirection()
            || response.status().is_client_error()
            || response.status().is_server_error()
        {
            return false;
        }

        self.config
            .compression
            .compress_types
            .iter()
            .any(|ct| content_type.starts_with(ct))
    }

    fn compress_gzip(&self, data: &[u8]) -> Result<Bytes> {
        let mut encoder = GzEncoder::new(
            Vec::new(),
            GzCompression::new(self.config.compression.compression_level),
        );
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        Ok(Bytes::from(compressed))
    }

    fn compress_brotli(&self, data: &[u8]) -> Result<Bytes> {
        let mut compressed = Vec::new();
        let mut writer = brotli::CompressorWriter::new(
            &mut compressed,
            4096, // buffer size
            self.config.compression.compression_level,
            22, // window size
        );
        writer.write_all(data)?;
        writer.flush()?;
        drop(writer);
        Ok(Bytes::from(compressed))
    }

    fn compress_zstd(&self, data: &[u8]) -> Result<Bytes> {
        let compressed =
            zstd::bulk::compress(data, self.config.compression.compression_level as i32)?;
        Ok(Bytes::from(compressed))
    }
}
