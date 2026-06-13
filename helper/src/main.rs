use std::env;
use std::fs;
use std::process::Command;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "/etc/keyd";

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Error: Missing the path to the source profiles directory as the argument");
        eprintln!("Usage: {} <source_profiles_dir>", args[0]);
        std::process::exit(1);
    }

    let source_dir = &args[1];
    let source_path = Path::new(source_dir);

    if !source_path.is_dir() {
        eprintln!("Error: The source path is not a directory or doesn't exist '{}'", source_dir);
        std::process::exit(1);
    }

    println!("Syncing keyd configs from {} to {}...", source_dir, CONFIG_DIR);

    // Ensure /etc/keyd exists
    if !Path::new(CONFIG_DIR).exists() {
        if let Err(e) = fs::create_dir_all(CONFIG_DIR) {
            eprintln!("Error: Could not create config directory '{}': {}", CONFIG_DIR, e);
            std::process::exit(1);
        }
    }

    // 1. Clean up existing .conf files in /etc/keyd to avoid stale configurations
    // Only remove .conf files to avoid touching other things if any.
    if let Ok(entries) = fs::read_dir(CONFIG_DIR) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "conf") {
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("Warning: Could not remove old config '{:?}': {}", path, e);
                }
            }
        }
    }

    // 2. Copy all .conf files from source directory to /etc/keyd
    if let Ok(entries) = fs::read_dir(source_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "conf") {
                let dest_path = PathBuf::from(CONFIG_DIR).join(path.file_name().unwrap());
                if let Err(e) = fs::copy(&path, &dest_path) {
                    eprintln!("Error: Failed to copy config '{:?}' to '{:?}': {}", path, dest_path, e);
                    std::process::exit(1);
                }
                println!("Copied: {:?}", dest_path);
            }
        }
    }

    println!("Sync complete. Reloading keyd...");

    let reload_status = Command::new("keyd")
        .arg("reload")
        .status();

    match reload_status {
        Ok(status) if status.success() => {
            println!("Keyd reloaded!");
        }
        Ok(status) => {
            eprintln!("Failed to reload keyd. Code: {:?}", status.code());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Couldn't run command 'keyd reload': {}", e);
            std::process::exit(1);
        }
    }
}
