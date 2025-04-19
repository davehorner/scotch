mod plugin_api;
mod lua_plugin;
mod rhai_plugin;
mod wasm_plugin;
mod scotch_plugin;
mod export_plugin;
use std::io;
use anyhow::Result;
use plugin_api::{Plugin, load_plugins};
use serde_json;
use std::path::Path;
use std::collections::HashSet;
use std::io::Write;
 use std::env;

fn main() -> Result<()> {
    let cwd = env::current_dir()?;
    let plugins = load_plugins()?;
    let mut all_targets = Vec::new();

    for plugin in &plugins {
        println!("[debug] trying plugin: {}", plugin.name());
        if plugin.matches(&cwd) {
            println!("[debug] plugin matched: {}", plugin.name());
            let targets = plugin.collect_targets(&cwd)?;
            for target in targets {
                println!(
                    "[debug] found target: '{}' from plugin '{}'",
                    target.name, plugin.name()
                );
                all_targets.push((plugin.name().to_string(), plugin, target));
            }
        }
    }

    if all_targets.is_empty() {
        eprintln!("No matching targets found.");
        std::process::exit(1);
    }
    // Deduplicate entries: skip targets with same plugin, target name, and source path
    let mut seen = HashSet::new();
    all_targets.retain(|(plugin_name, plugin, target)| {
        let source = plugin.source().unwrap_or_default();
        let key = (plugin_name.clone(), target.name.clone(), source.clone());
        seen.insert(key)
    });

    // Format and display available targets
    let formatted = format_targets(&all_targets);
    println!("Available targets:");
    for (i, line) in formatted.iter().enumerate() {
        println!("  {}: {}", i, line);
    }

    print!("Select a target by number: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let index: usize = input.trim().parse()?;
    let (plugin_name, plugin, target) = &all_targets[index];

    println!("Running target '{}' from plugin '{}'", target.name, plugin_name);
    // If this is a Rhai script plugin, run in-process via the embedded engine
    if let Some(src) = plugin.source() {
        if src.ends_with(".rhai") {
            // Run Rhai plugin in-process
            let output = plugin.run(&cwd, target)?;
            if !output.is_empty() {
                // First element: exit code
                let code_str = &output[0];
                let code = code_str.parse::<i32>().unwrap_or(0);
                // Display exit code
                println!("Exited with code: {}", code);
                // Print remaining output lines
                for line in &output[1..] {
                    println!("{}", line);
                }
                std::process::exit(code);
            } else {
                std::process::exit(0);
            }
        }
    }
    // For other plugins, build and run via external command
    let mut cmd = plugin.build_command(&cwd, target)?;
    // Special handling for export- and scotch-based plugins (WASM/DLL invocation)
    if plugin_name == "wasm-export" || plugin_name == "dll-export" || plugin_name == "scotch" {
        let output = cmd.output()?;
        if !output.status.success() {
            eprintln!("Error running {}: {}", plugin_name, String::from_utf8_lossy(&output.stderr));
            std::process::exit(output.status.code().unwrap_or(1));
        }
        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // If JSON array, print each element
        if out.starts_with('[') {
            if let Ok(arr) = serde_json::from_str::<Vec<String>>(&out) {
                for s in arr {
                    println!("{}", s);
                }
                std::process::exit(0);
            }
        }
        // Otherwise, try parse integer as exit code
        if let Ok(code) = out.parse::<i32>() {
            std::process::exit(code);
        }
        // Fallback: print raw output and exit success
        println!("{}", out);
        std::process::exit(0);
    }
    // Default: propagate child exit status
    let status = cmd.status()?;
    println!("Exited with: {}", status);
    std::process::exit(status.code().unwrap_or(1));
}

/// Helper to format targets into display strings
fn format_targets(
    all_targets: &Vec<(String, &Box<dyn Plugin>, plugin_api::Target)>
) -> Vec<String> {
    all_targets
        .iter()
        .map(|(plugin_name, plugin, target)| {
            // Determine display type: override generic export types to show the module name
            let display_type = if plugin_name == "wasm-export" || plugin_name == "dll-export" {
                if let Some(src) = plugin.source() {
                    Path::new(&src)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&plugin_name)
                        .to_string()
                } else {
                    plugin_name.clone()
                }
            } else {
                plugin_name.clone()
            };
            // Include optional source info
            let source_info = plugin
                .source()
                .map(|src| format!(" ({})", src))
                .unwrap_or_default();
            format!("[{}] {}{}", display_type, target.name, source_info)
        })
        .collect()
}
