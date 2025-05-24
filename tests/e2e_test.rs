use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;
use tokio::process::Command;
use tokio::time::Duration;
use rmcp::{model::{CallToolRequestParam, CallToolResult}, service::ServiceExt, transport::TokioChildProcess};

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

    // Call the process_office_document tool
    let result = service.call_tool(CallToolRequestParam {
        name: "process_office_document".into(),
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

    // Call the process_office_document tool
    let result = service.call_tool(CallToolRequestParam {
        name: "process_office_document".into(),
        arguments: serde_json::json!({
            "file_path": file_path
        }).as_object().cloned(),
    }).await.unwrap();
    println!("Result: {:?}", result);
    
    // Check that the result is not an error
    assert!(result.is_error.is_some() && !result.is_error.unwrap());
    
    // Check that the content contains Excel-specific content
    let content = result.content[0].as_text().unwrap().text.to_lowercase();
    assert!(content.contains("this is a test table"));

    // Kill the server process
    service.cancel().await.unwrap();
}
