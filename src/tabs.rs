//! Pure tab metadata; session identity is deliberately independent of display order.

use std::num::NonZeroU32;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TabId(NonZeroU32);

impl TabId {
    pub fn new(value: u32) -> Option<Self> {
        NonZeroU32::new(value).map(Self)
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TabName {
    pub manual: Option<String>,
    pub automatic: Option<String>,
}

pub fn clean_title(text: &str) -> Option<String> {
    let stripped: String = text.chars().filter(|c| !c.is_control()).collect();
    let text: String = stripped.trim().chars().take(256).collect();
    (!text.is_empty()).then_some(text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub cwd: PathBuf,
    pub name: TabName,
    pub ended: bool,
    pub failed: bool,
}

impl Tab {
    pub fn display_name(&self) -> String {
        if let Some(name) = self.name.manual.as_ref().or(self.name.automatic.as_ref()) {
            return name.clone();
        }
        if std::env::var_os("HOME").is_some_and(|home| self.cwd == home) {
            return "~".into();
        }
        self.cwd
            .file_name()
            .map_or_else(|| "/".into(), |name| name.to_string_lossy().into_owned())
    }
}

#[derive(Debug)]
pub struct Tabs {
    ordered: Vec<Tab>,
    active: Option<TabId>,
    next_id: u32,
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new()
    }
}

impl Tabs {
    pub fn new() -> Self {
        Self {
            ordered: Vec::new(),
            active: None,
            next_id: 1,
        }
    }

    pub fn reserve_id(&mut self) -> Result<TabId, String> {
        let id = TabId::new(self.next_id).ok_or("session ID space exhausted")?;
        self.next_id = self.next_id.checked_add(1).unwrap_or(0);
        Ok(id)
    }

    pub fn insert(&mut self, id: TabId, cwd: PathBuf, name: TabName) {
        if self.ordered.iter().any(|tab| tab.id == id) {
            return;
        }
        let name = TabName {
            manual: name.manual.as_deref().and_then(clean_title),
            automatic: name.automatic.as_deref().and_then(clean_title),
        };
        self.ordered.push(Tab {
            id,
            cwd,
            name,
            ended: false,
            failed: false,
        });
        self.active = Some(id);
    }

    pub fn active(&self) -> Option<TabId> {
        self.active
    }

    pub fn ordered(&self) -> &[Tab] {
        &self.ordered
    }

    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.ordered.iter().find(|tab| tab.id == id)
    }

    fn get_mut(&mut self, id: TabId) -> Option<&mut Tab> {
        self.ordered.iter_mut().find(|tab| tab.id == id)
    }

    pub fn remove(&mut self, id: TabId) -> bool {
        let Some(index) = self.ordered.iter().position(|tab| tab.id == id) else {
            return false;
        };
        self.ordered.remove(index);
        if self.active == Some(id) {
            self.active = self
                .ordered
                .get(index)
                .or_else(|| self.ordered.last())
                .map(|tab| tab.id);
        }
        true
    }

    pub fn select(&mut self, id: TabId) -> bool {
        if self.get(id).is_none() {
            return false;
        }
        self.active = Some(id);
        true
    }

    pub fn select_number(&mut self, n: u8) {
        let tab = match n {
            9 => self.ordered.last(),
            1..=8 => self.ordered.get(n as usize - 1),
            _ => None,
        };
        if let Some(tab) = tab {
            self.active = Some(tab.id);
        }
    }

    pub fn cycle(&mut self, backwards: bool) {
        if self.ordered.is_empty() {
            return;
        }
        let index = self
            .ordered
            .iter()
            .position(|tab| Some(tab.id) == self.active)
            .unwrap_or(0);
        let offset = if backwards { self.ordered.len() - 1 } else { 1 };
        self.active = Some(self.ordered[(index + offset) % self.ordered.len()].id);
    }

    pub fn rename(&mut self, id: TabId, text: &str) {
        if let Some(tab) = self.get_mut(id) {
            tab.name.manual = clean_title(text);
        }
    }
    pub fn set_auto_title(&mut self, id: TabId, text: &str) {
        if let Some(tab) = self.get_mut(id) {
            tab.name.automatic = clean_title(text);
        }
    }
    pub fn set_cwd(&mut self, id: TabId, cwd: PathBuf) {
        if let Some(tab) = self.get_mut(id) {
            tab.cwd = cwd;
        }
    }
    pub fn mark_ended(&mut self, id: TabId) {
        if let Some(tab) = self.get_mut(id) {
            tab.ended = true;
        }
    }

    pub fn mark_failed(&mut self, id: TabId) {
        if let Some(tab) = self.get_mut(id) {
            tab.failed = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three() -> (Tabs, [TabId; 3]) {
        let mut tabs = Tabs::new();
        let ids = std::array::from_fn(|_| {
            let id = tabs.reserve_id().unwrap();
            tabs.insert(id, "/projects/api".into(), TabName::default());
            id
        });
        (tabs, ids)
    }

    #[test]
    fn closing_active_selects_right_then_left() {
        let (mut tabs, [a, b, c]) = three();
        tabs.select(b);
        assert!(tabs.remove(b));
        assert_eq!(tabs.active(), Some(c));
        tabs.remove(c);
        assert_eq!(tabs.active(), Some(a));
        tabs.remove(a);
        assert_eq!(tabs.active(), None);
    }

    #[test]
    fn closing_inactive_preserves_selection() {
        let (mut tabs, [a, b, _]) = three();
        tabs.select(a);
        tabs.remove(b);
        assert_eq!(tabs.active(), Some(a));
    }

    #[test]
    fn removed_ids_are_never_reused() {
        let (mut tabs, [_, _, c]) = three();
        tabs.remove(c);
        assert!(tabs.reserve_id().unwrap() > c);
        tabs.next_id = u32::MAX;
        assert_eq!(tabs.reserve_id().unwrap().get(), u32::MAX);
        assert!(tabs.reserve_id().is_err());
    }

    #[test]
    fn selection_wraps_and_nine_selects_last() {
        let (mut tabs, [a, b, c]) = three();
        tabs.select(a);
        tabs.cycle(true);
        assert_eq!(tabs.active(), Some(c));
        tabs.cycle(false);
        assert_eq!(tabs.active(), Some(a));
        tabs.select_number(2);
        assert_eq!(tabs.active(), Some(b));
        tabs.select_number(9);
        assert_eq!(tabs.active(), Some(c));
    }

    #[test]
    fn missing_selection_is_noop() {
        let (mut tabs, [a, _, _]) = three();
        tabs.select(a);
        tabs.select_number(8);
        assert!(!tabs.select(TabId::new(999).unwrap()));
        assert_eq!(tabs.active(), Some(a));
        assert!(TabId::new(0).is_none());
        assert!(serde_json::from_str::<TabId>("0").is_err());
    }

    #[test]
    fn manual_title_survives_osc_and_clear_restores_latest_auto() {
        let (mut tabs, [a, _, _]) = three();
        tabs.set_auto_title(a, "api");
        tabs.rename(a, "Build server");
        tabs.set_auto_title(a, "ssh");
        assert_eq!(tabs.ordered()[0].display_name(), "Build server");
        tabs.rename(a, "  ");
        assert_eq!(tabs.ordered()[0].display_name(), "ssh");
        tabs.set_auto_title(a, "");
        assert_eq!(tabs.ordered()[0].display_name(), "api");
    }

    #[test]
    fn title_is_plain_bounded_text() {
        let (mut tabs, [a, _, _]) = three();
        tabs.rename(a, "  <script>\u{1b}\n  ");
        assert_eq!(tabs.ordered()[0].display_name(), "<script>");
        tabs.rename(a, &"é".repeat(300));
        assert_eq!(tabs.ordered()[0].display_name().chars().count(), 256);
        tabs.rename(a, "");
        tabs.set_cwd(a, "/".into());
        assert_eq!(tabs.ordered()[0].display_name(), "/");
        tabs.set_cwd(a, std::env::var_os("HOME").unwrap().into());
        assert_eq!(tabs.ordered()[0].display_name(), "~");
        tabs.mark_ended(a);
        assert!(tabs.ordered()[0].ended);
        assert_eq!(serde_json::to_string(&a).unwrap(), "1");
    }
}
