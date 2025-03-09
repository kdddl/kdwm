use penrose::extensions::hooks::SpawnOnStartup;
use penrose::stack;
use penrose::util::{spawn_for_output, spawn_for_output_with_args, spawn_with_args};
use penrose::{
    builtin::{
        actions::{exit, floating, key_handler, modify_with, send_layout_message, spawn},
        layout::{
            messages::{ExpandMain, IncMain, ShrinkMain},
            transformers::{Gaps, ReserveTop},
            Grid, MainAndStack,
        },
    },
    core::{
        bindings::{parse_keybindings_with_xmodmap, KeyEventHandler},
        layout::LayoutStack,
        Config, WindowManager,
    },
    extensions::actions::toggle_fullscreen,
    extensions::hooks::add_ewmh_hooks,
    map,
    x11rb::RustConn,
    Result,
};
use std::collections::HashMap;
use std::time::SystemTime;
use tracing_subscriber::{self, prelude::*};

mod bar;
mod theme;

// apps
const TERM: &str = "alacritty";
const RUNNER: &str = "rofi -show run";
const WINDOWS: &str = "rofi -show window";
const WEB_BROWSER: &str = "firefox";

const BAR_HEIGHT_PX: u32 = 18;
// status bar
const FONT: &str = "Ubuntu Mono";
const BLACK: u32 = 0x282828ff;
const WHITE: u32 = 0xd5c4a1ff;
const GREY: u32 = 0x665c54ff;
const BLUE: u32 = 0x83a598ff;

fn raw_key_bindings() -> HashMap<String, Box<dyn KeyEventHandler<RustConn>>> {
    let mut raw_bindings = map! {
        map_keys: |k: &str| k.to_string();

        "M-j" => modify_with(|cs| cs.focus_down()),
        "M-k" => modify_with(|cs| cs.focus_up()),
        "M-S-j" => modify_with(|cs| cs.swap_down()),
        "M-S-k" => modify_with(|cs| cs.swap_up()),
        "M-e" => modify_with(|cs| cs.focus_down()),
        "M-i" => modify_with(|cs| cs.focus_up()),
        "M-S-e" => modify_with(|cs| cs.swap_down()),
        "M-S-i" => modify_with(|cs| cs.swap_up()),
        "M-S-q" => modify_with(|cs| cs.kill_focused()),
        "M-Tab" => modify_with(|cs| cs.toggle_tag()),
        "M-l" => modify_with(|cs| cs.next_screen()),
        "M-h" => modify_with(|cs| cs.previous_screen()),
        "M-o" => modify_with(|cs| cs.next_screen()),
        "M-n" => modify_with(|cs| cs.previous_screen()),
        "M-grave" => modify_with(|cs| cs.next_layout()),
        "M-S-grave" => modify_with(|cs| cs.previous_layout()),
        "M-f" => toggle_fullscreen(),
        "M-S-Up" => send_layout_message(|| IncMain(1)),
        "M-S-Down" => send_layout_message(|| IncMain(-1)),
        "M-S-Right" => send_layout_message(|| ExpandMain),
        "M-S-Left" => send_layout_message(|| ShrinkMain),
        "M-semicolon" => spawn(RUNNER),
        "M-Return" => spawn(TERM),
        "M-apostrophe" => spawn(WEB_BROWSER),
        "M-r" => exit(),
        "M-Escape" => key_handler(|_, _| exit_menu()),
        "M-p" => key_handler(get_players),
        "M-backslash" => key_handler(|state, x| media(state, x, MediaMsg::PlayPause)),
        "M-bracketleft" => key_handler(|state, x| media(state, x, MediaMsg::Previous)),
        "M-bracketright" => key_handler(|state, x| media(state, x, MediaMsg::Next)),
        "M-slash" => spawn(WINDOWS),
        "M-s" => key_handler(|_, _| screenshot()),
        "M-S-s" => key_handler(|_, _| static_screenshot()),
        "M-c" => key_handler(|_, _| qalc()),
    };

    for tag in &["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        raw_bindings.extend([
            (
                format!("M-{tag}"),
                modify_with(move |client_set| client_set.focus_tag(tag)),
            ),
            (
                format!("M-S-{tag}"),
                modify_with(move |client_set| client_set.move_focused_to_tag(tag)),
            ),
        ]);
    }

    raw_bindings
}

