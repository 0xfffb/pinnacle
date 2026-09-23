use std::io::{self, Write};

use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::{Terminal, TerminalOptions, Viewport};

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn ok() -> Style {
    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
}

fn warn() -> Style {
    Style::default().fg(Color::Yellow)
}

fn err() -> Style {
    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
}

fn accent() -> Style {
    Style::default().fg(Color::Cyan)
}

fn render(height: u16, draw: impl FnOnce(&mut ratatui::Frame)) -> io::Result<()> {
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(io::stdout()),
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    )?;
    terminal.draw(draw)?;
    let mut out = io::stdout();
    writeln!(out)?;
    out.flush()
}

fn table<'a>(title: &'a str, rows: Vec<Row<'a>>) -> Table<'a> {
    Table::new(rows, [Constraint::Length(16), Constraint::Min(20)])
        .block(Block::default().borders(Borders::ALL).title(title))
}

fn kv(k: &str, v: impl Into<String>, style: Style) -> Row<'_> {
    Row::new([
        Cell::from(k.to_owned()).style(dim()),
        Cell::from(v.into()).style(style),
    ])
}

pub fn stats() -> io::Result<()> {
    render(9, |f| {
        let rows = vec![
            kv("qps", "47.2", accent()),
            kv("allowed", "1842", ok()),
            kv("challenge", "311", warn()),
            kv("banned", "14", err()),
            kv("upstream err", "2", err()),
            kv("p99", "21 ms", accent()),
        ];
        f.render_widget(table(" stats (mock) ", rows), f.area());
    })
}

pub fn status() -> io::Result<()> {
    render(8, |f| {
        let rows = vec![
            kv("gateway", "running", ok()),
            kv("listen", "0.0.0.0:6188", accent()),
            kv("upstream", "127.0.0.1:8080", ok()),
            kv("challenge", "/7kkfg0kdt80b9a4eg41b8sz9rdwndbx", accent()),
            kv("uptime", "12m 04s", Style::default()),
        ];
        f.render_widget(table(" status (mock) ", rows), f.area());
    })
}

pub fn health() -> io::Result<()> {
    render(7, |f| {
        let rows = vec![
            kv("gateway", "ok", ok()),
            kv("upstream", "ok  18 ms", ok()),
            kv("store", "memory  ok", ok()),
            kv("control", "not connected", warn()),
        ];
        f.render_widget(table(" health (mock) ", rows), f.area());
    })
}

pub fn bans() -> io::Result<()> {
    render(8, |f| {
        let header = Row::new(["ip", "reason", "ttl"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let rows = [
            ("203.0.113.14", "challenge_failed", "23m"),
            ("198.51.100.7", "manual", "permanent"),
            ("192.0.2.88", "store_banned", "4m"),
        ]
        .into_iter()
        .map(|(ip, reason, ttl)| {
            Row::new([
                Cell::from(ip).style(err()),
                Cell::from(reason),
                Cell::from(ttl).style(dim()),
            ])
        });
        let table = Table::new(
            rows,
            [
                Constraint::Length(16),
                Constraint::Length(20),
                Constraint::Min(10),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" bans (mock) "));
        f.render_widget(table, f.area());
    })
}

pub fn verify() -> io::Result<()> {
    render(8, |f| {
        let rows = vec![
            kv("issued", "128", accent()),
            kv("passed", "109", ok()),
            kv("failed", "19", err()),
            kv("pass rate", "85.2%", ok()),
            kv("active cid", "6", warn()),
        ];
        f.render_widget(table(" verify (mock) ", rows), f.area());
    })
}

pub fn inspect(ip: &str) -> io::Result<()> {
    let ip = ip.to_owned();
    render(9, |f| {
        let rows = vec![
            kv("ip", ip.clone(), accent()),
            kv("pass", "valid  41m left", ok()),
            kv("banned", "no", ok()),
            kv("challenges", "3", warn()),
            kv("last seen", "12s ago  GET /docs", Style::default()),
            kv("ua", "Mozilla/5.0 … Chrome/129", dim()),
        ];
        f.render_widget(table(" inspect (mock) ", rows), f.area());
    })
}

pub fn ban(ip: &str, reason: &str) -> io::Result<()> {
    let line = format!(" banned  {ip}  ({reason})");
    render(3, |f| {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(line, err())))
                .block(Block::default().borders(Borders::ALL).title(" ban (mock) ")),
            f.area(),
        );
    })
}

pub fn unban(ip: &str) -> io::Result<()> {
    let line = format!(" unbanned  {ip}");
    render(3, |f| {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(line, ok())))
                .block(Block::default().borders(Borders::ALL).title(" unban (mock) ")),
            f.area(),
        );
    })
}

pub fn logs() -> io::Result<()> {
    render(10, |f| {
        let lines = vec![
            Line::from(vec![
                Span::styled("12:01:04  ", dim()),
                Span::styled("CHALLENGE  ", warn()),
                Span::raw("203.0.113.14  GET /"),
            ]),
            Line::from(vec![
                Span::styled("12:01:05  ", dim()),
                Span::styled("VERIFY     ", accent()),
                Span::raw("203.0.113.14  pass issued"),
            ]),
            Line::from(vec![
                Span::styled("12:01:06  ", dim()),
                Span::styled("ALLOW      ", ok()),
                Span::raw("203.0.113.14  GET /docs"),
            ]),
            Line::from(vec![
                Span::styled("12:01:11  ", dim()),
                Span::styled("BAN        ", err()),
                Span::raw("198.51.100.7  challenge_failed"),
            ]),
            Line::from(vec![
                Span::styled("12:01:18  ", dim()),
                Span::styled("UPSTREAM   ", err()),
                Span::raw("502  10.0.0.8:8080 timeout"),
            ]),
        ];
        f.render_widget(
            Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(" logs (mock) ")),
            f.area(),
        );
    })
}
