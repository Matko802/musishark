use std::collections::BTreeSet;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{Animation, AnimationExt as _, Context, Div, FontWeight, div, px, rgba, rgb, svg};
use serde_json::Value;

use crate::bootstrap;
use crate::downloads::naming;
use crate::lyrics::prefs as lyrics_prefs;
use crate::scrobbler::SERVICES;

use super::assets;
use super::shell::{self, Root};
use super::theme::{Theme, ThemeMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsPage {
    General,
    Appearance,
    Lyrics,
    Services,
    Advanced,
}

const PAGES: [(SettingsPage, &str, &str); 5] = [
    (SettingsPage::General, "General", assets::SETTINGS),
    (SettingsPage::Appearance, "Appearance", assets::SUN),
    (SettingsPage::Lyrics, "Lyrics", assets::LYRICS),
    (SettingsPage::Services, "Services", assets::EXTERNAL),
    (SettingsPage::Advanced, "Advanced", assets::REFRESH),
];

const HISTORY_MODES: [(&str, &str); 3] = [("immediate", "Immediately"), ("after_30s", "After 30 seconds"), ("never", "Never")];
const FORMATS: [(&str, &str); 5] = [("opus", "Opus"), ("mp3", "MP3"), ("m4a", "M4A"), ("flac", "FLAC"), ("ogg", "OGG")];
const STRUCTURES: [(&str, &str); 3] = [("artist_album", "Artist / Album / Song"), ("artist", "Artist / Song"), ("flat", "No folders")];
const MATCH_MODES: [(&str, &str); 2] = [("quality", "Quality-aware"), ("strict", "Strict")];
const SECOND_LINES: [(&str, &str); 5] = [("off", "Off"), ("auto", "Auto"), ("romanization", "Romanization"), ("translation", "Translation"), ("background", "Background")];
const EFFECT_LEVELS: [(&str, &str); 3] = [("off", "Off"), ("subtle", "Subtle"), ("full", "Full")];
const DISCORD_STATUS: [(&str, &str); 3] = [("app_name", "App Name"), ("artist", "Artist"), ("song_title", "Song Title")];

type ApplyStr = fn(&mut Root, &str, &mut Context<Root>);
type ApplyBool = fn(&mut Root, bool, &mut Context<Root>);

pub fn panel(root: &mut Root, cx: &mut Context<Root>) -> gpui::AnyElement {
    let theme = root.theme;
    let backdrop = if theme.dark { rgba(0x000000e6) } else { rgba(0x0000005c) };
    let generation = root.settings_generation;
    let closing = root.settings_closing;

    let body = div()
        .w(px(920.0))
        .h(px(640.0))
        .max_w(gpui::relative(0.94))
        .max_h(gpui::relative(0.92))
        .flex()
        .flex_col()
        .bg(theme.bg)
        .border_1()
        .border_color(theme.border)
        .shadow_lg()
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |_, _, _, _| {}))
        .child(header(root, cx))
        .child(
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(sidebar(root, cx))
                .child(content(root, cx)),
        );

    let body = if closing {
        body.with_animation(
            gpui::SharedString::from(format!("settings-body-close-{generation}")),
            Animation::new(Duration::from_millis(180)).with_easing(gpui::ease_in_out),
            |el, delta| el.w(px(920.0 * (1.0 - 0.05 * delta))).h(px(640.0 * (1.0 - 0.05 * delta))),
        )
    } else {
        body.with_animation(
            gpui::SharedString::from(format!("settings-body-open-{generation}")),
            Animation::new(Duration::from_millis(340)).with_easing(shell::spring),
            |el, delta| {
                let scale = 0.94 + 0.06 * shell::spring(delta).clamp(0.0, 1.0);
                el.w(px(920.0 * scale)).h(px(640.0 * scale))
            },
        )
    };

    let overlay = div()
        .absolute()
        .inset_0()
        .bg(backdrop)
        .flex()
        .items_center()
        .justify_center()
        .occlude()
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.close_settings(cx)))
        .child(body);

    if closing {
        overlay
            .with_animation(
                gpui::SharedString::from(format!("settings-close-{generation}")),
                Animation::new(Duration::from_millis(180)).with_easing(gpui::ease_in_out),
                |el, delta| el.opacity(1.0 - delta),
            )
            .into_any_element()
    } else {
        overlay
            .with_animation(
                gpui::SharedString::from(format!("settings-open-{generation}")),
                Animation::new(Duration::from_millis(340)).with_easing(shell::spring),
                |el, delta| el.opacity(shell::spring(delta).clamp(0.0, 1.0)),
            )
            .into_any_element()
    }
}

