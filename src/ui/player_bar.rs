use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{Animation, AnimationExt as _, Bounds, Context, Div, FontWeight, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Window, canvas, div, px, relative, svg};

use crate::model::{LikeStatus, PlaybackStatus, RepeatMode, VideoId};

use super::assets;
use super::shell::{self, Load, Root, format_time};

struct Now {
    status: PlaybackStatus,
    position: f64,
    duration: f64,
    title: String,
    artist: String,
    album: String,
    thumb: String,
    video_id: String,
    like: LikeStatus,
    volume: f64,
    muted: bool,
    shuffle: bool,
    repeat: RepeatMode,
}

fn now(root: &Root) -> Now {
    let player = &root.backend.player;
    let state = player.state();
    let track = player.current_track();
    Now {
        status: state.status(),
        position: state.position(),
        duration: state.duration(),
        title: if state.title().is_empty() { track.as_ref().map(|t| t.title.clone()).unwrap_or_default() } else { state.title() },
        artist: if state.artist().is_empty() { track.as_ref().map(|t| t.artist.clone()).unwrap_or_default() } else { state.artist() },
        album: track.as_ref().and_then(|t| t.album.as_ref()).map(|a| a.name.clone()).unwrap_or_default(),
        thumb: if state.thumbnail_url().is_empty() { track.as_ref().and_then(|t| t.thumb.clone()).unwrap_or_default() } else { state.thumbnail_url() },
        video_id: state.video_id(),
        like: LikeStatus::parse(&state.like_status()),
        volume: state.volume(),
        muted: state.muted(),
        shuffle: state.shuffle(),
        repeat: state.repeat(),
    }
}

