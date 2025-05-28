# Office Reader MCP

A Model Context Protocol (MCP) server for reading and converting office documents (Excel, PDF, DOCX) to markdown format with **streaming support** for large files.

## Features

- 📄 **Document Support**: Excel (.xlsx, .xls), PDF (.pdf), and DOCX (.docx) files
- 🚀 **Streaming Mode**: Process large documents in chunks with progress reporting
- 📊 **Progress Tracking**: Real-time progress updates for long-running operations
- 🔄 **Non-blocking**: Asynchronous processing that doesn't freeze the MCP server
- 🛡️ **Error Handling**: Graceful error handling with detailed error messages
- 🌍 **UTF-8 Support**: Proper handling of multi-byte characters (Chinese, Japanese, Arabic, etc.)
- 🔒 **Loop Prevention**: Built-in safeguards against infinite loops in edge cases
- 🧪 **Comprehensive Testing**: Unit tests and end-to-end tests for all functionality

## Tools Available

### 1. `read_office_document`
**Best for**: Small to medium-sized documents
- Processes the entire document at once
- Returns complete markdown content
- Faster for smaller files

**Parameters**:
- `file_path` (string): Path to the office document file

### 2. `stream_office_document` ⭐ **NEW**
**Best for**: Large documents (like textbooks, reports)
- Processes documents in configurable chunks
- Returns progress information with each chunk
- Prevents timeouts on large files
- Provides real-time feedback

**Parameters**:
- `file_path` (string): Path to the office document file
- `chunk_size` (optional number): Maximum characters per chunk (default: 10,000)

## Quick Start

### Installation

```bash
git clone <repository-url>
cd office_reader
cargo build --release
```

### Running the MCP Server

```bash
cargo run
```

### Example Usage

#### Processing a Large PDF with Streaming

```json
{
  "name": "stream_office_document",
  "arguments": {
    "file_path": "/path/to/large-textbook.pdf",
    "chunk_size": 15000
  }
}
```

**Response Format**:
```json
{
  "current_page": 15000,
  "total_pages": 150000,
  "current_chunk": "# Document Title\n\n## Chunk 1 (characters 0-15000)\n\nContent here...",
  "is_complete": false,
  "error": null
}
```

#### Processing a Small Excel File

```json
{
  "name": "read_office_document",
  "arguments": {
    "file_path": "/path/to/spreadsheet.xlsx"
  }
}
```

## Testing

### Run All Tests

```bash
# Unit tests
cargo test --lib

# End-to-end tests
cargo test --test e2e_test

# All tests
cargo test
```

### Test Categories

#### Unit Tests (`src/streaming_parser.rs`)
- ✅ Configuration validation
- ✅ Progress serialization/deserialization
- ✅ Error handling
- ✅ Stream completion
- ✅ Custom chunk sizes

#### End-to-End Tests (`tests/e2e_test.rs`)
- ✅ Excel document streaming
- ✅ PDF document streaming with small chunks
- ✅ Non-existent file handling
- ✅ Unsupported file type handling
- ✅ Default chunk size behavior
- ✅ Tool availability verification

### Test Example

```bash
# Test streaming functionality specifically
cargo test streaming_parser

# Test a specific e2e scenario
cargo test test_stream_excel_document
```

## Streaming Demo

Run the included example to see streaming in action:

```bash
# Show help and usage
cargo run --example test_streaming

# Process a PDF with default chunk size (10,000 characters)
cargo run --example test_streaming document.pdf

# Process an Excel file with custom chunk size
cargo run --example test_streaming spreadsheet.xlsx 5000

# Process a large document with larger chunks
cargo run --example test_streaming large-textbook.pdf 15000
```

**Example Output:**
```
🚀 Testing Office Reader MCP Streaming Functionality
📄 Processing file: document.pdf
📊 File size: 2,450,000 bytes (2.34 MB)
📋 File type: .pdf

⚙️  Streaming Configuration:
   - Chunk size: 15000 characters
   - Pages per chunk: 5

🔄 Starting PDF streaming process...

📦 Chunk #1
   Current position: 14,856
   Progress: 6% (14856/245000)
   Content length: 14,856 characters
   Total processed: 14,856 characters
   Is complete: false
   ✅ Success
   Preview: # Document Title  ## Chunk 1 (characters 0-15000)  This is the beginning of the document...

📦 Chunk #2
   Current position: 29,712
   Progress: 12% (29712/245000)
   ...
```

This will:
1. ✅ **Validate** the file path and type
2. 📊 **Display** file information (size, type)
3. ⚙️ **Configure** streaming with your chosen chunk size
4. 🔄 **Process** the document chunk by chunk
5. 📈 **Show** real-time progress updates
6. 📋 **Preview** content from each chunk

## Configuration

### StreamingConfig Options

```rust
pub struct StreamingConfig {
    pub chunk_size_pages: usize,        // Pages per chunk (for future use)
    pub max_chunk_size_chars: usize,    // Characters per chunk
    pub include_progress: bool,         // Include progress info
}
```

### Default Settings
- **Chunk size**: 10,000 characters
- **Progress reporting**: Enabled
- **Word boundary breaking**: Enabled for better readability

## Performance Benefits

### Before (Traditional Mode)
- ❌ Large files could timeout
- ❌ No progress feedback
- ❌ Memory intensive for huge documents
- ❌ Blocking operation

### After (Streaming Mode)
- ✅ Processes files of any size
- ✅ Real-time progress updates
- ✅ Memory efficient chunking
- ✅ Non-blocking async processing
- ✅ Configurable chunk sizes
- ✅ UTF-8 character boundary safety
- ✅ Infinite loop prevention
- ✅ Word boundary optimization with fallbacks

## Error Handling

The streaming mode provides detailed error information:

```json
{
  "current_page": 0,
  "total_pages": null,
  "current_chunk": "",
  "is_complete": true,
  "error": "Failed to extract text from PDF: File not found"
}
```

## Use Cases

### Perfect for Streaming Mode:
- 📚 **Textbooks** (like "Reinforcement Learning 2ed-Richard Sutton")
- 📊 **Large Reports** (100+ pages)
- 📋 **Complex Spreadsheets** (multiple sheets)
- 📄 **Technical Documentation**

### Traditional Mode Still Good For:
- 📝 **Small Documents** (<50 pages)
- 🔍 **Quick Previews**
- ⚡ **When you need immediate full content**

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   MCP Client    │───▶│  Office Reader   │───▶│  Document       │
│                 │    │  MCP Server      │    │  Processors     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │                         │
                              ▼                         ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │  Streaming       │    │  • PDF Extract  │
                       │  Parser          │    │  • Calamine     │
                       │                  │    │  • DOCX-RS      │
                       └──────────────────┘    └─────────────────┘
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

## License

[Add your license information here] 