//! This module provides the `TermwizBackend` implementation for the [`Backend`] trait. It uses the
//! [Termwiz] crate to interact with the terminal.
//!
//! [`Backend`]: trait.Backend.html
//! [`TermwizBackend`]: crate::backend::TermionBackend
//! [Termwiz]: https://crates.io/crates/termwiz

use std::{error::Error, io};

use termwiz::{
    caps::Capabilities,
    cell::{AttributeChange, Blink, CellAttributes, Intensity, Underline},
    color::{AnsiColor, ColorAttribute, ColorSpec, LinearRgba, RgbColor, SrgbaTuple},
    surface::{Change, CursorVisibility, Position},
    terminal::{buffered::BufferedTerminal, ScreenSize, SystemTerminal, Terminal},
};

use crate::{
    backend::{Backend, WindowSize},
    buffer::Cell,
    layout::Size,
    prelude::Rect,
    style::{Color, Modifier, Style},
};

/// A [`Backend`] implementation that uses [Termwiz] to render to the terminal.
///
/// The `TermwizBackend` struct is a wrapper around a [`BufferedTerminal`], which is used to send
/// commands to the terminal. It provides methods for drawing content, manipulating the cursor, and
/// clearing the terminal screen.
///
/// Most applications should not call the methods on `TermwizBackend` directly, but will instead
/// use the [`Terminal`] struct, which provides a more ergonomic interface.
///
/// This backend automatically enables raw mode and switches to the alternate screen when it is
/// created using the [`TermwizBackend::new`] method (and disables raw mode and returns to the main
/// screen when dropped). Use the [`TermwizBackend::with_buffered_terminal`] to create a new
/// instance with a custom [`BufferedTerminal`] if this is not desired.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui::prelude::*;
///
/// let backend = TermwizBackend::new()?;
/// let mut terminal = Terminal::new(backend)?;
///
/// terminal.clear()?;
/// terminal.draw(|frame| {
///     // -- snip --
/// })?;
/// # std::result::Result::Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// See the the [examples] directory for more examples. See the [`backend`] module documentation
/// for more details on raw mode and alternate screen.
///
/// [`backend`]: crate::backend
/// [`Terminal`]: crate::terminal::Terminal
/// [`BufferedTerminal`]: termwiz::terminal::buffered::BufferedTerminal
/// [Termwiz]: https://crates.io/crates/termwiz
/// [examples]: https://github.com/ratatui-org/ratatui/tree/main/examples#readme
pub struct TermwizBackend {
    buffered_terminal: BufferedTerminal<SystemTerminal>,
}

impl TermwizBackend {
    /// Creates a new Termwiz backend instance.
    ///
    /// The backend will automatically enable raw mode and enter the alternate screen.
    ///
    /// # Errors
    ///
    /// Returns an error if unable to do any of the following:
    /// - query the terminal capabilities.
    /// - enter raw mode.
    /// - enter the alternate screen.
    /// - create the system or buffered terminal.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ratatui::prelude::*;
    /// let backend = TermwizBackend::new()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new() -> Result<TermwizBackend, Box<dyn Error>> {
        let mut buffered_terminal =
            BufferedTerminal::new(SystemTerminal::new(Capabilities::new_from_env()?)?)?;
        buffered_terminal.terminal().set_raw_mode()?;
        buffered_terminal.terminal().enter_alternate_screen()?;
        Ok(TermwizBackend { buffered_terminal })
    }

    /// Creates a new Termwiz backend instance with the given buffered terminal.
    pub fn with_buffered_terminal(instance: BufferedTerminal<SystemTerminal>) -> TermwizBackend {
        TermwizBackend {
            buffered_terminal: instance,
        }
    }

    /// Returns a reference to the buffered terminal used by the backend.
    pub fn buffered_terminal(&self) -> &BufferedTerminal<SystemTerminal> {
        &self.buffered_terminal
    }

    /// Returns a mutable reference to the buffered terminal used by the backend.
    pub fn buffered_terminal_mut(&mut self) -> &mut BufferedTerminal<SystemTerminal> {
        &mut self.buffered_terminal
    }
}

