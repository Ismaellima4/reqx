//! Lexer (tokenizer) for `.reqx` files.

/// Token types produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A comment line: `# some comment`
    Comment(String),
    /// Request separator: `###`
    Separator,
    /// Variable definition: `@name = value`
    Variable { name: String, value: String },
    /// HTTP method keyword (GET, POST, etc.)
    Method(String),
    /// A URL string
    Url(String),
    /// A header line: `Key: Value`
    Header { key: String, value: String },
    /// A body line (plain text or JSON)
    BodyLine(String),
    /// An empty line
    BlankLine,
}

/// A token with its source line number.
#[derive(Debug, Clone)]
pub struct LocatedToken {
    pub token: Token,
    pub line: usize,
}

const HTTP_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// Internal lexer state machine.
struct Lexer {
    tokens: Vec<LocatedToken>,
    in_body: bool,
    has_request_line: bool,
}

impl Lexer {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            in_body: false,
            has_request_line: false,
        }
    }

    /// Convenience: push a token with its line number.
    fn push(&mut self, token: Token, line: usize) {
        self.tokens.push(LocatedToken { token, line });
    }

    /// Returns the last non-blank token, if any.
    fn last_meaningful_token(&self) -> Option<&Token> {
        self.tokens
            .iter()
            .rev()
            .find(|t| t.token != Token::BlankLine)
            .map(|t| &t.token)
    }

    // ── Individual classifiers ───────────────────────────────────────

    /// `###` — request separator. Also resets body mode.
    fn try_separator(&mut self, trimmed: &str, line: usize) -> bool {
        if trimmed != "###" {
            return false;
        }
        self.in_body = false;
        self.has_request_line = false;
        self.push(Token::Separator, line);
        true
    }

    /// Empty / whitespace-only line. Detects transition into body mode.
    fn try_blank(&mut self, trimmed: &str, line: usize) -> bool {
        if !trimmed.is_empty() {
            return false;
        }
        if !self.in_body {
            let starts_body = matches!(
                self.last_meaningful_token(),
                Some(Token::Header { .. } | Token::Url(_))
            );
            if starts_body {
                self.in_body = true;
            }
        }
        self.push(Token::BlankLine, line);
        true
    }

    /// `@name = value` — variable definition.
    fn try_variable(&mut self, line_str: &str, line: usize) -> Result<bool, String> {
        if !line_str.starts_with('@') {
            return Ok(false);
        }
        let eq_pos = line_str.find('=').ok_or_else(|| {
            format!(
                "Line {}: invalid variable definition (missing '='): {}",
                line, line_str
            )
        })?;
        let name = line_str[1..eq_pos].trim().to_string();
        if name.is_empty() {
            return Err(format!("Line {}: empty variable name", line));
        }
        let value = line_str[eq_pos + 1..].trim().to_string();

        // If we find a variable, it marks the end of a body (extraction)
        self.in_body = false;

        self.push(Token::Variable { name, value }, line);
        Ok(true)
    }

    /// `# text` — comment (already guaranteed not to be `###`).
    fn try_comment(&mut self, line_str: &str, line: usize) -> bool {
        if !line_str.starts_with('#') {
            return false;
        }
        let text = line_str[1..].trim().to_string();
        self.push(Token::Comment(text), line);
        true
    }

    /// `METHOD url` or just `url` — request line.
    fn try_request_line(&mut self, trimmed: &str, line: usize) -> bool {
        if self.has_request_line {
            return false;
        }

        let first_word = trimmed.split_whitespace().next().unwrap_or("");
        let upper = first_word.to_uppercase();

        if HTTP_METHODS.contains(&upper.as_str()) {
            self.push(Token::Method(upper.clone()), line);
            let url = trimmed[upper.len()..].trim();
            if !url.is_empty() {
                self.push(Token::Url(url.to_string()), line);
            }
            self.has_request_line = true;
            return true;
        }

        // Check prefixes that unambiguously identify a URL without a method
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("localhost")
            || trimmed.starts_with(':')
        {
            self.push(Token::Url(trimmed.to_string()), line);
            self.has_request_line = true;
            return true;
        }

        false
    }

    /// `Key: Value` — HTTP header (key must have no spaces).
    fn try_header(&mut self, line_str: &str, line: usize) -> bool {
        let Some(colon) = line_str.find(':') else {
            return false;
        };
        let key = line_str[..colon].trim();
        if key.is_empty() || key.contains(' ') {
            return false;
        }
        let value = line_str[colon + 1..].trim().to_string();
        self.push(
            Token::Header {
                key: key.to_string(),
                value,
            },
            line,
        );
        true
    }

    // ── Main entry point ─────────────────────────────────────────────

    /// Classify a single source line and append the resulting token(s).
    fn classify_line(&mut self, raw_line: &str, line: usize) -> Result<(), String> {
        let trimmed = raw_line.trim();

        // Order matters: separator must come before comment (both start with `#`).
        if self.try_separator(trimmed, line) {
            return Ok(());
        }
        if self.try_blank(trimmed, line) {
            return Ok(());
        }
        if self.try_variable(trimmed, line)? {
            return Ok(());
        }
        if self.in_body {
            self.push(Token::BodyLine(raw_line.to_string()), line);
            return Ok(());
        }
        if self.try_comment(trimmed, line) {
            return Ok(());
        }
        if self.try_comment(trimmed, line) {
            return Ok(());
        }
        if self.try_request_line(trimmed, line) {
            return Ok(());
        }
        if self.try_header(trimmed, line) {
            return Ok(());
        }

        if !self.has_request_line {
            self.push(Token::Url(trimmed.to_string()), line);
            self.has_request_line = true;
            return Ok(());
        }

        // Fallback: treat unrecognised lines as body content.
        self.push(Token::BodyLine(raw_line.to_string()), line);
        Ok(())
    }
}

