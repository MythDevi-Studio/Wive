//! Lightweight, dependency-free syntax tokenization for the editor.

/// Broad language family used for syntax coloring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    /// Rust source.
    Rust,
    /// JavaScript and TypeScript source.
    JavaScript,
    /// Python source.
    Python,
    /// JSON, TOML, and YAML data.
    Data,
    /// Unclassified text.
    #[default]
    Plain,
}

/// Syntax category for a source token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// Ordinary source text or punctuation.
    Plain,
    /// A language keyword.
    Keyword,
    /// A string or character literal.
    String,
    /// A numeric literal.
    Number,
    /// A boolean or null-like literal.
    Literal,
    /// A type or identifier beginning with an uppercase letter.
    Type,
    /// An operator.
    Operator,
    /// A line comment.
    Comment,
}

/// A borrowed token from a source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token<'a> {
    /// Source text for this token.
    pub text: &'a str,
    /// Color category for the token.
    pub kind: TokenKind,
}

/// Infers a lightweight syntax family from a file name.
pub fn language_for_file(name: &str) -> Language {
    match std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "rs" => Language::Rust,
        "js" | "jsx" | "ts" | "tsx" => Language::JavaScript,
        "py" => Language::Python,
        "json" | "toml" | "yaml" | "yml" => Language::Data,
        _ => Language::Plain,
    }
}

/// Splits a line into tokens without changing or losing its whitespace.
pub fn tokenize_line(line: &str, language: Language) -> Vec<Token<'_>> {
    let line = line.trim_end_matches(['\n', '\r']);
    let mut tokens = Vec::new();
    let mut iter = line.char_indices().peekable();

    while let Some((start, ch)) = iter.next() {
        let next = iter.peek().map(|(_, next)| *next);
        if ch.is_whitespace() {
            tokens.push(Token {
                text: &line[start..start + ch.len_utf8()],
                kind: TokenKind::Plain,
            });
        } else if ch == '/' && next == Some('/') || ch == '#' && language == Language::Python {
            tokens.push(Token {
                text: &line[start..],
                kind: TokenKind::Comment,
            });
            break;
        } else if ch == '"' || ch == '\'' {
            let mut end = start + ch.len_utf8();
            let mut escaped = false;
            for (position, current) in iter.by_ref() {
                end = position + current.len_utf8();
                if current == ch && !escaped {
                    break;
                }
                escaped = current == '\\' && !escaped;
                if current != '\\' {
                    escaped = false;
                }
            }
            tokens.push(Token {
                text: &line[start..end],
                kind: TokenKind::String,
            });
        } else if ch.is_ascii_digit() {
            let mut end = start + ch.len_utf8();
            while let Some((position, current)) = iter.peek().copied() {
                if current.is_ascii_alphanumeric() || matches!(current, '_' | '.') {
                    iter.next();
                    end = position + current.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(Token {
                text: &line[start..end],
                kind: TokenKind::Number,
            });
        } else if ch.is_alphanumeric() || ch == '_' {
            let mut end = start + ch.len_utf8();
            while let Some((position, current)) = iter.peek().copied() {
                if current.is_alphanumeric() || current == '_' {
                    iter.next();
                    end = position + current.len_utf8();
                } else {
                    break;
                }
            }
            let text = &line[start..end];
            tokens.push(Token {
                text,
                kind: classify_word(text, ch),
            });
        } else {
            tokens.push(Token {
                text: &line[start..start + ch.len_utf8()],
                kind: if matches!(
                    ch,
                    '=' | '+' | '-' | '*' | '/' | '!' | '<' | '>' | '&' | '|'
                ) {
                    TokenKind::Operator
                } else {
                    TokenKind::Plain
                },
            });
        }
    }
    tokens
}

fn classify_word(word: &str, first: char) -> TokenKind {
    const KEYWORDS: &[&str] = &[
        "as",
        "async",
        "await",
        "break",
        "catch",
        "class",
        "const",
        "continue",
        "def",
        "else",
        "enum",
        "export",
        "extern",
        "false",
        "fn",
        "for",
        "from",
        "function",
        "if",
        "impl",
        "import",
        "interface",
        "let",
        "loop",
        "match",
        "mod",
        "move",
        "mut",
        "new",
        "None",
        "null",
        "of",
        "priv",
        "private",
        "pub",
        "public",
        "return",
        "self",
        "Self",
        "static",
        "struct",
        "super",
        "this",
        "throw",
        "trait",
        "try",
        "type",
        "unsafe",
        "use",
        "var",
        "while",
        "where",
    ];
    if matches!(word, "true" | "false" | "null" | "None" | "Some") {
        TokenKind::Literal
    } else if KEYWORDS.contains(&word) {
        TokenKind::Keyword
    } else if first.is_uppercase() {
        TokenKind::Type
    } else {
        TokenKind::Plain
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, TokenKind, language_for_file, tokenize_line};

    #[test]
    fn tokenizes_code_and_preserves_spaces() {
        let tokens = tokenize_line("let value = \"hi\"; // note", Language::Rust);
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Keyword));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::String));
        assert!(tokens.iter().any(|token| token.kind == TokenKind::Comment));
        assert_eq!(
            tokens.iter().map(|token| token.text).collect::<String>(),
            "let value = \"hi\"; // note"
        );
    }

    #[test]
    fn detects_common_file_families() {
        assert_eq!(language_for_file("main.rs"), Language::Rust);
        assert_eq!(language_for_file("unknown.txt"), Language::Plain);
    }
}
