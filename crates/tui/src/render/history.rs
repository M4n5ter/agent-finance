use ratatui::style::{Color, Style};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType};

pub(super) fn chart(points: &[(f64, f64)]) -> Chart<'_> {
    let bounds = chart_bounds(points);
    let dataset = Dataset::default()
        .name("close")
        .marker(ratatui::symbols::Marker::Braille)
        .graph_type(GraphType::Area)
        .style(Style::default().fg(Color::Green))
        .fill_to_y(bounds.y[0])
        .data(points);
    Chart::new(vec![dataset])
        .x_axis(Axis::default().bounds(bounds.x))
        .y_axis(Axis::default().bounds(bounds.y))
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ChartBounds {
    x: [f64; 2],
    y: [f64; 2],
}

pub(super) fn chart_points(closes: &[f64]) -> Vec<(f64, f64)> {
    closes
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, close)| close.is_finite())
        .map(|(index, close)| (index as f64, close))
        .collect()
}

fn chart_bounds(points: &[(f64, f64)]) -> ChartBounds {
    if points.is_empty() {
        return ChartBounds {
            x: [0.0, 1.0],
            y: [0.0, 1.0],
        };
    }
    let max_x = points.last().map(|(x, _)| *x).unwrap_or(1.0).max(1.0);
    let (min_y, max_y) = points.iter().fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(min_y, max_y), (_, y)| (min_y.min(*y), max_y.max(*y)),
    );
    let price_scale = min_y.abs().max(max_y.abs()).max(f64::MIN_POSITIVE);
    let padding = ((max_y - min_y).abs() * 0.05).max(price_scale * 0.001);
    let y_min = min_y - padding;
    let y_max = max_y + padding;
    let y = if min_y >= 0.0 && y_min < 0.0 {
        [0.0, y_max]
    } else if max_y <= 0.0 && y_max > 0.0 {
        [y_min, 0.0]
    } else {
        [y_min, y_max]
    };
    ChartBounds { x: [0.0, max_x], y }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_points_skip_bad_values_and_bounds_close_range() {
        let points = chart_points(&[10.0, f64::NAN, 15.0, 20.0]);
        assert_eq!(points, vec![(0.0, 10.0), (2.0, 15.0), (3.0, 20.0)]);

        let bounds = chart_bounds(&points);
        assert_eq!(bounds.x, [0.0, 3.0]);
        assert_eq!(bounds.y, [9.5, 20.5]);

        let flat_bounds = chart_bounds(&chart_points(&[10.0, 10.0]));
        assert_eq!(flat_bounds.y, [9.99, 10.01]);

        let micro_bounds = chart_bounds(&chart_points(&[0.000010, 0.000020]));
        assert_eq!(micro_bounds.y, [0.0000095, 0.0000205]);
    }
}