/// Tokenize the contents of a `.reqx` file.
pub fn tokenize(input: &str) -> Result<Vec<LocatedToken>, String> {
    let mut lexer = Lexer::new();

    for (idx, raw_line) in input.lines().enumerate() {
        lexer.classify_line(raw_line, idx + 1)?;
    }

    Ok(lexer.tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_variable() {
        let input = "@base_url = https://api.example.com";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0].token,
            Token::Variable {
                name: "base_url".to_string(),
                value: "https://api.example.com".to_string(),
            }
        );
    }

    #[test]
    fn test_tokenize_separator() {
        let input = "###";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, Token::Separator);
    }

    #[test]
    fn test_tokenize_comment() {
        let input = "# This is a comment";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0].token,
            Token::Comment("This is a comment".to_string())
        );
    }

    #[test]
    fn test_tokenize_method_and_url() {
        let input = "GET https://api.example.com/users";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token, Token::Method("GET".to_string()));
        assert_eq!(
            tokens[1].token,
            Token::Url("https://api.example.com/users".to_string())
        );
    }

    #[test]
    fn test_tokenize_url_only() {
        let input = ":3000/api/status";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, Token::Url(":3000/api/status".to_string()));
    }

    #[test]
    fn test_tokenize_header() {
        let input = "Content-Type: application/json";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0].token,
            Token::Header {
                key: "Content-Type".to_string(),
                value: "application/json".to_string(),
            }
        );
    }

    #[test]
    fn test_tokenize_full_request() {
        let input = r#"@token = abc123

###

# Get users
GET https://api.example.com/users
Authorization: Bearer {{token}}
Accept: application/json

{
  "key": "value"
}"#;
        let tokens = tokenize(input).unwrap();

        // Should contain: Variable, BlankLine, Separator, BlankLine, Comment, Method, Url, Header, Header, BlankLine, BodyLines...
        let mut found_var = false;
        let mut found_sep = false;
        let mut found_comment = false;
        let mut found_method = false;
        let mut found_body = false;
        for t in &tokens {
            match &t.token {
                Token::Variable { name, .. } if name == "token" => found_var = true,
                Token::Separator => found_sep = true,
                Token::Comment(c) if c == "Get users" => found_comment = true,
                Token::Method(m) if m == "GET" => found_method = true,
                Token::BodyLine(_) => found_body = true,
                _ => {}
            }
        }
        assert!(found_var, "should find variable");
        assert!(found_sep, "should find separator");
        assert!(found_comment, "should find comment");
        assert!(found_method, "should find method");
        assert!(found_body, "should find body");
    }
}
