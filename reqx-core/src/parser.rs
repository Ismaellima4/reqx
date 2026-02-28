/// Parser: converts a token stream into the AST.
use crate::ast::{Header, HttpMethod, Request, ReqxFile, Variable};
use crate::lexer::{LocatedToken, Token};

/// Parse a list of tokens into a `ReqxFile` AST.
pub fn parse(tokens: Vec<LocatedToken>) -> Result<ReqxFile, String> {
    let mut variables = Vec::new();
    let mut requests = Vec::new();

    let mut iter = tokens.into_iter().peekable();

    // Parse leading variables (stop if we see a comment that belongs to a request)
    while let Some(lt) = iter.peek() {
        match &lt.token {
            Token::Variable { .. } => {
                let lt = iter.next().unwrap();
                if let Token::Variable { name, value } = lt.token {
                    variables.push(Variable {
                        name,
                        value,
                        line: lt.line,
                    });
                }
            }
            Token::BlankLine => {
                iter.next();
            }
            Token::Comment(_) => {
                // Check if this comment is followed (possibly after blanks) by a Method token.
                // If so, it belongs to a request — don't consume it here.
                let remaining: Vec<LocatedToken> = iter.clone().collect();
                let next_meaningful = remaining
                    .iter()
                    .skip(1)
                    .find(|t| t.token != Token::BlankLine);
                if let Some(nlt) = next_meaningful {
                    if matches!(&nlt.token, Token::Method(_)) {
                        break; // let the request parser handle this comment
                    }
                }
                iter.next();
            }
            Token::Separator => {
                iter.next();
                break;
            }
            _ => break,
        }
    }

    // Parse request blocks
    loop {
        // Skip blank lines and separators between requests
        while let Some(lt) = iter.peek() {
            match &lt.token {
                Token::BlankLine | Token::Separator => {
                    iter.next();
                }
                _ => break,
            }
        }

        if iter.peek().is_none() {
            break;
        }

        match parse_request(&mut iter) {
            Ok(req) => requests.push(req),
            Err(e) => return Err(e),
        }
    }

    Ok(ReqxFile {
        variables,
        requests,
    })
}

fn parse_comment(
    iter: &mut std::iter::Peekable<std::vec::IntoIter<LocatedToken>>,
) -> Option<String> {
    let mut comment = None;
    while let Some(lt) = iter.peek() {
        match &lt.token {
            Token::Comment(_) => {
                let lt = iter.next().unwrap();
                if let Token::Comment(text) = lt.token {
                    comment = Some(text);
                }
            }
            Token::BlankLine => {
                iter.next();
            }
            _ => break,
        }
    }
    comment
}

fn parse_method_and_url(
    iter: &mut std::iter::Peekable<std::vec::IntoIter<LocatedToken>>,
) -> Result<(Option<HttpMethod>, String, usize), String> {
    let first_token = iter
        .next()
        .ok_or_else(|| "Unexpected end of input: expected HTTP method or URL".to_string())?;

    let req_line = first_token.line;

    match first_token.token {
        Token::Method(m_str) => {
            let method = m_str
                .parse::<HttpMethod>()
                .ok()
                .ok_or_else(|| format!("Line {}: unsupported HTTP method: {}", req_line, m_str))?;

            let url_token = iter
                .next()
                .ok_or_else(|| format!("Line {}: expected URL after method", req_line))?;

            let url = match url_token.token {
                Token::Url(u) => u,
                other => {
                    return Err(format!(
                        "Line {}: expected URL, found {:?}",
                        url_token.line, other
                    ));
                }
            };
            Ok((Some(method), url, req_line))
        }
        Token::Url(u) => Ok((None, u, req_line)),
        other => Err(format!(
            "Line {}: expected HTTP method or URL, found {:?}",
            req_line, other
        )),
    }
}

fn parse_headers(iter: &mut std::iter::Peekable<std::vec::IntoIter<LocatedToken>>) -> Vec<Header> {
    let mut headers = Vec::new();
    while let Some(lt) = iter.peek() {
        match &lt.token {
            Token::Header { .. } => {
                let lt = iter.next().unwrap();
                if let Token::Header { key, value } = lt.token {
                    headers.push(Header { key, value });
                }
            }
            Token::BlankLine => {
                iter.next();
                break;
            }
            Token::Separator | Token::Comment(_) | Token::Method(_) | Token::Variable { .. } => {
                break;
            }
            _ => {
                iter.next();
                break;
            }
        }
    }
    headers
}