pub fn bar(root: &mut Root, _window: &mut Window, cx: &mut Context<Root>) -> gpui::AnyElement {
    let theme = root.theme;
    let info = now(root);
    let playing = info.status == PlaybackStatus::Playing;
    let like_icon = if info.like == LikeStatus::Like { assets::THUMB_UP_FILLED } else { assets::THUMB_UP };
    let like_color = if info.like == LikeStatus::Like { theme.accent } else { theme.muted };
    let share_video = info.video_id.clone();
    let dislike_video = info.video_id.clone();
    let disliked = info.like == LikeStatus::Dislike;
    let compact_title_bounds = root.morph_bounds("compact-title");
    let compact_play_bounds = root.morph_bounds("compact-play");

    let left = div()
        .flex()
        .items_center()
        .gap_3()
        .flex_1()
        .min_w_0()
        .child({
            let compact_bounds = root.morph_bounds("compact-cover");
            div()
                .relative()
                .cursor_pointer()
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                    if !root.expanded {
                        root.toggle_expanded(cx);
                    }
                }))
                .child(root.cover(Some(&info.thumb), 48, cx))
                .child(canvas(
                    move |bounds, _, _| compact_bounds.set(bounds),
                    |_, _, _, _| {},
                ).absolute().inset_0())
        })
        .child(
            div()
                .relative()
                .flex()
                .flex_col()
                .gap_1()
                .min_w_0()
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).overflow_hidden().text_ellipsis().child(if info.title.is_empty() { "Nothing playing".to_owned() } else { info.title.clone() }))
                .child(div().text_xs().text_color(theme.muted).overflow_hidden().text_ellipsis().child(info.artist.clone()))
                .child(canvas(
                    move |bounds, _, _| compact_title_bounds.set(bounds),
                    |_, _, _, _| {},
                ).absolute().inset_0()),
        )
        .child(
            shell::icon_button(like_icon, 16.0, like_color, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                let next = if info.like == LikeStatus::Like { LikeStatus::Indifferent } else { LikeStatus::Like };
                if !info.video_id.is_empty() {
                    root.backend.player.set_like_status(VideoId(info.video_id.clone()), next);
                }
                cx.notify();
            })),
        )
        .child(shell::icon_button(assets::THUMB_DOWN, 15.0, if disliked { theme.accent } else { theme.muted }, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
            if !dislike_video.is_empty() {
                let next = if disliked { LikeStatus::Indifferent } else { LikeStatus::Dislike };
                root.backend.player.set_like_status(VideoId(dislike_video.clone()), next);
            }
            cx.notify();
        })))
        .child(shell::icon_button(assets::SHARE, 16.0, theme.muted, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
            root.share_track(&share_video, cx);
        })));

    let fraction = if info.duration > 0.0 { (info.position / info.duration).clamp(0.0, 1.0) } else { 0.0 };
    let progress = progress_bar(
        root,
        fraction,
        "bar-progress",
        cx,
        move |root, value, cx| {
            let duration = root.backend.player.state().duration();
            if duration > 0.0 {
                root.backend.player.seek(value * duration);
            }
            cx.notify();
        },
    );

    let controls = div()
        .flex()
        .items_center()
        .gap_2()
        .child(mode_button(assets::SHUFFLE, info.shuffle, root, cx, |root, cx| {
            root.backend.player.toggle_shuffle();
            cx.notify();
        }))
        .child(shell::icon_button(assets::PREV, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
            root.backend.player.previous();
            cx.notify();
        })))
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .size(px(42.0))
                .bg(theme.text)
                .cursor_pointer()
                .hover(move |s| s.opacity(0.88))
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                    root.backend.player.toggle_play();
                    cx.notify();
                }))
                .child(svg().path(if playing { assets::PAUSE } else { assets::PLAY }).size(px(18.0)).text_color(theme.on_accent))
                .child(canvas(
                    move |bounds, _, _| compact_play_bounds.set(bounds),
                    |_, _, _, _| {},
                ).absolute().inset_0()),
        )
        .child(shell::icon_button(assets::NEXT, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
            root.backend.player.next();
            cx.notify();
        })))
        .child(mode_button(assets::REPEAT, info.repeat != RepeatMode::Off, root, cx, |root, cx| {
            let next = match root.backend.player.repeat_mode() {
                RepeatMode::Off => RepeatMode::All,
                RepeatMode::All => RepeatMode::Track,
                RepeatMode::Track => RepeatMode::Off,
            };
            root.backend.player.set_repeat(next);
            cx.notify();
        }));

    let center = div()
        .flex()
        .flex_col()
        .gap_2()
        .w(px(440.0))
        .flex_shrink_0()
        .child(div().flex().justify_center().child(controls))
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(div().text_xs().text_color(theme.muted).w(px(38.0)).child(format_time(info.position)))
                .child(div().flex_1().child(progress))
                .child(div().text_xs().text_color(theme.muted).w(px(38.0)).child(format_time(info.duration))),
        );

    let volume = progress_bar(
        root,
        info.volume,
        "bar-volume",
        cx,
        move |root, value, cx| {
            root.backend.player.set_volume(value);
            cx.notify();
        },
    );

    let right = div()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .flex_1()
        .child({
            let volume_group = gpui::SharedString::from("volume-zone");
            let hover_group = volume_group.clone();
            div()
                .group(volume_group)
                .flex()
                .items_center()
                .child(
                    shell::icon_button(if info.muted || info.volume <= 0.001 { assets::VOLUME_MUTED } else { assets::VOLUME }, 16.0, theme.muted, theme.surface_hover)
                        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                            let muted = root.backend.player.state().muted();
                            root.backend.player.set_mute(!muted);
                            cx.notify();
                        })),
                )
                .child(
                    div()
                        .overflow_hidden()
                        .h(px(20.0))
                        .w(px(0.0))
                        .min_w_0()
                        .group_hover(hover_group, |mut style| {
                            style.size.width = Some(px(96.0).into());
                            style
                        })
                        .child(div().w(px(96.0)).child(volume)),
                )
        })
        .child(shell::icon_button(assets::QUEUE, 17.0, if root.queue_open { theme.accent } else { theme.text }, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.toggle_queue(cx))))
        .child(shell::icon_button(assets::CHEVRON_UP, 16.0, theme.muted, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
            if !root.expanded {
                root.toggle_expanded(cx);
            }
        })));

    let bar = div()
        .flex()
        .items_center()
        .gap_4()
        .h(px(84.0))
        .px_4()
        .flex_shrink_0()
        .bg(theme.bg)
        .child(left)
        .child(center)
        .child(right);
    bar.into_any_element()
}

