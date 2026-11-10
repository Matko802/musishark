use gpui::prelude::*;
use std::rc::Rc;

use gpui::{AnyElement, Context, Div, FontWeight, div, px, svg};

use crate::model::{LikeStatus, Track, VideoId};

use super::assets;
use super::pages;
use super::shell::{self, Load, PageKind, Root, format_duration, heading};
use crate::net::search::SearchFilter;

pub fn page(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    match &root.page {
        Some(PageKind::Playlist { .. }) => playlist(root, cx),
        Some(PageKind::Artist { .. }) => artist(root, cx),
        Some(PageKind::Category { .. }) => category(root, cx),
        Some(PageKind::Search { .. }) => search(root, cx),
        Some(PageKind::History { .. }) => history(root, cx),
        None => div().into_any_element(),
    }
}

fn top_bar(root: &mut Root, title: &str, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    div()
        .flex()
        .items_center()
        .gap_3()
        .mb_6()
        .child(shell::icon_button(assets::BACK, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.go_back(cx))))
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(theme.muted).child(title.to_owned()))
}

fn playlist(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let (id, listing_title, mut thumb, album) = match &root.page {
        Some(PageKind::Playlist { id, title, thumb, album, .. }) => (id.clone(), title.clone(), thumb.clone(), *album),
        _ => return div().into_any_element(),
    };
    let state = match &root.page {
        Some(PageKind::Playlist { data: Load::Loading, .. }) => Some(None),
        Some(PageKind::Playlist { data: Load::Failed(message), .. }) => Some(Some(message.clone())),
        Some(PageKind::Playlist { data: Load::Idle, .. }) => return div().into_any_element(),
        _ => None,
    };
    match state {
        Some(None) => return div().flex().flex_col().size_full().child(top_bar(root, &listing_title, cx)).child(shell::loading(theme)).into_any_element(),
        Some(Some(message)) => return div().flex().flex_col().size_full().child(top_bar(root, &listing_title, cx)).child(shell::failed(theme, &message)).into_any_element(),
        None => {}
    }
    let (title, album_type, year, track_count, duration, duration_seconds, author, description, thumbs) = match &root.page {
        Some(PageKind::Playlist { data: Load::Ready(details), .. }) => (
            details.title.clone(),
            details.album_type.clone(),
            details.year.clone(),
            details.track_count,
            details.duration.clone(),
            details.duration_seconds,
            details.author.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", "),
            details.description.clone(),
            details.thumbnails.last().cloned(),
        ),
        _ => return div().into_any_element(),
    };
    if thumbs.is_some() {
        thumb = thumbs;
    }
    let title = if title.is_empty() { listing_title } else { title };
    let cover = root.cover(thumb.as_deref(), 200, cx);
    let shared = match root.track_lists.get(&id) {
        Some(tracks) => tracks.clone(),
        None => {
            let tracks = Rc::new(Vec::new());
            root.track_lists.insert(id.clone(), tracks.clone());
            tracks
        }
    };
    let playable = shared.clone();
    let shuffle = shared.clone();
    let download = shared.clone();
    let needle = root.search_text.trim().to_lowercase();
    let query_label = root.search_text.trim().to_owned();
    let matches: Vec<usize> = if needle.is_empty() {
        (0..shared.len()).collect()
    } else {
        shared
            .iter()
            .enumerate()
            .filter(|(_, track)| track.title.to_lowercase().contains(&needle) || track.artist.to_lowercase().contains(&needle))
            .map(|(index, _)| index)
            .collect()
    };
    let meta = shell::join_detail([
        author,
        album_type.unwrap_or_default(),
        year.unwrap_or_default(),
        track_count.map(|count| format!("{count} songs")).unwrap_or_default(),
        duration.unwrap_or_else(|| duration_seconds.map(|s| super::shell::format_time(f64::from(s))).unwrap_or_default()),
    ]);

    let mut page = div().flex().flex_col().size_full();
    page = page.child(
        div()
            .flex()
            .flex_col()
            .px_8()
            .pt_5()
            .child(top_bar(root, if album { "Album" } else { "Playlist" }, cx))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_6()
                    .mb_8()
                    .child(cover)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .min_w_0()
                    .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted).child(if album { "ALBUM" } else { "PLAYLIST" }))
                    .child(div().text_3xl().font_weight(FontWeight::BOLD).child(title.clone()))
                    .child(div().text_sm().text_color(theme.muted).child(meta))
                    .when(!needle.is_empty(), |el| {
                        el.child(div().text_xs().text_color(theme.muted).child(format!("{} of {} tracks match \"{}\"", matches.len(), shared.len(), query_label)))
                    })
                    .when(!description.is_empty(), |el| {
                        el.child(div().text_xs().text_color(theme.muted).max_w(px(640.0)).overflow_hidden().text_ellipsis().child(description))
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_6()
                                    .py_2()
                                    .bg(theme.text)
                                    .text_color(theme.on_accent)
                                    .text_sm()
                                    .font_weight(FontWeight::BOLD)
                                    .cursor_pointer()
                                    .hover(move |s| s.opacity(0.85))
                                    .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.play_tracks(playable.as_ref().clone(), 0, false, cx)))
                                    .child(svg().path(assets::PLAY).size(px(16.0)).text_color(theme.on_accent))
                                    .child("Play"),
                            )
                            .child(shell::icon_button(assets::SHUFFLE, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                                root.play_tracks(shuffle.as_ref().clone(), 0, true, cx);
                            })))
                            .child({
                                let download_title = title.clone();
                                let download_id = id.clone();
                                shell::icon_button(assets::DOWNLOAD, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                                    root.backend.downloads.queue_tracks(download.as_ref().clone(), &download_title, &download_id);
                                    root.notice("Added to downloads");
                                    cx.notify();
                                }))
                            })
                            .child(shell::icon_button(assets::SHARE, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.share_page(cx)))),
                    ),
            ),
            ),
    );

    if shared.is_empty() {
        page = page.child(shell::empty_note(theme, "No tracks here"));
        return page.into_any_element();
    }
    if matches.is_empty() {
        page = page.child(shell::empty_note(theme, "No matches in this playlist"));
        return page.into_any_element();
    }
    let count = matches.len();
    let rows = shell::virtual_list(root, "playlist", count, 56.0, 0.0, 32.0, cx, move |root, range, cx| {
        let mut rows = div().flex().flex_col().w_full();
        for position in range {
            let Some(&index) = matches.get(position) else { continue };
            let Some(track) = shared.get(index) else { continue };
            rows = rows.child(track_row(root, shared.clone(), index, position, track, cx));
        }
        rows
    });
    page.child(rows).into_any_element()
}

