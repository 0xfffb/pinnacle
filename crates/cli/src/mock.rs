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

fn notice(title: &str, line: String, style: Style) -> io::Result<()> {
    render(3, |f| {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(line, style)))
                .block(Block::default().borders(Borders::ALL).title(title)),
            f.area(),
        );
    })
}

pub fn rules() -> io::Result<()> {
    render(9, |f| {
        let header = Row::new(["id", "match", "action", "hits"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let rows = [
            ("r-allow-bing", "ua Bingbot", "allow", "1.2k"),
            ("r-admin", "path /admin", "challenge", "86"),
            ("r-asn-16509", "asn 16509", "challenge", "214"),
            ("r-login", "path /login  rate>20/m", "ban", "9"),
        ]
        .into_iter()
        .map(|(id, m, action, hits)| {
            let action_style = match action {
                "allow" => ok(),
                "ban" => err(),
                _ => warn(),
            };
            Row::new([
                Cell::from(id).style(accent()),
                Cell::from(m),
                Cell::from(action).style(action_style),
                Cell::from(hits).style(dim()),
            ])
        });
        f.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(14),
                    Constraint::Min(18),
                    Constraint::Length(12),
                    Constraint::Length(8),
                ],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(" rules (mock) ")),
            f.area(),
        );
    })
}

pub fn reload() -> io::Result<()> {
    notice(" reload (mock) ", " reloaded  4 rules  2 sites  0 errors".into(), ok())
}

pub fn sites() -> io::Result<()> {
    render(8, |f| {
        let header = Row::new(["site", "upstream", "mode", "qps"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let rows = [
            ("docs.example", "10.0.0.8:8080", "challenge", "31"),
            ("api.example", "10.0.0.9:8080", "allow+rate", "12"),
            ("www.example", "10.0.0.8:8080", "challenge", "4"),
        ]
        .into_iter()
        .map(|(site, up, mode, qps)| {
            Row::new([
                Cell::from(site).style(accent()),
                Cell::from(up),
                Cell::from(mode).style(warn()),
                Cell::from(qps).style(dim()),
            ])
        });
        f.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(14),
                    Constraint::Length(16),
                    Constraint::Length(12),
                    Constraint::Min(6),
                ],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(" sites (mock) ")),
            f.area(),
        );
    })
}

pub fn allowlist() -> io::Result<()> {
    render(8, |f| {
        let header = Row::new(["entry", "type", "scope"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let rows = [
            ("66.249.64.0/19", "cidr", "global"),
            ("Googlebot", "ua", "www.example"),
            ("uptime.internal", "ua", "api.example"),
        ]
        .into_iter()
        .map(|(entry, kind, scope)| {
            Row::new([
                Cell::from(entry).style(ok()),
                Cell::from(kind),
                Cell::from(scope).style(dim()),
            ])
        });
        f.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(18),
                    Constraint::Length(8),
                    Constraint::Min(12),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" allowlist (mock) "),
            ),
            f.area(),
        );
    })
}

pub fn allow(entry: &str) -> io::Result<()> {
    notice(
        " allow (mock) ",
        format!(" allowlisted  {entry}"),
        ok(),
    )
}

pub fn deny(entry: &str) -> io::Result<()> {
    notice(
        " deny (mock) ",
        format!(" denylisted  {entry}"),
        err(),
    )
}

pub fn ratelimit() -> io::Result<()> {
    render(8, |f| {
        let header = Row::new(["key", "limit", "current", "action"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let rows = [
            ("ip 203.0.113.14", "60/m", "71", "challenge"),
            ("path /login", "20/m", "9", "allow"),
            ("asn 16509", "200/m", "188", "allow"),
        ]
        .into_iter()
        .map(|(key, limit, current, action)| {
            let action_style = if action == "allow" { ok() } else { warn() };
            Row::new([
                Cell::from(key),
                Cell::from(limit).style(dim()),
                Cell::from(current).style(accent()),
                Cell::from(action).style(action_style),
            ])
        });
        f.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Min(16),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Length(12),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" ratelimit (mock) "),
            ),
            f.area(),
        );
    })
}

