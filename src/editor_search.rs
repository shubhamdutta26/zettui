//! Literal, smart-case search over the live editor buffer.
use edtui::{EditorState, Highlight, Index2};
use ratatui::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

impl Direction {
    pub fn reverse(self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }

    pub fn prefix(self) -> char {
        match self {
            Self::Forward => '/',
            Self::Backward => '?',
        }
    }
}

pub struct SearchInput {
    pub query: String,
    pub direction: Direction,
    pub origin: Index2,
    pub viewport: (usize, usize),
}

#[derive(Default)]
pub struct NoteSearch {
    pub input: Option<SearchInput>,
    pub last: Option<(String, Direction)>,
    pub count: usize,
    pub active: usize,
    pub painted: Vec<(Index2, Index2)>,
}

fn matches(text: &str, query: &str) -> Vec<(Index2, Index2)> {
    if query.is_empty() {
        return Vec::new();
    }
    let sensitive = query.chars().any(char::is_uppercase);
    let needle: Vec<char> = query.chars().collect();
    let mut result = Vec::new();
    for (row, line) in text.split('\n').enumerate() {
        let chars: Vec<char> = line.chars().collect();
        for (col, window) in chars.windows(needle.len()).enumerate() {
            if window
                .iter()
                .zip(&needle)
                .all(|(&a, &b)| a == b || (!sensitive && a.to_lowercase().eq(b.to_lowercase())))
            {
                result.push((
                    Index2::new(row, col),
                    Index2::new(row, col + needle.len() - 1),
                ));
            }
        }
    }
    result
}

fn choose(hits: &[(Index2, Index2)], origin: &Index2, direction: Direction) -> Option<usize> {
    let origin = (origin.row, origin.col);
    if hits.is_empty() {
        return None;
    }
    Some(match direction {
        Direction::Forward => hits
            .iter()
            .position(|(p, _)| (p.row, p.col) > origin)
            .unwrap_or(0),
        Direction::Backward => hits
            .iter()
            .rposition(|(p, _)| (p.row, p.col) < origin)
            .unwrap_or(hits.len() - 1),
    })
}

impl NoteSearch {
    pub fn reset_matches(&mut self) {
        self.input = None;
        self.count = 0;
        self.active = 0;
        self.painted.clear();
    }

    pub fn begin(&mut self, state: &EditorState, direction: Direction) {
        self.input = Some(SearchInput {
            query: String::new(),
            direction,
            origin: state.cursor,
            viewport: state.viewport_offset(),
        });
        self.count = 0;
        self.active = 0;
        self.painted.clear();
    }

    pub fn jump(
        &mut self,
        state: &mut EditorState,
        query: &str,
        origin: Index2,
        direction: Direction,
        style: Style,
        current_style: Style,
    ) {
        let hits = matches(&state.lines.to_string(), query);
        self.count = hits.len();
        self.painted = hits.clone();
        self.active = 0;
        let chosen = choose(&hits, &origin, direction);
        state.cursor = origin;
        if let Some(i) = chosen {
            state.cursor = hits[i].0;
            self.active = i + 1;
        }
        state.set_highlights(
            hits.into_iter()
                .enumerate()
                .map(|(i, (start, end))| {
                    Highlight::new(
                        start,
                        end,
                        if Some(i) == chosen {
                            current_style
                        } else {
                            style
                        },
                    )
                })
                .collect(),
        );
    }

    pub fn preview(&mut self, state: &mut EditorState, style: Style, current_style: Style) {
        if let Some(input) = &self.input {
            let query = input.query.clone();
            let origin = input.origin;
            let direction = input.direction;
            let (x, y) = input.viewport;
            state.set_viewport_offset(x, y);
            self.jump(state, &query, origin, direction, style, current_style);
        }
    }

    pub fn cancel(&mut self, state: &mut EditorState) {
        if let Some(input) = self.input.take() {
            state.cursor = input.origin;
            state.set_viewport_offset(input.viewport.0, input.viewport.1);
        }
        state.clear_highlights();
        self.count = 0;
        self.active = 0;
        self.painted.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_phrases_and_smart_case() {
        let text = "Antibody binding\nantibody binding\nother";
        assert_eq!(matches(text, "antibody binding").len(), 2);
        assert_eq!(matches(text, "Antibody binding").len(), 1);
        assert!(matches(text, "").is_empty());
    }

    #[test]
    fn wraps_in_both_directions() {
        let hits = matches("x x\nx", "x");
        assert_eq!(
            choose(&hits, &Index2::new(1, 0), Direction::Forward),
            Some(0)
        );
        assert_eq!(
            choose(&hits, &Index2::new(0, 0), Direction::Backward),
            Some(2)
        );
    }

    #[test]
    fn unicode_columns_are_character_offsets() {
        let hits = matches("β café", "CAF");
        assert!(hits.is_empty());
        let hits = matches("β café", "café");
        assert_eq!(hits[0].0.col, 2);
        assert_eq!(hits[0].1.col, 5);
    }
}
