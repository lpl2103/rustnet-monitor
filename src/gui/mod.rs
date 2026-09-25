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

    // Carrega Segoe UI Symbol / Emoji como fallback nativo para símbolos Unicode
    let symbol_paths = [
        "C:\\Windows\\Fonts\\seguisym.ttf",
        "C:\\Windows\\Fonts\\seguiemj.ttf",
    ];
    for path in symbol_paths {
        if let Ok(font_data) = fs::read(path) {
            fonts.font_data.insert(
                "SegoeSymbol".to_string(),
                egui::FontData::from_owned(font_data).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("SegoeSymbol".to_string());
            break;
        }
    }

    ctx.set_fonts(fonts);

    // Configura estilos e tamanhos de texto aumentados para excelente legibilidade
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(22.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Name("Subheading".into()),
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(15.5, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(15.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(14.0, egui::FontFamily::Monospace),
            ),
        ]
        .into();

        style.spacing.item_spacing = egui::vec2(10.0, 7.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
    });
}
