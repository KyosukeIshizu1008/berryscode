//! OracleBerry panel — AI generation UI wired to `../berry-core-api`.
//!
//! Four tabs: Text→Image, Image→Image, Image→Anime, 2D→3D. Each tab
//! packs its state into the API's request shape, fires the request
//! through the existing tokio runtime, and feeds the response (base64
//! PNG or base64 GLB) back to the UI thread via an mpsc channel.

use std::path::PathBuf;
use std::time::Instant;

use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::button_style::primary_button;
use super::ui_colors;
use super::BerryCodeApp;

const DEFAULT_API_HOST: &str = "192.168.10.147:7001";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleBerryTab {
    Text2Image,
    Image2Image,
    Image2Anime,
    Two2Three,
}

impl OracleBerryTab {
    pub fn label(self) -> &'static str {
        match self {
            OracleBerryTab::Text2Image => "Text2Image",
            OracleBerryTab::Image2Image => "Image2Image",
            OracleBerryTab::Image2Anime => "Image2Anime",
            OracleBerryTab::Two2Three => "2D23D",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProvider {
    ComfyUI,
    Trellis,
}

impl ImageProvider {
    pub fn label(self) -> &'static str {
        match self {
            ImageProvider::ComfyUI => "ComfyUI",
            ImageProvider::Trellis => "Trellis (3D)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSize {
    Square512,
    Square1024,
    Wide1536,
    Tall1536,
}

impl ImageSize {
    pub fn label(self) -> &'static str {
        match self {
            ImageSize::Square512 => "512 × 512",
            ImageSize::Square1024 => "1024 × 1024",
            ImageSize::Wide1536 => "1536 × 1024 (wide)",
            ImageSize::Tall1536 => "1024 × 1536 (tall)",
        }
    }
    pub fn dims(self) -> (u32, u32) {
        match self {
            ImageSize::Square512 => (512, 512),
            ImageSize::Square1024 => (1024, 1024),
            ImageSize::Wide1536 => (1536, 1024),
            ImageSize::Tall1536 => (1024, 1536),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimeStyle {
    Default,
    CelShaded,
    SoftPastel,
    LineArt,
    Painterly,
}

impl AnimeStyle {
    pub fn label(self) -> &'static str {
        match self {
            AnimeStyle::Default => "Default",
            AnimeStyle::CelShaded => "Cel-shaded",
            AnimeStyle::SoftPastel => "Soft pastel",
            AnimeStyle::LineArt => "Line art",
            AnimeStyle::Painterly => "Painterly",
        }
    }
    /// Style descriptors appended to the user prompt before the
    /// img2img call, so the model receives a coherent style request
    /// even when the user leaves the prompt blank.
    pub fn prompt_suffix(self) -> &'static str {
        match self {
            AnimeStyle::Default => "anime style, high quality",
            AnimeStyle::CelShaded => "anime style, cel-shaded, flat colors, clean lineart",
            AnimeStyle::SoftPastel => {
                "anime style, soft pastel colors, muted tones, gentle lighting"
            }
            AnimeStyle::LineArt => "anime style, line art, monochrome, ink on paper",
            AnimeStyle::Painterly => "anime style, painterly, visible brushstrokes, oil-paint look",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshOutput {
    PointCloud,
    Mesh,
}

impl MeshOutput {
    pub fn label(self) -> &'static str {
        match self {
            MeshOutput::PointCloud => "Point cloud (.ply)",
            MeshOutput::Mesh => "Mesh (.glb)",
        }
    }
}

#[derive(Clone)]
pub enum GenerationStatus {
    Pending,
    Done {
        texture: Option<egui::TextureHandle>,
        glb_path: Option<PathBuf>,
    },
    Failed(String),
}

pub struct GeneratedImage {
    pub prompt: String,
    pub tab: OracleBerryTab,
    pub provider: ImageProvider,
    pub size: ImageSize,
    pub created_at: Instant,
    pub status: GenerationStatus,
    /// Raw response bytes kept for "Save to Assets". Only set for
    /// image responses (PNG/JPEG); GLB outputs use `glb_path` on the
    /// status side instead.
    pub bytes: Option<Vec<u8>>,
}

/// Message types sent from background HTTP tasks back to the UI
/// thread. The UI thread is the only place that can build egui
/// textures (it owns the `Context`), so we send raw bytes / paths
/// and let the drain step convert them.
enum SaveSource {
    Bytes(Vec<u8>),
    CopyFrom(PathBuf),
}

pub enum OracleBerryMessage {
    ImageBytes {
        idx: usize,
        bytes: Vec<u8>,
    },
    GlbPath {
        idx: usize,
        path: PathBuf,
        size: u64,
    },
    Error {
        idx: usize,
        message: String,
    },
}

pub struct OracleBerryState {
    pub active_tab: OracleBerryTab,

    pub api_host: String,

    // Shared inputs
    pub prompt: String,
    pub negative_prompt: String,
    pub provider: ImageProvider,
    pub size: ImageSize,
    pub num_images: u8,
    pub steps: u32,

    // Image→Image
    pub i2i_source: Option<PathBuf>,
    pub i2i_strength: f32,

    // Image→Anime
    pub i2a_source: Option<PathBuf>,
    pub i2a_strength: f32,
    pub i2a_style: AnimeStyle,

    // 2D→3D
    pub three_d_source: Option<PathBuf>,
    pub three_d_output: MeshOutput,
    pub three_d_quality: f32,

    // Prompt augmentation
    pub auto_translate: bool,
    pub auto_isolate: bool,

    pub last_error: Option<String>,
    pub history: Vec<GeneratedImage>,
    pub selected: Option<usize>,

    pub response_tx: mpsc::UnboundedSender<OracleBerryMessage>,
    pub response_rx: Option<mpsc::UnboundedReceiver<OracleBerryMessage>>,
}

impl Default for OracleBerryState {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            active_tab: OracleBerryTab::Text2Image,
            api_host: DEFAULT_API_HOST.to_string(),
            prompt: String::new(),
            negative_prompt: String::new(),
            provider: ImageProvider::ComfyUI,
            size: ImageSize::Square1024,
            num_images: 1,
            steps: 20,
            i2i_source: None,
            i2i_strength: 0.6,
            i2a_source: None,
            i2a_strength: 0.7,
            i2a_style: AnimeStyle::Default,
            three_d_source: None,
            three_d_output: MeshOutput::Mesh,
            three_d_quality: 0.7,
            auto_translate: false,
            auto_isolate: false,
            last_error: None,
            history: Vec::new(),
            selected: None,
            response_tx: tx,
            response_rx: Some(rx),
        }
    }
}

/// Validate inputs and reserve a history slot for the new generation.
/// Returns the history index to fire the request against, or None if
/// validation failed (in which case `last_error` is populated).
fn validate_and_reserve(state: &mut OracleBerryState) -> Option<usize> {
    let needs_prompt = matches!(
        state.active_tab,
        OracleBerryTab::Text2Image | OracleBerryTab::Image2Image
    );
    if needs_prompt && state.prompt.trim().is_empty() {
        state.last_error = Some("Prompt is empty.".into());
        return None;
    }
    let source_required = match state.active_tab {
        OracleBerryTab::Image2Image => state.i2i_source.is_none(),
        OracleBerryTab::Image2Anime => state.i2a_source.is_none(),
        OracleBerryTab::Two2Three => state.three_d_source.is_none(),
        OracleBerryTab::Text2Image => false,
    };
    if source_required {
        state.last_error = Some("Pick a source image.".into());
        return None;
    }
    state.last_error = None;
    let provider = match state.active_tab {
        OracleBerryTab::Two2Three => ImageProvider::Trellis,
        _ => ImageProvider::ComfyUI,
    };
    let prompt_for_history = if state.prompt.trim().is_empty() {
        match state.active_tab {
            OracleBerryTab::Image2Anime => state.i2a_style.prompt_suffix().to_string(),
            OracleBerryTab::Two2Three => state
                .three_d_source
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "(3D from image)".into()),
            _ => "(no prompt)".into(),
        }
    } else {
        state.prompt.clone()
    };
    state.history.insert(
        0,
        GeneratedImage {
            prompt: prompt_for_history,
            tab: state.active_tab,
            provider,
            size: state.size,
            created_at: Instant::now(),
            status: GenerationStatus::Pending,
            bytes: None,
        },
    );
    state.selected = Some(0);
    Some(0)
}

// ────────────────────────────── Wire types ──────────────────────────────

#[derive(Serialize)]
struct ComfyTxt2ImgRequest {
    prompt: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    negative_prompt: String,
    width: u32,
    height: u32,
    steps: u32,
    cfg: f32,
    seed: i64,
}

#[derive(Serialize)]
struct ComfyImg2ImgRequest {
    prompt: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    negative_prompt: String,
    image_base64: String,
    width: u32,
    height: u32,
    steps: u32,
    cfg: f32,
    denoise: f32,
    seed: i64,
}

#[derive(Deserialize)]
struct ComfyGenerateResponse {
    images: Vec<String>,
}

#[derive(Serialize)]
struct TrellisGenerateRequest {
    image_base64: String,
    filename: String,
    ss_guidance_scale: f32,
    ss_steps: u32,
    slat_guidance_scale: f32,
    slat_steps: u32,
    texture_size: u32,
}

#[derive(Deserialize)]
struct TrellisGenerateResponse {
    glb_base64: String,
    glb_size: u64,
}

// ────────────────────────────── Async helpers ──────────────────────────────

async fn read_and_b64(path: &PathBuf) -> Result<String, String> {
    let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Read an image, downscale if either dimension exceeds 1536 px, and
/// re-encode as JPEG (quality 85). Avoids `HTTP 413 Payload Too Large`
/// when the user picks a Retina screenshot or large camera image —
/// the server's default body buffer is well under what raw 4K PNGs
/// produce after base64 expansion.
async fn read_resize_b64(path: &PathBuf) -> Result<(String, u32, u32), String> {
    let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
    let img = image::load_from_memory(&bytes).map_err(|e| format!("Decode source: {e}"))?;
    const MAX_DIM: u32 = 1536;
    let (w, h) = (img.width(), img.height());
    let img = if w > MAX_DIM || h > MAX_DIM {
        let scale = MAX_DIM as f32 / w.max(h) as f32;
        let new_w = ((w as f32 * scale).round() as u32).max(1);
        let new_h = ((h as f32 * scale).round() as u32).max(1);
        img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let (sw, sh) = (img.width(), img.height());
    let rgb = img.to_rgb8();
    let mut buf = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
    encoder
        .encode(&rgb, sw, sh, image::ExtendedColorType::Rgb8)
        .map_err(|e| format!("Encode JPEG: {e}"))?;
    Ok((
        base64::engine::general_purpose::STANDARD.encode(&buf),
        sw,
        sh,
    ))
}

async fn do_txt2img(
    host: &str,
    prompt: String,
    negative: String,
    w: u32,
    h: u32,
    steps: u32,
) -> Result<Vec<u8>, String> {
    let req = ComfyTxt2ImgRequest {
        prompt,
        negative_prompt: negative,
        width: w,
        height: h,
        steps,
        cfg: 7.0,
        seed: -1,
    };
    let url = format!("http://{host}/api/comfyui/txt2img");
    post_comfy(&url, &req).await
}

async fn do_img2img(
    host: &str,
    prompt: String,
    negative: String,
    _w: u32, // ignored — width/height tracked from the source image after resize
    _h: u32,
    steps: u32,
    source: PathBuf,
    strength: f32,
) -> Result<Vec<u8>, String> {
    let (init_b64, sw, sh) = read_resize_b64(&source).await?;
    let req = ComfyImg2ImgRequest {
        prompt,
        negative_prompt: negative,
        image_base64: init_b64,
        width: sw,
        height: sh,
        steps,
        cfg: 7.0,
        denoise: strength,
        seed: -1,
    };
    let url = format!("http://{host}/api/comfyui/img2img");
    post_comfy(&url, &req).await
}

/// Walk reqwest's error source chain so the UI shows the real cause
/// (timeout, connection refused, etc.) instead of the bland top-level
/// "error sending request for url (...)" wrapper.
fn full_error_chain(e: &(dyn std::error::Error + 'static)) -> String {
    let mut msg = e.to_string();
    let mut src = e.source();
    while let Some(s) = src {
        msg.push_str(" → ");
        msg.push_str(&s.to_string());
        src = s.source();
    }
    msg
}

async fn post_comfy<T: Serialize>(url: &str, body: &T) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| full_error_chain(&e))?;
    tracing::info!("OracleBerry POST → {}", url);
    let resp = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| full_error_chain(&e))?;
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let parsed: ComfyGenerateResponse = resp.json().await.map_err(|e| full_error_chain(&e))?;
    let first = parsed
        .images
        .into_iter()
        .next()
        .ok_or_else(|| "No image in response".to_string())?;
    base64::engine::general_purpose::STANDARD
        .decode(first.as_bytes())
        .map_err(|e| format!("Base64 decode: {e}"))
}

async fn do_trellis(host: &str, source: PathBuf, quality: f32) -> Result<(Vec<u8>, u64), String> {
    let (image_b64, _sw, _sh) = read_resize_b64(&source).await?;
    let filename = source
        .file_stem()
        .map(|s| format!("{}.jpg", s.to_string_lossy()))
        .unwrap_or_else(|| "input.jpg".to_string());
    // Map quality slider 0.1..1.0 → step counts.
    let steps = (20.0 + quality * 60.0).round() as u32;
    let req = TrellisGenerateRequest {
        image_base64: image_b64,
        filename,
        ss_guidance_scale: 7.5,
        ss_steps: steps,
        slat_guidance_scale: 3.0,
        slat_steps: steps,
        texture_size: (1024.0 + quality * 3072.0).round() as u32,
    };
    let url = format!("http://{host}/api/trellis/generate");
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let parsed: TrellisGenerateResponse = resp.json().await.map_err(|e| e.to_string())?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(parsed.glb_base64.as_bytes())
        .map_err(|e| format!("Base64 decode: {e}"))?;
    Ok((bytes, parsed.glb_size))
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    message: String,
    system: &'a str,
    model: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    reply: String,
}

const TRANSLATE_SYSTEM_PROMPT: &str = "You translate user input into a concise English prompt for a Stable Diffusion / FLUX text-to-image model. Output ONLY the translated English text — no quotes, no preamble, no explanation. Keep it short and visual.";

async fn do_translate(host: &str, jp_prompt: String) -> Result<String, String> {
    let url = format!("http://{host}/chat");
    let body = ChatRequest {
        message: jp_prompt,
        system: TRANSLATE_SYSTEM_PROMPT,
        model: "qwen3.6:35b-a3b-q8_0",
    };
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let parsed: ChatResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.reply.trim().trim_matches('"').to_string())
}

fn needs_translation(s: &str) -> bool {
    s.chars().any(|c| {
        let cp = c as u32;
        (0x3040..=0x309F).contains(&cp) // Hiragana
            || (0x30A0..=0x30FF).contains(&cp) // Katakana
            || (0x4E00..=0x9FFF).contains(&cp) // CJK Unified Ideographs
            || (0x3400..=0x4DBF).contains(&cp) // CJK Extension A
    })
}

const ISOLATE_SUFFIX: &str =
    ", isolated subject on plain white background, product photography, studio lighting, no scenery, no people, no body parts";

const ISOLATE_NEGATIVE: &str =
    "background, scenery, person, body parts, hands, feet, room, floor, environment, additional objects";

fn apply_isolate(mut prompt: String, mut negative: String) -> (String, String) {
    if !prompt.is_empty() {
        prompt.push_str(ISOLATE_SUFFIX);
    } else {
        prompt = ISOLATE_SUFFIX.trim_start_matches(", ").to_string();
    }
    if negative.is_empty() {
        negative = ISOLATE_NEGATIVE.to_string();
    } else {
        negative.push_str(", ");
        negative.push_str(ISOLATE_NEGATIVE);
    }
    (prompt, negative)
}

async fn prepare_prompt(
    host: &str,
    prompt: String,
    negative: String,
    auto_translate: bool,
    auto_isolate: bool,
) -> (String, String) {
    let translated = if auto_translate && needs_translation(&prompt) {
        match do_translate(host, prompt.clone()).await {
            Ok(t) => t,
            Err(_) => prompt,
        }
    } else {
        prompt
    };
    if auto_isolate {
        apply_isolate(translated, negative)
    } else {
        (translated, negative)
    }
}

impl BerryCodeApp {
    /// Drain pending API responses and update history entries. Called
    /// at the top of `render_oracleberry_central` so textures and
    /// errors land in time for the same frame.
    fn drain_oracleberry_responses(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.oracleberry.response_rx.as_mut() else {
            return;
        };
        loop {
            let msg = match rx.try_recv() {
                Ok(m) => m,
                Err(_) => break,
            };
            match msg {
                OracleBerryMessage::ImageBytes { idx, bytes } => {
                    let Some(item) = self.oracleberry.history.get_mut(idx) else {
                        continue;
                    };
                    match image::load_from_memory(&bytes) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            let (w, h) = (rgba.width(), rgba.height());
                            let color = egui::ColorImage::from_rgba_unmultiplied(
                                [w as usize, h as usize],
                                &rgba,
                            );
                            let tex = ctx.load_texture(
                                format!("oracleberry-{idx}"),
                                color,
                                egui::TextureOptions::LINEAR,
                            );
                            item.status = GenerationStatus::Done {
                                texture: Some(tex),
                                glb_path: None,
                            };
                            item.bytes = Some(bytes);
                        }
                        Err(e) => {
                            item.status = GenerationStatus::Failed(format!("decode: {e}"));
                        }
                    }
                }
                OracleBerryMessage::GlbPath { idx, path, size } => {
                    if let Some(item) = self.oracleberry.history.get_mut(idx) {
                        item.status = GenerationStatus::Done {
                            texture: None,
                            glb_path: Some(path.clone()),
                        };
                        tracing::info!("Trellis GLB written: {} ({} bytes)", path.display(), size);
                    }
                }
                OracleBerryMessage::Error { idx, message } => {
                    if let Some(item) = self.oracleberry.history.get_mut(idx) {
                        item.status = GenerationStatus::Failed(message);
                    }
                }
            }
        }
    }

    /// Save the generated artifact (image bytes or GLB file) to the
    /// project's assets directory. Returns Ok(saved_path) or
    /// Err(message); both flow into `status_message` for the status
    /// bar.
    fn save_oracleberry_to_assets(&mut self, idx: usize) -> Result<PathBuf, String> {
        let Some(item) = self.oracleberry.history.get(idx) else {
            return Err("History entry missing".into());
        };
        if self.root_path.is_empty() {
            return Err("No project open".into());
        }
        let assets_dir = std::path::Path::new(&self.root_path).join("assets/oracleberry");
        std::fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let safe_prompt: String = item
            .prompt
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
            .take(40)
            .collect::<String>()
            .trim()
            .replace(' ', "-");

        let (ext, source): (&str, SaveSource) = match (&item.status, &item.bytes) {
            (
                GenerationStatus::Done {
                    glb_path: Some(p), ..
                },
                _,
            ) => ("glb", SaveSource::CopyFrom(p.clone())),
            (_, Some(bytes)) => ("png", SaveSource::Bytes(bytes.clone())),
            _ => return Err("Nothing to save yet".into()),
        };

        let stem = if safe_prompt.is_empty() {
            format!("oracleberry-{stamp}")
        } else {
            format!("{safe_prompt}-{stamp}")
        };
        let dest = assets_dir.join(format!("{stem}.{ext}"));
        match source {
            SaveSource::Bytes(bytes) => {
                std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
            }
            SaveSource::CopyFrom(src) => {
                std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
            }
        }
        Ok(dest)
    }

    fn kick_oracleberry_request(&mut self, idx: usize) {
        tracing::info!(
            "🎨 OracleBerry kick: tab={:?} host={}",
            self.oracleberry.active_tab,
            self.oracleberry.api_host
        );
        let st = &self.oracleberry;
        let host = st.api_host.clone();
        let tab = st.active_tab;
        let prompt = st.prompt.clone();
        let negative = st.negative_prompt.clone();
        let (w, h) = st.size.dims();
        let steps = st.steps;
        let i2i_source = st.i2i_source.clone();
        let i2i_strength = st.i2i_strength;
        let i2a_source = st.i2a_source.clone();
        let i2a_strength = st.i2a_strength;
        let i2a_style = st.i2a_style;
        let three_d_source = st.three_d_source.clone();
        let three_d_quality = st.three_d_quality;
        let auto_translate = st.auto_translate;
        let auto_isolate = st.auto_isolate;
        let tx = st.response_tx.clone();

        self.lsp_runtime.spawn(async move {
            match tab {
                OracleBerryTab::Text2Image => {
                    let (p, n) =
                        prepare_prompt(&host, prompt, negative, auto_translate, auto_isolate).await;
                    let result = do_txt2img(&host, p, n, w, h, steps).await;
                    send_image_result(&tx, idx, result);
                }
                OracleBerryTab::Image2Image => {
                    let Some(source) = i2i_source else {
                        let _ = tx.send(OracleBerryMessage::Error {
                            idx,
                            message: "Source image missing".into(),
                        });
                        return;
                    };
                    let (p, n) =
                        prepare_prompt(&host, prompt, negative, auto_translate, auto_isolate).await;
                    let result = do_img2img(&host, p, n, w, h, steps, source, i2i_strength).await;
                    send_image_result(&tx, idx, result);
                }
                OracleBerryTab::Image2Anime => {
                    let Some(source) = i2a_source else {
                        let _ = tx.send(OracleBerryMessage::Error {
                            idx,
                            message: "Source image missing".into(),
                        });
                        return;
                    };
                    let translated = if auto_translate && needs_translation(&prompt) {
                        do_translate(&host, prompt.clone()).await.unwrap_or(prompt)
                    } else {
                        prompt
                    };
                    let style_prompt = if translated.trim().is_empty() {
                        i2a_style.prompt_suffix().to_string()
                    } else {
                        format!("{}, {}", translated, i2a_style.prompt_suffix())
                    };
                    let (final_p, final_n) = if auto_isolate {
                        apply_isolate(style_prompt, negative)
                    } else {
                        (style_prompt, negative)
                    };
                    let result =
                        do_img2img(&host, final_p, final_n, w, h, steps, source, i2a_strength)
                            .await;
                    send_image_result(&tx, idx, result);
                }
                OracleBerryTab::Two2Three => {
                    let Some(source) = three_d_source else {
                        let _ = tx.send(OracleBerryMessage::Error {
                            idx,
                            message: "Source image missing".into(),
                        });
                        return;
                    };
                    match do_trellis(&host, source, three_d_quality).await {
                        Ok((bytes, size)) => {
                            let path = std::env::temp_dir().join(format!("oracleberry-{idx}.glb"));
                            if let Err(e) = std::fs::write(&path, &bytes) {
                                let _ = tx.send(OracleBerryMessage::Error {
                                    idx,
                                    message: format!("Write GLB: {e}"),
                                });
                                return;
                            }
                            let _ = tx.send(OracleBerryMessage::GlbPath { idx, path, size });
                        }
                        Err(e) => {
                            let _ = tx.send(OracleBerryMessage::Error { idx, message: e });
                        }
                    }
                }
            }
        });
    }

    pub(crate) fn render_oracleberry_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("OracleBerry");
        ui.label(
            egui::RichText::new("AI Generator")
                .small()
                .color(egui::Color32::from_gray(150)),
        );
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Host").small());
            ui.add(
                egui::TextEdit::singleline(&mut self.oracleberry.api_host)
                    .desired_width(f32::INFINITY),
            );
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        ui.label(egui::RichText::new("Recent").small());
        ui.add_space(2.0);

        if self.oracleberry.history.is_empty() {
            ui.label(
                egui::RichText::new("(no generations yet)")
                    .small()
                    .color(egui::Color32::from_gray(150)),
            );
            return;
        }

        let entries: Vec<(usize, String, OracleBerryTab, bool)> = self
            .oracleberry
            .history
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let pending = matches!(g.status, GenerationStatus::Pending);
                (i, truncate(&g.prompt, 26), g.tab, pending)
            })
            .collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, label, tab, pending) in entries {
                let selected = self.oracleberry.selected == Some(i);
                let prefix = match tab {
                    OracleBerryTab::Text2Image => "T2I",
                    OracleBerryTab::Image2Image => "I2I",
                    OracleBerryTab::Image2Anime => "I2A",
                    OracleBerryTab::Two2Three => "2D23D",
                };
                let display = if pending {
                    format!("[{prefix}] ⏳ {label}")
                } else {
                    format!("[{prefix}] {label}")
                };
                if ui.selectable_label(selected, display).clicked() {
                    self.oracleberry.selected = Some(i);
                }
            }
        });
    }

    pub(crate) fn render_oracleberry_central(&mut self, ui: &mut egui::Ui) {
        self.drain_oracleberry_responses(ui.ctx());

        ui.horizontal(|ui| {
            ui.heading("AI Generator");
            ui.label(
                egui::RichText::new(format!("via {}", self.oracleberry.api_host))
                    .small()
                    .color(egui::Color32::from_gray(150)),
            );
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            for tab in [
                OracleBerryTab::Text2Image,
                OracleBerryTab::Image2Image,
                OracleBerryTab::Image2Anime,
                OracleBerryTab::Two2Three,
            ] {
                ui.selectable_value(&mut self.oracleberry.active_tab, tab, tab.label());
            }
        });
        ui.separator();
        ui.add_space(8.0);

        match self.oracleberry.active_tab {
            OracleBerryTab::Text2Image => self.render_text2image(ui),
            OracleBerryTab::Image2Image => self.render_image2image(ui),
            OracleBerryTab::Image2Anime => self.render_image2anime(ui),
            OracleBerryTab::Two2Three => self.render_2d3d(ui),
        }
    }

    fn render_text2image(&mut self, ui: &mut egui::Ui) {
        ui.label("Prompt");
        ui.add(
            egui::TextEdit::multiline(&mut self.oracleberry.prompt)
                .hint_text("Describe the image you want to generate…")
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("Negative prompt (optional)")
                .small()
                .color(egui::Color32::from_gray(150)),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.oracleberry.negative_prompt)
                .hint_text("things to avoid…")
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);
        self.render_translate_isolate_row(ui);

        ui.add_space(8.0);
        self.render_size_steps(ui);
        ui.add_space(10.0);
        self.render_generate_row(ui);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        self.render_result_canvas(ui);
    }

    fn render_image2image(&mut self, ui: &mut egui::Ui) {
        source_picker(ui, "Source image:", &mut self.oracleberry.i2i_source);

        ui.add_space(8.0);
        ui.label("Prompt");
        ui.add(
            egui::TextEdit::multiline(&mut self.oracleberry.prompt)
                .hint_text("How should the source image change?")
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);
        self.render_translate_isolate_row(ui);

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Strength");
            ui.add(
                egui::Slider::new(&mut self.oracleberry.i2i_strength, 0.0..=1.0)
                    .text("(higher = more changes)"),
            );
        });

        ui.add_space(8.0);
        self.render_size_steps(ui);
        ui.add_space(10.0);
        self.render_generate_row(ui);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        self.render_pair_canvas(ui, "Source", "Result");
    }

    fn render_image2anime(&mut self, ui: &mut egui::Ui) {
        source_picker(ui, "Source image:", &mut self.oracleberry.i2a_source);

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Style");
            egui::ComboBox::from_id_salt("oracleberry_anime_style")
                .selected_text(self.oracleberry.i2a_style.label())
                .show_ui(ui, |ui| {
                    for s in [
                        AnimeStyle::Default,
                        AnimeStyle::CelShaded,
                        AnimeStyle::SoftPastel,
                        AnimeStyle::LineArt,
                        AnimeStyle::Painterly,
                    ] {
                        ui.selectable_value(&mut self.oracleberry.i2a_style, s, s.label());
                    }
                });
            ui.add_space(12.0);
            ui.label("Strength");
            ui.add(egui::Slider::new(
                &mut self.oracleberry.i2a_strength,
                0.0..=1.0,
            ));
        });

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Prompt (optional)")
                .small()
                .color(egui::Color32::from_gray(150)),
        );
        ui.add(
            egui::TextEdit::multiline(&mut self.oracleberry.prompt)
                .hint_text("Optional guidance, e.g. 'soft lighting, gentle color palette…'")
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(6.0);
        self.render_translate_isolate_row(ui);

        ui.add_space(8.0);
        self.render_size_steps(ui);
        ui.add_space(10.0);
        self.render_generate_row(ui);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        self.render_pair_canvas(ui, "Source", "Stylised");
    }

    fn render_2d3d(&mut self, ui: &mut egui::Ui) {
        source_picker(ui, "Source image:", &mut self.oracleberry.three_d_source);

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Output");
            egui::ComboBox::from_id_salt("oracleberry_3d_output")
                .selected_text(self.oracleberry.three_d_output.label())
                .show_ui(ui, |ui| {
                    for f in [MeshOutput::Mesh, MeshOutput::PointCloud] {
                        ui.selectable_value(&mut self.oracleberry.three_d_output, f, f.label());
                    }
                });
            ui.add_space(12.0);
            ui.label("Quality");
            ui.add(egui::Slider::new(
                &mut self.oracleberry.three_d_quality,
                0.1..=1.0,
            ));
        });

        ui.add_space(10.0);
        self.render_generate_row(ui);
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Source")
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );
                let size = egui::vec2(180.0, 180.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                draw_canvas_placeholder(
                    ui,
                    rect,
                    if self.oracleberry.three_d_source.is_some() {
                        "(thumbnail)"
                    } else {
                        "(no input)"
                    },
                );
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("3D output")
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );
                let avail = ui.available_size_before_wrap();
                let size = egui::vec2(avail.x.min(440.0), 320.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let hint = match self.oracleberry.history.first() {
                    Some(item) if matches!(item.tab, OracleBerryTab::Two2Three) => {
                        match &item.status {
                            GenerationStatus::Pending => "(generating GLB…)".to_string(),
                            GenerationStatus::Done {
                                glb_path: Some(p), ..
                            } => format!("GLB written: {}", p.display()),
                            GenerationStatus::Failed(e) => format!("Error: {e}"),
                            _ => "(orbit-cam mesh viewer — coming soon)".to_string(),
                        }
                    }
                    _ => "(no GLB yet)".to_string(),
                };
                draw_canvas_placeholder(ui, rect, &hint);
            });
        });

        let idx = self.oracleberry.selected.unwrap_or(0);
        let can_save = matches!(
            self.oracleberry.history.get(idx).map(|i| &i.status),
            Some(GenerationStatus::Done {
                glb_path: Some(_),
                ..
            })
        );
        ui.add_space(8.0);
        self.render_save_to_assets_row(ui, idx, can_save);
    }

    fn render_translate_isolate_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.oracleberry.auto_translate, "Translate JP→EN");
            ui.add_space(12.0);
            ui.checkbox(&mut self.oracleberry.auto_isolate, "Subject only (isolate)");
        });
    }

    fn render_size_steps(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Size");
            egui::ComboBox::from_id_salt("oracleberry_size")
                .selected_text(self.oracleberry.size.label())
                .show_ui(ui, |ui| {
                    for s in [
                        ImageSize::Square512,
                        ImageSize::Square1024,
                        ImageSize::Wide1536,
                        ImageSize::Tall1536,
                    ] {
                        ui.selectable_value(&mut self.oracleberry.size, s, s.label());
                    }
                });
            ui.add_space(12.0);

            ui.label("Steps");
            ui.add(egui::DragValue::new(&mut self.oracleberry.steps).range(5..=100));
        });
    }

    fn render_generate_row(&mut self, ui: &mut egui::Ui) {
        let pending = matches!(
            self.oracleberry.history.first(),
            Some(GeneratedImage {
                status: GenerationStatus::Pending,
                ..
            })
        );
        ui.horizontal(|ui| {
            let label = if pending { "Generating…" } else { "Generate" };
            let resp = primary_button(ui, label);
            if resp.clicked() && !pending {
                if let Some(idx) = validate_and_reserve(&mut self.oracleberry) {
                    self.kick_oracleberry_request(idx);
                }
            }
            if ui.button("Clear").clicked() {
                self.clear_oracleberry_inputs();
            }
            if let Some(err) = &self.oracleberry.last_error {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 90), err);
            }
        });
    }

    fn clear_oracleberry_inputs(&mut self) {
        let st = &mut self.oracleberry;
        st.prompt.clear();
        st.negative_prompt.clear();
        match st.active_tab {
            OracleBerryTab::Image2Image => st.i2i_source = None,
            OracleBerryTab::Image2Anime => st.i2a_source = None,
            OracleBerryTab::Two2Three => st.three_d_source = None,
            OracleBerryTab::Text2Image => {}
        }
        st.last_error = None;
    }

    fn render_result_canvas(&mut self, ui: &mut egui::Ui) {
        let active_tab = self.oracleberry.active_tab;
        let Some((idx, item)) = self
            .oracleberry
            .history
            .iter()
            .enumerate()
            .find(|(_, g)| g.tab == active_tab)
        else {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(
                    egui::RichText::new("Your generated images will appear here.")
                        .color(egui::Color32::from_gray(140)),
                );
                ui.add_space(40.0);
            });
            return;
        };

        let prompt = item.prompt.clone();
        let provider_label = item.provider.label();
        let size_label = item.size.label();
        let status_clone = item.status.clone();
        let can_save = matches!(item.status, GenerationStatus::Done { .. }) && item.bytes.is_some();

        ui.label(
            egui::RichText::new(prompt)
                .strong()
                .color(ui_colors::TEXT_DEFAULT),
        );
        ui.label(
            egui::RichText::new(format!("{provider_label}  •  {size_label}"))
                .small()
                .color(egui::Color32::from_gray(150)),
        );
        ui.add_space(6.0);
        self.render_save_to_assets_row(ui, idx, can_save);
        ui.add_space(8.0);

        let avail = ui.available_size_before_wrap();
        let canvas_size = egui::vec2(avail.x.min(640.0), 360.0);
        let (rect, _) = ui.allocate_exact_size(canvas_size, egui::Sense::hover());
        draw_status_canvas(ui, rect, &status_clone, "(image preview)");
    }

    fn render_save_to_assets_row(&mut self, ui: &mut egui::Ui, idx: usize, can_save: bool) {
        ui.horizontal(|ui| {
            ui.add_enabled_ui(can_save, |ui| {
                if ui.button("\u{eb4b}  Save to Assets").clicked() {
                    match self.save_oracleberry_to_assets(idx) {
                        Ok(p) => {
                            self.status_message = format!("Saved: {}", p.display());
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                        Err(e) => {
                            self.status_message = format!("Save failed: {e}");
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                }
            });
            if !can_save {
                ui.label(
                    egui::RichText::new("(generate first)")
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );
            }
        });
    }

    fn render_pair_canvas(&mut self, ui: &mut egui::Ui, src_label: &str, dst_label: &str) {
        let active_tab = self.oracleberry.active_tab;
        let item_data = self
            .oracleberry
            .history
            .iter()
            .enumerate()
            .find(|(_, g)| g.tab == active_tab)
            .map(|(i, g)| (i, g.status.clone(), g.bytes.is_some()));
        let idx = item_data.as_ref().map(|(i, _, _)| *i).unwrap_or(0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(src_label)
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );
                let avail = ui.available_size_before_wrap();
                let size = egui::vec2((avail.x - 12.0).min(360.0), 280.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let has_source = match self.oracleberry.active_tab {
                    OracleBerryTab::Image2Image => self.oracleberry.i2i_source.is_some(),
                    OracleBerryTab::Image2Anime => self.oracleberry.i2a_source.is_some(),
                    _ => false,
                };
                draw_canvas_placeholder(
                    ui,
                    rect,
                    if has_source {
                        "(source thumbnail — preview pending)"
                    } else {
                        "(no source picked)"
                    },
                );
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(dst_label)
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );
                let avail = ui.available_size_before_wrap();
                let size = egui::vec2(avail.x.min(360.0), 280.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                if let Some((_, status, _)) = &item_data {
                    draw_status_canvas(ui, rect, status, "(preview)");
                } else {
                    draw_canvas_placeholder(ui, rect, "(no result yet)");
                }
            });
        });

        if let Some((_, status, has_bytes)) = item_data {
            let can_save = matches!(status, GenerationStatus::Done { .. }) && has_bytes;
            ui.add_space(8.0);
            self.render_save_to_assets_row(ui, idx, can_save);
        }
    }
}

