use office_reader_mcp::shared_utils::resolve_file_path_string;
use std::env;

fn main() {
    println!("Testing Path Resolution with Security Model");
    println!("==========================================");
    
    // Test 1: No PROJECT_ROOT set
    println!("\n1. Testing without PROJECT_ROOT:");
    unsafe {
        env::remove_var("PROJECT_ROOT");
    }
    
    test_path("documents/test.pdf");
    test_path("/tmp/absolute.xlsx");
    test_path("./relative.docx");
    
    // Test 2: PROJECT_ROOT set
    println!("\n2. Testing with PROJECT_ROOT set:");
    unsafe {
        env::set_var("PROJECT_ROOT", "/home/user/myproject");
    }
    
    test_path("documents/test.pdf");
    test_path("/tmp/absolute.xlsx"); // This should be rejected
    test_path("./relative.docx");
    
    // Test 3: PROJECT_ROOT set to non-existent directory
    println!("\n3. Testing with non-existent PROJECT_ROOT:");
    unsafe {
        env::set_var("PROJECT_ROOT", "/nonexistent/directory");
    }
    
    test_path("documents/test.pdf");
}

fn test_path(input_path: &str) {
    match resolve_file_path_string(input_path) {
        Ok(resolved) => {
            println!("  '{}' → '{}' ✓", input_path, resolved);
        }
        Err(e) => {
            println!("  '{}' → ERROR: {} ✗", input_path, e);
        }
    }
}