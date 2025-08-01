# Office Reader MCP

A high-performance Model Context Protocol (MCP) server for reading and processing various office document formats including PDF, Excel, Word, and PowerPoint files.

## Features

### Document Support
- **PDF Files**: Extract text and render pages as images with multiple backend options
- **Excel Files**: Read spreadsheets with sheet-by-sheet processing
- **Word Documents**: Extract text content from DOCX files
- **PowerPoint Files**: Extract text content and generate slide snapshots as images using native Rust rendering

### PowerPoint Slide Snapshots
The library supports converting PowerPoint slides to images using **native Rust libraries** (no external dependencies required):

```rust
use office_reader_mcp::powerpoint_parser::generate_slide_snapshot;

// Generate PNG snapshot of slide 1
let result = generate_slide_snapshot("presentation.pptx", 1, "png");

match result.error {
    None => {
        if let Some(image_data) = result.image_data {
            std::fs::write("slide1.png", image_data).unwrap();
            println!("Slide snapshot saved as slide1.png");
        }
    }
    Some(error) => println!("Error: {}", error),
}
```

**Key Features:**
- **Native Implementation**: No external dependencies like LibreOffice required
- **Fast Rendering**: Uses `tiny-skia` graphics library for efficient image generation
- **Multiple Formats**: Supports PNG and JPEG output formats
- **Direct PPTX Parsing**: Extracts content directly from PowerPoint XML structure
- **Slide Validation**: Automatically validates slide numbers and file existence

**Supported Output Formats:**
- PNG (recommended for quality)
- JPG/JPEG (smaller file sizes)

### Performance Characteristics

#### PDF Processing
- **pdfium-render** (fastest): ~50-100ms per page
- **mupdf**: ~100-200ms per page  
- **poppler-rs**: ~200-300ms per page
- **pdf-extract** (fallback): ~500ms-2s per page

#### PowerPoint Processing
- **Native Rendering**: ~100-300ms per slide (no external process overhead)
- **Text Extraction**: ~50-100ms per slide
- **Memory Efficient**: Processes slides individually without loading entire presentation

## Installation

### Prerequisites
- **Rust** (latest stable version)
- **No external dependencies** required for PowerPoint functionality

### Optional PDF Backends
For enhanced PDF processing, you can enable optional features:

```toml
[dependencies]
office_reader_mcp = { version = "0.1.0", features = ["pdfium"] }
```

Available features:
- `pdfium`: Google's Pdfium library (fastest PDF processing)
- `mupdf`: MuPDF bindings (very fast)
- `poppler`: Poppler bindings (fast)

## Usage Examples

### PowerPoint Slide Snapshots

```rust
use office_reader_mcp::powerpoint_parser::{generate_slide_snapshot, get_powerpoint_slide_count};

// Get slide count
let slide_count = get_powerpoint_slide_count("presentation.pptx")?;
println!("Presentation has {} slides", slide_count);

// Generate snapshots for all slides
for slide_num in 1..=slide_count {
    let result = generate_slide_snapshot("presentation.pptx", slide_num, "png");
    
    if let Some(image_data) = result.image_data {
        let filename = format!("slide_{}.png", slide_num);
        std::fs::write(&filename, image_data)?;
        println!("Generated {}", filename);
    }
}
```

### Text Extraction

```rust
use office_reader_mcp::powerpoint_parser::process_powerpoint_with_slides;

// Extract text from specific slides
let result = process_powerpoint_with_slides("presentation.pptx", Some("1,3,5".to_string()));

if result.error.is_none() {
    println!("Extracted content:\n{}", result.content);
    println!("Processed slides: {:?}", result.returned_slides);
}
```

## Architecture

### Native PowerPoint Rendering Pipeline

1. **PPTX Parsing**: Direct ZIP archive reading and XML parsing
2. **Content Extraction**: Parse slide XML to extract text, shapes, and layout information
3. **Graphics Rendering**: Use `tiny-skia` to render slide content to raster images
4. **Format Conversion**: Output as PNG or JPEG using the `image` crate

### Benefits of Native Approach

- **No External Dependencies**: Eliminates LibreOffice requirement
- **Better Performance**: No process spawning overhead
- **Cross-Platform**: Works consistently across Windows, macOS, and Linux
- **Memory Efficient**: Processes slides individually
- **Deterministic**: Consistent rendering across environments

## Error Handling

The library provides comprehensive error handling:

```rust
let result = generate_slide_snapshot("presentation.pptx", 1, "png");

match result.error {
    None => println!("Success!"),
    Some(error) => {
        if error.contains("not found") {
            println!("File not found");
        } else if error.contains("does not exist") {
            println!("Invalid slide number");
        } else {
            println!("Rendering error: {}", error);
        }
    }
}
```

## Limitations

### Current PowerPoint Rendering Limitations

- **Text Rendering**: Currently renders text as placeholder rectangles (full text rendering requires additional font libraries)
- **Complex Layouts**: Advanced PowerPoint features like animations, transitions, and complex shapes have basic support
- **Embedded Media**: Videos and audio are not rendered
- **Font Handling**: Uses default fonts (can be extended with `fontdb` integration)

### Future Enhancements

- Full text rendering with proper fonts
- Enhanced shape and image support
- Animation frame extraction
- SVG output format
- Custom styling options

## Contributing

Contributions are welcome! Areas for improvement:

1. Enhanced text rendering with proper font support
2. Better shape and image extraction from PPTX files
3. Performance optimizations
4. Additional output formats (SVG, WebP)
5. Advanced layout handling

## License

This project is licensed under the MIT License. 