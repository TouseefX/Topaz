// Desktop binary entry point - Android uses android_main in lib.rs
use topaz_gui::TopazApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([960.0, 680.0])
            .with_min_inner_size([640.0, 440.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Topaz",
        options,
        Box::new(|cc| Ok(Box::new(TopazApp::new(cc)))),
    )
}
