# reqx 🚀

**reqx** is a lightweight, intuitive, and highly extensible Domain-Specific Language (DSL) specifically built for defining and executing HTTP requests via text files. 

Inspired by HTTPie and REST Client plugins, `reqx` allows you to write requests as simple plain-text `.reqx` files and execute them instantly via a beautiful CLI without depending on heavyweight UI applications like Postman or Insomnia.

---

## 🌟 Features

*   **Simple & Clean Syntax**: Write requests exactly as they read.
*   **Multiple Requests per file**: Chain multiple requests in the same file separated by `###`.
*   **Variables & Interpolation**: Define local variables (`@base_url = ...`) and inject them easily (`{{base_url}}`).
*   **Implicit HTTP Methods**: Omitting the method name? No problem. It defaults to `GET` automatically, or `POST` if a body payload is provided.
*   **Localhost URL Shorthand**: Just write `:3000/api` and it automatically expands to `http://localhost:3000/api`.
*   **Targeted Execution**: Run only a specific request by index (`-r 2`) or filter by method (`-m POST`).
*   **Dry Run & Verbose**: Inspect exactly what will be sent and received (`--dry-run`, `-v`).
*   **Modular Architecture**: Fully decoupled engine (`reqx-core`) allowing you to embed the lexer/parser in your own apps and provide custom HTTP client implementations.

---

## 📦 Installation

Since `reqx` is built in Rust, you'll need [Cargo](https://rustup.rs/) installed. 

Clone the repository and install the CLI package globally:

```bash
git clone https://github.com/your-username/reqx.git
cd reqx
cargo install --path reqx-cli
```

You can now use the `reqx` command anywhere in your terminal!

---

## ⚡ Quick Start

Create a file named `api.reqx` with the following content:

```reqx
# Variables
@domain = :8080/api/v1
@token = super-secret-jwt

###

# 1. This defaults to GET because there is no body
{{domain}}/health

###

# 2. This defaults to POST automatically because of the JSON body!
{{domain}}/users
Authorization: Bearer {{token}}
Content-Type: application/json

{
  "name": "Jane Doe",
  "email": "jane@example.com"
}

###

# 3. Explicit DELETE method
DELETE {{domain}}/users/123
Authorization: Bearer {{token}}
```

### Running the file

Execute all requests in the file sequentially:
```bash
reqx api.reqx
```

Execute **only** the second request (index is 1-based):
```bash
reqx api.reqx -r 2
```

Execute **only** the `POST` requests:
```bash
reqx api.reqx -m POST
```

See the full request and response headers (Verbose mode):
```bash
reqx api.reqx -v
```

See what would be executed without actually sending network requests:
```bash
reqx api.reqx --dry-run
```

---

## 🏗️ Workspace Architecture

The repository is built as a Cargo Workspace separated into 3 main crates to guarantee perfect decoupling:

1. **`reqx-core`**: The pure parsing and interpreting engine. It contains the Lexer, Parser, AST, and the generic `HttpClient` trait. It has zero knowledge of networking libraries like `reqwest`.
2. **`reqx-cli`**: The binary CLI application. It parses terminal arguments using `clap` and implements the HTTP calls dynamically using `reqwest` (rendering beautiful terminal outputs).
3. **`reqx-examples`**: Contains programmatic usages of the DSL. Want to embed `reqx` into your own application using a custom or mock `HttpClient` instead of `reqwest`? Check out the `main.rs` inside this folder!

---

## 🛡️ License

This project is licensed under the MIT License.
