use ordered_float::OrderedFloat;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs;

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
    size: u32,
    occupied: u32,
}

// Wrapper for the tiebreaking order
#[derive(Debug)]
struct Tiebreaker<'a> {
    order: VecDeque<&'a Item>,
    policy: TiebreakingPolicy,
    size: u32,
    occupied: u32,
}

// Hit policies. The first four have a default behavior implemented. Any after that will then defer
// the hit policy to whatever function you decide to assign to the enum. This can be anything and
// you don't need to keep the name 'custom'.
#[derive(Debug)]
#[warn(dead_code)]
enum HitPolicy {
    Lru,
    Fifo,
    Rand,
    Half,
    Custom,
}

#[derive(Debug)]
#[warn(dead_code)]
enum TiebreakingPolicy {
    Lru,
    Fifo,
    Rand,
    Custom,
}

#[derive(Debug)]
struct Landlord<'a> {
    cache: Cache<'a>,
    tiebreaker: Tiebreaker<'a>,
}

// IMPLEMENTATING STRUCTS
// -----------------------------------------------------------------------------

impl Item {
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
    fn new(size: u32, tiebreak_policy: TiebreakingPolicy, hit_policy: HitPolicy) -> Self {
        Self {
            cache: {
                Cache {
                    contents: BTreeMap::new(),
                    policy: hit_policy,
                    size,
                    occupied: 0,
                }
            },
            tiebreaker: {
                Tiebreaker {
                    order: VecDeque::new(),
                    policy: tiebreak_policy,
                    size,
                    occupied: 0,
                }
            },
        }
    }

