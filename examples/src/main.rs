use reqx_core::ast::HttpMethod;
use reqx_core::client::{HttpClient, HttpResponse};
use reqx_core::{interpreter, lexer, parser};
use std::fs;
use std::process;

/// A simple Mock Client for demonstration.
/// It doesn't actually make HTTP requests, but returns canned responses.
pub struct ExampleMockClient;

impl HttpClient for ExampleMockClient {
    fn execute(
        &self,
        method: &HttpMethod,
        url: &str,
        _headers: &[(String, String)],
        _body: Option<&str>,
    ) -> Result<HttpResponse, String> {
        println!(">>> [MOCK] Intercepted a {} request to '{}'", method, url);

        Ok(HttpResponse {
            status: 200,
            status_is_success: true,
            status_is_client_error: false,
            status_is_server_error: false,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: r#"{"message": "Hello from ExampleMockClient!"}"#.to_string(),
        })
    }
}

fn main() {
    let example_file = "reqx-examples/exemplo.reqx";

    println!("Loading {}...", example_file);
    let contents = match fs::read_to_string(example_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            process::exit(1);
        }
    };

    let tokens = lexer::tokenize(&contents).expect("Lexer failed");
    let reqx_file = parser::parse(tokens).expect("Parser failed");

    // We instantiate our custom mock client instead of reqwest
    let client = ExampleMockClient;

    // Set arguments as if we called from CLI
    let verbose = true;
    let dry_run = false;
    let request_index = None;
    let method_filter = None;

    println!("Executing parsed reqx file with Mock Client...\n");
    if let Err(e) = interpreter::execute(
        &client,
        &reqx_file,
        verbose,
        dry_run,
        request_index,
        method_filter,
    ) {
        eprintln!("Execution error: {}", e);
        process::exit(1);
    }
}
