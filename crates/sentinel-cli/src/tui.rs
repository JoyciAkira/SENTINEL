use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs, List, ListItem, ListState},
    Terminal, Frame,
};
use std::{io, time::{Duration, Instant}};

pub struct TuiApp {
    pub title: String,
    pub alignment_score: f64,
    pub current_tab: usize,
    pub should_quit: bool,
    pub goal_list_state: ListState,
    pub goals: Vec<String>,
}

impl TuiApp {
    pub fn new() -> Self {
        let mut goal_list_state = ListState::default();
        goal_list_state.select(Some(0));

        Self {
            title: "SENTINEL COGNITIVE DASHBOARD".to_string(),
            alignment_score: 0.85,
            current_tab: 0,
            should_quit: false,
            goal_list_state,
            goals: vec![
                "Layer 6: TUI Refinement".to_string(),
                "Layer 6: VS Code Extension".to_string(),
                "Layer 7: External Awareness".to_string(),
                "Layer 8: Multi-Agent Sync".to_string(),
            ],
        }
    }

    pub fn next_goal(&mut self) {
        let i = match self.goal_list_state.selected() {
            Some(i) => {
                if i >= self.goals.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.goal_list_state.select(Some(i));
    }

    pub fn previous_goal(&mut self) {
        let i = match self.goal_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.goals.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.goal_list_state.select(Some(i));
    }

    pub fn on_tick(&mut self) {}
}

pub fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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
                    KeyCode::Right => app.current_tab = (app.current_tab + 1) % 4,
                    KeyCode::Left => app.current_tab = if app.current_tab == 0 { 3 } else { app.current_tab - 1 },
                    KeyCode::Down => app.next_goal(),
                    KeyCode::Up => app.previous_goal(),
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

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
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
    let titles = vec!["Overview", "Goal Tree", "Knowledge Base", "Infrastructure"];
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

    // 3. Main Content
    match app.current_tab {
        0 => {
            let inner_text = "SYSTEM READY - Monitoring agent activity...\n\nSentinel is active and watching the Goal Manifold.\nAlignment gradients are stable.";
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("Cognitive Status"))
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, chunks[2]);
        }
        1 => {
            let goal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(chunks[2]);

            // List of goals
            let items: Vec<ListItem> = app.goals.iter().map(|g| ListItem::new(g.as_str())).collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Goal DAG"))
                .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, goal_chunks[0], &mut app.goal_list_state.clone());

            // Details
            let selected_index = app.goal_list_state.selected().unwrap_or(0);
            let details = format!(
                "GOAL: {}\n\nSTATUS: Active\nPROGRESS: 45%\n\nSUCCESS CRITERIA:\n- Validated by Layer 2\n- Code Coverage > 80%\n- Invariants Satisfied",
                app.goals[selected_index]
            );
            let detail_block = Paragraph::new(details)
                .block(Block::default().borders(Borders::ALL).title("Context Details"))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(detail_block, goal_chunks[1]);
        }
        2 => {
            let inner_text = "LEARNED PATTERNS:\n\n1. [Success] TestsBeforeImplementation (92%)\n2. [Deviation] LargeFileEditing (45% risk)\n3. [Success] AtomicCommits (88%)";
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("Meta-Learning Insights"))
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, chunks[2]);
        }
        3 => {
            let inner_text = "INFRASTRUCTURE MAP:\n\n- Frontend IP: 192.168.1.50 [ONLINE]\n- API Gateway: api.sentinel.internal [ONLINE]\n- Database: db.cluster.local [STABLE]";
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("Live Assets"))
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, chunks[2]);
        }
        _ => {}
    }

    // 4. Footer
    let footer = Paragraph::new("Press 'q' to quit | Arrows to navigate | Up/Down to select goals")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[3]);
}
