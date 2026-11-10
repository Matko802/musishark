use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use glib::prelude::*;

use gpui::prelude::*;
use gpui::{
    Animation, AnimationExt as _, App, Application, Bounds, ClipboardItem, Context, Div, FocusHandle, KeyDownEvent, Pixels, Render, RenderImage, Rgba, Timer, Window, WindowAppearance, WindowBounds, WindowOptions, canvas, div, px, size, svg,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub tab_generation: u64,
    pub nav_prev: Option<Tab>,
    pub lib_generation: u64,
    pub lib_prev: Option<LibraryTab>,
    pub search_generation: u64,
    pub content_slide: f32,
    pub nav_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub nav_tab_bounds: HashMap<Tab, Rc<Cell<Bounds<Pixels>>>>,
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
    pub expanded_closing: bool,
    pub expanded_generation: u64,
    pub viewport_height: f32,
    pub expanded_lyrics: bool,
    pub compact_bar_anim: bool,
    pub morph_bounds: HashMap<&'static str, Rc<Cell<Bounds<Pixels>>>>,
    pub morph_from: HashMap<&'static str, Bounds<Pixels>>,
    pub morph_to: HashMap<&'static str, Bounds<Pixels>>,
    pub settings_open: bool,
    pub settings_page: SettingsPage,
    pub notices: Rc<RefCell<Vec<String>>>,
    pub toast: Option<(String, Instant)>,
    pub covers: HashSet<String>,
    pub shelf_offsets: HashMap<String, usize>,
    pub viewport_width: f32,
    pub lyrics: HashMap<String, Load<Option<crate::lyrics::LyricsResult>>>,
    pub lyrics_active: HashMap<String, usize>,
    pub scroll_offsets: HashMap<&'static str, f32>,
    pub scroll_targets: HashMap<&'static str, f32>,
    pub scroll_metrics: HashMap<&'static str, Rc<Cell<(f32, f32)>>>,
    pub scroll_activity: HashMap<&'static str, Instant>,
    pub scroll_hover: HashSet<&'static str>,
    pub scrollbar_bounds: HashMap<&'static str, Rc<Cell<Bounds<Pixels>>>>,
    pub scroll_animating: bool,
    pub track_lists: HashMap<String, Rc<Vec<Track>>>,
    pub progress_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub volume_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub raise_flag: Rc<Cell<bool>>,
    pub quit_flag: Rc<Cell<bool>>,
}

