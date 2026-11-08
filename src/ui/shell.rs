use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use glib::prelude::*;

use gpui::prelude::*;
use gpui::{
    App, Application, Bounds, ClipboardItem, Context, Div, FocusHandle, KeyDownEvent, Pixels, Render, RenderImage, Timer, Window, WindowBounds, WindowOptions, div, px, size, svg,
};

use crate::App as Backend;
use crate::model::{ItemKind, MediaItem, Track};
use crate::net::artist::ArtistData;
use crate::net::explore::{CategorySection, ExploreData};
use crate::net::history::HistoryEntry;
use crate::net::home::HomeSection;
use crate::net::playlists::PlaylistDetails;
use crate::net::search::{SearchFilter, SearchResults};

use super::assets::{self, Assets};
use super::detail;
use super::pages;
use super::player_bar;
use super::queue;
use super::settings::{self, SettingsPage};
use super::theme::{Theme, ThemeMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    Library,
    Explore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryTab {
    Playlists,
    Albums,
    Artists,
    History,
}

#[derive(Clone)]
pub enum Load<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Load<T> {
    pub fn is_idle(&self) -> bool {
        matches!(self, Load::Idle)
    }
}

pub struct HomeData {
    pub dial: Vec<MediaItem>,
    pub sections: Vec<HomeSection>,
}

pub struct LibraryData {
    pub playlists: Load<Vec<MediaItem>>,
    pub albums: Load<Vec<MediaItem>>,
    pub artists: Load<Vec<MediaItem>>,
    pub history: Load<Vec<HistoryEntry>>,
}

#[allow(dead_code)]
pub enum PageKind {
    Playlist { id: String, title: String, thumb: Option<String>, album: bool, data: Load<PlaylistDetails> },
    Artist { id: String, name: String, data: Load<ArtistData> },
    Category { title: String, params: String, data: Load<Vec<CategorySection>> },
    Search { query: String, data: Load<SearchResults> },
    History { data: Load<Vec<HistoryEntry>> },
}

pub struct Root {
    pub backend: Rc<Backend>,
    pub theme: Theme,
    pub font: gpui::SharedString,
    pub tab: Tab,
    pub lib_tab: LibraryTab,
    pub page: Option<PageKind>,
    pub home: Load<HomeData>,
    pub explore: Load<ExploreData>,
    pub library: LibraryData,
    pub search_open: bool,
    pub search_text: String,
    pub search_filter: Option<SearchFilter>,
    pub root_focus: FocusHandle,
    pub queue_open: bool,
    pub queue_generation: u64,
    pub expanded: bool,
    pub expanded_generation: u64,
    pub expanded_lyrics: bool,
    pub settings_open: bool,
    pub settings_closing: bool,
    pub settings_generation: u64,
    pub settings_page: SettingsPage,
    pub theme_from: Theme,
    pub theme_to: Theme,
    pub theme_anim: Option<Instant>,
    pub notices: Rc<RefCell<Vec<String>>>,
    pub toast: Option<(String, Instant)>,
    pub covers: HashSet<String>,
    pub shelf_offsets: HashMap<String, usize>,
    pub viewport_width: f32,
    pub lyrics: HashMap<String, Load<Option<crate::lyrics::LyricsResult>>>,
    pub progress_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub volume_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub raise_flag: Rc<Cell<bool>>,
    pub quit_flag: Rc<Cell<bool>>,
}

pub fn run(backend: &Rc<Backend>, main_context: glib::MainContext) {
    let backend = backend.clone();
    let mode = ThemeMode::from_pref(backend.paths.read_prefs().get("ui_theme").and_then(|v| v.as_str()));
    super::cover::init_disk_cache(&backend.paths.cache_dir);
    {
        let (downloaded, cached, extract) = (backend.downloads.clone(), backend.downloads.clone(), backend.downloads.clone());
        super::cover::set_track_art_lookup(move |id| downloaded.is_downloaded(id), move |id| cached.cached_cover(id), move |id| extract.extract_cover(id));
    }
    Application::new().with_assets(Assets).run(move |cx: &mut App| {
        let main_context = main_context.clone();
        let bounds = Bounds::centered(None, size(px(1040.0), px(700.0)), cx);
        let backend = backend.clone();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(720.0), px(480.0))),
                    titlebar: Some(gpui::TitlebarOptions { title: Some(crate::APP_NAME.into()), ..Default::default() }),
                    app_id: Some(crate::APP_ID.into()),
                    ..Default::default()
                },
                move |window, cx| cx.new(|cx| Root::new(backend, mode, window, cx)),
            )
            .expect("app window");

        let root_entity = window.entity(cx).expect("root entity");
        let (raise_flag, quit_flag) = {
            let root = root_entity.read(cx);
            (root.raise_flag.clone(), root.quit_flag.clone())
        };
        let raise: Rc<dyn Fn()> = Rc::new(move || raise_flag.set(true));
        let quit: Rc<dyn Fn()> = Rc::new(move || quit_flag.set(true));
        let _ = root_entity.update(cx, |root, _| {
            root.backend.raise_window.replace(Some(raise));
            root.backend.quit_app.replace(Some(quit));
        });

        cx.on_window_closed(move |cx| {
            tracing::info!("window closed, quitting");
            cx.quit();
        })
        .detach();

        cx.spawn(async move |cx| loop {
            cx.background_executor().timer(Duration::from_millis(16)).await;
            while main_context.iteration(false) {}
        })
        .detach();

        tracing::info!("window opened");
        cx.activate(true);
    });
    tracing::info!("application loop returned");
}

