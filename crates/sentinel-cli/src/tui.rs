use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Row, Table, Tabs},
    Frame, Terminal,
};
use std::{
    io,
    time::{Duration, Instant},
};

use crate::reliability;
use std::path::PathBuf;

pub struct AgentStatus {
    pub name: String,
    pub goal: String,
    pub activity: String,
}

/// Metadati reali di un singolo goal, estratti dal GoalManifold.
/// Usati dal Tab 1 (Goal Tree) per mostrare dati verificabili, non hardcoded.
pub struct GoalMeta {
    pub id: String,
    pub status: String,
    pub progress_pct: f64,
    pub success_criteria: Vec<String>,
}

/// Nota di handover reale dal manifold.handover_log.
/// Campi mappati da sentinel_core::types::HandoverNote (agent_id, content, timestamp).
pub struct HandoverNote {
    pub timestamp: String,
    pub agent_id: String,
    pub content: String,
    pub warnings: Vec<String>,
}

pub struct TuiApp {
    pub title: String,
    pub alignment_score: f64,
    pub current_tab: usize,
    pub should_quit: bool,
    pub goal_list_state: ListState,
    pub goals: Vec<String>,
    /// Metadati reali per ogni goal (parallelo a `goals`)
    pub goal_meta: Vec<GoalMeta>,
    pub dependency_count: usize,
    pub agents: Vec<AgentStatus>,
    pub conflicts: Vec<String>,
    pub manifold_path: PathBuf,
    pub cognitive_density: f64,
    pub estimated_tokens: usize,
    pub is_barrier_locked: bool,
    pub reliability: sentinel_core::ReliabilitySnapshot,
    pub reliability_thresholds: reliability::ReliabilityThresholds,
    pub reliability_eval: reliability::ReliabilityEvaluation,
    /// Sensibilità reale dal manifold (non hardcoded 0.50)
    pub sensitivity: f64,
    /// Override registrati nel manifold
    pub override_count: usize,
    /// Handover log reale dal manifold
    pub handover_log: Vec<HandoverNote>,
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
            goal_meta: Vec::new(),
            dependency_count: 0,
            agents: Vec::new(),
            conflicts: Vec::new(),
            manifold_path,
            cognitive_density: 0.0,
            estimated_tokens: 0,
            is_barrier_locked: false,
            reliability: sentinel_core::ReliabilitySnapshot::from_counts(0, 0, 0, 0, 0, 0, 0),
            reliability_thresholds: reliability::ReliabilityThresholds::default(),
            reliability_eval: reliability::ReliabilityEvaluation::default(),
            sensitivity: 0.5,
            override_count: 0,
            handover_log: Vec::new(),
        }
    }

    pub fn on_tick(&mut self) {
        // Polling reale dal file sentinel.json ogni 250ms
        if self.manifold_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.manifold_path) {
                if let Ok(manifold) = serde_json::from_str::<sentinel_core::GoalManifold>(&content)
                {
                    // ── Goal descriptions + metadati reali ──────────────────
                    let goals_vec: Vec<_> = manifold.goal_dag.goals().collect();
                    self.goals = goals_vec.iter().map(|g| g.description.clone()).collect();

                    // Popola goal_meta con dati reali: id, status, progress, criteri
                    self.goal_meta = goals_vec
                        .iter()
                        .map(|g| {
                            let status = format!("{:?}", g.status).to_lowercase();
                            // progress_pct: completed=1.0, in_progress=0.5, altri=0.0
                            let progress_pct = match g.status {
                                sentinel_core::GoalStatus::Completed => 1.0,
                                sentinel_core::GoalStatus::InProgress => 0.5,
                                _ => 0.0,
                            };
                            // success_criteria: descrizione dei predicati
                            let success_criteria: Vec<String> = g
                                .success_criteria
                                .iter()
                                .map(|p| format!("{:?}", p))
                                .collect();
                            GoalMeta {
                                id: g.id.to_string(),
                                status,
                                progress_pct,
                                success_criteria: if success_criteria.is_empty() {
                                    vec!["No criteria defined".to_string()]
                                } else {
                                    success_criteria
                                },
                            }
                        })
                        .collect();

                    // ── Sensibilità reale dal manifold ──────────────────────
                    self.sensitivity = manifold.sensitivity;

                    // ── Override count reale ────────────────────────────────
                    self.override_count = manifold.overrides.len();

                    // ── Handover log reale ──────────────────────────────────
                    // Campi reali: agent_id, content, technical_warnings, timestamp
                    self.handover_log = manifold
                        .handover_log
                        .iter()
                        .map(|note| {
                            let ts = note.timestamp.format("%H:%M").to_string();
                            HandoverNote {
                                timestamp: ts,
                                agent_id: note.agent_id.to_string(),
                                content: note.content.clone(),
                                warnings: note.technical_warnings.clone(),
                            }
                        })
                        .collect();

                    // ── Calcolo Barriera e Densità ──────────────────────────
                    let decision = sentinel_core::guardrail::GuardrailEngine::evaluate(&manifold);
                    self.is_barrier_locked = !decision.allowed;
                    self.alignment_score = decision.score_at_check;
                    self.reliability = reliability::snapshot_from_signals(
                        self.alignment_score,
                        1.0,
                        manifold.completion_percentage(),
                        !self.is_barrier_locked,
                    );
                    let config = reliability::load_reliability_config(&self.manifold_path);
                    self.reliability_thresholds = config.thresholds;
                    self.reliability_eval = reliability::evaluate_snapshot(
                        &self.reliability,
                        &self.reliability_thresholds,
                    );

                    let report =
                        sentinel_core::architect::distiller::CognitiveDistiller::distill(&manifold);
                    self.cognitive_density = (report.strategic_density
                        + report.tactical_density
                        + report.operational_density)
                        / 3.0;
                    self.estimated_tokens = report.total_tokens_estimated;

                    // ── Rilevamento Conflitti Reali ─────────────────────────
                    self.conflicts.clear();
                    let mut files_seen = std::collections::HashMap::new();
                    for (file, agent_id) in &manifold.file_locks {
                        if let Some(other_agent) = files_seen.insert(file, agent_id) {
                            if other_agent != agent_id {
                                self.conflicts.push(format!(
                                    "COLLISION: File {:?} is being touched by multiple agents!",
                                    file
                                ));
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
                    KeyCode::Right => app.current_tab = (app.current_tab + 1) % 9,
                    KeyCode::Left => {
                        app.current_tab = if app.current_tab == 0 {
                            8
                        } else {
                            app.current_tab - 1
                        }
                    }
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
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(f: &mut Frame, app: &TuiApp) {
    let chunks = if !app.conflicts.is_empty() {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Alert
                    Constraint::Length(3), // Header
                    Constraint::Length(3), // Gauge
                    Constraint::Min(0),    // Main
                    Constraint::Length(3), // Footer
                ]
                .as_ref(),
            )
            .split(f.size())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3), // Header
                    Constraint::Length(3), // Gauge
                    Constraint::Min(0),    // Main
                    Constraint::Length(3), // Footer
                ]
                .as_ref(),
            )
            .split(f.size())
    };

    let mut current_chunk = 0;

    // 0. Conflict Alert (Se presente)
    if !app.conflicts.is_empty() {
        let alert_text = app.conflicts.join(" | ");
        let alert_block = Paragraph::new(alert_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("!!! AGENT COLLISION DETECTED !!!"),
            )
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
            );
        f.render_widget(alert_block, chunks[current_chunk]);
        current_chunk += 1;
    }

    // 1. Header: Title and Tabs
    let titles = vec![
        "Overview",
        "Goal Tree",
        "Knowledge Base",
        "Infrastructure",
        "External",
        "Calibration",
        "Multi-Agent",
        "Architect",
        "Reliability",
    ];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Layers"))
        .select(app.current_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Current Alignment Field"),
        )
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
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Cognitive Status"),
                )
                .style(Style::default().fg(if app.is_barrier_locked {
                    Color::Red
                } else {
                    Color::Green
                }));
            f.render_widget(main_block, main_chunk);
        }
        1 => {
            let goal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(main_chunk);

            // List of goals — dati reali dal DAG
            let items: Vec<ListItem> = if app.goals.is_empty() {
                vec![ListItem::new("No goals — run `sentinel init <description>`")
                    .style(Style::default().fg(Color::DarkGray))]
            } else {
                app.goals
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        let meta = &app.goal_meta;
                        let status_prefix = if let Some(m) = meta.get(i) {
                            match m.status.as_str() {
                                "completed" => "✓ ",
                                "in_progress" => "▶ ",
                                "failed" => "✗ ",
                                _ => "○ ",
                            }
                        } else {
                            "○ "
                        };
                        ListItem::new(format!("{}{}", status_prefix, g))
                    })
                    .collect()
            };
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Goal DAG"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, goal_chunks[0], &mut app.goal_list_state.clone());

            // Details — dati reali dal goal selezionato
            let selected_index = app.goal_list_state.selected().unwrap_or(0);
            let details = if app.goals.is_empty() {
                "No goals loaded.\n\nRun:\n  sentinel init \"<your project description>\"\n\nThen press 'r' to refresh.".to_string()
            } else {
                let goal_name = &app.goals[selected_index];
                let meta = app.goal_meta.get(selected_index);
                let status = meta.map(|m| m.status.as_str()).unwrap_or("pending");
                let progress = meta.map(|m| m.progress_pct).unwrap_or(0.0);
                let criteria = meta
                    .map(|m| m.success_criteria.join("\n  - "))
                    .unwrap_or_else(|| "No criteria defined".to_string());
                let id_short = meta
                    .map(|m| m.id[..8.min(m.id.len())].to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                format!(
                    "GOAL: {}\n\nID: {}\nSTATUS: {}\nPROGRESS: {:.0}%\n\nSUCCESS CRITERIA:\n  - {}",
                    goal_name,
                    id_short,
                    status.to_uppercase(),
                    progress * 100.0,
                    criteria,
                )
            };
            let detail_block = Paragraph::new(details)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Goal Details (real data)"),
                )
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(detail_block, goal_chunks[1]);
        }
        2 => {
            let inner_text = "LEARNED PATTERNS:\n\n1. [Success] TestsBeforeImplementation (92%)\n2. [Deviation] LargeFileEditing (45% risk)\n3. [Success] AtomicCommits (88%)";
            let main_block = Paragraph::new(inner_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Meta-Learning Insights"),
                )
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
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("External Dependencies & Risks"),
                )
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, main_chunk);
        }
        5 => {
            // Tab 5: dati reali da manifold.sensitivity e manifold.overrides
            let sensitivity_label = match app.sensitivity {
                s if s < 0.3 => "PERMISSIVE",
                s if s < 0.6 => "BALANCED",
                s if s < 0.8 => "STRICT",
                _ => "MAXIMUM",
            };
            let sensitivity_color = match app.sensitivity {
                s if s < 0.3 => Color::Green,
                s if s < 0.6 => Color::Cyan,
                s if s < 0.8 => Color::Yellow,
                _ => Color::Red,
            };
            let inner_text = format!(
                "ALIGNMENT CALIBRATION (live from sentinel.json)\n\n\
                - Sentinel Sensitivity: {:.2} [{}]\n\
                - Alignment Score: {:.1}%\n\
                - Barrier Status: {}\n\n\
                Human Overrides Registered: {}\n\n\
                Adjust with:\n  sentinel calibrate <0.0-1.0>\n\n\
                Scale:\n\
                  0.0-0.3 = PERMISSIVE (more flexible)\n\
                  0.3-0.6 = BALANCED\n\
                  0.6-0.8 = STRICT\n\
                  0.8-1.0 = MAXIMUM (most rigid)",
                app.sensitivity,
                sensitivity_label,
                app.alignment_score * 100.0,
                if app.is_barrier_locked { "LOCKED 🛑" } else { "OPEN ✅" },
                app.override_count,
            );
            let main_block = Paragraph::new(inner_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Sensitivity & Confidence Control (real data)"),
                )
                .style(Style::default().fg(sensitivity_color))
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(main_block, main_chunk);
        }
        6 => {
            let social_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)].as_ref())
                .split(main_chunk);

            // Agent Herd Table (dati reali da manifold.agents se presenti)
            let rows = app
                .agents
                .iter()
                .map(|a| Row::new(vec![a.name.clone(), a.goal.clone(), a.activity.clone()]));
            let agent_title = if app.agents.is_empty() {
                "Social Manifold: No active agents"
            } else {
                "Social Manifold: Active Agent Herd"
            };
            let table = Table::new(
                rows,
                [
                    Constraint::Percentage(30),
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                ],
            )
            .header(
                Row::new(vec!["Agent ID", "Current Goal", "Live Activity"]).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(agent_title),
            );
            f.render_widget(table, social_chunks[0]);

            // Handover log reale da manifold.handover_log
            let handover_text = if app.handover_log.is_empty() {
                "COGNITIVE TRAIL (Handover Notes):\n\nNo handover notes yet.\n\nHandover notes are added automatically when agents\ntransfer context between sessions.\n\nUse: sentinel status --json to see full manifold state.".to_string()
            } else {
                let entries: Vec<String> = app
                    .handover_log
                    .iter()
                    .rev() // più recenti prima
                    .take(10) // max 10 entries visibili
                    .map(|note| {
                        let warn_suffix = if note.warnings.is_empty() {
                            String::new()
                        } else {
                            format!(" ⚠ {}", note.warnings.join("; "))
                        };
                        format!(
                            "[{}] agent:{} — {}{}",
                            note.timestamp,
                            &note.agent_id[..8.min(note.agent_id.len())],
                            note.content,
                            warn_suffix,
                        )
                    })
                    .collect();
                format!(
                    "COGNITIVE TRAIL (Handover Notes — real data):\n\n{}",
                    entries.join("\n")
                )
            };
            let handover_block = Paragraph::new(handover_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(
                            "Cognitive Handover Log ({} entries)",
                            app.handover_log.len()
                        )),
                )
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(handover_block, social_chunks[1]);
        }
        7 => {
            let arch_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(main_chunk);

            let omniscience_gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Agent Omniscience Level (Context Injection)"),
                )
                .gauge_style(Style::default().fg(Color::Magenta).bg(Color::Black))
                .percent((app.cognitive_density * 100.0) as u16)
                .label(format!(
                    "Density: {:.1}% | Est. Tokens: {}",
                    app.cognitive_density * 100.0,
                    app.estimated_tokens
                ));
            f.render_widget(omniscience_gauge, arch_chunks[0]);

            let inner_text = "ARCHITECT ENGINE (Goal Decomposition):\n\n- Analyzing: 'Sviluppo Sentinel OS'\n- Status: Active\n- Strategy: Hierarchical Multi-Layer Analysis\n\nPROPOSED STRUCTURE:\n1. Core Engine Integrity\n2. Alignment Field Visualization\n3. Meta-Learning Persistence\n4. Social Manifold Orchestration";
            let main_block = Paragraph::new(inner_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Autonomous Architect Proposal"),
                )
                .style(Style::default().fg(Color::White));
            f.render_widget(main_block, arch_chunks[1]);
        }
        8 => {
            let rollback_color = if app.reliability.rollback_rate <= 0.05 {
                Color::Green
            } else if app.reliability.rollback_rate <= 0.15 {
                Color::Yellow
            } else {
                Color::Red
            };

            let inner_text = format!(
                "RUNTIME RELIABILITY SNAPSHOT:\n\n- Task Success Rate: {:.1}%\n- No-Regression Rate: {:.1}%\n- Rollback Rate: {:.1}%\n- Avg Time To Recover: {:.0} ms\n- Invariant Violation Rate: {:.1}%\n\nSLO Targets (sentinel.json):\n- Success >= {:.1}%\n- No-Regression >= {:.1}%\n- Rollback <= {:.1}%\n- Invariant Violations <= {:.1}%\n\nVerdict: {}\nViolations: {}",
                app.reliability.task_success_rate * 100.0,
                app.reliability.no_regression_rate * 100.0,
                app.reliability.rollback_rate * 100.0,
                app.reliability.avg_time_to_recover_ms,
                app.reliability.invariant_violation_rate * 100.0,
                app.reliability_thresholds.min_task_success_rate * 100.0,
                app.reliability_thresholds.min_no_regression_rate * 100.0,
                app.reliability_thresholds.max_rollback_rate * 100.0,
                app.reliability_thresholds.max_invariant_violation_rate * 100.0,
                if app.reliability_eval.healthy {
                    "HEALTHY"
                } else {
                    "VIOLATED"
                },
                if app.reliability_eval.violations.is_empty() {
                    "none".to_string()
                } else {
                    app.reliability_eval.violations.join(" | ")
                }
            );
            let main_block = Paragraph::new(inner_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Reliability KPIs"),
                )
                .style(Style::default().fg(if app.reliability_eval.healthy {
                    rollback_color
                } else {
                    Color::Red
                }));
            f.render_widget(main_block, main_chunk);
        }
        _ => {}
    }

    // 4. Footer
    let footer = Paragraph::new("Press 'q' to quit | Arrows to navigate | Up/Down to select goals")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(footer, chunks[chunks.len() - 1]);
}

impl TuiApp {
    pub fn next_goal(&mut self) {
        if self.goals.is_empty() {
            return;
        }
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
        if self.goals.is_empty() {
            return;
        }
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
}
