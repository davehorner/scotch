#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! cargo-e = "*"
//! eframe = "0.31.1"
//! egui = "0.31.1"
//! clap = { version = "4.5", features = ["derive"] }
//! ```

use cargo_e::e_collect::collect_all_targets;
use cargo_e::e_target::{CargoTarget, TargetKind};
use cargo_e::Cli;
use clap::Parser;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_from(vec!["cargo-e"]);
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let targets = collect_all_targets(true, num_threads)?;
    let mut seen = HashSet::new();
    let unique: Vec<CargoTarget> = targets
        .into_iter()
        .filter(|t| seen.insert((t.name.clone(), t.kind)))
        .collect();

    let app = TargetGuiApp::new(unique);
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "cargo-e Target Viewer",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );

    Ok(())
}

#[derive(PartialEq, Eq)]
enum SortMode {
    Name,
    Kind,
    Path,
}

struct TargetGuiApp {
    targets: Arc<Mutex<Vec<CargoTarget>>>,
    sort_mode: SortMode,
    run_result: Arc<Mutex<Option<String>>>,
}

impl TargetGuiApp {
    fn new(targets: Vec<CargoTarget>) -> Self {
        Self {
            targets: Arc::new(Mutex::new(targets)),
            sort_mode: SortMode::Name,
            run_result: Arc::new(Mutex::new(None)),
        }
    }

    fn sort_targets(&self, targets: &mut [CargoTarget]) {
        match self.sort_mode {
            SortMode::Name => targets.sort_by(|a, b| a.name.cmp(&b.name)),
            SortMode::Kind => {
                targets.sort_by(|a, b| format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)))
            }
            SortMode::Path => targets.sort_by(|a, b| a.manifest_path.cmp(&b.manifest_path)),
        }
    }

    fn run_target(&self, target: &CargoTarget) {
        let mut cmd = Command::new(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        });

        let cargo_cmd = match target.kind {
            TargetKind::Example => format!("cargo run --example {}", target.name),
            TargetKind::Binary => format!("cargo run --bin {}", target.name),
            _ => return,
        };

        if cfg!(target_os = "windows") {
            cmd.args(["/C", "start", "cmd", "/K", &cargo_cmd]);
        } else {
            cmd.args(["-c", &format!("x-terminal-emulator -e '{}'", cargo_cmd)]);
        }

        match cmd.spawn() {
            Ok(_) => {
                let mut lock = self.run_result.lock().unwrap();
                *lock = Some(format!("Running: {}", target.name));
            }
            Err(e) => {
                let mut lock = self.run_result.lock().unwrap();
                *lock = Some(format!("❌ Failed to run {}: {}", target.name, e));
            }
        }
    }
}

impl eframe::App for TargetGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🛠️ cargo-e Target Viewer");
            ui.separator();

            // Sorting dropdown
            egui::ComboBox::from_label("Sort by")
                .selected_text(match self.sort_mode {
                    SortMode::Name => "Name",
                    SortMode::Kind => "Kind",
                    SortMode::Path => "Manifest Path",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.sort_mode, SortMode::Name, "Name");
                    ui.selectable_value(&mut self.sort_mode, SortMode::Kind, "Kind");
                    ui.selectable_value(&mut self.sort_mode, SortMode::Path, "Manifest Path");
                });

            ui.separator();

            // Scrollable target list
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut targets = self.targets.lock().unwrap();
                self.sort_targets(&mut targets);

                for target in targets.iter() {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let kind = match target.kind {
                                    TargetKind::Example => "📘 Example",
                                    TargetKind::Binary => "🔧 Binary",
                                    TargetKind::Test => "🧪 Test",
                                    TargetKind::Bench => "📊 Bench",
                                    _ => "📄 Other",
                                };

                                ui.label(format!("[{}] {}", kind, target.display_name));

                                if matches!(target.kind, TargetKind::Example | TargetKind::Binary) {
                                    if ui.button("▶ Run").clicked() {
                                        self.run_target(target);
                                    }
                                }
                            });

                            ui.label(format!("📁 {}", target.manifest_path.display()));
                            // .wrap(true)
                            // .small();
                        });
                    });
                    ui.add_space(4.0);
                }
            });

            // Run status
            if let Some(msg) = &*self.run_result.lock().unwrap() {
                ui.separator();
                ui.label(msg);
            }
        });
    }
}
