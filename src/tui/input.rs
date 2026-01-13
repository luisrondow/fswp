use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Represents the result of handling a key event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// Quit the application
    Quit,
    /// Mark current file to keep
    Keep,
    /// Mark current file to trash
    Trash,
    /// Confirm trash action
    ConfirmTrash,
    /// Cancel trash action
    CancelTrash,
    /// Move to next file
    Next,
    /// Move to previous file
    Previous,
    /// Undo last decision
    Undo,
    /// Toggle help overlay
    Help,
    /// No action
    None,
}

/// Maps keyboard events to actions
pub fn handle_key_event(key: KeyEvent) -> KeyAction {
    match (key.code, key.modifiers) {
        // Quit: q or Ctrl+C
        (KeyCode::Char('q'), KeyModifiers::NONE) => KeyAction::Quit,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => KeyAction::Quit,
        (KeyCode::Esc, KeyModifiers::NONE) => KeyAction::Quit,

        // Keep: Right arrow or k
        (KeyCode::Right, KeyModifiers::NONE) => KeyAction::Keep,
        (KeyCode::Char('k'), KeyModifiers::NONE) => KeyAction::Keep,

        // Trash: Left arrow or t
        (KeyCode::Left, KeyModifiers::NONE) => KeyAction::Trash,
        (KeyCode::Char('t'), KeyModifiers::NONE) => KeyAction::Trash,

        // Navigation
        (KeyCode::Down, KeyModifiers::NONE) => KeyAction::Next,
        (KeyCode::Up, KeyModifiers::NONE) => KeyAction::Previous,
        (KeyCode::Char('j'), KeyModifiers::NONE) => KeyAction::Next,
        (KeyCode::Char('i'), KeyModifiers::NONE) => KeyAction::Previous,

        // Undo: u or Ctrl+Z
        (KeyCode::Char('u'), KeyModifiers::NONE) => KeyAction::Undo,
        (KeyCode::Char('z'), KeyModifiers::CONTROL) => KeyAction::Undo,

        // Help: ?
        (KeyCode::Char('?'), KeyModifiers::NONE) => KeyAction::Help,

        _ => KeyAction::None,
    }
}

/// Maps keyboard events to confirmation actions
/// Used when ViewState is ConfirmTrash
pub fn handle_confirm_input(key: KeyEvent) -> KeyAction {
    match (key.code, key.modifiers) {
        // Confirm: y or Enter
        (KeyCode::Char('y'), KeyModifiers::NONE) => KeyAction::ConfirmTrash,
        (KeyCode::Char('Y'), KeyModifiers::NONE) => KeyAction::ConfirmTrash,
        (KeyCode::Enter, KeyModifiers::NONE) => KeyAction::ConfirmTrash,

        // Cancel: n or Esc
        (KeyCode::Char('n'), KeyModifiers::NONE) => KeyAction::CancelTrash,
        (KeyCode::Char('N'), KeyModifiers::NONE) => KeyAction::CancelTrash,
        (KeyCode::Esc, KeyModifiers::NONE) => KeyAction::CancelTrash,

        _ => KeyAction::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_quit() {
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Quit);

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(handle_key_event(key), KeyAction::Quit);

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Quit);
    }

    #[test]
    fn test_key_keep() {
        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Keep);

        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Keep);
    }

    #[test]
    fn test_key_trash() {
        let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Trash);

        let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Trash);
    }

    #[test]
    fn test_key_navigation() {
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Next);

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Previous);

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Next);

        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Previous);
    }

    #[test]
    fn test_key_undo() {
        let key = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Undo);

        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(handle_key_event(key), KeyAction::Undo);
    }

    #[test]
    fn test_key_help() {
        let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::Help);
    }

    #[test]
    fn test_key_none() {
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(handle_key_event(key), KeyAction::None);
    }

    #[test]
    fn test_confirm_trash_keys() {
        // Test y key
        let key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::ConfirmTrash);

        // Test Y key (uppercase)
        let key = KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::ConfirmTrash);

        // Test Enter key
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::ConfirmTrash);
    }

    #[test]
    fn test_cancel_trash_keys() {
        // Test n key
        let key = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::CancelTrash);

        // Test N key (uppercase)
        let key = KeyEvent::new(KeyCode::Char('N'), KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::CancelTrash);

        // Test Esc key
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::CancelTrash);
    }

    #[test]
    fn test_confirm_input_none() {
        // Test other keys return None
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::None);

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(handle_confirm_input(key), KeyAction::None);
    }
}
