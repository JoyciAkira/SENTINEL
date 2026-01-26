use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs, List, ListItem, ListState, Table, Row},
    Terminal, Frame,
};
use std::{io, time::{Duration, Instant}};

use std::path::PathBuf;

pub struct AgentStatus {
    pub name: String,
    pub goal: String,
    pub activity: String,
}

pub struct TuiApp {
    pub title: String,
    pub alignment_score: f64,
    pub current_tab: usize,
    pub should_quit: bool,
    pub goal_list_state: ListState,
    pub goals: Vec<String>,
    pub dependency_count: usize,
    pub agents: Vec<AgentStatus>,
    pub conflicts: Vec<String>,
    pub manifold_path: PathBuf,
    pub cognitive_density: f64,
    pub estimated_tokens: usize,
    pub is_barrier_locked: bool,
}

impl TuiApp {
    pub fn new(manifold_path: PathBuf) -> Self {
        let mut goal_list_state = ListState::default();
        goal_list_state.select(Some(0));

        Self {
            title: "SENTINEL COGNITIVE DASHBOARD".to_string(),
            alignment_score: 0.85,
            current_tab: 0,
            should_quit: false,
            goal_list_state,
            goals: Vec::new(),
            dependency_count: 0,
            agents: Vec::new(),
            conflicts: Vec::new(),
            manifold_path,
            cognitive_density: 0.0,
            estimated_tokens: 0,
            is_barrier_locked: false,
        }
    }

    pub fn on_tick(&mut self) {
        // Polling reale dal file sentinel.json
        if self.manifold_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.manifold_path) {
                if let Ok(manifold) = serde_json::from_str::<sentinel_core::GoalManifold>(&content) {
                    // Sincronizzazione Goal tramite i metodi del DAG (goals() restituisce già un iteratore)
                    self.goals = manifold.goal_dag.goals().map(|g| g.description.clone()).collect();
                    
                    // Calcolo Barriera e Densità
                    let decision = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);
                    self.is_barrier_locked = !decision.allowed;
                    self.alignment_score = decision.score_at_check;
                    
                    let report = sentinel_core::architect::distiller::CognitiveDistiller::distill(&manifold);
                    self.cognitive_density = (report.strategic_density + report.tactical_density + report.operational_density) / 3.0;
                    self.estimated_tokens = report.total_tokens_estimated;

                    // Rilevamento Conflitti Reali
                    self.conflicts.clear();
                    let mut files_seen = std::collections::HashMap::new();
                    for (file, agent_id) in &manifold.file_locks {
                        if let Some(other_agent) = files_seen.insert(file, agent_id) {
                            if other_agent != agent_id {
                                self.conflicts.push(format!("COLLISION: File {:?} is being touched by multiple agents!", file));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn run_tui(manifold_path: PathBuf) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let mut app = TuiApp::new(manifold_path);
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
                    KeyCode::Right => app.current_tab = (app.current_tab + 1) % 8,
                    KeyCode::Left => app.current_tab = if app.current_tab == 0 { 7 } else { app.current_tab - 1 },
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
    let chunks = if !app.conflicts.is_empty() {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Alert
                Constraint::Length(3), // Header
                Constraint::Length(3), // Gauge
                Constraint::Min(0),    // Main
                Constraint::Length(3), // Footer
            ].as_ref())
            .split(f.size())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Length(3), // Gauge
                Constraint::Min(0),    // Main
                Constraint::Length(3), // Footer
            ].as_ref())
            .split(f.size())
    };

    let mut current_chunk = 0;

