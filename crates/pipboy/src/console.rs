//! Pure navigation and rendering; no collection, file access or terminal modes.

use pipboy_core::branding::PRODUCT_NAME;
use pipboy_core::catalog::CatalogSource;
use pipboy_core::notes::Note;
use pipboy_core::registry::{Readiness, MODULES};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap};

pub const PANELS: &[&str] = &["STAT", "LIBRARY", "NOTES", "MAP", "RADIO", "MESH"];

#[derive(Clone, Copy)]
pub enum Action {
    Next,
    Previous,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Default)]
pub struct Navigation {
    pub panel: usize,
    selected: [usize; 6],
    scroll: [u16; 6],
    max_scroll: [u16; 6],
    page_rows: [u16; 6],
}

impl Navigation {
    pub fn apply(&mut self, action: Action, item_count: usize) {
        let panel = self.panel;
        let last = item_count.saturating_sub(1);
        let step = self.page_rows[panel].saturating_sub(1).max(1);
        let collection = panel == 1 || panel == 2;
        let previous = self.selected[panel];
        match action {
            Action::Next => self.panel = (panel + 1) % PANELS.len(),
            Action::Previous => self.panel = (panel + PANELS.len() - 1) % PANELS.len(),
            Action::Up if collection => self.selected[panel] = previous.saturating_sub(1),
            Action::Down if collection => {
                self.selected[panel] = previous.saturating_add(1).min(last)
            }
            Action::Home if collection => self.selected[panel] = 0,
            Action::End if collection => self.selected[panel] = last,
            Action::Up => self.scroll[panel] = self.scroll[panel].saturating_sub(1),
            Action::Down => {
                self.scroll[panel] = self.scroll[panel]
                    .saturating_add(1)
                    .min(self.max_scroll[panel])
            }
            Action::PageUp => self.scroll[panel] = self.scroll[panel].saturating_sub(step),
            Action::PageDown => {
                self.scroll[panel] = self.scroll[panel]
                    .saturating_add(step)
                    .min(self.max_scroll[panel])
            }
            Action::Home => self.scroll[panel] = 0,
            Action::End => self.scroll[panel] = self.max_scroll[panel],
        }
        if collection && self.selected[panel] != previous {
            self.scroll[panel] = 0;
        }
    }
}

#[derive(Clone, Copy)]
pub struct View<'a> {
    pub stat: &'a str,
    pub library: &'a [CatalogSource],
    pub notes: &'a [Note],
}

