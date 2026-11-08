use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use gpui::RenderImage;
use image::GenericImageView;

use crate::model::HttpAuth;
use crate::net::NetHandle;

static DISK_DIR: OnceLock<PathBuf> = OnceLock::new();
const DISK_LIMIT: usize = 4000;

type TrackArtLookup = Box<dyn Fn(&str) -> Option<PathBuf> + Send + Sync>;
type DownloadedLookup = Box<dyn Fn(&str) -> bool + Send + Sync>;
static TRACK_ART: OnceLock<(DownloadedLookup, TrackArtLookup, TrackArtLookup)> = OnceLock::new();

pub fn set_track_art_lookup(
    downloaded: impl Fn(&str) -> bool + Send + Sync + 'static,
    cached: impl Fn(&str) -> Option<PathBuf> + Send + Sync + 'static,
    extract: impl Fn(&str) -> Option<PathBuf> + Send + Sync + 'static,
) {
    let _ = TRACK_ART.set((Box::new(downloaded), Box::new(cached), Box::new(extract)));
}

pub fn is_downloaded(video_id: &str) -> bool {
    !video_id.is_empty() && TRACK_ART.get().is_some_and(|(downloaded, _, _)| downloaded(video_id))
}

fn cached_track_art(video_id: &str) -> Option<PathBuf> {
    if video_id.is_empty() || !is_downloaded(video_id) {
        return None;
    }
    TRACK_ART.get().and_then(|(_, cached, _)| cached(video_id))
}

pub async fn track_art(net: &NetHandle, video_id: &str) -> Option<PathBuf> {
    if !is_downloaded(video_id) {
        return None;
    }
    if let Some(path) = cached_track_art(video_id) {
        return Some(path);
    }
    let id = video_id.to_owned();
    net.spawn(async move { tokio::task::spawn_blocking(move || TRACK_ART.get().and_then(|(_, _, extract)| extract(&id))).await.ok().flatten() }).await.ok().flatten()
}

pub async fn track_cover_address(net: &NetHandle, video_id: &str, url: &str) -> String {
    match track_art(net, video_id).await {
        Some(path) => path.to_string_lossy().into_owned(),
        None => url.to_owned(),
    }
}

pub fn init_disk_cache(cache_dir: &Path) {
    let dir = cache_dir.join("covers");
    if DISK_DIR.set(dir.clone()).is_err() {
        return;
    }
    std::thread::spawn(move || {
        let _ = std::fs::create_dir_all(&dir);
        let Ok(read) = std::fs::read_dir(&dir) else { return };
        let mut files: Vec<(std::time::SystemTime, PathBuf)> = read.filter_map(|e| e.ok()).filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
        if files.len() > DISK_LIMIT {
            files.sort();
            for (_, path) in &files[..files.len() - DISK_LIMIT] {
                let _ = std::fs::remove_file(path);
            }
        }
    });
}

fn disk_path(url: &str, target: Option<u32>) -> Option<PathBuf> {
    let dir = DISK_DIR.get()?;
    Some(dir.join(format!("{}-{}", address_hash(url), target.unwrap_or(0))))
}

fn disk_path_any_size(url: &str, skip: Option<u32>) -> Option<PathBuf> {
    let dir = DISK_DIR.get()?;
    let prefix = format!("{}-", address_hash(url));
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let size: u32 = name.strip_prefix(&prefix)?.parse().ok()?;
            (Some(size) != skip).then(|| (if size == 0 { u32::MAX } else { size }, e.path()))
        })
        .max_by_key(|(size, _)| *size)
        .map(|(_, path)| path)
}

