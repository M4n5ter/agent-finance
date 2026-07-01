use ratatui::buffer::Buffer;
use ratatui::style::Style;

use crate::history_chart::{CandleWickStyle, PricePoint};

#[derive(Debug, Clone, Copy)]
pub(super) struct CandleShape {
    pub high: PricePoint,
    pub low: PricePoint,
    pub open: PricePoint,
    pub close: PricePoint,
}

pub(super) fn render_split_candle(
    buffer: &mut Buffer,
    wick_x: u16,
    body_x: u16,
    shape: CandleShape,
    wick_style: CandleWickStyle,
    style: Style,
) {
    match wick_style {
        CandleWickStyle::StableCell => {
            render_cell_segment(buffer, wick_x, shape.high, shape.low, "│", style);
            render_vertical_segment(buffer, body_x, shape.open, shape.close, body_symbol, style);
        }
        CandleWickStyle::Subcell => {
            render_vertical_segment(buffer, wick_x, shape.high, shape.low, wick_symbol, style);
            render_vertical_segment(buffer, body_x, shape.open, shape.close, body_symbol, style);
        }
    }
}

pub(super) fn render_dense_candle(
    buffer: &mut Buffer,
    x: u16,
    shape: CandleShape,
    wick_style: CandleWickStyle,
    style: Style,
) {
    if wick_style == CandleWickStyle::StableCell {
        render_readable_dense_candle(buffer, x, shape, style);
        return;
    }
    let wick = SegmentSlots::between(shape.high, shape.low);
    let body = SegmentSlots::between(shape.open, shape.close);
    let top_row = shape
        .high
        .row
        .min(shape.low.row)
        .min(shape.open.row)
        .min(shape.close.row);
    let bottom_row = shape
        .high
        .row
        .max(shape.low.row)
        .max(shape.open.row)
        .max(shape.close.row);
    for row in top_row..=bottom_row {
        let mask = braille_mask(row, wick, body);
        if mask != 0 {
            buffer.set_string(x, row, braille_symbol(mask), style);
        }
    }
}

pub(super) fn render_close_only_candle(
    buffer: &mut Buffer,
    wick_x: u16,
    marker_x: u16,
    shape: CandleShape,
    candle_width: u16,
    wick_style: CandleWickStyle,
    style: Style,
) {
    if wick_style == CandleWickStyle::StableCell {
        render_cell_segment(buffer, wick_x, shape.high, shape.low, "│", style);
    } else {
        render_vertical_segment(buffer, wick_x, shape.high, shape.low, wick_symbol, style);
    }
    buffer.set_string(
        marker_x,
        shape.close.row,
        close_only_symbol(candle_width),
        style,
    );
}

pub(super) fn close_only_symbol(candle_width: u16) -> &'static str {
    if candle_width > 1 { "◆" } else { "•" }
}

pub(super) fn volume_symbol(eighths: u8) -> &'static str {
    const SYMBOLS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    SYMBOLS[usize::from(eighths.min(8))]
}

fn render_cell_segment(
    buffer: &mut Buffer,
    x: u16,
    start: PricePoint,
    end: PricePoint,
    symbol: &str,
    style: Style,
) {
    let top_row = start.row.min(end.row);
    let bottom_row = start.row.max(end.row);
    for row in top_row..=bottom_row {
        buffer.set_string(x, row, symbol, style);
    }
}

fn render_readable_dense_candle(buffer: &mut Buffer, x: u16, shape: CandleShape, style: Style) {
    render_cell_segment(buffer, x, shape.high, shape.low, "│", style);
    let body = SegmentSlots::between(shape.open, shape.close);
    for row in shape.open.row.min(shape.close.row)..=shape.open.row.max(shape.close.row) {
        let row_top = u32::from(row) * 4;
        let mask = (0..4).fold(0u8, |mask, slot| {
            if body.contains(row_top + slot) {
                mask | (1 << slot)
            } else {
                mask
            }
        });
        if mask != 0 {
            buffer.set_string(x, row, body_symbol(mask), style);
        }
    }
}

