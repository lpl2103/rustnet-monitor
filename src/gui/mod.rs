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

/// Configura as fontes padrão do aplicativo com Victor Mono Nerd Font embutida.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Carrega e descompacta a Victor Mono Nerd Font embutida no binário via zlib
    let compressed_font = include_bytes!("../../assets/victor_mono.deflate");
    match miniz_oxide::inflate::decompress_to_vec_zlib(compressed_font) {
        Ok(decompressed_ttf) => {
            fonts.font_data.insert(
                "VictorMonoNF".to_string(),
                egui::FontData::from_owned(decompressed_ttf).into(),
            );

            // Define Victor Mono Nerd Font como primeira prioridade para Proportional e Monospace
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "VictorMonoNF".to_string());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "VictorMonoNF".to_string());

            info!("Victor Mono Nerd Font embutida carregada com sucesso.");
        }
        Err(e) => {
            tracing::error!("Falha ao descompactar Victor Mono Nerd Font: {:?}", e);
        }
    }

    // 2. Carrega Segoe UI / Segoe Symbol do Windows como fallback secundário para emojis e símbolos
    let fallback_paths = [
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\seguisym.ttf",
        "C:\\Windows\\Fonts\\seguiemj.ttf",
    ];
    for path in fallback_paths {
        if let Ok(font_data) = fs::read(path) {
            let font_name = if path.contains("segoeui") {
                "SegoeUI"
            } else {
                "SegoeSymbol"
            };
            fonts.font_data.insert(
                font_name.to_string(),
                egui::FontData::from_owned(font_data).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(font_name.to_string());
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
