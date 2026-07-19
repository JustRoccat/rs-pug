use tokio::sync::mpsc;
use serde_json::json;
use crate::model::{App, EQ_PRESET_NAMES, eq_preset_bands, eq_preset_name};
use crate::core::CoreCmd;
pub fn cycle_eq_preset(
    app: &mut App,
    cmd_tx: &mpsc::UnboundedSender<CoreCmd>,
    delta: isize,
) {
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
    let freqs = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
    let parts: Vec<String> = bands
        .iter()
        .zip(freqs.iter())
        .map(|(gain, freq)| {
            format!("equalizer=frequency={}:gain={}:width_type=o:width=1.5", freq, gain)
        })
        .collect();
    let filter = parts.join(",");
    let _ = cmd_tx.send(CoreCmd::RawMpv(json!(["set_property", "af", filter])));
}
