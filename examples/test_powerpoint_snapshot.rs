use office_reader_mcp::powerpoint_parser::{generate_slide_snapshot, get_powerpoint_slide_count};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example PowerPoint file path (you'll need to provide a real file)
    let pptx_file = "examples/sample.pptx";
    
    if !Path::new(pptx_file).exists() {
        println!("Sample PowerPoint file not found at: {}", pptx_file);
        println!("Please create a sample PowerPoint file or update the path.");
        return Ok(());
    }
    
    println!("Testing PowerPoint slide snapshot functionality...");
    println!("Using native Rust rendering (no external dependencies required)");
    
    // Get slide count
    match get_powerpoint_slide_count(pptx_file) {
        Ok(count) => {
            println!("PowerPoint file has {} slides", count);
            
            // Generate snapshots for first few slides (or all if <= 5)
            let slides_to_process = std::cmp::min(count, 5);
            
            for slide_num in 1..=slides_to_process {
                println!("\nProcessing slide {}...", slide_num);
                
                // Test PNG format
                let png_result = generate_slide_snapshot(pptx_file, slide_num, "png");
                
                match png_result.error {
                    None => {
                        if let Some(image_data) = png_result.image_data {
                            let filename = format!("slide_{}_snapshot.png", slide_num);
                            fs::write(&filename, &image_data)?;
                            println!("✅ PNG snapshot saved: {} ({} bytes)", filename, image_data.len());
                        }
                    }
                    Some(error) => {
                        println!("❌ PNG generation failed: {}", error);
                        continue;
                    }
                }
                
                // Test JPEG format
                let jpg_result = generate_slide_snapshot(pptx_file, slide_num, "jpg");
                
                match jpg_result.error {
                    None => {
                        if let Some(image_data) = jpg_result.image_data {
                            let filename = format!("slide_{}_snapshot.jpg", slide_num);
                            fs::write(&filename, &image_data)?;
                            println!("✅ JPEG snapshot saved: {} ({} bytes)", filename, image_data.len());
                        }
                    }
                    Some(error) => {
                        println!("❌ JPEG generation failed: {}", error);
                    }
                }
            }
            
            if count > 5 {
                println!("\n💡 Only processed first 5 slides. Total slides: {}", count);
            }
        }
        Err(e) => {
            println!("❌ Failed to get slide count: {}", e);
            return Err(e.into());
        }
    }
    
    // Test error cases
    println!("\n--- Testing Error Cases ---");
    
    // Test invalid slide number
    let invalid_result = generate_slide_snapshot(pptx_file, 999, "png");
    if let Some(error) = invalid_result.error {
        println!("✅ Invalid slide number handled: {}", error);
    }
    
    // Test invalid format
    let invalid_format_result = generate_slide_snapshot(pptx_file, 1, "gif");
    if let Some(error) = invalid_format_result.error {
        println!("✅ Invalid format handled: {}", error);
    }
    
    // Test non-existent file
    let missing_file_result = generate_slide_snapshot("nonexistent.pptx", 1, "png");
    if let Some(error) = missing_file_result.error {
        println!("✅ Missing file handled: {}", error);
    }
    
    println!("\n🎉 PowerPoint snapshot testing completed!");
    println!("Generated images are saved in the current directory.");
    
    Ok(())
} 