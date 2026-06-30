use agent_finance_market::history_snapshot::HistoryBarSnapshot;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::chart::series::{CandleBucket, compressed_bars, moving_average, vwap};
use crate::theme::ThemeConfig;

pub(super) fn chart<'a>(
    bars: &'a [HistoryBarSnapshot],
    theme: &'a ThemeConfig,
) -> CandlestickChart<'a> {
    CandlestickChart { bars, theme }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CandlestickChart<'a> {
    bars: &'a [HistoryBarSnapshot],
    theme: &'a ThemeConfig,
}

impl Widget for CandlestickChart<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.width < 8 || area.height < 4 {
            return;
        }
        let buckets = compressed_bars(self.bars, area.width as usize);
        if buckets.is_empty() {
            return;
        }

        let axis_height = 1;
        let volume_height = volume_height(area.height);
        let price_height = area
            .height
            .saturating_sub(volume_height)
            .saturating_sub(axis_height);
        let price_area = Rect {
            height: price_height,
            ..area
        };
        let volume_area = Rect {
            y: area.y + price_height,
            height: volume_height,
            ..area
        };
        let time_area = Rect {
            y: area.y + price_height + volume_height,
            height: axis_height,
            ..area
        };
        let bounds = PriceBounds::from_buckets(&buckets);
        render_current_price_line(buffer, price_area, bounds, &buckets, self.theme);
        render_overlays(buffer, price_area, bounds, &buckets, self.theme);
        render_candles(buffer, price_area, bounds, &buckets, self.theme);
        render_volume(buffer, volume_area, &buckets, self.theme);
        render_price_labels(buffer, price_area, bounds, self.theme);
        render_time_labels(buffer, time_area, &buckets, self.theme);
    }
}

#[derive(Debug, Clone, Copy)]
struct PriceBounds {
    min: f64,
    max: f64,
}