fn header(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(62.0))
        .px_6()
        .flex_shrink_0()
        .border_b_1()
        .border_color(theme.border)
        .child(div().text_xl().font_weight(FontWeight::BOLD).child("Settings"))
        .child(
            shell::icon_button(assets::CLOSE, 16.0, theme.muted, theme.surface_hover).on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| root.close_settings(cx))),
        )
}

fn sidebar(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let current = root.settings_page;
    let mut list = div().flex().flex_col().gap_1().p_3();
    for (page, label, icon) in PAGES {
        let selected = current == page;
        list = list.child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .px_3()
                .py_2()
                .cursor_pointer()
                .text_sm()
                .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::NORMAL })
                .text_color(if selected { theme.text } else { theme.muted })
                .when(selected, |el| el.bg(theme.surface))
                .when(!selected, move |el| el.hover(move |s| s.bg(theme.surface_hover)))
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                    root.settings_page = page;
                    cx.notify();
                }))
                .child(svg().path(icon).size(px(15.0)).text_color(if selected { theme.accent } else { theme.muted }))
                .child(label),
        );
    }
    div().w(px(212.0)).flex_shrink_0().border_r_1().border_color(theme.border).bg(theme.bg).child(list)
}

fn content(root: &mut Root, cx: &mut Context<Root>) -> gpui::AnyElement {
    let theme = root.theme;
    let page = match root.settings_page {
        SettingsPage::General => general(root, cx),
        SettingsPage::Appearance => appearance(root, cx),
        SettingsPage::Lyrics => lyrics(root, cx),
        SettingsPage::Services => services(root, cx),
        SettingsPage::Advanced => advanced(root, cx),
    };
    let generation = root.settings_generation;
    let page_index = root.settings_page as u8;
    div()
        .id("settings-scroll")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px_8()
        .py_6()
        .bg(theme.bg)
        .child(page)
        .with_animation(
            gpui::SharedString::from(format!("settings-page-{generation}-{page_index}")),
            Animation::new(Duration::from_millis(280)).with_easing(shell::spring),
            |el, delta| el.opacity(shell::spring(delta).clamp(0.0, 1.0)).relative().top(px((1.0 - shell::spring(delta).clamp(0.0, 1.0)) * 8.0)),
        )
        .into_any_element()
}

fn page_header(_theme: Theme, title: &str, _description: &str) -> Div {
    div().flex().flex_col().mb_6().child(div().text_2xl().font_weight(FontWeight::BOLD).child(title.to_owned()))
}

fn group(theme: Theme, title: &str, description: Option<&str>, rows: Vec<Div>) -> Div {
    let mut card = div().flex().flex_col().bg(theme.surface).border_1().border_color(theme.border);
    for (index, row) in rows.into_iter().enumerate() {
        let row = if index > 0 { row.border_t_1().border_color(theme.border) } else { row };
        card = card.child(row);
    }
    let _ = description;
    div()
        .flex()
        .flex_col()
        .gap_2()
        .mb_7()
        .child(div().text_xs().font_weight(FontWeight::BOLD).text_color(theme.muted).child(title.to_owned()))
        .child(card)
}

fn row_shell(theme: Theme) -> Div {
    div().flex().items_center().justify_between().gap_6().px_4().py_3().hover(move |s| s.bg(theme.surface_hover))
}

fn labels(theme: Theme, title: &str, _subtitle: Option<&str>) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).text_color(theme.text).child(title.to_owned()))
}

fn switch_indicator(theme: Theme, on: bool, id: &str) -> Div {
    let knob = div().size(px(18.0)).bg(if on { theme.text } else { theme.muted }).with_animation(
        gpui::SharedString::from(format!("switch-{id}-{on}")),
        Animation::new(Duration::from_millis(320)).with_easing(shell::spring),
        move |el, delta| {
            let travel = if on { delta } else { 1.0 - delta };
            el.ml(px(travel.clamp(0.0, 1.0) * 18.0))
        },
    );
    div()
        .w(px(42.0))
        .h(px(24.0))
        .flex_shrink_0()
        .p(px(3.0))
        .flex()
        .items_center()
        .bg(if on { theme.accent } else { theme.border })
        .child(knob)
}

