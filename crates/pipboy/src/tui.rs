use crate::console::{self, Action, Navigation, View};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use pipboy_core::catalog::sources;
use pipboy_core::host_report::{self, HostReadings};
use pipboy_core::host_telemetry::HostTelemetryModule;
use pipboy_core::lifecycle::State;
use pipboy_core::metrics::Collector;
use pipboy_core::notes::NoteStore;
use pipboy_core::observation::Observation;
use pipboy_core::runtime::{Capability, LocalOperation, ModuleHost, PermissionSet, PollOutcome};
use ratatui::prelude::*;
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

pub fn run() -> Result<(), Box<dyn Error>> {
    let module = HostTelemetryModule::new(|| Ok(Collector::new()), 60, Duration::from_secs(4))?;
    let session_start = module.session_start();
    let mut host = ModuleHost::new(PermissionSet::offline_read_only());
    host.register(Box::new(module))?;
    host.start("stat")?;
    let (initial, mut history_info) = read_stat(&mut host)?;
    let mut latest = Some(initial);
    let library = host.run_local(LocalOperation::LibraryRead, sources)?;
    let store = NoteStore::from_environment()?;
    let mut navigation = Navigation::default();
    let mut notes = Vec::new();
    let mut last_sample = Instant::now();

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    loop {
        let session_ms = u64::try_from(session_start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let state = host.state("stat").unwrap_or(State::Unavailable);
        let body_stat = stat_body(latest.as_ref(), state, &history_info, session_ms);
        terminal.draw(|frame| {
            console::render(
                frame,
                &mut navigation,
                View {
                    stat: &body_stat,
                    library: &library,
                    notes: &notes,
                },
            )
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Right | KeyCode::Tab | KeyCode::Left | KeyCode::BackTab => {
                        let action = if matches!(key.code, KeyCode::Right | KeyCode::Tab) {
                            Action::Next
                        } else {
                            Action::Previous
                        };
                        navigation.apply(action, 0);
                        if navigation.panel == 2 {
                            notes = host.run_local(LocalOperation::NotesRead, || store.list())?;
                        }
                    }
                    KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End => {
                        let action = match key.code {
                            KeyCode::Up => Action::Up,
                            KeyCode::Down => Action::Down,
                            KeyCode::PageUp => Action::PageUp,
                            KeyCode::PageDown => Action::PageDown,
                            KeyCode::Home => Action::Home,
                            _ => Action::End,
                        };
                        let count = if navigation.panel == 1 {
                            library.len()
                        } else {
                            notes.len()
                        };
                        navigation.apply(action, count);
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') if navigation.panel == 2 => {
                        notes = host.run_local(LocalOperation::NotesRead, || store.list())?
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

fn read_stat(host: &mut ModuleHost) -> Result<(Observation<HostReadings>, String), String> {
    let poll = host
        .poll_once()
        .into_iter()
        .find(|poll| poll.module_id == "stat")
        .ok_or("missing STAT module")?;
    match poll.outcome {
        PollOutcome::Sample { data } => Ok((
            data.host_report.ok_or("missing STAT field report")?,
            data.detail,
        )),
        other => Err(format!("STAT did not sample: {other:?}")),
    }
}

fn stat_body(
    observation: Option<&Observation<HostReadings>>,
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
    host_report::refresh(&mut observation, now);
    let Some(readings) = observation.value.as_ref() else {
        return "STAT: UNKNOWN / no reading".into();
    };
    format!(
        "STAT: {state:?}\n{}\nSOURCE: {} | {:?} | age {} ms\nHISTORY: {} (memory only)\n\nP pauses collection and clears buffered readings.",
        host_report::display_fields(readings),
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

    #[test]
    fn dashboard_distinguishes_warmup_and_stale_values_without_collecting() {
        let sample = pipboy_core::metrics::Snapshot {
            observed_unix_s: 1234,
            os: "fixture".into(),
            arch: "test".into(),
            cpu_percent: None,
            logical_cores: 2,
            memory_used_mib: 0,
            memory_total_mib: 64,
            uptime_s: 0,
        };
        let raw = Observation::new(
            "host.stats",
            "fixture",
            None,
            100,
            None,
            Some(sample),
            pipboy_core::observation::Quality::Valid,
            Some(500),
        )
        .unwrap();
        let report = host_report::describe(&raw, None);
        let current = stat_body(Some(&report), State::Running, "1 retained / 0 dropped", 100);
        assert!(current.contains("CPU: Unknown (warming up or unavailable)"));
        assert!(current.contains("RAM: 0 [Valid] / 64 [Valid] MiB"));
        assert!(current.contains("first poll; interval unknown"));
        let stale = stat_body(Some(&report), State::Running, "history", 600);
        assert!(stale.contains("64 [Stale]"));
        assert!(stale.contains("CPU: Unknown"));
        let paused = stat_body(Some(&report), State::Disabled, "old history", 600);
        assert!(!paused.contains("fixture"));
        assert!(!paused.contains("64"));
        assert!(!paused.contains("old history"));
    }
}