fn mode_button(icon: &'static str, active: bool, root: &mut Root, cx: &mut Context<Root>, action: impl Fn(&mut Root, &mut Context<Root>) + 'static) -> Div {
    let theme = root.theme;
    let color = if active { theme.accent } else { theme.muted };
    shell::icon_button(icon, 16.0, color, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| action(root, cx)))
}

pub fn slider(
    root: &mut Root,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    fraction: f64,
    id: &'static str,
    cx: &mut Context<Root>,
    on_set: impl Fn(&mut Root, f64, &mut Context<Root>) + 'static,
) -> Div {
    let theme = root.theme;
    let fraction = fraction.clamp(0.0, 1.0) as f32;
    let handle = bounds.clone();
    let click_bounds = bounds.clone();
    let move_bounds = bounds;
    let on_set: Rc<dyn Fn(&mut Root, f64, &mut Context<Root>)> = Rc::new(on_set);
    let click_set = on_set.clone();
    let move_set = on_set;
    let group = gpui::SharedString::from(format!("slider-{id}"));
    let track_group = group.clone();
    let knob_group = group.clone();
    div()
        .group(group)
        .relative()
        .h(px(20.0))
        .w_full()
        .flex()
        .items_center()
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |root, event: &MouseDownEvent, _, cx| {
                let bounds = click_bounds.get();
                let width: f32 = bounds.size.width.into();
                if width > 0.0 {
                    let offset: f32 = (event.position.x - bounds.origin.x).into();
                    let value = (offset / width).clamp(0.0, 1.0) as f64;
                    click_set(root, value, cx);
                }
            }),
        )
        .on_mouse_move(cx.listener(move |root, event: &MouseMoveEvent, _, cx| {
            if event.pressed_button != Some(MouseButton::Left) {
                return;
            }
            let bounds = move_bounds.get();
            let width: f32 = bounds.size.width.into();
            if width > 0.0 {
                let offset: f32 = (event.position.x - bounds.origin.x).into();
                let value = (offset / width).clamp(0.0, 1.0) as f64;
                move_set(root, value, cx);
            }
        }))
        .child(
            div()
                .relative()
                .h(px(5.0))
                .w_full()
                .bg(theme.border)
                .group_hover(track_group, |mut style| {
                    style.size.height = Some(px(9.0).into());
                    style
                })
                .child(div().absolute().left_0().top_0().bottom_0().bg(theme.text).w(relative(fraction)))
                .child(
                    div()
                        .absolute()
                        .left(relative(fraction))
                        .top(px(-3.5))
                        .ml(px(-6.0))
                        .size(px(12.0))
                        .bg(theme.text)
                        .opacity(0.0)
                        .group_hover(knob_group, |mut style| {
                            style.opacity = Some(1.0);
                            style
                        }),
                )
                .child(canvas(
                    move |bounds, _, _| {
                        handle.set(bounds);
                    },
                    |_, _, _, _| {},
                ).absolute().inset_0()),
        )
}

fn progress_bar(root: &mut Root, fraction: f64, id: &'static str, cx: &mut Context<Root>, on_set: impl Fn(&mut Root, f64, &mut Context<Root>) + 'static) -> Div {
    let bounds = if id == "bar-progress" || id == "expanded-progress" { root.progress_bounds.clone() } else { root.volume_bounds.clone() };
    slider(root, bounds, fraction, id, cx, on_set)
}