fn switch_row(root: &mut Root, cx: &mut Context<Root>, key: &'static str, default: bool, title: &'static str, subtitle: &'static str, apply: ApplyBool) -> Div {
    let theme = root.theme;
    let on = pref_bool(&root.backend.paths, key, default);
    row_shell(theme)
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
            let next = !pref_bool(&root.backend.paths, key, default);
            save_bool(root, key, next);
            apply(root, next, cx);
            cx.notify();
        }))
        .child(labels(theme, title, Some(subtitle)))
        .child(switch_indicator(theme, on, key))
}

fn choice_row(
    root: &mut Root,
    cx: &mut Context<Root>,
    key: &'static str,
    default: &'static str,
    title: &'static str,
    subtitle: &'static str,
    options: &'static [(&'static str, &'static str)],
    apply: ApplyStr,
) -> Div {
    let theme = root.theme;
    let current = pref_str(&root.backend.paths, key, default);
    let mut chips = div().flex().flex_row().flex_wrap().gap_2();
    for (value, label) in options {
        let selected = current == *value;
        chips = chips.child(
            div()
                .px_3()
                .py_2()
                .border_1()
                .border_color(if selected { theme.text } else { theme.border })
                .bg(if selected { theme.text } else { theme.elevated })
                .text_color(if selected { theme.on_accent } else { theme.muted })
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .when(!selected, move |el| el.hover(move |s| s.bg(theme.surface_hover)))
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
                    save_str(root, key, value);
                    apply(root, value, cx);
                    cx.notify();
                }))
                .child(*label),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .py_3()
        .child(labels(theme, title, Some(subtitle)))
        .child(chips)
}

fn value_row(theme: Theme, title: &str, _subtitle: &str, value: &str) -> Div {
    row_shell(theme)
        .child(labels(theme, title, None))
        .child(div().text_sm().text_color(theme.muted).child(value.to_owned()))
}

fn themed_action_row(theme: Theme, title: &str, subtitle: &str, label: &str, accent: bool, cx: &mut Context<Root>, action: impl Fn(&mut Root, &mut Context<Root>) + 'static) -> Div {
    let button = div()
        .px_4()
        .py_2()
        .border_1()
        .border_color(if accent { theme.text } else { theme.border })
        .bg(if accent { theme.text } else { theme.elevated })
        .text_color(if accent { theme.on_accent } else { theme.text })
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(move |s| s.opacity(0.85))
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| action(root, cx)))
        .child(label.to_owned());
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_6()
        .px_4()
        .py_3()
        .child(labels(theme, title, Some(subtitle)))
        .child(button)
}

fn general(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    page_header(theme, "General", "Account, playback and downloads.")
        .child(account_group(root, cx))
        .child(playback_group(root, cx))
        .child(downloads_group(root, cx))
        .child(updates_group(root, cx))
}

fn account_group(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let state = root.backend.player.state();
    let authenticated = state.authenticated();
    let name = if state.account_name().is_empty() { "Signed in".to_owned() } else { state.account_name() };
    let handle = state.account_handle();
    let photo = state.account_photo_url();

    let mut rows = Vec::new();
    let avatar = root.cover(Some(&photo), 40, cx);
    let subtitle = if handle.is_empty() { "YouTube Music account".to_owned() } else { handle };
    let mut account_row = div()
        .flex()
        .items_center()
        .gap_3()
        .px_4()
        .py_3()
        .child(avatar)
        .child(labels(theme, &name, Some(&subtitle)));
    if authenticated {
        account_row = account_row.child(
            div()
                .px_4()
                .py_2()
                .border_1()
                .border_color(theme.border)
                .bg(theme.elevated)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(move |s| s.bg(theme.surface_hover))
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, |root, _, _, cx| sign_out(root, cx)))
                .child("Sign out"),
        );
    }
    rows.push(account_row);
    if !authenticated {
        rows.push(value_row(theme, "Sign in", "Account sign-in is not available in this build yet.", "Unavailable"));
    }
    group(theme, "Account", None, rows)
}

