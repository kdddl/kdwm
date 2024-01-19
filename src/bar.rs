use crate::theme;
use ::penrose_ui::bar::widgets::Text;
use penrose::util::{spawn_for_output_with_args, spawn_with_args};
use penrose::x::XConn;
use penrose_ui::bar::widgets::Widget;
use penrose_ui::{
    bar::widgets::{IntervalText, RefreshText},
    *,
};
use std::time;

pub fn create_bar<X: XConn>(state: &mut penrose::core::State<X>) -> Result<bar::StatusBar<X>> {
    let style: core::TextStyle = core::TextStyle {
        fg: theme::LIGHT[0].into(),
        bg: Some(theme::DARK[0].into()),
        padding: (10, 10),
    };

    let mut widgets: Vec<Box<dyn bar::widgets::Widget<X>>> = Vec::new();

    widgets.push(Box::new(bar::widgets::Workspaces::new(
        style,
        theme::GREEN,
        theme::DARK[3],
    )));

    widgets.push(Box::new(bar::widgets::ActiveWindowName::new(
        40, style, true, false,
    )));

    widgets.push(Box::new(IntervalText::new(
        style,
        get_datetime,
        time::Duration::from_secs(1),
    )));

    widgets.push(Box::new(IntervalText::new(
        style,
        get_updates,
        time::Duration::from_secs(60 * 15),
    )));

    widgets.push(Box::new(IntervalText::new(
        style,
        get_weather,
        time::Duration::from_secs(60 * 15),
    )));

    widgets.push(Box::new(MediaWidget::new(style)));

    bar::StatusBar::try_new(
        bar::Position::Top,
        theme::BAR_HEIGHT_PX,
        theme::DARK[0],
        theme::FONT,
        theme::POINT_SIZE,
        widgets,
    )
}

fn get_datetime() -> String {
    let datetime = chrono::Local::now();
    format!("{}", datetime.format("%d %b %Y | %H:%M:%S"))
}

// this is laggy af
fn get_updates() -> String {
    let updates = spawn_for_output_with_args("sh", &["-c", "checkupdates | wc -l"])
        .unwrap_or_default()
        .trim()
        .to_string();
    format!("UP: {updates}")
}

fn get_weather() -> String {
    let weather = spawn_for_output_with_args("curl", &["-s", "http://wttr.in?format=1"])
        .unwrap_or_default()
        .trim()
        .to_string();
    format!("{weather}")
}

struct MediaWidget {
    inner: Text,
}

impl MediaWidget {
    fn new(style: TextStyle) -> Self {
        Self {
            inner: Text::new("", style, false, true),
        }
    }
}

impl<X: XConn> Widget<X> for MediaWidget {
    fn draw(&mut self, ctx: &mut Context<'_>, s: usize, f: bool, w: u32, h: u32) -> Result<()> {
        Widget::<X>::draw(&mut self.inner, ctx, s, f, w, h)
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)> {
        Widget::<X>::current_extent(&mut self.inner, ctx, h)
    }

    fn is_greedy(&self) -> bool {
        Widget::<X>::is_greedy(&self.inner)
    }

    fn require_draw(&self) -> bool {
        Widget::<X>::require_draw(&self.inner)
    }

    fn on_refresh(&mut self, state: &mut penrose::core::State<X>, x: &X) -> Result<()> {
        let player = state.extension::<crate::Media>().unwrap();
        let player = player.borrow();
        let player = match &player.player {
            None => "",
            Some(value) => value,
        };
        let media = spawn_for_output_with_args(
            "playerctl",
            &[
                "-p",
                player,
                "metadata",
                "-f",
                "{{ artist }}: {{ title }} [{{ playerName }}]",
            ],
        )
        .unwrap_or_default()
        .trim()
        .to_string();
        self.inner.set_text(media);

        Ok(())
    }
}
