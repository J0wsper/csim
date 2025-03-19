use std::cmp::max;
use std::collections::BTreeMap;

// We're gonna put these into a map
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Item {
    label: String,
    cost: u32,
    size: u32,
}

impl Item {
    fn get_label(&self) -> &String {
        &self.label
    }
    fn get_cost(&self) -> u32 {
        self.cost
    }
    fn get_size(&self) -> u32 {
        self.size
    }
}

// Unfortunately, due to technical limitations, I can only do LRU or FIFO tiebreaking.
#[derive(Debug)]
enum TieBreaker {
    FIFO,
    LRU,
}

#[derive(Debug)]
struct Landlord {
    cache: BTreeMap<Item, u32>,
    size: u32,
    ref_scalar: u32,
    tie_breaking: TieBreaker,
}

impl Landlord {
    fn refresh(&self, val: &String) -> u32 {
        0
    }
}

fn main() {
    println!("Hello, world!");
}
