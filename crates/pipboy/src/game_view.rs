//! Pure game browser navigation and rendering; no network or terminal-mode I/O.
use pipboy_core::game::Database;
use ratatui::{
    prelude::*,
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};

pub enum Action {
    Up,
    Down,
    Open,
    Back,
    Root,
    PageUp,
    PageDown,
    Home,
    End,
}

struct Page {
    name: String,
    id: u32,
    key: Option<String>,
    list: ListState,
    scroll: u16,
    max_scroll: u16,
    rows: u16,
}

impl Page {
    fn new(name: String, id: u32) -> Self {
        Self {
            name,
            id,
            key: None,
            list: ListState::default(),
            scroll: 0,
            max_scroll: 0,
            rows: 0,
        }
    }

    fn sync(&mut self, children: &[(String, u32)]) {
        let index = self
            .key
            .as_ref()
            .and_then(|key| children.iter().position(|child| &child.0 == key))
            .unwrap_or(self.list.selected().unwrap_or(0))
            .min(children.len().saturating_sub(1));
        let key = children.get(index).map(|child| child.0.clone());
        if key != self.key {
            self.scroll = 0;
        }
        self.key = key;
        self.list.select(self.key.as_ref().map(|_| index));
    }
}

pub struct Browser {
    pages: Vec<Page>,
}

impl Default for Browser {
    fn default() -> Self {
        Self {
            pages: vec![Page::new("Fallout 4".to_owned(), 0)],
        }
    }
}

impl Browser {
    fn reconcile(&mut self, database: &Database) {
        // Follow replacement references by path key; return to a surviving parent
        // if an opened object disappears instead of displaying an orphan as live.
        for index in 1..self.pages.len() {
            let parent = self.pages[index - 1].id;
            let name = &self.pages[index].name;
            if let Some((_, id)) = database
                .children(parent)
                .iter()
                .find(|child| &child.0 == name)
            {
                if self.pages[index].id != *id {
                    self.pages[index] = Page::new(name.clone(), *id);
                }
            } else {
                self.pages.truncate(index);
                break;
            }
        }
    }

    pub fn apply(&mut self, action: Action, database: &Database) {
        self.reconcile(database);
        let page = self.pages.last_mut().unwrap();
        let children = database.children(page.id);
        page.sync(&children);
        let selected = page.list.selected().unwrap_or(0);
        match action {
            Action::Up | Action::Down => {
                let index = if matches!(action, Action::Up) {
                    selected.saturating_sub(1)
                } else {
                    selected
                        .saturating_add(1)
                        .min(children.len().saturating_sub(1))
                };
                if index != selected {
                    page.key = children.get(index).map(|child| child.0.clone());
                    page.list.select(page.key.as_ref().map(|_| index));
                    page.scroll = 0;
                }
            }
            Action::Open => {
                if let Some((name, id)) = children.get(selected) {
                    if self.pages.len() < 32 {
                        self.pages.push(Page::new(name.clone(), *id));
                    }
                }
            }
            Action::Back => {
                if self.pages.len() > 1 {
                    self.pages.pop();
                }
            }
            Action::Root => self.pages.truncate(1),
            Action::PageUp => {
                page.scroll = page
                    .scroll
                    .saturating_sub(page.rows.saturating_sub(1).max(1))
            }
            Action::PageDown => {
                page.scroll = page
                    .scroll
                    .saturating_add(page.rows.saturating_sub(1).max(1))
                    .min(page.max_scroll)
            }
            Action::Home => page.scroll = 0,
            Action::End => page.scroll = page.max_scroll,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, database: &Database, header: &str) {
        self.reconcile(database);
        let area = frame.area();
        if area.width < 40 || area.height < 12 {
            frame.render_widget(
                Paragraph::new("Rust-Boy GAME\nResize to 40x12 or larger\nQ/Esc/Ctrl+C: exit"),
                area,
            );
            return;
        }
        let sections = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);
        frame.render_widget(Paragraph::new(clean(header, 1024, true)), sections[0]);
        let panels = if area.width >= 80 {
            Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(sections[1])
        } else {
            Layout::vertical([
                Constraint::Length((sections[1].height / 3).max(3)),
                Constraint::Min(3),
            ])
            .split(sections[1])
        };
        let path = clean(
            &self
                .pages
                .iter()
                .map(|page| page.name.as_str())
                .collect::<Vec<_>>()
                .join(" / "),
            160,
            false,
        );
        let page = self.pages.last_mut().unwrap();
        let children = database.children(page.id);
        page.sync(&children);
        let selected = page.list.selected();
        let item = selected.and_then(|index| children.get(index));
        let selected_id = item.map(|child| child.1).unwrap_or(page.id);
        let detail_title = item
            .map(|child| clean(&child.0, 80, false))
            .unwrap_or_else(|| clean(&page.name, 80, false));
        let items: Vec<_> = children
            .iter()
            .map(|(name, _)| ListItem::new(clean(name, 200, false)))
            .collect();
        let title = format!(
            " {path} | {}/{} ",
            selected.map(|index| index + 1).unwrap_or(0),
            items.len()
        );
        if items.is_empty() {
            frame.render_widget(
                Paragraph::new("No child objects; view the current value.")
                    .wrap(Wrap { trim: false })
                    .block(Block::bordered().title(title)),
                panels[0],
            );
        } else {
            frame.render_stateful_widget(
                List::new(items)
                    .block(Block::bordered().title(title))
                    .highlight_symbol("> ")
                    .highlight_style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                panels[0],
                &mut page.list,
            );
        }
        let detail = serde_json::to_string_pretty(&database.view(selected_id)).unwrap();
        let detail = clean(&detail, 262144, true);
        let block = Block::bordered();
        let inner = block.inner(panels[1]);
        let paragraph = Paragraph::new(detail).wrap(Wrap { trim: false });
        page.max_scroll = paragraph
            .line_count(inner.width)
            .saturating_sub(inner.height as usize)
            .min(u16::MAX as usize) as u16;
        page.rows = inner.height;
        page.scroll = page.scroll.min(page.max_scroll);
        frame.render_widget(
            paragraph
                .scroll((page.scroll, 0))
                .block(block.title(format!(
                    " {detail_title} | text {}/{} ",
                    page.scroll as usize + 1,
                    page.max_scroll as usize + 1
                ))),
            panels[1],
        );
        frame.render_widget(
            Paragraph::new(if area.width >= 80 {
                "Up/Down: select | Enter/Right: open | Backspace/Left: back\nPgUp/PgDn, Home/End: text | R: root | Q/Esc/Ctrl+C: exit"
            } else {
                "↑↓: item | Enter: open | ←/Bksp: back\nPgUp/Dn: text | R: root | Q/Ctrl+C: exit"
            }),
            sections[2],
        );
    }
}

