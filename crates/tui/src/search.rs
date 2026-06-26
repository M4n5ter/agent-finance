use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use tui_input::{Input, InputRequest};

#[derive(Debug, Clone, Default)]
pub struct SearchListState {
    input: Input,
    selected: usize,
    matches: Vec<usize>,
}

impl SearchListState {
    pub fn with_matches(matches: Vec<usize>) -> Self {
        Self {
            input: Input::default(),
            selected: 0,
            matches,
        }
    }

    pub fn query(&self) -> &str {
        self.input.value()
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn index_at(&self, index: usize) -> Option<usize> {
        self.matches.get(index).copied()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.index_at(self.selected)
    }

    pub fn reset(&mut self, matches: Vec<usize>) {
        self.input = Input::default();
        self.matches = matches;
        self.selected = 0;
    }

    pub fn shift(&mut self, direction: isize) {
        if self.matches.is_empty() {
            self.selected = 0;
            return;
        }
        let len = self.matches.len() as isize;
        let selected = self.selected as isize;
        self.selected = (selected + direction).rem_euclid(len) as usize;
    }

    pub fn edit_query(&mut self, request: InputRequest, refresh: impl FnOnce(&str) -> Vec<usize>) {
        let previous = self.input.value().to_string();
        self.input.handle(request);
        if self.input.value() != previous {
            self.matches = refresh(self.input.value().trim());
            self.selected = 0;
        }
    }
}

pub fn fuzzy_indices(
    query: &str,
    indices: impl IntoIterator<Item = usize>,
    mut text_for: impl FnMut(usize) -> Option<String>,
) -> Vec<usize> {
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut utf32_buffer = Vec::new();
    let mut scored = indices
        .into_iter()
        .filter_map(|index| {
            let text = text_for(index)?;
            pattern
                .score(Utf32Str::new(&text, &mut utf32_buffer), &mut matcher)
                .map(|score| (index, score))
        })
        .collect::<Vec<_>>();
    scored.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    scored.into_iter().map(|(index, _)| index).collect()
}

#[derive(Debug, Clone, Default)]
pub struct SymbolSearchState {
    list: SearchListState,
}

impl SymbolSearchState {
    pub fn query(&self) -> &str {
        self.list.query()
    }

    pub fn selected(&self) -> usize {
        self.list.selected()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn symbol_index_at(&self, index: usize) -> Option<usize> {
        self.list.index_at(index)
    }

    pub fn selected_symbol_index(&self) -> Option<usize> {
        self.list.selected_index()
    }

    pub fn reset(&mut self, symbols: &[String]) {
        self.list.reset(all_symbol_indices(symbols));
    }

    pub fn shift(&mut self, direction: isize) {
        self.list.shift(direction);
    }

    pub fn edit_query(&mut self, symbols: &[String], request: InputRequest) {
        self.list
            .edit_query(request, |query| symbol_indices_for_query(query, symbols));
    }
}

fn symbol_indices_for_query(query: &str, symbols: &[String]) -> Vec<usize> {
    if query.is_empty() {
        all_symbol_indices(symbols)
    } else {
        fuzzy_symbol_indices(query, symbols)
    }
}

fn all_symbol_indices(symbols: &[String]) -> Vec<usize> {
    (0..symbols.len()).collect()
}

fn fuzzy_symbol_indices(query: &str, symbols: &[String]) -> Vec<usize> {
    fuzzy_indices(query, 0..symbols.len(), |index| symbols.get(index).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_search_filters_and_selects_watchlist_indices() {
        let symbols = ["AAPL", "CRDO", "BTCUSDT", "LITE"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut search = SymbolSearchState::default();
        search.reset(&symbols);

        assert_eq!(search.len(), 4);
        for character in "bt".chars() {
            search.edit_query(&symbols, InputRequest::InsertChar(character));
        }

        assert_eq!(search.query(), "bt");
        assert_eq!(search.selected_symbol_index(), Some(2));
    }

    #[test]
    fn symbol_search_selection_wraps_visible_matches() {
        let symbols = ["AAPL", "CRDO"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut search = SymbolSearchState::default();
        search.reset(&symbols);

        search.shift(-1);

        assert_eq!(search.selected_symbol_index(), Some(1));
    }
}
