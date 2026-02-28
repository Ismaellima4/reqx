/// AST types for the `.reqx` DSL.

/// Represents a parsed `.reqx` file.
#[derive(Debug, Clone)]
pub struct ReqxFile {
    pub variables: Vec<Variable>,
    pub requests: Vec<Request>,
}

/// A variable definition: `@name = value`
#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub line: usize,
}

/// An HTTP request block.
#[derive(Debug, Clone)]
pub struct Request {
    pub comment: Option<String>,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Option<String>,
    pub line: usize,
}

/// An HTTP header: `Key: Value`
#[derive(Debug, Clone)]
pub struct Header {
    pub key: String,
    pub value: String,
}

/// Supported HTTP methods.
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        };
        write!(f, "{}", s)
    }
}

impl HttpMethod {
    /// Parse a string into an HttpMethod, case-insensitive.
    pub fn from_str(s: &str) -> Option<HttpMethod> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            "HEAD" => Some(HttpMethod::Head),
            "OPTIONS" => Some(HttpMethod::Options),
            _ => None,
        }
    }
}
