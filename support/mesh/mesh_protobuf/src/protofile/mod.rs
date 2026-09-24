// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

//! Definitions for describing the format protobuf messages. These can be used
//! to generate `.proto` files that are binary compatible with the associated
//! Rust types.

mod writer;

#[cfg(feature = "std")]
pub use writer::DescriptorWriter;

use crate::DefaultEncoding;
use core::fmt::Display;
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
mod string_observation;
#[cfg(verus_keep_ghost)]
include!("type_url.spec.rs");
#[cfg(verus_keep_ghost)]
include!("type_url.proof.rs");

/// A trait for a self-describing protobuf message field.
pub trait DescribeField<T> {
    /// The type of the field.
    const FIELD_TYPE: FieldType<'static>;
    /// The type name of the field in a packed context.
    const PACKED_TYPE: Option<&'static str> = None;
}

/// A trait for a self-describing protobuf message.
///
/// This can be derived for `T` by deriving [`Protobuf`](crate::Protobuf) and
/// adding the attribute `#[mesh(package = "my.package.name")]`.
pub trait DescribeMessage<T> {
    /// The message description.
    const DESCRIPTION: MessageDescription<'static>;
}

/// A description of a message type.
#[derive(Copy, Clone)]
pub enum MessageDescription<'a> {
    /// An internally-defined type, described by the descriptor.
    Internal(&'a TopLevelDescriptor<'a>),
    /// An externally-defined type.
    External {
        /// The fully-qualified name of the message type.
        name: &'a str,
        /// The import path of the `.proto` file.
        import_path: &'a str,
    },
}

/// A type URL, used in [`ProtobufAny`](super::message::ProtobufAny) (which
/// shares an encoding with `google.protobuf.Any`).
#[derive(Debug, Copy, Clone)]
#[verus_verify(publish_source)]
#[verus_verify]
pub struct TypeUrl<'a> {
    package: &'a str,
    name: &'a str,
}

impl TypeUrl<'_> {
    #[vstd::prelude::verus_verify]
    #[verus_spec(result =>
        ensures result == self.accepts(type_url@),
    )]
    fn eq(&self, type_url: &str) -> bool {
        proof! {
            string_observation::observe_str_pattern("https://");
            string_observation::observe_str_pattern("type.googleapis.com/");
            string_observation::observe_dot_pattern();
        }
        let type_url = type_url.strip_prefix("https://").unwrap_or(type_url);
        proof! { lemma_url_shape(type_url@, self.package@, self.name@); }
        if let Some((package, name)) = type_url
            .strip_prefix("type.googleapis.com/")
            .and_then(verus_exec_expr!(
                |ty: &str| -> (result: Option<(&str, &str)>)
                    ensures string_observation::last_dot_result(ty@, result),
                { ty.rsplit_once('.') }
            ))
        {
            proof! {
                lemma_last_dot_unique(package@, name@, self.package@, self.name@);
                assert(type_url@ =~= "type.googleapis.com/"@ + package@ + seq!['.'] + name@);
                lemma_url_shape(type_url@, package@, name@);
            }
            self.package == package && self.name == name
        } else {
            false
        }
    }
}

impl Display for TypeUrl<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "type.googleapis.com/{}.{}", self.package, self.name)
    }
}

#[vstd::prelude::verus_verify]
impl PartialEq<str> for TypeUrl<'_> {
    fn eq(&self, other: &str) -> bool {
        self.eq(other)
    }
}

#[vstd::prelude::verus_verify]
impl PartialEq<TypeUrl<'_>> for str {
    fn eq(&self, other: &TypeUrl<'_>) -> bool {
        other.eq(self)
    }
}

#[cfg(test)]
mod type_url_tests {
    use super::TypeUrl;
    use test_with_tracing::test;

