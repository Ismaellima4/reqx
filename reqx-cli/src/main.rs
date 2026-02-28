use clap::Parser;
use colored::Colorize;
use std::fs;
use std::process;

use reqx_core::interpreter;
use reqx_core::lexer;
use reqx_core::parser;

mod reqwest_client;

/// reqx — Execute HTTP requests defined in .reqx files
#[derive(Parser, Debug)]
#[command(name = "reqx", version, about = "A DSL interpreter for HTTP requests")]
struct Cli {
    /// Path to the .reqx file to execute
    file: String,

    /// Show verbose output (headers, body details)
    #[arg(short, long)]
    verbose: bool,

    /// Show requests without actually sending them
    #[arg(short, long)]
    dry_run: bool,

    /// Execute only the request at the specified index (1-based)
    #[arg(short = 'r', long = "request")]
    request_index: Option<usize>,

    /// Execute only requests that match this HTTP method (e.g., GET, POST)
    #[arg(short = 'm', long = "method")]
    method_filter: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    // Read the input file
    let contents = match fs::read_to_string(&cli.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "{} Error reading file '{}': {}",
                "✖".red().bold(),
                cli.file.bold(),
                e
            );
            process::exit(1);
        }
    };

    // Tokenize
    let tokens = match lexer::tokenize(&contents) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{} Lexer error: {}", "✖".red().bold(), e);
            process::exit(1);
        }
    };

    // Parse
    let reqx_file = match parser::parse(tokens) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{} Parser error: {}", "✖".red().bold(), e);
            process::exit(1);
        }
    };

    // Execute
    let client = reqwest_client::ReqwestClient::new();
    if let Err(e) = interpreter::execute(
        &client,
        &reqx_file,
        cli.verbose,
        cli.dry_run,
        cli.request_index,
        cli.method_filter,
    ) {
        eprintln!("{} Execution error: {}", "✖".red().bold(), e);
        process::exit(1);
    }
}
