use crate::theme;
use ::penrose_ui::bar::widgets::Text;
use penrose::util::spawn_for_output_with_args;
use penrose::x::XConn;
use penrose_ui::bar::widgets::Widget;
use penrose_ui::{bar::widgets::IntervalText, *};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time;

pub fn create_bar<X: XConn>() -> Result<bar::StatusBar<X>> {
    let style: core::TextStyle = core::TextStyle {
        fg: theme::LIGHT[0].into(),
        bg: Some(theme::DARK[0].into()),
        padding: (10, 0),
    };

    let style2: core::TextStyle = core::TextStyle {
        fg: theme::LIGHT[0].into(),
        bg: Some(theme::DARK[0].into()),
        padding: (10, 0),
    };

    let internalsep: core::TextStyle = core::TextStyle {
        fg: theme::LIGHT[0].into(),
        bg: Some(theme::DARK[0].into()),
        padding: (0, 0),
    };

    let orange: core::TextStyle = core::TextStyle {
        fg: theme::ORANGE.into(),
        bg: Some(theme::DARK[0].into()),
        padding: (10, 0),
    };

    let blue: core::TextStyle = core::TextStyle {
        fg: theme::BLUE.into(),
        bg: Some(theme::DARK[0].into()),
        padding: (10, 0),
    };

    let green: core::TextStyle = core::TextStyle {
        fg: theme::GREEN.into(),
        bg: Some(theme::DARK[0].into()),
        padding: (10, 0),
    };

    let mut widgets: Vec<Box<dyn bar::widgets::Widget<X>>> = Vec::new();

    widgets.push(Box::new(bar::widgets::Workspaces::new(
        style,
        theme::DARK[3],
        theme::DARK[1],
    )));

    widgets.push(Box::new(bar::widgets::ActiveWindowName::new(
        40, green, true, false,
    )));

    widgets.push(Box::new(Text::new("|", style2, false, true)));

    widgets.push(Box::new(Text::new("PLAY:", blue, false, true)));
    widgets.push(Box::new(MediaWidget::new(green)));

    widgets.push(Box::new(Text::new("|", style2, false, true)));

    widgets.push(Box::new(Text::new("WTTR:", blue, false, true)));
    widgets.push(Box::new(IntervalText::new(
        style,
        get_weather,
        time::Duration::from_secs(60 * 60),
    )));

    widgets.push(Box::new(Text::new("|", style2, false, true)));

    widgets.push(Box::new(Text::new("PKGS:", blue, false, true)));
    widgets.push(Box::new(IntervalText::new(
        orange,
        get_updates,
        time::Duration::from_secs(60 * 15),
    )));

    widgets.push(Box::new(Text::new("|", style2, false, true)));

    widgets.push(Box::new(Text::new("BATT:", blue, false, true)));
    widgets.push(Box::new(IntervalText::new(
        orange,
        get_battery,
        time::Duration::from_secs(1),
    )));

    widgets.push(Box::new(Text::new("|", style2, false, true)));

    widgets.push(Box::new(Text::new("TIME:", blue, false, true)));
    widgets.push(Box::new(IntervalText::new(
        green,
        get_date,
        time::Duration::from_secs(1),
    )));
    widgets.push(Box::new(Text::new(",", internalsep, false, true)));
    widgets.push(Box::new(IntervalText::new(
        orange,
        get_time,
        time::Duration::from_secs(1),
    )));

    widgets.push(Box::new(Text::new(" ", style2, false, true)));

    bar::StatusBar::try_new(
        bar::Position::Top,
        theme::BAR_HEIGHT_PX,
        theme::DARK[0],
        theme::FONT,
        theme::POINT_SIZE,
        widgets,
    )
}

fn get_date() -> Option<String> {
    let datetime = chrono::Local::now();
    Some(format!("{}", datetime.format("\"%d %b %Y\"")))
}

fn get_time() -> Option<String> {
    let datetime = chrono::Local::now();
    Some(format!("{}", datetime.format("%H:%M:%S")))
}

// this is laggy af
fn get_updates() -> Option<String> {
    let updates = spawn_for_output_with_args("sh", &["-c", "checkupdates | wc -l"])
        .unwrap_or_default()
        .trim()
        .to_string();
    if updates != "" {
        Some(updates)
    } else {
        Some("0".to_string())
    }
}

