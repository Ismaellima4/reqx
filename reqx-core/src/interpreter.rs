/// Interpreter: resolves variables and executes HTTP requests.
use crate::ast::{HttpMethod, Request, ReqxFile};
use crate::client::HttpClient;
use colored::Colorize;
use std::collections::HashMap;

/// Execute all requests or a specific request in a `ReqxFile`.
pub fn execute<C: HttpClient>(
    client: &C,
    file: &ReqxFile,
    verbose: bool,
    dry_run: bool,
    request_index: Option<usize>,
    method_filter: Option<String>,
) -> Result<(), String> {
    // Build variable map
    let mut vars: HashMap<String, String> = HashMap::new();
    for var in &file.variables {
        vars.insert(var.name.clone(), var.value.clone());
    }

    if verbose {
        println!("{}", "── Variables ──".dimmed());
        for (k, v) in &vars {
            println!("  {} = {}", k.cyan(), v);
        }
        println!();
    }

    let total = file.requests.len();

    let mut requests_to_run: Vec<(usize, &Request)> = match request_index {
        Some(idx) => {
            if idx == 0 || idx > total {
                return Err(format!(
                    "Invalid request index: {}. The file has {} request(s).",
                    idx, total
                ));
            }
            vec![(idx - 1, &file.requests[idx - 1])]
        }
        None => file.requests.iter().enumerate().collect(),
    };

    if let Some(m_str) = method_filter {
        let target_method = m_str
            .parse::<HttpMethod>()
            .ok()
            .ok_or_else(|| format!("Invalid HTTP method filter: {}", m_str))?;

        requests_to_run.retain(|(_, req)| req.method == target_method);

        if requests_to_run.is_empty() {
            println!(
                "{}",
                format!("No requests matched the method filter: {}", m_str).dimmed()
            );
            return Ok(());
        }
    }

    for (i, req) in requests_to_run {
        println!(
            "{}",
            format!("━━━ Request {}/{} ━━━", i + 1, total).bold().blue()
        );

        if let Some(ref comment) = req.comment {
            println!("{} {}", "▸".green(), comment.bold());
        }

        execute_request(client, req, &vars, verbose, dry_run)?;
        println!();
    }

    Ok(())
}

fn parse_variable_name(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
) -> Result<String, String> {
    let mut var_name = String::new();
    loop {
        match chars.next() {
            Some('}') if chars.peek() == Some(&'}') => {
                chars.next(); // consume second '}'
                break;
            }
            Some(c) => var_name.push(c),
            None => {
                return Err(format!(
                    "Unclosed variable interpolation: {{{{{}}}",
                    var_name
                ))
            }
        }
    }
    Ok(var_name.trim().to_string())
}

/// Interpolate `{{var}}` placeholders in a string.
fn interpolate(s: &str, vars: &HashMap<String, String>) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let var_name = parse_variable_name(&mut chars)?;

            let val = vars
                .get(&var_name)
                .ok_or_else(|| format!("Undefined variable: {}", var_name))?;
            result.push_str(val);
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn expand_url(url: &str) -> String {
    if url.starts_with(':') {
        format!("http://localhost{}", url)
    } else {
        url.to_string()
    }
}