fn send_image_result(
    tx: &mpsc::UnboundedSender<OracleBerryMessage>,
    idx: usize,
    result: Result<Vec<u8>, String>,
) {
    let _ = match result {
        Ok(bytes) => {
            tracing::info!(
                "✅ OracleBerry response: {} bytes (idx={})",
                bytes.len(),
                idx
            );
            tx.send(OracleBerryMessage::ImageBytes { idx, bytes })
        }
        Err(message) => {
            tracing::error!("❌ OracleBerry error: {} (idx={})", message, idx);
            tx.send(OracleBerryMessage::Error { idx, message })
        }
    };
}

fn source_picker(ui: &mut egui::Ui, label: &str, slot: &mut Option<PathBuf>) {
    ui.horizontal(|ui| {
        ui.label(label);
        if ui.button("Pick…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp"])
                .pick_file()
            {
                *slot = Some(path);
            }
        }
        if let Some(path) = slot.as_ref() {
            ui.label(
                egui::RichText::new(
                    path.file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string()),
                )
                .color(ui_colors::TEXT_DEFAULT),
            );
            if ui.small_button("Clear").clicked() {
                *slot = None;
            }
        } else {
            ui.label(
                egui::RichText::new("(no file selected)")
                    .small()
                    .color(egui::Color32::from_gray(150)),
            );
        }
    });
}

