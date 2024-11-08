use model::{DevStatus, Field, Message};
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn test_pascal_case() {
    let cases = [
        ("GIMBAL_REPORT", "GimbalReport"),
        ("GOPRO_HEARTBEAT_STATUS", "GoproHeartbeatStatus"),
        ("SYSTEM_TIME", "SystemTime"),
        ("system_time", "SystemTime"),
        ("SystemTime", "SystemTime"),
        ("ASYNC", "Async"),
    ];

    for (input, expected) in cases {
        let ident: model::Ident = input.parse().unwrap();

        assert_eq!(
            ident.pascal_case().to_string(),
            expected,
            "case {:?}",
            input
        );
    }
}

#[test]
fn test_snake_case() {
    let cases = [
        ("msg_pack_size", "msg_pack_size"),
        ("height", "height"),
        ("gpsFix", "gps_fix"),
        ("gps_Fix", "gps_fix"),
        ("COG", "cog"),
        ("gpsOffsetLat", "gps_offset_lat"),
        ("sue_GPS_TYPE", "sue_gps_type"),
        ("Type", "r#type"),
    ];

    for (input, expected) in cases {
        let ident: model::Ident = input.parse().unwrap();
        assert_eq!(ident.snake_case().to_string(), expected, "case {:?}", input);
    }
}

#[test]
fn test_empty_input() {
    let result = Codegen::default().emit_doc(None, None);
    assert!(result.is_empty());
}