fn artist(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let name = match &root.page {
        Some(PageKind::Artist { name, .. }) => name.clone(),
        _ => return div().into_any_element(),
    };
    let loaded = match &root.page {
        Some(PageKind::Artist { data, .. }) => data.clone(),
        _ => return div().into_any_element(),
    };
    let data = match loaded {
        Load::Ready(data) => data,
        Load::Loading => return div().flex().flex_col().size_full().child(top_bar(root, &name, cx)).child(shell::loading(theme)).into_any_element(),
        Load::Failed(message) => return div().flex().flex_col().size_full().child(top_bar(root, &name, cx)).child(shell::failed(theme, &message)).into_any_element(),
        Load::Idle => return div().into_any_element(),
    };
    let thumb = data.thumbnails.last().cloned();
    let cover = root.cover(thumb.as_deref(), 180, cx);
    let title = if data.name.is_empty() { name } else { data.name.clone() };
    let subtitle = shell::join_detail([data.monthly_listeners.clone().unwrap_or_default(), data.subscribers.clone().unwrap_or_default(), data.views.clone().unwrap_or_default()]);
    let songs = data.songs.as_ref().map(|section| section.results.clone()).unwrap_or_default();
    let playable = songs.clone();
    let radio = data.radio_id.clone();
    let shuffle = songs.clone();

    let mut page = div().flex().flex_col().w_full();
    page = page.child(
        div()
            .flex()
            .flex_col()
            .px_8()
            .pt_5()
            .child(top_bar(root, "Artist", cx))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap_6()
                    .mb_8()
                    .child(cover)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .min_w_0()
                        .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted).child("ARTIST"))
                        .child(div().text_3xl().font_weight(FontWeight::BOLD).child(title))
                        .child(div().text_sm().text_color(theme.muted).child(subtitle))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .px_6()
                                        .py_2()
                                        .bg(theme.text)
                                        .text_color(theme.on_accent)
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD)
                                        .cursor_pointer()
                                        .hover(move |s| s.opacity(0.85))
                                        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.play_tracks(playable.clone(), 0, false, cx)))
                                        .child(svg().path(assets::PLAY).size(px(16.0)).text_color(theme.on_accent))
                                        .child("Play"),
                                )
                                .child(shell::icon_button(assets::SHUFFLE, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                                    root.play_tracks(shuffle.clone(), 0, true, cx);
                                })))
                                .child(shell::icon_button(assets::PLAY, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, _| {
                                    if let Some(id) = radio.clone() {
                                        root.backend.player.start_radio(None, Some(id));
                                    }
                                })))
                                .child(shell::icon_button(assets::SHARE, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.share_page(cx)))),
                        ),
                ),
            ),
    );

    if !songs.is_empty() {
        page = page.child(div().px_8().child(heading(theme, "Top songs", None)));
        let needle = root.search_text.trim().to_lowercase();
        let query_label = root.search_text.trim().to_owned();
        let matches: Vec<usize> = if needle.is_empty() {
            (0..songs.len()).collect()
        } else {
            songs
                .iter()
                .enumerate()
                .filter(|(_, track)| track.title.to_lowercase().contains(&needle) || track.artist.to_lowercase().contains(&needle))
                .map(|(index, _)| index)
                .collect()
        };
        if !needle.is_empty() {
            page = page.child(div().px_8().pb_3().text_xs().text_color(theme.muted).child(format!("{} of {} songs match \"{}\"", matches.len(), songs.len(), query_label)));
        }
        if matches.is_empty() {
            page = page.child(shell::empty_note(theme, "No matching songs"));
        } else {
            let shared = Rc::new(songs.clone());
            for (position, &index) in matches.iter().enumerate() {
                let Some(track) = songs.get(index) else { continue };
                page = page.child(track_row(root, shared.clone(), index, position, track, cx));
            }
        }
    }
    for (label, section) in [("Albums", &data.albums), ("Singles & EPs", &data.singles), ("Videos", &data.videos)] {
        if let Some(section) = section {
            if section.results.is_empty() {
                continue;
            }
            page = page.child(div().px_8().pt_5().child(pages::shelf(root, label, &section.results, 168.0, false, cx)));
        }
    }
    page = page.child(div().h(px(24.0)));
    shell::scroll_area(root, "artist", page, cx).into_any_element()
}

