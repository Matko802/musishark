use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, FontWeight, div, px};

use crate::model::MediaItem;

use super::assets;
use super::shell::{self, LibraryTab, Load, Root, format_duration, heading, join_detail};
use super::theme::Theme;

pub fn home(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    root.load_home(cx);
    let theme = root.theme;
    let (dial, sections) = match &root.home {
        Load::Ready(data) => (data.dial.clone(), data.sections.clone()),
        Load::Loading => return shell::loading(theme).into_any_element().into_any_element(),
        Load::Failed(message) => return shell::failed(theme, message).into_any_element().into_any_element(),
        Load::Idle => (Vec::new(), Vec::new()),
    };

    let mut page = div().id("home-scroll").flex().flex_col().size_full().overflow_y_scroll().px_8().pt_6().pb_8();
    if !dial.is_empty() {
        page = page.child(heading(theme, "Quick picks", Some(assets::PLAY)));
        for row in dial.chunks(3) {
            let mut line = div().flex().flex_row().gap_2().mb_2();
            for item in row {
                line = line.child(dial_tile(root, item, cx));
            }
            for _ in row.len()..3 {
                line = line.child(div().flex_1());
            }
            page = page.child(line);
        }
        page = page.child(div().h(px(20.0)));
    }
    for section in sections {
        if section.items.is_empty() {
            continue;
        }
        page = page.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .mb_4()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child(div().w(px(4.0)).h(px(20.0)).bg(theme.accent))
                .child(section.title.clone()),
        );
        let strip = horizontal_strip(root, &section.items, 168.0, cx);
        page = page.child(strip).child(div().h(px(24.0)));
    }
    page.into_any_element()
}

fn dial_tile(root: &mut Root, item: &MediaItem, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let cover = root.cover(item.thumb.as_deref(), 44, cx);
    let item = item.clone();
    let clicked = item.clone();
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap_3()
        .p_2()
        .bg(theme.surface)
        .cursor_pointer()
        .hover(move |s| s.bg(theme.surface_hover))
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.activate(clicked.clone(), cx)))
        .child(cover)
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .gap_1()
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).overflow_hidden().text_ellipsis().child(item.title.clone()))
                .child({
                    let detail = item.detail();
                    div().text_xs().text_color(theme.muted).overflow_hidden().text_ellipsis().child(detail)
                }),
        )
}

pub fn card(root: &mut Root, item: &MediaItem, size: f32, circular: bool, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let cover = root.cover(item.thumb.as_deref(), size as u32, cx);
    let cover = if circular { cover } else { cover };
    let item = item.clone();
    let clicked = item.clone();
    div()
        .w(px(size))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_2()
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.activate(clicked.clone(), cx)))
        .child(cover)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).overflow_hidden().text_ellipsis().child(item.title.clone()))
                .child({
                    let detail = item.detail();
                    div().text_xs().text_color(theme.muted).overflow_hidden().text_ellipsis().child(detail)
                }),
        )
}

pub fn horizontal_strip(root: &mut Root, items: &[MediaItem], size: f32, cx: &mut Context<Root>) -> AnyElement {
    let key = items.first().map(|item| item.id.clone()).unwrap_or_else(|| "empty".to_owned());
    let mut strip = div().id(gpui::SharedString::from(format!("strip-{key}-{}", size as u32))).flex().flex_row().gap_4().overflow_x_scroll().pb_2();
    for item in items {
        strip = strip.child(card(root, item, size, false, cx));
    }
    strip.into_any_element()
}

pub fn grid(root: &mut Root, items: &[MediaItem], columns: usize, size: f32, cx: &mut Context<Root>) -> Div {
    let mut out = div().flex().flex_col().gap_6();
    for row in items.chunks(columns) {
        let mut line = div().flex().flex_row().gap_6();
        for item in row {
            line = line.child(card(root, item, size, false, cx));
        }
        out = out.child(line);
    }
    out
}

pub fn library(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    root.load_library(root.lib_tab, cx);
    let theme = root.theme;
    let tabs = [(LibraryTab::Playlists, "Playlists"), (LibraryTab::Albums, "Albums"), (LibraryTab::Artists, "Artists"), (LibraryTab::History, "History")];
    let mut switcher = div().flex().items_center().gap_1().p_1().bg(theme.surface);
    for (tab, label) in tabs {
        let selected = root.lib_tab == tab;
        switcher = switcher.child(
            div()
                .px_4()
                .py_2()
                .cursor_pointer()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if selected { theme.on_accent } else { theme.text })
                .when(selected, |el| el.bg(theme.text))
                .when(!selected, move |el| el.hover(move |s| s.bg(theme.surface_hover)))
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                    root.lib_tab = tab;
                    root.load_library(tab, cx);
                    cx.notify();
                }))
                .child(label),
        );
    }

    let mut page = div()
        .id("library-scroll")
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_scroll()
        .px_8()
        .pt_6()
        .pb_8()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .mb_6()
                .child(heading(theme, "Library", None))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .when(root.lib_tab == LibraryTab::History, |el| {
                            el.child(
                                div()
                                    .px_4()
                                    .py_2()
                                    .border_1()
                                    .border_color(theme.border)
                                    .text_sm()
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(theme.surface_hover))
                                    .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.open_history(cx)))
                                    .child("Open history"),
                            )
                        })
                        .child(switcher),
                ),
        );

    match root.lib_tab {
        LibraryTab::Playlists => {
            let load = root.library.playlists.clone();
            page = page.child(media_section(root, &load, 6, 150.0, cx));
        }
        LibraryTab::Albums => {
            let load = root.library.albums.clone();
            page = page.child(media_section(root, &load, 6, 150.0, cx));
        }
        LibraryTab::Artists => {
            let load = root.library.artists.clone();
            page = page.child(artist_section(root, &load, cx));
        }
        LibraryTab::History => page = page.child(history_section(root, cx)),
    }
    page.into_any_element()
}