fn draw_canvas_placeholder(ui: &mut egui::Ui, rect: egui::Rect, label: &str) {
    ui.painter()
        .rect_filled(rect, 6.0, egui::Color32::from_gray(28));
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        egui::Color32::from_gray(120),
    );
}

fn draw_status_canvas(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    status: &GenerationStatus,
    pending_hint: &str,
) {
    match status {
        GenerationStatus::Pending => draw_canvas_placeholder(ui, rect, "Generating…"),
        GenerationStatus::Failed(e) => {
            ui.painter()
                .rect_filled(rect, 6.0, egui::Color32::from_rgb(45, 25, 25));
            ui.painter().rect_stroke(
                rect,
                6.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 50, 50)),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Error: {e}"),
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(220, 120, 120),
            );
        }
        GenerationStatus::Done {
            texture: Some(tex), ..
        } => {
            // Draw the texture letterboxed inside `rect`, preserving aspect.
            let tex_size = tex.size_vec2();
            let scale = (rect.width() / tex_size.x).min(rect.height() / tex_size.y);
            let visual = tex_size * scale;
            let inner = egui::Rect::from_center_size(rect.center(), visual);
            ui.painter()
                .rect_filled(rect, 6.0, egui::Color32::from_gray(20));
            egui::Image::new(tex).paint_at(ui, inner);
        }
        GenerationStatus::Done { .. } => draw_canvas_placeholder(ui, rect, pending_hint),
    }
}

fn truncate(s: &str, max: usize) -> String {
    let trimmed = s.trim();
    if trimmed.chars().count() <= max {
        trimmed.to_string()
    } else {
        let mut out: String = trimmed.chars().take(max).collect();
        out.push('…');
        out
    }
}