fn category(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let (title, data) = match &root.page {
        Some(PageKind::Category { title, data, .. }) => (title.clone(), data.clone()),
        _ => return div().into_any_element(),
    };
    let mut page = div()
        .flex()
        .flex_col()
        .w_full()
        .px_8()
        .pt_5()
        .pb_8()
        .child(top_bar(root, &title, cx));
    match &data {
        Load::Ready(sections) => {
            for section in sections {
                if section.items.is_empty() {
                    continue;
                }
                page = page.child(heading(theme, &section.title, None));
                page = page.child(pages::grid(root, &section.items, 6, 150.0, false, cx));
                page = page.child(div().h(px(24.0)));
            }
        }
        Load::Loading => page = page.child(shell::loading(theme)),
        Load::Failed(message) => page = page.child(shell::failed(theme, message)),
        Load::Idle => {}
    }
    shell::scroll_area(root, "category", page, cx).into_any_element()
}

fn search(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let (query, data) = match &root.page {
        Some(PageKind::Search { query, data }) => (query.clone(), data.clone()),
        _ => return div().into_any_element(),
    };
    let mut page = div()
        .flex()
        .flex_col()
        .w_full()
        .px_8()
        .pt_5()
        .pb_8()
        .child(top_bar(root, "Search", cx))
        .child(div().text_2xl().font_weight(FontWeight::BOLD).mb_4().child(format!("Results for \"{query}\"")))
        .child(search_filters(root, cx));
    match &data {
        Load::Ready(results) => {
            if let Some(top) = &results.top_result {
                let top = top.clone();
                let clicked = top.clone();
                let cover = root.cover(top.thumb.as_deref(), 160, cx);
                let detail = top.detail();
                page = page.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_5()
                        .p_4()
                        .mb_6()
                        .bg(theme.surface)
                        .cursor_pointer()
                        .hover(move |s| s.bg(theme.surface_hover))
                        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.activate(clicked.clone(), cx)))
                        .child(cover)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_2()
                                .child(div().text_xs().text_color(theme.muted).child("Top result"))
                                .child(div().text_2xl().font_weight(FontWeight::BOLD).child(top.title.clone()))
                                .child(div().text_sm().text_color(theme.muted).child(detail)),
                        ),
                );
            }
            if !results.items.is_empty() {
                page = page.child(heading(theme, "Songs & more", None));
                let items = results.items.clone();
                page = page.child(pages::grid(root, &items, 6, 150.0, false, cx));
            } else {
                page = page.child(shell::empty_note(theme, "No results"));
            }
        }
        Load::Loading => page = page.child(shell::loading(theme)),
        Load::Failed(message) => page = page.child(shell::failed(theme, message)),
        Load::Idle => {}
    }
    shell::scroll_area(root, "search", page, cx).into_any_element()
}

