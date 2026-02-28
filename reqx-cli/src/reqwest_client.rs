use reqx_core::ast::HttpMethod;
use reqx_core::client::{HttpClient, HttpResponse};

/// A default HTTP client using `reqwest` blocking client.
pub struct ReqwestClient {
    client: reqwest::blocking::Client,
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for ReqwestClient {
    fn execute(
        &self,
        method: &HttpMethod,
        url: &str,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> Result<HttpResponse, String> {
        let mut builder = match method {
            HttpMethod::Get => self.client.get(url),
            HttpMethod::Post => self.client.post(url),
            HttpMethod::Put => self.client.put(url),
            HttpMethod::Patch => self.client.patch(url),
            HttpMethod::Delete => self.client.delete(url),
            HttpMethod::Head => self.client.head(url),
            HttpMethod::Options => self.client.request(reqwest::Method::OPTIONS, url),
        };

        for (k, v) in headers {
            builder = builder.header(k.as_str(), v.as_str());
        }

        if let Some(b) = body {
            builder = builder.body(b.to_string());
        }

        let response = builder
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let status_code = status.as_u16();
        let status_is_success = status.is_success();
        let status_is_client_error = status.is_client_error();
        let status_is_server_error = status.is_server_error();

        let mut out_headers = Vec::new();
        for (k, v) in response.headers() {
            out_headers.push((
                k.as_str().to_string(),
                v.to_str().unwrap_or("(binary)").to_string(),
            ));
        }

        let body_text = response
            .text()
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        Ok(HttpResponse {
            status: status_code,
            status_is_success,
            status_is_client_error,
            status_is_server_error,
            headers: out_headers,
            body: body_text,
        })
    }
}
