use tui_input::{Input, InputRequest};

use crate::config::normalize_profile_name;

#[derive(Debug, Clone, Default)]
pub struct ProfileEditorState {
    input: Input,
}

impl ProfileEditorState {
    pub fn query(&self) -> &str {
        self.input.value()
    }

    pub fn edit_query(&mut self, request: InputRequest) {
        self.input.handle(request);
    }

    pub fn reset(&mut self, profile: Option<&str>) {
        self.input = profile.unwrap_or_default().into();
    }

    pub fn profile(&self) -> Option<String> {
        normalize_profile_name(Some(self.input.value().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_editor_normalizes_blank_profile_to_none() {
        let mut state = ProfileEditorState::default();
        for character in "   ".chars() {
            state.edit_query(InputRequest::InsertChar(character));
        }

        assert_eq!(state.profile(), None);
    }

    #[test]
    fn profile_editor_preserves_profile_name_case() {
        let mut state = ProfileEditorState::default();
        state.reset(Some(" mainnet "));

        assert_eq!(state.profile().as_deref(), Some("mainnet"));
    }
}