impl Backend for TermwizBackend {
    fn draw<'a, I>(&mut self, content: I) -> Result<(), io::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        for (x, y, cell) in content {
            self.buffered_terminal.add_changes(vec![
                Change::CursorPosition {
                    x: Position::Absolute(x as usize),
                    y: Position::Absolute(y as usize),
                },
                Change::Attribute(AttributeChange::Foreground(cell.fg.into())),
                Change::Attribute(AttributeChange::Background(cell.bg.into())),
            ]);

            self.buffered_terminal
                .add_change(Change::Attribute(AttributeChange::Intensity(
                    if cell.modifier.contains(Modifier::BOLD) {
                        Intensity::Bold
                    } else if cell.modifier.contains(Modifier::DIM) {
                        Intensity::Half
                    } else {
                        Intensity::Normal
                    },
                )));

            self.buffered_terminal
                .add_change(Change::Attribute(AttributeChange::Italic(
                    cell.modifier.contains(Modifier::ITALIC),
                )));

            self.buffered_terminal
                .add_change(Change::Attribute(AttributeChange::Underline(
                    if cell.modifier.contains(Modifier::UNDERLINED) {
                        Underline::Single
                    } else {
                        Underline::None
                    },
                )));

            self.buffered_terminal
                .add_change(Change::Attribute(AttributeChange::Reverse(
                    cell.modifier.contains(Modifier::REVERSED),
                )));

            self.buffered_terminal
                .add_change(Change::Attribute(AttributeChange::Invisible(
                    cell.modifier.contains(Modifier::HIDDEN),
                )));

            self.buffered_terminal
                .add_change(Change::Attribute(AttributeChange::StrikeThrough(
                    cell.modifier.contains(Modifier::CROSSED_OUT),
                )));

            self.buffered_terminal
                .add_change(Change::Attribute(AttributeChange::Blink(
                    if cell.modifier.contains(Modifier::SLOW_BLINK) {
                        Blink::Slow
                    } else if cell.modifier.contains(Modifier::RAPID_BLINK) {
                        Blink::Rapid
                    } else {
                        Blink::None
                    },
                )));

            self.buffered_terminal.add_change(cell.symbol());
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), io::Error> {
        self.buffered_terminal
            .add_change(Change::CursorVisibility(CursorVisibility::Hidden));
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), io::Error> {
        self.buffered_terminal
            .add_change(Change::CursorVisibility(CursorVisibility::Visible));
        Ok(())
    }

    fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        let (x, y) = self.buffered_terminal.cursor_position();
        Ok((x as u16, y as u16))
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.buffered_terminal.add_change(Change::CursorPosition {
            x: Position::Absolute(x as usize),
            y: Position::Absolute(y as usize),
        });

        Ok(())
    }

    fn clear(&mut self) -> Result<(), io::Error> {
        self.buffered_terminal
            .add_change(Change::ClearScreen(termwiz::color::ColorAttribute::Default));
        Ok(())
    }

    fn size(&self) -> Result<Rect, io::Error> {
        let (cols, rows) = self.buffered_terminal.dimensions();
        Ok(Rect::new(0, 0, u16_max(cols), u16_max(rows)))
    }

    fn window_size(&mut self) -> Result<WindowSize, io::Error> {
        let ScreenSize {
            cols,
            rows,
            xpixel,
            ypixel,
        } = self
            .buffered_terminal
            .terminal()
            .get_screen_size()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(WindowSize {
            columns_rows: Size {
                width: u16_max(cols),
                height: u16_max(rows),
            },
            pixels: Size {
                width: u16_max(xpixel),
                height: u16_max(ypixel),
            },
        })
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.buffered_terminal
            .flush()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(())
    }
}

