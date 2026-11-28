//! Context-filtered command picker using the same fuzzy ranker as headings.

use crate::{
    commands::{CATALOG, Command, CommandInfo},
    fuzzy,
};

pub(crate) struct CommandPalette {
    pub items: Vec<CommandInfo>,
    pub query: String,
    pub matches: Vec<usize>,
    pub selected: usize,
}

impl CommandPalette {
    pub fn new(available: impl Fn(Command) -> bool) -> Self {
        let items: Vec<_> = CATALOG
            .iter()
            .copied()
            .filter(|item| available(item.command))
            .collect();
        let matches = (0..items.len()).collect();
        Self {
            items,
            query: String::new(),
            matches,
            selected: 0,
        }
    }

    pub fn filter(&mut self) {
        if self.query.trim().is_empty() {
            self.matches = (0..self.items.len()).collect();
        } else {
            let mut ranked: Vec<_> = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    // Prefer the best score among the visible title, hint and aliases.
                    [item.title, item.shortcut, item.aliases]
                        .into_iter()
                        .filter_map(|text| fuzzy::score(text, &self.query))
                        .min()
                        .map(|score| (score, index))
                })
                .collect();
            ranked.sort_unstable();
            self.matches = ranked.into_iter().map(|(_, index)| index).collect();
        }
        self.selected = 0;
    }

    pub fn selected_command(&self) -> Option<Command> {
        self.matches
            .get(self.selected)
            .map(|&index| self.items[index].command)
    }

    pub fn move_selection(&mut self, down: bool) {
        self.selected = if down {
            self.selected
                .saturating_add(1)
                .min(self.matches.len().saturating_sub(1))
        } else {
            self.selected.saturating_sub(1)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_titles_aliases_and_shortcuts() {
        let mut palette = CommandPalette::new(|_| true);
        for (query, expected) in [
            ("SAVE NOTE", Command::Save),
            ("f7", Command::SearchHeadings),
            ("zotero", Command::CitationPicker),
            ("insert markdown link", Command::InsertLink),
            ("insert markdown image", Command::InsertImage),
            ("delete", Command::Trash),
        ] {
            palette.query = query.into();
            palette.filter();
            assert_eq!(palette.selected_command(), Some(expected), "{query}");
        }
        palette.query = "srchhdngs".into();
        palette.filter();
        assert!(
            palette
                .matches
                .iter()
                .any(|&i| palette.items[i].command == Command::SearchHeadings)
        );
    }

    #[test]
    fn context_filter_and_empty_results_are_safe() {
        let mut palette = CommandPalette::new(|command| command == Command::Save);
        assert_eq!(palette.selected_command(), Some(Command::Save));
        palette.query = "trash".into();
        palette.filter();
        palette.move_selection(true);
        palette.move_selection(false);
        assert_eq!(palette.selected_command(), None);
        palette.query.clear();
        palette.filter();
        assert_eq!(palette.selected_command(), Some(Command::Save));
    }

    #[test]
    fn catalog_has_unique_actions_and_no_unconfirmed_discard() {
        for (i, item) in CATALOG.iter().enumerate() {
            assert!(
                !CATALOG[..i]
                    .iter()
                    .any(|previous| previous.command == item.command)
            );
            assert!(!matches!(
                item.command,
                Command::DiscardAndClose | Command::OpenPalette
            ));
        }
    }
}