fn playback_group(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    group(
        theme,
        "Playback",
        None,
        vec![choice_row(
            root,
            cx,
            "history_mode",
            "immediate",
            "Record plays to history",
            "When a song counts as played for YouTube Music and this device.",
            &HISTORY_MODES,
            |root, value, _| root.backend.player.set_history_mode(value),
        )],
    )
}

fn downloads_group(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let folder = root.backend.paths.music_dir().display().to_string();
    group(
        theme,
        "Downloads",
        Some("File naming applies to new downloads."),
        vec![
            choice_row(root, cx, "download_format", naming::DEFAULT_FORMAT, "Audio format", "Container used for downloaded songs.", &FORMATS, |_, _, _| {}),
            choice_row(
                root,
                cx,
                "download_folder_structure",
                naming::DEFAULT_FOLDER_STRUCTURE,
                "Folder structure",
                "How downloaded songs are arranged inside the music folder.",
                &STRUCTURES,
                |_, _, _| {},
            ),
            switch_row(root, cx, "use_songs_subdir", false, "Use Songs subfolder", "Place downloads inside a Songs/ folder.", |_, _, _| {}),
            value_row(theme, "Download folder", "Songs are saved here.", &folder),
        ],
    )
}

fn updates_group(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    group(
        theme,
        "Updates",
        None,
        vec![
            switch_row(root, cx, "show_release_notes", true, "Release notes after updates", "Show what's new once for each version.", |_, _, _| {}),
            switch_row(root, cx, "show_donation_prompt", true, "Donation prompts", "Show the support banner with release notes.", |_, _, _| {}),
        ],
    )
}

fn appearance(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let cards = div()
        .flex()
        .flex_row()
        .gap_4()
        .child(theme_card(root, cx, ThemeMode::Amoled))
        .child(theme_card(root, cx, ThemeMode::White));
    page_header(theme, "Appearance", "")
        .child(group(theme, "Theme", None, vec![div().p_4().child(cards)]))
}

fn theme_card(root: &mut Root, cx: &mut Context<Root>, option: ThemeMode) -> Div {
    let theme = root.theme;
    let selected = theme.mode == option;
    let swatch = match option {
        ThemeMode::Amoled => rgb(0x000000),
        ThemeMode::White => rgb(0xffffff),
    };
    div()
        .flex_1()
        .flex()
        .items_center()
        .gap_4()
        .p_4()
        .bg(theme.elevated)
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .cursor_pointer()
        .hover(move |s| s.bg(theme.surface_hover))
        .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| {
            root.set_theme(option, cx);
        }))
        .child(div().w(px(48.0)).h(px(48.0)).flex_shrink_0().bg(swatch).border_1().border_color(theme.border))
        .child(div().flex().flex_col().flex_1().min_w_0().child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(option.label())))
        .when(selected, |el| el.child(svg().path(assets::CHECK).size(px(16.0)).text_color(theme.accent)))
}