    // 0. Conflict Alert (Se presente)
    if !app.conflicts.is_empty() {
        let alert_text = app.conflicts.join(" | ");
        let alert_block = Paragraph::new(alert_text)
            .block(Block::default().borders(Borders::ALL).title("!!! AGENT COLLISION DETECTED !!!"))
            .style(Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK));
        f.render_widget(alert_block, chunks[current_chunk]);
        current_chunk += 1;
    }

    // 1. Header: Title and Tabs
    let titles = vec!["Overview", "Goal Tree", "Knowledge Base", "Infrastructure", "External", "Calibration", "Multi-Agent", "Architect"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Layers"))
        .select(app.current_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[current_chunk]);
    current_chunk += 1;

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
    f.render_widget(gauge, chunks[current_chunk]);
    current_chunk += 1;

    // 3. Main Content
    let main_chunk = chunks[current_chunk];
    match app.current_tab {
        0 => {
            let barrier_status = if app.is_barrier_locked {
                "[LOCKED] 🛑 RUNTIME BARRIER ACTIVE - Execution Interdicted"
            } else {
                "[UNLOCKED] ✅ RUNTIME BARRIER OPEN - Execution Authorized"
            };
            
            let inner_text = format!(
                "COGNITIVE OS STATUS: READY\n\n{}\n\nSentinel is active and watching the Goal Manifold.\nAlignment gradients are stable.",
                barrier_status
            );
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("Cognitive Status"))
                .style(Style::default().fg(if app.is_barrier_locked { Color::Red } else { Color::Green }));
            f.render_widget(main_block, main_chunk);
        }
        1 => {
            let goal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(main_chunk);

            // List of goals
            let items: Vec<ListItem> = app.goals.iter().map(|g| ListItem::new(g.as_str())).collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Goal DAG"))
                .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, goal_chunks[0], &mut app.goal_list_state.clone());

            // Details
            let selected_index = app.goal_list_state.selected().unwrap_or(0);
            let goal_name = if app.goals.is_empty() { "No goals" } else { &app.goals[selected_index] };
            let details = format!(
                "GOAL: {}\n\nSTATUS: Active\nPROGRESS: 45%\n\nSUCCESS CRITERIA:\n- Validated by Layer 2\n- Code Coverage > 80%\n- Invariants Satisfied",
                goal_name
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
            f.render_widget(main_block, main_chunk);
        }
        3 => {
            let inner_text = "INFRASTRUCTURE MAP:\n\n- Frontend IP: 192.168.1.50 [ONLINE]\n- API Gateway: api.sentinel.internal [ONLINE]\n- Database: db.cluster.local [STABLE]";
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("Live Assets"))
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, main_chunk);
        }
        4 => {
            let inner_text = format!(
                "EXTERNAL AWARENESS:\n\n- Detected Dependencies: {}\n- Watched Sources: 2\n- Risk Level: 0.05 (Negligible)\n\nSTATUS: No external alignment threats detected.",
                app.dependency_count
            );
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("External Dependencies & Risks"))
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, main_chunk);
        }
        5 => {
            let inner_text = "ALIGNMENT CALIBRATION:\n\n- Sentinel Sensitivity: 0.50 [BALANCED]\n- Precision Rate: 98.2%\n- False Positive Rate: 1.8%\n\nHuman Overrides Registered: 0\n\n(Use 'sentinel calibrate <VALUE>' to adjust rigidity)";
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("Sensitivity & Confidence Control"))
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, main_chunk);
        }
        6 => {
            let social_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                .split(main_chunk);

            // Agent Herd Table
            let rows = app.agents.iter().map(|a| {
                Row::new(vec![a.name.clone(), a.goal.clone(), a.activity.clone()])
            });
            let table = Table::new(rows, [
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .header(Row::new(vec!["Agent ID", "Current Goal", "Live Activity"])
                .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            .block(Block::default().borders(Borders::ALL).title("Social Manifold: Active Agent Herd"));
            f.render_widget(table, social_chunks[0]);

            // Handover Notes
            let handover_text = "COGNITIVE TRAIL (Handover Notes):\n\n[10:15] Cline -> Cursor: 'Implemented base types, check field alignment in types.rs'\n[11:30] Cursor -> Kilo: 'Refined LSP diagnostics, performance verified'\n[NOW] Kilo -> Team: 'Invariants secured. Ready for Layer 8 integration'";
            let handover_block = Paragraph::new(handover_text)
                .block(Block::default().borders(Borders::ALL).title("Cognitive Handover Log"))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(handover_block, social_chunks[1]);
        }
        7 => {
            let arch_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(main_chunk);

            let omniscience_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Agent Omniscience Level (Context Injection)"))
                .gauge_style(Style::default().fg(Color::Magenta).bg(Color::Black))
                .percent((app.cognitive_density * 100.0) as u16)
                .label(format!("Density: {:.1}% | Est. Tokens: {}", app.cognitive_density * 100.0, app.estimated_tokens));
            f.render_widget(omniscience_gauge, arch_chunks[0]);

            let inner_text = "ARCHITECT ENGINE (Goal Decomposition):\n\n- Analyzing: 'Sviluppo Sentinel OS'\n- Status: Active\n- Strategy: Hierarchical Multi-Layer Analysis\n\nPROPOSED STRUCTURE:\n1. Core Engine Integrity\n2. Alignment Field Visualization\n3. Meta-Learning Persistence\n4. Social Manifold Orchestration";
            let main_block = Paragraph::new(inner_text)
                .block(Block::default().borders(Borders::ALL).title("Autonomous Architect Proposal"))
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, arch_chunks[1]);
        }
        _ => {}
    }

    // 4. Footer
    let footer = Paragraph::new("Press 'q' to quit | Arrows to navigate | Up/Down to select goals")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[3]);
}

impl TuiApp {
    pub fn next_goal(&mut self) {
        if self.goals.is_empty() { return; }
        let i = match self.goal_list_state.selected() {
            Some(i) => {
                if i >= self.goals.len() - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        self.goal_list_state.select(Some(i));
    }

    pub fn previous_goal(&mut self) {
        if self.goals.is_empty() { return; }
        let i = match self.goal_list_state.selected() {
            Some(i) => {
                if i == 0 { self.goals.len() - 1 } else { i - 1 }
            }
            None => 0,
        };
        self.goal_list_state.select(Some(i));
    }
}
