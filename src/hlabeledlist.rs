use std::fmt::Formatter;

use frunk_core::hlist::{HCons, HList, HNil};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

pub trait Labelled {
    const KEY: &'static str;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HLabelledMap<T>(pub T);

impl<T> HLabelledMap<T> {
    pub const fn as_ref(&self) -> HLabelledMapRef<T> {
        HLabelledMapRef(&self.0)
    }
}

#[derive(Debug)]
pub struct HLabelledMapRef<'a, T>(pub &'a T);

impl<'a, T> Serialize for HLabelledMapRef<'a, T>
where
    T: HList,
    Self: LabelledMapSerializable,
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

pub trait LabelledMapSerializable {
    fn serialize_map<S: SerializeMap>(&self, serializer: &mut S) -> Result<(), S::Error>;
}

impl<'a, E, T> LabelledMapSerializable for HLabelledMapRef<'a, HCons<E, T>>
where
    E: Serialize + Labelled,
    HLabelledMapRef<'a, T>: LabelledMapSerializable,
{
    fn serialize_map<S: SerializeMap>(&self, serializer: &mut S) -> Result<(), S::Error> {
        let e = &self.0.head;
        serializer.serialize_entry(E::KEY, e)?;
        HLabelledMapRef(&self.0.tail).serialize_map(serializer)
    }
}

impl<'a, E, T> LabelledMapSerializable for HLabelledMapRef<'a, HCons<Option<E>, T>>
where
    E: Serialize + Labelled,
    HLabelledMapRef<'a, T>: LabelledMapSerializable,
{
    fn serialize_map<S: SerializeMap>(&self, serializer: &mut S) -> Result<(), S::Error> {
        if let Some(e) = &self.0.head {
            serializer.serialize_entry(E::KEY, e)?;
        }
        HLabelledMapRef(&self.0.tail).serialize_map(serializer)
    }
}

impl LabelledMapSerializable for HLabelledMapRef<'_, HNil> {
    fn serialize_map<S: SerializeMap>(&self, _serializer: &mut S) -> Result<(), S::Error> {
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum MaybeUnfilled<T> {
    Filled(T),
    Unfilled,
}

impl<T> MaybeUnfilled<T> {
    fn fill(&mut self, item: T) {
        *self = Self::Filled(item);
    }
    fn try_unwrap<E: de::Error>(self, field: &'static str) -> Result<T, E> {
        Option::from(self).ok_or_else(|| de::Error::missing_field(field))
    }
}

impl<T> From<MaybeUnfilled<T>> for Option<T> {
    fn from(item: MaybeUnfilled<T>) -> Self {
        match item {
            MaybeUnfilled::Filled(inner) => Some(inner),
            MaybeUnfilled::Unfilled => None,
        }
    }
}

pub trait HListMaybeUnfilled: HList {}

impl<H, T: HListMaybeUnfilled> HListMaybeUnfilled for HCons<MaybeUnfilled<H>, T> {}

impl HListMaybeUnfilled for HNil {}

pub trait IntoHListMaybeUnfilled: HList {
    type Output: HListMaybeUnfilled;
    fn create() -> Self::Output;
}

impl<H, T> IntoHListMaybeUnfilled for HCons<H, T>
where
    T: IntoHListMaybeUnfilled,
{
    type Output = HCons<MaybeUnfilled<H>, T::Output>;

    fn create() -> Self::Output {
        T::create().prepend(MaybeUnfilled::Unfilled)
    }
}

impl IntoHListMaybeUnfilled for HNil {
    type Output = Self;

    fn create() -> Self::Output {
        Self
    }
}

pub trait IntoHListFilled<Output>: HListMaybeUnfilled {
    fn convert<E: de::Error>(self) -> Result<Output, E>;
}

impl<H, T, TOutput> IntoHListFilled<HCons<Option<H>, TOutput>> for HCons<MaybeUnfilled<H>, T>
where
    T: IntoHListFilled<TOutput>,
    TOutput: HList,
{
    fn convert<E: de::Error>(self) -> Result<HCons<Option<H>, TOutput>, E> {
        Ok(self.tail.convert()?.prepend(self.head.into()))
    }
}

impl<H, T, TOutput> IntoHListFilled<HCons<H, TOutput>> for HCons<MaybeUnfilled<H>, T>
where
    H: Labelled,
    T: IntoHListFilled<TOutput>,
    TOutput: HList,
{
    fn convert<E: de::Error>(self) -> Result<HCons<H, TOutput>, E> {
        Ok(self.tail.convert()?.prepend(self.head.try_unwrap(H::KEY)?))
    }
}

impl IntoHListFilled<Self> for HNil {
    fn convert<E: de::Error>(self) -> Result<Self, E> {
        Ok(Self)
    }
}

struct HLabelledMapVisitor<L: IntoHListMaybeUnfilled> {
    maybe_unfilled: L::Output,
}

impl<L: IntoHListMaybeUnfilled> Default for HLabelledMapVisitor<L> {
    fn default() -> Self {
        Self {
            maybe_unfilled: L::create(),
        }
    }
}

pub trait FillByLabel<'de> {
    fn fill_by_name<A: MapAccess<'de>>(&mut self, name: &str, map: &mut A) -> Result<(), A::Error>;
}

impl<'de, H, T> FillByLabel<'de> for HCons<MaybeUnfilled<H>, T>
where
    H: Labelled + Deserialize<'de>,
    T: FillByLabel<'de>,
{
    fn fill_by_name<A: MapAccess<'de>>(&mut self, name: &str, map: &mut A) -> Result<(), A::Error> {
        if H::KEY == name {
            self.head.fill(map.next_value()?);
            Ok(())
        } else {
            self.tail.fill_by_name(name, map)
        }
    }
}

impl<'de> FillByLabel<'de> for HNil {
    fn fill_by_name<A: MapAccess<'de>>(
        &mut self,
        _name: &str,
        _map: &mut A,
    ) -> Result<(), A::Error> {
        Ok(())
    }
}

impl<'de, L> Visitor<'de> for HLabelledMapVisitor<L>
where
    L: IntoHListMaybeUnfilled,
    L::Output: IntoHListFilled<L> + FillByLabel<'de>,
{
    type Value = HLabelledMap<L>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a labelled heterogeneous map")
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(key) = map.next_key()? {
            self.maybe_unfilled.fill_by_name(key, &mut map)?;
        }
        Ok(HLabelledMap(self.maybe_unfilled.convert()?))
    }
}

impl Default for HLabelledMap<HNil> {
    fn default() -> Self {
        Self(HNil)
    }
}

impl<'de, L> Deserialize<'de> for HLabelledMap<L>
where
    L: IntoHListMaybeUnfilled,
    L::Output: IntoHListFilled<L> + FillByLabel<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(HLabelledMapVisitor::default())
    }
}
