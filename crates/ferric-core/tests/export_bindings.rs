use ferric_core::{
    command::Command,
    model::{Art, Auth, Mode, Playback, PlayerState, Position, RepeatMode, Rgb, Theme, Track},
};
use ts_rs::TS;

#[test]
fn export_bindings() {
    let config = ts_rs::Config::default();
    PlayerState::export(&config).expect("export PlayerState");
    Track::export(&config).expect("export Track");
    Art::export(&config).expect("export Art");
    Playback::export(&config).expect("export Playback");
    Position::export(&config).expect("export Position");
    RepeatMode::export(&config).expect("export RepeatMode");
    Auth::export(&config).expect("export Auth");
    Rgb::export(&config).expect("export Rgb");
    Theme::export(&config).expect("export Theme");
    Mode::export(&config).expect("export Mode");
    Command::export(&config).expect("export Command");
}
