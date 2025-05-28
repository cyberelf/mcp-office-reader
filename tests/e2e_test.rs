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
    assert!(content.contains("file not found") || content.contains("error"));

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