fn parse_body(iter: &mut std::iter::Peekable<std::vec::IntoIter<LocatedToken>>) -> Option<String> {
    let mut body_lines = Vec::new();
    while let Some(lt) = iter.peek() {
        match &lt.token {
            Token::BodyLine(_) => {
                let lt = iter.next().unwrap();
                if let Token::BodyLine(line) = lt.token {
                    body_lines.push(line);
                }
            }
            Token::BlankLine => {
                let next = iter.peek();
                if let Some(nlt) = next {
                    match &nlt.token {
                        Token::BodyLine(_) => {
                            iter.next();
                            body_lines.push(String::new());
                        }
                        _ => break,
                    }
                } else {
                    break;
                }
            }
            Token::Separator | Token::Comment(_) | Token::Method(_) | Token::Variable { .. } => {
                break;
            }
            _ => break,
        }
    }

    if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n"))
    }
}

fn parse_request(
    iter: &mut std::iter::Peekable<std::vec::IntoIter<LocatedToken>>,
) -> Result<Request, String> {
    let comment = parse_comment(iter);
    let (method_opt, url, line) = parse_method_and_url(iter)?;
    let headers = parse_headers(iter);
    let body = parse_body(iter);

    let method = method_opt.unwrap_or_else(|| {
        if body.is_some() {
            HttpMethod::Post
        } else {
            HttpMethod::Get
        }
    });

    Ok(Request {
        comment,
        method,
        url,
        headers,
        body,
        line,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_parse_simple_get() {
        let input = "GET https://api.example.com/users\nAccept: application/json";
        let tokens = tokenize(input).unwrap();
        let file = parse(tokens).unwrap();
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].method, HttpMethod::Get);
        assert_eq!(file.requests[0].url, "https://api.example.com/users");
        assert_eq!(file.requests[0].headers.len(), 1);
        assert_eq!(file.requests[0].headers[0].key, "Accept");
    }

    #[test]
    fn test_parse_with_variables() {
        let input = r#"@base_url = https://api.example.com
@token = abc123

###

GET {{base_url}}/users
Authorization: Bearer {{token}}"#;
        let tokens = tokenize(input).unwrap();
        let file = parse(tokens).unwrap();
        assert_eq!(file.variables.len(), 2);
        assert_eq!(file.variables[0].name, "base_url");
        assert_eq!(file.variables[1].name, "token");
        assert_eq!(file.requests.len(), 1);
    }

    #[test]
    fn test_parse_post_with_body() {
        let input = r#"POST https://api.example.com/users
Content-Type: application/json

{
  "name": "Test User",
  "email": "test@example.com"
}"#;
        let tokens = tokenize(input).unwrap();
        let file = parse(tokens).unwrap();
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].method, HttpMethod::Post);
        assert!(file.requests[0].body.is_some());
        let body = file.requests[0].body.as_ref().unwrap();
        assert!(body.contains("\"name\""));
        assert!(body.contains("Test User"));
    }

    #[test]
    fn test_parse_multiple_requests() {
        let input = r#"# First request
GET https://api.example.com/users

###

# Second request
POST https://api.example.com/users
Content-Type: application/json

{"name": "test"}"#;
        let tokens = tokenize(input).unwrap();
        let file = parse(tokens).unwrap();
        assert_eq!(file.requests.len(), 2);
        assert_eq!(file.requests[0].method, HttpMethod::Get);
        assert_eq!(file.requests[0].comment, Some("First request".to_string()));
        assert_eq!(file.requests[1].method, HttpMethod::Post);
        assert_eq!(file.requests[1].comment, Some("Second request".to_string()));
    }

    #[test]
    fn test_parse_implicit_methods() {
        let input = r#"
# No body, defaults to GET
:3000/users

###

# Has body, defaults to POST
:3000/users

{"name": "test"}
        "#;
        let tokens = tokenize(input).unwrap();
        let file = parse(tokens).unwrap();
        assert_eq!(file.requests.len(), 2);
        assert_eq!(file.requests[0].method, HttpMethod::Get);
        assert_eq!(file.requests[0].url, ":3000/users");

        assert_eq!(file.requests[1].method, HttpMethod::Post);
        assert_eq!(file.requests[1].url, ":3000/users");
        assert!(file.requests[1].body.is_some());
    }
}
