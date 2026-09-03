//! The declaration macro behind the typed messages and its small type-mapping helpers.
//!
//! `message!` turns one field-list declaration into a struct, its on-wire serialization,
//! and the metadata (`ID`, `CRC_EXTRA`, base-field descriptors) that the rest of the crate
//! and the tests rely on, so a new message is a few lines rather than a hand-written codec.
//! Fields are declared in wire order (largest type first, as MAVLink reorders them); the
//! `CRC_EXTRA` test re-derives the seed from that order and catches a mistake.

// The Rust storage type for a MAVLink scalar type token. A `char` is stored as a byte.
macro_rules! mav_scalar_ty {
    (u8) => {
        u8
    };
    (i8) => {
        i8
    };
    (char) => {
        u8
    };
    (u16) => {
        u16
    };
    (i16) => {
        i16
    };
    (u32) => {
        u32
    };
    (i32) => {
        i32
    };
    (f32) => {
        f32
    };
    (u64) => {
        u64
    };
    (i64) => {
        i64
    };
    (f64) => {
        f64
    };
}

// The size in bytes of a MAVLink scalar type token.
macro_rules! mav_scalar_size {
    (u8) => {
        1
    };
    (i8) => {
        1
    };
    (char) => {
        1
    };
    (u16) => {
        2
    };
    (i16) => {
        2
    };
    (u32) => {
        4
    };
    (i32) => {
        4
    };
    (f32) => {
        4
    };
    (u64) => {
        8
    };
    (i64) => {
        8
    };
    (f64) => {
        8
    };
}

// The MAVLink C type name for a scalar type token, as it appears in the CRC_EXTRA string.
macro_rules! mav_scalar_str {
    (u8) => {
        "uint8_t"
    };
    (i8) => {
        "int8_t"
    };
    (char) => {
        "char"
    };
    (u16) => {
        "uint16_t"
    };
    (i16) => {
        "int16_t"
    };
    (u32) => {
        "uint32_t"
    };
    (i32) => {
        "int32_t"
    };
    (f32) => {
        "float"
    };
    (u64) => {
        "uint64_t"
    };
    (i64) => {
        "int64_t"
    };
    (f64) => {
        "double"
    };
}

// A field's Rust type: an array for `[k; n]`, otherwise the scalar's storage type.
macro_rules! mav_field_ty {
    ([ $k:tt ; $n:literal ]) => {
        [mav_scalar_ty!($k); $n]
    };
    ($k:tt) => {
        mav_scalar_ty!($k)
    };
}

// A field's total size in bytes.
macro_rules! mav_field_size {
    ([ $k:tt ; $n:literal ]) => {
        mav_scalar_size!($k) * $n
    };
    ($k:tt) => {
        mav_scalar_size!($k)
    };
}

// The `FieldType` for a MAVLink scalar type token, the runtime counterpart of the type
// name above.
macro_rules! mav_scalar_kind {
    (u8) => {
        $crate::dialect::FieldType::U8
    };
    (i8) => {
        $crate::dialect::FieldType::I8
    };
    (char) => {
        $crate::dialect::FieldType::Char
    };
    (u16) => {
        $crate::dialect::FieldType::U16
    };
    (i16) => {
        $crate::dialect::FieldType::I16
    };
    (u32) => {
        $crate::dialect::FieldType::U32
    };
    (i32) => {
        $crate::dialect::FieldType::I32
    };
    (f32) => {
        $crate::dialect::FieldType::F32
    };
    (u64) => {
        $crate::dialect::FieldType::U64
    };
    (i64) => {
        $crate::dialect::FieldType::I64
    };
    (f64) => {
        $crate::dialect::FieldType::F64
    };
}

// A field's `FieldType` (the element type for an array).
macro_rules! mav_field_kind {
    ([ $k:tt ; $n:literal ]) => {
        mav_scalar_kind!($k)
    };
    ($k:tt) => {
        mav_scalar_kind!($k)
    };
}

// A field's MAVLink C type name (the element type for an array).
macro_rules! mav_field_str {
    ([ $k:tt ; $n:literal ]) => {
        mav_scalar_str!($k)
    };
    ($k:tt) => {
        mav_scalar_str!($k)
    };
}

// A field's array length, or 0 for a scalar, as the CRC_EXTRA derivation expects.
macro_rules! mav_field_arraylen {
    ([ $k:tt ; $n:literal ]) => {
        $n
    };
    ($k:tt) => {
        0usize
    };
}

// Writes one field into `out` at `off`, advancing `off`.
macro_rules! mav_encode_field {
    ($out:ident, $off:ident, $value:expr, [ $k:tt ; $n:literal ]) => {{
        let mut __i = 0usize;
        while __i < $n {
            let __bytes = $value[__i].to_le_bytes();
            $out[$off..$off + __bytes.len()].copy_from_slice(&__bytes);
            $off += __bytes.len();
            __i += 1;
        }
    }};
    ($out:ident, $off:ident, $value:expr, $k:tt) => {{
        let __bytes = $value.to_le_bytes();
        $out[$off..$off + __bytes.len()].copy_from_slice(&__bytes);
        $off += __bytes.len();
    }};
}

