use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use crate::model::{self, FieldType, PrimitiveType};
use naming::IdentExt;

struct PrimitiveTypeAsRust(PrimitiveType);

impl std::fmt::Display for PrimitiveTypeAsRust {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            PrimitiveType::Float => write!(f, "f32"),
            PrimitiveType::Double => write!(f, "f64"),
            PrimitiveType::Char => write!(f, "u8"),
            PrimitiveType::Int8 => write!(f, "i8"),
            PrimitiveType::Uint8 => write!(f, "u8"),
            PrimitiveType::Uint8MavlinkVersion => write!(f, "u8"),
            PrimitiveType::Int16 => write!(f, "i16"),
            PrimitiveType::Uint16 => write!(f, "u16"),
            PrimitiveType::Int32 => write!(f, "i32"),
            PrimitiveType::Uint32 => write!(f, "u32"),
            PrimitiveType::Int64 => write!(f, "i64"),
            PrimitiveType::Uint64 => write!(f, "u64"),
        }
    }
}

fn field_type_as_rust(field_type: model::FieldType) -> TokenStream {
    match field_type {
        FieldType::Primitive(typ) => {
            let typ = format_ident!("{}", PrimitiveTypeAsRust(typ).to_string());
            quote! { #typ }
        }
        FieldType::Array(typ, size) => {
            let typ = format_ident!("{}", PrimitiveTypeAsRust(typ).to_string());
            let size = usize::from(size);
            quote! { [#typ; #size] }
        }
    }
}

fn field_type_default_value(field_type: FieldType) -> TokenStream {
    fn default_value(typ: PrimitiveType) -> TokenStream {
        match typ {
            PrimitiveType::Float | PrimitiveType::Double => quote!(0.0),
            PrimitiveType::Char
            | PrimitiveType::Int8
            | PrimitiveType::Uint8
            | PrimitiveType::Uint8MavlinkVersion
            | PrimitiveType::Int16
            | PrimitiveType::Uint16
            | PrimitiveType::Int32
            | PrimitiveType::Uint32
            | PrimitiveType::Int64
            | PrimitiveType::Uint64 => quote!(0),
        }
    }

    match field_type {
        FieldType::Primitive(typ) => default_value(typ),
        FieldType::Array(typ, size) => {
            let value = default_value(typ);
            let size = usize::from(size);
            quote!([#value; #size])
        }
    }
}

fn rust_size_type(typ: model::RustSizeType) -> syn::Ident {
    let literal = match typ {
        model::RustSizeType::U8 => "u8",
        model::RustSizeType::U16 => "u16",
        model::RustSizeType::U32 => "u32",
        model::RustSizeType::U64 => "u64",
    };

    syn::Ident::new(literal, proc_macro2::Span::call_site())
}

#[derive(Debug, Default)]
pub struct Codegen {}

impl Codegen {
    pub fn emit_module(&self, module: &model::MavlinkModule) -> TokenStream {
        let mut stream = self.emit_prelude(module);

        for r#enum in &module.enums {
            stream.extend(self.emit_enum(r#enum));
        }

        for message in &module.messages {
            stream.extend(self.emit_message(message));
        }

        stream.extend(self.emit_mav_message(module));

        stream
    }

    fn emit_prelude(&self, _module: &model::MavlinkModule) -> TokenStream {
        quote! {
            #![doc = "This file was automatically generated, do not edit"]

            #![allow(
                clippy::field_reassign_with_default,
                clippy::unnecessary_cast,
                clippy::unnecessary_fallible_conversions,
                clippy::useless_conversion,
                deprecated,
            )]

            #[allow(unused_imports)]
            use bitflags::bitflags;
            #[allow(unused_imports)]
            use mavlink_core::{
                bytes::Bytes, bytes_mut::BytesMut, error::ParserError, MavlinkVersion, Message,
                MessageData,
            };
            #[allow(unused_imports)]
            use num_derive::FromPrimitive;
            #[allow(unused_imports)]
            use num_derive::ToPrimitive;
            #[allow(unused_imports)]
            use num_traits::FromPrimitive;
            #[allow(unused_imports)]
            use num_traits::ToPrimitive;
        }
    }

    pub fn emit_enum(&self, r#enum: &model::Enum) -> TokenStream {
        let mut stream = if r#enum.bitmask {
            self.emit_bitmask_enum(r#enum)
        } else {
            self.emit_regular_enum(r#enum)
        };

        stream.extend(self.emit_enum_default_impl(r#enum));
        stream.extend(self.emit_enum_converters(r#enum));

        stream
    }

    fn emit_doc(
        &self,
        description: Option<&str>,
        dev_status: Option<&model::DevStatus>,
    ) -> TokenStream {
        use std::fmt::Write;
        let mut desc = String::new();

        if let Some(model::DevStatus::Wip { since, description }) = dev_status {
            desc.push_str("WIP");
            if let Some(since) = since {
                write!(desc, " since {}", since).expect("string is ok");
            }
            if let Some(description) = description {
                write!(desc, " - {}", description.trim()).expect("string is ok");
            }
            desc.push_str("\n\n");
        }

        if let Some(description) = description {
            desc.push_str(description.trim());
        }

        let mut stream = TokenStream::new();

        if !desc.is_empty() {
            let desc = desc.replace('\t', "    ");
            let doc = desc.trim();
            stream.extend(quote! { #[doc = #doc] });
        }

        if let Some(model::DevStatus::Deprecated {
            since,
            replaced_by,
            description,
        }) = dev_status
        {
            let mut note = format!("Since {since}, replaced by {replaced_by}");
            if let Some(description) = description {
                note.push_str(". ");
                note.push_str(description.replace('\t', "    ").trim());
            }

            stream.extend(quote! { #[deprecated(note = #note)] });
        }

        stream
    }

    fn emit_regular_enum(&self, r#enum: &model::Enum) -> TokenStream {
        let mut stream = self.emit_doc(r#enum.description.as_deref(), r#enum.dev_status.as_ref());

        let typ = rust_size_type(r#enum.min_rust_size());
        let name = r#enum.name.pascal_case();

        let entries: TokenStream = r#enum
            .entries
            .iter()
            .map(|entry| {
                let mut stream =
                    self.emit_doc(entry.description.as_deref(), entry.dev_status.as_ref());
                let name = entry.name.pascal_case();
                let value = Literal::u64_unsuffixed(entry.value);
                stream.extend(quote! {
                    #name = #value,
                });
                stream
            })
            .collect();

        stream.extend(quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
            #[repr(#typ)]
            pub enum #name {
                #entries
            }
        });

        stream
    }

    fn emit_bitmask_enum(&self, r#enum: &model::Enum) -> TokenStream {
        let name = r#enum.name.pascal_case();
        let size = rust_size_type(r#enum.min_rust_size());

        let entries: TokenStream = r#enum
            .entries
            .iter()
            .map(|entry| {
                let mut stream =
                    self.emit_doc(entry.description.as_deref(), entry.dev_status.as_ref());
                let name = entry.name.pascal_case();
                let value = Literal::u64_unsuffixed(entry.value);
                stream.extend(quote! {
                    const #name = #value;
                });
                stream
            })
            .collect();

        let mut stream = self.emit_doc(r#enum.description.as_deref(), r#enum.dev_status.as_ref());

        stream.extend(quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct #name: #size {
                #entries
            }
        });

        quote! {
            bitflags! {
                #stream
            }
        }
    }

    fn emit_enum_default_impl(&self, r#enum: &model::Enum) -> TokenStream {
        let name = r#enum.name.pascal_case();
        let default_entry = r#enum.entries[0].name.pascal_case();

        quote! {
            impl #name {
                pub const DEFAULT: Self = Self::#default_entry;
            }

            impl Default for #name {
                fn default() -> Self {
                    Self::DEFAULT
                }
            }
        }
    }

    fn emit_enum_converters(&self, r#enum: &model::Enum) -> TokenStream {
        let name = r#enum.name.pascal_case();
        let typ = rust_size_type(r#enum.min_rust_size());
        let raw_name = r#enum.name.as_ref();

        let methods = if r#enum.bitmask {
            quote! {
                #[allow(unused)]
                fn try_from_bits(value: #typ) -> Result<Self, ParserError> {
                    Self::from_bits(value).ok_or(ParserError::InvalidFlag {
                        flag_type: #raw_name,
                        value: value as u32,
                    })
                }
            }
        } else {
            let from_typ = format_ident!("from_{typ}");

            quote! {
                #[allow(unused)]
                fn try_from_bits(value: #typ) -> Result<Self, ParserError> {
                    FromPrimitive::#from_typ(value).ok_or(ParserError::InvalidEnum {
                        enum_type: #raw_name,
                        value: value as u32,
                    })
                }

                pub fn bits(self) -> #typ {
                    self as _
                }
            }
        };

        quote! {
            impl #name {
                #methods
            }
        }
    }

    fn emit_message(&self, message: &model::Message) -> TokenStream {
        let mut stream = self.emit_message_def(message);
        stream.extend(self.emit_message_default_impl(message));
        stream.extend(self.emit_message_message_data_impl(message));
        stream
    }

    fn emit_message_def(&self, message: &model::Message) -> TokenStream {
        let mut stream = self.emit_doc(message.description.as_deref(), message.dev_status.as_ref());

        if can_derive_eq(message) {
            stream.extend(quote! { #[derive(Debug, Clone, Copy, PartialEq, Eq)] });
        } else {
            stream.extend(quote! { #[derive(Debug, Clone, Copy, PartialEq)] });
        }

        let fields = message.fields.iter().chain(&message.extension_fields);
        let defs = fields.map(|field| {
            let mut stream = self.emit_doc(field.description.as_deref(), None);

            let name = field.name.snake_case();

            let def = match (&field.r#enum, field.r#type) {
                (Some(r#enum), FieldType::Array(_, size)) => {
                    let typ = r#enum.pascal_case();
                    let size = usize::from(size);
                    quote! { pub #name: [#typ; #size] }
                }
                (Some(r#enum), FieldType::Primitive(_)) => {
                    let typ = r#enum.pascal_case();
                    quote! { pub #name: #typ }
                }
                (None, typ) => {
                    let typ = field_type_as_rust(typ);
                    quote! { pub #name: #typ }
                }
            };

            stream.extend(def);
            stream
        });

        let name = message.name.pascal_case();

        stream.extend(quote! {
            pub struct #name {
                #(#defs),*
            }
        });

        stream
    }

    fn emit_message_default_impl(&self, message: &model::Message) -> TokenStream {
        let fields = message
            .fields
            .iter()
            .chain(message.extension_fields.iter())
            .map(|field| {
                let name = field.name.snake_case();

                match (field.r#type, &field.r#enum) {
                    (FieldType::Array(_, size), Some(r#enum)) => {
                        let enm = r#enum.pascal_case();
                        let size = usize::from(size);
                        quote! { #name: [#enm::DEFAULT; #size] }
                    }
                    (FieldType::Primitive(_), Some(r#enum)) => {
                        let enm = r#enum.pascal_case();
                        quote! { #name: #enm::DEFAULT }
                    }
                    (typ, None) => {
                        let val = field_type_default_value(typ);
                        quote! { #name: #val }
                    }
                }
            });

        let name = message.name.pascal_case();

        quote! {
            impl #name {
                pub const DEFAULT: Self = Self {
                    #(#fields),*
                };
            }

            impl Default for #name {
                fn default() -> Self {
                    Self::DEFAULT
                }
            }
        }
    }

    fn emit_message_message_data_impl(&self, message: &model::Message) -> TokenStream {
        let name = message.name.pascal_case();
        let id = message.id;
        let original_name = message.name.as_ref();
        let extra_crc = message.extra_crc();
        let encoded_len = message.wire_size();

        let serialiser = self.emit_message_serialise_impl(message);
        let deserialsier = self.emit_message_deserialise_impl(message);

        quote! {
            impl MessageData for #name {
                type Message = MavMessage;
                const ID: u32 = #id;
                const NAME: &'static str = #original_name;
                const EXTRA_CRC: u8 = #extra_crc;
                const ENCODED_LEN: usize = #encoded_len;

                #serialiser
                #deserialsier
            }
        }
    }

    fn emit_message_serialise_impl(&self, message: &model::Message) -> TokenStream {
        let fields = message.fields.iter().chain(&message.extension_fields).map(
            |field| -> TokenStream {
                let name = field.name.snake_case();

                match (field.r#type, &field.r#enum) {
                    (FieldType::Primitive(typ), Some(_)) => {
                        let serialiser = primitive_type_to_serialiser(typ);
                        quote! {
                            __cursor.#serialiser(self.#name.bits().try_into().expect("checked"));
                        }
                    }
                    (FieldType::Primitive(typ), None) => {
                        let serialiser = primitive_type_to_serialiser(typ);
                        quote! {
                            __cursor.#serialiser(self.#name);
                        }
                    }
                    (FieldType::Array(PrimitiveType::Uint8 | PrimitiveType::Char, _), None) => {
                        quote! {
                            __cursor.put_slice(&self.#name);
                        }
                    }
                    (FieldType::Array(typ, size), Some(_)) => {
                        let serialiser = primitive_type_to_serialiser(typ);
                        let size = usize::from(size);
                        quote! {
                            for i in 0..#size {
                                __cursor.#serialiser(self.#name[i].bits().try_into().expect("checked"));
                            }
                        }
                    }
                    (FieldType::Array(typ, size), None) => {
                        let serialiser = primitive_type_to_serialiser(typ);
                        let size = usize::from(size);
                        quote! {
                            for i in 0..#size {
                                __cursor.#serialiser(self.#name[i]);
                            }
                        }
                    }
                }
            },
        );

        quote! {
            fn ser(&self, version: MavlinkVersion, bytes: &mut [u8]) -> usize {
                let mut __cursor = BytesMut::new(bytes);
                if __cursor.remaining() < Self::ENCODED_LEN {
                    panic!(
                        "buffer is too small (need {} bytes, but got {})",
                        Self::ENCODED_LEN,
                        __cursor.remaining(),
                    );
                }

                #(#fields)*

                if matches!(version, MavlinkVersion::V2) {
                    let len = __cursor.len();
                    ::mavlink_core::utils::remove_trailing_zeroes(&bytes[..len])
                } else {
                    __cursor.len()
                }
            }
        }
    }

    fn emit_message_deserialise_impl(&self, message: &model::Message) -> TokenStream {
        let fields = message
            .fields
            .iter()
            // TODO: handle extensions fields for v1
            .chain(&message.extension_fields)
            .map(|field| {
                let name = field.name.snake_case();

                match (field.r#type, &field.r#enum) {
                    (FieldType::Primitive(typ), Some(r#enum)) => {
                        let enum_name = r#enum.pascal_case();
                        let deserialiser = primitive_type_to_deserialiser(typ);

                        quote! {
                            #name: #enum_name::try_from_bits(
                                __cursor.#deserialiser().try_into().expect("checked")
                            )?
                        }
                    }
                    (FieldType::Primitive(typ), None) => {
                        let deserialiser = primitive_type_to_deserialiser(typ);
                        quote! { #name: __cursor.#deserialiser() }
                    }
                    (FieldType::Array(PrimitiveType::Uint8 | PrimitiveType::Char, _), None) => {
                        quote! { #name: __cursor.get_array() }
                    }
                    (FieldType::Array(typ, size), Some(r#enum)) => {
                        let enum_name = r#enum.pascal_case();
                        let deserialiser = primitive_type_to_deserialiser(typ);
                        let array = (0..usize::from(size)).map(|_| {
                            quote! {
                                #enum_name::try_from_bits(
                                    __cursor.#deserialiser().try_into().expect("checked")
                                )?
                            }
                        });

                        quote! {
                            #name: [ #(#array),*, ]
                        }
                    }
                    (FieldType::Array(typ, size), None) => {
                        let deserialiser = primitive_type_to_deserialiser(typ);
                        let array = (0..usize::from(size)).map(|_| {
                            quote! {
                                __cursor.#deserialiser()
                            }
                        });

                        quote! {
                            #name: [ #(#array),*, ]
                        }
                    }
                }
            });

        quote! {
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
                    #(#fields),*
                })
            }
        }
    }

    fn emit_mav_message(&self, module: &model::MavlinkModule) -> TokenStream {
        let mut stream = self.emit_mav_message_def(&module.messages);
        stream.extend(self.emit_mav_message_impl(&module.messages));
        stream
    }

    fn emit_mav_message_def(&self, messages: &[model::Message]) -> TokenStream {
        let entries = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! { #name(#name) }
        });

        quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub enum MavMessage {
                #(#entries),*,
            }
        }
    }

    fn emit_mav_message_impl(&self, messages: &[model::Message]) -> TokenStream {
        let ser = self.emit_mav_message_ser(messages);
        let parse = self.emit_mav_message_parse(messages);
        let name = self.emit_mav_message_name(messages);
        let id = self.emit_mav_message_id(messages);
        let id_from_name = self.emit_mav_message_id_from_name(messages);
        let default_from_id = self.emit_mav_message_default_message_from_id(messages);
        let extra_crc = self.emit_mav_message_extra_crc(messages);

        quote! {
            impl Message for MavMessage {
                #ser
                #parse
                #name
                #id
                #id_from_name
                #default_from_id
                #extra_crc
            }
        }
    }

    fn emit_mav_message_ser(&self, messages: &[model::Message]) -> TokenStream {
        let messages = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! {
                Self::#name(body) => body.ser(version, bytes)
            }
        });

        quote! {
            fn ser(&self, version: MavlinkVersion, bytes: &mut [u8]) -> usize {
                match self {
                    #(#messages),*,
                }
            }
        }
    }

    fn emit_mav_message_parse(&self, messages: &[model::Message]) -> TokenStream {
        let messages = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! {
                #name::ID => {
                    #name::deser(version, payload).map(Self::#name)
                }
            }
        });

        quote! {
            fn parse(
                version: MavlinkVersion,
                id: u32,
                payload: &[u8],
            ) -> Result<Self, ::mavlink_core::error::ParserError> {
                match id {
                    #(#messages),*,

                    id => Err(::mavlink_core::error::ParserError::UnknownMessage { id })
                }
            }
        }
    }

    fn emit_mav_message_name(&self, messages: &[model::Message]) -> TokenStream {
        let messages = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! {
                Self::#name(..) => #name::NAME
            }
        });

        quote! {
            fn message_name(&self) -> &'static str {
                match self {
                    #(#messages),*,
                }
            }
        }
    }

    fn emit_mav_message_id(&self, messages: &[model::Message]) -> TokenStream {
        let messages = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! {
                Self::#name(..) => #name::ID
            }
        });

        quote! {
            fn message_id(&self) -> u32 {
                match self {
                    #(#messages),*,
                }
            }
        }
    }

    fn emit_mav_message_id_from_name(&self, messages: &[model::Message]) -> TokenStream {
        let messages = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! {
                #name::NAME => Ok(#name::ID)
            }
        });

        quote! {
            fn message_id_from_name(name: &str) -> Result<u32, &'static str> {
                match name {
                    #(#messages),*,
                    _ => Err("Invalid message name."),
                }
            }
        }
    }

    fn emit_mav_message_default_message_from_id(&self, messages: &[model::Message]) -> TokenStream {
        let messages = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! {
                #name::ID => Ok(Self::#name(#name::default()))
            }
        });

        quote! {
            fn default_message_from_id(id: u32) -> Result<Self, &'static str> {
                match id {
                    #(#messages),*,
                    _ => Err("Invalid message id."),
                }
            }
        }
    }

    fn emit_mav_message_extra_crc(&self, messages: &[model::Message]) -> TokenStream {
        let messages = messages.iter().map(|message| {
            let name = message.name.pascal_case();
            quote! {
                #name::ID => #name::EXTRA_CRC
            }
        });

        quote! {
            fn extra_crc(id: u32) -> u8 {
                match id {
                    #(#messages),*,
                    _ => 0,
                }
            }
        }
    }
}

