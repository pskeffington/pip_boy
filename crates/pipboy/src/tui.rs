use crate::console::{self, Action, Navigation, View};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
use pipboy_core::workbench::{WorkbenchStore, PALETTE, PRESETS};
use pipboy_core::runtime::{Capability, LocalOperation, ModuleHost, PermissionSet, PollOutcome};
use ratatui::prelude::*;
use std::error::Error;
use std::io;
use std::time::{Duration, Instant};

pub(crate) struct TerminalGuard;

impl TerminalGuard {
    pub(crate) fn enter() -> Result<Self, Box<dyn Error>> {
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

fn exit_requested(key: KeyEvent) -> bool {
    key.kind == KeyEventKind::Press
        && (matches!(key.code, KeyCode::Esc | KeyCode::Char('q' | 'Q'))
            || (matches!(key.code, KeyCode::Char('c' | 'C'))
                && key.modifiers.contains(KeyModifiers::CONTROL)
                && (key.modifiers - (KeyModifiers::CONTROL | KeyModifiers::SHIFT)).is_empty()))
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
    let bench_store = WorkbenchStore::from_environment()?;
    let mut workbench = bench_store.load()?;
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
                    workbench: &workbench,
                },
            )
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if navigation.panel == 3 && navigation.profiles_open {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                            navigation.profile_confirm = false;
                            navigation.profiles_open = false;
                        }
                        KeyCode::Enter => navigation.profile_confirm = true,
                        KeyCode::Char('y' | 'Y') if navigation.profile_confirm => {
                            if let Some(entry) = PRESETS.get(navigation.profile_selected) {
                                bench_store.apply_preset(entry.id)?;
                                workbench = bench_store.load()?;
                                navigation.focus_last_bench_slot(&workbench);
                                navigation.profile_confirm = false;
                                navigation.profiles_open = false;
                            }
                        }
                        KeyCode::Up => navigation.profile_move(Action::Up),
                        KeyCode::Down => navigation.profile_move(Action::Down),
                        KeyCode::PageUp => navigation.profile_move(Action::PageUp),
                        KeyCode::PageDown => navigation.profile_move(Action::PageDown),
                        KeyCode::Home => navigation.profile_move(Action::Home),
                        KeyCode::End => navigation.profile_move(Action::End),
                        KeyCode::Tab | KeyCode::Right => navigation.apply(Action::Next, 0),
                        KeyCode::BackTab | KeyCode::Left => {
                            navigation.apply(Action::Previous, 0);
                        }
                        _ if exit_requested(key) => break,
                        _ => {}
                    }
                    continue;
                }
                if navigation.panel == 3 && navigation.palette_open {
                    match key.code {
                        KeyCode::Esc => navigation.palette_open = false,
                        KeyCode::Enter => {
                            if let Some(entry) = PALETTE.get(navigation.palette_selected) {
                                if entry.availability.can_pin() {
                                    if !workbench.slots.iter().any(|slot| slot.tool == entry.id) {
                                        bench_store.add(entry.id)?;
                                        workbench = bench_store.load()?;
                                    }
                                    navigation.focus_last_bench_slot(&workbench);
                                    navigation.palette_open = false;
                                }
                            }
                        }
                        KeyCode::Up => navigation.palette_move(Action::Up),
                        KeyCode::Down => navigation.palette_move(Action::Down),
                        KeyCode::PageUp => navigation.palette_move(Action::PageUp),
                        KeyCode::PageDown => navigation.palette_move(Action::PageDown),
                        KeyCode::Home => navigation.palette_move(Action::Home),
                        KeyCode::End => navigation.palette_move(Action::End),
                        KeyCode::Tab | KeyCode::Right => navigation.apply(Action::Next, 0),
                        KeyCode::BackTab | KeyCode::Left => {
                            navigation.apply(Action::Previous, 0)
                        }
                        _ if exit_requested(key) => break,
                        _ => {}
                    }
                    continue;
                }
                if exit_requested(key) {
                    break;
                }
                match key.code {
                    KeyCode::Char('p' | 'P') if navigation.panel == 3 => {
                        navigation.profiles_open = true;
                        navigation.profile_confirm = false;
                        navigation.profile_selected = PRESETS
                            .iter()
                            .position(|entry| entry.id == workbench.profile)
                            .unwrap_or(0);
                    }
                    KeyCode::Char('a' | 'A') if navigation.panel == 3 => {
                        navigation.palette_open = true;
                    }
                    KeyCode::Char('x' | 'X') | KeyCode::Delete if navigation.panel == 3 => {
                        if let Some(id) = navigation.selected_bench_slot(&workbench) {
                            bench_store.remove(id)?;
                            workbench = bench_store.load()?;
                        }
                    }
                    KeyCode::Char('r' | 'R') if navigation.panel == 3 => {
                        workbench = bench_store.load()?;
                    }
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
                        if navigation.panel == 3 {
                            workbench = bench_store.load()?;
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
                        let count = match navigation.panel {
                            1 => library.len(),
                            2 => notes.len(),
                            3 => workbench.slots.len(),
                            _ => 0,
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
    fn interrupt_requires_an_explicit_press_and_preserves_other_shortcuts() {
        for modifiers in [
            KeyModifiers::CONTROL,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ] {
            assert!(exit_requested(KeyEvent::new(KeyCode::Char('c'), modifiers)));
        }
        for modifiers in [
            KeyModifiers::NONE,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ] {
            assert!(!exit_requested(KeyEvent::new(
                KeyCode::Char('c'),
                modifiers
            )));
        }
        let mut release = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        release.kind = KeyEventKind::Release;
        assert!(!exit_requested(release));
    }

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