    #[test]
    fn type_url_string_observation_edges() {
        for (input, prefix, expected) in [
            ("", "", Some("")),
            ("abc", "", Some("abc")),
            ("", "https://", None),
            ("https://", "https://", Some("")),
            ("type.googleapis.com/", "type.googleapis.com/", Some("")),
            ("\u{03b1}\u{03b2}", "\u{03b1}", Some("\u{03b2}")),
            ("\u{03b1}", "\u{03b1}\u{03b2}", None),
            ("https://https://x", "https://", Some("https://x")),
        ] {
            assert_eq!(input.strip_prefix(prefix), expected);
        }
        for (input, expected) in [
            ("", None),
            (".", Some(("", ""))),
            ("..", Some((".", ""))),
            ("\u{03b1}.\u{1f642}", Some(("\u{03b1}", "\u{1f642}"))),
            ("\u{03b1}.\u{03b2}.", Some(("\u{03b1}.\u{03b2}", ""))),
            ("\u{03b1}\u{ff0e}\u{03b2}", None),
        ] {
            assert_eq!(input.rsplit_once('.'), expected);
        }
    }

    #[test]
    fn type_url_contents() {
        for (package, name, input, accepted) in [
            ("a.b", "C", "type.googleapis.com/a.b.C", true),
            ("a.b", "C", "https://type.googleapis.com/a.b.C", true),
            ("a.b", "C", "http://type.googleapis.com/a.b.C", false),
            ("a.b", "C", "HTTPS://type.googleapis.com/a.b.C", false),
            ("a.b", "C", "https://https://type.googleapis.com/a.b.C", false),
            ("a.b", "C", "other.googleapis.com/a.b.C", false),
            ("a.b", "C", "type.googleapis.com/A.b.C", false),
            ("a.b", "C", "type.googleapis.com/a.b.c", false),
            ("a.b", "C", "type.googleapis.com/C", false),
            ("a.b", "C", "type.googleapis.com/a.b.C.extra", false),
            ("a", "b.C", "type.googleapis.com/a.b.C", false),
            ("", "C", "type.googleapis.com/.C", true),
            ("a", "", "type.googleapis.com/a.", true),
            ("", "", "type.googleapis.com/.", true),
            ("", "", "type.googleapis.com/", false),
            ("a.", "C", "type.googleapis.com/a..C", true),
            (" a", "C/?", "type.googleapis.com/ a.C/?", true),
            ("\u{03b1}", "\u{03b2}", "type.googleapis.com/\u{03b1}.\u{03b2}", true),
            ("", "google.protobuf.Any", "type.googleapis.com/.google.protobuf.Any", false),
        ] {
            let ty = TypeUrl { package, name };
            assert_eq!(ty.eq(input), accepted, "{package:?} {name:?} {input:?}");
            assert_eq!(&ty == input, accepted, "{input:?}");
            assert_eq!(&ty != input, !accepted, "{input:?}");
            assert_eq!(input == &ty, accepted, "{input:?}");
            assert_eq!(input != &ty, !accepted, "{input:?}");
        }
    }
}

impl MessageDescription<'_> {
    /// Returns the type URL to use with `google.protobuf.Any`.
    pub const fn type_url(&self) -> TypeUrl<'_> {
        match *self {
            MessageDescription::Internal(tld) => TypeUrl {
                package: tld.package,
                name: tld.message.name,
            },
            MessageDescription::External { name, .. } => TypeUrl { package: "", name },
        }
    }
}

/// Returns the top-level message descriptor for a type with a default encoding.
pub const fn message_description<T: DefaultEncoding>() -> MessageDescription<'static>
where
    T::Encoding: DescribeMessage<T>,
{
    <T::Encoding as DescribeMessage<T>>::DESCRIPTION
}

/// The description of a field type.
#[derive(Debug, Copy, Clone)]
pub struct FieldType<'a> {
    kind: FieldKind<'a>,
    sequence_type: Option<SequenceType<'a>>,
    annotation: &'a str,
}

#[cfg_attr(not(feature = "std"), expect(dead_code))]
#[derive(Debug, Copy, Clone)]
enum SequenceType<'a> {
    Optional,
    Repeated,
    Map(&'a str),
}

