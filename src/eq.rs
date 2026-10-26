use crate::core::CoreCmd;
use crate::model::{App, EQ_PRESET_NAMES, eq_preset_bands, eq_preset_name};
use serde_json::json;
use tokio::sync::mpsc;
pub fn cycle_eq_preset(app: &mut App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>, delta: isize) {
    let total = (EQ_PRESET_NAMES.len() + app.eq.custom_presets.len()) as isize;
    let next = (app.eq.preset_index as isize + delta).rem_euclid(total) as usize;
    app.eq.preset_index = next;
    app.eq.bands = eq_preset_bands(app, next);
    if app.eq.enabled {
        send_eq_update(cmd_tx, app.eq.bands);
    }
    app.set_flash(format!("EQ preset: {}", eq_preset_name(app, next)), 2);
}
pub fn send_eq_update(cmd_tx: &mpsc::UnboundedSender<CoreCmd>, bands: [f32; 10]) {
    let filter = crate::audio::build_af(bands, false, 0);
    let _ = cmd_tx.send(CoreCmd::RawMpv(json!(["set_property", "af", filter])));
}
pub fn send_audio_update(app: &App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>) {
    crate::audio::send_audio_filters(app, cmd_tx);
}