fn search_filters(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let current = root.search_filter;
    let filters: [(&str, Option<SearchFilter>); 6] = [
        ("All", None),
        ("Songs", Some(SearchFilter::Songs)),
        ("Videos", Some(SearchFilter::Videos)),
        ("Albums", Some(SearchFilter::Albums)),
        ("Artists", Some(SearchFilter::Artists)),
        ("Playlists", Some(SearchFilter::Playlists)),
    ];
    let mut row = div().flex().flex_row().flex_wrap().gap_2().mb_5();
    for (label, filter) in filters {
        let selected = current == filter;
        row = row.child(
            div()
                .px_3()
                .py_2()
                .border_1()
                .border_color(if selected { theme.text } else { theme.border })
                .bg(if selected { theme.text } else { theme.elevated })
                .text_color(if selected { theme.on_accent } else { theme.muted })
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .cursor_pointer()
                .when(!selected, move |el| el.hover(move |s| s.bg(theme.surface_hover)))
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.set_search_filter(filter, cx)))
                .child(label),
        );
    }
    row
}

fn history(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let mut page = div().flex().flex_col().w_full();
    page = page.child(
        div()
            .flex()
            .flex_col()
            .px_8()
            .pt_5()
            .child(top_bar(root, "History", cx))
            .child(heading(theme, "Recently played", None)),
    );
    let loaded = match &root.page {
        Some(PageKind::History { data }) => data.clone(),
        _ => Load::Idle,
    };
    match &loaded {
        Load::Ready(entries) => {
            let entries = entries.clone();
            let tracks: Vec<Track> = entries.iter().map(|entry| entry.track.clone()).collect();
            if tracks.is_empty() {
                page = page.child(shell::empty_note(theme, "Nothing played yet"));
            }
            let needle = root.search_text.trim().to_lowercase();
            let query_label = root.search_text.trim().to_owned();
            let matches: Vec<usize> = if needle.is_empty() {
                (0..tracks.len()).collect()
            } else {
                tracks
                    .iter()
                    .enumerate()
                    .filter(|(_, track)| track.title.to_lowercase().contains(&needle) || track.artist.to_lowercase().contains(&needle))
                    .map(|(index, _)| index)
                    .collect()
            };
            if !needle.is_empty() {
                page = page.child(div().px_8().pb_3().text_xs().text_color(theme.muted).child(format!("{} of {} tracks match \"{}\"", matches.len(), tracks.len(), query_label)));
            }
            if matches.is_empty() {
                page = page.child(shell::empty_note(theme, "No matches in your history"));
            } else {
                let shared = Rc::new(tracks.clone());
                for (position, &index) in matches.iter().enumerate() {
                    let Some(track) = tracks.get(index) else { continue };
                    page = page.child(track_row(root, shared.clone(), index, position, track, cx));
                }
            }
            page = page.child(div().h(px(24.0)));
        }
        Load::Loading => page = page.child(shell::loading(theme)),
        Load::Failed(message) => page = page.child(shell::failed(theme, message)),
        Load::Idle => {}
    }
    shell::scroll_area(root, "history", page, cx).into_any_element()
}

