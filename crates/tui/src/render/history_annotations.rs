use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use crate::history_chart::ChartWarning;
use crate::theme::ThemeConfig;

use super::history::ChartMode;

pub(super) fn render_warning_band(
    buffer: &mut Buffer,
    area: Rect,
    mode: ChartMode,
    warnings: &[ChartWarning],
    theme: &ThemeConfig,
) {
    let Some(first) = warnings.first() else {
        return;
    };
    if area.width < 24 || area.height < 4 {
        return;
    }
    let y = if mode == ChartMode::Workbench && area.width >= 72 && area.height >= 12 {
        area.y + 2
    } else {
        area.y
    };
    let suffix = if warnings.len() > 1 {
        format!(" +{} more", warnings.len() - 1)
    } else {
        String::new()
    };
    let line = format!("warning: {}{suffix}", first.message);
    let width = line.chars().count().min(area.width as usize);
    let text = clipped_prefix(&line, width);
    buffer.set_string(
        area.x,
        y,
        format!("{text:<width$}"),
        theme.warning_style().add_modifier(Modifier::BOLD),
    );
}

fn clipped_prefix(text: &str, max_chars: usize) -> &str {
    if max_chars == 0 {
        return "";
    }
    if text.chars().count() <= max_chars {
        return text;
    }
    text.char_indices()
        .nth(max_chars)
        .map_or(text, |(index, _)| &text[..index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_chart::ChartWarningKind;

    #[test]
    fn warning_band_summarizes_chart_data_quality() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 48, 6));
        let warnings = vec![
            ChartWarning {
                kind: ChartWarningKind::CloseOnly,
                message: "close-only bars=3".to_string(),
            },
            ChartWarning {
                kind: ChartWarningKind::Provider,
                message: "provider fallback failed".to_string(),
            },
        ];

        render_warning_band(
            &mut buffer,
            Rect::new(0, 0, 48, 6),
            ChartMode::Cockpit,
            &warnings,
            &ThemeConfig::default(),
        );

        assert!(row_text(&buffer, 0).starts_with("warning: close-only bars=3 +1 more"));
        assert!(buffer[(0, 0)].style().add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn warning_band_leaves_small_areas_empty() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 12, 3));
        let warnings = vec![ChartWarning {
            kind: ChartWarningKind::Provider,
            message: "fallback failed".to_string(),
        }];

        render_warning_band(
            &mut buffer,
            Rect::new(0, 0, 12, 3),
            ChartMode::Cockpit,
            &warnings,
            &ThemeConfig::default(),
        );

        assert!(buffer.content().iter().all(|cell| cell.symbol() == " "));
    }

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    }
}
