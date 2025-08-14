use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if the DLL changes
    println!("cargo:rerun-if-changed=lib/pdfium.dll");
    println!("cargo:rerun-if-changed=lib/libpdfium.so");
    println!("cargo:rerun-if-changed=lib/libpdfium.dylib");
    println!("cargo:rerun-if-changed=lib/libmupdf.so");
    println!("cargo:rerun-if-changed=lib/libmupdf.dylib");
    println!("cargo:rerun-if-changed=lib/libpoppler.so");
    println!("cargo:rerun-if-changed=lib/libpoppler.dylib");
    
    // Check if we're building for Windows
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    if let Ok(out_dir) = env::var("OUT_DIR") {
        // Navigate up from OUT_DIR to find the target directory
        let out_path = Path::new(&out_dir);
        if let Some(target_dir) = find_target_dir(out_path) {
            let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
            let dll_dest = target_dir.join(&profile).join("pdfium.dll");
            
            if dll_dest.exists() {
                return;
            }
            copy_dynamic_library("lib", target_dir, &target_os, &profile);
        }
    }
}

fn copy_dynamic_library(source_path: &str, target_dir: &Path, target_os: &str, profile: &str) {
    #[cfg(feature = "pdfium")]
    let file_name = match target_os {
        "windows" => Some("pdfium.dll"),
        "linux" => Some("libpdfium.so"),
        "macos" => Some("libpdfium.dylib"),
        _ => None,
    };
    #[cfg(feature = "mupdf_backend")]
    let file_name = match target_os {
        "windows" => Some("mupdf.dll"),
        "linux" => Some("libmupdf.so"),
        "macos" => Some("libmupdf.dylib"),
        _ => None,
    };
    #[cfg(feature = "poppler")]
    let file_name = match target_os {
        "windows" => Some("poppler.dll"),
        "linux" => Some("libpoppler.so"),
        "macos" => Some("libpoppler.dylib"),
        _ => None,
    };
    #[cfg(not(any(feature = "pdfium", feature = "mupdf_backend", feature = "poppler")))]
    let file_name: Option<&str> = None;

    if file_name.is_none() {
        return;
    }

    let dll_source = Path::new(source_path).join(file_name.unwrap());
    let dll_dest = target_dir.join(&profile).join(file_name.unwrap());

    if let Err(e) = fs::copy(&dll_source, &dll_dest) {
        println!("cargo:warning=Failed to copy {} to output directory: {}", target_os, e);
    } else {
        println!("cargo:warning=Copied {} to: {:?}", target_os, dll_dest);
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