fn media_section(root: &mut Root, load: &Load<Vec<MediaItem>>, columns: usize, size: f32, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    match load {
        Load::Ready(items) if items.is_empty() => empty(theme, "Nothing here yet"),
        Load::Ready(items) => {
            let items = items.clone();
            grid(root, &items, columns, size, cx)
        }
        Load::Loading | Load::Idle => shell::loading(theme),
        Load::Failed(message) => shell::failed(theme, message),
    }
}

fn artist_section(root: &mut Root, load: &Load<Vec<MediaItem>>, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    match load {
        Load::Ready(items) if items.is_empty() => empty(theme, "No artists yet"),
        Load::Ready(items) => {
            let items = items.clone();
            let mut out = div().flex().flex_col().gap_2();
            for item in &items {
                let cover = root.cover(item.thumb.as_deref(), 48, cx);
                let item = item.clone();
                let clicked = item.clone();
                out = out.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .p_2()
                        .cursor_pointer()
                        .hover(move |s| s.bg(theme.surface_hover))
                        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.activate(clicked.clone(), cx)))
                        .child(cover)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(item.title.clone()))
                                .child(div().text_xs().text_color(theme.muted).child(item.detail())),
                        ),
                );
            }
            out
        }
        Load::Loading | Load::Idle => shell::loading(theme),
        Load::Failed(message) => shell::failed(theme, message),
    }
}

fn history_section(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    match &root.library.history {
        Load::Ready(entries) if entries.is_empty() => empty(theme, "No listening history yet"),
        Load::Ready(entries) => {
            let entries = entries.clone();
            let tracks: Vec<_> = entries.iter().map(|entry| entry.track.clone()).collect();
            let mut out = div().flex().flex_col().gap_1();
            for (index, entry) in entries.iter().enumerate() {
                let cover = root.cover(entry.track.thumb.as_deref(), 44, cx);
                let track = entry.track.clone();
                let played = entry.played.clone();
                let all = tracks.clone();
                out = out.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .p_2()
                        .cursor_pointer()
                        .hover(move |s| s.bg(theme.surface_hover))
                        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.play_tracks(all.clone(), index, false, cx)))
                        .child(cover)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .gap_1()
                                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).overflow_hidden().text_ellipsis().child(track.title.clone()))
                                .child(div().text_xs().text_color(theme.muted).overflow_hidden().text_ellipsis().child(join_detail([track.artist.clone(), played]))),
                        )
                        .child(div().text_xs().text_color(theme.muted).child(format_duration(track.duration_seconds))),
                );
            }
            out
        }
        Load::Loading | Load::Idle => shell::loading(theme),
        Load::Failed(message) => shell::failed(theme, message),
    }
}

pub fn explore(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    root.load_explore(cx);
    let theme = root.theme;
    let data = match &root.explore {
        Load::Ready(data) => data.clone(),
        Load::Loading => return shell::loading(theme).into_any_element().into_any_element(),
        Load::Failed(message) => return shell::failed(theme, message).into_any_element().into_any_element(),
        Load::Idle => return div().into_any_element(),
    };

    let mut page = div().id("explore-scroll").flex().flex_col().size_full().overflow_y_scroll().px_8().pt_6().pb_8();
    page = page.child(heading(theme, "Explore", None));

    let categories: Vec<_> = data.for_you.iter().chain(data.moods.iter()).chain(data.genres.iter()).cloned().collect();
    if !categories.is_empty() {
        page = page.child(div().text_lg().font_weight(FontWeight::BOLD).mb_3().child("Moods & genres"));
        let mut pills = div().flex().flex_row().flex_wrap().gap_2().mb_6();
        for category in categories {
            let title = category.title.clone();
            let params = category.params.clone();
            pills = pills.child(
                div()
                    .px_4()
                    .py_2()
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .cursor_pointer()
                    .text_sm()
                    .hover(move |s| s.bg(theme.surface_hover))
                    .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.open_category(title.clone(), params.clone(), cx)))
                    .child(category.title.clone()),
            );
        }
        page = page.child(pills);
    }

    for (title, items) in [
        ("New releases", data.feed.new_releases.clone()),
        ("New videos", data.feed.new_videos.clone()),
        ("Trending", data.feed.trending.clone()),
    ] {
        if items.is_empty() {
            continue;
        }
        page = page.child(div().text_xl().font_weight(FontWeight::BOLD).mb_4().child(title));
        let strip = horizontal_strip(root, &items, 168.0, cx);
        page = page.child(strip).child(div().h(px(24.0)));
    }

    if let Some(charts) = &data.charts {
        if !charts.daily.is_empty() {
            page = page.child(div().text_xl().font_weight(FontWeight::BOLD).mb_4().child("Charts"));
            let strip = horizontal_strip(root, &charts.daily, 168.0, cx);
            page = page.child(strip);
        }
    }
    page.into_any_element()
}

fn empty(theme: Theme, message: &str) -> Div {
    div().flex().items_center().justify_center().py_16().text_sm().text_color(theme.muted).child(message.to_owned())
}

