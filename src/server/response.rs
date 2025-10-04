use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Response, StatusCode};
use std::collections::HashMap;

pub struct ResponseBuilder {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Bytes,
}

impl ResponseBuilder {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    pub fn header_string(mut self, name: &str, value: String) -> Self {
        self.headers.insert(name.to_string(), value);
        self
    }

    pub fn body(mut self, body: Bytes) -> Self {
        self.body = body;
        self
    }

    pub fn build(self) -> Response<Full<Bytes>> {
        let mut response = Response::builder().status(self.status);

        for (name, value) in self.headers {
            response = response.header(&name, value);
        }

        response
            .body(Full::new(self.body))
            .expect("Failed to build response")
    }
}

pub struct ErrorResponse;

impl ErrorResponse {
    pub fn bad_request() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::BAD_REQUEST)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/400.html")))
    }

    pub fn forbidden() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::FORBIDDEN)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/403.html")))
    }

    pub fn not_found() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::NOT_FOUND)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/404.html")))
    }

    pub fn method_not_allowed() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::METHOD_NOT_ALLOWED)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/405.html")))
    }

    pub fn internal_server_error() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::INTERNAL_SERVER_ERROR)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/500.html")))
    }

    pub fn bad_gateway() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::BAD_GATEWAY)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/502.html")))
    }

    pub fn service_unavailable() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::SERVICE_UNAVAILABLE)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/503.html")))
    }

    pub fn gateway_timeout() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::GATEWAY_TIMEOUT)
            .header("content-type", "text/html")
            .body(Bytes::from_static(include_bytes!("../../static/504.html")))
    }

    pub fn too_many_requests() -> ResponseBuilder {
        ResponseBuilder::new(StatusCode::TOO_MANY_REQUESTS)
            .header("content-type", "text/html")
            .header("retry-after", "60")
            .body(Bytes::from_static(include_bytes!("../../static/429.html")))
    }
}
