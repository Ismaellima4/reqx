use crate::ast::HttpMethod;

/// Output of a completed HTTP request.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub status_is_success: bool,
    pub status_is_client_error: bool,
    pub status_is_server_error: bool,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// A generic interface to execute an HTTP request.
/// Your application can implement this trait and pass it to `interpreter::execute`
/// to decouple `reqx` from any specific HTTP library.
pub trait HttpClient {
    fn execute(
        &self,
        method: &HttpMethod,
        url: &str,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> Result<HttpResponse, String>;
}
