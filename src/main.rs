use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
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
    order: BTreeMap<&'a Item, u32>,
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
    // Utility function to get the normalized credit of an item
    fn norm_credit(item: (&&'a Item, &OrderedFloat<f32>)) -> OrderedFloat<f32> {
        item.1 / OrderedFloat(item.0.get_size() as f32)
    }

    // Takes care of cleaning up our tiebreaking order
    fn manage_tiebreak(&mut self, item: &Item) {
        self.tiebreaker.order.remove(item);
    }

    // TODO: Implement this
    fn hit(&mut self, item: &'a Item) -> OrderedFloat<f32> {
        OrderedFloat(0.0)
    }

    // Finding the element we want to evict in the case of a tie
    fn tiebreak(&mut self, zeros: Vec<&'a Item>) -> &'a Item {
        if zeros.len() == 1 {
            return zeros[0];
        }
        for cand in self.tiebreaker.order.iter().rev() {
            if zeros.contains(cand.0) {
                return cand.0;
            }
        }
        panic!("Tiebreaking order mismanagement");
    }

    // Evicting an element if we do not have enough space for it
    fn evict(&mut self, size: u32) -> OrderedFloat<f32> {
        // Getting our return value
        let mut pressure = OrderedFloat(0.0);

        // Base case
        if self.cache.size as usize - self.cache.len >= size as usize {
            return pressure;
        }

        // Getting the normalized credit of the minimum credit item
        let min = Landlord::norm_credit(
            self.cache
                .contents
                .iter()
                .min_by_key(|a| a.1 / OrderedFloat(a.0.get_size() as f32))
                .expect("Could not find minimum credit element"),
        );
        let _ = self.cache.contents.iter().map(|a| a.1 - min);
        pressure += min;

        // Finding how many items of 0 credit there are now
        let mut zeros: Vec<&'_ Item> = Vec::new();
        for item in self.cache.contents.iter() {
            if *item.1 == OrderedFloat(0.0) {
                zeros.push(*item.0);
            }
        }
        // Letting our tiebreaking policy take care of choosing the evicted item
        let evicted = self.tiebreak(zeros);
        self.manage_tiebreak(evicted);

        // Removing the item it picks
        self.cache.contents.remove(evicted);
        self.cache.len -= 1;

        // Returning our pressure at the end
        pressure + self.evict(size)
    }

    // The function called whenever the Landlord implementation faults on a request
    fn fault(&mut self, item: &'a Item) -> OrderedFloat<f32> {
        // If the cache has too many items, throw an error
        if self.cache.len > self.cache.size as usize {
            panic!("Cache is overfull");
        }
        // If the cache has empty space, just add the item
        else if self.cache.len < (self.cache.size + item.get_size()) as usize {
            self.cache.contents.insert(item, item.get_cost());
            return OrderedFloat(0.0);
        }
        // Otherwise, the cache is full and we need evict something
        else {
            let size = item.get_size();
            return self.evict(size);
        }
    }
    fn request(&mut self, item: &'a Item) -> OrderedFloat<f32> {
        if self.cache.contents.contains_key(&item) {
            self.hit(item)
        } else {
            self.fault(item)
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