impl From<CellAttributes> for Style {
    fn from(value: CellAttributes) -> Self {
        let mut style = Style::new()
            .add_modifier(value.intensity().into())
            .add_modifier(value.underline().into())
            .add_modifier(value.blink().into());

        if value.italic() {
            style.add_modifier |= Modifier::ITALIC;
        }
        if value.reverse() {
            style.add_modifier |= Modifier::REVERSED;
        }
        if value.strikethrough() {
            style.add_modifier |= Modifier::CROSSED_OUT;
        }
        if value.invisible() {
            style.add_modifier |= Modifier::HIDDEN;
        }

        style.fg = Some(value.foreground().into());
        style.bg = Some(value.background().into());
        #[cfg(feature = "underline_color")]
        {
            style.underline_color = Some(value.underline_color().into());
        }

        style
    }
}

impl From<Intensity> for Modifier {
    fn from(value: Intensity) -> Self {
        match value {
            Intensity::Normal => Modifier::empty(),
            Intensity::Bold => Modifier::BOLD,
            Intensity::Half => Modifier::DIM,
        }
    }
}

impl From<Underline> for Modifier {
    fn from(value: Underline) -> Self {
        match value {
            Underline::None => Modifier::empty(),
            _ => Modifier::UNDERLINED,
        }
    }
}

impl From<Blink> for Modifier {
    fn from(value: Blink) -> Self {
        match value {
            Blink::None => Modifier::empty(),
            Blink::Slow => Modifier::SLOW_BLINK,
            Blink::Rapid => Modifier::RAPID_BLINK,
        }
    }
}

impl From<Color> for ColorAttribute {
    fn from(color: Color) -> ColorAttribute {
        match color {
            Color::Reset => ColorAttribute::Default,
            Color::Black => AnsiColor::Black.into(),
            Color::DarkGray => AnsiColor::Grey.into(),
            Color::Gray => AnsiColor::Silver.into(),
            Color::Red => AnsiColor::Maroon.into(),
            Color::LightRed => AnsiColor::Red.into(),
            Color::Green => AnsiColor::Green.into(),
            Color::LightGreen => AnsiColor::Lime.into(),
            Color::Yellow => AnsiColor::Olive.into(),
            Color::LightYellow => AnsiColor::Yellow.into(),
            Color::Magenta => AnsiColor::Purple.into(),
            Color::LightMagenta => AnsiColor::Fuchsia.into(),
            Color::Cyan => AnsiColor::Teal.into(),
            Color::LightCyan => AnsiColor::Aqua.into(),
            Color::White => AnsiColor::White.into(),
            Color::Blue => AnsiColor::Navy.into(),
            Color::LightBlue => AnsiColor::Blue.into(),
            Color::Indexed(i) => ColorAttribute::PaletteIndex(i),
            Color::Rgb(r, g, b) => {
                ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple::from((r, g, b)))
            }
        }
    }
}

impl From<AnsiColor> for Color {
    fn from(value: AnsiColor) -> Self {
        match value {
            AnsiColor::Black => Color::Black,
            AnsiColor::Grey => Color::DarkGray,
            AnsiColor::Silver => Color::Gray,
            AnsiColor::Maroon => Color::Red,
            AnsiColor::Red => Color::LightRed,
            AnsiColor::Green => Color::Green,
            AnsiColor::Lime => Color::LightGreen,
            AnsiColor::Olive => Color::Yellow,
            AnsiColor::Yellow => Color::LightYellow,
            AnsiColor::Purple => Color::Magenta,
            AnsiColor::Fuchsia => Color::LightMagenta,
            AnsiColor::Teal => Color::Cyan,
            AnsiColor::Aqua => Color::LightCyan,
            AnsiColor::White => Color::White,
            AnsiColor::Navy => Color::Blue,
            AnsiColor::Blue => Color::LightBlue,
        }
    }
}