fn render_vertical_segment(
    buffer: &mut Buffer,
    x: u16,
    start: PricePoint,
    end: PricePoint,
    symbol: fn(u8) -> &'static str,
    style: Style,
) {
    let top_slot = start.slot().min(end.slot());
    let bottom_slot = start.slot().max(end.slot());
    let top_row = start.row.min(end.row);
    let bottom_row = start.row.max(end.row);
    for row in top_row..=bottom_row {
        let row_top = u32::from(row) * 4;
        let mask = (0..4).fold(0u8, |mask, slot| {
            if (top_slot..=bottom_slot).contains(&(row_top + slot)) {
                mask | (1 << slot)
            } else {
                mask
            }
        });
        if mask != 0 {
            buffer.set_string(x, row, symbol(mask), style);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SegmentSlots {
    top: u32,
    bottom: u32,
}

impl SegmentSlots {
    fn between(start: PricePoint, end: PricePoint) -> Self {
        let start = start.slot();
        let end = end.slot();
        Self {
            top: start.min(end),
            bottom: start.max(end),
        }
    }

    fn contains(self, slot: u32) -> bool {
        (self.top..=self.bottom).contains(&slot)
    }
}

fn braille_mask(row: u16, wick: SegmentSlots, body: SegmentSlots) -> u8 {
    let row_top = u32::from(row) * 4;
    (0..4).fold(0u8, |mask, slot| {
        let absolute = row_top + slot;
        let wick_mask = if wick.contains(absolute) {
            left_braille_bit(slot)
        } else {
            0
        };
        let body_mask = if body.contains(absolute) {
            right_braille_bit(slot)
        } else {
            0
        };
        mask | wick_mask | body_mask
    })
}

fn braille_symbol(mask: u8) -> String {
    char::from_u32(0x2800 + u32::from(mask))
        .expect("braille mask is always a valid Unicode scalar")
        .to_string()
}

const fn left_braille_bit(slot: u32) -> u8 {
    match slot {
        0 => 0b0000_0001,
        1 => 0b0000_0010,
        2 => 0b0000_0100,
        3 => 0b0100_0000,
        _ => 0,
    }
}

const fn right_braille_bit(slot: u32) -> u8 {
    match slot {
        0 => 0b0000_1000,
        1 => 0b0001_0000,
        2 => 0b0010_0000,
        3 => 0b1000_0000,
        _ => 0,
    }
}

fn body_symbol(mask: u8) -> &'static str {
    match mask {
        0b0001 => "▔",
        0b0010 | 0b0100 | 0b0110 => "━",
        0b1000 => "▁",
        0b0011 | 0b0111 => "▀",
        0b1100 | 0b1110 => "▄",
        0b1111 => "█",
        _ => "█",
    }
}

fn wick_symbol(mask: u8) -> &'static str {
    const SYMBOLS: [&str; 16] = [
        " ", "⠁", "⠂", "⠃", "⠄", "⠅", "⠆", "⠇", "⡀", "⡁", "⡂", "⡃", "⡄", "⡅", "⡆", "⡇",
    ];
    SYMBOLS[usize::from(mask & 0b1111)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_chart::PriceBounds;
    use ratatui::layout::Rect;

    #[test]
    fn split_candle_separates_spike_wick_from_small_body() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 4));

        render_split_candle(
            &mut buffer,
            2,
            3,
            candle_shape(100.0, 110.0, 90.0, 100.5),
            CandleWickStyle::Subcell,
            Style::default(),
        );

        assert_eq!(buffer[(2, 0)].symbol(), "⡇");
        assert_eq!(buffer[(2, 1)].symbol(), "⡇");
        assert_eq!(buffer[(2, 2)].symbol(), "⡇");
        assert_eq!(buffer[(2, 3)].symbol(), "⡇");
        assert_eq!(buffer[(3, 1)].symbol(), "▁");
        assert_eq!(buffer[(3, 2)].symbol(), "▔");
    }

    #[test]
    fn readable_split_candle_uses_stable_wick_without_losing_body_shape() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 4));

        render_split_candle(
            &mut buffer,
            2,
            3,
            candle_shape(100.0, 110.0, 90.0, 100.5),
            CandleWickStyle::StableCell,
            Style::default(),
        );

        assert_eq!(buffer[(2, 0)].symbol(), "│");
        assert_eq!(buffer[(2, 1)].symbol(), "│");
        assert_eq!(buffer[(2, 2)].symbol(), "│");
        assert_eq!(buffer[(2, 3)].symbol(), "│");
        assert_eq!(buffer[(3, 1)].symbol(), "▁");
        assert_eq!(buffer[(3, 2)].symbol(), "▔");
    }

    #[test]
    fn dense_candle_combines_wick_and_body_in_one_cell() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 4));

        render_dense_candle(
            &mut buffer,
            2,
            candle_shape(100.0, 110.0, 90.0, 105.0),
            CandleWickStyle::Subcell,
            Style::default(),
        );

        assert_eq!(buffer[(2, 0)].symbol(), "⡇");
        assert_eq!(buffer[(2, 1)].symbol(), "⣿");
        assert_eq!(buffer[(2, 2)].symbol(), "⡏");
        assert_eq!(buffer[(2, 3)].symbol(), "⡇");
    }

    #[test]
    fn close_only_candle_keeps_intrabar_range_visible() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 4));

        render_close_only_candle(
            &mut buffer,
            2,
            3,
            candle_shape(100.0, 110.0, 90.0, 100.0),
            2,
            CandleWickStyle::Subcell,
            Style::default(),
        );

        assert_eq!(buffer[(2, 0)].symbol(), "⡇");
        assert_eq!(buffer[(2, 3)].symbol(), "⡇");
        assert_eq!(buffer[(3, 2)].symbol(), "◆");
    }

    fn candle_shape(open: f64, high: f64, low: f64, close: f64) -> CandleShape {
        let bounds = PriceBounds::new(90.0, 110.0);
        let area = Rect::new(0, 0, 4, 4);
        CandleShape {
            high: bounds.point(area, high),
            low: bounds.point(area, low),
            open: bounds.point(area, open),
            close: bounds.point(area, close),
        }
    }
}
