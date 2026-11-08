//! GPUI front end. The layout mirrors the old GTK window: a header with the
//! three tabs, the content stack, the queue sidebar, the player bar and the
//! expanded player. Playback, networking and storage stay in the backend
//! modules and are driven through `Rc<Player>` and the GLib-backed state object.

pub mod assets;
pub mod cover;
mod detail;
mod pages;
mod player_bar;
mod queue;
mod settings;
mod shell;
pub mod theme;

pub use shell::run;