fn lyrics_content(root: &mut Root, info: &Now, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    root.ensure_lyrics(&info.video_id, &info.title, &info.artist, info.duration, cx);
    let loaded = match root.lyrics.get(&info.video_id) {
        Some(Load::Ready(result)) => Some(result.clone()),
        _ => None,
    };
    let Some(result) = loaded else {
        let message = match root.lyrics.get(&info.video_id) {
            Some(Load::Ready(_)) => "No lyrics found",
            Some(Load::Failed(message)) => message.as_str(),
            _ => "Loading lyrics…",
        };
        return div().flex().items_center().justify_center().text_sm().text_color(theme.muted).child(message.to_owned());
    };
    let Some(result) = result else {
        return div().flex().items_center().justify_center().text_sm().text_color(theme.muted).child("No lyrics found");
    };
    let position = info.position;
    let mut active = if result.synced { 0 } else { usize::MAX };
    for (index, line) in result.lines.iter().enumerate() {
        if line.start.is_some_and(|start| start <= position) {
            active = index;
        }
    }
    let key = info.video_id.clone();
    let previous = root.lyrics_active.get(&key).copied();
    if previous != Some(active) {
        root.lyrics_active.insert(key, active);
        let target = if active == usize::MAX { 0.0 } else { (active as f32 * 44.0 - 170.0).max(0.0) };
        root.scroll_targets.insert("lyrics", target);
        root.start_scroll("lyrics", cx);
    }
    let mut list = div().flex().flex_col().gap_2().w_full().px_6().py_4();
    for (index, line) in result.lines.iter().enumerate() {
        let is_active = index == active;
        if line.text.trim().is_empty() {
            list = list.child(div().h(px(10.0)));
            continue;
        }
        let mut column = div().flex().flex_col().gap_1().child(
            div()
                .text_lg()
                .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                .text_color(if is_active { theme.accent } else { theme.muted })
                .child(line.text.clone()),
        );
        if let Some(second) = line.romanization.as_ref().or(line.translation.as_ref()) {
            if !second.is_empty() {
                column = column.child(div().text_xs().text_color(theme.muted).child(second.clone()));
            }
        }
        list = list.child(column);
    }
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_3()
        .w(px(760.0))
        .child(div().w_full().h(px(430.0)).child(shell::scroll_area(root, "lyrics", list, cx)))
        .child(div().text_xs().text_color(theme.muted).child(format!("Lyrics from {}", result.source)))
}

