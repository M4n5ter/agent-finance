use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::config::LayoutConfig;
use crate::layout::{self, CockpitLayout};
use crate::state::{AppState, FloatingKind, Panel, TaskLevel};

pub fn render(frame: &mut Frame<'_>, state: &AppState, layout_config: &LayoutConfig) {
    let layout = layout::build(frame.area(), layout_config, &state.floating);
    render_docked(frame, state, &layout);
    render_status(frame, state, layout.status);
    for floating in &layout.floating {
        frame.render_widget(Clear, floating.rect);
        render_floating(frame, state, floating.kind, floating.rect);
    }
}

fn render_docked(frame: &mut Frame<'_>, state: &AppState, layout: &CockpitLayout) {
    render_watchlist(frame, state, layout.panel_rect(Panel::Watchlist));
    render_quote(frame, state, layout.panel_rect(Panel::Quote));
    render_history(frame, layout.panel_rect(Panel::History));
    render_evidence(frame, layout.panel_rect(Panel::Evidence));
    render_research(frame, layout.panel_rect(Panel::Research));
    render_provider_health(frame, state, layout.panel_rect(Panel::ProviderHealth));
    render_task_log(frame, state, layout.panel_rect(Panel::TaskLog));
}

fn render_watchlist(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let items = state
        .watchlist
        .iter()
        .enumerate()
        .map(|(index, symbol)| {
            let marker = if index == state.selected_symbol {
                ">"
            } else {
                " "
            };
            let style = if index == state.selected_symbol {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, style),
                Span::raw(" "),
                Span::styled(symbol.clone(), style),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(panel_block(Panel::Watchlist, state)),
        area,
    );
}

fn render_quote(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let symbol = state.selected_symbol().unwrap_or("N/A");
    let text = vec![
        Line::from(vec![
            Span::styled(
                symbol,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" public market snapshot"),
        ]),
        Line::from("Price/session data tasks will attach here."),
        Line::from("Freshness, provider, partial errors, and proxy evidence stay visible."),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .block(panel_block(Panel::Quote, state))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_history(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("History chart placeholder\nDaily and intraday OHLCV will render here.")
            .block(simple_block(Panel::History.title()))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_evidence(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("Crypto evidence placeholder\nQuote/book/trades/candles/funding panels share market service data.")
            .block(simple_block(Panel::Evidence.title()))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_research(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("News, research highlights, SEC and provider facts will appear here.")
            .block(simple_block(Panel::Research.title()))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_provider_health(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let items = state
        .provider_profiles
        .iter()
        .take(8)
        .map(|profile| {
            ListItem::new(Line::from(vec![
                Span::styled(profile.provider.clone(), Style::default().fg(Color::Green)),
                Span::raw(" "),
                Span::raw(profile.best_for.clone()),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(simple_block(Panel::ProviderHealth.title())),
        area,
    );
}

fn render_task_log(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let items = state
        .task_log
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .map(|entry| {
            let style = match entry.level {
                TaskLevel::Info => Style::default().fg(Color::Gray),
            };
            ListItem::new(Line::from(Span::styled(entry.message.clone(), style)))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(simple_block(Panel::TaskLog.title())),
        area,
    );
}

fn render_status(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let symbol = state.selected_symbol().unwrap_or("N/A");
    let text = format!(
        " {} | focus: {} | j/k symbol | h help | : command | p providers | q quit ",
        symbol,
        state.focused_panel.title()
    );
    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(Color::DarkGray).fg(Color::White)),
        area,
    );
}

fn render_floating(frame: &mut Frame<'_>, state: &AppState, kind: FloatingKind, area: Rect) {
    let text = match kind {
        FloatingKind::CommandPalette => vec![
            Line::from("Type-to-filter commands will land here."),
            Line::from("Current actions: h help, p providers, Esc close, q quit."),
        ],
        FloatingKind::Help => vec![
            Line::from("agent-finance cockpit"),
            Line::from("j/k or arrows: switch selected symbol"),
            Line::from(": open command palette"),
            Line::from("p inspect provider details"),
            Line::from("r reset layout"),
            Line::from("q quit"),
        ],
        FloatingKind::ProviderDetails => state
            .provider_profiles
            .iter()
            .take(12)
            .map(|profile| {
                Line::from(format!(
                    "{}: {}",
                    profile.provider,
                    profile
                        .capabilities
                        .iter()
                        .filter(|capability| capability.implemented)
                        .map(|capability| format!("{}:{}", capability.module, capability.status))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
            .collect(),
    };
    frame.render_widget(
        Paragraph::new(text)
            .block(simple_block(kind.title()))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn panel_block(panel: Panel, state: &AppState) -> Block<'static> {
    let style = if state.focused_panel == panel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };
    simple_block(panel.title()).border_style(style)
}

fn simple_block(title: &'static str) -> Block<'static> {
    Block::default().title(title).borders(Borders::ALL)
}
