use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnimationExt as _, AnyElement, Context, Div, FontWeight, div, px};

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

    let dial_pool = Rc::new(dial.clone());
    let mut page = div().flex().flex_col().w_full().px_8().pt_6().pb_8();
    if !dial.is_empty() {
        page = page.child(heading(theme, "Quick picks", Some(assets::PLAY)));
        for row in dial.chunks(3) {
            let mut line = div().flex().flex_row().gap_2().mb_2();
            for item in row {
                line = line.child(dial_tile(root, item, &dial_pool, cx));
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
        page = page.child(shelf(root, &section.title, &section.items, 168.0, true, cx)).child(div().h(px(12.0)));
    }
    shell::scroll_area(root, "home", page, cx).into_any_element()
}

fn dial_tile(root: &mut Root, item: &MediaItem, pool: &Rc<Vec<MediaItem>>, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let cover = root.cover(item.thumb.as_deref(), 44, cx);
    let item = item.clone();
    let clicked = item.clone();
    let pool = pool.clone();
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
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.activate_in(clicked.clone(), pool.clone(), true, cx)))
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

pub fn card(root: &mut Root, item: &MediaItem, pool: &Rc<Vec<MediaItem>>, size: f32, circular: bool, radio: bool, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let cover = root.cover(item.thumb.as_deref(), size as u32, cx);
    let cover = if circular { cover } else { cover };
    let item = item.clone();
    let clicked = item.clone();
    let pool = pool.clone();
    div()
        .w(px(size))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap_2()
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.activate_in(clicked.clone(), pool.clone(), radio, cx)))
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

pub fn shelf(root: &mut Root, title: &str, items: &[MediaItem], size: f32, radio: bool, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let available = (root.viewport_width - 64.0).max(size + 16.0);
    let per_page = (((available + 16.0) / (size + 16.0)).floor() as usize).max(1);
    let max_start = items.len().saturating_sub(per_page);
    let key = format!("{title}|{}", items.first().map(|item| item.id.as_str()).unwrap_or(""));
    let start = root.shelf_offsets.get(&key).copied().unwrap_or(0).min(max_start);
    if root.shelf_offsets.get(&key).copied() != Some(start) {
        root.shelf_offsets.insert(key.clone(), start);
    }

    let mut header = div()
        .flex()
        .items_center()
        .justify_between()
        .mb_4()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child(div().w(px(4.0)).h(px(20.0)).bg(theme.accent))
                .child(title.to_owned()),
        );
    if items.len() > per_page {
        let left_key = key.clone();
        let right_key = key.clone();
        let left_color = if start > 0 { theme.text } else { theme.border };
        let right_color = if start < max_start { theme.text } else { theme.border };
        header = header.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    shell::icon_button(assets::BACK, 15.0, left_color, theme.surface_hover).on_mouse_down(
                        gpui::MouseButton::Left,
                        shell::press(cx, move |root, _, _, cx| {
                            let current = root.shelf_offsets.get(&left_key).copied().unwrap_or(0);
                            root.shelf_offsets.insert(left_key.clone(), current.saturating_sub(per_page));
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    shell::icon_button(assets::FORWARD, 15.0, right_color, theme.surface_hover).on_mouse_down(
                        gpui::MouseButton::Left,
                        shell::press(cx, move |root, _, _, cx| {
                            let current = root.shelf_offsets.get(&right_key).copied().unwrap_or(0);
                            root.shelf_offsets.insert(right_key.clone(), (current + per_page).min(max_start));
                            cx.notify();
                        }),
                    ),
                ),
        );
    }

    let pool = Rc::new(items.to_vec());
    let mut row = div().flex().flex_row().gap_4().overflow_hidden();
    for item in items.iter().skip(start).take(per_page) {
        row = row.child(card(root, item, &pool, size, false, radio, cx));
    }
    div().flex().flex_col().child(header).child(row)
}

pub fn grid(root: &mut Root, items: &[MediaItem], columns: usize, size: f32, radio: bool, cx: &mut Context<Root>) -> Div {
    let pool = Rc::new(items.to_vec());
    let mut out = div().flex().flex_col().gap_6();
    for row in items.chunks(columns) {
        let mut line = div().flex().flex_row().gap_6();
        for item in row {
            line = line.child(card(root, item, &pool, size, false, radio, cx));
        }
        out = out.child(line);
    }
    out
}

