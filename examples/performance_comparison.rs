use office_reader_mcp::fast_pdf_extractor::FastPdfExtractor;
use std::time::Instant;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Office Reader MCP - PDF Performance Comparison");
    println!("==============================================");
    
    // Test with a sample PDF file (you'll need to provide a real PDF file path)
    let pdf_path = "sample/sample.pdf"; // Replace with actual PDF file path
    
    if !Path::new(pdf_path).exists() {
        println!("Please provide a valid PDF file path in the example");
        println!("Current test path: {}", pdf_path);
        println!("\nTo test with your own PDF:");
        println!("1. Place a PDF file named 'sample.pdf' in the project root");
        println!("2. Or modify the pdf_path variable in this example");
        return Ok(());
    }
    
    println!("Testing PDF: {}", pdf_path);
    
    // Get file size for context
    let file_size = std::fs::metadata(pdf_path)?.len();
    println!("File size: {:.2} MB", file_size as f64 / 1024.0 / 1024.0);
    
    println!("\n=== Available PDF Backends ===");
    let backend_info = FastPdfExtractor::get_backend_info();
    for (backend, description, available) in backend_info {
        println!("{:?}: {} - {}", backend, description, if available { "Available" } else { "Not Available" });
    }
    
    println!("\n=== Performance Test ===");
    
    // Test with FastPdfExtractor (automatic backend selection)
    println!("\n1. Testing FastPdfExtractor (automatic backend selection):");
    let start_time = Instant::now();
    match FastPdfExtractor::extract_text(pdf_path) {
        Ok(text) => {
            let duration = start_time.elapsed();
            println!("   ✓ Success in {:?}", duration);
            println!("   ✓ Extracted {} characters", text.len());
            println!("   ✓ Speed: {:.2} MB/s", (file_size as f64 / 1024.0 / 1024.0) / duration.as_secs_f64());
        }
        Err(e) => {
            println!("   ✗ Failed: {}", e);
        }
    }
    
    // Test with pdf-extract for comparison (if available)
    println!("\n2. Testing pdf-extract (fallback library):");
    let start_time = Instant::now();
    match pdf_extract::extract_text(pdf_path) {
        Ok(text) => {
            let duration = start_time.elapsed();
            println!("   ✓ Success in {:?}", duration);
            println!("   ✓ Extracted {} characters", text.len());
            println!("   ✓ Speed: {:.2} MB/s", (file_size as f64 / 1024.0 / 1024.0) / duration.as_secs_f64());
        }
        Err(e) => {
            println!("   ✗ Failed: {}", e);
        }
    }
    
    println!("\n=== Memory Usage Test ===");
    
    // Test memory usage with multiple extractions
    println!("Testing memory efficiency with 5 consecutive extractions...");
    
    let start_time = Instant::now();
    for i in 1..=5 {
        match FastPdfExtractor::extract_text(pdf_path) {
            Ok(text) => {
                println!("   Extraction {}: {} characters", i, text.len());
            }
            Err(e) => {
                println!("   Extraction {} failed: {}", i, e);
                break;
            }
        }
    }
    let total_duration = start_time.elapsed();
    println!("Total time for 5 extractions: {:?}", total_duration);
    println!("Average time per extraction: {:?}", total_duration / 5);
    
    println!("\n=== Recommendations ===");
    println!("For best performance with large PDFs (>50MB):");
    println!("1. Use 'pdfium' feature (default) - Google's Pdfium library");
    println!("2. For very large files, consider 'mupdf_backend' feature");
    println!("3. Avoid pdf-extract for large files (it's very slow)");
    println!("\nTo enable different backends:");
    println!("cargo build --features mupdf_backend");
    println!("cargo build --features poppler");
    
    Ok(())
} 