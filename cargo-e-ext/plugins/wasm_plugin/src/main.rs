use clap::{App, Arg};
use serde_json::json;
use std::{env, fs};

fn main() {
    let matches = App::new("WASM Plugin")
        .arg(Arg::with_name("matches").long("matches").takes_value(true))
        .arg(Arg::with_name("collect_targets").long("collect_targets").takes_value(true))
        .arg(Arg::with_name("build_command").long("build_command").number_of_values(2))
        .get_matches();

    if let Some(dir) = matches.value_of("matches") {
    let has_pkg = fs::metadata(format!("{}/package.json", dir)).is_ok();
    println!("{}", has_pkg);
        // println!("{}", fs::metadata(format!("{}/package.json", dir)).is_ok());
    } else if let Some(dir) = matches.value_of("collect_targets") {
        eprintln!("[wasm debug] collect_targets dir: {}", dir);
        let data = fs::read_to_string(format!("{}/package.json", dir)).unwrap_or("{}".into());
        eprintln!("[wasm debug] loaded package.json: {}", data);
    
        let pkg: serde_json::Value = serde_json::from_str(&data).unwrap_or(json!({}));
        let empty = serde_json::Map::new();

        // let scripts = pkg.get("scripts").and_then(|v| v.as_object()).unwrap_or(&empty);
        // eprintln!("[wasm debug] found scripts: {:?}", scripts.keys().collect::<Vec<_>>());
    
        let mut targets = Vec::new();
                match pkg.get("scripts") {
    Some(serde_json::Value::Object(obj)) => {
        eprintln!("[wasm debug] scripts keys: {:?}", obj.keys().collect::<Vec<_>>());
        for (k, _) in obj {
            targets.push(json!({ "name": k, "metadata": null }));
        }
    }
    other => {
        eprintln!("[wasm warn] scripts field not an object: {:?}", other);
    }
}
    
        let output = serde_json::to_string(&targets).unwrap();
        println!("{}", output);
    } else if let Some(vals) = matches.values_of("build_command") {
        let args: Vec<&str> = vals.collect();
        let name = args[1];
        let spec = json!({
            "prog": "npm",
            "args": ["run", name],
            "cwd": null
        });
        println!("{}", spec.to_string());
    }
}
