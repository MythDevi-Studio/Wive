//! Unicode-aware text storage primitives.

use std::ops::{Deref, DerefMut};

use ropey::Rope;

/// A rope-backed document buffer with logarithmic editing and line access.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextBuffer(Rope);

impl TextBuffer {
    /// Creates an empty text buffer.
    pub fn new() -> Self {
        Self(Rope::new())
    }

    /// Creates a buffer initialized with `text`.
    pub fn from_text(text: &str) -> Self {
        Self(Rope::from_str(text))
    }

    /// Returns the number of characters in the buffer.
    pub fn char_len(&self) -> usize {
        self.0.len_chars()
    }
}

impl Deref for TextBuffer {
    type Target = Rope;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TextBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&str> for TextBuffer {
    fn from(value: &str) -> Self {
        Self::from_text(value)
    }
}

#[cfg(test)]
mod tests {
    use super::TextBuffer;

    #[test]
    fn preserves_unicode_and_line_access() {
        let buffer = TextBuffer::from_text("café\n二行目");
        assert_eq!(buffer.char_len(), 8);
        assert_eq!(buffer.line(0).to_string(), "café\n");
        assert_eq!(buffer.line(1).to_string(), "二行目");
    }
}
