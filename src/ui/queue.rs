use gpui::prelude::*;
use gpui::{Context, Div, FontWeight, div, px, svg};

use super::assets;
use super::shell::{self, Root};

pub fn panel(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let tracks = root.backend.player.queue_tracks();
    let current = root.backend.player.state().current_index();
    let count = tracks.len();
    let status = root.backend.player.state().status();

    let mut list = div().id("queue-list").flex().flex_col().flex_1().min_h_0().overflow_y_scroll().py_2();
    if tracks.is_empty() {
        list = list.child(div().flex().items_center().justify_center().py_16().text_sm().text_color(theme.muted).child("The queue is empty"));
    }
    for (index, track) in tracks.iter().enumerate() {
        let playing = current == index as i32;
        let cover = root.cover(track.thumb.as_deref(), 36, cx);
        let title = track.title.clone();
        let artist = track.artist.clone();
        let row_theme = theme;
        let mut row = div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .mx_2()
            .rounded_md()
            .cursor_pointer()
            .when(playing, |el| el.bg(row_theme.surface_hover))
            .hover(move |s| s.bg(row_theme.surface_hover))
            .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                root.backend.player.play_queue_index(index);
                cx.notify();
            }))
            .child(div().w(px(20.0)).flex_shrink_0().flex().justify_center().text_xs().text_color(if playing { row_theme.accent } else { row_theme.muted }).child(
                if playing {
                    svg().path(if status == crate::model::PlaybackStatus::Playing { assets::PAUSE } else { assets::PLAY }).size(px(12.0)).text_color(row_theme.accent).into_any_element()
                } else {
                    (index + 1).to_string().into_any_element()
                },
            ))
            .child(cover)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(if playing { row_theme.accent } else { row_theme.text }).overflow_hidden().text_ellipsis().child(title))
                    .child(div().text_xs().text_color(row_theme.muted).overflow_hidden().text_ellipsis().child(artist)),
            )
            .child(shell::icon_button(assets::CLOSE, 12.0, row_theme.muted, row_theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                root.backend.player.remove_from_queue(index);
                cx.notify();
            })));
        row = row.h(px(52.0));
        list = list.child(row);
    }

    div()
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .w(px(320.0))
        .flex()
        .flex_col()
        .bg(theme.bg)
        .border_l_1()
        .border_color(theme.border)
        .shadow_lg()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(theme.border)
                .child(svg().path(assets::QUEUE).size(px(16.0)).text_color(theme.text))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .gap_1()
                        .child(div().text_sm().font_weight(FontWeight::BOLD).child("Queue"))
                        .child(div().text_xs().text_color(theme.muted).child(format!("{count} tracks"))),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_full()
                        .bg(theme.surface)
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(move |s| s.bg(theme.surface_hover))
                        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                            root.backend.player.clear_queue();
                            cx.notify();
                        }))
                        .child("Clear"),
                )
                .child(shell::icon_button(assets::CLOSE, 14.0, theme.muted, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                    root.queue_open = false;
                    cx.notify();
                }))),
        )
        .child(list)
}

