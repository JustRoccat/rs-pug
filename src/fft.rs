use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
#[derive(Debug)]
pub struct FftState {
    pub bands: Vec<f64>,
    pub running: bool,
}
enum CaptureBackend {
    Parec,
    PwCatRaw,
    PwRecord,
}
const MPV_CLIENT_NAME: &str = "rs-pug";
async fn find_mpv_sink_input_index() -> Option<u32> {
    let output = Command::new("pactl")
        .args(["list", "sink-inputs"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_index: Option<u32> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Sink Input #") {
            current_index = rest.trim().parse().ok();
        } else if trimmed.starts_with("application.name")
            && trimmed.contains(&format!("\"{MPV_CLIENT_NAME}\""))
        {
            if let Some(idx) = current_index {
                return Some(idx);
            }
        }
    }
    None
}
async fn find_default_sink_id() -> Option<String> {
    let output = Command::new("wpctl").arg("status").output().await.ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let after_sinks = text.split("Sinks:").nth(1)?;
    let sinks_section = after_sinks.split("Sources:").next().unwrap_or(after_sinks);
    for line in sinks_section.lines() {
        let Some(pos) = line.find('*') else {
            continue;
        };
        let digits: String = line[pos + 1..]
            .trim()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return Some(digits);
        }
    }
    None
}
async fn spawn_capture() -> Option<(Child, CaptureBackend)> {
    let mut parec_args = vec![
        "--format=s16le".to_string(), "--channels=1".to_string(), "--rate=44100"
        .to_string(), "--latency-msec=50".to_string(),
    ];
    if let Some(idx) = find_mpv_sink_input_index().await {
        parec_args.push(format!("--monitor-stream={idx}"));
    } else {
        parec_args.push("-d".to_string());
        parec_args.push("@DEFAULT_SINK@.monitor".to_string());
    }
    if let Ok(child) = Command::new("parec")
        .args(&parec_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        return Some((child, CaptureBackend::Parec));
    }
    let default_sink = find_default_sink_id().await;
    let mut pw_cat_args = vec![
        "--record".to_string(), "--raw".to_string(), "--monitor".to_string(),
        "--format=s16".to_string(), "--rate=44100".to_string(), "--channels=1"
        .to_string(), "--latency=50ms".to_string(),
    ];
    if let Some(ref id) = default_sink {
        pw_cat_args.push(format!("--target={id}"));
    }
    pw_cat_args.push("-".to_string());
    if let Ok(child) = Command::new("pw-cat")
        .args(&pw_cat_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        return Some((child, CaptureBackend::PwCatRaw));
    }
    let mut pw_record_args = vec![
        "--monitor".to_string(), "--format=s16".to_string(), "--rate=44100".to_string(),
        "--channels=1".to_string(), "--latency=50ms".to_string(),
    ];
    if let Some(id) = default_sink {
        pw_record_args.push(format!("--target={id}"));
    }
    pw_record_args.push("-".to_string());
    if let Ok(child) = Command::new("pw-record")
        .args(&pw_record_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        return Some((child, CaptureBackend::PwRecord));
    }
    None
}
async fn skip_wav_header<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<()> {
    let mut window = [0u8; 4];
    let mut have = 0usize;
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).await?;
        if have < 4 {
            window[have] = byte[0];
            have += 1;
        } else {
            window.copy_within(1..4, 0);
            window[3] = byte[0];
        }
        if have == 4 && &window == b"data" {
            let mut size_buf = [0u8; 4];
            reader.read_exact(&mut size_buf).await?;
            return Ok(());
        }
    }
}
pub fn start_fft_monitor() -> Arc<Mutex<FftState>> {
    let state = Arc::new(
        Mutex::new(FftState {
            bands: vec![0.0; 32],
            running: false,
        }),
    );
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let (mut child, backend) = match spawn_capture().await {
            Some(pair) => pair,
            None => {
                return;
            }
        };
        let mut stdout = match child.stdout.take() {
            Some(s) => BufReader::new(s),
            None => return,
        };
        if matches!(backend, CaptureBackend::PwRecord) {
            if skip_wav_header(&mut stdout).await.is_err() {
                return;
            }
        }
        {
            let mut s = state_clone.lock().unwrap();
            s.running = true;
        }
        let mut buffer = [0u8; 2048];
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(1024);
        const NUM_BANDS: usize = 32;
        const USABLE_BINS: usize = 512;
        let mut band_peaks: [f32; NUM_BANDS] = [1e-3; NUM_BANDS];
        let mut smoothed = vec![0.0f64; NUM_BANDS];
        loop {
            match stdout.read_exact(&mut buffer).await {
                Ok(_) => {
                    let mut input: Vec<Complex<f32>> = buffer
                        .chunks_exact(2)
                        .map(|chunk| {
                            let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32
                                / 32768.0;
                            Complex { re: sample, im: 0.0 }
                        })
                        .collect();
                    if input.len() == 1024 {
                        fft.process(&mut input);
                        let mut raw_bands = [0.0f32; NUM_BANDS];
                        let mut lower_bin = 1usize;
                        for (i, band) in raw_bands.iter_mut().enumerate() {
                            let frac = (i + 1) as f32 / NUM_BANDS as f32;
                            let target = (USABLE_BINS as f32).powf(frac).round()
                                as usize;
                            let upper_bin = target.max(lower_bin + 1).min(USABLE_BINS);
                            let mut sum = 0.0f32;
                            let mut count = 0usize;
                            for bin in lower_bin..upper_bin {
                                if bin < input.len() {
                                    sum += input[bin].norm();
                                    count += 1;
                                }
                            }
                            *band = if count > 0 { sum / count as f32 } else { 0.0 };
                            lower_bin = upper_bin;
                        }
                        for i in 0..NUM_BANDS {
                            band_peaks[i] = (band_peaks[i] * 0.995)
                                .max(raw_bands[i])
                                .max(1e-3);
                            let ratio = (raw_bands[i] / band_peaks[i]).clamp(0.0, 1.0);
                            let normalized = (ratio.powf(1.5) * 0.9) as f64;
                            if normalized > smoothed[i] {
                                smoothed[i] = smoothed[i] * 0.45 + normalized * 0.55;
                            } else {
                                smoothed[i] = smoothed[i] * 0.85 + normalized * 0.15;
                            }
                        }
                        {
                            let mut s = state_clone.lock().unwrap();
                            s.bands = smoothed.clone();
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
        {
            let mut s = state_clone.lock().unwrap();
            s.running = false;
        }
        let _ = child.kill().await;
    });
    state
}