pub fn run(backend: &Rc<Backend>, main_context: glib::MainContext) {
    let backend = backend.clone();
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
                move |window, cx| {
                    let root = cx.new(|cx| Root::new(backend, window, cx));
                    let weak = root.downgrade();
                    window
                        .observe_window_appearance(move |window, cx| {
                            let dark = matches!(window.appearance(), WindowAppearance::Dark | WindowAppearance::VibrantDark);
                            weak.update(cx, |root, cx| root.apply_system_theme(dark, cx)).ok();
                        })
                        .detach();
                    root
                },
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
    pub fn new(backend: Rc<Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dark = system_dark_preference()
            .unwrap_or_else(|| matches!(window.appearance(), WindowAppearance::Dark | WindowAppearance::VibrantDark));
        let mode = if dark { ThemeMode::Amoled } else { ThemeMode::White };
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
            font: system_font(),
            tab: Tab::Home,
            lib_tab: LibraryTab::Playlists,
            tab_generation: 0,
            nav_prev: None,
            lib_generation: 0,
            lib_prev: None,
            search_generation: 0,
            content_slide: 1.0,
            nav_bounds: Rc::new(Cell::new(Bounds::default())),
            nav_tab_bounds: HashMap::new(),
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
            expanded_closing: false,
            expanded_generation: 0,
            viewport_height: 700.0,
            expanded_lyrics: false,
            compact_bar_anim: false,
            morph_bounds: HashMap::new(),
            morph_from: HashMap::new(),
            morph_to: HashMap::new(),
            settings_open: false,
            settings_page: SettingsPage::General,
            notices,
            toast: None,
            covers: HashSet::new(),
            shelf_offsets: HashMap::new(),
            viewport_width: 1040.0,
            lyrics: HashMap::new(),
            lyrics_active: HashMap::new(),
            scroll_offsets: HashMap::new(),
            scroll_targets: HashMap::new(),
            scroll_metrics: HashMap::new(),
            scroll_activity: HashMap::new(),
            scroll_hover: HashSet::new(),
            scrollbar_bounds: HashMap::new(),
            scroll_animating: false,
            track_lists: HashMap::new(),
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
        self.search_text.clear();
        self.reset_scroll("playlist");
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
            let cached = result.as_ref().map(|details| (details.id.clone(), Rc::new(details.tracks.clone())));
            let _ = this.update(cx, |root, cx| {
                if let Some((id, tracks)) = cached {
                    root.track_lists.insert(id, tracks);
                }
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
        self.search_text.clear();
        self.reset_scroll("artist");
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
        self.search_text.clear();
        self.reset_scroll("category");
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
        self.search_text.clear();
        self.set_search_open(false, cx);
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
        self.reset_scroll("search");
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
        self.search_text.clear();
        self.reset_scroll("history");
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

    fn reset_scroll(&mut self, id: &'static str) {
        self.scroll_offsets.insert(id, 0.0);
        self.scroll_targets.insert(id, 0.0);
    }

    fn step_scroll(&mut self) -> bool {
        let mut moving = false;
        for (id, target) in self.scroll_targets.iter_mut() {
            let max = self.scroll_metrics.get(id).map_or(0.0, |metrics| {
                let (content, viewport) = metrics.get();
                (content - viewport).max(0.0)
            });
            *target = target.clamp(0.0, max);
            let current = self.scroll_offsets.entry(*id).or_insert(0.0);
            let diff = *target - *current;
            if diff.abs() > 0.5 {
                *current += diff * 0.3;
                moving = true;
            } else {
                *current = *target;
            }
        }
        moving
    }

    pub fn start_scroll(&mut self, id: &'static str, cx: &mut Context<Self>) {
        self.scroll_activity.retain(|_, at| at.elapsed() < Duration::from_millis(4000));
        self.scroll_activity.insert(id, Instant::now());
        if self.scroll_animating {
            return;
        }
        self.scroll_animating = true;
        cx.spawn(async move |this, cx| loop {
            Timer::after(Duration::from_millis(16)).await;
            let (moving, visible) = this
                .update(cx, |root, cx| {
                    let moving = root.step_scroll();
                    let visible = root.scroll_activity.values().any(|at| at.elapsed() < Duration::from_millis(1100));
                    cx.notify();
                    (moving, visible)
                })
                .unwrap_or((false, false));
            if !moving && !visible {
                let _ = this.update(cx, |root, _| root.scroll_animating = false);
                break;
            }
        })
        .detach();
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
        self.search_text.clear();
        self.page = None;
        cx.notify();
    }

    pub fn scoped_search(&self) -> bool {
        match &self.page {
            Some(PageKind::Playlist { .. }) | Some(PageKind::Artist { .. }) | Some(PageKind::History { .. }) => true,
            Some(_) => false,
            None => self.tab == Tab::Library,
        }
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

    pub fn activate_in(&mut self, item: MediaItem, pool: Rc<Vec<MediaItem>>, radio: bool, cx: &mut Context<Self>) {
        if !matches!(item.kind, ItemKind::Song | ItemKind::Video) {
            self.activate(item, cx);
            return;
        }
        let mut tracks: Vec<Track> = pool.iter().filter_map(MediaItem::to_track).collect();
        let mut index = tracks.iter().position(|track| track.video_id.as_str() == item.id).unwrap_or(0);
        if tracks.is_empty() {
            let Some(track) = item.to_track() else { return };
            tracks = vec![track];
            index = 0;
        }
        if radio {
            let seed = tracks.last().map(|track| track.video_id.0.clone()).unwrap_or_default();
            self.backend.player.play_then_radio(tracks, index, &seed);
            cx.notify();
        } else {
            self.play_tracks(tracks, index, false, cx);
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

    pub fn apply_system_theme(&mut self, dark: bool, cx: &mut Context<Self>) {
        let mode = if dark { ThemeMode::Amoled } else { ThemeMode::White };
        if self.theme.mode == mode {
            return;
        }
        self.theme = Theme::new(mode);
        cx.notify();
    }

    pub fn toggle_queue(&mut self, cx: &mut Context<Self>) {
        if !self.queue_open {
            self.queue_generation = self.queue_generation.wrapping_add(1);
        }
        self.queue_open = !self.queue_open;
        cx.notify();
    }

    pub fn morph_bounds(&mut self, key: &'static str) -> Rc<Cell<Bounds<Pixels>>> {
        self.morph_bounds.entry(key).or_insert_with(|| Rc::new(Cell::new(Bounds::default()))).clone()
    }

    fn snapshot_morphs(&mut self, opening: bool) {
        let kinds = [
            ("cover", "compact-cover", "expanded-cover"),
            ("title", "compact-title", "expanded-title"),
            ("play", "compact-play", "expanded-play"),
        ];
        for (kind, compact, expanded) in kinds {
            let (from_key, to_key) = if opening { (compact, expanded) } else { (expanded, compact) };
            let from = self.morph_bounds.get(from_key).map_or_else(Bounds::default, |bounds| bounds.get());
            let to = self.morph_bounds.get(to_key).map_or_else(Bounds::default, |bounds| bounds.get());
            self.morph_from.insert(kind, from);
            self.morph_to.insert(kind, to);
        }
    }

    pub fn open_expanded(&mut self, cx: &mut Context<Self>) {
        if !self.expanded {
            self.expanded_generation = self.expanded_generation.wrapping_add(1);
            self.expanded_lyrics = false;
            self.compact_bar_anim = true;
            self.snapshot_morphs(true);
            let generation = self.expanded_generation;
            cx.spawn(async move |this, cx| {
                Timer::after(Duration::from_millis(380)).await;
                let _ = this.update(cx, |root, cx| {
                    if root.expanded_generation == generation {
                        root.compact_bar_anim = false;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        self.expanded = true;
        self.expanded_closing = false;
        cx.notify();
    }

    pub fn close_expanded(&mut self, cx: &mut Context<Self>) {
        if !self.expanded {
            return;
        }
        self.expanded = false;
        self.expanded_closing = true;
        self.compact_bar_anim = false;
        self.snapshot_morphs(false);
        let generation = self.expanded_generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(240)).await;
            let _ = this.update(cx, |root, cx| {
                if root.expanded_generation == generation && root.expanded_closing {
                    root.expanded_closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub fn toggle_expanded(&mut self, cx: &mut Context<Self>) {
        if self.expanded {
            self.close_expanded(cx);
        } else {
            self.open_expanded(cx);
        }
    }

    fn set_search_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.search_open != open {
            self.search_open = open;
            self.search_generation = self.search_generation.wrapping_add(1);
        }
        cx.notify();
    }

    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = true;
        cx.notify();
    }

    pub fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_open = false;
        cx.notify();
    }

    fn header(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme;
        let open = self.search_open;
        let tab_labels = [(Tab::Home, "Home", assets::HOME), (Tab::Library, "Library", assets::LIBRARY), (Tab::Explore, "Explore", assets::COMPASS)];
        let generation = self.tab_generation;
        let current_tab = (self.page.is_none()).then_some(self.tab);
        let prev_tab = if self.page.is_none() { self.nav_prev } else { None };
        let animating = generation > 0 && current_tab.is_some() && prev_tab.is_some() && prev_tab != current_tab;
        let nav_bounds_cell = self.nav_bounds.clone();
        let mut nav = div().relative().flex().items_center().gap_1().p_1().bg(theme.surface);
        nav = nav.child(canvas(move |bounds, _, _| nav_bounds_cell.set(bounds), |_, _, _, _| {}).absolute().inset_0());
        if animating {
            if let (Some(previous), Some(current)) = (prev_tab, current_tab) {
                let nav_box = self.nav_bounds.get();
                let from_box = self.nav_tab_bounds.get(&previous).map(|cell| cell.get());
                let to_box = self.nav_tab_bounds.get(&current).map(|cell| cell.get());
                if let (Some(from_box), Some(to_box)) = (from_box, to_box) {
                    if from_box.size.width > px(0.0) && to_box.size.width > px(0.0) {
                        let (fx, fy) = (f32::from(from_box.origin.x - nav_box.origin.x), f32::from(from_box.origin.y - nav_box.origin.y));
                        let (fw, fh) = (f32::from(from_box.size.width), f32::from(from_box.size.height));
                        let (tx, ty) = (f32::from(to_box.origin.x - nav_box.origin.x), f32::from(to_box.origin.y - nav_box.origin.y));
                        let (tw, th) = (f32::from(to_box.size.width), f32::from(to_box.size.height));
                        nav = nav.child(
                            div()
                                .absolute()
                                .bg(theme.text)
                                .with_animation(
                                    gpui::SharedString::from(format!("nav-indicator-{generation}")),
                                    Animation::new(Duration::from_millis(260)).with_easing(spring),
                                    move |el, delta| {
                                        el.left(px(fx + (tx - fx) * delta))
                                            .top(px(fy + (ty - fy) * delta))
                                            .w(px(fw + (tw - fw) * delta))
                                            .h(px(fh + (th - fh) * delta))
                                    },
                                ),
                        );
                    }
                }
            }
        }
        if self.search_generation > 0 {
            let mut field = div().id("search-field").flex().items_center().gap_2().h(px(36.0)).overflow_hidden().text_sm();
            if open {
                let value = self.search_text.clone();
                let empty = value.is_empty();
                let cursor = || {
                    div()
                        .flex_shrink_0()
                        .w(px(1.5))
                        .h(px(16.0))
                        .bg(theme.text)
                        .with_animation(
                            gpui::SharedString::from("search-cursor"),
                            Animation::new(Duration::from_millis(1060)).repeat(),
                            |el, delta| el.opacity(if delta < 0.5 { 1.0 } else { 0.0 }),
                        )
                        .into_any_element()
                };
                let text = div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(if empty { "Search songs, albums, artists".to_owned() } else { value });
                let mut row = div().flex().items_center().gap_1().flex_1().min_w_0().overflow_hidden();
                if empty {
                    row = row.child(cursor()).child(text);
                } else {
                    row = row.child(text).child(cursor());
                }
                field = field
                    .px_3()
                    .text_color(if empty { theme.muted } else { theme.text })
                    .child(svg().path(assets::SEARCH).size(px(15.0)).text_color(theme.muted))
                    .child(row);
            }
            let (from, to) = if open { (0.0, 280.0) } else { (280.0, 0.0) };
            nav = nav.child(
                field
                    .with_animation(
                        gpui::SharedString::from(format!("search-field-{}", self.search_generation)),
                        Animation::new(Duration::from_millis(220)).with_easing(spring),
                        move |el, delta| el.w(px(from + (to - from) * delta)),
                    )
                    .into_any_element(),
            );
        } else if open {
            nav = nav.child(div().id("search-field").flex().items_center().gap_2().w(px(280.0)).h(px(36.0)).px_3().text_sm());
        }
        for (tab, label, icon) in tab_labels {
            let selected = current_tab == Some(tab);
            let previous = animating && prev_tab == Some(tab);
            let icon_el: gpui::AnyElement = if animating {
                svg()
                    .path(icon)
                    .size(px(15.0))
                    .with_animation(
                        gpui::SharedString::from(format!("nav-icon-{generation}-{label}")),
                        Animation::new(Duration::from_millis(220)).with_easing(spring),
                        move |el, delta| {
                            let p = if selected { delta } else { 1.0 - delta };
                            el.text_color(mix(theme.text, theme.on_accent, p))
                        },
                    )
                    .into_any_element()
            } else {
                svg().path(icon).size(px(15.0)).text_color(if selected { theme.on_accent } else { theme.text }).into_any_element()
            };
            let mut button = div()
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .size_full()
                .cursor_pointer()
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(if selected { theme.on_accent } else { theme.text })
                .when(selected && !animating, |el| el.bg(theme.text))
                .when(!selected && !previous, move |el| el.hover(move |s| s.bg(theme.surface_hover)))
                .child(icon_el)
                .when(!open, |el| el.child(label));
            button = button.on_mouse_down(gpui::MouseButton::Left, press(cx, move |root, _, _, cx| {
                let changed = root.tab != tab || root.page.is_some();
                if changed {
                    let was_root = root.page.is_none() && root.tab != tab;
                    root.nav_prev = was_root.then_some(root.tab);
                    root.content_slide = if tab_order(tab) >= tab_order(root.tab) { 1.0 } else { -1.0 };
                    root.tab_generation = root.tab_generation.wrapping_add(1);
                }
                root.search_text.clear();
                root.tab = tab;
                root.page = None;
                match tab {
                    Tab::Home => root.load_home(cx),
                    Tab::Explore => root.load_explore(cx),
                    Tab::Library => root.load_library(root.lib_tab, cx),
                }
                cx.notify();
            }));
            let button_el: gpui::AnyElement = if animating {
                button
                    .with_animation(
                        gpui::SharedString::from(format!("nav-tab-{generation}-{label}")),
                        Animation::new(Duration::from_millis(220)).with_easing(spring),
                        move |el, delta| {
                            let p = if selected { delta } else { 1.0 - delta };
                            el.text_color(mix(theme.text, theme.on_accent, p))
                        },
                    )
                    .into_any_element()
            } else {
                button.into_any_element()
            };
            let cell = Rc::new(Cell::new(Bounds::default()));
            self.nav_tab_bounds.insert(tab, cell.clone());
            let outer = div()
                .relative()
                .h(px(36.0))
                .overflow_hidden()
                .child(button_el)
                .child(canvas(move |bounds, _, _| cell.set(bounds), |_, _, _, _| {}).absolute().inset_0());
            if self.search_generation > 0 {
                let target = if open { 36.0 } else { 96.0 };
                let from = if open { 96.0 } else { 36.0 };
                nav = nav.child(
                    outer
                        .with_animation(
                            gpui::SharedString::from(format!("search-tab-{}-{label}", self.search_generation)),
                            Animation::new(Duration::from_millis(220)).with_easing(spring),
                            move |el, delta| el.w(px(from + (target - from) * delta)),
                        )
                        .into_any_element(),
                );
            } else {
                nav = nav.child(outer.w(px(if open { 36.0 } else { 96.0 })));
            }
        }

        let search_button = icon_button(assets::SEARCH, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, press(cx, |root, _, _, cx| {
            root.set_search_open(!root.search_open, cx);
        }));

        let left = div()
            .flex()
            .items_center()
            .flex_1()
            .child(search_button);

        let right = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .flex_1()
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
                self.search_text.clear();
                self.set_search_open(false, cx);
                self.root_focus.focus(window);
            }
            "enter" => {
                if self.scoped_search() {
                    self.set_search_open(false, cx);
                    self.root_focus.focus(window);
                    return;
                }
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
            "/" => self.set_search_open(true, cx),
            "f" if command => self.set_search_open(true, cx),
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
                } else if !self.search_text.is_empty() && self.scoped_search() {
                    self.search_text.clear();
                } else if self.expanded {
                    self.close_expanded(cx);
                } else if self.queue_open {
                    self.queue_open = false;
                } else if self.page.is_some() {
                    self.page = None;
                }
                cx.notify();
            }
            "q" => self.toggle_queue(cx),
            "e" => self.toggle_expanded(cx),
            "l" if self.expanded => {
                self.expanded_lyrics = !self.expanded_lyrics;
                cx.notify();
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
        self.viewport_height = window.viewport_size().height.into();
        let theme = self.theme;
        let content: gpui::AnyElement = match &self.page {
            Some(_) => detail::page(self, cx),
            None => match self.tab {
                Tab::Home => pages::home(self, cx),
                Tab::Library => pages::library(self, cx),
                Tab::Explore => pages::explore(self, cx),
            },
        };
        let content = if self.tab_generation == 0 && self.lib_generation == 0 {
            content
        } else {
            let key = gpui::SharedString::from(format!("page-{}-{}", self.tab_generation, self.lib_generation));
            let slide = self.content_slide;
            div()
                .size_full()
                .child(content)
                .with_animation(key, Animation::new(Duration::from_millis(240)).with_easing(spring), move |el, delta| el.left(px(slide * (1.0 - delta) * 32.0)))
                .into_any_element()
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
        if self.expanded || self.expanded_closing {
            root = root.child(player_bar::expanded(self, cx));
        }
        let morphing = !self.expanded_lyrics && (self.expanded_closing || (self.expanded && self.compact_bar_anim));
        if morphing {
            root = root.child(player_bar::controls_morph(self, cx));
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

fn tab_order(tab: Tab) -> i32 {
    match tab {
        Tab::Home => 0,
        Tab::Library => 1,
        Tab::Explore => 2,
    }
}

pub fn lib_order(tab: LibraryTab) -> i32 {
    match tab {
        LibraryTab::Playlists => 0,
        LibraryTab::Albums => 1,
        LibraryTab::Artists => 2,
        LibraryTab::History => 3,
    }
}

pub fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: a.a + (b.a - a.a) * t }
}

pub fn spring(delta: f32) -> f32 {
    if delta >= 1.0 {
        return 1.0;
    }
    let t = delta.max(0.0);
    (1.0 - (-6.0 * t).exp() * (1.0 + 6.0 * t)).clamp(0.0, 1.0)
}

fn system_dark_preference() -> Option<bool> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>).ok()?;
    let reply = connection
        .call_sync(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings",
            "Read",
            Some(&("org.freedesktop.appearance", "color-scheme").to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            500,
            None::<&gio::Cancellable>,
        )
        .ok()?;
    let (scheme,) = reply.get::<(glib::Variant,)>()?;
    match scheme.get::<u32>()? {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
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

fn scroll_impl(
    root: &mut Root,
    id: &'static str,
    known_height: Option<f32>,
    make_content: impl FnOnce(&mut Root, f32, &mut Context<Root>) -> Div,
    cx: &mut Context<Root>,
) -> gpui::AnyElement {
    let theme = root.theme;
    let metrics = root.scroll_metrics.entry(id).or_insert_with(|| Rc::new(Cell::new((0.0, 0.0)))).clone();
    let bar_bounds = root.scrollbar_bounds.entry(id).or_insert_with(|| Rc::new(Cell::new(Bounds::default()))).clone();
    if let Some(total) = known_height {
        let (_, viewport_height) = metrics.get();
        metrics.set((total, viewport_height));
    }
    let max = {
        let (content_height, viewport_height) = metrics.get();
        (content_height - viewport_height).max(0.0)
    };
    let current = root.scroll_offsets.entry(id).or_insert(0.0);
    if *current > max {
        *current = max;
    }
    let current = *current;
    let content = make_content(root, current, cx);
    let measured = known_height.is_none();
    let (content_height, viewport_height) = metrics.get();
    let recently = root.scroll_activity.get(id).is_some_and(|at| at.elapsed() < Duration::from_millis(1100));
    let hovered = root.scroll_hover.contains(id);
    let show_bar = max > 0.0 && (recently || hovered);
    let thumb_height = if content_height > 0.0 { (viewport_height * viewport_height / content_height).max(36.0).min(viewport_height) } else { 0.0 };
    let thumb_top = if max > 0.0 { (current / max) * (viewport_height - thumb_height).max(0.0) } else { 0.0 };
    let thumb_color = gpui::Rgba { a: if hovered { 0.82 } else { 0.42 }, ..theme.muted };
    let wheel_metrics = metrics.clone();
    let content_metrics = metrics.clone();
    let viewport_metrics = metrics.clone();
    let drag_metrics_down = metrics.clone();
    let drag_bounds_down = bar_bounds.clone();
    let drag_metrics_move = metrics.clone();
    let drag_bounds_move = bar_bounds.clone();
    let mut rail = div()
        .id(gpui::SharedString::from(format!("scroll-rail-{id}")))
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .w(px(12.0))
        .on_hover(cx.listener(move |root, hovered: &bool, _, cx| {
            let changed = if *hovered { root.scroll_hover.insert(id) } else { root.scroll_hover.remove(&id) };
            if changed {
                cx.notify();
            }
        }))
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |root, event: &gpui::MouseDownEvent, _, cx| {
            let bounds = drag_bounds_down.get();
            let height: f32 = bounds.size.height.into();
            if height <= 0.0 {
                return;
            }
            let (content_height, viewport_height) = drag_metrics_down.get();
            let max = (content_height - viewport_height).max(0.0);
            let y: f32 = (event.position.y - bounds.origin.y).into();
            let value = (y / height).clamp(0.0, 1.0) * max;
            root.scroll_targets.insert(id, value);
            root.scroll_offsets.insert(id, value);
            root.start_scroll(id, cx);
            cx.notify();
        }))
        .on_mouse_move(cx.listener(move |root, event: &gpui::MouseMoveEvent, _, cx| {
            if event.pressed_button != Some(gpui::MouseButton::Left) {
                return;
            }
            let bounds = drag_bounds_move.get();
            let height: f32 = bounds.size.height.into();
            if height <= 0.0 {
                return;
            }
            let (content_height, viewport_height) = drag_metrics_move.get();
            let max = (content_height - viewport_height).max(0.0);
            let y: f32 = (event.position.y - bounds.origin.y).into();
            let value = (y / height).clamp(0.0, 1.0) * max;
            root.scroll_targets.insert(id, value);
            root.scroll_offsets.insert(id, value);
            root.start_scroll(id, cx);
            cx.notify();
        }))
        .child(canvas(
            move |bounds, _, _| {
                bar_bounds.set(bounds);
            },
            |_, _, _, _| {},
        ).absolute().inset_0());
    if show_bar {
        rail = rail.child(
            div()
                .absolute()
                .right(px(3.0))
                .top(px(thumb_top))
                .w(px(6.0))
                .h(px(thumb_height))
                .bg(thumb_color),
        );
    }
    let mut column = div()
        .flex()
        .flex_col()
        .w_full()
        .relative()
        .top(px(-current))
        .child(content);
    if measured {
        column = column.child(canvas(
            move |bounds, _, _| {
                let (_, viewport_height) = content_metrics.get();
                let height: f32 = bounds.size.height.into();
                content_metrics.set((height, viewport_height));
            },
            |_, _, _, _| {},
        ).absolute().inset_0());
    }
    div()
        .id(gpui::SharedString::from(format!("smooth-scroll-{id}")))
        .relative()
        .w_full()
        .h_full()
        .overflow_hidden()
        .on_scroll_wheel(cx.listener(move |root, event: &gpui::ScrollWheelEvent, _, cx| {
            let delta = event.delta.pixel_delta(px(20.0));
            let dy: f32 = delta.y.into();
            let (content_height, viewport_height) = wheel_metrics.get();
            let max = (content_height - viewport_height).max(0.0);
            let target = root.scroll_targets.entry(id).or_insert(0.0);
            *target = (*target - dy * 2.4).clamp(0.0, max);
            cx.stop_propagation();
            root.start_scroll(id, cx);
            cx.notify();
        }))
        .child(column)
        .child(canvas(
            move |bounds, _, _| {
                let (content_height, _) = viewport_metrics.get();
                let height: f32 = bounds.size.height.into();
                viewport_metrics.set((content_height, height));
            },
            |_, _, _, _| {},
        ).absolute().inset_0())
        .child(rail)
        .into_any_element()
}

pub fn scroll_area(root: &mut Root, id: &'static str, content: Div, cx: &mut Context<Root>) -> gpui::AnyElement {
    scroll_impl(root, id, None, move |_, _, _| content, cx)
}

pub fn virtual_list(
    root: &mut Root,
    id: &'static str,
    count: usize,
    row_height: f32,
    pad_top: f32,
    pad_bottom: f32,
    cx: &mut Context<Root>,
    build: impl FnOnce(&mut Root, std::ops::Range<usize>, &mut Context<Root>) -> Div,
) -> gpui::AnyElement {
    let total = pad_top + count as f32 * row_height + pad_bottom;
    let metrics = root.scroll_metrics.entry(id).or_insert_with(|| Rc::new(Cell::new((0.0, 0.0)))).clone();
    let (_, stored_viewport) = metrics.get();
    let viewport = if stored_viewport > 0.0 { stored_viewport } else { root.viewport_height.max(240.0) };
    let overscan = 5usize;
    let visible_rows = (viewport / row_height).ceil() as usize + overscan * 2 + 2;
    scroll_impl(root, id, Some(total), move |root, offset, cx| {
        let first = ((offset - overscan as f32 * row_height).max(0.0) / row_height).floor() as usize;
        let start = first.min(count);
        let end = (start + visible_rows).min(count);
        let mut list = div().flex().flex_col().w_full();
        let top = pad_top + start as f32 * row_height;
        if top > 0.0 {
            list = list.child(div().h(px(top)).flex_shrink_0());
        }
        if start < end {
            list = list.child(build(root, start..end, cx));
        }
        let bottom = (count - end) as f32 * row_height + pad_bottom;
        if bottom > 0.0 {
            list = list.child(div().h(px(bottom)).flex_shrink_0());
        }
        list
    }, cx)
}

pub fn slide_ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

