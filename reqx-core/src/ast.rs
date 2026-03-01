//! AST types for the `.reqx` DSL.

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
    pub extracts: Vec<Variable>,
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

impl std::str::FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "PATCH" => Ok(HttpMethod::Patch),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            _ => Err(()),
        }
    }
}
