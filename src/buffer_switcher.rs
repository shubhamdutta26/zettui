//! Fuzzy selection of live buffers, in stable opening order.
use crate::{buffers::Buffers, fuzzy};

pub(crate) struct BufferSwitcher {
    pub query: String,
    pub matches: Vec<usize>,
    pub selected: usize,
}

impl BufferSwitcher {
    pub fn new(buffers: &Buffers) -> Self {
        Self {
            query: String::new(),
            matches: (0..buffers.len()).collect(),
            selected: buffers.active_index().unwrap_or(0),
        }
    }

    pub fn filter(&mut self, buffers: &Buffers) {
        let mut ranked: Vec<_> = buffers
            .notes()
            .iter()
            .enumerate()
            .filter_map(|(i, note)| {
                let name = note.path.file_name()?.to_string_lossy();
                let path = note.path.to_string_lossy();
                [name.as_ref(), path.as_ref()]
                    .into_iter()
                    .filter_map(|label| fuzzy::score(label, &self.query))
                    .min()
                    .map(|score| (score, i))
            })
            .collect();
        ranked.sort_unstable();
        self.matches = ranked.into_iter().map(|(_, i)| i).collect();
        self.selected = 0;
    }
}
