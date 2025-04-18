use anyhow::Result;
use mlua::{Lua, Table, Function};
use serde_json;
use std::path::PathBuf;
use std::{path::Path, process::Command};

use crate::plugin_api::{Plugin, Target};

/// A Lua-based plugin implementation for the `Plugin` trait.
pub struct LuaPlugin {
    name: String,
    lua: &'static Lua,
    tbl: Table<'static>,
    path: PathBuf,
}

impl LuaPlugin {
    pub fn load(path: &Path) -> Result<Self> {
        let code = std::fs::read_to_string(path)?;
    
        // Create Lua context and convert to static
        let lua = Lua::new().into_static();
    
        // Evaluate the Lua code, expecting it to return a table
        let plugin_tbl: Table = lua.load(&code).eval()?; // <-- full version you asked for
    
        // Debug: print keys returned in the plugin table
        for pair in plugin_tbl.clone().pairs::<String, mlua::Value>() {
            let (k, _) = pair?;
            println!("[debug] Lua key: {}", k);
        }
    
        // Extract the plugin name from the table
        let name_val = plugin_tbl.get::<_, mlua::Value>("name")?;
        let name = match name_val {
            mlua::Value::String(s) => s.to_str()?.to_owned(),
            mlua::Value::Function(f) => f.call::<_, String>(())?,
            _ => anyhow::bail!("Expected 'name' to be string or function"),
        };
    
        // Transmute the plugin table to 'static now that Lua is static
        let tbl: Table<'static> = unsafe { std::mem::transmute(plugin_tbl) };
    
        Ok(LuaPlugin {
            name,
            lua,
            tbl,
            path: path.to_path_buf(),
        })
    }
}

impl Plugin for LuaPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn matches(&self, dir: &Path) -> bool {
        // Call the Lua `matches(dir)` function
        let f: Function = self.tbl.get("matches").unwrap();
        f.call(dir.to_string_lossy().as_ref()).unwrap_or(false)
    }

    fn collect_targets(&self, dir: &Path) -> Result<Vec<Target>> {
        // Call the Lua `collect_targets(dir)` function, which returns JSON
        let f: Function = self.tbl.get("collect_targets")?;
        let json: String = f.call(dir.to_string_lossy().as_ref())?;
        let v: Vec<Target> = serde_json::from_str(&json)?;
        Ok(v)
    }

    fn build_command(&self, dir: &Path, target: &Target) -> Result<Command> {
        // Call the Lua `build_command(dir, target_name)` function, which returns JSON
        let f: Function = self.tbl.get("build_command")?;
        let b = dir.to_string_lossy();
        let dir_str = b.as_ref();
        let target_str = target.name.as_str();
        // let json: String = f.call((dir_str, target_str))?;
        // let spec: CommandSpec = serde_json::from_str(&json)?;
        let json: String = f.call((dir_str, target_str))
    .map_err(|e| anyhow::anyhow!("Lua error calling build_command: {:?}", e))?;

let spec: CommandSpec = serde_json::from_str(&json)
    .map_err(|e| anyhow::anyhow!("Invalid JSON from Lua: {:?}\nOriginal: {}", e, json))?;
        Ok(spec.into_command(dir))
    }
    fn source(&self) -> Option<String> {
        Some(self.path.to_string_lossy().into())
    }
}

/// Internal structure matching the JSON command spec returned by Lua
#[derive(serde::Deserialize)]
pub struct CommandSpec {
    pub prog: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

impl CommandSpec {
    /// Convert the spec into a `std::process::Command`, defaulting to `default_dir` if `cwd` is None
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