fn lyrics(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let prefs = root.backend.lyrics.prefs();
    let order = prefs.full_provider_order();
    let disabled = prefs.disabled_providers();

    let mut provider_rows = Vec::new();
    let count = order.len();
    for (index, name) in order.iter().enumerate() {
        let enabled = !disabled.contains(name);
        let provider = name.clone();
        let up_name = provider.clone();
        let down_name = provider.clone();
        let mut controls = div().flex().items_center().gap_1();
        controls = controls.child(
            shell::icon_button(assets::CHEVRON_UP, 14.0, if index == 0 { theme.border } else { theme.muted }, theme.surface_hover).on_mouse_down(
                gpui::MouseButton::Left,
                shell::press(cx, move |root, _, _, cx| move_provider(root, &up_name, -1, cx)),
            ),
        );
        controls = controls.child(
            shell::icon_button(assets::CHEVRON_DOWN, 14.0, if index + 1 == count { theme.border } else { theme.muted }, theme.surface_hover)
                .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| move_provider(root, &down_name, 1, cx))),
        );
        let toggle_name = provider;
        let row = row_shell(theme)
            .cursor_pointer()
            .on_mouse_down(gpui::MouseButton::Left, shell::press(cx, move |root, _, _, cx| toggle_provider(root, &toggle_name, cx)))
            .child(labels(theme, &format!("{}. {name}", index + 1), Some(provider_blurb(name))))
            .child(controls)
            .child(switch_indicator(theme, enabled, name));
        provider_rows.push(if enabled { row } else { row.opacity(0.55) });
    }
    page_header(theme, "Lyrics", "Where lyrics come from and how they are shown.")
        .child(group(
            theme,
            "Search Queue",
            Some("Tried from the top down. Switch one off to skip it."),
            provider_rows,
        ))
        .child(group(
            theme,
            "Matching",
            Some("Quality-aware keeps looking for synced lyrics before settling for plain text. Strict takes the first hit."),
            vec![choice_row(
                root,
                cx,
                "lyrics_match_mode",
                lyrics_prefs::MATCH_QUALITY,
                "When to stop searching",
                "Quality-aware prefers timed lyrics; strict takes the first result.",
                &MATCH_MODES,
                |root, value, _| root.backend.lyrics.prefs().set_match_mode(value),
            )],
        ))
        .child(group(
            theme,
            "Display",
            Some("Second lines draw a romanization, a translation or the background vocal under each lyric line."),
            vec![
                choice_row(
                    root,
                    cx,
                    "lyrics_second_line",
                    lyrics_prefs::SECOND_LINE_DEFAULT,
                    "Second line",
                    "What appears under the lyric being sung.",
                    &SECOND_LINES,
                    |root, value, _| root.backend.lyrics.prefs().set_second_line_mode(value),
                ),
                choice_row(
                    root,
                    cx,
                    "lyrics_effects",
                    lyrics_prefs::EFFECTS_DEFAULT,
                    "Effects",
                    "Word fades, active-line growth and distance blur.",
                    &EFFECT_LEVELS,
                    |root, value, _| root.backend.lyrics.prefs().set_effects_level(value),
                ),
                switch_row(
                    root,
                    cx,
                    "lyrics_line_sweep",
                    true,
                    "Emulate word timing",
                    "Move the highlight across lines that only have line timing.",
                    |root, value, _| root.backend.lyrics.prefs().set_line_sweep(value),
                ),
            ],
        ))
        .child(group(
            theme,
            "Cache",
            Some("Queue changes only apply to tracks that are not cached yet."),
            vec![themed_action_row(
                theme,
                "Clear cached lyrics",
                "Remove every cached lyrics result from this device.",
                "Clear",
                false,
                cx,
                |root, cx| {
                    let removed = root.backend.lyrics.cache().clear_all();
                    root.notice(if removed > 0 { format!("Cleared {removed} cached track(s)") } else { "No cached lyrics to clear".to_owned() });
                    cx.notify();
                },
            )],
        ))
}

fn provider_blurb(name: &str) -> &'static str {
    match name {
        "Apple Music" => "Word-level timing. Best coverage for Western pop",
        "BetterLyrics" => "Word-level timing. Mirrors Apple's database",
        "BiniLyrics" => "Word-level timing. Strong on Japanese tracks",
        "NetEase" => "Line-synced. Romanization and translation for CJK",
        "LRCLIB" => "Line-synced. Large community LRC database",
        "YouTube Music" => "Plain text, no timing. Always available when signed in",
        _ => "",
    }
}

fn move_provider(root: &mut Root, name: &str, delta: i32, cx: &mut Context<Root>) {
    let prefs = root.backend.lyrics.prefs();
    let mut order = prefs.full_provider_order();
    let Some(index) = order.iter().position(|n| n == name) else { return };
    let target = index as i32 + delta;
    if target < 0 || target as usize >= order.len() {
        return;
    }
    order.swap(index, target as usize);
    prefs.set_provider_order(&order);
    cx.notify();
}

fn toggle_provider(root: &mut Root, name: &str, cx: &mut Context<Root>) {
    let prefs = root.backend.lyrics.prefs();
    let disabled: BTreeSet<String> = prefs.disabled_providers();
    let enabled = !disabled.contains(name);
    if enabled && prefs.provider_order().len() <= 1 {
        root.notice("Keep at least one lyrics provider enabled");
        return;
    }
    prefs.set_provider_enabled(name, !enabled);
    cx.notify();
}

