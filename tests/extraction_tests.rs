use reqx_core::ast::HttpMethod;
use reqx_core::client::HttpClient;
use reqx_core::interpreter::execute;
use reqx_core::lexer::tokenize;
use reqx_core::parser::parse;

#[test]
fn test_variable_extraction() {
    let input = r#"
POST https://api.com/login
{ "user": "test" }

@token = token
@uid = user.id

###

GET https://api.com/user/{{uid}}
Authorization: Bearer {{token}}
"#;

    let tokens = tokenize(input).expect("Tokenization failed");
    let file = parse(tokens).expect("Parsing failed");

    struct SequenceMockClient {
        pub calls: std::sync::Mutex<Vec<(String, Vec<(String, String)>)>>,
    }
    impl HttpClient for SequenceMockClient {
        fn execute(
            &self,
            _method: &HttpMethod,
            url: &str,
            headers: &[(String, String)],
            _body: Option<&str>,
        ) -> Result<reqx_core::client::HttpResponse, String> {
            let mut calls = self.calls.lock().unwrap();
            calls.push((url.to_string(), headers.to_vec()));

            if url.contains("/login") {
                Ok(reqx_core::client::HttpResponse {
                    status: 200,
                    status_is_success: true,
                    status_is_client_error: false,
                    status_is_server_error: false,
                    headers: Vec::new(),
                    body: r#"{ "token": "secret-123", "user": { "id": 42 } }"#.to_string(),
                })
            } else {
                Ok(reqx_core::client::HttpResponse {
                    status: 200,
                    status_is_success: true,
                    status_is_client_error: false,
                    status_is_server_error: false,
                    headers: Vec::new(),
                    body: "OK".to_string(),
                })
            }
        }
    }

    let client = SequenceMockClient {
        calls: std::sync::Mutex::new(Vec::new()),
    };

    // Run both requests
    execute(&client, &file, false, false, None, None).expect("Execution failed");

    let calls = client.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);

    // First call: login
    assert_eq!(calls[0].0, "https://api.com/login");

    // Second call: user info with extracted variables
    assert_eq!(calls[1].0, "https://api.com/user/42");
    let auth = calls[1]
        .1
        .iter()
        .find(|(k, _)| k == "Authorization")
        .map(|(_, v)| v.as_str());
    assert_eq!(auth, Some("Bearer secret-123"));
}
