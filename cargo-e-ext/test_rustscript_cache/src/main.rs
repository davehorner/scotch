use std::{env, path::PathBuf};
use anyhow::{Context, Result};
use walkdir::WalkDir;

/// Finds rust-script’s cache and collects all `.wasm` artifacts under it.
pub struct RustScriptCache {
    root: PathBuf,
}

impl RustScriptCache {
    /// Locate the root cache directory for rust-script.
    pub fn new() -> Result<Self> {
        #[cfg(windows)]
        let root = {
            // Allow overriding on Windows via XDG_CACHE_HOME (for testing) or fallback to LOCALAPPDATA
            if let Some(xdg) = env::var_os("XDG_CACHE_HOME") {
                PathBuf::from(xdg).join("rust-script")
            } else {
                let local = env::var_os("LOCALAPPDATA")
                    .context("LOCALAPPDATA env var not set")?;
                PathBuf::from(local).join("rust-script")
            }
        };

        #[cfg(not(windows))]
        let root = {
            let base = env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .or_else(|| env::var_os("HOME").map(PathBuf::from))
                .context("Neither XDG_CACHE_HOME nor HOME env var is set")?;
            base.join("rust-script")
        };

        Ok(RustScriptCache { root })
    }

    /// Walk the cache tree and return the full paths to every `.wasm` file found.
    pub fn wasm_paths(&self) -> Result<Vec<PathBuf>> {
        let mut results = Vec::new();

        for entry in WalkDir::new(&self.root) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                results.push(path.to_path_buf());
            }
        }

        Ok(results)
    }
}

fn main() -> Result<()> {
    let cache = RustScriptCache::new()?;
    let paths = cache.wasm_paths()?;
    if paths.is_empty() {
        println!("No .wasm files found in rust-script cache: {}", cache.root.display());
    } else {
        println!("Found .wasm files:");
        for p in paths {
            println!("  {}", p.display());
        }
    }
    Ok(())
}
 
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_wasm_paths_found() -> Result<()> {
        // Create a temporary cache root
        let dir = tempdir()?;
        let root = dir.path().join("rust-script");
        // Simulate a compiled Wasm artifact
        let wasm_dir = root
            .join("proj1")
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release");
        fs::create_dir_all(&wasm_dir)?;
        let wasm_file = wasm_dir.join("foo.wasm");
        fs::write(&wasm_file, b"dummy")?;

        // Override the cache env var
        std::env::set_var("XDG_CACHE_HOME", dir.path());
        let cache = RustScriptCache::new()?;
        let mut paths = cache.wasm_paths()?;
        paths.sort();
        assert_eq!(paths, vec![wasm_file]);
        Ok(())
    }

    #[test]
    fn test_no_wasm() -> Result<()> {
        let dir = tempdir()?;
        std::env::set_var("XDG_CACHE_HOME", dir.path());
        let cache = RustScriptCache::new()?;
        let paths = cache.wasm_paths()?;
        assert!(paths.is_empty());
        Ok(())
    }
}