use frunk_core::hlist;
use serde::{Deserialize, Serialize};

use crate::hlabeledlist::{HLabelledMap, Labelled};
use crate::hmap::HMap;

mod hlabeledlist;
mod hmap;

fn decl_type_wrapper<T, F, I>(_dep: &T, f: F, input: I) -> T
where
    F: FnOnce(I) -> T,
{
    f(input)
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct StrangeTypeA {
    touko: u32,
}

impl Labelled for StrangeTypeA {
    const KEY: &'static str = "touko";
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct StrangeTypeB {
    aspirin: String,
}

impl Labelled for StrangeTypeB {
    const KEY: &'static str = "aspirin";
}

fn main() {
    let l = hlist![
        (1, "2"),
        ("2", 3),
        ("3", String::from("4")),
        Option::<(i64, i64)>::None,
        Some((4, 5))
    ];
    println!("{}", serde_json::to_string(&HMap(l).as_ref()).unwrap());

    let l = HMap(hlist![(1, "2"), ("2", 3), (3, 4)]);
    let serialized = serde_json::to_string(&l.as_ref()).unwrap();
    let l2 = decl_type_wrapper(&l, deserialize, serialized.as_str());
    assert_eq!(l, l2);

    let l = HLabelledMap(hlist![
        StrangeTypeA { touko: 1 },
        StrangeTypeB {
            aspirin: String::from("a")
        }
    ]);
    let serialized = serde_json::to_string(&l.as_ref()).unwrap();
    println!("{}", serialized);
    let l2 = decl_type_wrapper(&l, deserialize, serialized.as_str());
    assert_eq!(l, l2);
}

fn deserialize<'de, T>(input: &'de str) -> T
where
    T: Deserialize<'de>,
{
    serde_json::from_str(input).unwrap()
}
