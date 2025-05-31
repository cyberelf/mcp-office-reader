use anyhow::Result;
use tokio::runtime::Runtime;
use office_reader_mcp::mcp_handler;

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