fn clean(text: &str, limit: usize, newlines: bool) -> String {
    text.chars()
        .filter(|character| !character.is_control() || (newlines && *character == '\n'))
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    fn object(id: u32, fields: &[(&str, u32)], removed: &[u32]) -> Vec<u8> {
        let mut wire = vec![8];
        wire.extend(id.to_le_bytes());
        wire.extend((fields.len() as u16).to_le_bytes());
        for (name, child) in fields {
            wire.extend(child.to_le_bytes());
            wire.extend(name.as_bytes());
            wire.push(0);
        }
        wire.extend((removed.len() as u16).to_le_bytes());
        for child in removed {
            wire.extend(child.to_le_bytes());
        }
        wire
    }

    fn unsigned(id: u32, value: u32) -> Vec<u8> {
        let mut wire = vec![4];
        wire.extend(id.to_le_bytes());
        wire.extend(value.to_le_bytes());
        wire
    }

    fn text(id: u32, value: &str) -> Vec<u8> {
        let mut wire = vec![6];
        wire.extend(id.to_le_bytes());
        wire.extend(value.as_bytes());
        wire.push(0);
        wire
    }

    fn fixture() -> Database {
        let mut wire = object(0, &[("Inventory", 1), ("Map", 2), ("Stats", 3)], &[]);
        let names: Vec<_> = (0..40).map(|index| format!("Item {index:02}")).collect();
        let fields: Vec<_> = names
            .iter()
            .enumerate()
            .map(|(index, name)| (name.as_str(), 100 + index as u32))
            .collect();
        wire.extend(object(1, &fields, &[]));
        for index in 0..40 {
            wire.extend(unsigned(100 + index, 1000 + index));
        }
        wire.extend(object(2, &[("Player X", 200)], &[]));
        wire.extend(unsigned(200, 42));
        wire.extend(text(
            3,
            &format!("DETAIL_BEGIN {} DETAIL_TAIL", "Long Unicode Ω ".repeat(180)),
        ));
        let mut database = Database::default();
        database.update(&wire).unwrap();
        database
    }

    fn screen(width: u16, height: u16, browser: &mut Browser, database: &Database) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                browser.render(
                    frame,
                    database,
                    "GAME TEST / playstation\nCONNECTED | read-only\nfixture",
                )
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn large_inventory_selection_stays_visible_and_back_restores_context() {
        let database = fixture();
        let mut browser = Browser::default();
        browser.apply(Action::Open, &database);
        for _ in 0..25 {
            browser.apply(Action::Down, &database);
        }
        let view = screen(100, 24, &mut browser, &database);
        assert!(view.contains("> Item 25"), "{view}");
        assert!(view.contains("1025"));
        browser.apply(Action::Open, &database);
        assert_eq!(browser.pages.len(), 3);
        browser.apply(Action::Back, &database);
        assert_eq!(
            browser.pages.last().unwrap().key.as_deref(),
            Some("Item 25")
        );
        assert!(screen(100, 24, &mut browser, &database).contains("> Item 25"));
        browser.apply(Action::Root, &database);
        assert_eq!(browser.pages.len(), 1);
        assert_eq!(browser.pages[0].key.as_deref(), Some("Inventory"));
    }

    #[test]
    fn named_selection_and_open_paths_follow_replacements_and_removals() {
        let mut database = fixture();
        let mut browser = Browser::default();
        browser.apply(Action::Down, &database);
        let mut update = object(0, &[("AAA", 400)], &[]);
        update.extend(unsigned(400, 1));
        database.update(&update).unwrap();
        screen(100, 24, &mut browser, &database);
        assert_eq!(browser.pages[0].key.as_deref(), Some("Map"));
        assert_eq!(browser.pages[0].list.selected(), Some(2));
        browser.apply(Action::Open, &database);
        assert_eq!(browser.pages[1].id, 2);

        let mut update = object(0, &[("Map", 500)], &[]);
        update.extend(object(500, &[("Replacement", 501)], &[]));
        update.extend(unsigned(501, 777));
        database.update(&update).unwrap();
        let view = screen(100, 24, &mut browser, &database);
        assert_eq!(browser.pages[1].id, 500);
        assert!(view.contains("Replacement") && view.contains("777"));

        database.update(&object(0, &[], &[500])).unwrap();
        screen(100, 24, &mut browser, &database);
        assert_eq!(browser.pages.len(), 1);
        assert_eq!(browser.pages[0].key.as_deref(), Some("Stats"));
    }

    #[test]
    fn text_scroll_clamps_and_keeps_the_tail_visible_after_resize() {
        let database = fixture();
        let mut browser = Browser::default();
        browser.apply(Action::Down, &database);
        browser.apply(Action::Down, &database);
        screen(40, 12, &mut browser, &database);
        browser.apply(Action::End, &database);
        let view = screen(40, 12, &mut browser, &database);
        assert!(browser.pages[0].scroll > 0);
        assert!(view.contains("DETAIL_TAIL"), "{view}");
        let before = browser.pages[0].scroll;
        for _ in 0..100 {
            browser.apply(Action::PageDown, &database);
        }
        assert_eq!(browser.pages[0].scroll, before);
        assert!(screen(120, 80, &mut browser, &database).contains("DETAIL_TAIL"));
        assert!(browser.pages[0].scroll < before);
        browser.apply(Action::Home, &database);
        assert!(screen(100, 24, &mut browser, &database).contains("DETAIL_BEGIN"));
    }

    #[test]
    fn tiny_empty_and_populated_views_render_without_losing_navigation() {
        let database = fixture();
        let mut browser = Browser::default();
        browser.apply(Action::Open, &database);
        for _ in 0..25 {
            browser.apply(Action::Down, &database);
        }
        for (width, height) in [
            (0, 0),
            (1, 1),
            (20, 5),
            (39, 11),
            (40, 12),
            (60, 16),
            (80, 24),
            (100, 24),
        ] {
            let view = screen(width, height, &mut browser, &database);
            if width == 20 {
                assert!(view.contains("Resize to 40x12"));
            }
            assert_eq!(
                browser.pages.last().unwrap().key.as_deref(),
                Some("Item 25")
            );
        }
        assert!(screen(100, 24, &mut browser, &database).contains("> Item 25"));
        let view = screen(100, 24, &mut Browser::default(), &Database::default());
        assert!(view.contains("No child objects"));
        assert!(view.contains("awaiting reference"));
    }

    #[test]
    fn game_text_cannot_inject_terminal_controls_or_label_lines() {
        let mut database = Database::default();
        let mut wire = object(0, &[("Bad\u{1b}[31m\n\r\tName", 1)], &[]);
        wire.extend(text(1, "value\u{1b}]0;title\u{7}"));
        database.update(&wire).unwrap();
        let mut browser = Browser::default();
        let view = screen(100, 24, &mut browser, &database);
        assert!(view.contains("Bad[31mName"));
        for control in ['\u{1b}', '\r', '\t', '\u{7}'] {
            assert!(!view.contains(control));
        }
        assert_eq!(clean("header\u{1b}\nnext", 100, true), "header\nnext");
    }

    #[test]
    fn empty_actions_and_cyclic_navigation_have_a_fixed_depth_limit() {
        let mut browser = Browser::default();
        let mut database = Database::default();
        for action in [
            Action::Open,
            Action::Back,
            Action::Up,
            Action::Down,
            Action::PageUp,
            Action::PageDown,
            Action::End,
        ] {
            browser.apply(action, &database);
        }
        assert_eq!(browser.pages.len(), 1);
        assert_eq!(browser.pages[0].list.selected(), None);
        database.update(&object(0, &[("Loop", 0)], &[])).unwrap();
        for _ in 0..100 {
            browser.apply(Action::Open, &database);
        }
        assert_eq!(browser.pages.len(), 32);
        assert!(screen(100, 24, &mut browser, &database).contains("[cycle]"));
        browser.apply(Action::Root, &database);
        browser.apply(Action::Back, &database);
        assert_eq!(browser.pages.len(), 1);
    }
}