fn services(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let discord_status = root.backend.discord.status();

    let mut scrobble_rows = vec![
        switch_row(root, cx, "scrobble_enabled", true, "Enable scrobbling", "Send finished plays to connected services.", |root, value, _| {
            root.backend.scrobbler.set_enabled(value);
        }),
        switch_row(root, cx, "scrobble_now_playing", true, "Send now playing", "Show the current track on your profile while it plays.", |root, value, _| {
            root.backend.scrobbler.set_now_playing_enabled(value);
        }),
    ];
    for service in SERVICES {
        let connected = root.backend.scrobbler.is_connected(service);
        let username = root.backend.scrobbler.username(service);
        let subtitle = if connected && !username.is_empty() { format!("Connected as {username}") } else if connected { "Connected".to_owned() } else { "Not connected".to_owned() };
        if connected {
            scrobble_rows.push(themed_action_row(theme, service.label(), &subtitle, "Disconnect", false, cx, move |root, cx| {
                root.backend.scrobbler.disconnect(service);
                root.notice(format!("Disconnected {}", service.label()));
                cx.notify();
            }));
        } else {
            scrobble_rows.push(value_row(theme, service.label(), &subtitle, "Not connected"));
        }
    }
    page_header(theme, "Services", "Integrations with other apps and listening trackers.")
        .child(group(
            theme,
            "Discord Rich Presence",
            None,
            vec![
                value_row(theme, "Connection status", "State of the Discord socket.", &discord_status),
                switch_row(root, cx, "discord_rpc_enabled", true, "Enable Discord RPC", "Show what you are listening to on Discord.", |root, value, _| {
                    root.backend.discord.set_enabled(value);
                    if value {
                        crate::presence::update_discord(&root.backend);
                    }
                }),
                choice_row(
                    root,
                    cx,
                    "discord_rpc_status_display",
                    "artist",
                    "Status display",
                    "What appears in the status line under your name.",
                    &DISCORD_STATUS,
                    |root, _, _| crate::presence::update_discord(&root.backend),
                ),
                switch_row(root, cx, "discord_rpc_hide_pause_enabled", false, "Hide on pause", "Hide Discord RPC while music is paused.", |root, _, _| {
                    crate::presence::update_discord(&root.backend);
                }),
                switch_row(root, cx, "discord_rpc_small_icon_enabled", true, "Show play/pause icon", "Display a small indicator on the album art.", |root, _, _| {
                    crate::presence::update_discord(&root.backend);
                }),
            ],
        ))
        .child(group(
            theme,
            "Scrobbling",
            Some("Connect services in a future build; existing sessions keep working."),
            scrobble_rows,
        ))
}

fn advanced(root: &mut Root, cx: &mut Context<Root>) -> Div {
    let theme = root.theme;
    let font = root.font.to_string();
    page_header(theme, "Advanced", "Diagnostics and build information.")
        .child(group(
            theme,
            "Troubleshooting",
            None,
            vec![
                switch_row(root, cx, "debug_logs", false, "Enable debug logs", "Print diagnostic information to the terminal.", |root, value, _| {
                    bootstrap::set_debug_logs(&root.backend.paths, value);
                }),
                switch_row(root, cx, "force_offline", false, "Force offline mode", "Start with network requests disabled.", |_, _, _| {}),
            ],
        ))
        .child(group(
            theme,
            "About",
            None,
            vec![
                value_row(theme, "Musishark", "A Linux-first YouTube Music player.", env!("CARGO_PKG_VERSION")),
                value_row(theme, "UI framework", "Desktop interface toolkit.", "GPUI 0.2"),
                value_row(theme, "Interface font", "Resolved from the system at startup.", &font),
            ],
        ))
}

fn sign_out(root: &mut Root, cx: &mut Context<Root>) {
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
}

fn pref_bool(paths: &crate::paths::Paths, key: &str, default: bool) -> bool {
    paths.read_prefs().get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn pref_str(paths: &crate::paths::Paths, key: &str, default: &str) -> String {
    paths.read_prefs().get(key).and_then(Value::as_str).unwrap_or(default).to_owned()
}

fn save_bool(root: &Root, key: &str, value: bool) {
    root.backend.paths.update_prefs(|prefs| {
        prefs.insert(key.to_owned(), Value::Bool(value));
    });
}

fn save_str(root: &Root, key: &str, value: &str) {
    root.backend.paths.update_prefs(|prefs| {
        prefs.insert(key.to_owned(), Value::String(value.to_owned()));
    });
}
