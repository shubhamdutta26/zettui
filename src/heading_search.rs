//! Searchable heading snapshots from the current (possibly unsaved) note.

use crate::{fuzzy, outline};

#[derive(Debug)]
pub(crate) struct HeadingHit {
    pub level: usize,
    pub row: usize,
    pub title: String,
}

pub(crate) struct HeadingSearch {
    pub items: Vec<HeadingHit>,
    pub query: String,
    pub matches: Vec<usize>,
    pub selected: usize,
}

impl HeadingSearch {
    pub fn new(source: &str) -> Self {
        let items: Vec<_> = outline::extract(source)
            .into_iter()
            .map(|heading| HeadingHit {
                level: heading.level,
                row: heading.row,
                title: heading.title.to_owned(),
            })
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
                .filter_map(|(index, hit)| {
                    fuzzy::score(&hit.title, &self.query).map(|score| (score, index))
                })
                .collect();
            // The original source index breaks ties, including duplicate titles.
            ranked.sort_unstable();
            self.matches = ranked.into_iter().map(|(_, index)| index).collect();
        }
        self.selected = 0;
    }

    pub fn selected_row(&self) -> Option<usize> {
        self.matches
            .get(self.selected)
            .map(|&index| self.items[index].row)
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
    fn results_are_ranked_and_duplicate_titles_keep_source_order() {
        let mut search = HeadingSearch::new(
            "# Size exclusion chromatography\n## SEC\n## SEC analysis\n## SEC\n# Other",
        );
        assert_eq!(search.matches, vec![0, 1, 2, 3, 4]);
        search.query = "sec".into();
        search.filter();
        assert_eq!(search.matches, vec![1, 3, 2, 0]);
        assert_eq!(search.selected_row(), Some(1));
        search.move_selection(true);
        assert_eq!(search.selected_row(), Some(3));
        search.query.clear();
        search.filter();
        assert_eq!(search.matches, vec![0, 1, 2, 3, 4]);
        assert_eq!(search.selected, 0);
    }

    #[test]
    fn ignores_non_heading_blocks_and_handles_no_results() {
        let mut search = HeadingSearch::new(
            "---\n# YAML\n---\n# Actual\n```r\n# Fake\n```\n<!--\n# Fake\n-->\n## Actual",
        );
        assert_eq!(search.items.len(), 2);
        search.query = "fake".into();
        search.filter();
        search.move_selection(true);
        search.move_selection(false);
        assert_eq!(search.selected_row(), None);
        assert_eq!(search.selected, 0);
    }
}
