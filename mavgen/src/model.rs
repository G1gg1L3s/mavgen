use std::path::PathBuf;

use crate::xml;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(String);

impl std::fmt::Display for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevStatus {
    Deprecated {
        since: String,
        replaced_by: String,
        description: Option<String>,
    },
    Wip {
        since: Option<String>,
        description: Option<String>,
    },
}

impl From<xml::DevStatus> for DevStatus {
    fn from(value: xml::DevStatus) -> Self {
        match value {
            xml::DevStatus::Deprecated(depr) => DevStatus::Deprecated {
                since: depr.since,
                replaced_by: depr.replaced_by,
                description: non_empty(depr.description),
            },
            xml::DevStatus::Wip(wip) => DevStatus::Wip {
                since: wip.since,
                description: non_empty(wip.description),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: Ident,
    pub description: Option<String>,
    pub dev_status: Option<DevStatus>,
    pub value: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub name: Ident,
    pub bitmask: bool,
    pub description: Option<String>,
    pub dev_status: Option<DevStatus>,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
    Float,
    Double,
    Char,
    Int8,
    Uint8,
    Uint8MavlinkVersion,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Primitive(PrimitiveType),
    Array(PrimitiveType, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustSizeType {
    U8,
    U16,
    U32,
    U64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: Ident,
    pub r#type: FieldType,

    pub print_format: Option<String>,
    pub r#enum: Option<Ident>,
    pub display: Option<String>,
    pub units: Option<String>,
    pub increment: Option<f32>,
    pub min_value: Option<f32>,
    pub max_value: Option<f32>,
    pub multiplier: Option<String>,
    pub default: Option<String>,
    pub instance: Option<bool>,
    pub invalid: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub name: Ident,
    pub id: u32,
    pub dev_status: Option<DevStatus>,
    pub description: Option<String>,
    pub fields: Vec<Field>,
    pub extension_fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MavlinkModule {
    pub path: PathBuf,
    pub version: Option<u8>,
    pub dialect: Option<u8>,
    pub enums: Vec<Enum>,
    pub messages: Vec<Message>,
}

impl Enum {
    pub fn min_rust_size(&self) -> RustSizeType {
        let max_value = self
            .entries
            .iter()
            .map(|entry| entry.value)
            .max()
            .expect("enums are not empty");

        match max_value {
            val if val <= u64::from(u8::MAX) => RustSizeType::U8,
            val if val <= u64::from(u16::MAX) => RustSizeType::U16,
            val if val <= u64::from(u32::MAX) => RustSizeType::U32,
            _ => RustSizeType::U64,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InvalidIdentError;

impl std::fmt::Display for InvalidIdentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid identifier")
    }
}

impl std::str::FromStr for Ident {
    type Err = InvalidIdentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const FORBIDDEN_NAMES: &[&str] = &[
            "break",
            "case",
            "class",
            "catch",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "export",
            "extends",
            "finally",
            "for",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "let",
            "new",
            "return",
            "super",
            "switch",
            "this",
            "throw",
            "try",
            "typeof",
            "var",
            "void",
            "while",
            "with",
            "yield",
            "enum",
            "await",
            "implements",
            "package",
            "protected",
            "static",
            "interface",
            "private",
            "public",
            "abstract",
            "boolean",
            "byte",
            "char",
            "double",
            "final",
            "float",
            "goto",
            "int",
            "long",
            "native",
            "short",
            "synchronized",
            "transient",
            "volatile",
        ];

        // TODO: ideally, it should parse identifiers in the same way python or
        // rust parses them:
        // identifier   ::=  xid_start xid_continue*
        //
        // Currently they will accept symbols like :: or ^ just fine.
        // Rust compiler uses https://github.com/unicode-rs/unicode-xid/

        if s.is_empty() || s == "_" {
            return Err(InvalidIdentError);
        }

        if matches!(s.chars().next(), Some('0'..='9')) {
            return Err(InvalidIdentError);
        }

        if s.contains(char::is_whitespace) {
            return Err(InvalidIdentError);
        }

        if FORBIDDEN_NAMES.contains(&s.to_lowercase().as_str()) {
            return Err(InvalidIdentError);
        }

        Ok(Ident(s.to_owned()))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InvalidTypeError;

impl std::fmt::Display for InvalidTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid field type")
    }
}

impl std::str::FromStr for PrimitiveType {
    type Err = InvalidTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "float" => Ok(Self::Float),
            "double" => Ok(Self::Double),
            "char" => Ok(Self::Char),
            "int8_t" => Ok(Self::Int8),
            "uint8_t" => Ok(Self::Uint8),
            "uint8_t_mavlink_version" => Ok(Self::Uint8MavlinkVersion),
            "int16_t" => Ok(Self::Int16),
            "uint16_t" => Ok(Self::Uint16),
            "int32_t" => Ok(Self::Int32),
            "uint32_t" => Ok(Self::Uint32),
            "int64_t" => Ok(Self::Int64),
            "uint64_t" => Ok(Self::Uint64),
            _ => Err(InvalidTypeError),
        }
    }
}

impl std::str::FromStr for FieldType {
    type Err = InvalidTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(without_closing_bracket) = s.strip_suffix(']') {
            let (type_part, size_part) = without_closing_bracket
                .split_once('[')
                .ok_or(InvalidTypeError)?;

            let typ = type_part.parse()?;
            let size = size_part.parse().map_err(|_| InvalidTypeError)?;

            Ok(Self::Array(typ, size))
        } else {
            Ok(Self::Primitive(s.parse()?))
        }
    }
}

fn non_empty(str: String) -> Option<String> {
    if str.is_empty() {
        None
    } else {
        Some(str)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_ident_parse() {
        Ident::from_str("break").unwrap_err();
        Ident::from_str("case").unwrap_err();
        Ident::from_str("class").unwrap_err();
        Ident::from_str("catch").unwrap_err();
        Ident::from_str("const").unwrap_err();
        Ident::from_str("continue").unwrap_err();
        Ident::from_str("debugger").unwrap_err();
        Ident::from_str("default").unwrap_err();
        Ident::from_str("delete").unwrap_err();
        Ident::from_str("do").unwrap_err();
        Ident::from_str("else").unwrap_err();
        Ident::from_str("export").unwrap_err();
        Ident::from_str("extends").unwrap_err();
        Ident::from_str("finally").unwrap_err();
        Ident::from_str("for").unwrap_err();
        Ident::from_str("function").unwrap_err();
        Ident::from_str("if").unwrap_err();
        Ident::from_str("import").unwrap_err();
        Ident::from_str("in").unwrap_err();
        Ident::from_str("instanceof").unwrap_err();
        Ident::from_str("let").unwrap_err();
        Ident::from_str("new").unwrap_err();
        Ident::from_str("return").unwrap_err();
        Ident::from_str("super").unwrap_err();
        Ident::from_str("switch").unwrap_err();
        Ident::from_str("this").unwrap_err();
        Ident::from_str("throw").unwrap_err();
        Ident::from_str("try").unwrap_err();
        Ident::from_str("typeof").unwrap_err();
        Ident::from_str("var").unwrap_err();
        Ident::from_str("void").unwrap_err();
        Ident::from_str("while").unwrap_err();
        Ident::from_str("with").unwrap_err();
        Ident::from_str("yield").unwrap_err();
        Ident::from_str("enum").unwrap_err();
        Ident::from_str("await").unwrap_err();
        Ident::from_str("implements").unwrap_err();
        Ident::from_str("package").unwrap_err();
        Ident::from_str("protected").unwrap_err();
        Ident::from_str("static").unwrap_err();
        Ident::from_str("interface").unwrap_err();
        Ident::from_str("private").unwrap_err();
        Ident::from_str("public").unwrap_err();
        Ident::from_str("abstract").unwrap_err();
        Ident::from_str("boolean").unwrap_err();
        Ident::from_str("byte").unwrap_err();
        Ident::from_str("char").unwrap_err();
        Ident::from_str("double").unwrap_err();
        Ident::from_str("final").unwrap_err();
        Ident::from_str("float").unwrap_err();
        Ident::from_str("goto").unwrap_err();
        Ident::from_str("int").unwrap_err();
        Ident::from_str("long").unwrap_err();
        Ident::from_str("native").unwrap_err();
        Ident::from_str("short").unwrap_err();
        Ident::from_str("synchronized").unwrap_err();
        Ident::from_str("transient").unwrap_err();
        Ident::from_str("volatile").unwrap_err();
        Ident::from_str("some space").unwrap_err();
        Ident::from_str("some\ttab").unwrap_err();
        Ident::from_str("    I need more space   ").unwrap_err();
        Ident::from_str("123turbofish").unwrap_err();
        Ident::from_str("9turbofish").unwrap_err();
        Ident::from_str("0turbofish").unwrap_err();
        Ident::from_str("").unwrap_err();
        Ident::from_str("_").unwrap_err();
        Ident::from_str(" ::<> ").unwrap_err();
        assert_eq!(Ident::from_str("HELLO").unwrap(), Ident("HELLO".to_owned()));
        assert_eq!(
            Ident::from_str("THIS_SHOULD_BE_VALID").unwrap(),
            Ident("THIS_SHOULD_BE_VALID".to_owned())
        );
        assert_eq!(Ident::from_str("A").unwrap(), Ident("A".to_owned()));
    }

    #[test]
    fn test_field_type_parse() {
        let valid_cases = [
            ("float", FieldType::Primitive(PrimitiveType::Float)),
            ("double", FieldType::Primitive(PrimitiveType::Double)),
            ("char", FieldType::Primitive(PrimitiveType::Char)),
            ("int8_t", FieldType::Primitive(PrimitiveType::Int8)),
            ("uint8_t", FieldType::Primitive(PrimitiveType::Uint8)),
            (
                "uint8_t_mavlink_version",
                FieldType::Primitive(PrimitiveType::Uint8MavlinkVersion),
            ),
            ("int16_t", FieldType::Primitive(PrimitiveType::Int16)),
            ("uint16_t", FieldType::Primitive(PrimitiveType::Uint16)),
            ("int32_t", FieldType::Primitive(PrimitiveType::Int32)),
            ("uint32_t", FieldType::Primitive(PrimitiveType::Uint32)),
            ("int64_t", FieldType::Primitive(PrimitiveType::Int64)),
            ("uint64_t", FieldType::Primitive(PrimitiveType::Uint64)),
            ("float[0]", FieldType::Array(PrimitiveType::Float, 0)),
            ("double[1]", FieldType::Array(PrimitiveType::Double, 1)),
            ("char[2]", FieldType::Array(PrimitiveType::Char, 2)),
            ("int8_t[3]", FieldType::Array(PrimitiveType::Int8, 3)),
            ("uint8_t[4]", FieldType::Array(PrimitiveType::Uint8, 4)),
            (
                "uint8_t_mavlink_version[5]",
                FieldType::Array(PrimitiveType::Uint8MavlinkVersion, 5),
            ),
            ("int16_t[6]", FieldType::Array(PrimitiveType::Int16, 6)),
            ("uint16_t[7]", FieldType::Array(PrimitiveType::Uint16, 7)),
            ("int32_t[8]", FieldType::Array(PrimitiveType::Int32, 8)),
            ("uint32_t[9]", FieldType::Array(PrimitiveType::Uint32, 9)),
            ("int64_t[10]", FieldType::Array(PrimitiveType::Int64, 10)),
            (
                "uint64_t[100]",
                FieldType::Array(PrimitiveType::Uint64, 100),
            ),
        ];

        for (input, output) in valid_cases {
            println!("+> {:?} {:?}", input, output);
            assert_eq!(FieldType::from_str(input).unwrap(), output);
        }

        let invalid_cases = [
            "char_t",
            "not_found",
            "INT8_T",
            "int16_t[9][10]",
            "int16_t[300]",
        ];

        for case in invalid_cases {
            println!("-> {:?}", case);
            FieldType::from_str(case).unwrap_err();
        }
    }

    #[test]
    fn test_min_size() {
        let mut enm = Enum {
            name: Ident::from_str("TEST").unwrap(),
            bitmask: false,
            description: None,
            dev_status: None,
            entries: vec![
                Entry {
                    name: Ident::from_str("TEST_1").unwrap(),
                    description: None,
                    dev_status: None,
                    value: 0,
                },
                Entry {
                    name: Ident::from_str("TEST_1").unwrap(),
                    description: None,
                    dev_status: None,
                    value: 1,
                },
            ],
        };

        assert_eq!(enm.min_rust_size(), RustSizeType::U8);
        enm.entries[0].value = 255;
        assert_eq!(enm.min_rust_size(), RustSizeType::U8);
        enm.entries[0].value = 256;
        assert_eq!(enm.min_rust_size(), RustSizeType::U16);
        enm.entries[0].value = 65535;
        assert_eq!(enm.min_rust_size(), RustSizeType::U16);
        enm.entries[0].value = 65536;
        assert_eq!(enm.min_rust_size(), RustSizeType::U32);
        enm.entries[0].value = 4294967295;
        assert_eq!(enm.min_rust_size(), RustSizeType::U32);
        enm.entries[0].value = 4294967296;
        assert_eq!(enm.min_rust_size(), RustSizeType::U64);
    }
}