fn execute_request<C: HttpClient>(
    client: &C,
    req: &Request,
    vars: &HashMap<String, String>,
    verbose: bool,
    dry_run: bool,
) -> Result<(), String> {
    let interpolated_url = interpolate(&req.url, vars)?;
    let url = expand_url(&interpolated_url);

    let mut resolved_headers = Vec::new();
    for h in &req.headers {
        let key = interpolate(&h.key, vars)?;
        let value = interpolate(&h.value, vars)?;
        resolved_headers.push((key, value));
    }

    let body = match &req.body {
        Some(b) => Some(interpolate(b, vars)?),
        None => None,
    };

    // Display the request
    let method_colored = match req.method {
        HttpMethod::Get => "GET".green().bold(),
        HttpMethod::Post => "POST".yellow().bold(),
        HttpMethod::Put => "PUT".blue().bold(),
        HttpMethod::Patch => "PATCH".magenta().bold(),
        HttpMethod::Delete => "DELETE".red().bold(),
        HttpMethod::Head => "HEAD".cyan().bold(),
        HttpMethod::Options => "OPTIONS".white().bold(),
    };

    println!("{} {}", method_colored, url.underline());

    if verbose {
        for (k, v) in &resolved_headers {
            println!("  {}: {}", k.dimmed(), v);
        }
        if let Some(ref b) = body {
            println!("  {}", "Body:".dimmed());
            // Try to pretty-print JSON bodies
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(b) {
                let pretty = serde_json::to_string_pretty(&json).unwrap_or_else(|_| b.clone());
                for line in pretty.lines() {
                    println!("    {}", line);
                }
            } else {
                for line in b.lines() {
                    println!("    {}", line);
                }
            }
        }
    }

    if dry_run {
        println!("{}", "  (dry-run: request not sent)".dimmed().italic());
        return Ok(());
    }

    // Actually execute the request
    let response = client.execute(&req.method, &url, &resolved_headers, body.as_deref())?;

    // Display response
    let status = response.status;
    let status_colored = if response.status_is_success {
        format!("{}", status).green().bold()
    } else if response.status_is_client_error {
        format!("{}", status).yellow().bold()
    } else if response.status_is_server_error {
        format!("{}", status).red().bold()
    } else {
        format!("{}", status).white().bold()
    };

    println!("  {} {}", "Status:".dimmed(), status_colored);

    if verbose {
        println!("  {}", "Response Headers:".dimmed());
        for (k, v) in &response.headers {
            println!("    {}: {}", k.as_str().dimmed(), v.as_str());
        }
    }

    // Print response body
    let resp_body = &response.body;

    if !resp_body.is_empty() {
        // Try to pretty-print JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(resp_body) {
            let pretty = serde_json::to_string_pretty(&json).unwrap_or_else(|_| resp_body.clone());
            println!("  {}", "Response Body:".dimmed());
            for line in pretty.lines() {
                println!("    {}", line);
            }
        } else {
            println!("  {}", "Response Body:".dimmed());
            // Limit output for very large responses
            let max_lines = 50;
            let lines: Vec<&str> = resp_body.lines().collect();
            for line in lines.iter().take(max_lines) {
                println!("    {}", line);
            }
            if lines.len() > max_lines {
                println!(
                    "    {}",
                    format!("... ({} more lines)", lines.len() - max_lines).dimmed()
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_basic() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "world".to_string());
        let result = interpolate("hello {{name}}!", &vars).unwrap();
        assert_eq!(result, "hello world!");
    }

    #[test]
    fn test_interpolate_multiple() {
        let mut vars = HashMap::new();
        vars.insert("base".to_string(), "https://api.example.com".to_string());
        vars.insert("version".to_string(), "v2".to_string());
        let result = interpolate("{{base}}/{{version}}/users", &vars).unwrap();
        assert_eq!(result, "https://api.example.com/v2/users");
    }

    #[test]
    fn test_interpolate_undefined_var() {
        let vars = HashMap::new();
        let result = interpolate("hello {{missing}}", &vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_interpolate_no_vars() {
        let vars = HashMap::new();
        let result = interpolate("no interpolation here", &vars).unwrap();
        assert_eq!(result, "no interpolation here");
    }

    #[test]
    fn test_expand_url_localhost_shorthand() {
        assert_eq!(expand_url(":3000"), "http://localhost:3000");
        assert_eq!(
            expand_url(":8080/api/users"),
            "http://localhost:8080/api/users"
        );
        assert_eq!(expand_url("https://api.com"), "https://api.com");
        assert_eq!(expand_url("http://127.0.0.1:8000"), "http://127.0.0.1:8000");
    }
}