impl PriceBounds {
    fn from_buckets(buckets: &[CandleBucket]) -> Self {
        let (min, max) = buckets
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), bucket| {
                (min.min(bucket.low), max.max(bucket.high))
            });
        let scale = min.abs().max(max.abs()).max(f64::MIN_POSITIVE);
        let padding = ((max - min).abs() * 0.05).max(scale * 0.001);
        Self {
            min: min - padding,
            max: max + padding,
        }
    }

    fn row(self, area: Rect, price: f64) -> u16 {
        if area.height <= 1 || (self.max - self.min).abs() <= f64::EPSILON {
            return area.y;
        }
        let ratio = ((price - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        area.y + area.height - 1 - (ratio * f64::from(area.height - 1)).round() as u16
    }

    fn subrow(self, area: Rect, price: f64) -> u32 {
        if area.height <= 1 || (self.max - self.min).abs() <= f64::EPSILON {
            return 0;
        }
        let subrows = u32::from(area.height) * 4;
        let ratio = ((price - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        subrows - 1 - (ratio * f64::from(subrows - 1)).round() as u32
    }
}

fn volume_height(height: u16) -> u16 {
    match height {
        0..=7 => 0,
        8..=12 => 2,
        _ => (height / 5).clamp(3, 6),
    }
}

fn render_candles(
    buffer: &mut Buffer,
    area: Rect,
    bounds: PriceBounds,
    buckets: &[CandleBucket],
    theme: &ThemeConfig,
) {
    for (index, bucket) in buckets.iter().enumerate().take(area.width as usize) {
        let x = area.x + index as u16;
        let style = candle_style(bucket, theme);
        let high = bounds.row(area, bucket.high);
        let low = bounds.row(area, bucket.low);
        for row in high.min(low)..=high.max(low) {
            if let Some(symbol) = braille_candle_symbol(bucket, bounds, area, row) {
                buffer.set_string(x, row, symbol.as_str(), style);
            }
        }
    }
}

fn render_current_price_line(
    buffer: &mut Buffer,
    area: Rect,
    bounds: PriceBounds,
    buckets: &[CandleBucket],
    theme: &ThemeConfig,
) {
    let Some(last) = buckets.last() else {
        return;
    };
    let row = bounds.row(area, last.close);
    for x in area.x..area.x + area.width {
        if buffer[(x, row)].symbol() == " " {
            buffer.set_string(x, row, "·", theme.neutral_style());
        }
    }
}

fn render_overlays(
    buffer: &mut Buffer,
    area: Rect,
    bounds: PriceBounds,
    buckets: &[CandleBucket],
    theme: &ThemeConfig,
) {
    render_series(
        buffer,
        area,
        bounds,
        &moving_average(buckets, 20),
        "∙",
        theme.accent_style(),
    );
    render_series(
        buffer,
        area,
        bounds,
        &moving_average(buckets, 50),
        "·",
        theme.warning_style(),
    );
    render_series(
        buffer,
        area,
        bounds,
        &vwap(buckets),
        "×",
        theme.prediction_style(),
    );
}

fn render_series(
    buffer: &mut Buffer,
    area: Rect,
    bounds: PriceBounds,
    series: &[Option<f64>],
    marker: &str,
    style: Style,
) {
    for (index, value) in series.iter().enumerate().take(area.width as usize) {
        let Some(value) = value else {
            continue;
        };
        let row = bounds.row(area, *value);
        buffer.set_string(area.x + index as u16, row, marker, style);
    }
}

fn render_volume(buffer: &mut Buffer, area: Rect, buckets: &[CandleBucket], theme: &ThemeConfig) {
    if area.height == 0 {
        return;
    }
    let max_volume = buckets
        .iter()
        .filter_map(|bucket| bucket.volume)
        .fold(0.0, f64::max);
    if max_volume <= 0.0 {
        return;
    }
    for (index, bucket) in buckets.iter().enumerate().take(area.width as usize) {
        let Some(volume) = bucket.volume else {
            continue;
        };
        let height = ((volume / max_volume) * f64::from(area.height)).ceil() as u16;
        let style = candle_style(bucket, theme);
        for offset in 0..height.max(1).min(area.height) {
            buffer.set_string(
                area.x + index as u16,
                area.y + area.height - 1 - offset,
                "█",
                style,
            );
        }
    }
}

fn render_price_labels(buffer: &mut Buffer, area: Rect, bounds: PriceBounds, theme: &ThemeConfig) {
    let high = format!("{:.2}", bounds.max);
    let low = format!("{:.2}", bounds.min);
    write_right(
        buffer,
        area.x,
        area.y,
        area.width,
        &high,
        theme.muted_style(),
    );
    write_right(
        buffer,
        area.x,
        area.y + area.height.saturating_sub(1),
        area.width,
        &low,
        theme.muted_style(),
    );
}

fn render_time_labels(
    buffer: &mut Buffer,
    area: Rect,
    buckets: &[CandleBucket],
    theme: &ThemeConfig,
) {
    if let Some(first) = buckets.first() {
        write_left_clipped(buffer, area, &first.open_time, theme.muted_style());
    }
    if let Some(last) = buckets.last() {
        write_right_clipped(buffer, area, &last.open_time, theme.muted_style());
    }
}

fn write_right(buffer: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    write_right_clipped(
        buffer,
        Rect {
            x,
            y,
            width,
            height: 1,
        },
        text,
        style,
    );
}

fn write_left_clipped(buffer: &mut Buffer, area: Rect, text: &str, style: Style) {
    let text = clipped_prefix(text, area.width as usize);
    buffer.set_string(area.x, area.y, text, style);
}

fn write_right_clipped(buffer: &mut Buffer, area: Rect, text: &str, style: Style) {
    let text = clipped_suffix(text, area.width as usize);
    let start = area.x + area.width.saturating_sub(text.chars().count() as u16);
    buffer.set_string(start, area.y, text, style);
}

fn candle_style(bucket: &CandleBucket, theme: &ThemeConfig) -> Style {
    if bucket.close > bucket.open {
        theme.success_style()
    } else if bucket.close < bucket.open {
        theme.danger_style()
    } else {
        theme.neutral_style()
    }
}

fn braille_candle_symbol(
    bucket: &CandleBucket,
    bounds: PriceBounds,
    area: Rect,
    row: u16,
) -> Option<String> {
    let bits = braille_candle_bits(bucket, bounds, area, row);
    (bits != 0).then(|| {
        char::from_u32(0x2800 + u32::from(bits))
            .unwrap_or(' ')
            .to_string()
    })
}

fn braille_candle_bits(bucket: &CandleBucket, bounds: PriceBounds, area: Rect, row: u16) -> u8 {
    let high = bounds.subrow(area, bucket.high);
    let low = bounds.subrow(area, bucket.low);
    let open = bounds.subrow(area, bucket.open);
    let close = bounds.subrow(area, bucket.close);
    let wick = high.min(low)..=high.max(low);
    let body = open.min(close)..=open.max(close);
    let row_base = u32::from(row.saturating_sub(area.y)) * 4;
    let mut bits = 0;

    for offset in 0..4 {
        let subrow = row_base + offset;
        if wick.contains(&subrow) && (bucket.close_only || !body.contains(&subrow)) {
            bits |= braille_dot(offset, BrailleLane::Left);
        }
        if bucket.close_only {
            if subrow == close {
                bits |= braille_dot(offset, BrailleLane::Left);
                bits |= braille_dot(offset, BrailleLane::Right);
            }
        } else if body.contains(&subrow) {
            bits |= braille_dot(offset, BrailleLane::Right);
        }
    }

    bits
}

#[derive(Debug, Clone, Copy)]
enum BrailleLane {
    Left,
    Right,
}

fn braille_dot(offset: u32, lane: BrailleLane) -> u8 {
    match (lane, offset) {
        (BrailleLane::Left, 0) => 0x01,
        (BrailleLane::Left, 1) => 0x02,
        (BrailleLane::Left, 2) => 0x04,
        (BrailleLane::Left, 3) => 0x40,
        (BrailleLane::Right, 0) => 0x08,
        (BrailleLane::Right, 1) => 0x10,
        (BrailleLane::Right, 2) => 0x20,
        (BrailleLane::Right, 3) => 0x80,
        _ => 0,
    }
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

fn clipped_suffix(text: &str, max_chars: usize) -> &str {
    if max_chars == 0 {
        return "";
    }
    let len = text.chars().count();
    if len <= max_chars {
        return text;
    }
    let skip = len - max_chars;
    text.char_indices()
        .nth(skip)
        .map_or(text, |(index, _)| &text[index..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_writes_are_clipped_to_their_rect() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 2));
        write_left_clipped(
            &mut buffer,
            Rect::new(0, 0, 4, 1),
            "2026-06-30T09:30:00+08:00",
            Style::default(),
        );
        write_right_clipped(
            &mut buffer,
            Rect::new(0, 1, 4, 1),
            "2026-06-30T16:00:00+08:00",
            Style::default(),
        );

        assert_eq!(row_text(&buffer, 0), "2026  ");
        assert_eq!(row_text(&buffer, 1), "8:00  ");
    }

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
    }

    #[test]
    fn braille_candles_use_subcell_price_precision() {
        let bounds = PriceBounds {
            min: 0.0,
            max: 16.0,
        };
        let area = Rect::new(0, 0, 1, 4);
        let bucket = CandleBucket {
            open_time: "t".to_string(),
            open: 4.0,
            high: 16.0,
            low: 0.0,
            close: 12.0,
            volume: None,
            close_only: false,
        };

        let rows = (0..4)
            .map(|row| braille_candle_bits(&bucket, bounds, area, row))
            .collect::<Vec<_>>();

        assert!(rows.iter().all(|bits| *bits != 0));
        assert!(rows.iter().any(|bits| bits & 0x01 != 0));
        assert!(rows.iter().any(|bits| bits & 0x08 != 0));
    }
}
