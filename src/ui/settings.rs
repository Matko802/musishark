use gpui::prelude::*;
use gpui::{Context, Div, FontWeight, div, px, rgba, svg};

use super::assets;
use super::shell::{self, Root};
use super::theme::ThemeMode;

pub fn panel(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let mode = theme.mode;
    let authenticated = root.backend.player.state().authenticated();
    let name = root.backend.player.state().account_name();
    let handle = root.backend.player.state().account_handle();
    let avatar = root.account_cover(40, cx);
    let backdrop = if theme.dark { rgba(0x000000cc) } else { rgba(0x00000040) };

    let theme_card = |option: ThemeMode, cx: &mut Context<Root>| {
        let selected = mode == option;
        let swatch = match option {
            ThemeMode::Amoled => gpui::rgb(0x000000),
            ThemeMode::White => gpui::rgb(0xffffff),
        };
        div()
            .flex_1()
            .flex()
            .items_center()
            .gap_3()
            .p_3()
            .rounded_lg()
            .bg(theme.surface)
            .border_1()
            .border_color(if selected { theme.accent } else { theme.border })
            .cursor_pointer()
            .hover(move |s| s.bg(theme.surface_hover))
            .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                root.set_theme(option, cx);
            }))
            .child(div().size(px(36.0)).rounded_md().bg(swatch).border_1().border_color(theme.border))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_1()
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(option.label()))
                    .child(div().text_xs().text_color(theme.muted).child(match option {
                        ThemeMode::Amoled => "Pure black, made for OLED",
                        ThemeMode::White => "Clean light interface",
                    })),
            )
            .when(selected, |el| el.child(svg().path(assets::CHECK).size(px(16.0)).text_color(theme.accent)))
    };

    let account_row = if authenticated {
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(avatar.rounded_full())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_1()
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(if name.is_empty() { "Signed in".to_owned() } else { name }))
                    .child(div().text_xs().text_color(theme.muted).child(if handle.is_empty() { String::new() } else { handle })),
            )
            .child(
                div()
                    .px_4()
                    .py_2()
                    .rounded_full()
                    .border_1()
                    .border_color(theme.border)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(move |s| s.bg(theme.surface_hover))
                    .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                        let client = root.backend.net.client().clone();
                        let player = root.backend.player.clone();
                        cx.spawn(async move |this, cx| {
                            client.logout().await;
                            let _ = this.update(cx, |root, cx| {
                                let state = player.state();
                                state.set_authenticated(false);
                                state.set_account_name(String::new());
                                state.set_account_handle(String::new());
                                state.set_account_photo_url(String::new());
                                root.notice("Signed out");
                                cx.notify();
                            });
                        })
                        .detach();
                    }))
                    .child("Sign out"),
            )
    } else {
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(avatar.rounded_full())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_1()
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child("Not signed in"))
                    .child(div().text_xs().text_color(theme.muted).child("Account sign-in is not available in this build yet.")),
            )
    };

    div()
        .absolute()
        .inset_0()
        .bg(backdrop)
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
            root.settings_open = false;
            cx.notify();
        }))
        .child(
            div()
                .w(px(440.0))
                .flex()
                .flex_col()
                .gap_5()
                .p_6()
                .rounded_lg()
                .bg(theme.bg)
                .border_1()
                .border_color(theme.border)
                .shadow_lg()
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |_, _, _, cx| cx.stop_propagation()))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_xl().font_weight(FontWeight::BOLD).child("Settings"))
                        .child(shell::icon_button(assets::CLOSE, 16.0, theme.muted, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| {
                            root.settings_open = false;
                            cx.notify();
                        }))),
                )
                .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted).child("APPEARANCE"))
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(theme_card(ThemeMode::Amoled, cx))
                        .child(theme_card(ThemeMode::White, cx)),
                )
                .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted).child("ACCOUNT"))
                .child(account_row)
                .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted).child("ABOUT"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_sm().child("Musishark"))
                        .child(div().text_xs().text_color(theme.muted).child(env!("CARGO_PKG_VERSION"))),
                ),
        )
}
