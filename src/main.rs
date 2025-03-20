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

// Tiebreaking order struct
#[derive(Debug)]
struct Tiebreaker<'a> {
    order: VecDeque<&'a Item>,
    len: u32,
    size: u32,
}

#[derive(Debug)]
struct Cache<'a> {
    contents: BTreeMap<&'a Item, f32>,
    len: u32,
    size: u32,
}

// LRU Landlord
#[derive(Debug)]
struct LLL<'a> {
    cache: Cache<'a>,
    tiebreaking: Tiebreaker<'a>,
}

// FIFO Landlord
#[derive(Debug)]
struct FLL<'a> {
    cache: Cache<'a>,
    tiebreaking: Tiebreaker<'a>,
}

// TRAITS
// -----------------------------------------------------------------------------

// Hit policy trait. You can implement this for your Landlord variant to give it custom hit policy
// behavior that doesn't just refresh the credit to some scalar between 0 and 1.
trait HitPolicy {
    fn refresh(&mut self, item: &Item) -> f32;
}

// Tiebreaking trait. You can implement this for your Landlord variant to give it custom
// tiebreaking behavior that isn't LRU or FIFO.
trait TiebreakPolicy<'a> {
    fn tiebreak_update(&mut self, item: &'a Item);
}

trait Request {
    fn request(&mut self, item: &Item) -> f32;
}

// Implements the default Landlord functionality as its own trait
trait Landlord {
    fn hit() -> f32;
    fn fault() -> Option<f32>;
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
    fn get_cost(&self) -> f32 {
        self.cost as f32
    }
    fn get_size(&self) -> u32 {
        self.size
    }
}

// Generic tiebreaking behavior
impl<'a> Tiebreaker<'_> {
    // We tiebreak according to whichever item is closest to the back.
    fn tiebreak(&mut self, cache: &Cache) -> &Item {
        for (i, item) in self.order.iter().rev().enumerate() {
            let credit = *match cache.contents.get(item) {
                Some(i) => i,
                None => panic!("Cache tiebreaking ordering contains items that are not in cache"),
            };
            if credit == 0.0 {
                return item;
            }
        }
        panic!("Cache does not have any 0 credit elements at the time of eviction.");
    }
}

// If we fault and it returns 'false', we should enter into a tiebreaking decision.
impl<'a> Cache<'a> {

    fn fault(&mut self, req: &'a Item) -> bool {
        if self.len > self.size {
            panic!("Cache contains too many reqs");
        } else if self.len < self.size {
            self.contents.insert(req, req.get_cost());
            return true;
        } else {
            let mut min = &f32::MAX;

            let mut min = self.contents.into_iter().min_by_key(|a| a.1);

            // And another pass to find if there are any others that share that credit.
            let mut num = 0;
            let mut i: &Item;
            for (item, cred) in self.contents.iter() {
                if cred == min {
                    i = item;
                    num += 1;
                }
            }

            if num == 1 {
                self.contents.iter().map(|(x, k)| k - min);                
                self.contents.remove(i);
                return true;
            }
            else {
                return false;
            }
        }
    }
    fn request(&mut self, item: &'a Item) -> bool {
        if self.contents.contains_key(item) {
            Cache::<'_>hit(item);
        } else {
            Cache::<'_>fault(item);
        }
    }
}

// IMPLEMENTING TRAITS
// -----------------------------------------------------------------------------

impl<'a> TiebreakPolicy<'a> for FLL<'a> {
    // We do not update the tiebreaking policy on a hit to an item. However, we do update the
    // ordering if that item just entered cache.
    fn tiebreak_update(&mut self, item: &'a Item) {
        let contains = self.tiebreaking.order.contains(&item);
        if !contains && self.tiebreaking.len >= self.tiebreaking.size {
            panic!("Tiebreaking order housekeeping failed");
        } else if !contains {
            self.tiebreaking.order.push_front(item);
        }
    }
}

// Implementing the LRU-Landlord tiebreaking policy
impl<'a> TiebreakPolicy<'a> for LLL<'a> {
    fn tiebreak_update(&mut self, item: &'a Item) {
        let contains = self.tiebreaking.order.contains(&item);
        if !contains && self.tiebreaking.len >= self.tiebreaking.size {
            panic!("Tiebreaking order housekeeping failed");
        } else if !contains {
            self.tiebreaking.order.push_front(item);
        } else {
            let pos = self
                .tiebreaking
                .order
                .iter()
                .position(|x| x == &item)
                .expect("Could not find the position of the given item in the tiebreaking order despite containment");
            self.tiebreaking.order.remove(pos);
            self.tiebreaking.order.push_front(item);
        }
    }
}

fn main() {
    let data: &str = &fs::read_to_string("items.toml").expect("Could not read file");
    let test: TraceInfo = toml::from_str(data).expect("Could not convert TOML file");
    dbg!(test);
}
