use anyhow::Result;
use crate::plugin_api::{Plugin, Target};
use std::{fs, path::{Path, PathBuf}, process::Command};
use which::which;
use wasmparser::{Parser, Payload, ExternalKind};

/// A generic WASM export plugin: lists all exported functions in a .wasm file
pub struct WasmExportPlugin {
    path: PathBuf,
    wasmtime_path: PathBuf,
}

impl WasmExportPlugin {
    /// Load the generic export plugin if Wasmtime is available
    pub fn load(path: &Path) -> Result<Option<Self>> {
        // Check for Wasmtime CLI
        let wasmtime_path = which("wasmtime")
            .map_err(|e| anyhow::anyhow!("wasmtime not found in PATH: {}", e))?;
        Ok(Some(Self { path: path.to_path_buf(), wasmtime_path }))
    }
}

impl Plugin for WasmExportPlugin {
    fn name(&self) -> &str {
        // Label native DLL exports differently from Wasm exports
        match self.path.extension().and_then(|s| s.to_str()).unwrap_or("") {
            "dll" => "dll-export",
            _     => "wasm-export",
        }
    }

    fn matches(&self, _dir: &Path) -> bool {
        // Generic export plugin applies in any directory
        true
    }

    fn collect_targets(&self, _dir: &Path) -> Result<Vec<Target>> {
        // Native DLL plugin: provide a single default target (the library base name)
        if self.path.extension().and_then(|s| s.to_str()) == Some("dll") {
            let name = self.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("dll_export")
                .to_string();
            return Ok(vec![Target { name, metadata: None }]);
        }
        // WASM plugin: parse exports
        let data = fs::read(&self.path)?;
        let mut targets = Vec::new();
        for payload in Parser::new(0).parse_all(&data) {
            let payload = payload?;
            if let Payload::ExportSection(mut section) = payload {
                for export in section {
                    let export = export?;
                    if export.kind == ExternalKind::Func {
                        targets.push(Target { name: export.name.to_string(), metadata: None });
                    }
                }
            }
        }
        Ok(targets)
    }

    fn build_command(&self, _dir: &Path, target: &Target) -> Result<Command> {
        // Invoke the exported function via Wasmtime
        let mut cmd = Command::new(&self.wasmtime_path);
        cmd.arg(&self.path)
           .arg("--invoke")
           .arg(&target.name);
        Ok(cmd)
    }
    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }
}