fn get_weather() -> Option<String> {
    let weather = spawn_for_output_with_args("curl", &["-s", "http://wttr.in?format=1"])
        .unwrap_or_default()
        .trim()
        .to_string();
    Some(format!("{weather}"))
}

fn get_battery() -> Option<String> {
    let full = get_battery_helper("BAT0", "charge_full");
    let current = get_battery_helper("BAT0", "charge_now");
    let status = match std::fs::read_to_string(format!("/sys/class/power_supply/BAT0/status")).ok()
    {
        None => "",
        Some(string) => match string.trim() {
            "Charging" => "▲",
            "Discharging" => "▼",
            _ => "-",
        },
    };
    Some(if let Some(full) = full {
        if let Some(current) = current {
            let charge_percent = (current as f32) / (full as f32) * 100.0;
            format!("{charge_percent:.2}% {status}")
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    })
}

fn get_battery_helper(bat: &str, fname: &str) -> Option<u32> {
    match std::fs::read_to_string(format!("/sys/class/power_supply/{bat}/{fname}"))
        .ok()
        .map(|s| s.trim().to_string().parse().ok())
    {
        None => None,
        Some(item) => item,
    }
}

struct MediaWidget {
    inner: Arc<Mutex<Text>>,
    tx: mpsc::Sender<String>,
}

struct MediaText {
    player: Text,
    title: Text,
    artist: Text,
}

impl MediaWidget {
    fn new(style: TextStyle) -> Self {
        let inner = Arc::new(Mutex::new(Text::new("", style, false, true)));
        let text = Arc::clone(&inner);
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut player = "".to_string();
            loop {
                tracing::info!("Updating media widget");
                if let Ok(value) = rx.recv() {
                    player = value;
                }
                {
                    let mut t = match text.lock() {
                        Ok(inner) => inner,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    let mut media = spawn_for_output_with_args(
                        "playerctl",
                        &[
                            "-p",
                            &player,
                            "metadata",
                            "-f",
                            "{{ artist }}: {{ title }} [{{ playerName }}]",
                        ],
                    )
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                    if media == "" {
                        media = "None".to_string()
                    }
                    t.set_text(&format!("{media}"));
                }
                thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        Self { inner, tx }
    }
}

impl<X: XConn> Widget<X> for MediaWidget {
    fn draw(&mut self, ctx: &mut Context<'_>, s: usize, f: bool, w: u32, h: u32) -> Result<()> {
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::draw(&mut *inner, ctx, s, f, w, h)
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)> {
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::current_extent(&mut *inner, ctx, h)
    }

    fn is_greedy(&self) -> bool {
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::is_greedy(&*inner)
    }

    fn require_draw(&self) -> bool {
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(poisoned) => poisoned.into_inner(),
        };

        Widget::<X>::require_draw(&*inner)
    }
    fn on_refresh(&mut self, state: &mut penrose::core::State<X>, x: &X) -> Result<()> {
        let player = state.extension::<crate::Media>().unwrap();
        let player = player.borrow();
        let player = match &player.player {
            None => "",
            Some(value) => value,
        };

        self.tx.send(player.to_string()).unwrap();

        Ok(())
    }
}

struct MultiText {
    text: Vec<Text>,
    styles: Vec<TextStyle>,
    right_justified: bool,
}

impl MultiText {
    pub fn new(text: Vec<&str>, styles: Vec<TextStyle>, right_justified: bool) -> Self {
        let text = text
            .iter()
            .enumerate()
            .map(|(index, text)| Text::new(text.to_string(), styles[index], false, right_justified))
            .collect();
        Self {
            text,
            styles,
            right_justified,
        }
    }
}

impl<X: XConn> Widget<X> for MultiText {
    fn draw(&mut self, ctx: &mut Context<'_>, s: usize, f: bool, w: u32, h: u32) -> Result<()> {
        for text in self.text.iter_mut() {
            Widget::<X>::draw(text, ctx, s, f, w, h)?;
        }

        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)> {
        let mut sum = (0u32, 0u32);
        for text in self.text.iter_mut() {
            match Widget::<X>::current_extent(text, ctx, h) {
                Err(_) => {}
                Ok(value) => {
                    sum.0 += value.0;
                    sum.1 += value.0;
                }
            }
        }

        Ok(sum)
    }

    fn is_greedy(&self) -> bool {
        false
    }

    fn require_draw(&self) -> bool {
        for text in self.text.iter() {
            if Widget::<X>::require_draw(&*text) {
                return true;
            }
        }
        false
    }
}