fn primitive_type_to_serialiser(typ: PrimitiveType) -> syn::Ident {
    let func = match typ {
        PrimitiveType::Float => "put_f32_le",
        PrimitiveType::Double => "put_f64_le",
        PrimitiveType::Uint8 | PrimitiveType::Uint8MavlinkVersion | PrimitiveType::Char => "put_u8",
        PrimitiveType::Int8 => "put_i8",
        PrimitiveType::Int16 => "put_i16_le",
        PrimitiveType::Uint16 => "put_u16_le",
        PrimitiveType::Int32 => "put_i32_le",
        PrimitiveType::Uint32 => "put_u32_le",
        PrimitiveType::Int64 => "put_i64_le",
        PrimitiveType::Uint64 => "put_u64_le",
    };

    syn::Ident::new(func, proc_macro2::Span::call_site())
}

fn primitive_type_to_deserialiser(typ: PrimitiveType) -> syn::Ident {
    let func = match typ {
        PrimitiveType::Float => "get_f32_le",
        PrimitiveType::Double => "get_f64_le",
        PrimitiveType::Uint8 | PrimitiveType::Uint8MavlinkVersion | PrimitiveType::Char => "get_u8",
        PrimitiveType::Int8 => "get_i8",
        PrimitiveType::Int16 => "get_i16_le",
        PrimitiveType::Uint16 => "get_u16_le",
        PrimitiveType::Int32 => "get_i32_le",
        PrimitiveType::Uint32 => "get_u32_le",
        PrimitiveType::Int64 => "get_i64_le",
        PrimitiveType::Uint64 => "get_u64_le",
    };
    syn::Ident::new(func, proc_macro2::Span::call_site())
}

fn can_derive_eq(message: &model::Message) -> bool {
    let has_floats = message
        .fields
        .iter()
        .chain(message.extension_fields.iter())
        .any(|field| {
            matches!(
                field.r#type.primitive_type(),
                model::PrimitiveType::Float | model::PrimitiveType::Double
            )
        });

    !has_floats
}

mod naming;
#[cfg(test)]
mod tests;
