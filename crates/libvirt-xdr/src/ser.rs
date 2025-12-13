//! XDR Serializer implementation.

use crate::error::{Error, Result};
use serde::{ser, Serialize};

/// XDR Serializer.
pub struct XdrSerializer {
    output: Vec<u8>,
}

impl XdrSerializer {
    /// Create a new XDR serializer.
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }

    /// Create a new XDR serializer with a capacity hint.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: Vec::with_capacity(capacity),
        }
    }

    /// Get the serialized bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.output
    }

    /// Write padding bytes for 4-byte alignment.
    fn write_padding(&mut self, len: usize) {
        let padding = (4 - (len % 4)) % 4;
        self.output.extend(std::iter::repeat(0u8).take(padding));
    }
}

impl Default for XdrSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Special serializer for fixed-length opaque that writes bytes without length prefix.
struct FixedOpaqueSerializer<'a> {
    output: &'a mut Vec<u8>,
}

impl<'a> ser::Serializer for &'a mut FixedOpaqueSerializer<'a> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = ser::Impossible<(), Error>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        // Write raw bytes without length prefix
        self.output.extend_from_slice(v);
        // Add padding for 4-byte alignment
        let padding = (4 - (v.len() % 4)) % 4;
        self.output.extend(std::iter::repeat(0u8).take(padding));
        Ok(())
    }

    // All other methods are unsupported - we only expect serialize_bytes
    fn serialize_bool(self, _: bool) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_i8(self, _: i8) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_i16(self, _: i16) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_i32(self, _: i32) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_i64(self, _: i64) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_u8(self, _: u8) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_u16(self, _: u16) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_u32(self, _: u32) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_u64(self, _: u64) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_f32(self, _: f32) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_f64(self, _: f64) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_char(self, _: char) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_str(self, _: &str) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_none(self) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_unit(self) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_unit_struct(self, _: &'static str) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _: &'static str, _: &T) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _: &'static str, _: u32, _: &'static str, _: &T) -> Result<()> { Err(Error::Message("unsupported".into())) }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq> { Err(Error::Message("unsupported".into())) }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple> { Err(Error::Message("unsupported".into())) }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeTupleStruct> { Err(Error::Message("unsupported".into())) }
    fn serialize_tuple_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeTupleVariant> { Err(Error::Message("unsupported".into())) }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap> { Err(Error::Message("unsupported".into())) }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct> { Err(Error::Message("unsupported".into())) }
    fn serialize_struct_variant(self, _: &'static str, _: u32, _: &'static str, _: usize) -> Result<Self::SerializeStructVariant> { Err(Error::Message("unsupported".into())) }
}

impl<'a> ser::Serializer for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_u32(if v { 1 } else { 0 })
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        let bytes = v.as_bytes();
        self.serialize_u32(bytes.len() as u32)?;
        self.output.extend_from_slice(bytes);
        self.write_padding(bytes.len());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.serialize_u32(v.len() as u32)?;
        self.output.extend_from_slice(v);
        self.write_padding(v.len());
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_u32(0)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        self.serialize_u32(1)?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.serialize_i32(variant_index as i32)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<()> {
        if name == "FixedOpaque16" {
            // Special handling: inner value will call serialize_bytes,
            // but we override to not write length prefix
            let mut fixed_ser = FixedOpaqueSerializer { output: &mut self.output };
            value.serialize(&mut fixed_ser)
        } else {
            value.serialize(self)
        }
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.serialize_i32(variant_index as i32)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        if let Some(len) = len {
            self.serialize_u32(len as u32)?;
        }
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.serialize_i32(variant_index as i32)?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        if let Some(len) = len {
            self.serialize_u32(len as u32)?;
        }
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_i32(variant_index as i32)?;
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut XdrSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
        crate::to_bytes(value)
    }

    #[test]
    fn test_serialize_i32() {
        assert_eq!(to_bytes(&42i32).unwrap(), vec![0, 0, 0, 42]);
        assert_eq!(to_bytes(&-1i32).unwrap(), vec![255, 255, 255, 255]);
    }

    #[test]
    fn test_serialize_u32() {
        assert_eq!(to_bytes(&42u32).unwrap(), vec![0, 0, 0, 42]);
        assert_eq!(to_bytes(&0xFFFFFFFFu32).unwrap(), vec![255, 255, 255, 255]);
    }

    #[test]
    fn test_serialize_i64() {
        assert_eq!(
            to_bytes(&42i64).unwrap(),
            vec![0, 0, 0, 0, 0, 0, 0, 42]
        );
    }

    #[test]
    fn test_serialize_bool() {
        assert_eq!(to_bytes(&true).unwrap(), vec![0, 0, 0, 1]);
        assert_eq!(to_bytes(&false).unwrap(), vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_serialize_string() {
        // "hi" -> length 2 + "hi" + 2 bytes padding
        assert_eq!(to_bytes(&"hi").unwrap(), vec![0, 0, 0, 2, b'h', b'i', 0, 0]);

        // "test" -> length 4 + "test" (already aligned)
        assert_eq!(
            to_bytes(&"test").unwrap(),
            vec![0, 0, 0, 4, b't', b'e', b's', b't']
        );
    }

    #[test]
    fn test_serialize_vec() {
        let v: Vec<i32> = vec![1, 2, 3];
        assert_eq!(
            to_bytes(&v).unwrap(),
            vec![
                0, 0, 0, 3, // length
                0, 0, 0, 1, // 1
                0, 0, 0, 2, // 2
                0, 0, 0, 3, // 3
            ]
        );
    }

    #[test]
    fn test_serialize_option() {
        let some: Option<i32> = Some(42);
        let none: Option<i32> = None;

        assert_eq!(
            to_bytes(&some).unwrap(),
            vec![0, 0, 0, 1, 0, 0, 0, 42]
        );
        assert_eq!(to_bytes(&none).unwrap(), vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_serialize_struct() {
        #[derive(Serialize)]
        struct Point {
            x: i32,
            y: i32,
        }

        let p = Point { x: 10, y: 20 };
        assert_eq!(
            to_bytes(&p).unwrap(),
            vec![0, 0, 0, 10, 0, 0, 0, 20]
        );
    }
}

