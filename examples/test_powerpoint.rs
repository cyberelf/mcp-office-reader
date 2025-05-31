use office_reader_mcp::{
    process_powerpoint_with_slides,
    get_powerpoint_slide_info,
    generate_slide_snapshot,
};

fn main() {
    // Example PowerPoint file path (you would replace this with an actual file)
    let ppt_file = "sample.pptx";
    
    println!("=== PowerPoint Reader Example ===\n");
    
    // 1. Get slide information
    println!("1. Getting slide information...");
    let slide_info = get_powerpoint_slide_info(ppt_file);
    
    if let Some(error) = &slide_info.error {
        println!("   Error: {}", error);
        if error == "file_not_found" {
            println!("   Note: Create a sample.pptx file to test this functionality");
        }
    } else {
        println!("   File: {}", slide_info.file_path);
        if let Some(total) = slide_info.total_slides {
            println!("   Total slides: {}", total);
        }
        println!("   Info: {}", slide_info.slide_info);
    }
    
    println!();
    
    // 2. Read all slides
    println!("2. Reading all slides...");
    let all_slides_result = process_powerpoint_with_slides(ppt_file, Some("all".to_string()));
    
    if let Some(error) = &all_slides_result.error {
        println!("   Error: {}", error);
    } else {
        println!("   Successfully extracted content from {} slides", 
                all_slides_result.returned_slides.len());
        println!("   Content preview (first 200 chars):");
        let preview = if all_slides_result.content.len() > 200 {
            format!("{}...", &all_slides_result.content[..200])
        } else {
            all_slides_result.content.clone()
        };
        println!("   {}", preview);
    }
    
    println!();
    
    // 3. Read specific slides
    println!("3. Reading specific slides (1,3)...");
    let specific_slides_result = process_powerpoint_with_slides(ppt_file, Some("1,3".to_string()));
    
    if let Some(error) = &specific_slides_result.error {
        println!("   Error: {}", error);
    } else {
        println!("   Successfully extracted content from slides: {:?}", 
                specific_slides_result.returned_slides);
        println!("   Requested: {}", specific_slides_result.requested_slides);
    }
    
    println!();
    
    // 4. Generate slide snapshot (placeholder)
    println!("4. Generating slide snapshot...");
    let snapshot_result = generate_slide_snapshot(ppt_file, 1, "png");
    
    if let Some(error) = &snapshot_result.error {
        println!("   Error: {}", error);
    } else {
        println!("   Successfully generated snapshot for slide {}", snapshot_result.slide_number);
        if let Some(data) = &snapshot_result.image_data {
            println!("   Image size: {} bytes", data.len());
        }
    }
    
    println!("\n=== Example completed ===");
    
    // Show supported features
    println!("\nSupported PowerPoint features:");
    println!("✓ Text extraction from PPTX files");
    println!("✓ Slide-specific content retrieval");
    println!("✓ Slide count and information");
    println!("✓ Page/slide range selection (e.g., '1,3,5-7')");
    println!("○ Slide snapshot generation (requires external tools)");
    println!("○ PPT (legacy) format support (limited)");
} 