fn layouts() -> LayoutStack {
    stack!(
        MainAndStack::boxed_default(),
        MainAndStack::side_mirrored(1, 0.6, 0.1),
        Grid::boxed()
    )
    .map(|layout| ReserveTop::wrap(layout, BAR_HEIGHT_PX))
    .map(|layout| Gaps::wrap(layout, 4, 4))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .finish()
        .init();

    let startup = SpawnOnStartup::boxed("/usr/local/scripts/startup.sh");

    let conn = RustConn::new()?;
    let key_bindings = parse_keybindings_with_xmodmap(raw_key_bindings())?;
    let config = add_ewmh_hooks(Config {
        default_layouts: layouts(),
        focused_border: theme::LIGHT[2].into(),
        normal_border: theme::DARK[2].into(),
        focus_follow_mouse: true,
        border_width: 1,
        startup_hook: startup.into(),
        floating_classes: vec!["Qalculate!".to_string()],
        ..Config::default()
    });

    let mut wm = WindowManager::new(config, key_bindings, HashMap::new(), conn)?;

    let bar = bar::create_bar().unwrap();
    wm = bar.add_to(wm);

    wm.add_extension(Media { player: None });

    wm.run()
}

struct Media {
    player: Option<String>,
}

use penrose::x::XConn;
fn get_players<X: XConn>(state: &mut penrose::core::State<X>, _x: &X) -> Result<()> {
    let players = spawn_for_output_with_args("playerctl", &["-l"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let players: Vec<&str> = players.split("\n").map(|x| x.trim()).collect();
    let selection: usize = spawn_for_output_with_args("sh", &["-c", "playerctl -a metadata -f '{{ artist}}: {{ title }} [{{ playerName }}]' | rofi -dmenu -only-match -format 'i'"]).unwrap_or_default().trim().parse().unwrap();
    let player = state.extension::<Media>().unwrap();
    player.borrow_mut().player = Some(players[selection].to_string());
    Ok(())
}

enum MediaMsg {
    PlayPause,
    Next,
    Previous,
}

fn media<X: XConn>(state: &mut penrose::core::State<X>, _x: &X, msg: MediaMsg) -> Result<()> {
    let media = state.extension::<Media>().unwrap();
    let media = media.borrow();
    let player = match &media.player {
        None => "",
        Some(value) => value,
    };
    let action = match msg {
        MediaMsg::PlayPause => "play-pause",
        MediaMsg::Next => "next",
        MediaMsg::Previous => "previous",
    };
    spawn_with_args("playerctl", &["-p", player, action]).unwrap_or_default();
    Ok(())
}

fn screenshot() -> Result<()> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let path = format!("~/screenshots/{time}.png");
    let shell = format!("maim -s -u {path}");
    let _ = spawn_for_output_with_args("sh", &["-c", &shell])?;
    let shell = format!("cat {path} | xclip -selection clipboard -t image/png -i");
    spawn_with_args("sh", &["-c", &shell])?;
    Ok(())
}

fn static_screenshot() -> Result<()> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let path = format!("~/screenshots/{time}.png");
    let shell = format!("maim -u {path}");
    let _ = spawn_for_output_with_args("sh", &["-c", &shell])?;
    let shell = format!("cat {path} | xclip -selection clipboard -t image/png -i");
    spawn_with_args("sh", &["-c", &shell])?;
    Ok(())
}

fn exit_menu() -> Result<()> {
    let options = "quit\nreboot\nshutdown";
    let selection =
        spawn_for_output_with_args("sh", &["-c", &format!("echo '{options}' | rofi -dmenu")])?;
    match selection.as_str() {
        "quit" => {
            spawn_with_args("pkill", &["-fi", "kdwm"])?;
        }
        "reboot" => {
            spawn_with_args("reboot", &[])?;
        }
        "shutdown" => {
            spawn_with_args("shutdown", &["now"])?;
        }
        _ => {}
    }
    Ok(())
}

fn qalc() -> Result<()> {
    let thread = std::thread::spawn(move || {
        let equation =
            spawn_for_output_with_args("rofi", &["-dmenu", "-l", "0", "-p", "calc"]).unwrap();
        if equation != "" {
            let result = spawn_for_output_with_args("qalc", &["-t", &equation]).unwrap();
            spawn_with_args("rofi", &["-e", &result.trim()]).unwrap();
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_parse_correctly_with_xmodmap() {
        let res = parse_keybindings_with_xmodmap(raw_key_bindings());

        if let Err(e) = res {
            panic!("{e}");
        }
    }
}
