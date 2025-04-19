use anyhow::{anyhow, Result};
use rhai::{Engine, AST, Scope};
use serde_json;
use std::{fs, path::{Path, PathBuf}, process::Command};
use crate::plugin_api::{Plugin, Target, CommandSpec};

/// A Rhai-based plugin implementation for the `Plugin` trait.
pub struct RhaiPlugin {
    name: String,
    engine: Engine,
    ast: AST,
    path: PathBuf,
}

impl RhaiPlugin {
    /// Load the Rhai script plugin from the given path.
    pub fn load(path: &Path) -> Result<Self> {
        let code = fs::read_to_string(path)?;
        let engine = Engine::new();
        let ast = engine.compile(&code)?;
        // Retrieve the plugin name by calling the `name()` function in the script
        let mut scope = Scope::new();
        let name: String = engine
            .call_fn(&mut scope, &ast, "name", ())
            .map_err(|e| anyhow!("Rhai error calling name: {:?}", e))?;
        Ok(Self { name, engine, ast, path: path.to_path_buf() })
    }
}

impl Plugin for RhaiPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn matches(&self, dir: &Path) -> bool {
        let dir_str = dir.to_string_lossy().to_string();
        let mut scope = Scope::new();
        self.engine
            .call_fn::<bool>(&mut scope, &self.ast, "matches", (dir_str,))
            .unwrap_or(false)
    }

    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>> {
        let dir_str = dir.to_string_lossy().to_string();
        let mut scope = Scope::new();
        let json: String = self
            .engine
            .call_fn(&mut scope, &self.ast, "collect_targets", (dir_str,))
            .map_err(|e| anyhow!("Rhai error calling collect_targets: {:?}", e))?;
        let targets: Vec<Target> = serde_json::from_str(&json)?;
        Ok(targets)
    }

    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        let dir_str = dir.to_string_lossy().to_string();
        let target_str = target.name.clone();
        let mut scope = Scope::new();
        let json: String = self
            .engine
            .call_fn(&mut scope, &self.ast, "build_command", (dir_str, target_str))
            .map_err(|e| anyhow!("Rhai error calling build_command: {:?}", e))?;
        let spec: CommandSpec = serde_json::from_str(&json)
            .map_err(|e| anyhow!("Invalid JSON from Rhai: {:?}\nOriginal: {}", e, json))?;
        Ok(spec.into_command(dir))
    }

    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }
}