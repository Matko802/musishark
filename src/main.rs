
mod audio;
mod bootstrap;
mod discord;
mod downloads;
mod local_library;
mod lyrics;
mod model;
mod mpris;
mod net;
mod paths;
mod player;
mod presence;
mod queue;
#[cfg(all(target_os = "linux", any(target_arch = "aarch64", target_arch = "x86_64")))]
mod sampler;
mod scrobbler;
mod state;
mod tray;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::mpris::Mpris;
use crate::paths::Paths;
use crate::player::Player;

pub const APP_ID: &str = "io.github.matko802.Musishark";
pub const APP_NAME: &str = "Musishark";

pub struct App {
    pub player: Rc<Player>,
    pub net: net::NetHandle,
    pub downloads: Arc<downloads::Downloads>,
    pub download_events: RefCell<Option<async_channel::Receiver<downloads::Event>>>,
    pub paths: Paths,
    pub mpris: RefCell<Option<Rc<Mpris>>>,
    pub scrobbler: Arc<scrobbler::Scrobbler>,
    pub lyrics: lyrics::Lyrics,
    pub discord: discord::Discord,
    pub local: Arc<local_library::LocalLibrary>,
    pub raise_window: RefCell<Option<Rc<dyn Fn()>>>,
    pub quit_app: RefCell<Option<Rc<dyn Fn()>>>,
}

fn main() -> std::process::ExitCode {
    bootstrap::cap_malloc_arenas();
    bootstrap::raise_fd_limit();

    let paths = Paths::discover();
    bootstrap::init_logging(&paths);
    bootstrap::apply_gsk_renderer_pref(&paths);

    if let Err(err) = gst::init() {
        eprintln!("GStreamer init failed: {err}");
        return std::process::ExitCode::FAILURE;
    }

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("musishark-net")
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("tokio runtime failed: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let net = match net::NetHandle::new(runtime.handle().clone(), &paths) {
        Ok(net) => net,
        Err(err) => {
            eprintln!("network client failed: {err:#}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let (audio, audio_events) = match audio::spawn() {
        Ok(pair) => pair,
        Err(err) => {
            eprintln!("audio engine failed: {err:#}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let (downloads, download_events) = downloads::Downloads::new(paths.clone(), net.clone());
    let scrobbler = scrobbler::Scrobbler::start(&paths, runtime.handle());
    let lyrics = lyrics::Lyrics::new(&paths, net.client().http().clone(), net.client().clone());
    let local = local_library::LocalLibrary::open(&paths);
    let app_ctx = Rc::new(App {
        player: Player::new(net.clone(), downloads.clone(), local.clone(), audio, audio_events, &paths),
        local,
        net,
        downloads,
        download_events: RefCell::new(Some(download_events)),
        scrobbler,
        lyrics,
        discord: discord::Discord::new(discord::enabled_pref(&paths.read_prefs())),
        paths,
        mpris: RefCell::new(None),
        raise_window: RefCell::new(None),
        quit_app: RefCell::new(None),
    });

    let main_context = glib::MainContext::default();
    let _main_context_guard = main_context.acquire().expect("main context is free at startup");
    let main_context_for_ui = main_context.clone();

    tracing::info!(auth = ?app_ctx.net.client().auth_state(), "core started");
    app_ctx.player.start();
    if let Some(crate::net::links::Link::Song { video_id, playlist_id }) = std::env::args().skip(1).find_map(|arg| crate::net::links::parse(&arg)) {
        app_ctx.player.play_link(video_id, playlist_id);
    }
    app_ctx.mpris.replace(Some(Mpris::start(&app_ctx)));
    tray::start(&app_ctx);
    presence::wire(&app_ctx);
    ui::run(&app_ctx, main_context_for_ui);

    tray::stop();
    if let Some(mpris) = app_ctx.mpris.borrow_mut().take() {
        mpris.shutdown();
    }
    app_ctx.scrobbler.stop();
    app_ctx.discord.stop();
    app_ctx.player.shutdown();
    runtime.shutdown_timeout(Duration::from_secs(2));
    std::process::ExitCode::SUCCESS
}
