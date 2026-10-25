use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use pipboy_core::catalog::sources;
use pipboy_core::metrics::{Collector, Snapshot};
use pipboy_core::notes::NoteStore;
use pipboy_core::registry::{Readiness, MODULES};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
            disable_raw_mode()?;
            return Err(Box::new(error));
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

const PANELS: &[&str] = &["STAT", "LIBRARY", "NOTES", "MAP", "RADIO", "MESH"];

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut collector = Collector::new();
    collector.prime();
    let mut snapshot = collector.sample();
    let library = sources()?;
    let store = NoteStore::from_environment()?;
    let mut panel = 0usize;
    let mut last_sample = Instant::now();

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    loop {
        let notes = if panel == 2 { Some(store.list()?) } else { None };
        terminal.draw(|frame| {
            let sections = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ]).split(frame.area());

            let tab = Tabs::new(PANELS.iter().copied())
                .block(Block::default().borders(Borders::ALL).title(" PIP_BOY // RUST NATIVE "))
                .select(panel)
                .style(Style::default().fg(Color::DarkGray))
                .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
            frame.render_widget(tab, sections[0]);

            let body = match panel {
                0 => format!(
                    "HOST: {} / {}\nCPU: {:.1}% ({} logical cores)\nRAM: {} / {} MiB\nUPTIME: {} seconds\nSOURCE: local sysinfo\n\nNo device, geolocation or network access.",
                    snapshot.os, snapshot.arch, snapshot.cpu_percent, snapshot.logical_cores,
                    snapshot.memory_used_mib, snapshot.memory_total_mib, snapshot.uptime_s
                ),
                1 => {
                    let mut text = format!("{} linked source projects | metadata snapshot only\n\n", library.len());
                    for entry in &library {
                        text.push_str(&format!("{} / {} / {}\n", entry.project, entry.license, entry.category));
                    }
                    text
                }
                2 => {
                    let mut text = String::from("LOCAL NOTES (plaintext; user-owned)\nAdd notes with: pipboy notes add \"text\"\n\n");
                    if let Some(ref notes) = notes {
                        for note in notes.iter().rev().take(12) {
                            text.push_str(&format!("#{}  {}\n", note.id, note.text));
                        }
                    }
                    text
                }
                index => {
                    let module_id = PANELS[index].to_ascii_lowercase();
                    let module = MODULES.iter().find(|value| value.id == module_id);
                    match module {
                        Some(module) if module.readiness == Readiness::Planned =>
                            format!("{}: PLANNED / NOT CONNECTED\n\n{}\n\nThis panel does not invent live devices, fixes, peer links or signals.", module.title.to_uppercase(), module.description),
                        _ => "Module not available".to_owned(),
                    }
                }
            };
            frame.render_widget(
                Paragraph::new(body)
                    .block(Block::default().borders(Borders::ALL).title(" CONSOLE "))
                    .style(Style::default().fg(Color::Green))
                    .wrap(Wrap { trim: false }),
                sections[1],
            );
            frame.render_widget(
                Paragraph::new(" LEFT/RIGHT or TAB: panel    Q: quit    Offline-first / read-only host telemetry ")
                    .style(Style::default().fg(Color::Gray)),
                sections[2],
            );
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Right | KeyCode::Tab => panel = (panel + 1) % PANELS.len(),
                    KeyCode::Left | KeyCode::BackTab => panel = (panel + PANELS.len() - 1) % PANELS.len(),
                    _ => {}
                }
            }
        }
        if last_sample.elapsed() >= Duration::from_secs(2) {
            snapshot = collector.sample();
            last_sample = Instant::now();
        }
    }
    Ok(())
}
