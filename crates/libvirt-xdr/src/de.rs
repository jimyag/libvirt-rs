//! XDR Deserializer implementation.

use crate::error::{Error, Result};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};

/// XDR Deserializer.
pub struct XdrDeserializer<'de> {
    input: &'de [u8],
    pos: usize,
}

impl<'de> XdrDeserializer<'de> {
    /// Create a new XDR deserializer.
    pub fn new(input: &'de [u8]) -> Self {
        Self { input, pos: 0 }
    }

    /// Get remaining bytes.
    pub fn remaining(&self) -> usize {
        self.input.len() - self.pos
    }

    /// Read exactly `n` bytes.
    fn read_bytes(&mut self, n: usize) -> Result<&'de [u8]> {
        if self.pos + n > self.input.len() {
            return Err(Error::Eof);
        }
        let bytes = &self.input[self.pos..self.pos + n];
        self.pos += n;
        Ok(bytes)
    }

    /// Skip padding bytes for 4-byte alignment.
    fn skip_padding(&mut self, len: usize) -> Result<()> {
        let padding = (4 - (len % 4)) % 4;
        if padding > 0 {
            self.read_bytes(padding)?;
        }
        Ok(())
    }

    fn read_i32(&mut self) -> Result<i32> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_u64(&mut self) -> Result<u64> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut XdrDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message(
            "XDR does not support deserialize_any".to_string(),
        ))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let v = self.read_u32()?;
        match v {
            0 => visitor.visit_bool(false),
            1 => visitor.visit_bool(true),
            _ => Err(Error::InvalidBool(v)),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i8(self.read_i32()? as i8)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i16(self.read_i32()? as i16)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i32(self.read_i32()?)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i64(self.read_i64()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u8(self.read_u32()? as u8)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u16(self.read_u32()? as u16)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u32(self.read_u32()?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u64(self.read_u64()?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let bytes = self.read_bytes(4)?;
        let v = f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        visitor.visit_f32(v)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let bytes = self.read_bytes(8)?;
        let v = f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        visitor.visit_f64(v)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let v = self.read_u32()?;
        match char::from_u32(v) {
            Some(c) => visitor.visit_char(c),
            None => Err(Error::Message(format!("invalid char: {}", v))),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        self.skip_padding(len)?;
        let s = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
        visitor.visit_borrowed_str(s)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        self.skip_padding(len)?;
        let s = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
        visitor.visit_string(s.to_string())
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        self.skip_padding(len)?;
        visitor.visit_borrowed_bytes(bytes)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        self.skip_padding(len)?;
        visitor.visit_byte_buf(bytes.to_vec())
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let present = self.read_u32()?;
        match present {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(self),
            _ => Err(Error::Message(format!(
                "invalid option discriminant: {}",
                present
            ))),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        // Special handling for FixedOpaque16 (UUID) - read 16 raw bytes without length prefix
        if name == "FixedOpaque16" {
            let bytes = self.read_bytes(16)?;
            // No padding needed for 16 bytes (already 4-byte aligned)
            return visitor.visit_bytes(bytes);
        }
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_u32()? as usize;
        visitor.visit_seq(SeqAccessor::new(self, len))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(SeqAccessor::new(self, len))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(SeqAccessor::new(self, len))
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.read_u32()? as usize;
        visitor.visit_map(MapAccessor::new(self, len))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(SeqAccessor::new(self, fields.len()))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(EnumAccessor::new(self))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let variant_index = self.read_i32()?;
        visitor.visit_u32(variant_index as u32)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message(
            "XDR does not support deserialize_ignored_any".to_string(),
        ))
    }
}

struct SeqAccessor<'a, 'de: 'a> {
    de: &'a mut XdrDeserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> SeqAccessor<'a, 'de> {
    fn new(de: &'a mut XdrDeserializer<'de>, len: usize) -> Self {
        Self { de, remaining: len }
    }
}

impl<'de, 'a> SeqAccess<'de> for SeqAccessor<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct MapAccessor<'a, 'de: 'a> {
    de: &'a mut XdrDeserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> MapAccessor<'a, 'de> {
    fn new(de: &'a mut XdrDeserializer<'de>, len: usize) -> Self {
        Self { de, remaining: len }
    }
}

impl<'de, 'a> MapAccess<'de> for MapAccessor<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.de)
    }
}