#[cfg_attr(not(feature = "std"), expect(dead_code))]
#[derive(Debug, Copy, Clone)]
enum FieldKind<'a> {
    Builtin(&'a str),
    Local(&'a str),
    External {
        name: &'a str,
        import_path: &'static str,
    },
    Message(fn() -> MessageDescription<'a>),
    Tuple(&'a [FieldType<'a>]),
    KeyValue(&'a [FieldType<'a>; 2]),
}

impl<'a> FieldType<'a> {
    /// Returns a repeated version of this field type.
    ///
    /// Panics if the field type is already a sequence type.
    pub const fn repeated(mut self) -> Self {
        assert!(self.sequence_type.is_none());
        self.sequence_type = Some(SequenceType::Repeated);
        self
    }

    /// Returns a optional version of this field type.
    ///
    /// Panics if the field type is already a sequence type.
    pub const fn optional(mut self) -> Self {
        assert!(self.sequence_type.is_none());
        self.sequence_type = Some(SequenceType::Optional);
        self
    }

    /// Sets an annotation to show up in the .proto file.
    pub const fn annotate(mut self, annotation: &'a str) -> Self {
        self.annotation = annotation;
        self
    }

    /// Returns a map type.
    ///
    /// If `key` is not a builtin numeric scalar or string type, or if `value`
    /// is an optional or repeated type, then this will result in a repeated
    /// tuple instead of a protobuf `map` type. The encodings for these are the
    /// same, but `.proto` `map` types are constrained to mappings from scalars
    /// to non-optional/repeated scalars and messages.
    pub const fn map(kv: &'a [FieldType<'a>; 2]) -> Self {
        let [key, value] = kv;
        if !key.is_sequence() && !value.is_sequence() {
            if let FieldKind::Builtin(ty) = key.kind {
                if let b"uint32" | b"int32" | b"sint32" | b"uint64" | b"sint64" | b"int64"
                | b"fixed32" | b"fixed64" | b"sfixed32" | b"sfixed64" | b"bool" | b"string" =
                    ty.as_bytes()
                {
                    return Self {
                        kind: value.kind,
                        sequence_type: Some(SequenceType::Map(ty)),
                        annotation: "",
                    };
                }
            }
        }
        Self {
            kind: FieldKind::KeyValue(kv),
            sequence_type: Some(SequenceType::Repeated),
            annotation: "",
        }
    }

    /// Returns a field type for a message whose top-level descriptor is
    /// returned by `f`.
    ///
    /// This is abstracted through a function to allow for recursive types.
    /// Currently Rust does not allow a `const` to refer to a `static`, but it
    /// does allow a `const` to refer to a function that returns a `&'static`.
    pub const fn message(f: fn() -> MessageDescription<'a>) -> Self {
        Self {
            kind: FieldKind::Message(f),
            sequence_type: None,
            annotation: "",
        }
    }

    /// Returns a field type for a local message type with `name`.
    pub const fn local(name: &'a str) -> Self {
        Self {
            kind: FieldKind::Local(name),
            sequence_type: None,
            annotation: "",
        }
    }

    /// Returns a field type for a builtin type, such as `uint32`.
    pub const fn builtin(name: &'a str) -> Self {
        Self {
            kind: FieldKind::Builtin(name),
            sequence_type: None,
            annotation: "",
        }
    }

    /// Returns a field type for an anonymous tuple.
    pub const fn tuple(field_types: &'a [Self]) -> Self {
        // Use well-known types instead of new anonymous ones when possible.
        match field_types {
            [] => {
                return Self::external("google.protobuf.Empty", "google/protobuf/empty.proto");
            }
            &[
                Self {
                    kind: FieldKind::Builtin(ty),
                    sequence_type: None,
                    annotation,
                },
            ] if annotation.is_empty() => {
                let wrapper = match ty.as_bytes() {
                    b"double" => Some("google.protobuf.DoubleValue"),
                    b"float" => Some("google.protobuf.FloatValue"),
                    b"int64" => Some("google.protobuf.Int64Value"),
                    b"uint64" => Some("google.protobuf.UInt64Value"),
                    b"int32" => Some("google.protobuf.Int32Value"),
                    b"uint32" => Some("google.protobuf.UInt32Value"),
                    b"bool" => Some("google.protobuf.BoolValue"),
                    b"string" => Some("google.protobuf.StringValue"),
                    b"bytes" => Some("google.protobuf.BytesValue"),
                    _ => None,
                };
                if let Some(wrapper) = wrapper {
                    return Self::external(wrapper, "google/protobuf/wrappers.proto");
                }
            }
            _ => {}
        }
        Self {
            kind: FieldKind::Tuple(field_types),
            sequence_type: None,
            annotation: "",
        }
    }

    /// Returns a field type for an external type with the given fully-qualified
    /// name and protoc import path.
    pub const fn external(name: &'a str, import_path: &'static str) -> Self {
        Self {
            kind: FieldKind::External { name, import_path },
            sequence_type: None,
            annotation: "",
        }
    }

    /// Returns true if this is a sequence type (optional or repeated).
    pub const fn is_sequence(&self) -> bool {
        self.sequence_type.is_some()
    }

    /// Returns true if this type can use a packed encoding in a repeated
    /// context.
    pub const fn can_pack(&self) -> bool {
        if self.sequence_type.is_some() {
            return false;
        }
        match self.kind {
            FieldKind::Builtin(v) => matches!(
                v.as_bytes(),
                b"double" | b"float" | b"int64" | b"uint64" | b"int32" | b"uint32" | b"bool"
            ),
            _ => false,
        }
    }
}

