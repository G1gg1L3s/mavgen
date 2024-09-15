use quote::format_ident;

use crate::model;

pub fn snake_case(s: &str) -> String {
    let mut state = State::Start;

    enum State {
        Start,
        PreviousCharWasUpper,
        PreviousCharWasLower,
        PreviousCharWasUnderscore,
    }

    let mut transformed = String::new();

    for c in s.chars() {
        if c.is_uppercase() {
            if let State::PreviousCharWasLower = state {
                // Insert an underscore before the first uppercase character in a sequence
                transformed.push('_');
            }
            for lower_c in c.to_lowercase() {
                transformed.push(lower_c);
            }
            state = State::PreviousCharWasUpper;
        } else {
            transformed.push(c);
            state = if c == '_' {
                // this it to avoid adding double underscore
                State::PreviousCharWasUnderscore
            } else {
                State::PreviousCharWasLower
            };
        }
    }

    transformed
}

pub struct PascalCase<'a>(&'a model::Ident);

impl<'a> std::fmt::Display for PascalCase<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        let snake_case = snake_case(self.0.as_ref()).to_string();

        let mut capitalize_next = true;

        for c in snake_case.chars() {
            if c == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                for upper_c in c.to_uppercase() {
                    f.write_char(upper_c)?;
                }
                capitalize_next = false;
            } else {
                for lower_c in c.to_lowercase() {
                    f.write_char(lower_c)?;
                }
            }
        }
        Ok(())
    }
}

pub struct SnakeCase<'a>(&'a model::Ident);

impl<'a> std::fmt::Display for SnakeCase<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let transformed = snake_case(self.0.as_ref());

        const RESERVED_KEYWORDS: [&str; 38] = [
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
            "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
            "unsafe", "use", "where", "while", "async", "await", "dyn",
        ];

        if RESERVED_KEYWORDS.contains(&transformed.as_str()) {
            write!(f, "r#{}", transformed)
        } else {
            write!(f, "{}", transformed)
        }
    }
}

pub trait IdentExt {
    fn pascal_case(&self) -> proc_macro2::Ident;
    fn snake_case(&self) -> proc_macro2::Ident;
}

impl IdentExt for model::Ident {
    fn pascal_case(&self) -> proc_macro2::Ident {
        format_ident!("{}", PascalCase(self).to_string())
    }

    fn snake_case(&self) -> proc_macro2::Ident {
        format_ident!("{}", SnakeCase(self).to_string())
    }
}