pub fn expanded(root: &mut Root, cx: &mut Context<Root>) -> gpui::AnyElement {
    let theme = root.theme;
    let generation = root.expanded_generation;
    let closing = root.expanded_closing;
    let height = root.viewport_height.max(1.0);
    let info = now(root);
    let playing = info.status == PlaybackStatus::Playing;
    let cover = root.cover(Some(&info.thumb), 320, cx);
    let fraction = if info.duration > 0.0 { (info.position / info.duration).clamp(0.0, 1.0) } else { 0.0 };
    let show_lyrics = root.expanded_lyrics;
    let morphing = root.compact_bar_anim || root.expanded_closing;
    let stable = !root.compact_bar_anim && !root.expanded_closing;
    let expanded_title_bounds = root.morph_bounds("expanded-title");
    let expanded_play_bounds = root.morph_bounds("expanded-play");
    let share_video = info.video_id.clone();

    let mut player_tab = div().px_4().py_2().text_sm().font_weight(FontWeight::SEMIBOLD).cursor_pointer();
    player_tab = if !show_lyrics {
        player_tab.bg(theme.surface_hover).text_color(theme.text)
    } else {
        player_tab.text_color(theme.muted).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
            root.expanded_lyrics = false;
            cx.notify();
        }))
    };
    let mut lyrics_tab = div().px_4().py_2().text_sm().font_weight(FontWeight::SEMIBOLD).cursor_pointer();
    lyrics_tab = if show_lyrics {
        lyrics_tab.bg(theme.surface_hover).text_color(theme.text)
    } else {
        lyrics_tab.text_color(theme.muted).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
            root.expanded_lyrics = true;
            cx.notify();
        }))
    };

    let content = if show_lyrics {
        lyrics_content(root, &info, cx)
    } else {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_5()
            .child({
                let expanded_bounds = root.morph_bounds("expanded-cover");
                div()
                    .relative()
                    .size(px(320.0))
                    .flex_shrink_0()
                    .child(if morphing { div().size_full().bg(theme.bg) } else { cover })
                    .child(canvas(
                        move |bounds, _, _| {
                            if stable {
                                expanded_bounds.set(bounds);
                            }
                        },
                        |_, _, _, _| {},
                    ).absolute().inset_0())
            })
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .when(morphing, |el| el.opacity(0.0))
                    .child(div().text_2xl().font_weight(FontWeight::BOLD).child(if info.title.is_empty() { "Nothing playing".to_owned() } else { info.title.clone() }))
                    .child(div().text_sm().text_color(theme.muted).child(shell::join_detail([info.artist.clone(), info.album.clone()])))
                    .child(canvas(
                        move |bounds, _, _| {
                            if stable {
                                expanded_title_bounds.set(bounds);
                            }
                        },
                        |_, _, _, _| {},
                    ).absolute().inset_0()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .w(px(460.0))
                    .child(div().text_xs().text_color(theme.muted).w(px(40.0)).child(format_time(info.position)))
                    .child(div().flex_1().child(progress_bar(root, fraction, "expanded-progress", cx, |root, value, cx| {
                        let duration = root.backend.player.state().duration();
                        if duration > 0.0 {
                            root.backend.player.seek(value * duration);
                        }
                        cx.notify();
                    })))
                    .child(div().text_xs().text_color(theme.muted).w(px(40.0)).child(format_time(info.duration))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(shell::icon_button(assets::SHUFFLE, 20.0, if info.shuffle { theme.accent } else { theme.muted }, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                        root.backend.player.toggle_shuffle();
                        cx.notify();
                    })))
                    .child(shell::icon_button(assets::PREV, 24.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                        root.backend.player.previous();
                        cx.notify();
                    })))
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(56.0))
                            .bg(theme.text)
                            .cursor_pointer()
                            .hover(move |s| s.opacity(0.88))
                            .when(morphing, |el| el.opacity(0.0))
                            .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                                root.backend.player.toggle_play();
                                cx.notify();
                            }))
                            .child(svg().path(if playing { assets::PAUSE } else { assets::PLAY }).size(px(24.0)).text_color(theme.on_accent))
                            .child(canvas(
                                move |bounds, _, _| {
                                    if stable {
                                        expanded_play_bounds.set(bounds);
                                    }
                                },
                                |_, _, _, _| {},
                            ).absolute().inset_0()),
                    )
                    .child(shell::icon_button(assets::NEXT, 24.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                        root.backend.player.next();
                        cx.notify();
                    })))
                    .child(shell::icon_button(assets::REPEAT, 20.0, if info.repeat != RepeatMode::Off { theme.accent } else { theme.muted }, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                        let next = match root.backend.player.repeat_mode() {
                            RepeatMode::Off => RepeatMode::All,
                            RepeatMode::All => RepeatMode::Track,
                            RepeatMode::Track => RepeatMode::Off,
                        };
                        root.backend.player.set_repeat(next);
                        cx.notify();
                    }))),
            )
    };

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .flex()
        .flex_col()
        .items_center()
        .occlude()
        .bg(theme.bg)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .w_full()
                .px_4()
                .h(px(56.0))
                .min_h(px(56.0))
                .flex_shrink_0()
                .child(div().w(px(72.0)))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .p_1()
                        .border_1()
                        .border_color(theme.border)
                        .child(player_tab.child("Player"))
                        .child(lyrics_tab.child("Lyrics")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(shell::icon_button(assets::SHARE, 18.0, theme.text, theme.surface_hover).on_mouse_down(
                            gpui::MouseButton::Left,
                            shell::press(cx, move |root, _, _, cx| root.share_track(&share_video, cx)),
                        ))
                        .child(shell::icon_button(assets::QUEUE, 18.0, if root.queue_open { theme.accent } else { theme.text }, theme.surface_hover).on_mouse_down(
                            gpui::MouseButton::Left,
                            shell::press(cx, |root, _, _, cx| root.toggle_queue(cx)),
                        )),
                ),
        )
        .child(div().id("expanded-scroll").flex_1().min_h_0().overflow_y_scroll().flex().items_center().justify_center().child(content))
        .child(
            div()
                .absolute()
                .right(px(16.0))
                .bottom(px(26.0))
                .child(shell::icon_button(assets::CHEVRON_DOWN, 18.0, theme.text, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.close_expanded(cx)))),
        )
        .with_animation(
            gpui::SharedString::from(format!("expanded-player-{generation}-{closing}")),
            if closing {
                Animation::new(Duration::from_millis(240)).with_easing(shell::slide_ease)
            } else {
                Animation::new(Duration::from_millis(340)).with_easing(shell::slide_ease)
            },
            move |el, delta| {
                let travel = if closing { delta } else { 1.0 - delta };
                el.top(px(travel * height)).bottom(px(-travel * height))
            },
        )
        .into_any_element()
}


