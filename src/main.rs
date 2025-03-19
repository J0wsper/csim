use std::cmp::max;
use std::collections::BTreeMap;

// We're gonna put these into a map
#[derive(Debug, PartialOrd, Ord)]
struct Item {
    label: String,
    cost: u32,
    size: u32,
}

impl Item {
    fn dummy(_label: String) -> Self {
        Item {
            label: _label,
            cost: 0,
            size: 0,
        }
    }
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
    fn get_cost(&self) -> u32 {
        self.cost
    }
    fn get_size(&self) -> u32 {
        self.size
    }
}

// Just comparing by label
impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.label.trim() == other.label.trim()
    }
}

impl Eq for Item {}

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
    // Refreshes the credit of the given item if it exists
    fn refresh(&mut self, val: &Item) -> u32 {
        let mut cred = match self.cache.get_mut(&val) {
            Some(i) => i,
            None => &u32::MAX,
        };
        let bind = self.ref_scalar * val.cost;
        cred = max(cred, &bind);
        *cred
    }

    // Requests the given item. Refreshes the credit of the item if it exists and otherwise the
    // item gets added. We update the tiebreaking order accordingly.
    fn request(&mut self, val: &Item) -> u32 {
        self.refresh(&val)
    }
}

fn main() {
    println!("Hello, world!");
}
