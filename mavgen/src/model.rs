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

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl PrimitiveType {
    pub fn size(self) -> usize {
        match self {
            PrimitiveType::Float => 4,
            PrimitiveType::Double => 8,
            PrimitiveType::Char => 1,
            PrimitiveType::Int8 => 1,
            PrimitiveType::Uint8 => 1,
            PrimitiveType::Uint8MavlinkVersion => 1,
            PrimitiveType::Int16 => 2,
            PrimitiveType::Uint16 => 2,
            PrimitiveType::Int32 => 4,
            PrimitiveType::Uint32 => 4,
            PrimitiveType::Int64 => 8,
            PrimitiveType::Uint64 => 8,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PrimitiveType::Float => "float",
            PrimitiveType::Double => "double",
            PrimitiveType::Char => "char",
            PrimitiveType::Int8 => "int8_t",
            PrimitiveType::Uint8 => "uint8_t",
            PrimitiveType::Uint8MavlinkVersion => "uint8_t",
            PrimitiveType::Int16 => "int16_t",
            PrimitiveType::Uint16 => "uint16_t",
            PrimitiveType::Int32 => "int32_t",
            PrimitiveType::Uint32 => "uint32_t",
            PrimitiveType::Int64 => "int64_t",
            PrimitiveType::Uint64 => "uint64_t",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldType {
    Primitive(PrimitiveType),
    Array(PrimitiveType, u8),
}

impl FieldType {
    pub fn wire_size(self) -> usize {
        match self {
            FieldType::Primitive(typ) => typ.size(),
            FieldType::Array(typ, num) => typ.size() * usize::from(num),
        }
    }

    pub fn primitive_type(self) -> PrimitiveType {
        match self {
            FieldType::Primitive(typ) => typ,
            FieldType::Array(typ, _) => typ,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RustSizeType {
    U8,
    U16,
    U32,
    U64,
}

impl std::fmt::Display for RustSizeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            RustSizeType::U8 => "u8",
            RustSizeType::U16 => "u16",
            RustSizeType::U32 => "u32",
            RustSizeType::U64 => "u64",
        };
        write!(f, "{}", text)
    }
}

impl From<RustSizeType> for PrimitiveType {
    fn from(value: RustSizeType) -> Self {
        match value {
            RustSizeType::U8 => PrimitiveType::Uint8,
            RustSizeType::U16 => PrimitiveType::Uint16,
            RustSizeType::U32 => PrimitiveType::Uint32,
            RustSizeType::U64 => PrimitiveType::Uint64,
        }
    }
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

impl Message {
    pub fn wire_size(&self) -> usize {
        self.fields
            .iter()
            .chain(&self.extension_fields)
            .map(|field| field.r#type.wire_size())
            .sum()
    }
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

impl Message {
    pub fn extra_crc(&self) -> u8 {
        let mut crc = crc_any::CRCu16::crc16mcrf4cc();

        crc.digest(self.name.as_ref().as_bytes());
        crc.digest(b" ");

        for field in &self.fields {
            let typ = match field.r#type {
                FieldType::Primitive(typ) => typ,
                FieldType::Array(typ, _) => typ,
            };

            crc.digest(typ.as_str().as_bytes());
            crc.digest(b" ");

            crc.digest(field.name.as_ref().as_bytes());
            crc.digest(b" ");

            if let FieldType::Array(_, size) = field.r#type {
                crc.digest(&[size]);
            }
        }

        let crcval = crc.get_crc();
        ((crcval & 0xFF) ^ (crcval >> 8)) as u8
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

    fn default_field() -> Field {
        Field {
            name: "empty".parse().unwrap(),
            r#type: FieldType::Primitive(PrimitiveType::Uint8MavlinkVersion),
            print_format: None,
            r#enum: None,
            display: None,
            units: None,
            increment: None,
            min_value: None,
            max_value: None,
            multiplier: None,
            default: None,
            instance: None,
            invalid: None,
            description: None,
        }
    }

    #[test]
    fn test_crc_extra_one_field() {
        let message = Message {
            name: "UAVIONIX_ADSB_TRANSCEIVER_HEALTH_REPORT".parse().unwrap(),
            id: 10003,
            dev_status: None,
            description: None,
            fields: vec![Field {
                name: "rfHealth".parse().unwrap(),
                r#type: FieldType::Primitive(PrimitiveType::Uint8),
                print_format: None,
                r#enum: Some("UAVIONIX_ADSB_RF_HEALTH".parse().unwrap()),
                display: Some("bitmask".into()),
                description: Some("ADS-B transponder messages".into()),
                ..default_field()
            }],
            extension_fields: vec![],
        };

        assert_eq!(message.extra_crc(), 4);
    }

    #[test]
    fn test_crc_extra_many_fields() {
        let message = Message {
            name: "UAVIONIX_ADSB_OUT_DYNAMIC".parse().unwrap(),
            id: 10002,
            dev_status: None,
            description: Some(
                "Dynamic data used to generate ADS-B out transponder data (send at 5Hz)".into(),
            ),
            fields: vec![
                Field {
                    name: "utcTime".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint32),
                    ..default_field()
                },
                Field {
                    name: "gpsLat".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Int32),
                    ..default_field()
                },
                Field {
                    name: "gpsLon".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Int32),
                    ..default_field()
                },
                Field {
                    name: "gpsAlt".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Int32),
                    ..default_field()
                },
                Field {
                    name: "baroAltMSL".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Int32),
                    ..default_field()
                },
                Field {
                    name: "accuracyHor".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint32),
                    ..default_field()
                },
                Field {
                    name: "accuracyVert".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint16),
                    ..default_field()
                },
                Field {
                    name: "accuracyVel".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint16),
                    ..default_field()
                },
                Field {
                    name: "velVert".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Int16),
                    ..default_field()
                },
                Field {
                    name: "velNS".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Int16),
                    ..default_field()
                },
                Field {
                    name: "VelEW".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Int16),
                    ..default_field()
                },
                Field {
                    name: "state".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint16),
                    ..default_field()
                },
                Field {
                    name: "squawk".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint16),
                    ..default_field()
                },
                Field {
                    name: "gpsFix".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8),
                    ..default_field()
                },
                Field {
                    name: "numSats".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8),
                    ..default_field()
                },
                Field {
                    name: "emergencyStatus".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8),
                    ..default_field()
                },
            ],
            extension_fields: vec![],
        };

        assert_eq!(message.extra_crc(), 186);
    }

    #[test]
    fn test_crc_extra_ext_fields() {
        let mut message = Message {
            name: "MEMINFO".parse().unwrap(),
            id: 152,
            dev_status: None,
            description: None,
            fields: vec![
                Field {
                    name: "brkval".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint16),
                    ..default_field()
                },
                Field {
                    name: "freemem".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint16),
                    ..default_field()
                },
            ],
            extension_fields: vec![Field {
                name: "freemem32".parse().unwrap(),
                r#type: FieldType::Primitive(PrimitiveType::Uint32),
                ..default_field()
            }],
        };

        assert_eq!(message.extra_crc(), 208);

        message.extension_fields.pop();
        assert_eq!(message.extra_crc(), 208);
    }

    #[test]
    fn test_crc_extra_mavlink_version_type() {
        let message = Message {
            name: "HEARTBEAT".parse().unwrap(),
            id: 0,
            dev_status: None,
            description: None,
            fields: vec![
                Field {
                    name: "custom_mode".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint32),
                    ..default_field()
                },
                Field {
                    name: "type".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8),
                    ..default_field()
                },
                Field {
                    name: "autopilot".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8),
                    ..default_field()
                },
                Field {
                    name: "base_mode".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8),
                    ..default_field()
                },
                Field {
                    name: "system_status".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8),
                    ..default_field()
                },
                Field {
                    name: "mavlink_version".parse().unwrap(),
                    r#type: FieldType::Primitive(PrimitiveType::Uint8MavlinkVersion),
                    ..default_field()
                },
            ],
            extension_fields: vec![],
        };

        assert_eq!(message.extra_crc(), 50);
    }
}