#[test]
fn test_description_only() {
    let description = "    A test description.    ";
    let result = Codegen::default().emit_doc(Some(description), None);
    let expected = quote! { #[doc = "A test description."] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_description_with_leading_newline() {
    let description = "    A test\n    description\n\talso this";
    let result = Codegen::default().emit_doc(Some(description), None);
    let expected = quote! { #[doc = "A test\ndescription\nalso this"] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_wip_status_only() {
    let dev_status = model::DevStatus::Wip {
        since: Some("2024-09-02".to_owned()),
        description: Some("Use this fancy thing".to_owned()),
    };
    let result = Codegen::default().emit_doc(None, Some(&dev_status));
    let expected = quote! { #[doc = "WIP since 2024-09-02 - Use this fancy thing"] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_wip_status_without_since() {
    let dev_status = model::DevStatus::Wip {
        since: None,
        description: Some("   Use fancy pants   ".to_owned()),
    };
    let result = Codegen::default().emit_doc(None, Some(&dev_status));
    let expected = quote! { #[doc = "WIP - Use fancy pants"] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_wip_status_without_description() {
    let dev_status = model::DevStatus::Wip {
        since: Some("2024-09-02".to_owned()),
        description: None,
    };
    let result = Codegen::default().emit_doc(None, Some(&dev_status));
    let expected = quote! { #[doc = "WIP since 2024-09-02"] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_deprecated_status() {
    let dev_status = model::DevStatus::Deprecated {
        since: "2024-09-02".to_string(),
        replaced_by: "pes_patron".to_string(),
        description: Some("Use pes_patron instead".to_string()),
    };
    let result = Codegen::default().emit_doc(None, Some(&dev_status));
    let expected = quote! { #[deprecated(note = "Since 2024-09-02, replaced by pes_patron. Use pes_patron instead")] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_deprecated_status_without_description() {
    let dev_status = model::DevStatus::Deprecated {
        since: "2024-09-02".to_string(),
        replaced_by: "pes_patron".to_string(),
        description: None,
    };
    let result = Codegen::default().emit_doc(None, Some(&dev_status));
    let expected = quote! { #[deprecated(note = "Since 2024-09-02, replaced by pes_patron")] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_description_and_wip_status() {
    let description = "This is a test description.";
    let dev_status = model::DevStatus::Wip {
        since: Some("2024-09-02".to_owned()),
        description: Some("Work in progress".to_owned()),
    };
    let result = Codegen::default().emit_doc(Some(description), Some(&dev_status));
    let expected = quote! { #[doc = "WIP since 2024-09-02 - Work in progress\n\nThis is a test description."] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_description_with_tabs() {
    let description = "This is a\ttest\tdescription with tabs.";
    let result = Codegen::default().emit_doc(Some(description), None);
    let expected = quote! { #[doc = "This is a    test    description with tabs."] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_deprecated_status_with_tabs() {
    let dev_status = model::DevStatus::Deprecated {
        since: "2024-09-02".to_string(),
        replaced_by: "pes_patron".to_string(),
        description: Some("Use\n\tpes_patron\n\tinstead".to_string()),
    };
    let result = Codegen::default().emit_doc(None, Some(&dev_status));
    let expected = quote! { #[deprecated(note = "Since 2024-09-02, replaced by pes_patron. Use\n    pes_patron\n    instead")] };
    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_basic_enum() {
    let enum_def = model::Enum {
        name: "TestEnum".parse().unwrap(),
        bitmask: false,
        description: None,
        dev_status: None,
        entries: vec![
            model::Entry {
                name: "One".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 1,
            },
            model::Entry {
                name: "Two".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 2,
            },
        ],
    };

    let result = Codegen::default().emit_regular_enum(&enum_def);

    let expected = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[repr(u8)]
        pub enum TestEnum {
            One = 1,
            Two = 2,
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_enum_with_description() {
    let enum_def = model::Enum {
        name: "DescEnum".parse().unwrap(),
        bitmask: false,
        description: Some("This is a test enum".to_string()),
        dev_status: None,
        entries: vec![model::Entry {
            name: "First".parse().unwrap(),
            description: Some("First entry".to_string()),
            dev_status: None,
            value: 0,
        }],
    };

    let result = Codegen::default().emit_regular_enum(&enum_def);

    let expected = quote! {
        #[doc = "This is a test enum"]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[repr(u8)]
        pub enum DescEnum {
            #[doc = "First entry"]
            First = 0,
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_enum_with_dev_status() {
    let enum_def = model::Enum {
        name: "StatusEnum".parse().unwrap(),
        bitmask: false,
        description: None,
        dev_status: Some(model::DevStatus::Wip {
            since: Some("v1.0.0".to_string()),
            description: Some("Work in progress".to_string()),
        }),
        entries: vec![model::Entry {
            name: "Entry1".parse().unwrap(),
            description: None,
            dev_status: Some(model::DevStatus::Deprecated {
                since: "v2.0.0".to_string(),
                replaced_by: "NewEntry".to_string(),
                description: Some("Use NewEntry instead".to_string()),
            }),
            value: 0,
        }],
    };

    let result = Codegen::default().emit_regular_enum(&enum_def);

    let expected = quote! {
        #[doc = "WIP since v1.0.0 - Work in progress"]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[repr(u8)]
        pub enum StatusEnum {
            #[deprecated(note = "Since v2.0.0, replaced by NewEntry. Use NewEntry instead")]
            Entry1 = 0,
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_enum_with_larger_values() {
    let enum_def = model::Enum {
        name: "LARGE_ENUM".parse().unwrap(),
        bitmask: false,
        description: None,
        dev_status: None,
        entries: vec![
            model::Entry {
                name: "SMALL".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 0,
            },
            model::Entry {
                name: "LARGE".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 1000000,
            },
        ],
    };

    let result = Codegen::default().emit_regular_enum(&enum_def);

    let expected = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[repr(u32)]
        pub enum LargeEnum {
            Small = 0,
            Large = 1000000,
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_basic_bitmask_enum() {
    let enum_def = model::Enum {
        name: "TestFlags".parse().unwrap(),
        bitmask: true,
        description: None,
        dev_status: None,
        entries: vec![
            model::Entry {
                name: "inferno".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 1,
            },
            model::Entry {
                name: "dust2".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 2,
            },
        ],
    };

    let result = Codegen::default().emit_bitmask_enum(&enum_def);

    let expected = quote! {
        bitflags! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct TestFlags: u8 {
                const Inferno = 1;
                const Dust2 = 2;
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_bitmask_enum_with_description() {
    let enum_def = model::Enum {
        name: "DescFlags".parse().unwrap(),
        bitmask: true,
        description: Some("This is a test bitmask enum".to_string()),
        dev_status: None,
        entries: vec![model::Entry {
            name: "First".parse().unwrap(),
            description: Some("First flag".to_string()),
            dev_status: None,
            value: 1,
        }],
    };

    let result = Codegen::default().emit_bitmask_enum(&enum_def);

    let expected = quote! {
        bitflags! {
            #[doc = "This is a test bitmask enum"]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct DescFlags: u8 {
                #[doc = "First flag"]
                const First = 1;
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_bitmask_enum_with_dev_status() {
    let enum_def = model::Enum {
        name: "status_flags".parse().unwrap(),
        bitmask: true,
        description: None,
        dev_status: Some(model::DevStatus::Wip {
            since: Some("v1.0.0".to_string()),
            description: Some("Work in progress".to_string()),
        }),
        entries: vec![model::Entry {
            name: "OLD_FLAG".parse().unwrap(),
            description: None,
            dev_status: Some(model::DevStatus::Deprecated {
                since: "v2.0.0".to_string(),
                replaced_by: "NewFlag".to_string(),
                description: Some("Use NewFlag instead".to_string()),
            }),
            value: 1,
        }],
    };

    let result = Codegen::default().emit_bitmask_enum(&enum_def);

    let expected = quote! {
        bitflags! {
            #[doc = "WIP since v1.0.0 - Work in progress"]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct StatusFlags: u8 {
                #[deprecated(note = "Since v2.0.0, replaced by NewFlag. Use NewFlag instead")]
                const OldFlag = 1;
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_bitmask_enum_with_larger_values() {
    let enum_def = model::Enum {
        name: "LargeFlags".parse().unwrap(),
        bitmask: true,
        description: None,
        dev_status: None,
        entries: vec![
            model::Entry {
                name: "Small".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 1,
            },
            model::Entry {
                name: "Large".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 1 << 31,
            },
        ],
    };

    let result = Codegen::default().emit_bitmask_enum(&enum_def);

    let expected = quote! {
        bitflags! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct LargeFlags: u32 {
                const Small = 1;
                const Large = 2147483648;
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_basic_enum_default_impl() {
    let enum_def = model::Enum {
        name: "TestEnum".parse().unwrap(),
        bitmask: false,
        description: None,
        dev_status: None,
        entries: vec![
            model::Entry {
                name: "First".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 0,
            },
            model::Entry {
                name: "Second".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 1,
            },
        ],
    };

    let result = Codegen::default().emit_enum_default_impl(&enum_def);

    let expected = quote! {
        impl TestEnum {
            pub const DEFAULT: Self = Self::First;
        }
        impl Default for TestEnum {
            fn default() -> Self {
                Self::DEFAULT
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_bitmask_enum_converters() {
    let enum_def = model::Enum {
        name: "TestFlags".parse().unwrap(),
        bitmask: true,
        description: None,
        dev_status: None,
        entries: vec![model::Entry {
            name: "First".parse().unwrap(),
            description: None,
            dev_status: None,
            value: 0,
        }],
    };

    let result = Codegen::default().emit_enum_converters(&enum_def);

    let expected = quote! {
        impl TestFlags {
            #[allow(unused)]
            fn try_from_bits(value: u8) -> Result<Self, ParserError> {
                Self::from_bits(value).ok_or(ParserError::InvalidFlag {
                    flag_type: "TestFlags",
                    value: value as u32,
                })
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_regular_enum_converters() {
    let enum_def = model::Enum {
        name: "TestEnum".parse().unwrap(),
        bitmask: false,
        description: None,
        dev_status: None,
        entries: vec![model::Entry {
            name: "First".parse().unwrap(),
            description: None,
            dev_status: None,
            value: 0,
        }],
    };

    let result = Codegen::default().emit_enum_converters(&enum_def);

    let expected = quote! {
        impl TestEnum {
            #[allow(unused)]
            fn try_from_bits(value: u8) -> Result<Self, ParserError> {
                FromPrimitive::from_u8(value).ok_or(ParserError::InvalidEnum {
                    enum_type: "TestEnum",
                    value: value as u32,
                })
            }
            pub fn bits(self) -> u8 {
                self as _
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_emit_regular_enum() {
    let enum_def = model::Enum {
        name: "COOL_ENUM".parse().unwrap(),
        bitmask: false,
        description: Some("A test enum".to_string()),
        dev_status: None,
        entries: vec![
            model::Entry {
                name: "First".parse().unwrap(),
                description: Some("First entry".to_string()),
                dev_status: None,
                value: 0,
            },
            model::Entry {
                name: "Second".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 1,
            },
        ],
    };

    let result = Codegen::default().emit_enum(&enum_def);

    let expected = quote! {
        #[doc = "A test enum"]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[repr(u8)]
        pub enum CoolEnum {
            #[doc = "First entry"]
            First = 0,
            Second = 1,
        }

        impl CoolEnum {
            pub const DEFAULT: Self = Self::First;
        }

        impl Default for CoolEnum {
            fn default() -> Self {
                Self::DEFAULT
            }
        }

        impl CoolEnum {
            #[allow(unused)]
            fn try_from_bits(value: u8) -> Result<Self, ParserError> {
                FromPrimitive::from_u8(value).ok_or(ParserError::InvalidEnum {
                    enum_type: "COOL_ENUM",
                    value: value as u32,
                })
            }
            pub fn bits(self) -> u8 {
                self as _
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

#[test]
fn test_emit_bitmask_enum() {
    let enum_def = model::Enum {
        name: "COOL_BITMASK_ENUM".parse().unwrap(),
        bitmask: true,
        description: Some("A test bitmask enum".to_string()),
        dev_status: None,
        entries: vec![
            model::Entry {
                name: "Flag1".parse().unwrap(),
                description: Some("First flag".to_string()),
                dev_status: None,
                value: 1,
            },
            model::Entry {
                name: "Flag2".parse().unwrap(),
                description: None,
                dev_status: None,
                value: 2,
            },
        ],
    };

    let result = Codegen::default().emit_enum(&enum_def);

    let expected = quote! {
        bitflags! {
            #[doc = "A test bitmask enum"]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
            pub struct CoolBitmaskEnum: u8 {
                #[doc = "First flag"]
                const Flag1 = 1;
                const Flag2 = 2;
            }
        }

        impl CoolBitmaskEnum {
            pub const DEFAULT: Self = Self::Flag1;
        }

        impl Default for CoolBitmaskEnum {
            fn default() -> Self {
                Self::DEFAULT
            }
        }

        impl CoolBitmaskEnum {
            #[allow(unused)]
            fn try_from_bits(value: u8) -> Result<Self, ParserError> {
                Self::from_bits(value).ok_or(ParserError::InvalidFlag {
                    flag_type: "COOL_BITMASK_ENUM",
                    value: value as u32,
                })
            }
        }
    };

    assert_eq!(result.to_string(), expected.to_string());
}

fn default_field() -> Field {
    Field {
        name: "test".parse().unwrap(),
        r#type: FieldType::Primitive(PrimitiveType::Char),
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

fn test_message() -> Message {
    model::Message {
        name: "COOL_TEST_MESSAGE".parse().unwrap(),
        id: 10002,
        dev_status: None,
        description: Some("Some test message".into()),
        fields: vec![
            Field {
                name: "enum_array".parse().unwrap(),
                r#enum: Some("COOL_ENUM".parse().unwrap()),
                r#type: FieldType::Array(PrimitiveType::Uint8, 4),
                description: Some("Array of enums".into()),
                ..default_field()
            },
            Field {
                name: "enum_plain".parse().unwrap(),
                r#enum: Some("COOL_ENUM".parse().unwrap()),
                r#type: FieldType::Primitive(PrimitiveType::Int32),
                description: Some("Regular enum".into()),
                ..default_field()
            },
            Field {
                name: "plain".parse().unwrap(),
                r#type: FieldType::Primitive(PrimitiveType::Int8),
                ..default_field()
            },
            Field {
                name: "plain_array".parse().unwrap(),
                r#type: FieldType::Array(PrimitiveType::Char, 20),
                ..default_field()
            },
        ],
        extension_fields: vec![Field {
            name: "extension_field".parse().unwrap(),
            r#type: FieldType::Array(PrimitiveType::Uint64, 8),
            description: Some("Emergency status".into()),
            ..default_field()
        }],
    }
}

#[test]
fn test_emit_message_def() {
    let codegen = Codegen::default();

    let message = test_message();

    let stream = codegen.emit_message_def(&message);
    let expected = quote! {
        #[doc = "Some test message"]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        pub struct CoolTestMessage {
            #[doc = "Array of enums"]
            #[cfg_attr(feature = "serde", serde(with = "serde_arrays"))]
            pub enum_array: [CoolEnum; 4usize],
            #[doc = "Regular enum"]
            pub enum_plain: CoolEnum,
            pub plain: i8,
            #[cfg_attr(feature = "serde", serde(with = "serde_arrays"))]
            pub plain_array: [u8; 20usize],
            #[doc = "Emergency status"]
            #[cfg_attr(feature = "serde", serde(with = "serde_arrays"))]
            #[cfg_attr(feature = "serde", serde(default = "mavlink_core::utils::RustDefault::rust_default"))]
            pub extension_field: [u64; 8usize]
        }
    };

    assert_eq!(stream.to_string(), expected.to_string());
}

#[test]
fn test_emit_message_def_without_eq() {
    let codegen = Codegen::default();
    let message = Message {
        name: "NON_EQ_MESSAGE".parse().unwrap(),
        id: 7,
        dev_status: Some(DevStatus::Deprecated {
            since: "2024-09".into(),
            replaced_by: "YOU".into(),
            description: None,
        }),
        description: None,
        fields: vec![Field {
            name: "float_field".parse().unwrap(),
            r#type: FieldType::Primitive(PrimitiveType::Float),
            ..default_field()
        }],
        extension_fields: vec![],
    };

    let stream = codegen.emit_message_def(&message);
    let expected = quote! {
        #[deprecated(note = "Since 2024-09, replaced by YOU")]
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        pub struct NonEqMessage {
            pub float_field: f32
        }
    };

    assert_eq!(stream.to_string(), expected.to_string());
}

#[test]
fn test_emit_message_default_impl() {
    let codegen = Codegen::default();
    let message = test_message();

    let stream = codegen.emit_message_default_impl(&message);
    let expected = quote! {
        impl CoolTestMessage {
            pub const DEFAULT: Self = Self {
                enum_array: [CoolEnum::DEFAULT; 4usize],
                enum_plain: CoolEnum::DEFAULT,
                plain: 0,
                plain_array: [0; 20usize],
                extension_field: [0; 8usize]
            };
        }

        impl Default for CoolTestMessage {
            fn default() -> Self {
                Self::DEFAULT
            }
        }
    };

    assert_eq!(stream.to_string(), expected.to_string());
}

#[test]
fn test_emit_message_message_data_impl() {
    let codegen = Codegen::default();
    let message = test_message();

    let stream = codegen.emit_message_message_data_impl(&message);
    let expected = quote! {
        impl MessageData for CoolTestMessage {
            type Message = MavMessage;
            const ID: u32 = 10002u32;
            const NAME: &'static str = "COOL_TEST_MESSAGE";
            const EXTRA_CRC: u8 = 62u8;
            const ENCODED_LEN: usize = 93usize;

            fn ser(&self, version: MavlinkVersion, bytes: &mut [u8]) -> usize {
                let mut __cursor = BytesMut::new(bytes);
                if __cursor.remaining() < Self::ENCODED_LEN {
                    panic!(
                        "buffer is too small (need {} bytes, but got {})",
                        Self::ENCODED_LEN,
                        __cursor.remaining(),
                    );
                }

                __cursor.put_i32_le(self.enum_plain.bits().try_into().expect("checked"));
                for i in 0..4usize {
                    __cursor.put_u8(self.enum_array[i].bits().try_into().expect("checked"));
                }
                __cursor.put_i8(self.plain);
                __cursor.put_slice(&self.plain_array);
                for i in 0..8usize {
                    __cursor.put_u64_le(self.extension_field[i]);
                }

                if matches!(version, MavlinkVersion::V2) {
                    let len = __cursor.len();
                    ::mavlink_core::utils::remove_trailing_zeroes(&bytes[..len])
                } else {
                    __cursor.len()
                }
            }

            fn deser(
                _version: MavlinkVersion,
                __input: &[u8],
            ) -> Result<Self, ::mavlink_core::error::ParserError> {
                let __avail_len = __input.len();
                let mut __payload_buf = [0; Self::ENCODED_LEN];
                let mut __cursor = if __avail_len < Self::ENCODED_LEN {
                    __payload_buf[0..__avail_len].copy_from_slice(__input);
                    Bytes::new(&__payload_buf)
                } else {
                    Bytes::new(__input)
                };
                Ok(Self{
                    enum_plain: CoolEnum::try_from_bits(__cursor.get_i32_le().try_into().expect("checked"))?,
                    enum_array: [
                        CoolEnum::try_from_bits(__cursor.get_u8().try_into().expect("checked"))?,
                        CoolEnum::try_from_bits(__cursor.get_u8().try_into().expect("checked"))?,
                        CoolEnum::try_from_bits(__cursor.get_u8().try_into().expect("checked"))?,
                        CoolEnum::try_from_bits(__cursor.get_u8().try_into().expect("checked"))?,
                    ],
                    plain: __cursor.get_i8(),
                    plain_array: __cursor.get_array(),
                    extension_field: [
                        __cursor.get_u64_le(),
                        __cursor.get_u64_le(),
                        __cursor.get_u64_le(),
                        __cursor.get_u64_le(),
                        __cursor.get_u64_le(),
                        __cursor.get_u64_le(),
                        __cursor.get_u64_le(),
                        __cursor.get_u64_le(),
                    ]
                })
            }
        }
    };

    assert_eq!(stream.to_string(), expected.to_string());
}

fn test_mav_messages() -> Vec<model::Message> {
    vec![
        model::Message {
            name: "HEARTBEAT".parse().unwrap(),
            id: 0,
            dev_status: None,
            description: None,
            fields: vec![default_field()],
            extension_fields: vec![],
        },
        model::Message {
            name: "PROTOCOL_VERSION".parse().unwrap(),
            id: 300,
            dev_status: None,
            description: None,
            fields: vec![default_field()],
            extension_fields: vec![],
        },
    ]
}

#[test]
fn test_emit_mav_message_def() {
    let codegen = Codegen::default();
    let messages = test_mav_messages();

    let stream = codegen.emit_mav_message_def(&messages);
    let expected = quote! {
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[cfg_attr(feature = "serde", serde(tag = "mavpackettype"))]
        pub enum MavMessage {
            #[cfg_attr(feature = "serde", serde(rename = "HEARTBEAT"))]
            Heartbeat(Heartbeat),
            #[cfg_attr(feature = "serde", serde(rename = "PROTOCOL_VERSION"))]
            ProtocolVersion(ProtocolVersion),
        }
    };

    assert_eq!(stream.to_string(), expected.to_string());
}

#[test]
fn test_emit_mav_message_impl() {
    let codegen = Codegen::default();
    let messages = test_mav_messages();

    let stream = codegen.emit_mav_message_impl(&messages);
    let expected = quote! {
        impl Message for MavMessage {
            fn ser(&self, version: MavlinkVersion, bytes: &mut [u8]) -> usize {
                match self {
                    Self::Heartbeat(body) => body.ser(version, bytes),
                    Self::ProtocolVersion(body) => body.ser(version, bytes),
                }
            }

            fn parse(
                version: MavlinkVersion,
                id: u32,
                payload: &[u8],
            ) -> Result<Self, ::mavlink_core::error::ParserError> {
                match id {
                    Heartbeat::ID => {
                        Heartbeat::deser(version, payload).map(Self::Heartbeat)
                    },
                    ProtocolVersion::ID => {
                        ProtocolVersion::deser(version, payload).map(Self::ProtocolVersion)
                    },
                    id => Err(::mavlink_core::error::ParserError::UnknownMessage { id })
                }
            }

            fn message_name(&self) -> &'static str {
                match self {
                    Self::Heartbeat(..) => Heartbeat::NAME,
                    Self::ProtocolVersion(..) => ProtocolVersion::NAME,
                }
            }

            fn message_id(&self) -> u32 {
                match self {
                    Self::Heartbeat(..) => Heartbeat::ID,
                    Self::ProtocolVersion(..) => ProtocolVersion::ID,
                }
            }

            fn message_id_from_name(name: &str) -> Result<u32, &'static str> {
                match name {
                    Heartbeat::NAME => Ok(Heartbeat::ID),
                    ProtocolVersion::NAME => Ok(ProtocolVersion::ID),
                    _ => Err("Invalid message name."),
                }
            }

            fn default_message_from_id(id: u32) -> Result<Self, &'static str> {
                match id {
                    Heartbeat::ID => Ok(Self::Heartbeat(Heartbeat::default())),
                    ProtocolVersion::ID => Ok(Self::ProtocolVersion(ProtocolVersion::default())),
                    _ => Err("Invalid message id."),
                }
            }

            fn extra_crc(id: u32) -> u8 {
                match id {
                    Heartbeat::ID => Heartbeat::EXTRA_CRC,
                    ProtocolVersion::ID => ProtocolVersion::EXTRA_CRC,
                    _ => 0,
                }
            }
        }
    };

    assert_eq!(stream.to_string(), expected.to_string());
}
