use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::{BTreeMap, VecDeque};
use std::{env, fs};
use toml::toml;

// STRUCTS
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct TraceInfo {
    items: Vec<Item>,
    trace: Vec<String>,
}

// These are the keys for our cache map
#[derive(Debug, PartialOrd, PartialEq, Eq, Ord, Serialize, Deserialize)]
struct Item {
    label: String,
    cost: u32,
    size: u32,
}

// Wrapper for the cache
#[derive(Debug)]
struct Cache<'a> {
    contents: BTreeMap<&'a Item, OrderedFloat<f32>>,
    policy: HitPolicy,
    len: usize,
    size: u32,
}

// Wrapper for the tiebreaking order
#[derive(Debug)]
struct Tiebreaker<'a> {
    order: VecDeque<&'a Item>,
    policy: TiebreakingPolicy,
    len: usize,
    size: u32,
}

// Hit policies. The first four have a default behavior implemented. Any after that will then defer
// the hit policy to whatever function you decide to assign to the enum. This can be anything and
// you don't need to keep the name 'custom'.
#[derive(Debug)]
enum HitPolicy {
    Lru,
    Fifo,
    Rand,
    Half,
    Custom(fn(&mut Cache, &mut Tiebreaker, &Item)),
}

#[derive(Debug)]
enum TiebreakingPolicy {
    Lru,
    Fifo,
    Rand,
    Custom(fn(&mut Cache, &mut Tiebreaker, &Item)),
}

#[derive(Debug)]
struct Landlord<'a> {
    cache: Cache<'a>,
    tiebreaker: Tiebreaker<'a>,
}

// IMPLEMENTATING STRUCTS
// -----------------------------------------------------------------------------

impl Item {
    fn new(_label: String, _cost: u32, _size: u32) -> Self {
        Item {
            label: _label,
            cost: _cost,
            size: _size,
        }
    }
    fn get_label(&self) -> &String {
        &self.label
    }
    fn get_cost(&self) -> OrderedFloat<f32> {
        let float_cost = self.cost as f32;
        OrderedFloat(float_cost)
    }
    fn get_size(&self) -> u32 {
        self.size
    }
}

impl<'a> Landlord<'a> {
    fn hit(&mut self, item: &Item) -> OrderedFloat<f32>;
    fn fault(&mut self, item: &Item) -> OrderedFloat<f32> {
        // If the cache has too many items, throw an error
        if self.cache.len > self.cache.size as usize {
            panic!("Cache is overfull");
        }
        // If the cache has empty space, just add the item
        else if self.cache.len < self.cache.size as usize {
            self.cache.contents.insert(item, item.get_cost());
            return OrderedFloat(0.0);
        }
        // Otherwise, the cache is full and we need evict something
        else {
        }
    }
    fn request(&mut self, item: &Item) -> OrderedFloat<f32> {
        if self.cache.contents.contains_key(&item) {
            return self.hit(item);
        } else {
            return self.fault(item);
        }
    }
}

// IMPLEMENTING TRAITS
// -----------------------------------------------------------------------------

fn main() {
    let data: &str = &fs::read_to_string("items.toml").expect("Could not read file");
    let test: TraceInfo = toml::from_str(data).expect("Could not convert TOML file");
    dbg!(test);
}
