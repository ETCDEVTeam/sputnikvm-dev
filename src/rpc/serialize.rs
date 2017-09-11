use serde::{Serialize, Serializer, Deserializer, Deserialize, de};
use std::fmt::{self, LowerHex};
use std::marker::PhantomData;
use std::str::FromStr;

pub struct Hex<T>(pub T);

impl<T: LowerHex> Serialize for Hex<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(&format!("0x{:x}", self.0))
    }
}

struct HexVisitor<T> {
    _marker: PhantomData<T>,
}

impl<'de, T: FromStr> de::Visitor<'de> for HexVisitor<T> {
    type Value = Hex<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Must be a valid hex string")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
        where E: de::Error
    {
        match T::from_str(s) {
            Ok(s) => Ok(Hex(s)),
            Err(_) => Err(de::Error::invalid_value(de::Unexpected::Str(s), &self)),
        }
    }
}

impl<'de, T: FromStr> Deserialize<'de> for Hex<T> {
    fn deserialize<D>(deserializer: D) -> Result<Hex<T>, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_str(HexVisitor {
            _marker: PhantomData
        })
    }
}
