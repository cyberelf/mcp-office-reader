use office_reader_mcp::{stream_pdf_to_markdown, stream_excel_to_markdown, StreamingConfig};
use tokio_stream::StreamExt;
use std::env;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Office Reader MCP Streaming Functionality");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("\n❌ Error: File path required");
        println!("\nUsage: {} <file_path> [chunk_size]", args[0]);
        println!("\nExamples:");
        println!("  {} document.pdf", args[0]);
        println!("  {} spreadsheet.xlsx 5000", args[0]);
        println!("  {} report.docx 15000", args[0]);
        println!("\nSupported formats: .pdf, .xlsx, .xls, .docx");
        std::process::exit(1);
    }
    
    let file_path = &args[1];
    let chunk_size = if args.len() > 2 {
        args[2].parse::<usize>().unwrap_or_else(|_| {
            println!("⚠️  Warning: Invalid chunk size '{}', using default (10000)", args[2]);
            10000
        })
    } else {
        10000 // Default chunk size
    };
    
    // Verify file exists
    if !Path::new(file_path).exists() {
        println!("❌ Error: File '{}' not found", file_path);
        std::process::exit(1);
    }
    
    // Get file info
    let file_metadata = std::fs::metadata(file_path)?;
    let file_size = file_metadata.len();
    let file_extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown");
    
    println!("📄 Processing file: {}", file_path);
    println!("📊 File size: {} bytes ({:.2} MB)", file_size, file_size as f64 / 1_048_576.0);
    println!("📋 File type: .{}", file_extension);
    
    // Configure streaming
    let mut config = StreamingConfig::default();
    config.max_chunk_size_chars = chunk_size;
    
    println!("\n⚙️  Streaming Configuration:");
    println!("   - Chunk size: {} characters", config.max_chunk_size_chars);
    
    // Determine which streaming function to use based on file extension
    let stream_result = match file_extension.to_lowercase().as_str() {
        "pdf" => {
            println!("\n🔄 Starting PDF streaming process...\n");
            let stream = Box::pin(stream_pdf_to_markdown(file_path, config));
            stream_document(stream).await
        }
        "xlsx" | "xls" => {
            println!("\n🔄 Starting Excel streaming process...\n");
            let stream = Box::pin(stream_excel_to_markdown(file_path, config));
            stream_document(stream).await
        }
        _ => {
            println!("❌ Error: Unsupported file type '.{}'", file_extension);
            println!("Supported formats: .pdf, .xlsx, .xls");
            std::process::exit(1);
        }
    };
    
    match stream_result {
        Ok(chunk_count) => {
            println!("🎉 Streaming completed successfully! Processed {} chunks total.", chunk_count);
        }
        Err(e) => {
            println!("❌ Streaming failed: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

async fn stream_document<S>(mut stream: S) -> Result<usize, Box<dyn std::error::Error>>
where
    S: futures::Stream<Item = office_reader_mcp::ProcessingProgress> + Unpin,
{
    let mut chunk_count = 0;
    let mut total_content_length = 0;
    
    while let Some(progress) = stream.next().await {
        chunk_count += 1;
        total_content_length += progress.current_chunk.len();
        
        println!("📦 Chunk #{}", chunk_count);
        println!("   Current position: {}", progress.current_page);
        
        if let Some(total) = progress.total_pages {
            let percentage = (progress.current_page as f64 / total as f64 * 100.0) as u32;
            println!("   Progress: {}% ({}/{})", percentage, progress.current_page, total);
        }
        
        println!("   Content length: {} characters", progress.current_chunk.len());
        println!("   Total processed: {} characters", total_content_length);
        println!("   Is complete: {}", progress.is_complete);
        
        if let Some(error) = &progress.error {
            println!("   ❌ Error: {}", error);
            return Err(error.clone().into());
        } else {
            println!("   ✅ Success");
            // Show first 100 characters of content (character-safe)
            let preview = if progress.current_chunk.chars().count() > 100 {
                let preview_chars: String = progress.current_chunk.chars().take(100).collect();
                format!("{}...", preview_chars)
            } else {
                progress.current_chunk.clone()
            };
            println!("   Preview: {}", preview.replace('\n', " "));
        }
        
        println!();
        
        if progress.is_complete {
            break;
        }
    }
    
    Ok(chunk_count)
} 