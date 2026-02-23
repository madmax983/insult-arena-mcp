//! JSON utilities for the server.

use serde::Deserializer;
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use std::fmt;

/// Helper for lossy string deserialization.
///
/// If the input is a string, it returns it.
/// If the input is anything else (or null), it returns an empty string.
///
/// # Robustness
///
/// This preserves legacy behavior where invalid types were treated as empty strings.
/// This prevents deserialization errors from crashing the request handler when
/// clients send unexpected types (e.g., numbers, nulls).
pub fn deserialize_lossy_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(LossyStringVisitor)
}

struct LossyStringVisitor;

impl<'de> Visitor<'de> for LossyStringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string or anything else (which becomes empty string)")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(v.to_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(v)
    }

    fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(String::new())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(String::new())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(String::new())
    }
}
