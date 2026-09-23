use std::io::{self, stdout};
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Borders, Cell, Chart, Dataset, Gauge, GraphType, Paragraph, Row, Sparkline, Table,
};
use ratatui::Frame;
use ratatui::Terminal;

#[derive(Clone)]
struct Snapshot {
    qps: f64,
    allowed: u64,
    challenged: u64,
    banned: u64,
    upstream_err: u64,
    p99_ms: f64,
    listen: String,
    upstream: String,
    challenge_path: String,
}

struct Demo {
    started: Instant,
    qps: Vec<(f64, f64)>,
    recent_qps: Vec<u64>,
}

impl Demo {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            qps: Vec::new(),
            recent_qps: vec![0; 48],
        }
    }

    fn tick(&mut self) -> Snapshot {
        let t = self.started.elapsed().as_secs_f64();
        let qps = 42.0 + 18.0 * (t * 0.7).sin() + 6.0 * (t * 1.9).cos();
        let qps = qps.max(8.0);

        self.qps.push((t, qps));
        if self.qps.len() > 80 {
            self.qps.remove(0);
        }
        self.recent_qps.push(qps as u64);
        if self.recent_qps.len() > 48 {
            self.recent_qps.remove(0);
        }

        let elapsed = t.max(1.0);
        Snapshot {
            qps,
            allowed: (elapsed * 31.0) as u64,
            challenged: (elapsed * 9.0) as u64,
            banned: 12 + (elapsed * 0.15) as u64,
            upstream_err: (elapsed * 0.04) as u64,
            p99_ms: 18.0 + 7.0 * (t * 0.4).sin().abs(),
            listen: "0.0.0.0:6188".into(),
            upstream: "127.0.0.1:8080".into(),
            challenge_path: "/7kkfg0kdt80b9a4eg41b8sz9rdwndbx".into(),
        }
    }
}

pub fn run() -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;

    let mut demo = Demo::new();
    let tick = Duration::from_millis(200);
    let result = loop {
        let snap = demo.tick();
        terminal.draw(|f| draw(f, &snap, &demo))?;

        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press
                    && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
                {
                    break Ok(());
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn draw(frame: &mut Frame, snap: &Snapshot, demo: &Demo) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Min(10),
            Constraint::Length(8),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, root[0], snap);
    draw_kpis(frame, root[1], snap);
    draw_chart(frame, root[2], demo);
    draw_bottom(frame, root[3], snap, demo);
    frame.render_widget(
        Paragraph::new(" q quit · demo data · not connected to gateway").style(dim()),
        root[4],
    );
}

fn draw_header(frame: &mut Frame, area: Rect, snap: &Snapshot) {
    let title = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            " PINNACLE ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  gateway  "),
        Span::styled(&snap.listen, Style::default().fg(Color::Cyan)),
        Span::raw("  →  "),
        Span::styled(&snap.upstream, Style::default().fg(Color::Green)),
    ])])
    .block(Block::default().borders(Borders::ALL).title(" status "));
    frame.render_widget(title, area);
}

fn draw_kpis(frame: &mut Frame, area: Rect, snap: &Snapshot) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    kpi(frame, cols[0], "QPS", format!("{:.1}", snap.qps), Color::Cyan);
    kpi(
        frame,
        cols[1],
        "allowed",
        snap.allowed.to_string(),
        Color::Green,
    );
    kpi(
        frame,
        cols[2],
        "challenge",
        snap.challenged.to_string(),
        Color::Yellow,
    );
    kpi(frame, cols[3], "banned", snap.banned.to_string(), Color::Red);
}

fn kpi(frame: &mut Frame, area: Rect, label: &str, value: String, color: Color) {
    let text = Paragraph::new(vec![
        Line::from(Span::styled(label, dim())),
        Line::from(Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
    ])
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(text, area);
}

fn draw_chart(frame: &mut Frame, area: Rect, demo: &Demo) {
    if demo.qps.len() < 2 {
        return;
    }
    let min_x = demo.qps.first().map(|p| p.0).unwrap_or(0.0);
    let max_x = demo.qps.last().map(|p| p.0).unwrap_or(1.0);
    let max_y = demo
        .qps
        .iter()
        .map(|p| p.1)
        .fold(10.0_f64, |a, b| a.max(b))
        * 1.15;

    let dataset = Dataset::default()
        .name("qps")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&demo.qps);

    let max_label = format!("{max_y:.0}");
    let chart = Chart::new(vec![dataset])
        .block(Block::default().borders(Borders::ALL).title(" request rate "))
        .x_axis(
            Axis::default()
                .style(dim())
                .bounds([min_x, max_x.max(min_x + 1.0)]),
        )
        .y_axis(
            Axis::default()
                .style(dim())
                .bounds([0.0, max_y])
                .labels(["0".to_string(), max_label]),
        );
    frame.render_widget(chart, area);
}

fn draw_bottom(frame: &mut Frame, area: Rect, snap: &Snapshot, demo: &Demo) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(32),
            Constraint::Percentage(30),
        ])
        .split(area);

    let err = snap.upstream_err.to_string();
    let rows = [
        ("listen", snap.listen.as_str()),
        ("upstream", snap.upstream.as_str()),
        ("challenge", snap.challenge_path.as_str()),
        ("upstream err", err.as_str()),
    ]
    .into_iter()
    .map(|(k, v)| {
        Row::new([
            Cell::from(k).style(dim()),
            Cell::from(v).style(Style::default().fg(Color::White)),
        ])
    });

    frame.render_widget(
        Table::new(rows, [Constraint::Length(14), Constraint::Min(10)])
            .block(Block::default().borders(Borders::ALL).title(" endpoints ")),
        cols[0],
    );

    frame.render_widget(
        Sparkline::default()
            .block(Block::default().borders(Borders::ALL).title(" qps spark "))
            .style(Style::default().fg(Color::Cyan))
            .data(&demo.recent_qps),
        cols[1],
    );

    let ratio = (snap.p99_ms / 80.0).clamp(0.05, 1.0);
    frame.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(" p99 latency "))
            .gauge_style(Style::default().fg(Color::Magenta))
            .ratio(ratio)
            .label(format!("{:.0} ms", snap.p99_ms)),
        cols[2],
    );
}

fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}