/// A descriptor for a message field.
#[cfg_attr(not(feature = "std"), expect(dead_code))]
#[derive(Debug, Copy, Clone)]
pub struct FieldDescriptor<'a> {
    field_type: FieldType<'a>,
    field_number: u32,
    comment: &'a str,
    name: &'a str,
}

impl<'a> FieldDescriptor<'a> {
    /// Returns a new descriptor.
    pub const fn new(
        comment: &'a str,
        field_type: FieldType<'a>,
        name: &'a str,
        field_number: u32,
    ) -> Self {
        Self {
            field_type,
            field_number,
            comment,
            name,
        }
    }
}

/// A description of a protobuf `oneof`.
#[cfg_attr(not(feature = "std"), expect(dead_code))]
#[derive(Debug, Copy, Clone)]
pub struct OneofDescriptor<'a> {
    name: &'a str,
    variants: &'a [FieldDescriptor<'a>],
}

impl<'a> OneofDescriptor<'a> {
    /// Returns a new descriptor.
    pub const fn new(name: &'a str, variants: &'a [FieldDescriptor<'a>]) -> Self {
        Self { name, variants }
    }
}

/// A message descriptor.
#[derive(Debug, Copy, Clone)]
#[cfg_attr(not(feature = "std"), expect(dead_code))]
pub struct MessageDescriptor<'a> {
    comment: &'a str,
    name: &'a str,
    fields: &'a [FieldDescriptor<'a>],
    oneofs: &'a [OneofDescriptor<'a>],
    messages: &'a [MessageDescriptor<'a>],
}

impl<'a> MessageDescriptor<'a> {
    /// Creates a new message descriptor.
    pub const fn new(
        name: &'a str,
        comment: &'a str,
        fields: &'a [FieldDescriptor<'a>],
        oneofs: &'a [OneofDescriptor<'a>],
        messages: &'a [MessageDescriptor<'a>],
    ) -> Self {
        Self {
            comment,
            name,
            fields,
            oneofs,
            messages,
        }
    }
}

/// A message descriptor for a message rooted directly in a package (and not
/// nested in another message type).
#[derive(Debug, Copy, Clone)]
pub struct TopLevelDescriptor<'a> {
    package: &'a str,
    message: &'a MessageDescriptor<'a>,
}

impl<'a> TopLevelDescriptor<'a> {
    /// Returns a new descriptor.
    pub const fn message(package: &'a str, message: &'a MessageDescriptor<'a>) -> Self {
        Self { package, message }
    }
}
