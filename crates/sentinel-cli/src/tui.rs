use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs},
    Terminal, Frame,
};
use std::{io, time::{Duration, Instant}};

pub struct TuiApp {
    pub title: String,
    pub alignment_score: f64,
    pub current_tab: usize,
    pub should_quit: bool,
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            title: "SENTINEL COGNITIVE DASHBOARD".to_string(),
            alignment_score: 0.85, // Mock iniziale
            current_tab: 0,
            should_quit: false,
        }
    }

    pub fn on_tick(&mut self) {
        // Logica per aggiornare i dati in tempo reale
    }
}

pub fn run_tui() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run loop
    let tick_rate = Duration::from_millis(250);
    let mut app = TuiApp::new();
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Right => app.current_tab = (app.current_tab + 1) % 3,
                    KeyCode::Left => app.current_tab = app.current_tab.saturating_sub(1),
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Alignment Gauge
            Constraint::Min(0),    // Main Content
            Constraint::Length(3), // Footer
        ].as_ref())
        .split(f.size());

    // 1. Header: Title and Tabs
    let titles = vec!["Overview", "Goal Tree", "Knowledge Base"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Layers"))
        .select(app.current_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    // 2. Alignment Gauge
    let gauge_color = if app.alignment_score > 0.8 {
        Color::Green
    } else if app.alignment_score > 0.5 {
        Color::Yellow
    } else {
        Color::Red
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Current Alignment Field"))
        .gauge_style(Style::default().fg(gauge_color).bg(Color::Black))
        .percent((app.alignment_score * 100.0) as u16);
    f.render_widget(gauge, chunks[1]);

    // 3. Main Content (Placeholder for now)
    let inner_text = match app.current_tab {
        0 => "SYSTEM READY - Monitoring agent activity...",
        1 => "GOAL MANIFOLD GRAPH - [Loading DAG...]",
        2 => "LEARNED PATTERNS - [Querying Knowledge Base...]",
        _ => "",
    };

    let main_block = Paragraph::new(inner_text)
        .block(Block::default().borders(Borders::ALL).title("Monitoring Area"))
        .style(Style::default().fg(Color::White));
    f.render_widget(main_block, chunks[2]);

    // 4. Footer
    let footer = Paragraph::new("Press 'q' to quit | Use Arrow Keys to navigate tabs")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[3]);
}
