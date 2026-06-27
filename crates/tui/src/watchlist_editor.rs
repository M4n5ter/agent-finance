use tui_input::{Input, InputRequest};

use crate::config::normalize_symbols;

#[derive(Debug, Clone, Default)]
pub struct WatchlistAddState {
    input: Input,
}

impl WatchlistAddState {
    pub fn query(&self) -> &str {
        self.input.value()
    }

    pub fn edit_query(&mut self, request: InputRequest) {
        self.input.handle(request);
    }

    pub fn reset(&mut self) {
        self.input = Input::default();
    }

    pub fn symbols(&self) -> Vec<String> {
        normalize_symbols(&[self.input.value().to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchlist_add_state_normalizes_comma_separated_symbols() {
        let mut state = WatchlistAddState::default();
        for character in " lite,aaoi,LITE ".chars() {
            state.edit_query(InputRequest::InsertChar(character));
        }

        assert_eq!(state.symbols(), ["LITE", "AAOI"]);
    }
}