impl Root {
    pub fn new(backend: Rc<Backend>, mode: ThemeMode, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let notices = Rc::new(RefCell::new(Vec::new()));
        let sink = notices.clone();
        let state = backend.player.state().clone();
        state.connect_local("notice", false, move |values| {
            if let Some(message) = values.get(1).and_then(|v| v.get::<String>().ok()) {
                sink.borrow_mut().push(message);
            }
            None
        });
        let errors = notices.clone();
        state.connect_local("track-error", false, move |values| {
            let title = values.get(2).and_then(|v| v.get::<String>().ok()).unwrap_or_default();
            let message = values.get(3).and_then(|v| v.get::<String>().ok()).unwrap_or_default();
            errors.borrow_mut().push(format!("{title}: {message}"));
            None
        });
        let root_focus = cx.focus_handle();
        root_focus.focus(window);
        window.set_window_title(crate::APP_NAME);
        let root = Self {
            backend,
            theme: Theme::new(mode),
            theme_from: Theme::new(mode),
            theme_to: Theme::new(mode),
            theme_anim: None,
            font: system_font(),
            tab: Tab::Home,
            lib_tab: LibraryTab::Playlists,
            page: None,
            home: Load::Idle,
            explore: Load::Idle,
            library: LibraryData { playlists: Load::Idle, albums: Load::Idle, artists: Load::Idle, history: Load::Idle },
            search_open: false,
            search_text: String::new(),
            search_filter: None,
            root_focus,
            queue_open: false,
            queue_generation: 0,
            expanded: false,
            expanded_generation: 0,
            expanded_lyrics: false,
            settings_open: false,
            settings_closing: false,
            settings_generation: 0,
            settings_page: SettingsPage::General,
            notices,
            toast: None,
            covers: HashSet::new(),
            shelf_offsets: HashMap::new(),
            viewport_width: 1040.0,
            lyrics: HashMap::new(),
            progress_bounds: Rc::new(Cell::new(Bounds::default())),
            volume_bounds: Rc::new(Cell::new(Bounds::default())),
            raise_flag: Rc::new(Cell::new(false)),
            quit_flag: Rc::new(Cell::new(false)),
        };
        cx.spawn(async move |this, cx| loop {
            Timer::after(Duration::from_millis(100)).await;
            let alive = this.update(cx, |root, cx| {
                root.tick(cx);
                cx.notify();
            });
            if alive.is_err() {
                break;
            }
        })
        .detach();
        root
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        if self.quit_flag.take() {
            tracing::info!("quit requested from outside the UI");
            cx.quit();
            return;
        }
        if self.raise_flag.take() {
            cx.activate(true);
        }
        if let Some(message) = self.notices.borrow_mut().pop() {
            self.toast = Some((message, Instant::now() + Duration::from_secs(4)));
        }
        if let Some((_, expires)) = &self.toast {
            if Instant::now() >= *expires {
                self.toast = None;
            }
        }
    }

