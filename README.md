# Office Reader MCP

An MCP tool for reading and converting Office documents (Excel, PDF, DOCX) to markdown format, implemented in Rust.

## Features

- Converts Excel spreadsheets to markdown tables
- Extracts text from PDF files
- Converts Word documents to markdown
- Exposes functionality through RMCP (Rust Model Context Protocol)

## Usage

### As a library

```rust
use office_reader_mcp::process_document;

fn main() {
    // Convert an Excel file to markdown
    let markdown = process_document("path/to/document.xlsx");
    println!("{}", markdown);
}
```

### As an MCP server

```bash
# Run the MCP server
cargo run
```

## Supported File Types

- Excel files (.xlsx, .xls)
- PDF files (.pdf)
- Word documents (.docx, .doc)

## Development

To build the project:

```bash
cargo build
```

To run tests:

```bash
cargo test
``` 