    // Utility function to get the normalized credit of an item
    fn norm_credit(item: (&&'a Item, &OrderedFloat<f32>)) -> OrderedFloat<f32> {
        item.1 / OrderedFloat(item.0.get_size() as f32)
    }

    // Gets the tiebreaking index of a particular item
    fn get_tiebreaker_index(&self, item: &'a Item) -> usize {
        self.tiebreaker
            .order
            .iter()
            .position(|n| n == &item)
            .expect("Item not found in vector")
    }

    // Takes care of cleaning up our tiebreaking order
    fn manage_tiebreak(&mut self, item: &Item) {
        let index = self.get_tiebreaker_index(item);
        self.tiebreaker.order.remove(index);
        self.tiebreaker.occupied -= item.get_size();
    }

    // Update our tiebreaking order on any request
    // NOTE: This is the place where custom tiebreaking is handled
    fn update_tiebreak(&mut self, item: &'a Item) {
        if self.tiebreaker.order.contains(&item) {
            let index = self.get_tiebreaker_index(item);
            match self.tiebreaker.policy {
                // Push the item to the back of the order
                TiebreakingPolicy::Lru => {
                    self.tiebreaker.order.remove(index);
                    self.tiebreaker.order.push_back(item);
                }
                // Do nothing
                TiebreakingPolicy::Fifo => {}
                // Insert the item into a random slot in the tiebreaking order
                TiebreakingPolicy::Rand => {
                    let k = self.tiebreaker.size;
                    let new_index = rand::rng().random_range(0..k - 1) as usize;
                    self.tiebreaker.order.remove(index);
                    self.tiebreaker.order.insert(new_index, item);
                }
                // Perform custom tiebreaking
                TiebreakingPolicy::Custom => self.custom_tiebreak(item),
            }
        } else {
            match self.tiebreaker.policy {
                TiebreakingPolicy::Lru => {
                    self.tiebreaker.order.push_back(item);
                }
                TiebreakingPolicy::Fifo => {
                    self.tiebreaker.order.push_back(item);
                }
                TiebreakingPolicy::Rand => {
                    let k = self.tiebreaker.size;
                    let index = rand::rng().random_range(0..k - 1) as usize;
                    self.tiebreaker.order.insert(index, item);
                }
                TiebreakingPolicy::Custom => self.custom_tiebreak(item),
            }
        }
    }

    // NOTE: You need to implement this function if you want your custom hit policy to work
    #[warn(unused_variables)]
    fn custom_hit(&mut self, item: &Item, old_cred: f32) -> OrderedFloat<f32> {
        todo!()
    }

    // NOTE: You need to implement this function if you want your custom tiebreaking policy to work
    #[warn(unused_variables)]
    fn custom_tiebreak(&mut self, item: &Item) {
        todo!()
    }

    // Function called whenever Landlord hits on an item
    // NOTE: This is where the custom hit policy is handled
    fn hit(&mut self, label: &'a Item) {
        let old_cred = self
            .cache
            .contents
            .get(label)
            .expect("Could not find hit item")
            .0;

        // Getting rid of the bad NaN case
        if old_cred.is_nan() {
            panic!("NaN credit found");
        }

        // Refresh the requested item's credit according to hit policy
        let new_cred = match &self.cache.policy {
            HitPolicy::Lru => label.get_cost(),
            HitPolicy::Fifo => OrderedFloat(old_cred),
            HitPolicy::Rand => OrderedFloat(rand::rng().random_range(old_cred..label.get_cost().0)),
            HitPolicy::Half => (label.get_cost() - old_cred) / 2.0,
            HitPolicy::Custom => self.custom_hit(label, old_cred),
        };

        // Assigning our new credit to the item
        let mut _assign_cred = self.cache.contents.get(label).unwrap();
        _assign_cred = &new_cred;
    }

    // Finding the element we want to evict in the case of a tie
    fn tiebreak(&mut self, zeros: Vec<&'a Item>) -> &'a Item {
        dbg!(&zeros);
        if zeros.len() == 1 {
            return zeros[0];
        }
        for cand in self.tiebreaker.order.iter() {
            if zeros.contains(cand) {
                return cand;
            }
        }
        panic!("Tiebreaking order mismanagement");
    }

    // Evicting an element if we do not have enough space for it
    fn evict(&mut self, size: u32) -> OrderedFloat<f32> {
        // Getting our return value
        let mut pressure = OrderedFloat(0.0);

        // Base case
        if self.cache.size - self.cache.occupied >= size {
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

        // Decrementing the credit of each item in proportion to their size
        for (item, cred) in self.cache.contents.iter_mut() {
            *cred -= min * item.get_size() as f32;
        }
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
        self.cache.occupied -= evicted.get_size();

        // Returning our pressure at the end
        pressure + self.evict(size)
    }

    // The function called whenever the Landlord implementation faults on a request
    fn fault(&mut self, item: &'a Item) -> OrderedFloat<f32> {
        // If the cache has too many items, throw an error
        if self.cache.occupied > self.cache.size {
            panic!("Cache is overfull");
        }
        // If the cache has empty space, just add the item
        else if self.cache.occupied + item.get_size() <= self.cache.size {
            self.cache.contents.insert(item, item.get_cost());
            self.cache.occupied += item.get_size();
            self.tiebreaker.occupied += item.get_size();
            OrderedFloat(0.0)
        }
        // Otherwise, the cache is full and we need evict something
        else {
            let size = item.get_size();
            let pressure = self.evict(size);
            self.cache.contents.insert(item, item.get_cost());
            pressure
        }
    }

    // Handle our request
    fn request(&mut self, item: &'a Item) -> OrderedFloat<f32> {
        if self.cache.contents.contains_key(&item) {
            self.hit(item);
            self.update_tiebreak(item);
            OrderedFloat(0.0)
        } else {
            let pressure = self.fault(item);
            self.update_tiebreak(item);
            pressure
        }
    }

    // Run our Landlord implementation over the provided trace
    fn run(&mut self, trace: VecDeque<&'a Item>) {
        for request in trace.iter() {
            self.request(request);
            dbg!(&self);
        }
    }
}

// Converts our deserialized trace of strings into a trace of items
fn strings_to_items<'a>(trace: &'a TraceInfo) -> VecDeque<&'a Item> {
    let mut requests = VecDeque::new();
    let mut counter = 0;
    for request in trace.trace.iter() {
        for item in trace.items.iter() {
            if *item.get_label() == *request {
                requests.push_back(item);
                counter += 1;
            }
        }
    }
    if counter as usize != trace.trace.len() {
        panic!("Invalid trace generation");
    }
    requests
}

fn main() {
    let data: &str = &fs::read_to_string("items.toml").expect("Could not read file");
    let raw_trace: TraceInfo = toml::from_str(data).expect("Could not convert TOML file");
    let item_trace = strings_to_items(&raw_trace);
    let mut lru_landlord = Landlord::new(15, TiebreakingPolicy::Lru, HitPolicy::Lru);
    lru_landlord.run(item_trace);
}
