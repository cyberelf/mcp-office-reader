use anyhow::Result;
use tokio::runtime::Runtime;
use office_reader_mcp::mcp_handler;
use std::panic;
use std::fs;
use chrono::Utc;

fn main() -> Result<()> {
    // Create logs directory if it doesn't exist
    let logs_dir = "logs";
    if !std::path::Path::new(logs_dir).exists() {
        fs::create_dir_all(logs_dir)?;
    }
    
    // Generate log file name with timestamp
    let log_filename = format!("{}/mcp_office_reader_{}.log", 
                              logs_dir, 
                              Utc::now().format("%Y%m%d_%H%M%S"));
    
    // Initialize logging with file output only (stdout interferes with MCP protocol)
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file(&log_filename)?) // Output to file only
        .apply()?;
    
    log::info!("📁 Log file created: {}", log_filename);
    
    // Set up a panic hook to capture panic information
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Log the panic with our logging infrastructure
        let _panic_msg = if let Some(location) = panic_info.location() {
            if let Some(msg) = panic_info.payload().downcast_ref::<&str>() {
                let full_msg = format!("🚨 PANIC at {}:{} - {}", location.file(), location.line(), msg);
                log::error!("{}", full_msg);
                full_msg
            } else if let Some(msg) = panic_info.payload().downcast_ref::<String>() {
                let full_msg = format!("🚨 PANIC at {}:{} - {}", location.file(), location.line(), msg);
                log::error!("{}", full_msg);
                full_msg
            } else {
                let full_msg = format!("🚨 PANIC at {}:{} - Unknown panic payload", location.file(), location.line());
                log::error!("{}", full_msg);
                full_msg
            }
        } else {
            let full_msg = "🚨 PANIC - Unknown location".to_string();
            log::error!("{}", full_msg);
            full_msg
        };
        
        // Note: Not writing to stderr to avoid interfering with MCP protocol
        // All panic details are captured in the log file
        
        // Call the original hook to get the default panic behavior
        original_hook(panic_info);
    }));
    
    // log::info!("🚀 Starting Office Reader MCP Server with debug logging enabled");
    
    // Create a Tokio runtime for async operations
    let rt = Runtime::new()?;
    
    // Run the RMCP server in the Tokio runtime
    let result = rt.block_on(async {
        // log::debug!("🔍 main: About to start MCP server");
        // Start the MCP server
        mcp_handler::start_server().await
    });
    
    match &result {
        Ok(_) => {
            log::info!("✅ Office Reader MCP Server shut down cleanly");
            log::info!("📁 Complete logs saved to: {}", log_filename);
        },
        Err(e) => {
            log::error!("❌ Office Reader MCP Server exited with error: {}", e);
            log::error!("📁 Error logs saved to: {}", log_filename);
        }
    }
    
    result
}