pub fn controls_morph(root: &mut Root, cx: &mut Context<Root>) -> gpui::AnyElement {
    let theme = root.theme;
    let info = now(root);
    let closing = root.expanded_closing;
    let generation = root.expanded_generation;
    let width = root.viewport_width.max(1.0);
    let height = root.viewport_height.max(1.0);
    let big_top = height / 2.0 - 232.0;
    let valid = |bounds: Bounds<Pixels>| bounds.size.width > px(0.0) && bounds.size.height > px(0.0);
    let compact_cover = Bounds {
        origin: gpui::point(px(16.0), px(height - 66.0)),
        size: gpui::size(px(48.0), px(48.0)),
    };
    let expanded_cover = Bounds {
        origin: gpui::point(px((width - 320.0) / 2.0), px(big_top)),
        size: gpui::size(px(320.0), px(320.0)),
    };
    let compact_title = Bounds {
        origin: gpui::point(px(76.0), px(height - 60.0)),
        size: gpui::size(px((width * 0.32).min(300.0)), px(34.0)),
    };
    let expanded_title = Bounds {
        origin: gpui::point(px((width - 420.0) / 2.0), px(big_top + 340.0)),
        size: gpui::size(px(420.0), px(52.0)),
    };
    let compact_play = Bounds {
        origin: gpui::point(px(width / 2.0 - 21.0), px(height - 63.0)),
        size: gpui::size(px(42.0), px(42.0)),
    };
    let expanded_play = Bounds {
        origin: gpui::point(px(width / 2.0 - 28.0), px(big_top + 452.0)),
        size: gpui::size(px(56.0), px(56.0)),
    };
    let pick = |kind: &'static str, compact: Bounds<Pixels>, expanded: Bounds<Pixels>| {
        let (from_default, to_default) = if closing { (expanded, compact) } else { (compact, expanded) };
        let from = root.morph_from.get(kind).copied().filter(|bounds| valid(*bounds)).unwrap_or(from_default);
        let to = root.morph_to.get(kind).copied().filter(|bounds| valid(*bounds)).unwrap_or(to_default);
        (from, to)
    };
    let (cover_from, cover_to) = pick("cover", compact_cover, expanded_cover);
    let (title_from, title_to) = pick("title", compact_title, expanded_title);
    let (play_from, play_to) = pick("play", compact_play, expanded_play);
    let duration = if closing { 240 } else { 340 };
    let ease = || Animation::new(Duration::from_millis(duration)).with_easing(shell::slide_ease);

    let image = if info.thumb.is_empty() {
        None
    } else {
        super::cover::cached_cover(&info.thumb, 320).or_else(|| super::cover::cached_cover(&info.thumb, 48))
    };
    let mut cover = div().absolute().overflow_hidden().bg(theme.surface);
    if let Some(image) = image {
        cover = cover.child(gpui::img(image).size_full().object_fit(gpui::ObjectFit::Cover));
    }
    let (fx, fy, fw, fh) = (f32::from(cover_from.origin.x), f32::from(cover_from.origin.y), f32::from(cover_from.size.width), f32::from(cover_from.size.height));
    let (tx, ty, tw, th) = (f32::from(cover_to.origin.x), f32::from(cover_to.origin.y), f32::from(cover_to.size.width), f32::from(cover_to.size.height));
    let cover = cover
        .with_animation(
            gpui::SharedString::from(format!("morph-cover-{generation}-{closing}")),
            ease(),
            move |el, delta| {
                el.left(px(fx + (tx - fx) * delta))
                    .top(px(fy + (ty - fy) * delta))
                    .w(px(fw + (tw - fw) * delta))
                    .h(px(fh + (th - fh) * delta))
            },
        );

    let title = if info.title.is_empty() { "Nothing playing".to_owned() } else { info.title.clone() };
    let artist = shell::join_detail([info.artist.clone(), info.album.clone()]);
    let (name_from, name_to) = if closing { (24.0, 14.0) } else { (14.0, 24.0) };
    let (artist_from, artist_to) = if closing { (14.0, 12.0) } else { (12.0, 14.0) };
    let name = div()
        .font_weight(FontWeight::BOLD)
        .text_color(theme.text)
        .whitespace_nowrap()
        .text_ellipsis()
        .child(title)
        .with_animation(
            gpui::SharedString::from(format!("morph-title-name-{generation}-{closing}")),
            ease(),
            move |el, delta| el.text_size(px(name_from + (name_to - name_from) * delta)),
        );
    let artist = div()
        .text_color(theme.muted)
        .whitespace_nowrap()
        .text_ellipsis()
        .child(artist)
        .with_animation(
            gpui::SharedString::from(format!("morph-title-artist-{generation}-{closing}")),
            ease(),
            move |el, delta| el.text_size(px(artist_from + (artist_to - artist_from) * delta)),
        );
    let (tfx, tfy, tfw, tfh) = (f32::from(title_from.origin.x), f32::from(title_from.origin.y), f32::from(title_from.size.width), f32::from(title_from.size.height));
    let (ttx, tty, ttw, tth) = (f32::from(title_to.origin.x), f32::from(title_to.origin.y), f32::from(title_to.size.width), f32::from(title_to.size.height));
    let title = div()
        .absolute()
        .overflow_hidden()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(name)
        .child(artist)
        .with_animation(
            gpui::SharedString::from(format!("morph-title-box-{generation}-{closing}")),
            ease(),
            move |el, delta| {
                el.left(px(tfx + (ttx - tfx) * delta))
                    .top(px(tfy + (tty - tfy) * delta))
                    .w(px(tfw + (ttw - tfw) * delta))
                    .h(px(tfh + (tth - tfh) * delta))
            },
        );

    let (icon_from, icon_to) = if closing { (24.0, 18.0) } else { (18.0, 24.0) };
    let icon = if info.status == PlaybackStatus::Playing { assets::PAUSE } else { assets::PLAY };
    let icon = svg()
        .path(icon)
        .text_color(theme.on_accent)
        .with_animation(
            gpui::SharedString::from(format!("morph-play-icon-{generation}-{closing}")),
            ease(),
            move |el, delta| el.size(px(icon_from + (icon_to - icon_from) * delta)),
        );
    let (pfx, pfy, pfw, pfh) = (f32::from(play_from.origin.x), f32::from(play_from.origin.y), f32::from(play_from.size.width), f32::from(play_from.size.height));
    let (ptx, pty, ptw, pth) = (f32::from(play_to.origin.x), f32::from(play_to.origin.y), f32::from(play_to.size.width), f32::from(play_to.size.height));
    let play = div()
        .absolute()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.text)
        .child(icon)
        .with_animation(
            gpui::SharedString::from(format!("morph-play-box-{generation}-{closing}")),
            ease(),
            move |el, delta| {
                el.left(px(pfx + (ptx - pfx) * delta))
                    .top(px(pfy + (pty - pfy) * delta))
                    .w(px(pfw + (ptw - pfw) * delta))
                    .h(px(pfh + (pth - pfh) * delta))
            },
        );

    div().absolute().inset_0().child(cover).child(title).child(play).into_any_element()
}
