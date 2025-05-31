use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if the DLL changes
    println!("cargo:rerun-if-changed=lib/pdfium.dll");
    println!("cargo:rerun-if-changed=bin/pdfium.dll");
    
    // Check if we're building for Windows
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "windows" {
        // Find the DLL in one of the expected locations
        let dll_paths = ["lib/pdfium.dll"];
        let mut dll_source = None;
        
        for dll_path in &dll_paths {
            if Path::new(dll_path).exists() {
                println!("cargo:rustc-env=PDFIUM_DLL_PATH={}", dll_path);
                dll_source = Some(dll_path);
                break;
            }
        }
        
        if let Some(source_path) = dll_source {
            // Copy DLL to output directory for simple deployment
            if let Ok(out_dir) = env::var("OUT_DIR") {
                // Navigate up from OUT_DIR to find the target directory
                let out_path = Path::new(&out_dir);
                if let Some(target_dir) = find_target_dir(out_path) {
                    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
                    let dll_dest = target_dir.join(&profile).join("pdfium.dll");
                    
                    if dll_dest.exists() {
                        return;
                    }
                    if let Err(e) = fs::copy(source_path, &dll_dest) {
                        println!("cargo:warning=Failed to copy DLL to output directory: {}", e);
                    } else {
                        println!("cargo:warning=Copied pdfium.dll to: {:?}", dll_dest);
                    }
                }
            }
        } else {
            panic!("pdfium.dll not found in lib/ or bin/ directories");
        }
    }
}

fn find_target_dir(mut path: &Path) -> Option<&Path> {
    loop {
        if path.file_name() == Some(std::ffi::OsStr::new("target")) {
            return Some(path);
        }
        path = path.parent()?;
    }
} 