use ratatui::{
    crossterm::event::KeyCode,
    layout::{Position, Rect},
    widgets::{Block, Paragraph},
    Frame,
};


#[derive(Debug)]
pub struct InputWidget {
    input: String,
    character_index: usize,
}

impl InputWidget {
    pub const fn new() -> Self {
        Self { input: String::new(), character_index: 0 }
    }

    pub fn handle_key_input(&mut self, key_code: KeyCode) {
        match key_code {
            KeyCode::Char(new_char) => self.enter_char(new_char),
            KeyCode::Backspace => self.delete_char(),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            _ => (),
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: &Rect) {
        let input = Paragraph::new(self.input.as_str())
            .block(Block::bordered().title(" User Input "));
        frame.render_widget(input, *area);
        frame.set_cursor_position(Position::new(
                area.x + self.character_index as u16 + 1,
                area.y + 1,
            ));
    }

    pub fn get_message(&mut self) -> String {
        let r = self.input.clone();
        self.input.clear();
        self.reset_cursor();
        r
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }



}