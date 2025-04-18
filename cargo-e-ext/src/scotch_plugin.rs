use anyhow::{anyhow, Result};
use crate::plugin_api::{Plugin, Target};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

/// A “Scotch” Rust → Wasm plugin: builds a Rust crate
/// that depends on `scotch-guest` into `.wasm` and exposes its guest functions.
pub struct ScotchPlugin {
    root: PathBuf,
    crate_name: String,
}

impl ScotchPlugin {
    /// Load the plugin from the given directory and extract its crate name.
    pub fn load(root: &Path) -> Result<Self> {
        // Read Cargo.toml to get the package name
        let cargo_toml = root.join("Cargo.toml");
        let toml_str = fs::read_to_string(&cargo_toml)?;
        let doc: toml::Value = toml::from_str(&toml_str)?;
        let crate_name = doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("Failed to determine crate name in {}", cargo_toml.display()))?;
        Ok(Self {
            root: root.to_path_buf(),
            crate_name,
        })
    }

    /// Path to the Cargo.toml file of the crate.
    fn cargo_toml(&self) -> PathBuf {
        self.root.join("Cargo.toml")
    }

    /// Extract the crate name from Cargo.toml `[package].name`.
    fn crate_name(&self) -> Option<String> {
        let toml_str = fs::read_to_string(self.cargo_toml()).ok()?;
        let doc: toml::Value = toml::from_str(&toml_str).ok()?;
        doc.get("package")?
            .get("name")?
            .as_str()
            .map(|s| s.to_string())
    }

    /// List all guest functions marked with `#[scotch_guest::guest_function]`.
    fn list_guest_functions(&self) -> Result<Vec<String>> {
        let mut funcs = Vec::new();
        let src_dir = self.root.join("src");
        use std::ffi::OsStr;
        for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !entry.file_type().is_file() {
                continue;
            }
            // only Rust source files
            if path.extension().and_then(OsStr::to_str) != Some("rs") {
                continue;
            }
            let text = fs::read_to_string(path)?;
            // scan for the guest_function attribute, then parse next non-empty line for fn signature
            let mut lines = text.lines().peekable();
            while let Some(line) = lines.next() {
                if line.trim_start().starts_with("#[scotch_guest::guest_function]") {
                    // find the next non-empty line
                    while let Some(sig_line) = lines.peek() {
                        let sig = sig_line.trim_start();
                        if sig.is_empty() {
                            lines.next();
                            continue;
                        }
                        // look for "fn " in the signature (handles visibility, async, etc.)
                        if let Some(fn_pos) = sig.find("fn ") {
                            let rest = &sig[fn_pos + 3..];
                            // determine end of the function name (before generics or parameters)
                            if let Some(paren_pos) = rest.find('(') {
                                let end = match rest.find('<') {
                                    Some(gen_pos) if gen_pos < paren_pos => gen_pos,
                                    _ => paren_pos,
                                };
                                let name = rest[..end].trim();
                                if !name.is_empty() {
                                    funcs.push(name.to_string());
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
        Ok(funcs)
    }
}

impl Plugin for ScotchPlugin {
    fn name(&self) -> &str {
        "scotch"
    }

    fn matches(&self, _dir: &Path) -> bool {
        // Match if Cargo.toml declares scotch-guest dependency
        let cargo = self.root.join("Cargo.toml");
        if let Ok(toml_str) = fs::read_to_string(&cargo) {
            if toml_str.contains("scotch-guest") {
                return true;
            }
        }
        // Or if any source file contains the guest_function attribute
        let src_dir = self.root.join("src");
        for entry in WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Ok(text) = fs::read_to_string(entry.path()) {
                    if text.contains("#[scotch_guest::guest_function]") {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn collect_targets(&self, _dir: &Path) -> Result<Vec<Target>> {
        let names = self.list_guest_functions()?;
        let targets = names
            .into_iter()
            .map(|name| Target { name, metadata: None })
            .collect();
        Ok(targets)
    }

    fn build_command(&self, _dir: &Path, target: &Target) -> Result<Command> {
        // Build the crate as wasm, then invoke the guest function with Wasmtime
        #[cfg(unix)]
        {
            let cmd_str = format!(
                "cargo build --release --target wasm32-unknown-unknown -p {p} && \
                 wasmtime target/wasm32-unknown-unknown/release/{p}.wasm --invoke {f}",
                p = self.crate_name,
                f = target.name
            );
            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(cmd_str);
            cmd.current_dir(&self.root);
            Ok(cmd)
        }
        #[cfg(windows)]
        {
            let cmd_str = format!(
                "cargo build --release --target wasm32-unknown-unknown -p {p} && \
                 wasmtime target\\wasm32-unknown-unknown\\release\\{p}.wasm --invoke {f}",
                p = self.crate_name,
                f = target.name
            );
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(cmd_str);
            cmd.current_dir(&self.root);
            Ok(cmd)
        }
    }
    fn source(&self) -> Option<String> {
        // Use the built wasm path as source info if available
        let wasm = self
            .root
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(format!("{}.wasm", self.crate_name));
        if wasm.exists() {
            Some(wasm.to_string_lossy().into())
        } else {
            Some(self.root.to_string_lossy().into())
        }
    }
}