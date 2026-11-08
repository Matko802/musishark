use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, FontWeight, div, px, svg};

use crate::model::{LikeStatus, Track, VideoId};

use super::assets;
use super::pages;
use super::shell::{self, Load, PageKind, Root, format_duration, heading};

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
    let (title, mut thumb, album, loaded) = match &root.page {
        Some(PageKind::Playlist { title, thumb, album, data, .. }) => (title.clone(), thumb.clone(), *album, data.clone()),
        _ => return div().into_any_element(),
    };
    let mut details = match loaded {
        Load::Ready(details) => details.clone(),
        Load::Loading => return div().flex().flex_col().size_full().child(top_bar(root, &title, cx)).child(shell::loading(theme)).into_any_element(),
        Load::Failed(message) => return div().flex().flex_col().size_full().child(top_bar(root, &title, cx)).child(shell::failed(theme, &message)).into_any_element(),
        Load::Idle => return div().into_any_element(),
    };
    if let Some(url) = details.thumbnails.last() {
        thumb = Some(url.clone());
    }
    let cover = root.cover(thumb.as_deref(), 200, cx);
    let tracks = details.tracks.clone();
    let album_id = details.id.clone();
    let album_title = details.title.clone();
    let playable = tracks.clone();
    let shuffle = tracks.clone();
    let download = tracks.clone();
    let meta = shell::join_detail([
        details.author.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", "),
        details.album_type.clone().unwrap_or_default(),
        details.year.clone().unwrap_or_default(),
        details.track_count.map(|count| format!("{count} songs")).unwrap_or_default(),
        details.duration.clone().unwrap_or_else(|| details.duration_seconds.map(|s| super::shell::format_time(f64::from(s))).unwrap_or_default()),
    ]);
    let description = details.description.clone();
    if details.title.is_empty() {
        details.title = title;
    }

    let mut page = div()
        .id("detail-scroll")
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_scroll()
        .px_8()
        .pt_5()
        .pb_8()
        .child(top_bar(root, if album { "Album" } else { "Playlist" }, cx));

    page = page.child(
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
                    .child(div().text_3xl().font_weight(FontWeight::BOLD).child(details.title.clone()))
                    .child(div().text_sm().text_color(theme.muted).child(meta))
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
                                    .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.play_tracks(playable.clone(), 0, false, cx)))
                                    .child(svg().path(assets::PLAY).size(px(16.0)).text_color(theme.on_accent))
                                    .child("Play"),
                            )
                            .child(shell::icon_button(assets::SHUFFLE, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                                root.play_tracks(shuffle.clone(), 0, true, cx);
                            })))
                            .child(shell::icon_button(assets::DOWNLOAD, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                                root.backend.downloads.queue_tracks(download.clone(), &album_title, &album_id);
                                root.notice("Added to downloads");
                                cx.notify();
                            }))),
                    ),
            ),
    );

    if tracks.is_empty() {
        page = page.child(shell::empty_note(theme, "No tracks here"));
        return page.into_any_element();
    }
    for (index, track) in tracks.iter().enumerate() {
        page = page.child(track_row(root, &tracks, index, track, cx));
    }
    page.into_any_element()
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

    let mut page = div()
        .id("artist-scroll")
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_scroll()
        .px_8()
        .pt_5()
        .pb_8()
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
                                }))),
                        ),
                ),
        );

    if !songs.is_empty() {
        page = page.child(heading(theme, "Top songs", None));
        for (index, track) in songs.iter().enumerate() {
            page = page.child(track_row(root, &songs, index, track, cx));
        }
    }
    for (label, section) in [("Albums", &data.albums), ("Singles & EPs", &data.singles), ("Videos", &data.videos)] {
        if let Some(section) = section {
            if section.results.is_empty() {
                continue;
            }
            page = page.child(div().h(px(20.0))).child(heading(theme, label, None));
            page = page.child(pages::horizontal_strip(root, &section.results, 168.0, cx));
        }
    }
    page.into_any_element()
}

fn category(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let (title, data) = match &root.page {
        Some(PageKind::Category { title, data, .. }) => (title.clone(), data.clone()),
        _ => return div().into_any_element(),
    };
    let mut page = div()
        .id("category-scroll")
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_scroll()
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
                page = page.child(pages::grid(root, &section.items, 6, 150.0, cx));
                page = page.child(div().h(px(24.0)));
            }
        }
        Load::Loading => page = page.child(shell::loading(theme)),
        Load::Failed(message) => page = page.child(shell::failed(theme, message)),
        Load::Idle => {}
    }
    page.into_any_element()
}

fn search(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let (query, data) = match &root.page {
        Some(PageKind::Search { query, data }) => (query.clone(), data.clone()),
        _ => return div().into_any_element(),
    };
    let mut page = div()
        .id("search-scroll")
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_scroll()
        .px_8()
        .pt_5()
        .pb_8()
        .child(top_bar(root, "Search", cx))
        .child(div().text_2xl().font_weight(FontWeight::BOLD).mb_6().child(format!("Results for \"{query}\"")));
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
                page = page.child(pages::grid(root, &items, 6, 150.0, cx));
            } else {
                page = page.child(shell::empty_note(theme, "No results"));
            }
        }
        Load::Loading => page = page.child(shell::loading(theme)),
        Load::Failed(message) => page = page.child(shell::failed(theme, message)),
        Load::Idle => {}
    }
    page.into_any_element()
}

fn history(root: &mut Root, cx: &mut Context<Root>) -> AnyElement {
    let theme = root.theme;
    let mut page = div()
        .id("history-scroll")
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_scroll()
        .px_8()
        .pt_5()
        .pb_8()
        .child(top_bar(root, "History", cx))
        .child(heading(theme, "Recently played", None));
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
            for (index, track) in tracks.iter().enumerate() {
                page = page.child(track_row(root, &tracks, index, track, cx));
            }
        }
        Load::Loading => page = page.child(shell::loading(theme)),
        Load::Failed(message) => page = page.child(shell::failed(theme, message)),
        Load::Idle => {}
    }
    page.into_any_element()
}

pub fn track_row(root: &mut Root, tracks: &[Track], index: usize, track: &Track, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let (playing, paused) = {
        let state = root.backend.player.state();
        let playing = state.is_playing_id(track.video_id.as_str());
        (playing, playing && state.status() != crate::model::PlaybackStatus::Playing)
    };
    let cover = root.cover(track.thumb.as_deref(), 40, cx);
    let all = tracks.to_vec();
    let track = track.clone();
    let like = track.like_status;
    let liked = like == LikeStatus::Like;
    let video = track.video_id.clone();
    let add = track.clone();
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .cursor_pointer()
        .when(playing, |el| el.bg(theme.surface))
        .hover(move |s| s.bg(theme.surface_hover))
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| root.play_tracks(all.clone(), index, false, cx)))
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
            shell::icon_button(if liked { assets::HEART_FILLED } else { assets::HEART }, 16.0, if liked { theme.accent } else { theme.muted }, theme.surface_hover)
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
}

