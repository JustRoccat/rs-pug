use crate::model::Song;
use lofty::file::TaggedFileExt;
use std::path::PathBuf;
use std::time::Duration;

pub struct CoverResult {
    pub key: String,
    pub bytes: Option<Vec<u8>>,
    pub bitmap: Option<image::DynamicImage>,
    pub palette: Option<[[u8; 3]; 3]>,
}

// Cover key per song
pub fn song_key(song: &Song) -> String {
    format!(
        "{}|{}|{}",
        song.id,
        song.title,
        song.uploader.clone().unwrap_or_default()
    )
}

// Local path or none
pub fn local_path_for(song: &Song) -> Option<PathBuf> {
    for cand in [&song.webpage_url, &song.id] {
        let s = cand.trim();
        if s.is_empty() {
            continue;
        }
        let s = s.strip_prefix("file://").unwrap_or(s);
        let expanded = if let Some(rest) = s.strip_prefix('~') {
            match std::env::var("HOME") {
                Ok(home) => format!("{home}{rest}"),
                Err(_) => s.to_owned(),
            }
        } else {
            s.to_owned()
        };
        let path = PathBuf::from(&expanded);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

// Embedded art
pub fn embedded_for_song(song: &Song) -> Option<Vec<u8>> {
    let path = local_path_for(song)?;
    embedded_bytes(&path)
}

fn embedded_bytes(path: &PathBuf) -> Option<Vec<u8>> {
    let tagged = lofty::read_from_path(path).ok()?;
    let tag = tagged.primary_tag()?;
    let pic = tag.pictures().first()?;
    let data = pic.data();
    if data.is_empty() {
        return None;
    }
    if data.len() > 24 * 1024 * 1024 {
        return None;
    }
    Some(data.to_vec())
}

// Load cover bytes
pub fn load_blocking(song: &Song) -> Option<Vec<u8>> {
    if let Some(bytes) = embedded_for_song(song) {
        return Some(bytes);
    }
    if let Some(bytes) = sonum_art_bytes(song) {
        return Some(bytes);
    }
    remote_bytes(song)
}

// Fetch Sonum cover
fn sonum_art_bytes(song: &Song) -> Option<Vec<u8>> {
    let config = crate::sonum::load_sonum_config();
    let base = config.base_url();
    if !song.webpage_url.starts_with(&base) {
        return None;
    }
    let id = song.id.trim();
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
    {
        return None;
    }
    let url = format!("{base}/tracks/{id}/art?size=thumbnail&placeholder=false");
    let mut request = ureq::get(&url)
        .config()
        .timeout_global(Some(Duration::from_secs(15)))
        .build();
    if let Some(token) = &config.api_token {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let mut response = request.call().ok()?;
    let body = response.body_mut().read_to_vec().ok()?;
    if body.is_empty() || body.len() > 12 * 1024 * 1024 {
        return None;
    }
    if image::guess_format(&body).is_err() {
        return None;
    }
    Some(body)
}

// Decode cover bytes
pub fn decode_cover(bytes: &[u8]) -> Option<image::DynamicImage> {
    let img = image::load_from_memory(bytes).ok()?;
    Some(crop_landscape_to_square(img))
}

// Top 3 cover colors
pub fn palette3(img: &image::DynamicImage) -> [[u8; 3]; 3] {
    let small = img.resize_exact(24, 24, image::imageops::FilterType::Nearest);
    let small = small.to_rgba8();
    let mut count = [0u32; 4096];
    let mut sum = [[0u64; 3]; 4096];
    let mut total = [0u64; 3];
    let mut n = 0u64;
    for px in small.pixels() {
        let p = px.0;
        if p[3] < 128 {
            continue;
        }
        let (r, g, b) = (p[0] as u64, p[1] as u64, p[2] as u64);
        let idx =
            (((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4)) as usize;
        count[idx] += 1;
        sum[idx][0] += r;
        sum[idx][1] += g;
        sum[idx][2] += b;
        total[0] += r;
        total[1] += g;
        total[2] += b;
        n += 1;
    }
    if n == 0 {
        return [normalize_accent([128, 128, 128]); 3];
    }
    let mut scored: Vec<(f64, [u8; 3])> = Vec::new();
    for (i, &c) in count.iter().enumerate() {
        if c == 0 {
            continue;
        }
        let avg = [
            (sum[i][0] as f64 / c as f64) as u8,
            (sum[i][1] as f64 / c as f64) as u8,
            (sum[i][2] as f64 / c as f64) as u8,
        ];
        let mx = avg[0].max(avg[1]).max(avg[2]) as f64;
        let mn = avg[0].min(avg[1]).min(avg[2]) as f64;
        let saturation = if mx <= 0.0 { 0.0 } else { (mx - mn) / mx };
        scored.push((c as f64 * (0.15 + saturation), avg));
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let average = [
        (total[0] / n) as u8,
        (total[1] / n) as u8,
        (total[2] / n) as u8,
    ];
    let average = normalize_accent(average);
    let mut stops = [average, average, average];
    for (k, (_, color)) in scored.into_iter().take(3).enumerate() {
        stops[k] = normalize_accent(color);
    }
    stops
}

// Boost dark colors
fn normalize_accent([r, g, b]: [u8; 3]) -> [u8; 3] {
    let m = r.max(g).max(b);
    if m == 0 {
        return [85, 85, 85];
    }
    let s = (140.0 / m as f32).min(6.0);
    [
        (r as f32 * s).min(255.0) as u8,
        (g as f32 * s).min(255.0) as u8,
        (b as f32 * s).min(255.0) as u8,
    ]
}

// Grow small covers
pub fn upscale_to_fill(
    img: image::DynamicImage,
    cols: u16,
    rows: u16,
    font_w: u16,
    font_h: u16,
) -> image::DynamicImage {
    let (iw, ih) = (img.width().max(1) as f64, img.height().max(1) as f64);
    let target_w = cols.max(1) as f64 * font_w.max(1) as f64;
    let target_h = rows.max(1) as f64 * font_h.max(1) as f64;
    let scale = (target_w / iw).max(target_h / ih);
    if scale <= 1.0 {
        return img;
    }
    let (nw, nh) = (
        ((iw * scale) as u32).max(1),
        ((ih * scale) as u32).max(1),
    );
    img.resize_exact(nw, nh, image::imageops::FilterType::Triangle)
}

// Crop wide thumbs
pub fn crop_landscape_to_square(img: image::DynamicImage) -> image::DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return img;
    }
    if (w as u64) * 4 > (h as u64) * 5 {
        let side = h;
        let x = (w - side) / 2;
        img.crop_imm(x, 0, side, side)
    } else {
        img
    }
}

fn remote_bytes(song: &Song) -> Option<Vec<u8>> {
    let url = song.webpage_url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let thumb = thumbnail_url(url)?;
    if thumb.len() > 2048 {
        return None;
    }
    fetch_bytes(&thumb)
}

// Get thumb URL
fn thumbnail_url(page_url: &str) -> Option<String> {
    let out = std::process::Command::new("yt-dlp")
        .arg("--no-playlist")
        .arg("--skip-download")
        .arg("--print")
        .arg("thumbnail")
        .arg(page_url)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && l != &"NA")?
        .to_owned();
    if first.starts_with("http://") || first.starts_with("https://") {
        Some(first)
    } else {
        None
    }
}

fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let mut resp = ureq::get(url)
        .config()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .call()
        .ok()?;
    let body = resp.body_mut().read_to_vec().ok()?;
    if body.is_empty() || body.len() > 12 * 1024 * 1024 {
        return None;
    }
    if image::guess_format(&body).is_err() {
        return None;
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(webpage_url: &str, id: &str) -> Song {
        Song {
            id: id.to_owned(),
            title: "T".to_owned(),
            webpage_url: webpage_url.to_owned(),
            uploader: Some("A".to_owned()),
            duration: None,
        }
    }

    #[test]
    fn song_key_is_stable_and_unique() {
        let a = song("u", "1");
        let b = song("u", "1");
        let c = song("u", "2");
        assert_eq!(song_key(&a), song_key(&b));
        assert_ne!(song_key(&a), song_key(&c));
    }

    #[test]
    fn local_path_missing_returns_none() {
        let s = song("/definitely/not/here.mp3", "/also/not/here.mp3");
        assert!(local_path_for(&s).is_none());
        assert!(embedded_for_song(&s).is_none());
    }

    #[test]
    fn local_path_finds_existing_file() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_string_lossy().to_string();
        let s = song(&path, &path);
        assert_eq!(local_path_for(&s).map(|p| p.to_string_lossy().to_string()), Some(path));
        assert!(embedded_for_song(&s).is_none());
    }

    #[test]
    fn remote_rejected_for_non_http() {
        let s = song("/local/file.mp3", "/local/file.mp3");
        assert!(remote_bytes(&s).is_none());
    }

    #[test]
    fn sonum_art_rejected_for_non_sonum_songs() {
        let s = song("https://www.youtube.com/watch?v=abc", "abc");
        assert!(sonum_art_bytes(&s).is_none());
    }

    #[test]
    fn sonum_art_rejects_path_traversal_ids() {
        let base = crate::sonum::load_sonum_config().base_url();
        for evil in ["../etc/passwd", "a/b", "a\\b", "", "  "] {
            let s = song(&format!("{base}/tracks/x/stream"), evil);
            assert!(sonum_art_bytes(&s).is_none(), "id {evil:?} rejected");
        }
    }

    #[test]
    fn fetch_rejects_non_image_bytes() {
        assert!(fetch_bytes("http://invalid.invalid/cover.jpg").is_none());
    }

    #[test]
    fn landscape_thumbs_crop_to_centered_square() {
        let mut img = image::RgbaImage::new(160, 90);
        for (x, _, px) in img.enumerate_pixels_mut() {
            *px = if x < 35 {
                image::Rgba([120, 80, 20, 255])
            } else if x < 125 {
                image::Rgba([200, 10, 10, 255])
            } else {
                image::Rgba([120, 80, 20, 255])
            };
        }
        let cropped =
            crop_landscape_to_square(image::DynamicImage::ImageRgba8(img));
        assert_eq!((cropped.width(), cropped.height()), (90, 90));
        let rgba = cropped.to_rgba8();
        assert_eq!(rgba.get_pixel(45, 45).0, [200, 10, 10, 255]);
        assert_eq!(rgba.get_pixel(0, 0).0, [200, 10, 10, 255]);

        let sq = image::DynamicImage::ImageRgba8(image::RgbaImage::new(90, 90));
        let sq = crop_landscape_to_square(sq);
        assert_eq!((sq.width(), sq.height()), (90, 90));
        let tall = image::DynamicImage::ImageRgba8(image::RgbaImage::new(90, 160));
        let tall = crop_landscape_to_square(tall);
        assert_eq!((tall.width(), tall.height()), (90, 160));
    }

    #[test]
    fn upscale_to_fill_grows_small_sonum_thumbs() {
        let small = image::DynamicImage::ImageRgba8(image::RgbaImage::new(120, 120));
        let grown = upscale_to_fill(small, 60, 20, 8, 16);
        assert!(grown.width() >= 480 && grown.height() >= 320);
        assert_eq!(
            (grown.width(), grown.height()),
            (480, 480),
            "square stays square"
        );
        let big = image::DynamicImage::ImageRgba8(image::RgbaImage::new(1280, 720));
        let kept = upscale_to_fill(big, 60, 20, 8, 16);
        assert_eq!((kept.width(), kept.height()), (1280, 720));
    }

    #[test]
    fn palette3_collects_three_vivid_hues() {
        let mut img = image::RgbaImage::new(90, 90);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = if y < 30 {
                image::Rgba([220, 20, 20, 255])
            } else if y < 60 {
                image::Rgba([20, 220, 20, 255])
            } else if x < 45 {
                image::Rgba([20, 20, 220, 255])
            } else {
                image::Rgba([15, 15, 15, 255])
            };
        }
        let stops = palette3(&image::DynamicImage::ImageRgba8(img));
        let has_hue = |want: usize| {
            stops.iter().any(|c| {
                c[want] > 100
                    && c[(want + 1) % 3] < 90
                    && c[(want + 2) % 3] < 90
            })
        };
        assert!(has_hue(0), "red present: {stops:?}");
        assert!(has_hue(1), "green present: {stops:?}");
        assert!(has_hue(2), "blue present: {stops:?}");
    }

    #[test]
    fn palette3_normalizes_dark_covers() {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            40,
            40,
            image::Rgba([12, 8, 30, 255]),
        ));
        let stops = palette3(&img);
        let m = stops[0].iter().max().unwrap();
        assert!(*m >= 100, "boosted visible: {stops:?}");
    }

    #[test]
    fn palette3_handles_flat_gray() {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            40,
            40,
            image::Rgba([120, 120, 120, 255]),
        ));
        let stops = palette3(&img);
        for c in &stops {
            assert!(
                (c[0] as i16 - c[1] as i16).abs() < 25
                    && (c[1] as i16 - c[2] as i16).abs() < 25,
                "near gray: {stops:?}"
            );
        }
    }

    #[test]
    fn decode_cover_roundtrips_png() {
        use image::ImageEncoder;
        let mut img = image::RgbaImage::new(64, 48);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = image::Rgba([(x * 4) as u8, (y * 5) as u8, 128, 255]);
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(
                img.as_raw(),
                64,
                48,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        let decoded = decode_cover(&png).expect("decodable");
        assert_eq!((decoded.width(), decoded.height()), (48, 48));
        assert!(decode_cover(b"definitely not an image").is_none());
    }
}