pub fn sessions() -> io::Result<()> {
    render(8, |f| {
        let header = Row::new(["ip", "kind", "left", "state"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let rows = [
            ("203.0.113.14", "pass", "41m", "ok"),
            ("198.51.100.7", "cid", "8m", "pending"),
            ("192.0.2.10", "pass", "2m", "expiring"),
        ]
        .into_iter()
        .map(|(ip, kind, left, state)| {
            let state_style = match state {
                "ok" => ok(),
                "expiring" => warn(),
                _ => accent(),
            };
            Row::new([
                Cell::from(ip),
                Cell::from(kind).style(dim()),
                Cell::from(left),
                Cell::from(state).style(state_style),
            ])
        });
        f.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(16),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Min(10),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" sessions (mock) "),
            ),
            f.area(),
        );
    })
}

pub fn explain(target: &str) -> io::Result<()> {
    let target = target.to_owned();
    render(8, |f| {
        let rows = vec![
            kv("request", target.clone(), accent()),
            kv("matched", "r-admin  path /admin", warn()),
            kv("score", "0.72", warn()),
            kv("decision", "challenge  cookie", warn()),
            kv("why", "no valid pass + admin prefix", Style::default()),
        ];
        f.render_widget(table(" explain (mock) ", rows), f.area());
    })
}

pub fn metrics() -> io::Result<()> {
    render(10, |f| {
        let body = vec![
            Line::from(Span::styled("# mock prometheus exposition", dim())),
            Line::from("pinnacle_requests_total{action=\"allow\"} 1842"),
            Line::from("pinnacle_requests_total{action=\"challenge\"} 311"),
            Line::from("pinnacle_requests_total{action=\"ban\"} 14"),
            Line::from("pinnacle_upstream_errors_total 2"),
            Line::from("pinnacle_request_duration_ms{quantile=\"0.99\"} 21"),
        ];
        f.render_widget(
            Paragraph::new(body).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" metrics (mock) "),
            ),
            f.area(),
        );
    })
}

pub fn config_show() -> io::Result<()> {
    render(9, |f| {
        let rows = vec![
            kv("listen", "0.0.0.0:6188", accent()),
            kv("upstream", "127.0.0.1:8080", ok()),
            kv("tls", "off", dim()),
            kv("store", "memory", warn()),
            kv("challenge", "cookie  random-path", accent()),
            kv("control", "127.0.0.1:6190  (planned)", dim()),
        ];
        f.render_widget(table(" config (mock) ", rows), f.area());
    })
}

pub fn audit() -> io::Result<()> {
    render(9, |f| {
        let lines = vec![
            Line::from(vec![
                Span::styled("11:58:02  ", dim()),
                Span::styled("ops     ", accent()),
                Span::raw("ban  198.51.100.7  manual"),
            ]),
            Line::from(vec![
                Span::styled("11:59:11  ", dim()),
                Span::styled("ops     ", accent()),
                Span::raw("reload  rules.yaml"),
            ]),
            Line::from(vec![
                Span::styled("12:00:44  ", dim()),
                Span::styled("system  ", dim()),
                Span::raw("rotate  challenge path"),
            ]),
            Line::from(vec![
                Span::styled("12:01:11  ", dim()),
                Span::styled("auto    ", err()),
                Span::raw("ban  198.51.100.7  challenge_failed"),
            ]),
        ];
        f.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" audit (mock) "),
            ),
            f.area(),
        );
    })
}

pub fn rotate() -> io::Result<()> {
    notice(
        " rotate (mock) ",
        " rotated  challenge path  /k2n9c1xq0m4p7w8d3f5h6j".into(),
        accent(),
    )
}

pub fn peers() -> io::Result<()> {
    render(8, |f| {
        let header = Row::new(["peer", "role", "lag", "store"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let rows = [
            ("edge-1", "leader", "0ms", "memory"),
            ("edge-2", "replica", "12ms", "memory"),
            ("edge-3", "down", "-", "—"),
        ]
        .into_iter()
        .map(|(peer, role, lag, store)| {
            let role_style = match role {
                "leader" => ok(),
                "down" => err(),
                _ => warn(),
            };
            Row::new([
                Cell::from(peer).style(accent()),
                Cell::from(role).style(role_style),
                Cell::from(lag).style(dim()),
                Cell::from(store),
            ])
        });
        f.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(8),
                    Constraint::Min(8),
                ],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(" peers (mock) ")),
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
