use std::fmt::Formatter;
use std::marker::PhantomData;

use frunk_core::hlist::{HCons, HList, HNil};
use frunk_core::traits::IntoReverse;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HMap<T>(pub T);

impl<T> HMap<T> {
    pub const fn as_ref(&self) -> HMapRef<T> {
        HMapRef(&self.0)
    }
}

#[derive(Debug)]
pub struct HMapRef<'a, T>(pub &'a T);

impl<'a, T> Serialize for HMapRef<'a, T>
where
    T: HList,
    Self: MapSerializable,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(T::LEN))?;
        self.serialize_map(&mut map)?;
        map.end()
    }
}

pub trait MapSerializable {
    fn serialize_map<S: SerializeMap>(&self, serializer: &mut S) -> Result<(), S::Error>;
}

impl<'a, K, V, T> MapSerializable for HMapRef<'a, HCons<(K, V), T>>
where
    K: Serialize,
    V: Serialize,
    HMapRef<'a, T>: MapSerializable,
{
    fn serialize_map<S: SerializeMap>(&self, serializer: &mut S) -> Result<(), S::Error> {
        let (k, v) = &self.0.head;
        serializer.serialize_entry(k, v)?;
        HMapRef(&self.0.tail).serialize_map(serializer)
    }
}

impl<'a, K, V, T> MapSerializable for HMapRef<'a, HCons<Option<(K, V)>, T>>
where
    K: Serialize,
    V: Serialize,
    HMapRef<'a, T>: MapSerializable,
{
    fn serialize_map<S: SerializeMap>(&self, serializer: &mut S) -> Result<(), S::Error> {
        if let Some((k, v)) = &self.0.head {
            serializer.serialize_entry(k, v)?;
        }
        HMapRef(&self.0.tail).serialize_map(serializer)
    }
}

impl MapSerializable for HMapRef<'_, HNil> {
    fn serialize_map<S: SerializeMap>(&self, _serializer: &mut S) -> Result<(), S::Error> {
        Ok(())
    }
}

struct HMapVisitor<L>(PhantomData<L>);

impl<L> Default for HMapVisitor<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<'de, L> Visitor<'de> for HMapVisitor<L>
where
    L: IntoReverse,
    L::Output: MapDeserializable<'de> + IntoReverse<Output = L>,
{
    type Value = HMap<L>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a heterogeneous map")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let (reversed, _) = <L::Output as MapDeserializable<'de>>::visit_map(map)?;
        Ok(HMap(reversed.into_reverse()))
    }
}

pub trait MapDeserializable<'de>: HList {
    fn visit_map<A: MapAccess<'de>>(map: A) -> Result<(Self, A), A::Error>;
}

impl<'de, K, V, T> MapDeserializable<'de> for HCons<(K, V), T>
where
    K: Deserialize<'de>,
    V: Deserialize<'de>,
    T: MapDeserializable<'de>,
{
    fn visit_map<A: MapAccess<'de>>(map: A) -> Result<(Self, A), A::Error> {
        let (append, mut map) = T::visit_map(map)?;
        let (k, v) = map.next_entry()?.expect("unexpected eof");
        Ok((append.prepend((k, v)), map))
    }
}

impl<'de> MapDeserializable<'de> for HNil {
    fn visit_map<A: MapAccess<'de>>(map: A) -> Result<(Self, A), A::Error> {
        Ok((HNil, map))
    }
}

impl Default for HMap<HNil> {
    fn default() -> Self {
        Self(HNil)
    }
}

impl<'de, L> Deserialize<'de> for HMap<L>
where
    L: IntoReverse,
    L::Output: MapDeserializable<'de> + IntoReverse<Output = L>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(HMapVisitor::default())
    }
}