// Reads one field from `buf` at `off`, advancing `off`, and evaluates to its value.
macro_rules! mav_decode_field {
    ($buf:ident, $off:ident, [ $k:tt ; $n:literal ]) => {{
        type __Elem = mav_scalar_ty!($k);
        const __SZ: usize = mav_scalar_size!($k);
        let mut __arr: [__Elem; $n] = [Default::default(); $n];
        let mut __i = 0usize;
        while __i < $n {
            let __bytes: [u8; __SZ] = (&$buf[$off..$off + __SZ]).try_into().unwrap();
            __arr[__i] = <__Elem>::from_le_bytes(__bytes);
            $off += __SZ;
            __i += 1;
        }
        __arr
    }};
    ($buf:ident, $off:ident, $k:tt) => {{
        type __Scalar = mav_scalar_ty!($k);
        const __SZ: usize = mav_scalar_size!($k);
        let __bytes: [u8; __SZ] = (&$buf[$off..$off + __SZ]).try_into().unwrap();
        $off += __SZ;
        <__Scalar>::from_le_bytes(__bytes)
    }};
}

/// Declares a typed MAVLink message from its name, id, `CRC_EXTRA`, and wire-ordered base
/// fields, generating the struct, its serialization, and its [`Message`](crate::dialect::Message)
/// implementation.
///
/// An optional `; ext { .. }` group after the base fields declares MAVLink 2 extension
/// fields. Extensions are appended after the base fields, in the order written (they are not
/// size-reordered), and are excluded from [`CRC_EXTRA`](crate::dialect::Message::CRC_EXTRA), so
/// adding them to a message never changes its seed. A frame that carries only the base fields
/// still decodes, with the extensions read as zero.
macro_rules! message {
    (
        $(#[$meta:meta])*
        $struct:ident = $id:literal, crc = $crc:literal, name = $wire:literal;
        $( $field:ident : $fty:tt ),+ $(,)?
        $( ; ext { $( $efield:ident : $efty:tt ),+ $(,)? } )?
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq)]
        #[allow(missing_docs)]
        pub struct $struct {
            $( pub $field: mav_field_ty!($fty), )+
            $( $( pub $efield: mav_field_ty!($efty), )+ )?
        }

        impl $crate::dialect::Message for $struct {
            const ID: u32 = $id;
            const NAME: &'static str = $wire;
            const CRC_EXTRA: u8 = $crc;
            const WIRE_LEN: usize =
                0 $( + mav_field_size!($fty) )+ $( $( + mav_field_size!($efty) )+ )?;
            const BASE_FIELDS: &'static [(&'static str, &'static str, u8)] = &[
                $(
                    (
                        mav_field_str!($fty),
                        $crate::dialect::xml_name(stringify!($field)),
                        mav_field_arraylen!($fty) as u8,
                    ),
                )+
            ];

            const DESCRIPTOR: &'static $crate::dialect::MessageDescriptor<'static> =
                &$crate::dialect::MessageDescriptor {
                    id: $id,
                    name: $wire,
                    crc_extra: $crc,
                    fields: &[
                        $(
                            $crate::dialect::FieldDescriptor {
                                name: $crate::dialect::xml_name(stringify!($field)),
                                ty: mav_field_kind!($fty),
                                array_len: mav_field_arraylen!($fty) as u8,
                                extension: false,
                            },
                        )+
                        $($(
                            $crate::dialect::FieldDescriptor {
                                name: $crate::dialect::xml_name(stringify!($efield)),
                                ty: mav_field_kind!($efty),
                                array_len: mav_field_arraylen!($efty) as u8,
                                extension: true,
                            },
                        )+)?
                    ],
                };

            fn encode(&self, out: &mut [u8]) -> usize {
                let mut off = 0usize;
                $( mav_encode_field!(out, off, self.$field, $fty); )+
                $( $( mav_encode_field!(out, off, self.$efield, $efty); )+ )?
                off
            }

            fn decode(payload: &[u8]) -> $crate::Result<Self> {
                let mut buf = [0u8; Self::WIRE_LEN];
                let n = core::cmp::min(payload.len(), Self::WIRE_LEN);
                buf[..n].copy_from_slice(&payload[..n]);
                let mut off = 0usize;
                $( let $field = mav_decode_field!(buf, off, $fty); )+
                $( $( let $efield = mav_decode_field!(buf, off, $efty); )+ )?
                let _ = off;
                Ok($struct { $( $field, )+ $( $( $efield, )+ )? })
            }
        }
    };
}
