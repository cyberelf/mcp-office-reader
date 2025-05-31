# Office Reader MCP

A Model Context Protocol (MCP) server for reading and parsing office documents. Supports Excel, PDF, DOCX, and PowerPoint files with streaming capabilities and page-based content extraction.

## Features

- **Excel Files (.xlsx, .xls)**: Extract content from specific sheets with sheet-based selection
- **PDF Files**: Fast text extraction with page-specific selection using multiple backends (Pdfium, MuPDF, Poppler)
- **Word Documents (.docx, .doc)**: Extract text content with page estimation
- **PowerPoint Presentations (.pptx, .ppt)**: Extract text from specific slides with slide-based selection
- **Streaming Support**: Process large documents in chunks with progress tracking
- **Page/Slide Selection**: Flexible selection syntax (e.g., "1,3,5-7", "all")
- **Caching**: Intelligent PDF content caching for improved performance

## Supported File Types

| Format | Extension | Features | Page/Slide Support |
|--------|-----------|----------|-------------------|
| Excel | .xlsx, .xls | Sheet extraction, table formatting | ✓ (sheets) |
| PDF | .pdf | Fast text extraction, multiple backends | ✓ (pages) |
| Word | .docx, .doc | Text extraction, page estimation | ✓ (estimated) |
| PowerPoint | .pptx, .ppt | Text extraction, slide selection | ✓ (slides) |

## PowerPoint Support

The PowerPoint parser provides comprehensive support for PPTX files:

### Features
- **Text Extraction**: Extract text content from slides using manual XML parsing
- **Slide Selection**: Support for specific slide selection (e.g., "1,3,5-7")
- **Slide Information**: Get slide count and metadata without reading full content
- **Slide Snapshots**: Placeholder for image generation (requires external tools)

### Usage Examples

```rust
use office_reader_mcp::{
    process_powerpoint_with_slides,
    get_powerpoint_slide_info,
    generate_slide_snapshot,
};

// Get slide information
let slide_info = get_powerpoint_slide_info("presentation.pptx");
println!("Total slides: {:?}", slide_info.total_slides);

// Extract all slides
let all_slides = process_powerpoint_with_slides("presentation.pptx", Some("all".to_string()));

// Extract specific slides
let specific_slides = process_powerpoint_with_slides("presentation.pptx", Some("1,3,5-7".to_string()));

// Generate slide snapshot (placeholder)
let snapshot = generate_slide_snapshot("presentation.pptx", 1, "png");
```

### MCP Tools for PowerPoint

1. **`get_powerpoint_slide_info`**: Get slide count and information
2. **`read_powerpoint_slides`**: Extract text from specific slides
3. **`generate_powerpoint_slide_snapshot`**: Generate slide images (requires external tools)

### Slide Selection Syntax

- `"all"` - All slides
- `"1"` - Single slide
- `"1,3,5"` - Multiple specific slides
- `"1-5"` - Range of slides
- `"1,3,5-7"` - Mixed selection

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
office_reader_mcp = "0.1.0"
```

## MCP Tools

The server provides the following tools:

### Document Processing
- `get_document_page_info`: Get page/slide information without reading full content
- `read_office_document`: Read documents with page/slide selection
- `stream_office_document`: Stream document content in chunks

### PowerPoint Specific
- `get_powerpoint_slide_info`: Get PowerPoint slide information
- `read_powerpoint_slides`: Read PowerPoint slides with selection
- `generate_powerpoint_slide_snapshot`: Generate slide snapshots

## Examples

Run the PowerPoint example:
```bash
cargo run --example test_powerpoint
```

## Dependencies

- **Core**: `rmcp`, `anyhow`, `serde`, `tokio`
- **Excel**: `calamine`
- **PDF**: `pdf-extract`, `lopdf`, `pdfium-render` (optional), `mupdf` (optional), `poppler-rs` (optional)
- **Word**: `docx-rs`
- **PowerPoint**: `zip`, `quick-xml`

## License

This project is licensed under the MIT License. 