pub fn track_row(root: &mut Root, tracks: Rc<Vec<Track>>, index: usize, stripe: usize, track: &Track, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let (playing, paused) = {
        let state = root.backend.player.state();
        let playing = state.is_playing_id(track.video_id.as_str());
        (playing, playing && state.status() != crate::model::PlaybackStatus::Playing)
    };
    let cover = root.cover(track.thumb.as_deref(), 40, cx);
    let all = tracks;
    let track = track.clone();
    let like = track.like_status;
    let liked = like == LikeStatus::Like;
    let video = track.video_id.clone();
    let share_id = track.video_id.0.clone();
    let add = track.clone();
    let row_bg = if stripe % 2 == 0 { theme.surface } else { theme.surface_hover };
    div()
        .flex()
        .items_center()
        .gap_3()
        .w_full()
        .h(px(56.0))
        .px_8()
        .cursor_pointer()
        .bg(row_bg)
        .when(playing, |el| el.bg(theme.elevated))
        .hover(move |s| s.bg(theme.border))
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.play_tracks(all.as_ref().clone(), index, false, cx)))
        .child(div().w(px(24.0)).flex_shrink_0().flex().justify_center().text_xs().text_color(if playing { theme.accent } else { theme.muted }).child(match playing {
            true => svg().path(if paused { assets::PLAY } else { assets::PAUSE }).size(px(13.0)).text_color(theme.accent).into_any_element(),
            false => div().child((index + 1).to_string()).into_any_element(),
        }))
        .child(cover)
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(if playing { theme.accent } else { theme.text }).overflow_hidden().text_ellipsis().child(track.title.clone()))
                .child(div().text_xs().text_color(theme.muted).overflow_hidden().text_ellipsis().child(track.artist.clone())),
        )
        .child(div().text_xs().text_color(theme.muted).child(format_duration(track.duration_seconds)))
        .child(
            shell::icon_button(if liked { assets::THUMB_UP_FILLED } else { assets::THUMB_UP }, 16.0, if liked { theme.accent } else { theme.muted }, theme.surface_hover)
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                    let next = if liked { LikeStatus::Indifferent } else { LikeStatus::Like };
                    root.backend.player.set_like_status(VideoId(video.0.clone()), next);
                    cx.notify();
                })),
        )
        .child(shell::icon_button(assets::PLUS, 16.0, theme.muted, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
            root.backend.player.add_to_queue(vec![add.clone()], false);
            root.notice("Added to queue");
            cx.notify();
        })))
        .child(shell::icon_button(assets::SHARE, 15.0, theme.muted, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
            root.share_track(&share_id, cx);
        })))
}