    pub fn load_home(&mut self, cx: &mut Context<Self>) {
        if !self.home.is_idle() {
            return;
        }
        self.home = Load::Loading;
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        cx.spawn(async move |this, cx| {
            let result = net
                .spawn(async move {
                    let sections = crate::net::home::get_home(api, 14).await.map_err(|err| err.to_string())?;
                    Ok::<_, String>(sections)
                })
                .await
                .ok()
                .and_then(Result::ok);
            let _ = this.update(cx, |root, cx| {
                root.home = match result {
                    Some(sections) => {
                        let (dial, sections) = crate::net::home::arrange(sections);
                        Load::Ready(HomeData { dial, sections })
                    }
                    None => Load::Failed("Could not load Home".to_owned()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub fn load_explore(&mut self, cx: &mut Context<Self>) {
        if !self.explore.is_idle() {
            return;
        }
        self.explore = Load::Loading;
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        let country = self.backend.paths.read_prefs().get("country").and_then(|v| v.as_str()).unwrap_or("US").to_owned();
        cx.spawn(async move |this, cx| {
            let result = net.spawn(async move { crate::net::explore::load_explore(api, country).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
            let _ = this.update(cx, |root, cx| {
                root.explore = match result {
                    Some(data) => Load::Ready(data),
                    None => Load::Failed("Could not load Explore".to_owned()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub fn load_library(&mut self, tab: LibraryTab, cx: &mut Context<Self>) {
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        match tab {
            LibraryTab::Playlists if self.library.playlists.is_idle() => {
                self.library.playlists = Load::Loading;
                cx.spawn(async move |this, cx| {
                    let result = net.spawn(async move { crate::net::library::library_playlists(api).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
                    let _ = this.update(cx, |root, cx| {
                        root.library.playlists = result.map_or_else(|| Load::Failed("Could not load playlists".to_owned()), Load::Ready);
                        cx.notify();
                    });
                })
                .detach();
            }
            LibraryTab::Albums if self.library.albums.is_idle() => {
                self.library.albums = Load::Loading;
                cx.spawn(async move |this, cx| {
                    let result = net.spawn(async move { crate::net::library::library_albums(api).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
                    let _ = this.update(cx, |root, cx| {
                        root.library.albums = result.map_or_else(|| Load::Failed("Could not load albums".to_owned()), Load::Ready);
                        cx.notify();
                    });
                })
                .detach();
            }
            LibraryTab::Artists if self.library.artists.is_idle() => {
                self.library.artists = Load::Loading;
                cx.spawn(async move |this, cx| {
                    let result = net.spawn(async move { crate::net::library::library_subscriptions(api).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
                    let _ = this.update(cx, |root, cx| {
                        root.library.artists = result.map_or_else(|| Load::Failed("Could not load artists".to_owned()), Load::Ready);
                        cx.notify();
                    });
                })
                .detach();
            }
            LibraryTab::History if self.library.history.is_idle() => {
                self.library.history = Load::Loading;
                cx.spawn(async move |this, cx| {
                    let result = net.spawn(async move { crate::net::history::get_history(api).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
                    let _ = this.update(cx, |root, cx| {
                        root.library.history = result.map_or_else(|| Load::Failed("Could not load history".to_owned()), Load::Ready);
                        cx.notify();
                    });
                })
                .detach();
            }
            _ => {}
        }
    }

    pub fn open_playlist(&mut self, id: String, title: String, thumb: Option<String>, album: bool, cx: &mut Context<Self>) {
        self.page = Some(PageKind::Playlist { id: id.clone(), title, thumb, album, data: Load::Loading });
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        cx.spawn(async move |this, cx| {
            let result = net
                .spawn(async move {
                    if album {
                        crate::net::playlists::get_album(&*api, &id).await.map_err(|err| err.to_string())
                    } else {
                        crate::net::playlists::get_playlist(&*api, &id, None).await.map_err(|err| err.to_string())
                    }
                })
                .await
                .ok()
                .and_then(Result::ok);
            let _ = this.update(cx, |root, cx| {
                if let Some(PageKind::Playlist { data, .. }) = root.page.as_mut() {
                    *data = result.map_or_else(|| Load::Failed("Could not load this collection".to_owned()), Load::Ready);
                }
                cx.notify();
            });
        })
        .detach();
        self.refresh(cx);
    }

    pub fn open_artist(&mut self, id: String, name: String, cx: &mut Context<Self>) {
        self.page = Some(PageKind::Artist { id: id.clone(), name, data: Load::Loading });
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        cx.spawn(async move |this, cx| {
            let result = net.spawn(async move { crate::net::artist::get_artist(api, &id).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
            let _ = this.update(cx, |root, cx| {
                if let Some(PageKind::Artist { data, .. }) = root.page.as_mut() {
                    *data = result.map_or_else(|| Load::Failed("Could not load this artist".to_owned()), Load::Ready);
                }
                cx.notify();
            });
        })
        .detach();
        self.refresh(cx);
    }

    pub fn open_category(&mut self, title: String, params: String, cx: &mut Context<Self>) {
        self.page = Some(PageKind::Category { title, params: params.clone(), data: Load::Loading });
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        cx.spawn(async move |this, cx| {
            let result = net.spawn(async move { crate::net::explore::get_category_page(&*api, &params).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
            let _ = this.update(cx, |root, cx| {
                if let Some(PageKind::Category { data, .. }) = root.page.as_mut() {
                    *data = result.map_or_else(|| Load::Failed("Could not load this category".to_owned()), Load::Ready);
                }
                cx.notify();
            });
        })
        .detach();
        self.refresh(cx);
    }

    pub fn open_search(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_open = false;
        self.search_filter = None;
        self.load_search(query, None, cx);
    }

    pub fn set_search_filter(&mut self, filter: Option<SearchFilter>, cx: &mut Context<Self>) {
        let query = match &self.page {
            Some(PageKind::Search { query, .. }) => query.clone(),
            _ => return,
        };
        self.load_search(query, filter, cx);
    }

    fn load_search(&mut self, query: String, filter: Option<SearchFilter>, cx: &mut Context<Self>) {
        self.search_filter = filter;
        self.page = Some(PageKind::Search { query: query.clone(), data: Load::Loading });
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        cx.spawn(async move |this, cx| {
            let result = net
                .spawn(async move {
                    match filter {
                        Some(filter) => crate::net::search::search(&*api, &query, Some(filter)).await,
                        None => crate::net::search::search_all(api, query).await,
                    }
                    .map_err(|err| err.to_string())
                })
                .await
                .ok()
                .and_then(Result::ok);
            let _ = this.update(cx, |root, cx| {
                if let Some(PageKind::Search { data, .. }) = root.page.as_mut() {
                    *data = result.map_or_else(|| Load::Failed("Search failed".to_owned()), Load::Ready);
                }
                cx.notify();
            });
        })
        .detach();
        self.refresh(cx);
    }

    pub fn open_history(&mut self, cx: &mut Context<Self>) {
        self.page = Some(PageKind::History { data: Load::Loading });
        let api = self.backend.net.client().api();
        let net = self.backend.net.clone();
        cx.spawn(async move |this, cx| {
            let result = net.spawn(async move { crate::net::history::get_history(api).await.map_err(|err| err.to_string()) }).await.ok().and_then(Result::ok);
            let _ = this.update(cx, |root, cx| {
                if let Some(PageKind::History { data }) = root.page.as_mut() {
                    *data = result.map_or_else(|| Load::Failed("Could not load history".to_owned()), Load::Ready);
                }
                cx.notify();
            });
        })
        .detach();
        self.refresh(cx);
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    pub fn ensure_lyrics(&mut self, video_id: &str, title: &str, artist: &str, duration: f64, cx: &mut Context<Self>) {
        if video_id.is_empty() || self.lyrics.contains_key(video_id) {
            return;
        }
        self.lyrics.insert(video_id.to_owned(), Load::Loading);
        let lyrics = self.backend.lyrics.clone();
        let net = self.backend.net.clone();
        let query = crate::lyrics::TrackQuery::new(video_id, Some(title), Some(artist), Some(duration as u32));
        let id = video_id.to_owned();
        cx.spawn(async move |this, cx| {
            let result = net.spawn(async move { lyrics.get_lyrics(&query).await }).await.ok().flatten();
            let _ = this.update(cx, |root, cx| {
                root.lyrics.insert(id, Load::Ready(result));
                cx.notify();
            });
        })
        .detach();
    }

    pub fn copy_link(&mut self, url: String, cx: &mut Context<Self>) {
        if url.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(url));
        self.notice("Link copied to clipboard");
        cx.notify();
    }

    pub fn share_track(&mut self, video_id: &str, cx: &mut Context<Self>) {
        if video_id.is_empty() {
            return;
        }
        self.copy_link(format!("https://music.youtube.com/watch?v={video_id}"), cx);
    }

    pub fn share_page(&mut self, cx: &mut Context<Self>) {
        let url = match &self.page {
            Some(PageKind::Playlist { id, album, data, .. }) => {
                let audio_id = match data {
                    Load::Ready(details) => details.audio_playlist_id.clone(),
                    _ => None,
                };
                if id.starts_with("OLAK") || id.starts_with("MPSP") {
                    format!("https://music.youtube.com/playlist?list={id}")
                } else if *album || id.starts_with("MPRE") {
                    match audio_id {
                        Some(playlist) => format!("https://music.youtube.com/playlist?list={playlist}"),
                        None => format!("https://music.youtube.com/browse/{id}"),
                    }
                } else {
                    format!("https://music.youtube.com/playlist?list={id}")
                }
            }
            Some(PageKind::Artist { id, .. }) if !id.is_empty() => format!("https://music.youtube.com/browse/{id}"),
            _ => String::new(),
        };
        self.copy_link(url, cx);
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        self.page = None;
        cx.notify();
    }

    pub fn activate(&mut self, item: MediaItem, cx: &mut Context<Self>) {
        match item.kind {
            ItemKind::Playlist => self.open_playlist(item.id, item.title, item.thumb, false, cx),
            ItemKind::Album => self.open_playlist(item.id, item.title, item.thumb, true, cx),
            ItemKind::Artist => {
                if let Some(id) = item.artists.first().and_then(|a| a.id.clone()).or_else(|| (!item.id.is_empty()).then(|| item.id.clone())) {
                    self.open_artist(id, item.title, cx);
                }
            }
            ItemKind::Song | ItemKind::Video => {
                if let Some(track) = item.to_track() {
                    self.play_tracks(vec![track], 0, false, cx);
                }
            }
        }
    }

    pub fn play_tracks(&mut self, tracks: Vec<Track>, index: usize, shuffle: bool, cx: &mut Context<Self>) {
        if tracks.is_empty() {
            return;
        }
        self.backend.player.play_tracks(tracks, index, shuffle, None, false);
        cx.notify();
    }

    pub fn ensure_cover(&mut self, url: &str, target: u32, cx: &mut Context<Self>) {
        if url.is_empty() || self.covers.contains(url) || super::cover::cached_cover(url, target).is_some() {
            return;
        }
        self.covers.insert(url.to_owned());
        let net = self.backend.net.clone();
        let url = url.to_owned();
        cx.spawn(async move |this, cx| {
            let image = super::cover::load_cover(&net, &url, target).await;
            let _ = this.update(cx, |root, cx| {
                root.covers.remove(&url);
                cx.notify();
            });
            drop(image);
        })
        .detach();
    }

    pub fn cover(&mut self, url: Option<&str>, target: u32, cx: &mut Context<Self>) -> Div {
        let Some(url) = url.filter(|u| !u.is_empty()) else {
            return placeholder_cover(target, self.theme);
        };
        self.ensure_cover(url, target, cx);
        match super::cover::cached_cover(url, target) {
            Some(image) => cover_image(image, target, self.theme),
            None => placeholder_cover(target, self.theme),
        }
    }

    pub fn account_cover(&mut self, target: u32, cx: &mut Context<Self>) -> Div {
        let url = self.backend.player.state().account_photo_url();
        self.cover(Some(&url), target, cx)
    }

    pub fn set_theme(&mut self, mode: ThemeMode, cx: &mut Context<Self>) {
        let target = Theme::new(mode);
        if target.mode == self.theme_to.mode && self.theme_anim.is_none() {
            return;
        }
        let paths = self.backend.paths.clone();
        paths.update_prefs(|prefs| {
            prefs.insert("ui_theme".to_owned(), serde_json::Value::String(mode.key().to_owned()));
        });
        self.theme_from = self.theme;
        self.theme_to = target;
        let start = Instant::now();
        self.theme_anim = Some(start);
        cx.spawn(async move |this, cx| loop {
            Timer::after(Duration::from_millis(16)).await;
            let done = this
                .update(cx, |root, cx| {
                    let Some(start) = root.theme_anim else { return true };
                    let t = (start.elapsed().as_secs_f32() / 0.36).min(1.0);
                    if t >= 1.0 {
                        root.theme = root.theme_to;
                        root.theme_anim = None;
                    } else {
                        root.theme = Theme::lerp(root.theme_from, root.theme_to, spring(t).clamp(0.0, 1.0));
                    }
                    cx.notify();
                    t >= 1.0
                })
                .unwrap_or(true);
            if done {
                break;
            }
        })
        .detach();
        cx.notify();
    }

    pub fn toggle_queue(&mut self, cx: &mut Context<Self>) {
        if !self.queue_open {
            self.queue_generation = self.queue_generation.wrapping_add(1);
        }
        self.queue_open = !self.queue_open;
        cx.notify();
    }

    pub fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        if !self.expanded {
            self.expanded_generation = self.expanded_generation.wrapping_add(1);
            self.expanded_lyrics = false;
        }
        self.expanded = !self.expanded;
        cx.notify();
    }

    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_generation = self.settings_generation.wrapping_add(1);
        self.settings_open = true;
        self.settings_closing = false;
        cx.notify();
    }

    pub fn close_settings(&mut self, cx: &mut Context<Self>) {
        if self.settings_closing || !self.settings_open {
            return;
        }
        self.settings_closing = true;
        let generation = self.settings_generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(200)).await;
            let _ = this.update(cx, |root, cx| {
                if root.settings_generation == generation && root.settings_closing {
                    root.settings_open = false;
                    root.settings_closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn header(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme;
        let tab_labels = [(Tab::Home, "Home", assets::HOME), (Tab::Library, "Library", assets::LIBRARY), (Tab::Explore, "Explore", assets::COMPASS)];
        let mut nav = div().flex().items_center().gap_1().p_1().bg(theme.surface);
        for (tab, label, icon) in tab_labels {
            let selected = self.tab == tab && self.page.is_none();
            let mut button = div()
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .cursor_pointer()
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(if selected { theme.on_accent } else { theme.text })
                .when(selected, |el| el.bg(theme.text))
                .when(!selected, |el| el.hover(move |s| s.bg(theme.surface_hover)))
                .child(svg().path(icon).size(px(15.0)).text_color(if selected { theme.on_accent } else { theme.text }))
                .child(label);
            button = button.on_mouse_down(gpui::MouseButton::Left, press(cx, move |root, _, _, cx| {
                root.tab = tab;
                root.page = None;
                match tab {
                    Tab::Home => root.load_home(cx),
                    Tab::Explore => root.load_explore(cx),
                    Tab::Library => root.load_library(root.lib_tab, cx),
                }
                cx.notify();
            }));
            nav = nav.child(button);
        }

        let search_button = icon_button(assets::SEARCH, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, press(cx, |root, _, _, cx| {
            root.search_open = !root.search_open;
            cx.notify();
        }));

        let mut search = if self.search_open {
            let theme = self.theme;
            let value = self.search_text.clone();
            Some(
                div()
                    .id("search-field")
                    .flex()
                    .items_center()
                    .gap_2()
                    .w(px(340.0))
                    .px_3()
                    .py_2()
                    .ml_2()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .child(svg().path(assets::SEARCH).size(px(15.0)).text_color(theme.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .text_color(if value.is_empty() { theme.muted } else { theme.text })
                            .child(if value.is_empty() { "Search songs, albums, artists".to_owned() } else { value }),
                    )
                    .child(div().w(px(1.5)).h(px(16.0)).bg(theme.text)),
            )
        } else {
            None
        };

        let left = div()
            .flex()
            .items_center()
            .flex_1()
            .child(search_button)
            .when_some(search.take(), |el, field| el.child(field));

        let right = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .flex_1()
            .child(icon_button(if theme.dark { assets::SUN } else { assets::MOON }, 17.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, press(cx, |root, _, _, cx| {
                let next = if root.theme.dark { ThemeMode::White } else { ThemeMode::Amoled };
                root.set_theme(next, cx);
            })))
            .child(icon_button(assets::QUEUE, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, press(cx, |root, _, _, cx| {
                root.toggle_queue(cx);
            })))
            .child(
                div()
                    .size(px(32.0))
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, press(cx, |root, _, _, cx| {
                        if root.settings_open {
                            root.close_settings(cx);
                        } else {
                            root.open_settings(cx);
                        }
                    }))
                    .child(self.account_cover(32, cx)),
            );

        div()
            .flex()
            .items_center()
            .gap_3()
            .h(px(64.0))
            .px_4()
            .flex_shrink_0()
            .bg(theme.bg)
            .child(left)
            .child(nav)
            .child(right)
    }

    fn search_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        match key {
            "escape" => {
                self.search_open = false;
                self.root_focus.focus(window);
            }
            "enter" => {
                let query = self.search_text.trim().to_owned();
                if query.len() >= 2 {
                    self.search_text.clear();
                    self.open_search(query, cx);
                }
                self.root_focus.focus(window);
            }
            "backspace" => {
                self.search_text.pop();
            }
            "space" => {
                self.search_text.push(' ');
            }
            _ => {
                if event.keystroke.modifiers.control || event.keystroke.modifiers.platform {
                    return;
                }
                if let Some(character) = event.keystroke.key.chars().next().filter(|_| event.keystroke.key.chars().count() == 1) {
                    self.search_text.push(character);
                }
            }
        }
        cx.notify();
    }

    fn global_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            self.search_key(event, window, cx);
            return;
        }
        let key = event.keystroke.key.as_str();
        if self.settings_open {
            match key {
                "escape" => self.close_settings(cx),
                "1" => self.settings_page = SettingsPage::General,
                "2" => self.settings_page = SettingsPage::Appearance,
                "3" => self.settings_page = SettingsPage::Lyrics,
                "4" => self.settings_page = SettingsPage::Services,
                "5" => self.settings_page = SettingsPage::Advanced,
                _ => cx.notify(),
            }
            return;
        }
        let command = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        match key {
            "/" => {
                self.search_open = true;
                cx.notify();
            }
            "f" if command => {
                self.search_open = true;
                cx.notify();
            }
            "," => {
                if self.settings_open {
                    self.close_settings(cx);
                } else {
                    self.open_settings(cx);
                }
            }
            "escape" => {
                if self.settings_open {
                    self.settings_open = false;
                } else if self.expanded {
                    self.expanded = false;
                } else if self.queue_open {
                    self.queue_open = false;
                } else if self.page.is_some() {
                    self.page = None;
                }
                cx.notify();
            }
            "q" => self.toggle_queue(cx),
            "e" => self.toggle_expanded(cx),
            "t" => {
                let next = if self.theme_to.dark { ThemeMode::White } else { ThemeMode::Amoled };
                self.set_theme(next, cx);
            }
            " " => {
                self.backend.player.toggle_play();
                cx.notify();
            }
            _ => {}
        }
    }

    pub fn notice(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now() + Duration::from_secs(4)));
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.viewport_width = window.viewport_size().width.into();
        let theme = self.theme;
        let content: gpui::AnyElement = match &self.page {
            Some(_) => detail::page(self, cx),
            None => match self.tab {
                Tab::Home => pages::home(self, cx),
                Tab::Library => pages::library(self, cx),
                Tab::Explore => pages::explore(self, cx),
            },
        };

        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .text_color(theme.text)
            .font_family(self.font.clone())
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(|root, event: &KeyDownEvent, window, cx| root.global_key(event, window, cx)));
        root = root.child(self.header(cx));
        let mut body = div().flex_1().min_h_0().relative().overflow_hidden().child(content);
        if self.queue_open {
            body = body.child(queue::panel(self, cx));
        }
        root = root.child(body);
        root = root.child(player_bar::bar(self, window, cx));
        if self.expanded {
            root = root.child(player_bar::expanded(self, cx));
        }
        if self.settings_open {
            root = root.child(settings::panel(self, cx));
        }
        if let Some((message, _)) = &self.toast {
            let message = message.clone();
            root = root.child(
                div()
                    .absolute()
                    .bottom(px(96.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(div().px_4().py_2().bg(theme.elevated).border_1().border_color(theme.border).text_sm().child(message)),
            );
        }
        root
    }
}

pub fn spring(delta: f32) -> f32 {
    if delta >= 1.0 {
        return 1.0;
    }
    let t = delta.max(0.0);
    (1.0 - (-6.0 * t).exp() * (1.0 + 6.0 * t)).clamp(0.0, 1.0)
}

fn system_font() -> gpui::SharedString {
    std::process::Command::new("fc-match")
        .args(["-f", "%{family[0]}", "sans-serif"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|family| family.trim().to_owned())
        .filter(|family| !family.is_empty())
        .map(gpui::SharedString::from)
        .unwrap_or_else(|| gpui::SharedString::from(".SystemUIFont"))
}

pub fn press<T: 'static>(
    cx: &Context<T>,
    f: impl Fn(&mut T, &gpui::MouseDownEvent, &mut Window, &mut Context<T>) + 'static,
) -> impl Fn(&gpui::MouseDownEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
        cx.stop_propagation();
        f(this, event, window, cx);
    })
}

pub fn icon_button(path: &'static str, size: f32, color: gpui::Rgba, hover: gpui::Rgba) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(size + 16.0))
        .cursor_pointer()
        .text_color(color)
        .hover(move |s| s.bg(hover))
        .child(svg().path(path).size(px(size)).text_color(color))
}

pub fn cover_image(image: Arc<RenderImage>, target: u32, theme: Theme) -> Div {
    let side = target as f32;
    div()
        .size(px(side))
        .flex_shrink_0()
        .overflow_hidden()
        .bg(theme.surface)
        .child(gpui::img(image).size_full().object_fit(gpui::ObjectFit::Cover))
}

pub fn placeholder_cover(target: u32, theme: Theme) -> Div {
    let side = target as f32;
    div()
        .size(px(side))
        .flex_shrink_0()
        .overflow_hidden()
        .bg(theme.surface)
        .flex()
        .items_center()
        .justify_center()
        .child(svg().path(assets::MUSIC).size(px((side * 0.3).max(12.0))).text_color(theme.muted))
}

pub fn format_duration(seconds: Option<u32>) -> String {
    let Some(seconds) = seconds else { return String::new() };
    if seconds >= 3600 {
        format!("{}:{:02}:{:02}", seconds / 3600, (seconds % 3600) / 60, seconds % 60)
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}

pub fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0:00".to_owned();
    }
    let total = seconds as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

pub fn join_detail(parts: impl IntoIterator<Item = String>) -> String {
    parts.into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" · ")
}

pub fn loading(theme: Theme) -> Div {
    div().flex().items_center().justify_center().py_16().text_sm().text_color(theme.muted).child("Loading…")
}

pub fn empty_note(theme: Theme, message: &str) -> Div {
    div().flex().items_center().justify_center().py_16().text_sm().text_color(theme.muted).child(message.to_owned())
}

pub fn failed(theme: Theme, message: &str) -> Div {
    div().flex().items_center().justify_center().py_16().text_sm().text_color(theme.danger).child(message.to_owned())
}

pub fn heading(theme: Theme, title: &str, icon_path: Option<&'static str>) -> Div {
    let mut row = div().flex().items_center().gap_2().mb_4().text_xl().font_weight(gpui::FontWeight::BOLD).text_color(theme.text);
    if let Some(icon_path) = icon_path {
        row = row.child(svg().path(icon_path).size(px(20.0)).text_color(theme.text));
    }
    row.child(title.to_owned())
}
