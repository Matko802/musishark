use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub const PLAY: &str = "icons/play.svg";
pub const PAUSE: &str = "icons/pause.svg";
pub const NEXT: &str = "icons/next.svg";
pub const PREV: &str = "icons/prev.svg";
pub const SEARCH: &str = "icons/search.svg";
pub const HOME: &str = "icons/home.svg";
pub const LIBRARY: &str = "icons/library.svg";
pub const QUEUE: &str = "icons/queue.svg";
pub const VOLUME: &str = "icons/volume.svg";
pub const VOLUME_MUTED: &str = "icons/volume-muted.svg";
pub const CHEVRON_UP: &str = "icons/chevron-up.svg";
pub const CHEVRON_DOWN: &str = "icons/chevron-down.svg";
pub const MORE: &str = "icons/more.svg";
pub const SHUFFLE: &str = "icons/shuffle.svg";
pub const REPEAT: &str = "icons/repeat.svg";
pub const CLOSE: &str = "icons/close.svg";
pub const USER: &str = "icons/user.svg";
pub const DOWNLOAD: &str = "icons/download.svg";
pub const PLUS: &str = "icons/plus.svg";
pub const LYRICS: &str = "icons/lyrics.svg";
pub const SETTINGS: &str = "icons/settings.svg";
pub const REFRESH: &str = "icons/refresh.svg";
pub const BACK: &str = "icons/back.svg";
pub const FORWARD: &str = "icons/arrow-forward.svg";
pub const THUMB_UP: &str = "icons/thumb-up.svg";
pub const THUMB_UP_FILLED: &str = "icons/thumb-up-filled.svg";
pub const COMPASS: &str = "icons/compass.svg";
pub const MUSIC: &str = "icons/music.svg";
pub const TRASH: &str = "icons/trash.svg";
pub const CHECK: &str = "icons/check.svg";
pub const ANTENNA: &str = "icons/antenna.svg";
pub const EXTERNAL: &str = "icons/external.svg";
pub const SHARE: &str = "icons/share.svg";
pub const THUMB_DOWN: &str = "icons/thumb-down.svg";
pub const SUN: &str = "icons/sun.svg";
pub const MOON: &str = "icons/moon.svg";

pub struct Assets;

fn bytes(path: &str) -> Option<&'static [u8]> {
    Some(match path {
        PLAY => include_bytes!("../../assets/icons/gpui/play.svg"),
        PAUSE => include_bytes!("../../assets/icons/gpui/pause.svg"),
        NEXT => include_bytes!("../../assets/icons/gpui/next.svg"),
        PREV => include_bytes!("../../assets/icons/gpui/prev.svg"),
        SEARCH => include_bytes!("../../assets/icons/gpui/search.svg"),
        HOME => include_bytes!("../../assets/icons/gpui/home.svg"),
        LIBRARY => include_bytes!("../../assets/icons/gpui/library.svg"),
        QUEUE => include_bytes!("../../assets/icons/gpui/queue.svg"),
        VOLUME => include_bytes!("../../assets/icons/gpui/volume.svg"),
        VOLUME_MUTED => include_bytes!("../../assets/icons/gpui/volume-muted.svg"),
        CHEVRON_UP => include_bytes!("../../assets/icons/gpui/chevron-up.svg"),
        CHEVRON_DOWN => include_bytes!("../../assets/icons/gpui/chevron-down.svg"),
        MORE => include_bytes!("../../assets/icons/gpui/more.svg"),
        SHUFFLE => include_bytes!("../../assets/icons/gpui/shuffle.svg"),
        REPEAT => include_bytes!("../../assets/icons/gpui/repeat.svg"),
        CLOSE => include_bytes!("../../assets/icons/gpui/close.svg"),
        USER => include_bytes!("../../assets/icons/gpui/user.svg"),
        DOWNLOAD => include_bytes!("../../assets/icons/gpui/download.svg"),
        PLUS => include_bytes!("../../assets/icons/gpui/plus.svg"),
        LYRICS => include_bytes!("../../assets/icons/gpui/lyrics.svg"),
        SETTINGS => include_bytes!("../../assets/icons/gpui/settings.svg"),
        REFRESH => include_bytes!("../../assets/icons/gpui/refresh.svg"),
        BACK => include_bytes!("../../assets/icons/gpui/back.svg"),
        FORWARD => include_bytes!("../../assets/icons/gpui/arrow-forward.svg"),
        THUMB_UP => include_bytes!("../../assets/icons/gpui/thumb-up.svg"),
        THUMB_UP_FILLED => include_bytes!("../../assets/icons/gpui/thumb-up-filled.svg"),
        COMPASS => include_bytes!("../../assets/icons/gpui/compass.svg"),
        MUSIC => include_bytes!("../../assets/icons/gpui/music.svg"),
        TRASH => include_bytes!("../../assets/icons/gpui/trash.svg"),
        CHECK => include_bytes!("../../assets/icons/gpui/check.svg"),
        ANTENNA => include_bytes!("../../assets/icons/gpui/antenna.svg"),
        EXTERNAL => include_bytes!("../../assets/icons/gpui/external.svg"),
        SHARE => include_bytes!("../../assets/icons/gpui/share.svg"),
        THUMB_DOWN => include_bytes!("../../assets/icons/gpui/thumb-down.svg"),
        SUN => include_bytes!("../../assets/icons/gpui/sun.svg"),
        MOON => include_bytes!("../../assets/icons/gpui/moon.svg"),
        _ => return None,
    })
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(bytes(path).map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
