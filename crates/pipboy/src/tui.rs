use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use pipboy_core::branding::{CLI_COMMAND, PRODUCT_NAME};
use pipboy_core::catalog::sources;
use pipboy_core::host_telemetry::HostTelemetryModule;
use pipboy_core::lifecycle::State;
use pipboy_core::metrics::{Collector, Snapshot};
use pipboy_core::notes::NoteStore;
use pipboy_core::observation::Observation;
use pipboy_core::registry::{Readiness, MODULES};
use pipboy_core::runtime::{Capability, ModuleHost, PermissionSet, PollOutcome};
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
    let module = HostTelemetryModule::new(
        || {
            let mut collector = Collector::new();
            collector.prime();
            Ok(collector)
        },
        60,
        Duration::from_secs(4),
    )?;
    let session_start = module.session_start();
    let mut host = ModuleHost::new(PermissionSet::offline_read_only());
    host.register(Box::new(module))?;
    host.start("stat")?;
    let (initial, mut history_info) = read_stat(&mut host)?;
    let mut latest = Some(initial);
    let library = sources()?;
    let store = NoteStore::from_environment()?;
    let mut panel = 0usize;
    let mut last_sample = Instant::now();

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    loop {
        let notes = if panel == 2 {
            Some(store.list()?)
        } else {
            None
        };
        let session_ms = u64::try_from(session_start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let state = host.state("stat").unwrap_or(State::Unavailable);
        let body_stat = stat_body(latest.as_ref(), state, &history_info, session_ms);
        terminal.draw(|frame| {
            let sections = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ]).split(frame.area());

            let tab = Tabs::new(PANELS.iter().copied())
                .block(Block::default().borders(Borders::ALL).title(format!(" {PRODUCT_NAME} // RUST NATIVE ")))
                .select(panel)
                .style(Style::default().fg(Color::DarkGray))
                .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
            frame.render_widget(tab, sections[0]);

            let body = match panel {
                0 => body_stat,
                1 => {
                    let mut text = format!("{} linked source projects | metadata snapshot only\n\n", library.len());
                    for entry in &library {
                        text.push_str(&format!("{} / {} / {}\n", entry.project, entry.license, entry.category));
                    }
                    text
                }
                2 => {
                    let mut text = format!("LOCAL NOTES (plaintext; user-owned)\nAdd notes with: {CLI_COMMAND} notes add \"text\"\n\n");
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
                Paragraph::new(" LEFT/RIGHT or TAB: panel    P: pause/resume STAT    Q: quit ")
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
                    KeyCode::Left | KeyCode::BackTab => {
                        panel = (panel + PANELS.len() - 1) % PANELS.len()
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') => {
                        if host.permissions().allows(Capability::HostRead) {
                            host.revoke(Capability::HostRead);
                            latest = None;
                            history_info.clear();
                        } else {
                            host.grant(Capability::HostRead);
                            host.start("stat")?;
                            let (observation, detail) = read_stat(&mut host)?;
                            latest = Some(observation);
                            history_info = detail;
                        }
                        last_sample = Instant::now();
                    }
                    _ => {}
                }
            }
        }
        if matches!(host.state("stat"), Some(State::Running | State::Degraded))
            && last_sample.elapsed() >= Duration::from_secs(2)
        {
            let (observation, detail) = read_stat(&mut host)?;
            latest = Some(observation);
            history_info = detail;
            last_sample = Instant::now();
        }
    }
    Ok(())
}

fn read_stat(host: &mut ModuleHost) -> Result<(Observation<Snapshot>, String), String> {
    let poll = host
        .poll_once()
        .into_iter()
        .find(|poll| poll.module_id == "stat")
        .ok_or("missing STAT module")?;
    match poll.outcome {
        PollOutcome::Sample { data } => Ok((
            data.host_observation.ok_or("missing STAT observation")?,
            data.detail,
        )),
        other => Err(format!("STAT did not sample: {other:?}")),
    }
}

fn stat_body(
    observation: Option<&Observation<Snapshot>>,
    state: State,
    history: &str,
    now: u64,
) -> String {
    let Some(mut observation) = observation
        .cloned()
        .filter(|_| matches!(state, State::Running | State::Degraded))
    else {
        return format!("STAT: {state:?} / PAUSED\nHost read permission revoked. Buffered readings cleared.\nPress P to grant local host read access and resume.");
    };
    observation.update_freshness(now);
    let Some(snapshot) = observation.value.as_ref() else {
        return "STAT: UNKNOWN / no reading".into();
    };
    format!(
        "STAT: {state:?}\nHOST: {} / {}\nCPU: {:.1}% ({} logical cores)\nRAM: {} / {} MiB\nUPTIME: {} seconds\nSOURCE: {} | {:?} | age {} ms\nHISTORY: {} (memory only)\n\nP pauses collection and clears buffered readings.",
        snapshot.os, snapshot.arch, snapshot.cpu_percent, snapshot.logical_cores,
        snapshot.memory_used_mib, snapshot.memory_total_mib, snapshot.uptime_s,
        observation.provenance, observation.quality, now.saturating_sub(observation.captured_session_ms), history,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pause_message_has_no_host_values_and_explains_resume() {
        let text = stat_body(None, State::Disabled, "old history", 1000);
        assert!(text.contains("PAUSED"));
        assert!(text.contains("Press P"));
        assert!(!text.contains("CPU:"));
        assert!(!text.contains("old history"));
    }
}
