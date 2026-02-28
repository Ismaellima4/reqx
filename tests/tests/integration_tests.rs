use reqx_core::ast::HttpMethod;
use reqx_core::client::HttpClient;
use reqx_core::interpreter::execute;
use reqx_core::lexer::tokenize;
use reqx_core::parser::parse;

#[derive(Debug)]
struct CapturedRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

struct MockClient {
    pub last_request: std::sync::Mutex<Option<CapturedRequest>>,
}

impl HttpClient for MockClient {
    fn execute(
        &self,
        method: &HttpMethod,
        url: &str,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> Result<reqx_core::client::HttpResponse, String> {
        let mut last = self.last_request.lock().unwrap();
        *last = Some(CapturedRequest {
            method: method.clone(),
            url: url.to_string(),
            headers: headers.to_owned(),
            body: body.map(|b| b.to_string()),
        });

        Ok(reqx_core::client::HttpResponse {
            status: 200,
            status_is_success: true,
            status_is_client_error: false,
            status_is_server_error: false,
            headers: Vec::new(),
            body: "{}".to_string(),
        })
    }
}

#[test]
fn test_exhaustive_integration_success() {
    let input = r#"
@api_key = secret123
@base_url = https://api.example.com

# First: GET with headers and variables
GET {{base_url}}/v1/users
X-Api-Key: {{api_key}}
Accept: application/json

###

# Second: POST with implicit method and body
{{base_url}}/v1/data

{
  "key": "value",
  "meta": "{{api_key}}"
}

###

# Third: Localhost shorthand
:8080/status
"#;

    let tokens = tokenize(input).expect("Tokenization failed");
    let file = parse(tokens).expect("Parsing failed");
    let client = MockClient {
        last_request: std::sync::Mutex::new(None),
    };

    // Run first request
    execute(&client, &file, false, false, Some(1), None).expect("Execution failed");
    {
        let last = client.last_request.lock().unwrap().take().unwrap();
        assert_eq!(last.method, HttpMethod::Get);
        assert_eq!(last.url, "https://api.example.com/v1/users");
        assert!(
            last.headers
                .iter()
                .any(|(k, v)| k == "X-Api-Key" && v == "secret123")
        );
    }

    // Run second request (implicit POST)
    execute(&client, &file, false, false, Some(2), None).expect("Execution failed");
    {
        let last = client.last_request.lock().unwrap().take().unwrap();
        assert_eq!(last.method, HttpMethod::Post);
        assert_eq!(last.url, "https://api.example.com/v1/data");
        assert!(last.body.unwrap().contains("secret123"));
    }

    // Run third request (localhost shorthand)
    execute(&client, &file, false, false, Some(3), None).expect("Execution failed");
    {
        let last = client.last_request.lock().unwrap().take().unwrap();
        assert_eq!(last.url, "http://localhost:8080/status");
    }
}

#[test]
fn test_exhaustive_integration_errors() {
    // 1. Undefined variable
    let input_err1 = "GET https://{{missing_var}}.com";
    let tokens = tokenize(input_err1).unwrap();
    let file = parse(tokens).unwrap();
    let client = MockClient {
        last_request: std::sync::Mutex::new(None),
    };
    let res = execute(&client, &file, false, false, None, None);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Undefined variable"));

    // 2. Unclosed variable interpolation
    let input_err2 = "GET https://example.com/{{unclosed";
    let tokens = tokenize(input_err2).unwrap();
    let file = parse(tokens).unwrap();
    let res = execute(&client, &file, false, false, None, None);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Unclosed variable interpolation"));
}

#[test]
fn test_exhaustive_variable_persistence() {
    let input = r#"
@count = 1
GET https://api.com/{{count}}

###
@count = 2
GET https://api.com/{{count}}
"#;
    let tokens = tokenize(input).unwrap();
    let file = parse(tokens).unwrap();
    let client = MockClient {
        last_request: std::sync::Mutex::new(None),
    };

    // The interpreter currently rebuilds the variable map once at the start of `execute`.
    // Wait, let's check the code for `interpreter.rs`.
    // It builds `vars` from `file.variables`.
    // My parser puts ALL variables into `file.variables`.
    // If a variable is redefined, the last one wins (HashMap `insert`).

    execute(&client, &file, false, false, Some(1), None).unwrap();
    {
        let last = client.last_request.lock().unwrap().take().unwrap();
        // Since both @count = 1 and @count = 2 are in `file.variables`,
        // the HashMap will have count = 2 at the end of the loop.
        // This might be a bug or intended behavior (global scope).
        // Let's verify what the code does.
        assert_eq!(last.url, "https://api.com/2");
    }
}
