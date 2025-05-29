use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;
use tokio::process::Command;
use tokio::time::Duration;
use rmcp::{model::CallToolRequestParam, service::ServiceExt, transport::TokioChildProcess};

#[tokio::test]
async fn test_process_text_document() {
    // Create a test document
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(b"Test document content").expect("Failed to write to temp file");
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get server info
    let server_info = service.peer_info();
    println!("Server info: {:?}", server_info);
    assert!(server_info.server_info.name == "rmcp");
    
    // List tools
    let tools = service.list_tools(Default::default()).await.unwrap();
    println!("Tools: {:?}", tools);
    assert!(tools.tools.len() > 0);

    // Call the read_office_document tool
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Result: {:?}", result);
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    // assert!(result.content[0].as_text().unwrap().text == "Test document content");

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_process_excel_document() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get server info
    let server_info = service.peer_info();
    println!("Server info: {:?}", server_info);
    assert!(server_info.server_info.name == "rmcp");
    
    // List tools
    let tools = service.list_tools(Default::default()).await.unwrap();
    println!("Tools: {:?}", tools);
    assert!(tools.tools.len() > 0);

    // Call the read_office_document tool
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains Excel-specific content
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("this is a test table"));

    // Also test with explicit page parameter
    let result_with_pages = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": "all"
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Result with pages: {:?}", result_with_pages);
    
    // Check that the result with pages is also not an error
    assert!(result_with_pages.is_error.is_some() && !result_with_pages.is_error.unwrap());
    
    // Check that the content contains page metadata
    let content_with_pages = result_with_pages.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content_with_pages.contains("this is a test table"));
    assert!(content_with_pages.contains("requested_pages") || content_with_pages.contains("page"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_stream_excel_document() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Get server info
    let server_info = service.peer_info();
    println!("Server info: {:?}", server_info);
    assert!(server_info.server_info.name == "rmcp");
    
    // List tools
    let tools = service.list_tools(Default::default()).await.unwrap();
    println!("Tools: {:?}", tools);
    assert!(tools.tools.len() >= 2); // Should have both read_office_document and stream_office_document

    // Verify stream_office_document tool exists
    let stream_tool = tools.tools.iter().find(|t| t.name == "stream_office_document");
    assert!(stream_tool.is_some(), "stream_office_document tool should be available");

    // Call the stream_office_document tool
    let result = service.call_tool(CallToolRequestParam {
        name: "stream_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "chunk_size": 5000
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Streaming Result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains streaming progress information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("current_page") || content.contains("chunk"));
    assert!(content.contains("this is a test table"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_stream_pdf_document_with_small_chunk() {
    // Create a test PDF content (we'll use a text file for simplicity in testing)
    let mut temp_file = NamedTempFile::with_suffix(".pdf").expect("Failed to create temp PDF file");
    
    // Create a longer text content to test chunking
    let long_content = "This is a test PDF document. ".repeat(100); // About 2900 characters
    temp_file.write_all(long_content.as_bytes()).expect("Failed to write to temp file");
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the stream_office_document tool with a small chunk size
    let result = service.call_tool(CallToolRequestParam {
        name: "stream_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "chunk_size": 1000  // Small chunk size to test chunking
        }).as_object().cloned(),
    }).await.unwrap();
    println!("PDF Streaming Result: {:?}", result);
    
    // The result should contain progress information
    let content = result.content[0].as_text().unwrap().text.clone();
    
    // Should contain JSON progress info
    assert!(content.contains("current_page") || content.contains("chunk"));
    
    // For a PDF file that doesn't exist or can't be processed, we should get an error message
    // but the tool call itself should succeed
    assert!(result.is_error.is_some() && !result.is_error.unwrap());

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_stream_nonexistent_file() {
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the stream_office_document tool with a non-existent file
    let result = service.call_tool(CallToolRequestParam {
        name: "stream_office_document".into(),
        arguments: serde_json::json!({
            "file_path": "nonexistent_file.pdf",
            "chunk_size": 1000
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Nonexistent file result: {:?}", result);
    
    // The tool call should succeed but return an error in the content
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("file not found"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_stream_unsupported_file_type() {
    // Create a test file with unsupported extension
    let mut temp_file = NamedTempFile::with_suffix(".txt").expect("Failed to create temp file");
    temp_file.write_all(b"This is a text file").expect("Failed to write to temp file");
    let file_path = temp_file.path().to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the stream_office_document tool with an unsupported file type
    let result = service.call_tool(CallToolRequestParam {
        name: "stream_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "chunk_size": 1000
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Unsupported file type result: {:?}", result);
    
    // The tool call should succeed but return an error about unsupported file type
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("unsupported file type") || content.contains("error"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_stream_with_default_chunk_size() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the stream_office_document tool without specifying chunk_size (should use default)
    let result = service.call_tool(CallToolRequestParam {
        name: "stream_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path
            // No chunk_size specified - should use default
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Default chunk size result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains expected information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("current_page") || content.contains("chunk"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_get_document_page_info() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // List tools to verify get_document_page_info exists
    let tools = service.list_tools(Default::default()).await.unwrap();
    let page_info_tool = tools.tools.iter().find(|t| t.name == "get_document_page_info");
    assert!(page_info_tool.is_some(), "get_document_page_info tool should be available");

    // Call the get_document_page_info tool
    let result = service.call_tool(CallToolRequestParam {
        name: "get_document_page_info".into(),
        arguments: serde_json::json!({
            "file_path": file_path
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Page info result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains page information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("total pages") || content.contains("page"));
    assert!(content.contains("file:"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_get_document_page_info_nonexistent_file() {
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the get_document_page_info tool with a non-existent file
    let result = service.call_tool(CallToolRequestParam {
        name: "get_document_page_info".into(),
        arguments: serde_json::json!({
            "file_path": "nonexistent_file.xlsx"
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Nonexistent file page info result: {:?}", result);
    
    // The tool call should succeed but return file not found info
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("file not found"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_read_document_with_specific_pages() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the read_office_document tool with specific page selection
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": "1"
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Specific page result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains page-specific information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("requested_pages") || content.contains("page"));
    assert!(content.contains("this is a test table"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_read_document_with_page_range() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // First get page info to understand available pages
    let page_info_result = service.call_tool(CallToolRequestParam {
        name: "get_document_page_info".into(),
        arguments: serde_json::json!({
            "file_path": file_path
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Page info for range test: {:?}", page_info_result);

    // Call the read_office_document tool with page range (if multiple pages exist)
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": "1-1"  // Range format, even if only one page
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Page range result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains range information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("requested_pages") || content.contains("page"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_read_document_with_all_pages() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the read_office_document tool with "all" pages
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": "all"
        }).as_object().cloned(),
    }).await.unwrap();
    println!("All pages result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains all pages information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("requested pages"));
    assert!(content.contains("this is a test table"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_read_document_with_invalid_page_range() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the read_office_document tool with invalid page range
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": "999"  // Page that likely doesn't exist
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Invalid page range result: {:?}", result);
    
    // The tool call should succeed but return an error about invalid pages
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("error") || content.contains("exceeds") || content.contains("invalid"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_read_document_with_multiple_page_selection() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the read_office_document tool with multiple page selection
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": "1,1"  // Duplicate pages (should be handled gracefully)
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Multiple page selection result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains page information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("requested_pages") || content.contains("page"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_page_workflow_integration() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Step 1: Get page information
    let page_info_result = service.call_tool(CallToolRequestParam {
        name: "get_document_page_info".into(),
        arguments: serde_json::json!({
            "file_path": file_path
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Workflow - Page info: {:?}", page_info_result);
    assert!(page_info_result.is_error.is_some() && !page_info_result.is_error.unwrap());

    // Step 2: Read specific pages based on the info
    let read_result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": "1"  // Read first page
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Workflow - Read result: {:?}", read_result);
    assert!(read_result.is_error.is_some() && !read_result.is_error.unwrap());

    // Step 3: Stream the document for comparison
    let stream_result = service.call_tool(CallToolRequestParam {
        name: "stream_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "chunk_size": 5000
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Workflow - Stream result: {:?}", stream_result);
    assert!(stream_result.is_error.is_some() && !stream_result.is_error.unwrap());

    // Verify all three approaches return content about the same document
    let page_info_content = page_info_result.content[0].as_text().unwrap().text.clone().to_lowercase();
    let read_content = read_result.content[0].as_text().unwrap().text.clone().to_lowercase();
    let stream_content = stream_result.content[0].as_text().unwrap().text.clone().to_lowercase();

    // All should reference the same file
    assert!(page_info_content.contains(&file_path.to_lowercase()) || page_info_content.contains("test.xlsx"));
    assert!(read_content.contains("this is a test table"));
    assert!(stream_content.contains("this is a test table"));

    // Kill the server process
    service.cancel().await.unwrap();
}

#[tokio::test]
async fn test_read_document_with_integer_page() {
    // Path to the Excel test file
    let file_path = Path::new("tests").join("test.xlsx");
    let file_path = file_path.to_str().unwrap().to_string();
    
    // Start the MCP server in a separate process
    let service = ()
        .serve(TokioChildProcess::new(
            Command::new("cargo").arg("run"),
        ).unwrap())
        .await.unwrap();
    
    // Give the server some time to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Call the read_office_document tool with integer page parameter
    let result = service.call_tool(CallToolRequestParam {
        name: "read_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path,
            "pages": 1  // Integer instead of string
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Integer page result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains page-specific information
    let content = result.content[0].as_text().unwrap().text.clone().to_lowercase();
    assert!(content.contains("requested pages") || content.contains("page"));
    assert!(content.contains("this is a test table"));

    // Kill the server process
    service.cancel().await.unwrap();
}
