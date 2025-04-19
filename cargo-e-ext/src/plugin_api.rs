use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};
use std::path::PathBuf;
use walkdir::WalkDir;
use crate::wasm_plugin::WasmPlugin;
use toml;
use crate::export_plugin::WasmExportPlugin;
use std::fs;
use crate::scotch_plugin::ScotchPlugin;
use crate::rhai_plugin::RhaiPlugin;

pub use crate::lua_plugin::CommandSpec;
/// Returns the directories to search for plugins: first the project-local `plugins/`,
/// then the `plugins/` folder alongside the executable (for built-in plugins).
fn plugin_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    // project-local plugins
    dirs.push(PathBuf::from("plugins"));
    dirs
}

pub fn find_wasm_plugins() -> Vec<PathBuf> {
    let mut wasm_paths = Vec::new();
    // Search in each plugin directory
    for base in plugin_directories() {
        if !base.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&base)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                let path = e.path();
                // Only allow *.wasm files
                let is_wasm = path.extension().map_or(false, |ext| ext == "wasm");
                // also allow native dynamic libraries as plugins
                let is_dll = path.extension().map_or(false, |ext| ext == "dll");
                let is_wasm_or_dll = is_wasm || is_dll;
                // Skip anything inside a /deps/ directory
                let not_in_deps = !path
                    .components()
                    .any(|c| c.as_os_str().to_string_lossy() == "deps");
                is_wasm_or_dll && not_in_deps
            })
        {
            wasm_paths.push(entry.into_path());
        }
    }
    wasm_paths
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Target {
    pub name: String,
    pub metadata: Option<String>,
}

pub trait Plugin {
    fn name(&self) -> &str;
    fn matches(&self, dir: &Path) -> bool;
    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>>;
    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command>;
    /// Optional human-readable source path of the plugin (e.g., .lua script, .wasm file, crate path)
    fn source(&self) -> Option<String> {
        None
    }
}

pub fn load_plugins() -> Result<Vec<Box<dyn Plugin>>> {
    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();
    // current directory for matches
    let cwd = std::env::current_dir()?;

    // Load Lua and Rhai script plugins from project-local and built-in `plugins/` directories
    for base in plugin_directories() {
        if !base.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&base)? {
            let path = entry?.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "lua" {
                    let plugin = crate::lua_plugin::LuaPlugin::load(&path)?;
                    plugins.push(Box::new(plugin));
                } else if ext == "rhai" {
                    let plugin = RhaiPlugin::load(&path)?;
                    plugins.push(Box::new(plugin));
                }
            }
        }
    }

    // Recursively find all .wasm plugins in plugins/**/target/**/*.wasm
    for wasm_path in find_wasm_plugins() {
        println!("[debug] trying plugin: {}", wasm_path.display());
        // First, try the protocol-aware WasmPlugin
        if let Some(wp) = WasmPlugin::load(&wasm_path)? {
            if wp.matches(&cwd) {
                plugins.push(Box::new(wp));
                continue;
            }
        }
        // Fallback to generic export plugin for arbitrary exports
        if let Some(gp) = WasmExportPlugin::load(&wasm_path)? {
            plugins.push(Box::new(gp));
        }
    }
    // Rust crate plugins: first Scotch-style (scotch-guest), else generic wasm exports
    for base in plugin_directories() {
        if !base.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&base)? {
            let path = entry?.path();
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                // Try Scotch-style plugin (requires scotch-guest dep)
                let scotch = ScotchPlugin::load(&path)?;
                if scotch.matches(&path) {
                    plugins.push(Box::new(scotch));
                } else {
                    // Generic Rust crate: attempt to read its package name and build its wasm
                    let toml_text = fs::read_to_string(&cargo_toml)?;
                    if let Ok(doc) = toml::from_str::<toml::Value>(&toml_text) {
                        if let Some(pkg) = doc.get("package")
                                               .and_then(|p| p.get("name"))
                                               .and_then(|v| v.as_str())
                                               .map(|s| s.to_string())
                        {
                            // build wasm32 target
                            let mut bcmd = Command::new("cargo");
                            bcmd.current_dir(&path)
                                .arg("build")
                                .arg("--release")
                                .arg("--target")
                                .arg("wasm32-unknown-unknown")
                                .arg("-p")
                                .arg(&pkg);
                            let _ = bcmd.status()?;
                            // load the resulting .wasm
                            let wasm_file = path.join("target")
                                                 .join("wasm32-unknown-unknown")
                                                 .join("release")
                                                 .join(format!("{}.wasm", pkg));
                            if wasm_file.exists() {
                                if let Some(gp) = WasmExportPlugin::load(&wasm_file)? {
                                    plugins.push(Box::new(gp));
                                }
                            }
                            // Also load native dynamic library if present
                            #[cfg(target_os = "windows")]
                            {
                                let dll_file = path.join("target").join("release").join(format!("{}.dll", pkg));
                                if dll_file.exists() {
                                    if let Some(gp) = WasmExportPlugin::load(&dll_file)? {
                                        plugins.push(Box::new(gp));
                                    }
                                }
                            }
                            #[cfg(all(not(target_os = "windows"), target_os = "macos"))]
                            {
                                let dylib_file = path.join("target").join("release").join(format!("lib{}.dylib", pkg));
                                if dylib_file.exists() {
                                    if let Some(gp) = WasmExportPlugin::load(&dylib_file)? {
                                        plugins.push(Box::new(gp));
                                    }
                                }
                            }
                            #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
                            {
                                let so_file = path.join("target").join("release").join(format!("lib{}.so", pkg));
                                if so_file.exists() {
                                    if let Some(gp) = WasmExportPlugin::load(&so_file)? {
                                        plugins.push(Box::new(gp));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(plugins)
}
