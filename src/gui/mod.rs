pub mod app;
pub mod charts;
pub mod dashboard;
pub mod events;
pub mod history;
pub mod settings;

use eframe::egui;
use std::fs;
use tracing::info;

pub use app::RustNetApp;

/// Configura as fontes padrão do aplicativo com prioridade para Segoe UI nativa do Windows.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Caminhos padrão da Segoe UI no Windows 10 e Windows 11
    let segoe_paths = [
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\SegoeUI.ttf",
    ];

    let mut loaded = false;
    for path in segoe_paths {
        if let Ok(font_data) = fs::read(path) {
            fonts.font_data.insert(
                "SegoeUI".to_string(),
                egui::FontData::from_owned(font_data).into(),
            );

            // Define Segoe UI como a primeira prioridade para Proportional
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "SegoeUI".to_string());

            info!("Fonte nativa do Windows carregada com sucesso: {}", path);
            loaded = true;
            break;
        }
    }

    if !loaded {
        info!("Segoe UI não encontrada no caminho padrão, utilizando fontes do sistema egui.");
    }

    ctx.set_fonts(fonts);
}
