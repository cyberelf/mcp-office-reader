use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

use anyhow::{Result, Context};
use zip::ZipArchive;
use quick_xml::Reader;
use quick_xml::events::Event;
use crate::cache_system::CacheManager;
use crate::impl_cacheable_content;

/// Cache for storing extracted PowerPoint content
#[derive(Debug, Clone)]
pub struct PowerPointCache {
    pub content: String,
    pub char_indices: Vec<usize>,
    pub total_slides: Option<usize>,
    pub slide_texts: HashMap<usize, String>,
}

// Implement CacheableContent for PowerPointCache
impl_cacheable_content!(PowerPointCache, content, char_indices, total_slides);


lazy_static::lazy_static! {
    /// Global PowerPoint cache manager
    pub static ref POWERPOINT_CACHE_MANAGER: CacheManager<PowerPointCache> = CacheManager::new();
}

/// PowerPoint slide snapshot result
#[derive(Debug, Clone)]
pub struct SlideSnapshotResult {
    pub slide_number: usize,
    pub image_data: Option<Vec<u8>>,
    pub image_format: String,
    pub error: Option<String>,
}

/// PowerPoint page information result
#[derive(Debug, Clone)]
pub struct PowerPointPageInfoResult {
    pub file_path: String,
    pub total_slides: Option<usize>,
    pub slide_info: String,
    pub error: Option<String>,
}

/// Result of PowerPoint processing with slide-based support
#[derive(Debug, Clone)]
pub struct PowerPointProcessingResult {
    pub content: String,
    pub total_slides: Option<usize>,
    pub requested_slides: String,
    pub returned_slides: Vec<usize>,
    pub file_path: String,
    pub slide_texts: HashMap<usize, String>,
    pub error: Option<String>,
}

/// Slide content structure for rendering
#[derive(Debug, Clone)]
pub struct SlideContent {
    pub title: Option<String>,
    pub text_elements: Vec<TextElement>,
    pub images: Vec<ImageElement>,
    pub shapes: Vec<ShapeElement>,
    pub background: Option<Background>,
}