struct EnumAccessor<'a, 'de: 'a> {
    de: &'a mut XdrDeserializer<'de>,
}

impl<'a, 'de> EnumAccessor<'a, 'de> {
    fn new(de: &'a mut XdrDeserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for EnumAccessor<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for EnumAccessor<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_tuple(&mut *self.de, len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        de::Deserializer::deserialize_struct(&mut *self.de, "", fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    fn from_bytes<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
        crate::from_bytes(bytes)
    }

    #[test]
    fn test_deserialize_i32() {
        assert_eq!(from_bytes::<i32>(&[0, 0, 0, 42]).unwrap(), 42);
        assert_eq!(from_bytes::<i32>(&[255, 255, 255, 255]).unwrap(), -1);
    }

    #[test]
    fn test_deserialize_u32() {
        assert_eq!(from_bytes::<u32>(&[0, 0, 0, 42]).unwrap(), 42);
        assert_eq!(
            from_bytes::<u32>(&[255, 255, 255, 255]).unwrap(),
            0xFFFFFFFF
        );
    }

    #[test]
    fn test_deserialize_i64() {
        assert_eq!(
            from_bytes::<i64>(&[0, 0, 0, 0, 0, 0, 0, 42]).unwrap(),
            42
        );
    }

    #[test]
    fn test_deserialize_bool() {
        assert!(from_bytes::<bool>(&[0, 0, 0, 1]).unwrap());
        assert!(!from_bytes::<bool>(&[0, 0, 0, 0]).unwrap());
        assert!(from_bytes::<bool>(&[0, 0, 0, 2]).is_err());
    }

    #[test]
    fn test_deserialize_string() {
        assert_eq!(
            from_bytes::<String>(&[0, 0, 0, 2, b'h', b'i', 0, 0]).unwrap(),
            "hi"
        );
        assert_eq!(
            from_bytes::<String>(&[0, 0, 0, 4, b't', b'e', b's', b't']).unwrap(),
            "test"
        );
    }

    #[test]
    fn test_deserialize_vec() {
        let bytes = [
            0, 0, 0, 3, // length
            0, 0, 0, 1, // 1
            0, 0, 0, 2, // 2
            0, 0, 0, 3, // 3
        ];
        assert_eq!(from_bytes::<Vec<i32>>(&bytes).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_deserialize_option() {
        assert_eq!(
            from_bytes::<Option<i32>>(&[0, 0, 0, 1, 0, 0, 0, 42]).unwrap(),
            Some(42)
        );
        assert_eq!(
            from_bytes::<Option<i32>>(&[0, 0, 0, 0]).unwrap(),
            None
        );
    }

    #[test]
    fn test_deserialize_struct() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Point {
            x: i32,
            y: i32,
        }

        let bytes = [0, 0, 0, 10, 0, 0, 0, 20];
        assert_eq!(
            from_bytes::<Point>(&bytes).unwrap(),
            Point { x: 10, y: 20 }
        );
    }

    #[test]
    fn test_roundtrip() {
        use serde::Serialize;

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct TestStruct {
            name: String,
            value: i32,
            items: Vec<u32>,
            optional: Option<i64>,
        }

        let original = TestStruct {
            name: "test".to_string(),
            value: -42,
            items: vec![1, 2, 3],
            optional: Some(12345),
        };

        let bytes = crate::to_bytes(&original).unwrap();
        let decoded: TestStruct = crate::from_bytes(&bytes).unwrap();

        assert_eq!(original, decoded);
    }
}
