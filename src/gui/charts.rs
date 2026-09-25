use eframe::egui::{self, Color32, RichText, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use std::collections::HashMap;

/// Cores distintas para as linhas de cada host no gráfico
const HOST_COLORS: &[Color32] = &[
    Color32::from_rgb(52, 152, 219), // Azul
    Color32::from_rgb(46, 204, 113), // Verde
    Color32::from_rgb(241, 196, 15), // Amarelo
    Color32::from_rgb(155, 89, 182), // Roxo
    Color32::from_rgb(230, 126, 34), // Laranja
    Color32::from_rgb(26, 188, 156), // Turquesa
];

pub fn render_charts(
    ui: &mut Ui,
    history_points: &HashMap<String, Vec<(f64, f64)>>,
    active_hosts: &mut HashMap<String, bool>,
) {
    ui.horizontal(|ui| {
        ui.heading(RichText::new("Gráficos de Latência em Tempo Real").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("Use o scroll do mouse para zoom e arraste para mover").weak());
        });
    });

    ui.separator();

    // Filtros de seleção de hosts visíveis
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Hosts exibidos:").strong());
        for host_name in history_points.keys() {
            let is_active = active_hosts.entry(host_name.clone()).or_insert(true);
            ui.checkbox(is_active, host_name);
        }
    });

    ui.add_space(8.0);

    let plot = Plot::new("latency_realtime_plot")
        .legend(Legend::default())
        .y_axis_label("Latência (ms)")
        .x_axis_label("Tempo decorrido (segundos)")
        .height(ui.available_height() - 20.0);

    plot.show(ui, |plot_ui| {
        for (i, (host_name, points)) in history_points.iter().enumerate() {
            if !*active_hosts.get(host_name).unwrap_or(&true) {
                continue;
            }

            let color = HOST_COLORS[i % HOST_COLORS.len()];
            let plot_points: PlotPoints = points.iter().map(|&(x, y)| [x, y]).collect();
            let line = Line::new(host_name.clone(), plot_points)
                .color(color)
                .width(2.0);

            plot_ui.line(line);
        }
    });
}