#[derive(Debug, Clone)]
pub struct TextElement {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub font_family: String,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone)]
pub struct ImageElement {
    pub data: Vec<u8>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct ShapeElement {
    pub shape_type: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub stroke_width: f32,
}

#[derive(Debug, Clone)]
pub struct Background {
    pub color: Option<String>,
    pub image: Option<Vec<u8>>,
}

impl PowerPointProcessingResult {
    /// Create a new result for successful processing
    pub fn success(
        content: String,
        total_slides: Option<usize>,
        requested_slides: String,
        returned_slides: Vec<usize>,
        file_path: String,
        slide_texts: HashMap<usize, String>,
    ) -> Self {
        Self {
            content,
            total_slides,
            requested_slides,
            returned_slides,
            file_path,
            slide_texts,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(file_path: String, error: String) -> Self {
        Self {
            content: error.clone(),
            total_slides: None,
            requested_slides: String::new(),
            returned_slides: Vec::new(),
            file_path,
            slide_texts: HashMap::new(),
            error: Some(error),
        }
    }
}

impl PowerPointPageInfoResult {
    /// Create a new result for successful page info retrieval
    pub fn success(
        file_path: String,
        total_slides: Option<usize>,
        slide_info: String,
    ) -> Self {
        Self {
            file_path,
            total_slides,
            slide_info,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(file_path: String, error: String) -> Self {
        Self {
            file_path,
            total_slides: None,
            slide_info: String::new(),
            error: Some(error),
        }
    }

    /// Check if the file exists (no error or error is not file_not_found)
    pub fn file_exists(&self) -> bool {
        self.error.as_ref() != Some(&"file_not_found".to_string())
    }
}

impl SlideSnapshotResult {
    /// Create a new result for successful snapshot
    pub fn success(
        slide_number: usize,
        image_data: Vec<u8>,
        image_format: String,
    ) -> Self {
        Self {
            slide_number,
            image_data: Some(image_data),
            image_format,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(slide_number: usize, error: String) -> Self {
        Self {
            slide_number,
            image_data: None,
            image_format: String::new(),
            error: Some(error),
        }
    }
}

/// Function to extract PowerPoint content and create cache
fn extract_powerpoint_content(file_path: &str) -> Result<PowerPointCache> {
    let (all_text, slide_texts) = extract_powerpoint_text_manual(file_path)?;
    let total_slides = slide_texts.len();
    
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    markdown.push_str(&all_text);
    
    // Pre-compute character byte indices for efficient slicing
    let mut char_indices = Vec::new();
    let mut byte_pos = 0;
    
    for ch in markdown.chars() {
        char_indices.push(byte_pos);
        byte_pos += ch.len_utf8();
    }
    char_indices.push(byte_pos);
    
    Ok(PowerPointCache {
        content: markdown,
        char_indices,
        total_slides: Some(total_slides),
        slide_texts,
    })
}

/// Function to extract specific slides from PowerPoint
fn extract_powerpoint_slides(file_path: &str, slide_numbers: &[usize]) -> Result<String> {
    let (_, slide_texts) = extract_powerpoint_text_manual(file_path)?;
    
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    
    for &slide_number in slide_numbers {
        if let Some(slide_text) = slide_texts.get(&slide_number) {
            if !slide_text.trim().is_empty() {
                markdown.push_str(&format!("## Slide {}\n\n{}\n\n", slide_number, slide_text));
            }
        }
    }
    
    Ok(markdown)
}

/// Extract text from PowerPoint file by manually parsing PPTX structure
pub fn extract_powerpoint_text_manual(file_path: &str) -> Result<(String, HashMap<usize, String>)> {
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open PowerPoint file: {}", file_path))?;
    
    let mut archive = ZipArchive::new(file)
        .with_context(|| "Failed to read PowerPoint file as ZIP archive")?;
    
    let mut slide_texts = HashMap::new();
    let mut all_text = String::new();
    
    // Find all slide files
    let slide_files: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    Some(name.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    
    // Sort slide files to ensure proper order
    let mut sorted_slides = slide_files;
    sorted_slides.sort_by(|a, b| {
        let a_num = extract_slide_number(a);
        let b_num = extract_slide_number(b);
        a_num.cmp(&b_num)
    });
    
    // Extract text from each slide
    for (index, slide_file) in sorted_slides.iter().enumerate() {
        let slide_number = index + 1;
        
        if let Ok(mut file) = archive.by_name(slide_file) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                let slide_text = extract_text_from_slide_xml(&contents)?;
                slide_texts.insert(slide_number, slide_text.clone());
                
                if !slide_text.trim().is_empty() {
                    all_text.push_str(&format!("## Slide {}\n\n{}\n\n", slide_number, slide_text));
                }
            }
        }
    }
    
    Ok((all_text, slide_texts))
}

/// Extract slide number from slide file name
fn extract_slide_number(filename: &str) -> usize {
    // Extract number from "ppt/slides/slide1.xml" format
    if let Some(start) = filename.rfind("slide") {
        if let Some(end) = filename.rfind(".xml") {
            let number_str = &filename[start + 5..end];
            return number_str.parse().unwrap_or(0);
        }
    }
    0
}

/// Extract text content from slide XML
fn extract_text_from_slide_xml(xml_content: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);
    
    let mut text_content = String::new();
    let mut in_text_element = false;
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"a:t" => in_text_element = true,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"a:t" => in_text_element = false,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    let text = std::str::from_utf8(&e).unwrap_or_default();
                    text_content.push_str(&text);
                    text_content.push(' ');
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!("Error parsing slide XML: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    
    Ok(text_content.trim().to_string())
}

/// Get PowerPoint slide count
pub fn get_powerpoint_slide_count(file_path: &str) -> Result<usize> {
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open PowerPoint file: {}", file_path))?;
    
    let mut archive = ZipArchive::new(file)
        .with_context(|| "Failed to read PowerPoint file as ZIP archive")?;
    
    // Count slide files
    let slide_count = (0..archive.len())
        .filter(|&i| {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                name.starts_with("ppt/slides/slide") && name.ends_with(".xml")
            } else {
                false
            }
        })
        .count();
    
    Ok(slide_count)
}

/// Generate slide snapshot using native Rust graphics libraries
/// Expects a resolved file path
pub fn generate_slide_snapshot(
    resolved_file_path: &str,
    slide_number: usize,
    output_format: &str,
) -> SlideSnapshotResult {
    // Validate input parameters
    if slide_number == 0 {
        return SlideSnapshotResult::error(
            slide_number,
            "Slide number must be greater than 0".to_string(),
        );
    }
    
    let supported_formats = ["png", "jpg", "jpeg"];
    if !supported_formats.contains(&output_format.to_lowercase().as_str()) {
        return SlideSnapshotResult::error(
            slide_number,
            format!("Unsupported format '{}'. Supported formats: {}", output_format, supported_formats.join(", ")),
        );
    }
    
    // Check if file exists
    if !Path::new(resolved_file_path).exists() {
        return SlideSnapshotResult::error(
            slide_number,
            format!("PowerPoint file not found: {}", resolved_file_path),
        );
    }
    
    // Get total slide count to validate slide number
    let total_slides = match get_powerpoint_slide_count(resolved_file_path) {
        Ok(count) => count,
        Err(e) => return SlideSnapshotResult::error(
            slide_number,
            format!("Failed to get slide count: {}", e),
        ),
    };
    
    if slide_number > total_slides {
        return SlideSnapshotResult::error(
            slide_number,
            format!("Slide {} does not exist. File has {} slides", slide_number, total_slides),
        );
    }
    
    // Parse slide content and render to image
    match parse_and_render_slide(resolved_file_path, slide_number, output_format) {
        Ok(image_data) => SlideSnapshotResult::success(
            slide_number,
            image_data,
            output_format.to_string(),
        ),
        Err(e) => SlideSnapshotResult::error(
            slide_number,
            format!("Failed to render slide: {}", e),
        ),
    }
}

/// Parse slide content and render it to an image
fn parse_and_render_slide(
    file_path: &str,
    slide_number: usize,
    output_format: &str,
) -> Result<Vec<u8>> {
    // Parse slide content
    let slide_content = parse_slide_content(file_path, slide_number)?;
    
    // Render slide to image
    render_slide_to_image(&slide_content, output_format)
}

/// Parse slide content from PPTX file
fn parse_slide_content(file_path: &str, slide_number: usize) -> Result<SlideContent> {
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;
    
    // Find the specific slide file
    let slide_file_name = format!("ppt/slides/slide{}.xml", slide_number);
    let mut slide_file = archive.by_name(&slide_file_name)
        .with_context(|| format!("Slide {} not found", slide_number))?;
    
    let mut slide_xml = String::new();
    slide_file.read_to_string(&mut slide_xml)?;
    
    // Drop the slide_file to release the mutable borrow
    drop(slide_file);
    
    // Parse slide XML to extract content
    parse_slide_xml(&slide_xml, &mut archive)
}

/// Parse slide XML content
fn parse_slide_xml(xml_content: &str, _archive: &mut ZipArchive<File>) -> Result<SlideContent> {
    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);
    
    let mut slide_content = SlideContent {
        title: None,
        text_elements: Vec::new(),
        images: Vec::new(),
        shapes: Vec::new(),
        background: None,
    };
    
    let mut buf = Vec::new();
    let mut current_text = String::new();
    let mut in_text_element = false;
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"a:t" => {
                        in_text_element = true;
                        current_text.clear();
                    }
                    b"p:sp" => {
                        // Shape element - could be text box, shape, etc.
                    }
                    b"a:blip" => {
                        // Image element
                        if let Some(embed_attr) = e.attributes().find(|attr| {
                            attr.as_ref().map(|a| a.key.as_ref() == b"r:embed").unwrap_or(false)
                        }) {
                            if let Ok(attr) = embed_attr {
                                let _embed_id = String::from_utf8_lossy(&attr.value);
                                // TODO: Extract image from relationships
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"a:t" => {
                        in_text_element = false;
                        if !current_text.trim().is_empty() {
                            // Create a text element with default positioning
                            slide_content.text_elements.push(TextElement {
                                text: current_text.clone(),
                                x: 50.0,
                                y: 50.0 + (slide_content.text_elements.len() as f32 * 30.0),
                                width: 600.0,
                                height: 25.0,
                                font_size: 18.0,
                                font_family: "Arial".to_string(),
                                color: "#000000".to_string(),
                                bold: false,
                                italic: false,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    let text = std::str::from_utf8(&e).unwrap_or_default();
                    current_text.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!("Error parsing slide XML: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    
    Ok(slide_content)
}

/// Render slide content to image using tiny-skia
fn render_slide_to_image(slide_content: &SlideContent, output_format: &str) -> Result<Vec<u8>> {
    use tiny_skia::*;
    
    // Standard slide dimensions (16:9 aspect ratio)
    let width = 1920;
    let height = 1080;
    
    let mut pixmap = Pixmap::new(width, height)
        .ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;
    
    // Fill background
    let background_color = if let Some(ref bg) = slide_content.background {
        parse_color(bg.color.as_deref().unwrap_or("#FFFFFF"))
    } else {
        Color::WHITE
    };
    
    pixmap.fill(background_color);
    
    // Render text elements
    for text_element in &slide_content.text_elements {
        render_text_element(&mut pixmap, text_element)?;
    }
    
    // Render shapes
    for shape_element in &slide_content.shapes {
        render_shape_element(&mut pixmap, shape_element)?;
    }
    
    // Convert to output format
    match output_format.to_lowercase().as_str() {
        "png" => {
            Ok(pixmap.encode_png()?)
        }
        "jpg" | "jpeg" => {
            // Convert to RGB and then to JPEG
            let rgb_data = pixmap_to_rgb(&pixmap);
            encode_jpeg(&rgb_data, width, height)
        }
        _ => Err(anyhow::anyhow!("Unsupported format: {}", output_format)),
    }
}

/// Render text element on the pixmap
fn render_text_element(pixmap: &mut tiny_skia::Pixmap, text_element: &TextElement) -> Result<()> {
    // For now, we'll render text as simple rectangles with the text content
    // A full implementation would require a text rendering library like rusttype or fontdue
    
    let rect = tiny_skia::Rect::from_xywh(
        text_element.x,
        text_element.y,
        text_element.width,
        text_element.height,
    ).ok_or_else(|| anyhow::anyhow!("Invalid text element bounds"))?;
    
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(parse_color(&text_element.color));
    paint.anti_alias = true;
    
    // Draw a simple rectangle to represent text for now
    let path = tiny_skia::PathBuilder::from_rect(rect);
    pixmap.stroke_path(&path, &paint, &tiny_skia::Stroke::default(), tiny_skia::Transform::identity(), None);
    
    Ok(())
}

/// Render shape element on the pixmap
fn render_shape_element(pixmap: &mut tiny_skia::Pixmap, shape_element: &ShapeElement) -> Result<()> {
    let rect = tiny_skia::Rect::from_xywh(
        shape_element.x,
        shape_element.y,
        shape_element.width,
        shape_element.height,
    ).ok_or_else(|| anyhow::anyhow!("Invalid shape element bounds"))?;
    
    let path = tiny_skia::PathBuilder::from_rect(rect);
    
    // Fill if fill color is specified
    if let Some(ref fill_color) = shape_element.fill_color {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(parse_color(fill_color));
        paint.anti_alias = true;
        
        pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
    }
    
    // Stroke if stroke color is specified
    if let Some(ref stroke_color) = shape_element.stroke_color {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(parse_color(stroke_color));
        paint.anti_alias = true;
        
        let stroke = tiny_skia::Stroke {
            width: shape_element.stroke_width,
            ..Default::default()
        };
        
        pixmap.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
    }
    
    Ok(())
}

/// Parse color string to tiny-skia Color
fn parse_color(color_str: &str) -> tiny_skia::Color {
    if color_str.starts_with('#') && color_str.len() == 7 {
        if let Ok(hex) = u32::from_str_radix(&color_str[1..], 16) {
            let r = ((hex >> 16) & 0xFF) as u8;
            let g = ((hex >> 8) & 0xFF) as u8;
            let b = (hex & 0xFF) as u8;
            return tiny_skia::Color::from_rgba8(r, g, b, 255);
        }
    }
    
    // Default to black if parsing fails
    tiny_skia::Color::BLACK
}

/// Convert pixmap to RGB data
fn pixmap_to_rgb(pixmap: &tiny_skia::Pixmap) -> Vec<u8> {
    let mut rgb_data = Vec::with_capacity(pixmap.width() as usize * pixmap.height() as usize * 3);
    
    for pixel in pixmap.pixels() {
        rgb_data.push(pixel.red());
        rgb_data.push(pixel.green());
        rgb_data.push(pixel.blue());
    }
    
    rgb_data
}

/// Encode RGB data as JPEG
fn encode_jpeg(rgb_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    use image::{ImageBuffer, Rgb};
    
    let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, rgb_data)
        .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;
    
    let mut jpeg_data = Vec::new();
    {
        use std::io::Cursor;
        let mut cursor = Cursor::new(&mut jpeg_data);
        img.write_to(&mut cursor, image::ImageFormat::Jpeg)?;
    }
    
    Ok(jpeg_data)
}

/// Convert PowerPoint to markdown with slide-based selection
/// Expects a resolved file path
pub fn process_powerpoint_with_slides(
    resolved_file_path: &str,
    slides: Option<String>,
) -> PowerPointProcessingResult {
    use crate::shared_utils::{parse_pages_parameter, validate_file_path};
    
    let file_path_string = resolved_file_path.to_string();
    let slides = slides.unwrap_or_else(|| "all".to_string());
    
    // Validate file
    if let Err(e) = validate_file_path(resolved_file_path) {
        return PowerPointProcessingResult::error(file_path_string, e);
    }

    // Get or create cached PowerPoint content
    let powerpoint_cache = match POWERPOINT_CACHE_MANAGER.get_or_cache(resolved_file_path, extract_powerpoint_content) {
        Ok(cache) => cache,
        Err(e) => return PowerPointProcessingResult::error(
            file_path_string,
            format!("Failed to extract PowerPoint content: {}", e),
        ),
    };

    let total_slides = powerpoint_cache.total_slides.unwrap_or(0);
    
    // Parse the slides parameter
    let requested_slide_indices = match parse_pages_parameter(&slides, total_slides) {
        Ok(indices) => indices,
        Err(e) => return PowerPointProcessingResult::error(
            file_path_string,
            format!("Invalid slides parameter: {}", e),
        ),
    };

    // Extract specific slides if not all slides are requested
    let content = if requested_slide_indices.len() == total_slides {
        // All slides requested - use cached content
        powerpoint_cache.content.clone()
    } else {
        // Specific slides requested - extract them
        match POWERPOINT_CACHE_MANAGER.extract_units(&powerpoint_cache, &requested_slide_indices, resolved_file_path, extract_powerpoint_slides) {
            Ok(content) => content,
            Err(e) => return PowerPointProcessingResult::error(
                file_path_string,
                format!("Failed to extract specific slides: {}", e),
            ),
        }
    };

    PowerPointProcessingResult::success(
        content,
        Some(total_slides),
        slides,
        requested_slide_indices,
        file_path_string,
        powerpoint_cache.slide_texts,
    )
}

/// Get PowerPoint slide information
/// Expects a resolved file path
pub fn get_powerpoint_slide_info(resolved_file_path: &str) -> PowerPointPageInfoResult {
    use crate::shared_utils::validate_file_path;
    
    let file_path_string = resolved_file_path.to_string();
    
    // Validate file
    if let Err(e) = validate_file_path(resolved_file_path) {
        if e.contains("File not found") {
            return PowerPointPageInfoResult::error(file_path_string, "file_not_found".to_string());
        } else {
            return PowerPointPageInfoResult::error(file_path_string, e);
        }
    }

    // Get or create cached PowerPoint content to get slide count
    match POWERPOINT_CACHE_MANAGER.get_or_cache(resolved_file_path, extract_powerpoint_content) {
        Ok(powerpoint_cache) => {
            let slide_count = powerpoint_cache.total_slides.unwrap_or(0);
            PowerPointPageInfoResult::success(
                file_path_string,
                Some(slide_count),
                format!("PowerPoint file with {} slides", slide_count),
            )
        },
        Err(e) => PowerPointPageInfoResult::error(
            file_path_string,
            format!("Failed to analyze PowerPoint file: {}", e),
        ),
    }
} 