pub fn library(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    root.load_library(root.lib_tab, cx);
    let theme = root.theme;
    let needle = root.search_text.trim().to_lowercase();
    let query_label = root.search_text.trim().to_owned();
    let tabs = [(LibraryTab::Playlists, "Playlists"), (LibraryTab::Albums, "Albums"), (LibraryTab::Artists, "Artists"), (LibraryTab::History, "History")];
    let mut switcher = div().flex().items_center().gap_1().p_1().bg(theme.surface);
    for (tab, label) in tabs {
        let selected = root.lib_tab == tab;
        let previous = root.lib_prev == Some(tab);
        let generation = root.lib_generation;
        let animate = generation > 0 && (selected || previous);
        let mut button = div()
            .px_4()
            .py_2()
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(if selected { theme.on_accent } else { theme.text })
            .when(selected && !animate, |el| el.bg(theme.text))
            .when(!selected && !previous, move |el| el.hover(move |s| s.bg(theme.surface_hover)))
            .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                if root.lib_tab != tab {
                    root.lib_prev = Some(root.lib_tab);
                    root.content_slide = if shell::lib_order(tab) >= shell::lib_order(root.lib_tab) { 1.0 } else { -1.0 };
                    root.lib_generation = root.lib_generation.wrapping_add(1);
                }
                root.search_text.clear();
                root.lib_tab = tab;
                root.load_library(tab, cx);
                cx.notify();
            }))
            .child(label);
        if animate {
            switcher = switcher.child(
                button
                    .with_animation(
                        gpui::SharedString::from(format!("lib-tab-{generation}-{label}")),
                        gpui::Animation::new(std::time::Duration::from_millis(200)).with_easing(shell::spring),
                        move |el, delta| {
                            let p = if selected { delta } else { 1.0 - delta };
                            el.bg(shell::mix(theme.surface, theme.text, p)).text_color(shell::mix(theme.text, theme.on_accent, p))
                        },
                    )
                    .into_any_element(),
            );
        } else {
            switcher = switcher.child(button);
        }
    }

    let mut page = div()
        .flex()
        .flex_col()
        .w_full()
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

    if !needle.is_empty() {
        page = page.child(div().mb_4().text_xs().text_color(theme.muted).child(format!("Searching your library for \"{query_label}\"")));
    }
    match root.lib_tab {
        LibraryTab::Playlists => {
            let load = filter_load(&root.library.playlists, &needle);
            page = page.child(media_section(root, &load, 6, 150.0, cx));
        }
        LibraryTab::Albums => {
            let load = filter_load(&root.library.albums, &needle);
            page = page.child(media_section(root, &load, 6, 150.0, cx));
        }
        LibraryTab::Artists => {
            let load = filter_load(&root.library.artists, &needle);
            page = page.child(artist_section(root, &load, cx));
        }
        LibraryTab::History => page = page.child(history_section(root, cx)),
    }
    shell::scroll_area(root, "library", page, cx).into_any_element()
}

fn item_matches(item: &MediaItem, needle: &str) -> bool {
    item.title.to_lowercase().contains(needle)
        || item.detail().to_lowercase().contains(needle)
        || item.artists.iter().any(|artist| artist.name.to_lowercase().contains(needle))
}

fn filter_load(load: &Load<Vec<MediaItem>>, needle: &str) -> Load<Vec<MediaItem>> {
    if needle.is_empty() {
        return load.clone();
    }
    match load {
        Load::Ready(items) => Load::Ready(items.iter().filter(|item| item_matches(item, needle)).cloned().collect()),
        other => other.clone(),
    }
}

fn media_section(root: &mut Root, load: &Load<Vec<MediaItem>>, columns: usize, size: f32, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    match load {
        Load::Ready(items) if items.is_empty() => empty(theme, "Nothing here yet"),
        Load::Ready(items) => {
            let items = items.clone();
            grid(root, &items, columns, size, false, cx)
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
    let needle = root.search_text.trim().to_lowercase();
    match &root.library.history {
        Load::Ready(entries) if entries.is_empty() => empty(theme, "No listening history yet"),
        Load::Ready(entries) => {
            let entries: Vec<_> = if needle.is_empty() {
                entries.clone()
            } else {
                entries
                    .iter()
                    .filter(|entry| entry.track.title.to_lowercase().contains(&needle) || entry.track.artist.to_lowercase().contains(&needle))
                    .cloned()
                    .collect()
            };
            if entries.is_empty() {
                return empty(theme, "No matches in your history");
            }
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

    let mut page = div().flex().flex_col().w_full().px_8().pt_6().pb_8();
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
        page = page.child(shelf(root, title, &items, 168.0, false, cx)).child(div().h(px(12.0)));
    }

    if let Some(charts) = &data.charts {
        if !charts.daily.is_empty() {
            page = page.child(shelf(root, "Charts", &charts.daily, 168.0, false, cx));
        }
    }
    shell::scroll_area(root, "explore", page, cx).into_any_element()
}

fn empty(theme: Theme, message: &str) -> Div {
    div().flex().items_center().justify_center().py_16().text_sm().text_color(theme.muted).child(message.to_owned())
}