fn address_hash(url: &str) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(unsigned_address(url).as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn unsigned_address(url: &str) -> String {
    let Some((path, query)) = url.split_once('?') else { return url.to_owned() };
    let kept: Vec<&str> = query.split('&').filter(|param| !param.starts_with("sqp=") && !param.starts_with("rs=")).collect();
    if kept.is_empty() { path.to_owned() } else { format!("{path}?{}", kept.join("&")) }
}

async fn write_disk(path: &Path, bytes: &[u8]) {
    let tmp = path.with_extension("tmp");
    let write = async {
        tokio::fs::write(&tmp, bytes).await?;
        tokio::fs::rename(&tmp, path).await
    };
    if let Err(err) = write.await {
        tracing::debug!(%err, ?path, "cover not cached on disk");
    }
}

pub async fn fetch_cover_bytes(http: &reqwest::Client, auth: Option<&HttpAuth>, url: &str, target: Option<u32>) -> Option<Vec<u8>> {
    if let Some(path) = local_path(url) {
        return tokio::fs::read(path).await.ok().filter(|b| !b.is_empty());
    }
    for candidate in fallback_chain(url, target) {
        let mut request = http.get(&candidate);
        if let Some(auth) = auth.filter(|_| ["youtube.com", "ytimg.com", "googleusercontent.com", "ggpht.com"].iter().any(|d| candidate.contains(d))) {
            request = request.header("Cookie", &auth.cookie).header("User-Agent", &auth.user_agent);
        }
        match request.send().await.and_then(|r| r.error_for_status()) {
            Ok(response) => match response.bytes().await {
                Ok(bytes) if !bytes.is_empty() => return Some(bytes.to_vec()),
                _ => continue,
            },
            Err(_) => continue,
        }
    }
    let path = disk_path(url, target).filter(|p| p.is_file()).or_else(|| disk_path_any_size(url, None))?;
    tokio::fs::read(path).await.ok().filter(|b| !b.is_empty())
}

fn fallback_chain(url: &str, target: Option<u32>) -> Vec<String> {
    const QUALITIES: [&str; 5] = ["maxresdefault", "sddefault", "hqdefault", "mqdefault", "default"];
    let mut out = Vec::new();
    let mut push = |candidate: String| {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    };
    let upscaled = high_res_url(url, target);
    push(upscaled.clone());
    push(url.to_owned());
    for base in [upscaled, url.to_owned()] {
        if base.contains("i.ytimg.com") {
            if let Some(pos) = QUALITIES.iter().position(|q| base.contains(q)) {
                for lower in &QUALITIES[pos + 1..] {
                    push(base.replace(QUALITIES[pos], lower));
                }
            }
        }
    }
    out
}

fn local_path(url: &str) -> Option<PathBuf> {
    if url.starts_with("file://") {
        return glib::filename_from_uri(url).ok().map(|(path, _)| path);
    }
    Path::new(url).is_absolute().then(|| PathBuf::from(url))
}

pub fn high_res_url(url: &str, target_size: Option<u32>) -> String {
    use std::sync::LazyLock;

    static SIGNATURE_PARAMS: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"([?&])(sqp|rs)=[^&]*&?").expect("static regex"));
    static WIDTH_HEIGHT: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"([=-])w\d+-h\d+").expect("static regex"));
    static SQUARE_SIZE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"([=-])s\d+(-|$)").expect("static regex"));

    if url.is_empty() {
        return String::new();
    }
    let clean = if url.contains("vi_locker") || url.contains("/pl_c/") {
        url.to_owned()
    } else {
        SIGNATURE_PARAMS.replace_all(url, "$1").replace("?&", "?").trim_end_matches(['?', '&']).to_owned()
    };
    if clean.contains("i.ytimg.com") {
        for quality in ["maxresdefault", "sddefault", "hqdefault", "mqdefault", "default"] {
            if clean.contains(quality) {
                return clean.replace(quality, "maxresdefault");
            }
        }
        return clean;
    }
    let dim = target_size.map(|s| s * 2).unwrap_or(544);
    if clean.contains("googleusercontent.com") || clean.contains("ggpht.com") {
        if WIDTH_HEIGHT.is_match(&clean) {
            return WIDTH_HEIGHT.replace_all(&clean, format!("${{1}}w{dim}-h{dim}")).into_owned();
        }
        return SQUARE_SIZE.replace_all(&clean, format!("${{1}}s{dim}$2")).into_owned();
    }
    clean
}

pub fn decode_cover(bytes: &[u8], target: Option<u32>) -> Option<std::sync::Arc<RenderImage>> {
    let decoded = image::load_from_memory(bytes).ok()?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let side = width.min(height);
    let mut rgba = decoded.crop_imm((width - side) / 2, (height - side) / 2, side, side).to_rgba8();
    if let Some(target) = target {
        let wanted = target.saturating_mul(2).max(1);
        if side > wanted {
            rgba = image::imageops::resize(&rgba, wanted, wanted, image::imageops::FilterType::Lanczos3);
        }
    }
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Some(std::sync::Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])))
}

thread_local! {
    static MEMORY: RefCell<std::collections::HashMap<String, std::sync::Arc<RenderImage>>> = RefCell::new(std::collections::HashMap::new());
}

pub fn cached_cover(url: &str, target: u32) -> Option<std::sync::Arc<RenderImage>> {
    MEMORY.with(|cache| cache.borrow().get(&format!("{url}\n{target}")).cloned())
}

pub fn remember_cover(url: &str, target: u32, image: std::sync::Arc<RenderImage>) {
    MEMORY.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 512 {
            cache.clear();
        }
        cache.insert(format!("{url}\n{target}"), image);
    });
}

pub async fn load_cover(net: &NetHandle, url: &str, target: u32) -> Option<std::sync::Arc<RenderImage>> {
    if url.is_empty() {
        return None;
    }
    if let Some(image) = cached_cover(url, target) {
        return Some(image);
    }
    let http = net.client().http().clone();
    let auth = net.client().media_auth();
    let address = url.to_owned();
    let disk = disk_path(url, Some(target));
    let image = net
        .spawn(async move {
            let bytes = fetch_cover_bytes(&http, auth.as_ref(), &address, Some(target)).await?;
            if let Some(path) = &disk {
                write_disk(path, &bytes).await;
            }
            tokio::task::spawn_blocking(move || decode_cover(&bytes, Some(target))).await.ok().flatten()
        })
        .await
        .ok()
        .flatten()?;
    remember_cover(url, target, image.clone());
    Some(image)
}
