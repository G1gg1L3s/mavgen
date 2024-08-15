use std::collections::{HashMap, HashSet};

use crate::{
    flatten,
    model::{
        DevStatus, Entry, Enum, Field, FieldType, Ident, MavlinkModule, Message, PrimitiveType,
        RustSizeType,
    },
    xml,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    InvalidName {
        item: &'static str,
        super_item: Option<Ident>,
        name: String,
    },
    ItemRedefinition {
        item: &'static str,
        super_item: Option<Ident>,
        name: Ident,
    },
    InvalidEntry {
        err: ParseEntryValueError,
        r#enum: Ident,
        entry: String,
    },
    BitmaskWithoutValue {
        r#enum: Ident,
    },
    RepeatedEntryValue {
        r#enum: Ident,
        entry_1: Ident,
        entry_2: Ident,
        value: u64,
    },
    RepeatedMessageId {
        msg_1: Ident,
        msg_2: Ident,
        id: u32,
    },
    InvalidFieldType {
        message: Ident,
        field: Ident,
        r#type: String,
    },
    InvalidEnumReference {
        message: Ident,
        field: Ident,
        r#enum: String,
    },
    NoSubItems {
        item: &'static str,
        sub_items: &'static str,
        name: Ident,
    },
    FieldTypeIsIncompatibleWithEnum {
        message: Ident,
        field: Ident,
        r#enum: Ident,
        field_type: FieldType,
    },
    MessageIsTooBig {
        message: Ident,
        size: usize,
        max_size: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct MaybeSuper<'a>(Option<&'a Ident>);

        impl<'a> std::fmt::Display for MaybeSuper<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if let Some(ident) = self.0 {
                    write!(f, "[{ident}]")?;
                }
                Ok(())
            }
        }

        match self {
            Error::InvalidName {
                item,
                super_item,
                name,
            } => write!(
                f,
                "{} {} has invalid name {:?}",
                MaybeSuper(super_item.as_ref()),
                item,
                name
            ),
            Error::ItemRedefinition {
                item,
                super_item,
                name,
            } => write!(
                f,
                "{} {} {} is defined multiple times",
                MaybeSuper(super_item.as_ref()),
                item,
                name
            ),
            Error::InvalidEntry {
                err: _,
                r#enum,
                entry,
            } => write!(
                f,
                "{} invalid enum entry {}",
                MaybeSuper(Some(r#enum)),
                entry,
            ),

            Error::BitmaskWithoutValue { r#enum } => write!(
                f,
                "{} bitmask without explicit values is not supported",
                MaybeSuper(Some(r#enum)),
            ),
            Error::RepeatedEntryValue {
                r#enum,
                entry_1,
                entry_2,
                value,
            } => write!(
                f,
                "{} entries {} and {} have the same value of {}",
                MaybeSuper(Some(r#enum)),
                entry_1,
                entry_2,
                value,
            ),
            Error::RepeatedMessageId { msg_1, msg_2, id } => write!(
                f,
                "messages {} and {} have the same message ID of {}",
                msg_1, msg_2, id,
            ),
            Error::InvalidFieldType {
                message,
                field,
                r#type,
            } => write!(
                f,
                "{} field {} has invalid type {:?}",
                MaybeSuper(Some(message)),
                field,
                r#type,
            ),
            Error::InvalidEnumReference {
                message,
                field,
                r#enum,
            } => write!(
                f,
                "{} field {} references undefined enum {:?}",
                MaybeSuper(Some(message)),
                field,
                r#enum,
            ),
            Error::NoSubItems {
                item,
                sub_items,
                name,
            } => write!(f, "{} {} has no {}", item, name, sub_items,),
            Error::FieldTypeIsIncompatibleWithEnum {
                message,
                field,
                r#enum,
                field_type,
            } => write!(
                f,
                "{} field {} has type {:?} which is incompatible with enum {}",
                MaybeSuper(Some(message)),
                field,
                field_type,
                r#enum,
            ),
            Error::MessageIsTooBig {
                message,
                size,
                max_size,
            } => write!(
                f,
                "{} payload is too big: wire size is {} bytes, but allowed maximum {}",
                MaybeSuper(Some(message)),
                size,
                max_size,
            ),
        }
    }
}

impl std::error::Error for Error {}

fn non_empty(str: String) -> Option<String> {
    if str.is_empty() {
        None
    } else {
        Some(str)
    }
}

#[derive(Debug, Clone, Copy)]
struct NormalisedEnum {
    size: RustSizeType,
}

#[derive(Debug, Clone, Default)]
pub struct Normaliser {
    enums: HashMap<Ident, NormalisedEnum>,
    messages: HashSet<Ident>,
    allocated_message_ids: HashMap<u32, Ident>,

    errors: Vec<Error>,
}

impl Normaliser {
    pub fn normalise_module(
        mut self,
        module: flatten::MavlinkModule,
    ) -> Result<MavlinkModule, Vec<Error>> {
        let enums = self.normalise_enums(module.enums);
        let messages = self.normalise_messages(module.messages);

        if self.errors.is_empty() {
            Ok(MavlinkModule {
                path: module.path,
                version: module.version,
                dialect: module.dialect,
                enums,
                messages,
            })
        } else {
            Err(self.errors)
        }
    }

    fn normalise_enums(&mut self, enums: Vec<xml::Enum>) -> Vec<Enum> {
        enums
            .into_iter()
            .filter_map(|r#enum| match self.normalise_enum(r#enum) {
                Ok(ok) => Some(ok),
                Err(err) => {
                    self.errors.push(err);
                    None
                }
            })
            .collect()
    }

    fn normalise_enum(&mut self, r#enum: xml::Enum) -> Result<Enum, Error> {
        let Ok(name) = r#enum.name.parse::<Ident>() else {
            return Err(Error::InvalidName {
                item: "enum",
                super_item: None,
                name: r#enum.name,
            });
        };

        if self.enums.contains_key(&name) {
            return Err(Error::ItemRedefinition {
                item: "enum",
                super_item: None,
                name: name.clone(),
            });
        }

        let bitmask = r#enum.bitmask.unwrap_or(false);
        let description = r#enum.description.map(|desc| desc.into_string());
        let dev_status = r#enum.dev_status.map(DevStatus::from);

        if r#enum.entries.is_empty() {
            return Err(Error::NoSubItems {
                item: "enum",
                sub_items: "entries",
                name,
            });
        }

        let entries = self.normalise_entries(&name, bitmask, r#enum.entries)?;

        let final_enum = Enum {
            name,
            bitmask,
            description,
            dev_status,
            entries,
        };

        self.enums.insert(
            final_enum.name.clone(),
            NormalisedEnum {
                size: final_enum.min_rust_size(),
            },
        );

        Ok(final_enum)
    }

    fn normalise_entries(
        &self,
        r#enum: &Ident,
        bitmask: bool,
        entries: Vec<xml::Entry>,
    ) -> Result<Vec<Entry>, Error> {
        // TODO: probably should check that bitmask values do not overlap
        if bitmask && entries.iter().any(|entry| entry.value.is_none()) {
            return Err(Error::BitmaskWithoutValue {
                r#enum: r#enum.clone(),
            });
        }

        let values = derive_enum_entry_values(r#enum, &entries)?;
        let mut result = Vec::with_capacity(entries.len());
        let mut allocated_values = HashMap::with_capacity(entries.len());
        let mut allocated_names = HashSet::with_capacity(entries.len());

        for (entry, value) in entries.into_iter().zip(values) {
            let Ok(name) = entry.name.parse::<Ident>() else {
                return Err(Error::InvalidName {
                    item: "entry",
                    super_item: Some(r#enum.clone()),
                    name: entry.name,
                });
            };

            let description = entry.description.map(|desc| desc.into_string());
            let dev_status = entry.dev_status.map(DevStatus::from);

            let new = allocated_names.insert(name.clone());
            if !new {
                return Err(Error::ItemRedefinition {
                    item: "entry",
                    super_item: Some(r#enum.clone()),
                    name,
                });
            }

            let old = allocated_values.insert(value, name.clone());
            if let Some(old_entry) = old {
                return Err(Error::RepeatedEntryValue {
                    r#enum: r#enum.clone(),
                    entry_1: name.clone(),
                    entry_2: old_entry,
                    value,
                });
            }

            result.push(Entry {
                name,
                description,
                dev_status,
                value,
            })
        }

        Ok(result)
    }

    fn normalise_messages(&mut self, messages: Vec<xml::Message>) -> Vec<Message> {
        messages
            .into_iter()
            .filter_map(|message| match self.normalise_message(message) {
                Ok(ok) => Some(ok),
                Err(err) => {
                    self.errors.push(err);
                    None
                }
            })
            .collect()
    }

    fn normalise_message(&mut self, message: xml::Message) -> Result<Message, Error> {
        let Ok(name) = message.name.parse::<Ident>() else {
            return Err(Error::InvalidName {
                item: "message",
                super_item: None,
                name: message.name,
            });
        };

        let new = self.messages.insert(name.clone());
        if !new {
            return Err(Error::ItemRedefinition {
                item: "message",
                super_item: None,
                name,
            });
        }

        let old = self.allocated_message_ids.insert(message.id, name.clone());
        if let Some(old) = old {
            return Err(Error::RepeatedMessageId {
                msg_1: old,
                msg_2: name,
                id: message.id,
            });
        }

        let description = message.description.map(|desc| desc.into_string());
        let dev_status = message.dev_status.map(DevStatus::from);

        if message.fields.is_empty() {
            return Err(Error::NoSubItems {
                item: "message",
                sub_items: "fields",
                name,
            });
        }

        let (fields, extension_fields) =
            self.normalise_fields(&name, message.fields, message.extension_fields)?;

        Ok(Message {
            name,
            id: message.id,
            dev_status,
            description,
            fields,
            extension_fields,
        })
    }

    fn normalise_fields(
        &self,
        message: &Ident,
        fields: Vec<xml::Field>,
        extension_fields: Vec<xml::Field>,
    ) -> Result<(Vec<Field>, Vec<Field>), Error> {
        let mut result_fields = Vec::with_capacity(fields.len());
        let mut result_extension_fields = Vec::with_capacity(extension_fields.len());

        let mut allocated_field_names =
            HashSet::with_capacity(fields.len() + extension_fields.len());

        #[derive(Clone, Copy)]
        enum FieldKind {
            Regular,
            Extension,
        }

        let fields = fields
            .into_iter()
            .zip(std::iter::repeat(FieldKind::Regular));
        let ext_fields = extension_fields
            .into_iter()
            .zip(std::iter::repeat(FieldKind::Extension));

        for (field, kind) in fields.chain(ext_fields) {
            let field = self.normalise_field(message, field)?;

            let new = allocated_field_names.insert(field.name.clone());
            if !new {
                return Err(Error::ItemRedefinition {
                    item: "field",
                    super_item: Some(message.clone()),
                    name: field.name,
                });
            }

            match kind {
                FieldKind::Regular => result_fields.push(field),
                FieldKind::Extension => result_extension_fields.push(field),
            }
        }

        // See https://mavlink.io/en/guide/serialization.html#field_reordering
        result_fields.sort_by_key(|field| {
            std::cmp::Reverse(match field.r#type {
                FieldType::Primitive(typ) => typ.size(),
                FieldType::Array(typ, _) => typ.size(),
            })
        });

        let total_wire_size: usize = result_fields
            .iter()
            .chain(&result_extension_fields)
            .map(|field| field.r#type.wire_size())
            .sum();

        // Maximum size of payload is 255 bytes
        // https://mavlink.io/en/guide/serialization.html#mavlink2_packet_format
        if total_wire_size > 255 {
            return Err(Error::MessageIsTooBig {
                message: message.clone(),
                size: total_wire_size,
                max_size: 255,
            });
        }

        Ok((result_fields, result_extension_fields))
    }

    fn normalise_field(&self, message: &Ident, field: xml::Field) -> Result<Field, Error> {
        let Ok(name) = field.name.parse::<Ident>() else {
            return Err(Error::InvalidName {
                item: "field",
                super_item: Some(message.clone()),
                name: field.name,
            });
        };

        let Ok(r#type) = field.r#type.parse::<FieldType>() else {
            return Err(Error::InvalidFieldType {
                message: message.clone(),
                field: name,
                r#type: field.r#type,
            });
        };

        let r#enum = if let Some(r#enum) = field.r#enum {
            let r#enum = self.resolve_enum_reference(r#enum, &r#type, message, &name)?;
            Some(r#enum.clone())
        } else {
            None
        };

        Ok(Field {
            name,
            r#type,
            print_format: field.print_format,
            r#enum,
            display: field.display,
            units: field.units,
            increment: field.increment,
            min_value: field.min_value,
            max_value: field.max_value,
            multiplier: field.multiplier,
            default: field.default,
            instance: field.instance,
            invalid: field.invalid,
            description: non_empty(field.description),
        })
    }

    fn resolve_enum_reference(
        &self,
        r#enum: String,
        r#type: &FieldType,
        message: &Ident,
        field: &Ident,
    ) -> Result<&Ident, Error> {
        let enum_reference = r#enum
            .parse::<Ident>()
            .ok()
            .and_then(|r#enum| self.enums.get_key_value(&r#enum));
        let Some((r#enum, enum_meta)) = enum_reference else {
            return Err(Error::InvalidEnumReference {
                message: message.clone(),
                field: field.clone(),
                r#enum,
            });
        };
        let field_size = match *r#type {
            FieldType::Primitive(PrimitiveType::Uint8) => RustSizeType::U8,
            FieldType::Primitive(PrimitiveType::Uint16) => RustSizeType::U16,
            FieldType::Primitive(PrimitiveType::Uint32) => RustSizeType::U32,
            FieldType::Primitive(PrimitiveType::Uint64) => RustSizeType::U64,
            _ => {
                return Err(Error::FieldTypeIsIncompatibleWithEnum {
                    message: message.clone(),
                    field: field.clone(),
                    r#enum: r#enum.clone(),
                    field_type: *r#type,
                })
            }
        };
        if field_size < enum_meta.size {
            return Err(Error::FieldTypeIsIncompatibleWithEnum {
                message: message.clone(),
                field: field.clone(),
                r#enum: r#enum.clone(),
                field_type: *r#type,
            });
        }
        Ok(r#enum)
    }
}

fn derive_enum_entry_values(r#enum: &Ident, entries: &[xml::Entry]) -> Result<Vec<u64>, Error> {
    let mut result = Vec::with_capacity(entries.len());

    let mut next_value = 1;

    for entry in entries {
        if let Some(value) = &entry.value {
            match parse_entry_value(value.to_owned()) {
                Ok(ok) => {
                    result.push(ok);
                    next_value = ok + 1;
                }
                Err(err) => {
                    return Err(Error::InvalidEntry {
                        err,
                        r#enum: r#enum.clone(),
                        entry: entry.name.clone(),
                    })
                }
            }
        } else {
            result.push(next_value);
            next_value += 1;
        }
    }

    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseEntryValueError {
    ParseInt(std::num::ParseIntError),
    ExponentiationOverflowed,
    UnknownFormat,
}

fn try_parse_python_exp_syntax(value: &str) -> Option<(&str, &str)> {
    let mut parts = value.split("**");

    let base = parts.next()?;
    let exponent = parts.next()?;

    if parts.next().is_none() {
        Some((base, exponent))
    } else {
        None
    }
}

fn parse_entry_value(mut value: String) -> Result<u64, ParseEntryValueError> {
    value.make_ascii_lowercase();
    if value.chars().all(|c| c.is_ascii_digit()) {
        return value.parse::<u64>().map_err(ParseEntryValueError::ParseInt);
    }

    if let Some(body) = value.strip_prefix("0x") {
        return u64::from_str_radix(body, 16).map_err(ParseEntryValueError::ParseInt);
    }
    if let Some(body) = value.strip_prefix("0b") {
        return u64::from_str_radix(body, 2).map_err(ParseEntryValueError::ParseInt);
    }

    if let Some((base, exp)) = try_parse_python_exp_syntax(&value) {
        let base: u64 = base.parse().map_err(ParseEntryValueError::ParseInt)?;
        let exp: u8 = exp.parse().map_err(ParseEntryValueError::ParseInt)?;

        return base
            .checked_pow(u32::from(exp))
            .ok_or(ParseEntryValueError::ExponentiationOverflowed);
    }

    Err(ParseEntryValueError::UnknownFormat)
}

#[cfg(test)]
mod tests {
    use hard_xml::XmlRead;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_entry_value() {
        assert_eq!(parse_entry_value("123".into()), Ok(123));
        assert_eq!(parse_entry_value("1".into()), Ok(1));
        assert_eq!(parse_entry_value("0xAA".into()), Ok(0xaa));
        assert_eq!(parse_entry_value("0b101010111".into()), Ok(0b101010111));
        assert_eq!(parse_entry_value("2**16".into()), Ok(1 << 16));
        assert_eq!(parse_entry_value("10**5".into()), Ok(100000));

        assert!(parse_entry_value(" 123".into()).is_err());
        assert!(parse_entry_value("".into()).is_err());
        assert!(parse_entry_value("2**2**2".into()).is_err());
        assert!(parse_entry_value("0xFFFFFFFFFFFFFFFFFFFFFFFFFFF".into()).is_err());
    }

    #[test]
    fn test_derive_enum_entry_values() {
        let values = derive_enum_entry_values(
            &Ident::from_str("TEST").unwrap(),
            &[
                xml::Entry::new_min("TEST_1", Some("1")),
                xml::Entry::new_min("TEST_2", Some("2")),
                xml::Entry::new_min("TEST_3", Some("3")),
                xml::Entry::new_min("TEST_4", Some("4")),
                xml::Entry::new_min("TEST_5", Some("5")),
            ],
        )
        .unwrap();
        assert_eq!(values, &[1, 2, 3, 4, 5]);

        let values = derive_enum_entry_values(
            &Ident::from_str("TEST").unwrap(),
            &[
                xml::Entry::new_min("TEST_1", Option::<String>::None),
                xml::Entry::new_min("TEST_2", Option::<String>::None),
                xml::Entry::new_min("TEST_3", Option::<String>::None),
                xml::Entry::new_min("TEST_4", Option::<String>::None),
                xml::Entry::new_min("TEST_5", Option::<String>::None),
            ],
        )
        .unwrap();
        assert_eq!(values, &[1, 2, 3, 4, 5]);

        let values = derive_enum_entry_values(
            &Ident::from_str("TEST").unwrap(),
            &[
                xml::Entry::new_min("TEST_1", Option::<String>::None),
                xml::Entry::new_min("TEST_2", Option::<String>::None),
                xml::Entry::new_min("TEST_3", Some("30")),
                xml::Entry::new_min("TEST_4", Option::<String>::None),
                xml::Entry::new_min("TEST_5", Option::<String>::None),
            ],
        )
        .unwrap();
        assert_eq!(values, &[1, 2, 30, 31, 32]);

        let values = derive_enum_entry_values(
            &Ident::from_str("TEST").unwrap(),
            &[
                xml::Entry::new_min("TEST_1", Some("100")),
                xml::Entry::new_min("TEST_2", Option::<String>::None),
                xml::Entry::new_min("TEST_3", Some("20")),
                xml::Entry::new_min("TEST_4", Option::<String>::None),
                xml::Entry::new_min("TEST_5", Option::<String>::None),
            ],
        )
        .unwrap();
        assert_eq!(values, &[100, 101, 20, 21, 22]);

        derive_enum_entry_values(
            &Ident::from_str("TEST").unwrap(),
            &[
                xml::Entry::new_min("TEST_1", Some("100")),
                xml::Entry::new_min("TEST_2", Option::<String>::None),
                xml::Entry::new_min("TEST_3", Some("20")),
                xml::Entry::new_min("TEST_4", Some("badvalue")),
                xml::Entry::new_min("TEST_5", Option::<String>::None),
            ],
        )
        .unwrap_err();
    }

    #[test]
    fn test_normalise_entries_bitmask_without_value() {
        let normaliser = Normaliser::default();

        let entries = vec![
            xml::Entry::new_min("TEST_1", Option::<String>::None),
            xml::Entry::new_min("TEST_2", Some("2")),
        ];

        let r#enum = Ident::from_str("TEST").unwrap();

        let err = normaliser
            .normalise_entries(&r#enum, true, entries)
            .unwrap_err();

        assert_eq!(
            err,
            Error::BitmaskWithoutValue {
                r#enum: r#enum.clone()
            }
        );
    }

    #[test]
    fn test_normalise_entries_bitmask_with_value() {
        let normaliser = Normaliser::default();

        let entries = vec![
            xml::Entry::new_min("TEST_1", Some("1")),
            xml::Entry::new_min("TEST_2", Some("2")),
        ];

        let r#enum = Ident::from_str("TEST").unwrap();

        let ok = normaliser
            .normalise_entries(&r#enum, true, entries)
            .unwrap();

        assert_eq!(
            ok,
            [
                Entry {
                    name: Ident::from_str("TEST_1").unwrap(),
                    description: None,
                    dev_status: None,
                    value: 1,
                },
                Entry {
                    name: Ident::from_str("TEST_2").unwrap(),
                    description: None,
                    dev_status: None,
                    value: 2,
                }
            ]
        );
    }

    #[test]
    fn test_normalise_entries_invalid_name() {
        let normaliser = Normaliser::default();

        let entries = vec![
            xml::Entry::new_min("TEST_1", Some("1")),
            xml::Entry::new_min(" ::<> ", Some("2")),
        ];

        let r#enum = Ident::from_str("TEST").unwrap();

        let err = normaliser
            .normalise_entries(&r#enum, true, entries)
            .unwrap_err();

        assert_eq!(
            err,
            Error::InvalidName {
                item: "entry",
                super_item: Some(r#enum),
                name: " ::<> ".into()
            },
        );
    }

    #[test]
    fn test_normalise_entries_item_redefinition() {
        let normaliser = Normaliser::default();

        let entries = vec![
            xml::Entry::new_min("TEST_1", Some("1")),
            xml::Entry::new_min("TEST_2", Some("2")),
            xml::Entry::new_min("TEST_1", Some("3")),
        ];

        let r#enum = Ident::from_str("TEST").unwrap();

        let err = normaliser
            .normalise_entries(&r#enum, false, entries)
            .unwrap_err();

        assert_eq!(
            err,
            Error::ItemRedefinition {
                item: "entry",
                super_item: Some(r#enum),
                name: "TEST_1".parse().unwrap(),
            },
        );
    }

    #[test]
    fn test_normalise_entries_repeated_value() {
        let normaliser = Normaliser::default();

        let entries = vec![
            xml::Entry::new_min("TEST_1", Option::<String>::None),
            xml::Entry::new_min("TEST_2", Option::<String>::None),
            xml::Entry::new_min("TEST_3", Option::<String>::None),
            xml::Entry::new_min("TEST_4", Some("2")),
            xml::Entry::new_min("TEST_5", Some("5")),
        ];

        let r#enum = Ident::from_str("TEST").unwrap();

        let err = normaliser
            .normalise_entries(&r#enum, false, entries)
            .unwrap_err();

        assert_eq!(
            err,
            Error::RepeatedEntryValue {
                r#enum,
                entry_1: "TEST_4".parse().unwrap(),
                entry_2: "TEST_2".parse().unwrap(),
                value: 2
            }
        );
    }

    #[test]
    fn test_normalise_enum() {
        let mut normaliser = Normaliser::default();

        let r#enum = xml::Enum {
            name: "TEST_ENUM".parse().unwrap(),
            bitmask: None,
            description: None,
            dev_status: Some(xml::DevStatus::Deprecated(xml::Deprecated {
                description: "Some description".into(),
                since: "2022-08".into(),
                replaced_by: "SUPERCOOLTHING".into(),
            })),
            entries: vec![
                xml::Entry::new_min("TEST_1", Option::<String>::None),
                xml::Entry::new_min("TEST_2", Option::<String>::None),
                xml::Entry::new_min("TEST_3", Option::<String>::None),
                xml::Entry::new_min("TEST_4", Some("10")),
                xml::Entry::new_min("TEST_5", Some("11")),
            ],
        };

        let ok = normaliser.normalise_enum(r#enum.clone()).unwrap();

        assert_eq!(
            ok,
            Enum {
                name: "TEST_ENUM".parse().unwrap(),
                description: None,
                dev_status: Some(DevStatus::Deprecated {
                    description: Some("Some description".into()),
                    since: "2022-08".into(),
                    replaced_by: "SUPERCOOLTHING".into(),
                }),
                bitmask: false,
                entries: vec![
                    Entry {
                        name: Ident::from_str("TEST_1").unwrap(),
                        description: None,
                        dev_status: None,
                        value: 1,
                    },
                    Entry {
                        name: Ident::from_str("TEST_2").unwrap(),
                        description: None,
                        dev_status: None,
                        value: 2,
                    },
                    Entry {
                        name: Ident::from_str("TEST_3").unwrap(),
                        description: None,
                        dev_status: None,
                        value: 3,
                    },
                    Entry {
                        name: Ident::from_str("TEST_4").unwrap(),
                        description: None,
                        dev_status: None,
                        value: 10,
                    },
                    Entry {
                        name: Ident::from_str("TEST_5").unwrap(),
                        description: None,
                        dev_status: None,
                        value: 11,
                    },
                ]
            }
        );

        let err = normaliser.normalise_enum(r#enum).unwrap_err();
        assert_eq!(
            err,
            Error::ItemRedefinition {
                item: "enum",
                super_item: None,
                name: "TEST_ENUM".parse().unwrap()
            }
        );
    }

    #[test]
    fn test_normalise_enum_empty() {
        let mut normaliser = Normaliser::default();

        let r#enum = xml::Enum {
            name: "TEST_ENUM".parse().unwrap(),
            bitmask: None,
            description: None,
            dev_status: Some(xml::DevStatus::Deprecated(xml::Deprecated {
                description: "Some description".into(),
                since: "2022-08".into(),
                replaced_by: "SUPERCOOLTHING".into(),
            })),
            entries: vec![],
        };

        let err = normaliser.normalise_enum(r#enum.clone()).unwrap_err();

        assert_eq!(
            err,
            Error::NoSubItems {
                item: "enum",
                sub_items: "entries",
                name: "TEST_ENUM".parse().unwrap()
            }
        );
    }

    #[test]
    fn test_normalise_enum_bad_name() {
        let mut normaliser = Normaliser::default();

        let r#enum = xml::Enum {
            name: "TEST_ENUM ::<> ".parse().unwrap(),
            bitmask: None,
            description: None,
            dev_status: Some(xml::DevStatus::Deprecated(xml::Deprecated {
                description: "Some description".into(),
                since: "2022-08".into(),
                replaced_by: "SUPERCOOLTHING".into(),
            })),
            entries: vec![],
        };

        let err = normaliser.normalise_enum(r#enum.clone()).unwrap_err();

        assert_eq!(
            err,
            Error::InvalidName {
                item: "enum",
                super_item: None,
                name: "TEST_ENUM ::<> ".into()
            }
        );
    }

    #[test]
    fn test_normalise_many_enums() {
        let mut normaliser = Normaliser::default();

        let enums = xml::Enums::from_str(
            r#"<?xml version="1.0"?>
            <enums>
                <enum name="FAILURE_UNIT">
                    <description>List of possible units where failures can be injected.</description>
                    <entry value="0" name="FAILURE_UNIT_SENSOR_GYRO"/>
                    <entry value="1" name="FAILURE_UNIT_SENSOR_ACCEL"/>
                </enum>
                <enum name="MAV_STORM32_CAMERA_PREARM_FLAGS" bitmask="true">
                    <description>STorM32 camera prearm check flags.</description>
                    <entry value="1" name="MAV_STORM32_CAMERA_PREARM_FLAGS_CONNECTED">
                        <description>The camera has been found and is connected.</description>
                    </entry>
                </enum>
                <enum name="FAILURE_UNIT">
                    <entry value="0" name="FAILURE_UNIT_SENSOR_GYRO"/>
                </enum>
                <enum name="OSD_PARAM_CONFIG_ERROR">
                    <description>The error type for the OSD parameter editor.</description>
                    <entry value="0" name="OSD_PARAM_SUCCESS"/>
                    <entry value="1" name="OSD_PARAM_INVALID_SCREEN"/>
                    <entry value="2" name="OSD_PARAM_INVALID_PARAMETER_INDEX"/>
                    <entry value="3" name="OSD_PARAM_INVALID_PARAMETER"/>
                </enum>
                <enum name="TRACKER_MODE">
                    <description>A mapping of antenna tracker flight modes for custom_mode field of heartbeat.</description>
                    <entry value="0" name="TRACKER_MODE_MANUAL"/>
                    <entry value="1" name="TRACKER_MODE_STOP"/>
                    <entry value="2" name="TRACKER_MODE_SCAN"/>
                    <entry value="3" name="TRACKER_MODE_SERVO_TEST"/>
                    <entry value="10" name="TRACKER_MODE_AUTO"/>
                    <entry value="2" name="TRACKER_MODE_INITIALIZING"/>
                </enum>
            </enums>
            "#,
        ).unwrap().0;

        let result = normaliser.normalise_enums(enums);

        assert_eq!(
            result,
            vec![
                Enum {
                    name: "FAILURE_UNIT".parse().unwrap(),
                    bitmask: false,
                    description: Some(
                        "List of possible units where failures can be injected.".into()
                    ),
                    dev_status: None,
                    entries: vec![
                        Entry {
                            name: "FAILURE_UNIT_SENSOR_GYRO".parse().unwrap(),
                            description: None,
                            dev_status: None,
                            value: 0,
                        },
                        Entry {
                            name: "FAILURE_UNIT_SENSOR_ACCEL".parse().unwrap(),
                            description: None,
                            dev_status: None,
                            value: 1,
                        },
                    ],
                },
                Enum {
                    name: "MAV_STORM32_CAMERA_PREARM_FLAGS".parse().unwrap(),
                    bitmask: true,
                    description: Some("STorM32 camera prearm check flags.".into()),
                    dev_status: None,
                    entries: vec![Entry {
                        name: "MAV_STORM32_CAMERA_PREARM_FLAGS_CONNECTED".parse().unwrap(),
                        description: Some("The camera has been found and is connected.".into()),
                        dev_status: None,
                        value: 1,
                    },],
                },
                Enum {
                    name: "OSD_PARAM_CONFIG_ERROR".parse().unwrap(),
                    bitmask: false,
                    description: Some("The error type for the OSD parameter editor.".into()),
                    dev_status: None,
                    entries: vec![
                        Entry {
                            name: "OSD_PARAM_SUCCESS".parse().unwrap(),
                            description: None,
                            dev_status: None,
                            value: 0,
                        },
                        Entry {
                            name: "OSD_PARAM_INVALID_SCREEN".parse().unwrap(),
                            description: None,
                            dev_status: None,
                            value: 1,
                        },
                        Entry {
                            name: "OSD_PARAM_INVALID_PARAMETER_INDEX".parse().unwrap(),
                            description: None,
                            dev_status: None,
                            value: 2,
                        },
                        Entry {
                            name: "OSD_PARAM_INVALID_PARAMETER".parse().unwrap(),
                            description: None,
                            dev_status: None,
                            value: 3,
                        },
                    ],
                },
            ]
        );

        assert_eq!(
            normaliser.errors,
            vec![
                Error::ItemRedefinition {
                    item: "enum",
                    super_item: None,
                    name: "FAILURE_UNIT".parse().unwrap(),
                },
                Error::RepeatedEntryValue {
                    r#enum: "TRACKER_MODE".parse().unwrap(),
                    entry_1: "TRACKER_MODE_INITIALIZING".parse().unwrap(),
                    entry_2: "TRACKER_MODE_SCAN".parse().unwrap(),
                    value: 2,
                },
            ]
        );
    }

    #[test]
    fn test_resolve_enum_reference_doesnt_exist() {
        let normaliser = Normaliser::default();
        let message = "SOME_MESSAGE".parse().unwrap();
        let field = "SOME_FIELD".parse().unwrap();

        let err = normaliser
            .resolve_enum_reference(
                "DOESNTEXIST".into(),
                &FieldType::Primitive(PrimitiveType::Char),
                &message,
                &field,
            )
            .unwrap_err();

        assert_eq!(
            err,
            Error::InvalidEnumReference {
                message: message.clone(),
                field: field.clone(),
                r#enum: "DOESNTEXIST".into()
            }
        );

        let err = normaliser
            .resolve_enum_reference(
                " INVALID NAME ".into(),
                &FieldType::Primitive(PrimitiveType::Char),
                &message,
                &field,
            )
            .unwrap_err();

        assert_eq!(
            err,
            Error::InvalidEnumReference {
                message: message.clone(),
                field: field.clone(),
                r#enum: " INVALID NAME ".into()
            }
        );
    }

    #[test]
    fn test_resolve_enum_reference_works_only_with_unsigned() {
        let mut normaliser = Normaliser::default();
        let message = "SOME_MESSAGE".parse().unwrap();
        let field = "SOME_FIELD".parse().unwrap();

        normaliser.enums.insert(
            "SOME_ENUM".parse().unwrap(),
            NormalisedEnum {
                size: RustSizeType::U8,
            },
        );

        let ok_types = [
            FieldType::Primitive(PrimitiveType::Uint8),
            FieldType::Primitive(PrimitiveType::Uint16),
            FieldType::Primitive(PrimitiveType::Uint32),
            FieldType::Primitive(PrimitiveType::Uint64),
        ];

        for typ in ok_types {
            let ok = normaliser
                .resolve_enum_reference("SOME_ENUM".into(), &typ, &message, &field)
                .unwrap();
            assert_eq!(*ok, "SOME_ENUM".parse::<Ident>().unwrap());
        }

        let not_ok_types = [
            FieldType::Primitive(PrimitiveType::Char),
            FieldType::Primitive(PrimitiveType::Double),
            FieldType::Primitive(PrimitiveType::Float),
            FieldType::Primitive(PrimitiveType::Uint8MavlinkVersion),
            FieldType::Primitive(PrimitiveType::Int32),
            FieldType::Primitive(PrimitiveType::Int64),
            FieldType::Primitive(PrimitiveType::Int8),
            FieldType::Array(PrimitiveType::Uint8, 1),
        ];

        for typ in not_ok_types {
            let err = normaliser
                .resolve_enum_reference("SOME_ENUM".into(), &typ, &message, &field)
                .unwrap_err();
            assert_eq!(
                err,
                Error::FieldTypeIsIncompatibleWithEnum {
                    message: message.clone(),
                    field: field.clone(),
                    r#enum: "SOME_ENUM".parse().unwrap(),
                    field_type: typ,
                }
            );
        }
    }

    #[test]
    fn test_resolve_enum_reference_size_fits() {
        let mut normaliser = Normaliser::default();
        let message = "SOME_MESSAGE".parse().unwrap();
        let field = "SOME_FIELD".parse().unwrap();

        normaliser.enums.insert(
            "SOME_ENUM".parse().unwrap(),
            NormalisedEnum {
                size: RustSizeType::U32,
            },
        );

        let ok_types = [
            FieldType::Primitive(PrimitiveType::Uint32),
            FieldType::Primitive(PrimitiveType::Uint64),
        ];

        for typ in ok_types {
            let ok = normaliser
                .resolve_enum_reference("SOME_ENUM".into(), &typ, &message, &field)
                .unwrap();
            assert_eq!(*ok, "SOME_ENUM".parse::<Ident>().unwrap());
        }

        let not_ok_types = [
            FieldType::Primitive(PrimitiveType::Uint8),
            FieldType::Primitive(PrimitiveType::Uint16),
        ];

        for typ in not_ok_types {
            let err = normaliser
                .resolve_enum_reference("SOME_ENUM".into(), &typ, &message, &field)
                .unwrap_err();
            assert_eq!(
                err,
                Error::FieldTypeIsIncompatibleWithEnum {
                    message: message.clone(),
                    field: field.clone(),
                    r#enum: "SOME_ENUM".parse().unwrap(),
                    field_type: typ,
                }
            );
        }
    }

    #[test]
    fn test_normalise_field_enum() {
        let mut normaliser = Normaliser::default();
        normaliser.enums.insert(
            "SOME_ENUM".parse().unwrap(),
            NormalisedEnum {
                size: RustSizeType::U16,
            },
        );

        let message = Ident::from_str("TEST_MSG").unwrap();
        let mut field = xml::Field::new_min("TEST_FIELD", "uint64_t");
        field.r#enum = Some("SOME_ENUM".into());

        let field = normaliser.normalise_field(&message, field).unwrap();

        assert_eq!(
            field,
            Field {
                name: "TEST_FIELD".parse().unwrap(),
                r#type: FieldType::Primitive(PrimitiveType::Uint64),
                print_format: None,
                r#enum: Some("SOME_ENUM".parse().unwrap()),
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
        );
    }

    #[test]
    fn test_normalise_field() {
        let normaliser = Normaliser::default();
        let message = Ident::from_str("TEST_MSG").unwrap();

        let field = normaliser
            .normalise_field(
                &message,
                xml::Field {
                    name: "TEST_FIELD".into(),
                    r#type: "int8_t".into(),
                    print_format: Some("%s".into()),
                    r#enum: None,
                    display: None,
                    units: Some("cats".into()),
                    increment: Some(1.0),
                    min_value: Some(-1.0),
                    max_value: Some(12.0),
                    multiplier: Some("16".into()),
                    default: Some("default".into()),
                    instance: Some(true),
                    invalid: Some("true".into()),
                    description: "Description".into(),
                },
            )
            .unwrap();

        assert_eq!(
            field,
            Field {
                name: "TEST_FIELD".parse().unwrap(),
                r#type: FieldType::Primitive(PrimitiveType::Int8),
                print_format: Some("%s".into()),
                r#enum: None,
                display: None,
                units: Some("cats".into()),
                increment: Some(1.0),
                min_value: Some(-1.0),
                max_value: Some(12.0),
                multiplier: Some("16".into()),
                default: Some("default".into()),
                instance: Some(true),
                invalid: Some("true".into()),
                description: Some("Description".into()),
            }
        )
    }

    #[test]
    fn test_normalise_field_bad_name_type() {
        let normaliser = Normaliser::default();
        let message = Ident::from_str("TEST_MSG").unwrap();

        let err = normaliser
            .normalise_field(
                &message,
                xml::Field::new_min("TEST_FIELD TURBO FISH", "int8_t"),
            )
            .unwrap_err();

        assert_eq!(
            err,
            Error::InvalidName {
                item: "field",
                super_item: Some(message.clone()),
                name: "TEST_FIELD TURBO FISH".into()
            }
        );

        let err = normaliser
            .normalise_field(&message, xml::Field::new_min("TEST_FIELD", "turbo fish"))
            .unwrap_err();

        assert_eq!(
            err,
            Error::InvalidFieldType {
                message,
                field: "TEST_FIELD".parse().unwrap(),
                r#type: "turbo fish".into()
            }
        );
    }

    #[test]
    fn test_normalise_fields() {
        let normaliser = Normaliser::default();
        let fields = vec![
            xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
            xml::Field::new_min("TEST_FIELD_2", "uint16_t"),
            xml::Field::new_min("TEST_FIELD_3", "uint32_t"),
        ];
        let extension_fields = vec![
            xml::Field::new_min("EXT_FIELD_1", "int16_t"),
            xml::Field::new_min("EXT_FIELD_2", "int16_t"),
        ];

        let message = Ident::from_str("TEST_MSG").unwrap();

        let (result, ext_result) = normaliser
            .normalise_fields(&message, fields, extension_fields)
            .unwrap();

        fn field_min(name: &str, r#type: FieldType) -> Field {
            Field {
                name: name.parse().unwrap(),
                r#type,
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

        assert_eq!(
            result,
            vec![
                field_min("TEST_FIELD_3", FieldType::Primitive(PrimitiveType::Uint32)),
                field_min("TEST_FIELD_2", FieldType::Primitive(PrimitiveType::Uint16)),
                field_min("TEST_FIELD_1", FieldType::Primitive(PrimitiveType::Uint8)),
            ]
        );

        assert_eq!(
            ext_result,
            vec![
                field_min("EXT_FIELD_1", FieldType::Primitive(PrimitiveType::Int16)),
                field_min("EXT_FIELD_2", FieldType::Primitive(PrimitiveType::Int16)),
            ]
        );
    }

    #[test]
    fn test_normalise_fields_order() {
        let normaliser = Normaliser::default();
        let fields = vec![
            xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
            xml::Field::new_min("TEST_FIELD_2", "int16_t"),
            xml::Field::new_min("TEST_FIELD_3", "uint16_t"),
            xml::Field::new_min("TEST_FIELD_4", "int32_t"),
            xml::Field::new_min("TEST_FIELD_5", "uint8_t[20]"),
            xml::Field::new_min("TEST_FIELD_6", "double"),
            xml::Field::new_min("TEST_FIELD_7", "float"),
            xml::Field::new_min("TEST_FIELD_8", "uint32_t[10]"),
            xml::Field::new_min("TEST_FIELD_9", "uint64_t"),
            xml::Field::new_min("TEST_FIELD_10", "uint32_t"),
            xml::Field::new_min("TEST_FIELD_11", "uint64_t[8]"),
        ];
        let extension_fields = vec![
            xml::Field::new_min("EXT_FIELD_1", "int8_t"),
            xml::Field::new_min("EXT_FIELD_2", "int16_t"),
            xml::Field::new_min("EXT_FIELD_3", "int64_t[10]"),
        ];

        let message = Ident::from_str("TEST_MSG").unwrap();

        let (result, ext_result) = normaliser
            .normalise_fields(&message, fields, extension_fields)
            .unwrap();

        fn field_min(name: &str, r#type: FieldType) -> Field {
            Field {
                name: name.parse().unwrap(),
                r#type,
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

        assert_eq!(
            result,
            vec![
                field_min("TEST_FIELD_6", FieldType::Primitive(PrimitiveType::Double)),
                field_min("TEST_FIELD_9", FieldType::Primitive(PrimitiveType::Uint64)),
                field_min("TEST_FIELD_11", FieldType::Array(PrimitiveType::Uint64, 8)),
                field_min("TEST_FIELD_4", FieldType::Primitive(PrimitiveType::Int32)),
                field_min("TEST_FIELD_7", FieldType::Primitive(PrimitiveType::Float)),
                field_min("TEST_FIELD_8", FieldType::Array(PrimitiveType::Uint32, 10)),
                field_min("TEST_FIELD_10", FieldType::Primitive(PrimitiveType::Uint32)),
                field_min("TEST_FIELD_2", FieldType::Primitive(PrimitiveType::Int16)),
                field_min("TEST_FIELD_3", FieldType::Primitive(PrimitiveType::Uint16)),
                field_min("TEST_FIELD_1", FieldType::Primitive(PrimitiveType::Uint8)),
                field_min("TEST_FIELD_5", FieldType::Array(PrimitiveType::Uint8, 20)),
            ]
        );

        assert_eq!(
            ext_result,
            vec![
                field_min("EXT_FIELD_1", FieldType::Primitive(PrimitiveType::Int8)),
                field_min("EXT_FIELD_2", FieldType::Primitive(PrimitiveType::Int16)),
                field_min("EXT_FIELD_3", FieldType::Array(PrimitiveType::Int64, 10)),
            ]
        );
    }

    #[test]
    fn test_normalise_field_redefinition() {
        let normaliser = Normaliser::default();
        let fields = vec![
            xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
            xml::Field::new_min("TEST_FIELD_2", "uint16_t"),
            xml::Field::new_min("TEST_FIELD_1", "uint32_t"),
        ];

        let message = Ident::from_str("TEST_MSG").unwrap();

        let err = normaliser
            .normalise_fields(&message, fields, vec![])
            .unwrap_err();

        assert_eq!(
            err,
            Error::ItemRedefinition {
                item: "field",
                super_item: Some(message),
                name: "TEST_FIELD_1".parse().unwrap(),
            }
        );
    }

    #[test]
    fn test_normalise_fields_message_too_big() {
        let normaliser = Normaliser::default();
        let fields = vec![
            xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
            xml::Field::new_min("TEST_FIELD_2", "uint16_t"),
            xml::Field::new_min("TEST_FIELD_3", "uint64_t[100]"),
        ];

        let message = Ident::from_str("TEST_MSG").unwrap();

        let err = normaliser
            .normalise_fields(&message, fields, vec![])
            .unwrap_err();

        assert_eq!(
            err,
            Error::MessageIsTooBig {
                message,
                size: 1 + 2 + 100 * 8,
                max_size: 255
            }
        );
    }

    #[test]
    fn test_normalise_message() {
        let mut normaliser = Normaliser::default();
        let message = xml::Message {
            name: "SOME_MESSAGE".into(),
            id: 1234,
            dev_status: None,
            description: Some("Description.".into()),
            fields: vec![
                xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
                xml::Field::new_min("TEST_FIELD_2", "uint16_t"),
                xml::Field::new_min("TEST_FIELD_3", "uint32_t"),
            ],
            extension_fields: vec![xml::Field::new_min("EXT_FIELD_1", "uint32_t")],
        };

        let msg = normaliser.normalise_message(message).unwrap();

        fn field_min(name: &str, r#type: FieldType) -> Field {
            Field {
                name: name.parse().unwrap(),
                r#type,
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

        assert_eq!(
            msg,
            Message {
                name: "SOME_MESSAGE".parse().unwrap(),
                id: 1234,
                dev_status: None,
                description: Some("Description.".into()),
                fields: vec![
                    field_min("TEST_FIELD_3", FieldType::Primitive(PrimitiveType::Uint32)),
                    field_min("TEST_FIELD_2", FieldType::Primitive(PrimitiveType::Uint16)),
                    field_min("TEST_FIELD_1", FieldType::Primitive(PrimitiveType::Uint8)),
                ],
                extension_fields: vec![field_min(
                    "EXT_FIELD_1",
                    FieldType::Primitive(PrimitiveType::Uint32)
                ),],
            }
        );
    }

    #[test]
    fn test_normalise_message_bad_name() {
        let mut normaliser = Normaliser::default();
        let message = xml::Message {
            name: "SOME_MESSAGE BAD NAME".into(),
            id: 1234,
            dev_status: None,
            description: Some("Description.".into()),
            fields: vec![
                xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
                xml::Field::new_min("TEST_FIELD_2", "uint16_t"),
                xml::Field::new_min("TEST_FIELD_3", "uint32_t"),
            ],
            extension_fields: vec![xml::Field::new_min("EXT_FIELD_1", "uint32_t")],
        };

        let err = normaliser.normalise_message(message).unwrap_err();
        assert_eq!(
            err,
            Error::InvalidName {
                item: "message",
                super_item: None,
                name: "SOME_MESSAGE BAD NAME".into()
            }
        );
    }

    #[test]
    fn test_normalise_message_item_redefinition() {
        let mut normaliser = Normaliser::default();

        let message = xml::Message {
            name: "SOME_MESSAGE".into(),
            id: 1234,
            dev_status: None,
            description: Some("Description.".into()),
            fields: vec![
                xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
                xml::Field::new_min("TEST_FIELD_2", "uint16_t"),
                xml::Field::new_min("TEST_FIELD_3", "uint32_t"),
            ],
            extension_fields: vec![xml::Field::new_min("EXT_FIELD_1", "uint32_t")],
        };

        normaliser.normalise_message(message.clone()).unwrap();
        let err = normaliser.normalise_message(message).unwrap_err();
        assert_eq!(
            err,
            Error::ItemRedefinition {
                item: "message",
                super_item: None,
                name: "SOME_MESSAGE".parse().unwrap()
            }
        );
    }

    #[test]
    fn test_normalise_message_repeated_id() {
        let mut normaliser = Normaliser::default();

        let mut message = xml::Message {
            name: "SOME_MESSAGE".into(),
            id: 1234,
            dev_status: None,
            description: Some("Description.".into()),
            fields: vec![
                xml::Field::new_min("TEST_FIELD_1", "uint8_t"),
                xml::Field::new_min("TEST_FIELD_2", "uint16_t"),
                xml::Field::new_min("TEST_FIELD_3", "uint32_t"),
            ],
            extension_fields: vec![xml::Field::new_min("EXT_FIELD_1", "uint32_t")],
        };

        normaliser.normalise_message(message.clone()).unwrap();

        message.name = "ANOTHER_MESSAGE".parse().unwrap();

        let err = normaliser.normalise_message(message).unwrap_err();
        assert_eq!(
            err,
            Error::RepeatedMessageId {
                msg_1: "SOME_MESSAGE".parse().unwrap(),
                msg_2: "ANOTHER_MESSAGE".parse().unwrap(),
                id: 1234
            },
        );
    }

    #[test]
    fn test_normalise_empty_message() {
        let mut normaliser = Normaliser::default();

        let message = xml::Message {
            name: "SOME_MESSAGE".into(),
            id: 1234,
            dev_status: None,
            description: Some("Description.".into()),
            fields: vec![],
            extension_fields: vec![],
        };

        let err = normaliser.normalise_message(message).unwrap_err();
        assert_eq!(
            err,
            Error::NoSubItems {
                item: "message",
                sub_items: "fields",
                name: "SOME_MESSAGE".parse().unwrap()
            },
        );
    }

    #[test]
    fn test_normalise_module() {
        let malink = xml::Mavlink::from_str(
            r#"<?xml version="1.0"?>
            <mavlink>
            <enums>
                <enum name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE" bitmask="true">
                    <description>State flags for ADS-B transponder dynamic report</description>
                    <entry value="1" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_INTENT_CHANGE"/>
                    <entry value="2" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_AUTOPILOT_ENABLED"/>
                    <entry value="4" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_NICBARO_CROSSCHECKED"/>
                    <entry value="8" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_ON_GROUND"/>
                    <entry value="16" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_IDENT"/>
                </enum>
                <enum name="UAVIONIX_ADSB_OUT_RF_SELECT" bitmask="true">
                    <description>Transceiver RF control flags for ADS-B transponder dynamic reports</description>
                    <entry value="0" name="UAVIONIX_ADSB_OUT_RF_SELECT_STANDBY"/>
                    <entry value="1" name="UAVIONIX_ADSB_OUT_RF_SELECT_RX_ENABLED"/>
                    <entry value="2" name="UAVIONIX_ADSB_OUT_RF_SELECT_TX_ENABLED"/>
                </enum>
                <enum name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX">
                    <description>Status for ADS-B transponder dynamic input</description>
                    <entry value="0" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_0"/>
                    <entry value="1" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_1"/>
                    <entry value="2" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_2D"/>
                    <entry value="3" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_3D"/>
                    <entry value="4" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_DGPS"/>
                    <entry value="5" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_RTK"/>
                </enum>
                <enum name="UAVIONIX_ADSB_RF_HEALTH" bitmask="true">
                    <description>Status flags for ADS-B transponder dynamic output</description>
                    <entry value="0" name="UAVIONIX_ADSB_RF_HEALTH_INITIALIZING"/>
                    <entry value="1" name="UAVIONIX_ADSB_RF_HEALTH_OK"/>
                    <entry value="2" name="UAVIONIX_ADSB_RF_HEALTH_FAIL_TX"/>
                    <entry value="16" name="UAVIONIX_ADSB_RF_HEALTH_FAIL_RX"/>
                </enum>
                <enum name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE">
                    <description>Definitions for aircraft size</description>
                    <entry value="0" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_NO_DATA"/>
                    <entry value="1" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L15M_W23M"/>
                    <entry value="2" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L25M_W28P5M"/>
                    <entry value="3" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L25_34M"/>
                    <entry value="4" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L35_33M"/>
                    <entry value="5" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L35_38M"/>
                    <entry value="6" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L45_39P5M"/>
                    <entry value="7" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L45_45M"/>
                    <entry value="8" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L55_45M"/>
                    <entry value="9" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L55_52M"/>
                    <entry value="10" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L65_59P5M"/>
                    <entry value="11" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L65_67M"/>
                    <entry value="12" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L75_W72P5M"/>
                    <entry value="13" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L75_W80M"/>
                    <entry value="14" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L85_W80M"/>
                    <entry value="15" name="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L85_W90M"/>
                </enum>
                <enum name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT">
                    <description>GPS lataral offset encoding</description>
                    <entry value="0" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_NO_DATA"/>
                    <entry value="1" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_LEFT_2M"/>
                    <entry value="2" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_LEFT_4M"/>
                    <entry value="3" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_LEFT_6M"/>
                    <entry value="4" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_0M"/>
                    <entry value="5" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_2M"/>
                    <entry value="6" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_4M"/>
                    <entry value="7" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_6M"/>
                </enum>
                <enum name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON">
                    <description>GPS longitudinal offset encoding</description>
                    <entry value="0" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON_NO_DATA"/>
                    <entry value="1" name="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON_APPLIED_BY_SENSOR"/>
                </enum>
                <enum name="UAVIONIX_ADSB_EMERGENCY_STATUS">
                    <description>Emergency status encoding</description>
                    <entry value="0" name="UAVIONIX_ADSB_OUT_NO_EMERGENCY"/>
                    <entry value="1" name="UAVIONIX_ADSB_OUT_GENERAL_EMERGENCY"/>
                    <entry value="2" name="UAVIONIX_ADSB_OUT_LIFEGUARD_EMERGENCY"/>
                    <entry value="3" name="UAVIONIX_ADSB_OUT_MINIMUM_FUEL_EMERGENCY"/>
                    <entry value="4" name="UAVIONIX_ADSB_OUT_NO_COMM_EMERGENCY"/>
                    <entry value="5" name="UAVIONIX_ADSB_OUT_UNLAWFUL_INTERFERANCE_EMERGENCY"/>
                    <entry value="6" name="UAVIONIX_ADSB_OUT_DOWNED_AIRCRAFT_EMERGENCY"/>
                    <entry value="7" name="UAVIONIX_ADSB_OUT_RESERVED"/>
                </enum>
            </enums>
            <messages>
                <message id="10001" name="UAVIONIX_ADSB_OUT_CFG">
                    <description>Static data to configure the ADS-B transponder (send within 10 sec of a POR and every 10 sec thereafter)</description>
                    <field type="uint32_t" name="ICAO">Vehicle address (24 bit)</field>
                    <field type="char[9]" name="callsign">Vehicle identifier (8 characters, null terminated, valid characters are A-Z, 0-9, " " only)</field>
                    <field type="uint8_t" name="emitterType">Transmitting vehicle type. See ADSB_EMITTER_TYPE enum</field>
                    <field type="uint8_t" name="aircraftSize" enum="UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE">Aircraft length and width encoding (table 2-35 of DO-282B)</field>
                    <field type="uint8_t" name="gpsOffsetLat" enum="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT">GPS antenna lateral offset (table 2-36 of DO-282B)</field>
                    <field type="uint8_t" name="gpsOffsetLon" enum="UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON">GPS antenna longitudinal offset from nose [if non-zero, take position (in meters) divide by 2 and add one] (table 2-37 DO-282B)</field>
                    <field type="uint16_t" name="stallSpeed" units="cm/s">Aircraft stall speed in cm/s</field>
                    <field type="uint8_t" name="rfSelect" enum="UAVIONIX_ADSB_OUT_RF_SELECT" display="bitmask">ADS-B transponder receiver and transmit enable flags</field>
                </message>
                <message id="10002" name="UAVIONIX_ADSB_OUT_DYNAMIC">
                    <description>Dynamic data used to generate ADS-B out transponder data (send at 5Hz)</description>
                    <field type="uint32_t" name="utcTime" units="s">UTC time in seconds since GPS epoch (Jan 6, 1980). If unknown set to UINT32_MAX</field>
                    <field type="int32_t" name="gpsLat" units="degE7">Latitude WGS84 (deg * 1E7). If unknown set to INT32_MAX</field>
                    <field type="int32_t" name="gpsLon" units="degE7">Longitude WGS84 (deg * 1E7). If unknown set to INT32_MAX</field>
                    <field type="int32_t" name="gpsAlt" units="mm">Altitude (WGS84). UP +ve. If unknown set to INT32_MAX</field>
                    <field type="uint8_t" name="gpsFix" enum="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX">0-1: no fix, 2: 2D fix, 3: 3D fix, 4: DGPS, 5: RTK</field>
                    <field type="uint8_t" name="numSats">Number of satellites visible. If unknown set to UINT8_MAX</field>
                    <field type="int32_t" name="baroAltMSL" units="mbar">Barometric pressure altitude (MSL) relative to a standard atmosphere of 1013.2 mBar and NOT bar corrected altitude (m * 1E-3). (up +ve). If unknown set to INT32_MAX</field>
                    <field type="uint32_t" name="accuracyHor" units="mm">Horizontal accuracy in mm (m * 1E-3). If unknown set to UINT32_MAX</field>
                    <field type="uint16_t" name="accuracyVert" units="cm">Vertical accuracy in cm. If unknown set to UINT16_MAX</field>
                    <field type="uint16_t" name="accuracyVel" units="mm/s">Velocity accuracy in mm/s (m * 1E-3). If unknown set to UINT16_MAX</field>
                    <field type="int16_t" name="velVert" units="cm/s">GPS vertical speed in cm/s. If unknown set to INT16_MAX</field>
                    <field type="int16_t" name="velNS" units="cm/s">North-South velocity over ground in cm/s North +ve. If unknown set to INT16_MAX</field>
                    <field type="int16_t" name="VelEW" units="cm/s">East-West velocity over ground in cm/s East +ve. If unknown set to INT16_MAX</field>
                    <field type="uint8_t" name="emergencyStatus" enum="UAVIONIX_ADSB_EMERGENCY_STATUS">Emergency status</field>
                    <field type="uint16_t" name="state" enum="UAVIONIX_ADSB_OUT_DYNAMIC_STATE" display="bitmask">ADS-B transponder dynamic input state flags</field>
                    <field type="uint16_t" name="squawk">Mode A code (typically 1200 [0x04B0] for VFR)</field>
                </message>
                <message id="10003" name="UAVIONIX_ADSB_TRANSCEIVER_HEALTH_REPORT">
                    <description>Transceiver heartbeat with health report (updated every 10s)</description>
                    <field type="uint8_t" name="rfHealth" enum="UAVIONIX_ADSB_RF_HEALTH" display="bitmask">ADS-B transponder messages</field>
                </message>
            </messages>
            </mavlink>
            "#,
        )
        .unwrap();

        let normaliser = Normaliser::default();
        let module = normaliser
            .normalise_module(flatten::MavlinkModule {
                path: "some_path.xml".into(),
                version: Some(1),
                dialect: None,
                enums: malink.enums.unwrap().0,
                messages: malink.messages.unwrap().0,
            })
            .unwrap();

        fn entry(name: &str, value: u64) -> Entry {
            Entry {
                name: name.parse().unwrap(),
                description: None,
                dev_status: None,
                value,
            }
        }

        let default_field = Field {
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
        };

        let expected = MavlinkModule {
            path: "some_path.xml".into(),
            version: Some(
                1,
            ),
            dialect: None,
            r#enums: vec![
                r#Enum {
                    name: "UAVIONIX_ADSB_OUT_DYNAMIC_STATE".parse().unwrap(),
                    bitmask: true,
                    description: Some(
                        "State flags for ADS-B transponder dynamic report".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_INTENT_CHANGE", 1),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_AUTOPILOT_ENABLED", 2),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_NICBARO_CROSSCHECKED", 4),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_ON_GROUND", 8),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_IDENT", 16),
                    ],
                },
                r#Enum {
                    name: "UAVIONIX_ADSB_OUT_RF_SELECT".parse().unwrap(),
                    bitmask: true,
                    description: Some(
                        "Transceiver RF control flags for ADS-B transponder dynamic reports".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_OUT_RF_SELECT_STANDBY", 0),
                        entry("UAVIONIX_ADSB_OUT_RF_SELECT_RX_ENABLED", 1),
                        entry("UAVIONIX_ADSB_OUT_RF_SELECT_TX_ENABLED", 2),
                    ],
                },
                r#Enum {
                    name: "UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX".parse().unwrap(),
                    bitmask: false,
                    description: Some(
                        "Status for ADS-B transponder dynamic input".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_0", 0),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_1", 1),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_2D", 2),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_3D", 3),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_DGPS", 4),
                        entry("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_RTK", 5),
                    ],
                },
                r#Enum {
                    name: "UAVIONIX_ADSB_RF_HEALTH".parse().unwrap(),
                    bitmask: true,
                    description: Some(
                        "Status flags for ADS-B transponder dynamic output".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_RF_HEALTH_INITIALIZING", 0),
                        entry("UAVIONIX_ADSB_RF_HEALTH_OK", 1),
                        entry("UAVIONIX_ADSB_RF_HEALTH_FAIL_TX", 2),
                        entry("UAVIONIX_ADSB_RF_HEALTH_FAIL_RX", 16),
                    ],
                },
                r#Enum {
                    name: "UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE".parse().unwrap(),
                    bitmask: false,
                    description: Some(
                        "Definitions for aircraft size".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_NO_DATA", 0),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L15M_W23M", 1),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L25M_W28P5M", 2),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L25_34M", 3),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L35_33M", 4),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L35_38M", 5),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L45_39P5M", 6),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L45_45M", 7),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L55_45M", 8),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L55_52M", 9),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L65_59P5M", 10),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L65_67M", 11),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L75_W72P5M", 12),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L75_W80M", 13),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L85_W80M", 14),
                        entry("UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE_L85_W90M", 15),
                    ],
                },
                r#Enum {
                    name: "UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT".parse().unwrap(),
                    bitmask: false,
                    description: Some(
                        "GPS lataral offset encoding".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_NO_DATA", 0),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_LEFT_2M", 1),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_LEFT_4M", 2),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_LEFT_6M", 3),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_0M", 4),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_2M", 5),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_4M", 6),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT_RIGHT_6M", 7),
                    ],
                },
                r#Enum {
                    name: "UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON".parse().unwrap(),
                    bitmask: false,
                    description: Some(
                        "GPS longitudinal offset encoding".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON_NO_DATA", 0),
                        entry("UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON_APPLIED_BY_SENSOR", 1),
                    ],
                },
                r#Enum {
                    name: "UAVIONIX_ADSB_EMERGENCY_STATUS".parse().unwrap(),
                    bitmask: false,
                    description: Some(
                        "Emergency status encoding".into(),
                    ),
                    dev_status: None,
                    entries: vec![
                        entry("UAVIONIX_ADSB_OUT_NO_EMERGENCY", 0),
                        entry("UAVIONIX_ADSB_OUT_GENERAL_EMERGENCY", 1),
                        entry("UAVIONIX_ADSB_OUT_LIFEGUARD_EMERGENCY", 2),
                        entry("UAVIONIX_ADSB_OUT_MINIMUM_FUEL_EMERGENCY", 3),
                        entry("UAVIONIX_ADSB_OUT_NO_COMM_EMERGENCY", 4),
                        entry("UAVIONIX_ADSB_OUT_UNLAWFUL_INTERFERANCE_EMERGENCY", 5),
                        entry("UAVIONIX_ADSB_OUT_DOWNED_AIRCRAFT_EMERGENCY", 6),
                        entry("UAVIONIX_ADSB_OUT_RESERVED", 7),
                    ],
                },
            ],
            messages: vec![
                Message {
                    name: "UAVIONIX_ADSB_OUT_CFG".parse().unwrap(),
                    id: 10001,
                    dev_status: None,
                    description: Some(
                        "Static data to configure the ADS-B transponder (send within 10 sec of a POR and every 10 sec thereafter)".into(),
                    ),
                    fields: vec![
                        Field {
                            name: "ICAO".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint32,
                            ),
                            description: Some(
                                "Vehicle address (24 bit)".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "stallSpeed".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint16,
                            ),
                            units: Some(
                                "cm/s".into(),
                            ),
                            description: Some(
                                "Aircraft stall speed in cm/s".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "callsign".parse().unwrap(),
                            r#type: FieldType::Array(
                                PrimitiveType::Char,
                                9,
                            ),
                            description: Some(
                                "Vehicle identifier (8 characters, null terminated, valid characters are A-Z, 0-9, \" \" only)".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "emitterType".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            description: Some(
                                "Transmitting vehicle type. See ADSB_EMITTER_TYPE enum".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "aircraftSize".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            r#enum: Some(
                                "UAVIONIX_ADSB_OUT_CFG_AIRCRAFT_SIZE".parse().unwrap(),
                            ),
                            description: Some(
                                "Aircraft length and width encoding (table 2-35 of DO-282B)".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "gpsOffsetLat".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            r#enum: Some(
                                "UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LAT".parse().unwrap(),
                            ),
                            description: Some(
                                "GPS antenna lateral offset (table 2-36 of DO-282B)".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "gpsOffsetLon".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            r#enum: Some(
                                "UAVIONIX_ADSB_OUT_CFG_GPS_OFFSET_LON".parse().unwrap(),
                            ),
                            description: Some(
                                "GPS antenna longitudinal offset from nose [if non-zero, take position (in meters) divide by 2 and add one] (table 2-37 DO-282B)".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "rfSelect".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            r#enum: Some(
                                "UAVIONIX_ADSB_OUT_RF_SELECT".parse().unwrap(),
                            ),
                            display: Some(
                                "bitmask".into(),
                            ),
                            description: Some(
                                "ADS-B transponder receiver and transmit enable flags".into(),
                            ),
                            ..default_field.clone()
                        },
                    ],
                    extension_fields: vec![],
                },
                Message {
                    name: "UAVIONIX_ADSB_OUT_DYNAMIC".parse().unwrap(),
                    id: 10002,
                    dev_status: None,
                    description: Some(
                        "Dynamic data used to generate ADS-B out transponder data (send at 5Hz)".into(),
                    ),
                    fields: vec![
                        Field {
                            name: "utcTime".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint32,
                            ),
                            units: Some(
                                "s".into(),
                            ),
                            description: Some(
                                "UTC time in seconds since GPS epoch (Jan 6, 1980). If unknown set to UINT32_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "gpsLat".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Int32,
                            ),
                            units: Some(
                                "degE7".into(),
                            ),
                            description: Some(
                                "Latitude WGS84 (deg * 1E7). If unknown set to INT32_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "gpsLon".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Int32,
                            ),
                            units: Some(
                                "degE7".into(),
                            ),
                            description: Some(
                                "Longitude WGS84 (deg * 1E7). If unknown set to INT32_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "gpsAlt".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Int32,
                            ),
                            units: Some(
                                "mm".into(),
                            ),
                            description: Some(
                                "Altitude (WGS84). UP +ve. If unknown set to INT32_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "baroAltMSL".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Int32,
                            ),
                            units: Some(
                                "mbar".into(),
                            ),
                            description: Some(
                                "Barometric pressure altitude (MSL) relative to a standard atmosphere of 1013.2 mBar and NOT bar corrected altitude (m * 1E-3). (up +ve). If unknown set to INT32_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "accuracyHor".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint32,
                            ),
                            units: Some(
                                "mm".into(),
                            ),
                            description: Some(
                                "Horizontal accuracy in mm (m * 1E-3). If unknown set to UINT32_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "accuracyVert".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint16,
                            ),
                            units: Some(
                                "cm".into(),
                            ),
                            description: Some(
                                "Vertical accuracy in cm. If unknown set to UINT16_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "accuracyVel".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint16,
                            ),
                            units: Some(
                                "mm/s".into(),
                            ),
                            description: Some(
                                "Velocity accuracy in mm/s (m * 1E-3). If unknown set to UINT16_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "velVert".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Int16,
                            ),
                            units: Some(
                                "cm/s".into(),
                            ),
                            description: Some(
                                "GPS vertical speed in cm/s. If unknown set to INT16_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "velNS".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Int16,
                            ),
                            units: Some(
                                "cm/s".into(),
                            ),
                            description: Some(
                                "North-South velocity over ground in cm/s North +ve. If unknown set to INT16_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "VelEW".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Int16,
                            ),
                            units: Some(
                                "cm/s".into(),
                            ),
                            description: Some(
                                "East-West velocity over ground in cm/s East +ve. If unknown set to INT16_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "state".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint16,
                            ),
                            r#enum: Some(
                                "UAVIONIX_ADSB_OUT_DYNAMIC_STATE".parse().unwrap(),
                            ),
                            display: Some(
                                "bitmask".into(),
                            ),
                            description: Some(
                                "ADS-B transponder dynamic input state flags".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "squawk".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint16,
                            ),
                            description: Some(
                                "Mode A code (typically 1200 [0x04B0] for VFR)".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "gpsFix".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            r#enum: Some(
                                "UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX".parse().unwrap(),
                            ),
                            description: Some(
                                "0-1: no fix, 2: 2D fix, 3: 3D fix, 4: DGPS, 5: RTK".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "numSats".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            description: Some(
                                "Number of satellites visible. If unknown set to UINT8_MAX".into(),
                            ),
                            ..default_field.clone()
                        },
                        Field {
                            name: "emergencyStatus".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            r#enum: Some(
                                "UAVIONIX_ADSB_EMERGENCY_STATUS".parse().unwrap(),
                            ),
                            description: Some(
                                "Emergency status".into(),
                            ),
                            ..default_field.clone()
                        },
                    ],
                    extension_fields: vec![],
                },
                Message {
                    name: "UAVIONIX_ADSB_TRANSCEIVER_HEALTH_REPORT".parse().unwrap(),
                    id: 10003,
                    dev_status: None,
                    description: Some(
                        "Transceiver heartbeat with health report (updated every 10s)".into(),
                    ),
                    fields: vec![
                        Field {
                            name: "rfHealth".parse().unwrap(),
                            r#type: FieldType::Primitive(
                                PrimitiveType::Uint8,
                            ),
                            print_format: None,
                            r#enum: Some(
                                "UAVIONIX_ADSB_RF_HEALTH".parse().unwrap(),
                            ),
                            display: Some(
                                "bitmask".into(),
                            ),
                            description: Some(
                                "ADS-B transponder messages".into(),
                            ),
                            ..default_field.clone()
                        },
                    ],
                    extension_fields: vec![],
                },
            ],
        };

        assert_eq!(module, expected);
    }
}