pub fn render(frame: &mut Frame, navigation: &mut Navigation, view: View<'_>) {
    let area = frame.area();
    if area.width < 40 || area.height < 12 {
        frame.render_widget(
            Paragraph::new(format!(
                "{PRODUCT_NAME}\nResize to 40x12 or larger\nQ: quit | Tab: panel"
            )),
            area,
        );
        return;
    }
    let sections = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);
    if area.width < 70 {
        frame.render_widget(
            Paragraph::new(format!(
                "{} ({}/6) | Tab: panel",
                PANELS[navigation.panel],
                navigation.panel + 1
            ))
            .block(Block::bordered().title(PRODUCT_NAME)),
            sections[0],
        );
    } else {
        frame.render_widget(
            Tabs::new(PANELS.iter().copied())
                .block(Block::bordered().title(format!(" {PRODUCT_NAME} // RUST NATIVE ")))
                .select(navigation.panel)
                .style(Style::default().fg(Color::Gray))
                .highlight_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            sections[0],
        );
    }
    let (labels, detail) = match navigation.panel {
        0 => (Vec::new(), view.stat.to_owned()),
        1 => {
            let selected = navigation.selected[1].min(view.library.len().saturating_sub(1));
            navigation.selected[1] = selected;
            let labels = view
                .library
                .iter()
                .map(|entry| entry.project.clone())
                .collect();
            let detail = view
                .library
                .get(selected)
                .map(|entry| {
                    format!(
                        "{}\n{}\nLICENSE: {} / {}\nDISPOSITION: {}\n{}\n\n{}",
                        entry.project,
                        entry.repository,
                        entry.license,
                        entry.license_status,
                        entry.disposition,
                        entry.url(),
                        entry.summary,
                    )
                })
                .unwrap_or_else(|| "No source references available.".into());
            (labels, detail)
        }
        2 => {
            let selected = navigation.selected[2].min(view.notes.len().saturating_sub(1));
            navigation.selected[2] = selected;
            let labels = view
                .notes
                .iter()
                .rev()
                .map(|note| format!("Note #{}", note.id))
                .collect();
            let detail = view
                .notes
                .iter()
                .rev()
                .nth(selected)
                .map(|note| {
                    format!(
                        "NOTE #{} | PLAINTEXT\nCREATED: {} Unix seconds\n\n{}",
                        note.id, note.created_unix_s, note.text,
                    )
                })
                .unwrap_or_else(|| {
                    "No local notes.\nUse: pipboy notes add \"text\"\nR: reload notes".into()
                });
            (labels, detail)
        }
        panel => {
            let id = PANELS[panel].to_ascii_lowercase();
            let detail = MODULES
                .iter()
                .find(|module| module.id == id && module.readiness == Readiness::Planned)
                .map(|module| {
                    format!(
                        "{}: PLANNED / NOT CONNECTED\n\n{}",
                        module.title.to_uppercase(),
                        module.description
                    )
                })
                .unwrap_or_else(|| "Module unavailable".into());
            (Vec::new(), detail)
        }
    };
    let detail_area = if navigation.panel == 1 || navigation.panel == 2 {
        let regions = Layout::vertical([
            Constraint::Length(((sections[1].height as u32 * 2 / 5) as u16).max(3)),
            Constraint::Min(3),
        ])
        .split(sections[1]);
        let selected = navigation.selected[navigation.panel];
        let title = if labels.is_empty() {
            " ITEMS: 0 ".to_owned()
        } else {
            format!(" ITEMS: {}/{} | Up/Down ", selected + 1, labels.len())
        };
        let items: Vec<_> = labels.into_iter().map(ListItem::new).collect();
        let mut state = ListState::default().with_selected(if items.is_empty() {
            None
        } else {
            Some(selected)
        });
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(title))
                .highlight_symbol("> ")
                .highlight_style(Style::default().add_modifier(Modifier::BOLD)),
            regions[0],
            &mut state,
        );
        regions[1]
    } else {
        sections[1]
    };
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(detail_area);
    let paragraph = Paragraph::new(detail).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(inner.width);
    let panel = navigation.panel;
    let maximum = lines
        .saturating_sub(inner.height as usize)
        .min(u16::MAX as usize) as u16;
    navigation.max_scroll[panel] = maximum;
    navigation.page_rows[panel] = inner.height;
    navigation.scroll[panel] = navigation.scroll[panel].min(maximum);
    let offset = navigation.scroll[panel];
    frame.render_widget(
        paragraph
            .scroll((offset, 0))
            .block(block.title(format!(
                " TEXT {}/{} | PgUp/PgDn ",
                offset as usize + 1,
                maximum as usize + 1
            )))
            .style(Style::default().fg(Color::Green)),
        detail_area,
    );
    frame.render_widget(Paragraph::new("Tab: panel | Up/Down: item or text\nPgUp/PgDn: text | Home/End: first/last\nP: pause/resume | R: notes | Q: quit")
        .style(Style::default().fg(Color::Gray)), sections[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn screen(width: u16, height: u16, navigation: &mut Navigation, view: View<'_>) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, navigation, view))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn fallback_and_compact_layouts_render_without_panics() {
        let library = pipboy_core::catalog::sources().unwrap();
        let notes = vec![Note {
            id: 1,
            created_unix_s: 1234,
            text: "café 測試 fixture".into(),
        }];
        let view = View {
            stat: "STAT fixture",
            library: &library,
            notes: &notes,
        };
        for (width, height) in [
            (0, 0),
            (1, 1),
            (20, 5),
            (39, 11),
            (40, 12),
            (60, 16),
            (100, 24),
        ] {
            for (panel, name) in PANELS.iter().enumerate() {
                let mut navigation = Navigation {
                    panel,
                    ..Navigation::default()
                };
                let output = screen(width, height, &mut navigation, view);
                if width >= 20 && height >= 5 && (width < 40 || height < 12) {
                    assert!(output.contains("Resize to 40x12"));
                    assert!(output.contains("Q: quit"));
                }
                if width >= 40 && height >= 12 {
                    assert!(output.contains(name));
                    assert!(output.contains("Q: quit"));
                    if panel == 2 {
                        assert!(output.contains("Note #1"));
                    }
                }
            }
        }
    }

    #[test]
    fn navigation_bounds_empty_lists_and_preserves_panel_selection() {
        let mut navigation = Navigation::default();
        navigation.apply(Action::Previous, 0);
        assert_eq!(navigation.panel, 5);
        navigation.apply(Action::Next, 0);
        assert_eq!(navigation.panel, 0);
        navigation.apply(Action::Next, 0);
        navigation.apply(Action::End, 0);
        navigation.apply(Action::Down, 0);
        assert_eq!(navigation.selected[1], 0);
        navigation.apply(Action::End, 14);
        assert_eq!(navigation.selected[1], 13);
        navigation.apply(Action::Down, 14);
        assert_eq!(navigation.selected[1], 13);
        navigation.apply(Action::Next, 0);
        navigation.apply(Action::Previous, 0);
        assert_eq!(navigation.selected[1], 13);
        navigation.apply(Action::Home, 14);
        navigation.apply(Action::Up, 14);
        assert_eq!(navigation.selected[1], 0);
    }

    #[test]
    fn old_notes_and_wrapped_tail_are_reachable_then_resize_clamps_scroll() {
        let mut notes: Vec<_> = (1..=20)
            .map(|id| Note {
                id,
                created_unix_s: 1234,
                text: format!("fixture {id}"),
            })
            .collect();
        notes[0].text = format!("{} TAIL_MARKER", "測試 word ".repeat(90));
        let view = View {
            stat: "fixture",
            library: &[],
            notes: &notes,
        };
        let mut navigation = Navigation {
            panel: 2,
            ..Navigation::default()
        };
        screen(40, 12, &mut navigation, view);
        navigation.apply(Action::End, notes.len());
        let output = screen(40, 12, &mut navigation, view);
        assert!(output.contains("NOTE #1"));
        for _ in 0..250 {
            navigation.apply(Action::PageDown, notes.len());
            screen(40, 12, &mut navigation, view);
        }
        let output = screen(40, 12, &mut navigation, view);
        assert!(output.contains("TAIL_MARKER"));
        assert_eq!(navigation.scroll[2], navigation.max_scroll[2]);
        let output = screen(120, 50, &mut navigation, view);
        assert_eq!(navigation.scroll[2], 0);
        assert!(output.contains("NOTE #1"));
        navigation.apply(Action::Home, notes.len());
        assert_eq!(navigation.selected[2], 0);
        assert_eq!(navigation.scroll[2], 0);
    }

    #[test]
    fn empty_collections_and_planned_modules_have_explicit_text() {
        let view = View {
            stat: "fixture",
            library: &[],
            notes: &[],
        };
        let mut navigation = Navigation {
            panel: 2,
            ..Navigation::default()
        };
        assert!(screen(80, 24, &mut navigation, view).contains("No local notes"));
        navigation.panel = 1;
        assert!(screen(80, 24, &mut navigation, view).contains("No source references"));
        for panel in 3..6 {
            navigation.panel = panel;
            assert!(screen(80, 24, &mut navigation, view).contains("PLANNED / NOT CONNECTED"));
        }
    }
}
