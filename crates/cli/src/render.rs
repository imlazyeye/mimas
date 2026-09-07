use miette::{GraphicalReportHandler, GraphicalTheme, ThemeCharacters, ThemeStyles};
use owo_colors::OwoColorize;

/// Render any [`miette::Report`] (or other `miette::Diagnostic`) to stderr.
pub fn emit(diag: &dyn miette::Diagnostic, color: bool) {
    use std::io::IsTerminal;
    let no_color = std::env::var_os("NO_COLOR").is_some();
    let use_color = !no_color && (color || std::io::stderr().is_terminal());

    let theme = if use_color {
        let mut characters = ThemeCharacters::unicode();
        characters.error = "error: ".into();
        characters.warning = "warning: ".into();

        characters.advice = "advice: ".into();

        let styles = ThemeStyles {
            error: owo_colors::Style::new().bright_red().bold(),
            warning: owo_colors::Style::new().bright_yellow().bold(),
            advice: owo_colors::Style::new().bright_cyan().bold(),
            help: owo_colors::Style::new().cyan().italic(),
            link: owo_colors::Style::new().cyan().underline(),
            linum: owo_colors::Style::new().cyan().dimmed(),
            highlights: vec![
                owo_colors::Style::new().bright_yellow().bold(),
                owo_colors::Style::new().bright_green().bold(),
                owo_colors::Style::new().bright_magenta().bold(),
            ],
        };

        GraphicalTheme { characters, styles }
    } else {
        GraphicalTheme::none()
    };

    let handler = GraphicalReportHandler::new()
        .with_show_related_as_nested(true)
        .with_theme(theme);
    let mut buf = String::new();
    handler.render_report(&mut buf, diag).unwrap();

    eprintln!("{buf}");

    if buf.contains(shared::INTERNAL_TY_MARKER) {
        let note = "note: the `?mimas<...>` shown above is an unresolved internal type -- mimas should have resolved it before showing this error, so its presence here is a bug on our end. please report this case.";
        if use_color {
            eprintln!("{}", note.italic().bright_black());
        } else {
            eprintln!("{note}");
        }
    }
}
