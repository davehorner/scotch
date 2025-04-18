use anyhow::{anyhow, Context, Result};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::plugin_api::{Plugin, Target};
use crate::lua_plugin::CommandSpec;

// On Unix, check executable permission bits; on Windows, check for .exe extension
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// An external CLI plugin following the cargo-e-ext plugin protocol.
#[derive(Debug)]
pub struct ExternalPlugin {
    path: PathBuf,
    name: String,
    version: String,
}

impl ExternalPlugin {
    /// Attempt to load an external plugin from the given path.
    /// Returns Ok(Some(plugin)) if the path is an executable CLI plugin
    /// that supports the current client version, else Ok(None).
    pub fn load(path: &Path) -> Result<Option<Self>> {
        // Only consider executable files
        if !Self::is_executable(path) {
            return Ok(None);
        }
        let path_buf = path.to_path_buf();
        // Ensure plugin supports this client version
        let client_version = env!("CARGO_PKG_VERSION");
        let status = Command::new(&path_buf)
            .arg("--client-version")
            .arg(client_version)
            .status()
            .with_context(|| format!("failed to run plugin {} for client-version check", path_buf.display()))?;
        if !status.success() {
            return Ok(None);
        }
        // Query plugin name
        let output = Command::new(&path_buf)
            .arg("--name")
            .output()
            .with_context(|| format!("failed to run plugin {} --name", path_buf.display()))?;
        if !output.status.success() {
            return Ok(None);
        }
        let name = String::from_utf8(output.stdout)?.trim().to_string();
        // Query plugin version
        let output_v = Command::new(&path_buf)
            .arg("--version")
            .output()
            .with_context(|| format!("failed to run plugin {} --version", path_buf.display()))?;
        if !output_v.status.success() {
            return Ok(None);
        }
        let version = String::from_utf8(output_v.stdout)?.trim().to_string();
        Ok(Some(Self { path: path_buf, name, version }))
    }

    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        if let Ok(meta) = fs::metadata(path) {
            let perm = meta.permissions();
            // any execute bit set
            perm.mode() & 0o111 != 0
        } else {
            false
        }
    }

    #[cfg(windows)]
    fn is_executable(path: &Path) -> bool {
        path.is_file() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe"))
    }
}

impl Plugin for ExternalPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn matches(&self, dir: &Path) -> bool {
        match Command::new(&self.path).arg("matches").arg(dir).output() {
            Ok(output) if output.status.success() => {
                let s = String::from_utf8_lossy(&output.stdout);
                let s = s.trim();
                s == "true" || s == "1"
            }
            _ => false,
        }
    }

    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>> {
        let output = Command::new(&self.path)
            .arg("collect-targets")
            .arg(dir)
            .output()
            .with_context(|| format!("failed to run plugin {} collect-targets", self.path.display()))?;
        if !output.status.success() {
            return Err(anyhow!("plugin collect-targets failed"));
        }
        let json = String::from_utf8(output.stdout)?;
        let targets: Vec<Target> = serde_json::from_str(&json)?;
        Ok(targets)
    }

    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        let output = Command::new(&self.path)
            .arg("build-command")
            .arg(dir)
            .arg(&target.name)
            .output()
            .with_context(|| format!("failed to run plugin {} build-command", self.path.display()))?;
        if !output.status.success() {
            return Err(anyhow!("plugin build-command failed"));
        }
        let json = String::from_utf8(output.stdout)?;
        let spec: CommandSpec = serde_json::from_str(&json)?;
        Ok(spec.into_command(dir))
    }

    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into_owned())
    }
}