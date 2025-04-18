// This example provides a cross-platform build driver for the my_scotch plugin.
use std::process::Command;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    fn run(cmd: &str, args: &[&str]) -> io::Result<()> {
        let status = Command::new(cmd)
            .args(args)
            .status()?;
        if !status.success() {
            panic!("`{:?} {:?}` failed with exit code {}", cmd, args, status.code().unwrap_or(-1));
        }
        Ok(())
    }

    println!("Building native cdylib…");
    run("cargo", &["build"])?;

    println!("\nEnsuring wasm32-unknown-unknown target is installed…");
    let _ = Command::new("rustup")
        .args(&["target", "add", "wasm32-unknown-unknown"]);

    println!("Building WebAssembly (release)…");
    run("cargo", &["build", "--release", "--target", "wasm32-unknown-unknown"])?;

    println!("\n🎉  Build complete! 🎉");
    println!(" - native dynamic library: target/debug or target/release/libmy_scotch.*");
    println!(" - wasm module: target/wasm32-unknown-unknown/release/my_scotch.wasm");

    io::stdout().flush()?;
    Ok(())
}