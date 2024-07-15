use egui::{vec2, Ui};

use crate::{MaximaEguiApp, APP_MARGIN};

pub struct QueuedDownload {
    pub slug: String,
    pub downloaded_bytes: usize,
    pub total_bytes: usize,
    // maybe add a thing here for updates? idk there's no real api to hook this up to yet
}

pub fn downloads_view(app : &mut MaximaEguiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = APP_MARGIN.y;
    for game in &app.install_queue {
        ui.allocate_ui(vec2(ui.available_width(), 260.0), |ui| {
            let game = app.games.get(&game.slug).unwrap();
            ui.label(&game.name);
        });
    }
}