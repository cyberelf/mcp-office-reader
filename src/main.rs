mod document_parser;
mod mcp_handler;

use anyhow::Result;
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    // eprintln!("Starting Office Reader MCP Server...");
    
    // Create a Tokio runtime for async operations
    let rt = Runtime::new()?;
    
    // Run the RMCP server in the Tokio runtime
    rt.block_on(async {
        // Start the MCP server
        mcp_handler::start_server().await
    })?;

    Ok(())
}



