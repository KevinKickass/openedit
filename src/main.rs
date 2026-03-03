use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    env_logger::init();

    // Collect file paths from command line args
    let files: Vec<PathBuf> = std::env::args()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenEdit")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OpenEdit",
        options,
        Box::new(move |_cc| Ok(Box::new(openedit_ui::OpenEditApp::new(files)))),
    )
}
