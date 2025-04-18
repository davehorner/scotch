use anyhow::{bail, Result};
use std::{path::Path, process::Command};
use std::path::PathBuf;
use crate::plugin_api::{Plugin, Target};
use which::which;

pub struct WasmPlugin {
    path: std::path::PathBuf,
    wasmtime_path: PathBuf,
}

impl WasmPlugin {
    /// Attempt to locate `wasmtime`; if found, return a WasmPlugin, else skip
    pub fn load(path: &Path) -> Result<Option<Self>> {
        match which("wasmtime") {
            Ok(wasmtime_path) => Ok(Some(Self { path: path.to_path_buf(), wasmtime_path })),
            Err(_) => {
                eprintln!("[warn] wasmtime not found in PATH — skipping wasm plugin: {}", path.display());
                Ok(None)
            }
        }
    }

    // fn run_wasm(&self, args: &[&str]) -> Result<String> {
    //     let output = Command::new(&self.wasmtime_path)
    //     .arg("--dir").arg(".") // allow access to current dir
    //         .arg(&self.path)
    //         .args(args)
    //         .output()?;
    //     if !output.status.success() {
    //         return Err(anyhow::anyhow!(
    //             "WASM plugin error: {}",
    //             String::from_utf8_lossy(&output.stderr)
    //         ));
    //     }
    //     Ok(String::from_utf8(output.stdout)?)
    // }
    /// Run the Wasm plugin via Wasmtime, preopening `dir` for WASI, passing `args` to the module
    fn run_wasm(&self, args: &[&str], dir: &Path) -> Result<String> {
        let output = Command::new(&self.wasmtime_path)
            .arg("--dir").arg(dir)
            .arg(&self.path)
            .args(args)
            .output()?;
        if !output.status.success() {
            eprintln!("[wasm stderr] {}", String::from_utf8_lossy(&output.stderr));
            bail!("WASM plugin error: {}", String::from_utf8_lossy(&output.stderr));
        }
        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("[wasm stdout] {}", out);
        Ok(out)
    }
}

impl Plugin for WasmPlugin {
    fn name(&self) -> &str {
        "wasm-plugin"
    }

    fn matches(&self, dir: &Path) -> bool {
        println!("[debug] WASM plugin checking matches for {}", dir.display());
        match self.run_wasm(&["--matches", &dir.to_string_lossy()], dir) {
            Ok(s) => {
                let matched = s.trim() == "true";
                println!("[debug] WASM plugin match result: {}", matched);
                matched
            }
            Err(e) => {
                eprintln!("[warn] WASM plugin match failed: {}", e);
                false
            }
        }
    }

    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>> {
        println!("[debug] WASM plugin collecting targets from {}", dir.display());
        let json = self.run_wasm(&["--collect_targets", &dir.to_string_lossy()], dir)?;
        let targets: Vec<Target> = serde_json::from_str(&json)?;
        Ok(targets)
    }

    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        println!("[debug] WASM plugin building target '{}' in {}", target.name, dir.display());
        let json = self.run_wasm(
            &["--build_command", &dir.to_string_lossy(), &target.name],
            dir,
        )?;
        let spec: CommandSpec = serde_json::from_str(&json)?;
        Ok(spec.into_command(dir))
    }
    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }
}

#[derive(serde::Deserialize)]
struct CommandSpec {
    prog: String,
    args: Vec<String>,
    cwd: Option<String>,
}

impl CommandSpec {
    fn into_command(self, default_dir: &Path) -> Command {
        let mut cmd = Command::new(self.prog);
        for arg in self.args {
            cmd.arg(arg);
        }
        if let Some(cwd) = self.cwd {
            cmd.current_dir(cwd);
        } else {
            cmd.current_dir(default_dir);
        }
        cmd
    }
}