impl From<ColorAttribute> for Color {
    fn from(value: ColorAttribute) -> Self {
        match value {
            ColorAttribute::TrueColorWithDefaultFallback(srgba)
            | ColorAttribute::TrueColorWithPaletteFallback(srgba, _) => srgba.into(),
            ColorAttribute::PaletteIndex(i) => Color::Indexed(i),
            ColorAttribute::Default => Color::Reset,
        }
    }
}

impl From<ColorSpec> for Color {
    fn from(value: ColorSpec) -> Self {
        match value {
            ColorSpec::Default => Color::Reset,
            ColorSpec::PaletteIndex(i) => Color::Indexed(i),
            ColorSpec::TrueColor(srgba) => srgba.into(),
        }
    }
}

impl From<SrgbaTuple> for Color {
    fn from(value: SrgbaTuple) -> Self {
        let (r, g, b, _) = value.to_srgb_u8();
        Color::Rgb(r, g, b)
    }
}

impl From<RgbColor> for Color {
    fn from(value: RgbColor) -> Self {
        let (r, g, b) = value.to_tuple_rgb8();
        Color::Rgb(r, g, b)
    }
}

impl From<LinearRgba> for Color {
    fn from(value: LinearRgba) -> Self {
        value.to_srgb().into()
    }
}

