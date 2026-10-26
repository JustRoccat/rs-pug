use tokio::sync::mpsc;

use crate::core::CoreCmd;
use crate::model::App;

pub fn build_af(bands: [f32; 10], normalize: bool, fade_secs: u8) -> String {
    let freqs = [32, 64, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
    let parts: Vec<String> = bands
        .iter()
        .zip(freqs.iter())
        .map(|(gain, freq)| {
            format!("equalizer=frequency={freq}:gain={gain}:width_type=o:width=1.5")
        })
        .collect();
    let mut filter = parts.join(",");
    if normalize {
        if !filter.is_empty() {
            filter.push(',');
        }
        filter.push_str("loudnorm=I=-16:TP=-1.5:LRA=11");
    }
    let fade = fade_secs.min(10);
    if fade > 0 {
        if !filter.is_empty() {
            filter.push(',');
        }
        filter.push_str(&format!("afade=t=in:st=0:d={fade}"));
    }
    filter
}

pub fn send_audio_filters(app: &App, cmd_tx: &mpsc::UnboundedSender<CoreCmd>) {
    let bands = if app.eq.enabled {
        app.eq.bands
    } else {
        [0.0f32; 10]
    };
    let filter = build_af(bands, app.opt_audio_normalize, app.opt_fade_secs);
    let _ = cmd_tx.send(CoreCmd::RawMpv(serde_json::json!([
        "set_property",
        "af",
        filter
    ])));
}

pub fn apply_startup_audio(cmd_tx: &mpsc::UnboundedSender<CoreCmd>, app: &App) {
    send_audio_filters(app, cmd_tx);
    let _ = cmd_tx.send(CoreCmd::RawMpv(serde_json::json!([
        "set_property",
        "replaygain",
        crate::config::mpv_replaygain(&app.opt_replaygain_mode)
    ])));
    let _ = cmd_tx.send(CoreCmd::RawMpv(serde_json::json!([
        "set_property",
        "gapless-audio",
        app.opt_gapless
    ])));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn af_contains_eq_and_optional_filters() {
        let base = build_af([0.0; 10], false, 0);
        assert!(base.contains("equalizer"));
        assert!(!base.contains("loudnorm"));
        assert!(!base.contains("afade"));
        let norm = build_af([0.0; 10], true, 0);
        assert!(norm.contains("loudnorm"));
        let fade = build_af([0.0; 10], false, 2);
        assert!(fade.contains("afade"));
        let both = build_af([1.0; 10], true, 3);
        assert!(both.contains("loudnorm") && both.contains("afade"));
    }
}
