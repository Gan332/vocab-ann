mod models;
mod db;
mod parser;
mod theme;
mod app;

use eframe::egui;
use app::VocabApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 720.0])
            .with_min_inner_size([360.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native("Vocab App", options, Box::new(|_cc| {
        let db_path = get_db_path();
        Ok(Box::new(VocabApp::new(&db_path)))
    }))
}

fn get_db_path() -> String {
    let mut path = std::env::current_exe().unwrap_or_default();
    path.pop();
    path.push("vocab_app.db");
    path.to_string_lossy().to_string()
}