#[inline]
fn u16_max(i: usize) -> u16 {
    u16::try_from(i).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Stylize;

    mod into_color {
        use Color as C;

        use super::*;

        #[test]
        fn from_linear_rgba() {
            // full black + opaque
            assert_eq!(C::from(LinearRgba(0., 0., 0., 1.)), Color::Rgb(0, 0, 0));
            // full black + transparent
            assert_eq!(C::from(LinearRgba(0., 0., 0., 0.)), Color::Rgb(0, 0, 0));

            // full white + opaque
            assert_eq!(C::from(LinearRgba(1., 1., 1., 1.)), C::Rgb(254, 254, 254));
            // full white + transparent
            assert_eq!(C::from(LinearRgba(1., 1., 1., 0.)), C::Rgb(254, 254, 254));

            // full red
            assert_eq!(C::from(LinearRgba(1., 0., 0., 1.)), C::Rgb(254, 0, 0));
            // full green
            assert_eq!(C::from(LinearRgba(0., 1., 0., 1.)), C::Rgb(0, 254, 0));
            // full blue
            assert_eq!(C::from(LinearRgba(0., 0., 1., 1.)), C::Rgb(0, 0, 254));

            // See https://stackoverflow.com/questions/12524623/what-are-the-practical-differences-when-working-with-colors-in-a-linear-vs-a-no
            // for an explanation

            // half red
            assert_eq!(C::from(LinearRgba(0.214, 0., 0., 1.)), C::Rgb(127, 0, 0));
            // half green
            assert_eq!(C::from(LinearRgba(0., 0.214, 0., 1.)), C::Rgb(0, 127, 0));
            // half blue
            assert_eq!(C::from(LinearRgba(0., 0., 0.214, 1.)), C::Rgb(0, 0, 127));
        }

        #[test]
        fn from_srgba() {
            // full black + opaque
            assert_eq!(C::from(SrgbaTuple(0., 0., 0., 1.)), Color::Rgb(0, 0, 0));
            // full black + transparent
            assert_eq!(C::from(SrgbaTuple(0., 0., 0., 0.)), Color::Rgb(0, 0, 0));

            // full white + opaque
            assert_eq!(C::from(SrgbaTuple(1., 1., 1., 1.)), C::Rgb(255, 255, 255));
            // full white + transparent
            assert_eq!(C::from(SrgbaTuple(1., 1., 1., 0.)), C::Rgb(255, 255, 255));

            // full red
            assert_eq!(C::from(SrgbaTuple(1., 0., 0., 1.)), C::Rgb(255, 0, 0));
            // full green
            assert_eq!(C::from(SrgbaTuple(0., 1., 0., 1.)), C::Rgb(0, 255, 0));
            // full blue
            assert_eq!(C::from(SrgbaTuple(0., 0., 1., 1.)), C::Rgb(0, 0, 255));

            // half red
            assert_eq!(C::from(SrgbaTuple(0.5, 0., 0., 1.)), C::Rgb(127, 0, 0));
            // half green
            assert_eq!(C::from(SrgbaTuple(0., 0.5, 0., 1.)), C::Rgb(0, 127, 0));
            // half blue
            assert_eq!(C::from(SrgbaTuple(0., 0., 0.5, 1.)), C::Rgb(0, 0, 127));
        }

        #[test]
        fn from_rgbcolor() {
            // full black
            assert_eq!(C::from(RgbColor::new_8bpc(0, 0, 0)), Color::Rgb(0, 0, 0));
            // full white
            assert_eq!(
                C::from(RgbColor::new_8bpc(255, 255, 255)),
                C::Rgb(255, 255, 255)
            );

            // full red
            assert_eq!(C::from(RgbColor::new_8bpc(255, 0, 0)), C::Rgb(255, 0, 0));
            // full green
            assert_eq!(C::from(RgbColor::new_8bpc(0, 255, 0)), C::Rgb(0, 255, 0));
            // full blue
            assert_eq!(C::from(RgbColor::new_8bpc(0, 0, 255)), C::Rgb(0, 0, 255));

            // half red
            assert_eq!(C::from(RgbColor::new_8bpc(127, 0, 0)), C::Rgb(127, 0, 0));
            // half green
            assert_eq!(C::from(RgbColor::new_8bpc(0, 127, 0)), C::Rgb(0, 127, 0));
            // half blue
            assert_eq!(C::from(RgbColor::new_8bpc(0, 0, 127)), C::Rgb(0, 0, 127));
        }

        #[test]
        fn from_colorspec() {
            assert_eq!(C::from(ColorSpec::Default), C::Reset);
            assert_eq!(C::from(ColorSpec::PaletteIndex(33)), C::Indexed(33));
            assert_eq!(
                C::from(ColorSpec::TrueColor(SrgbaTuple(0., 0., 0., 1.))),
                C::Rgb(0, 0, 0)
            );
        }

        #[test]
        fn from_colorattribute() {
            assert_eq!(C::from(ColorAttribute::Default), C::Reset);
            assert_eq!(C::from(ColorAttribute::PaletteIndex(32)), C::Indexed(32));
            assert_eq!(
                C::from(ColorAttribute::TrueColorWithDefaultFallback(SrgbaTuple(
                    0., 0., 0., 1.
                ))),
                C::Rgb(0, 0, 0)
            );
            assert_eq!(
                C::from(ColorAttribute::TrueColorWithPaletteFallback(
                    SrgbaTuple(0., 0., 0., 1.),
                    31
                )),
                C::Rgb(0, 0, 0)
            );
        }

        #[test]
        fn from_ansicolor() {
            assert_eq!(C::from(AnsiColor::Black), Color::Black);
            assert_eq!(C::from(AnsiColor::Grey), Color::DarkGray);
            assert_eq!(C::from(AnsiColor::Silver), Color::Gray);
            assert_eq!(C::from(AnsiColor::Maroon), Color::Red);
            assert_eq!(C::from(AnsiColor::Red), Color::LightRed);
            assert_eq!(C::from(AnsiColor::Green), Color::Green);
            assert_eq!(C::from(AnsiColor::Lime), Color::LightGreen);
            assert_eq!(C::from(AnsiColor::Olive), Color::Yellow);
            assert_eq!(C::from(AnsiColor::Yellow), Color::LightYellow);
            assert_eq!(C::from(AnsiColor::Purple), Color::Magenta);
            assert_eq!(C::from(AnsiColor::Fuchsia), Color::LightMagenta);
            assert_eq!(C::from(AnsiColor::Teal), Color::Cyan);
            assert_eq!(C::from(AnsiColor::Aqua), Color::LightCyan);
            assert_eq!(C::from(AnsiColor::White), Color::White);
            assert_eq!(C::from(AnsiColor::Navy), Color::Blue);
            assert_eq!(C::from(AnsiColor::Blue), Color::LightBlue);
        }
    }

    mod into_modifier {
        use super::*;

        #[test]
        fn from_intensity() {
            assert_eq!(Modifier::from(Intensity::Normal), Modifier::empty());
            assert_eq!(Modifier::from(Intensity::Bold), Modifier::BOLD);
            assert_eq!(Modifier::from(Intensity::Half), Modifier::DIM);
        }

        #[test]
        fn from_underline() {
            assert_eq!(Modifier::from(Underline::None), Modifier::empty());
            assert_eq!(Modifier::from(Underline::Single), Modifier::UNDERLINED);
            assert_eq!(Modifier::from(Underline::Double), Modifier::UNDERLINED);
            assert_eq!(Modifier::from(Underline::Curly), Modifier::UNDERLINED);
            assert_eq!(Modifier::from(Underline::Dashed), Modifier::UNDERLINED);
            assert_eq!(Modifier::from(Underline::Dotted), Modifier::UNDERLINED);
        }

        #[test]
        fn from_blink() {
            assert_eq!(Modifier::from(Blink::None), Modifier::empty());
            assert_eq!(Modifier::from(Blink::Slow), Modifier::SLOW_BLINK);
            assert_eq!(Modifier::from(Blink::Rapid), Modifier::RAPID_BLINK);
        }
    }

    #[test]
    fn from_cell_attribute_for_style() {
        // default
        assert_eq!(
            Style::from(CellAttributes::default()),
            Style::new().fg(Color::Reset).bg(Color::Reset)
        );
        // foreground color
        assert_eq!(
            Style::from(
                CellAttributes::default()
                    .set_foreground(ColorAttribute::PaletteIndex(31))
                    .to_owned()
            ),
            Style::new().fg(Color::Indexed(31)).bg(Color::Reset)
        );
        // background color
        assert_eq!(
            Style::from(
                CellAttributes::default()
                    .set_background(ColorAttribute::PaletteIndex(31))
                    .to_owned()
            ),
            Style::new().fg(Color::Reset).bg(Color::Indexed(31))
        );
        // underline color
        #[cfg(feature = "underline_color")]
        assert_eq!(
            Style::from(
                CellAttributes::default()
                    .set_underline_color(AnsiColor::Red)
                    .set
                    .to_owned()
            ),
            Style::new()
                .fg(Color::Reset)
                .bg(Color::Reset)
                .underline_color(Color::Red)
        );
        // underlined
        assert_eq!(
            Style::from(
                CellAttributes::default()
                    .set_underline(Underline::Single)
                    .to_owned()
            ),
            Style::new().fg(Color::Reset).bg(Color::Reset).underlined()
        );
        // blink
        assert_eq!(
            Style::from(CellAttributes::default().set_blink(Blink::Slow).to_owned()),
            Style::new().fg(Color::Reset).bg(Color::Reset).slow_blink()
        );
        // intensity
        assert_eq!(
            Style::from(
                CellAttributes::default()
                    .set_intensity(Intensity::Bold)
                    .to_owned()
            ),
            Style::new().fg(Color::Reset).bg(Color::Reset).bold()
        );
        // italic
        assert_eq!(
            Style::from(CellAttributes::default().set_italic(true).to_owned()),
            Style::new().fg(Color::Reset).bg(Color::Reset).italic()
        );
        // reversed
        assert_eq!(
            Style::from(CellAttributes::default().set_reverse(true).to_owned()),
            Style::new().fg(Color::Reset).bg(Color::Reset).reversed()
        );
        // strikethrough
        assert_eq!(
            Style::from(CellAttributes::default().set_strikethrough(true).to_owned()),
            Style::new().fg(Color::Reset).bg(Color::Reset).crossed_out()
        );
        // hidden
        assert_eq!(
            Style::from(CellAttributes::default().set_invisible(true).to_owned()),
            Style::new().fg(Color::Reset).bg(Color::Reset).hidden()
        );
    }
}
