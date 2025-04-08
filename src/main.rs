use logger::Tracker;
use ordered_float::OrderedFloat;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs;

pub mod logger;

// STRUCTS
// ----------------------------------------------------------------------------

// This is an enum to hold whether or not a request was a hit or a fault. This is used for both the
// full trace cache and the suffix. The data in the fault field is the pressure increase on that
// fault.
#[derive(Debug)]
pub enum RequestResult {
    Hit,
    Fault(f32),
}

// True means hit, false means fault.
pub enum RequestFullOrSuffix {
    Full(bool),
    Suff(bool),
}

// This is the data structure that serde will deserialize the items.toml file into. The items must
// be an exhaustive list of the costs and sizes of the items requested in our trace. Meanwhile, the
// trace is just a vector of strings where each string is an item's label.
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceInfo {
    items: Vec<Item>,
    trace: Vec<String>,
}

// These are the keys for our cache map. The label is the name of the item and it is also what we
// are comparing against while iterating through our trace. Cost and size are pretty self
// explanatory.
#[derive(Debug, PartialOrd, PartialEq, Eq, Ord, Serialize, Deserialize)]
pub struct Item {
    label: String,
    cost: u32,
    size: u32,
}

// Wrapper for the cache. The contents are stored as a BTreeMap where each key-value pair is an
// item and its associated normalized credit. These credits are stored as floats to allow for
// fractional credits. In particular, we wrap our floats in the OrderedFloat struct so that we can
// more easily find our minimum-credit element in cache when we must evict something. This is
// because the Ord trait is not implemented for f32 in Rust in compliance with IEEE 754. The policy
// is our hit policy which we check in a match statement later when we have a hit. The size of our
// cache is the number of cache lines available. Meanwhile, occupied is the number of cache lines
// that currently have items in them.
#[derive(Debug)]
struct Cache<'a> {
    contents: BTreeMap<&'a Item, OrderedFloat<f32>>,
    policy: HitPolicy,
    size: u32,
    occupied: u32,
}

// Wrapper for the tiebreaking order. This maintains a VecDeque which stores the order that, if
// there were to be a credit tie, which order we should evict our cache items. Items closer to the
// front will be evicted sooner, items at the back will be evicted later. Our policy is just our
// tiebreaking policy which we match against when we must evict something. Size and occupied are
// identical to what we had in the Cache struct.
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
pub enum HitPolicy {
    Lru,
    Fifo,
    Rand,
    Half,
}

// Tiebreaking policies. The first four have a default behavior implemented. Any after that will
// then defer the hit policy to whatever function you decide to assign to the enum. This can be
// anything and you don't need to keep the name 'custom'.
#[derive(Debug)]
#[warn(dead_code)]
pub enum TiebreakingPolicy {
    Lru,
    Fifo,
    Rand,
}

// The primary struct. We have a cache and a tiebreaker which implements our tiebreaking policy.
#[derive(Debug)]
pub struct Landlord<'a> {
    cache: Cache<'a>,
    tiebreaker: Tiebreaker<'a>,
}

// IMPLEMENTATING STRUCTS
// -----------------------------------------------------------------------------

impl Item {
    // Getters.
    pub fn get_label(&self) -> &String {
        &self.label
    }
    pub fn get_cost(&self) -> OrderedFloat<f32> {
        let float_cost = self.cost as f32;
        OrderedFloat(float_cost)
    }
    pub fn get_size(&self) -> u32 {
        self.size
    }
}

impl<'a> Landlord<'a> {
    // Creates a new Landlord instance with the specified size, tiebreaking policy and hit policy.
    pub fn new(size: u32, tiebreak_policy: TiebreakingPolicy, hit_policy: HitPolicy) -> Self {
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

    // Utility function to get the normalized credit of an item as an ordered float.
    pub fn norm_credit(item: (&&'a Item, &OrderedFloat<f32>)) -> OrderedFloat<f32> {
        item.1 / OrderedFloat(item.0.get_size() as f32)
    }

    // Gets the tiebreaking index of a particular item if it exists in the tiebreaking vector.
    fn get_tiebreaker_index(&self, item: &'a Item) -> Option<usize> {
        self.tiebreaker.order.iter().position(|n| *n == item)
    }

    // Takes care of cleaning up our tiebreaking order by removing a particular item once it gets
    // evicted.
    fn manage_tiebreak(&mut self, item: &Item) {
        if let Some(index) = self.get_tiebreaker_index(item) {
            self.tiebreaker.order.remove(index);
            self.tiebreaker.occupied -= item.get_size();
        }
    }

    // Update our tiebreaking order on any request.
    fn update_tiebreak(&mut self, item: &'a Item) {
        // Get the updated item's index
        let index = self.get_tiebreaker_index(item);
        // If that item is in our tiebreaking order, we remove it from the order first.
        if let Some(loc) = index {
            self.tiebreaker.order.remove(loc);
        }
        match self.tiebreaker.policy {
            // Push the item to the back of the order
            TiebreakingPolicy::Lru => {
                self.tiebreaker.order.push_back(item);
            }
            TiebreakingPolicy::Fifo => {
                // If the item is in cache, place it back at its original index. If it is not in
                // cache, we push it to the back of the order.
                match index {
                    Some(loc) => self.tiebreaker.order.insert(loc, item),
                    None => self.tiebreaker.order.push_back(item),
                }
            }
            // Insert the item into a random slot in the tiebreaking order
            TiebreakingPolicy::Rand => {
                let k = self.tiebreaker.size;
                let new_index = rand::rng().random_range(0..k - 1) as usize;
                self.tiebreaker.order.insert(new_index, item);
            }
        }
    }

    // Function called whenever Landlord hits on an item
    fn hit(&mut self, label: &'a Item) {
        // We first get the item's old credit.
        let cred = match self.cache.contents.get_mut(label) {
            Some(val) => val,
            None => panic!("Could not find hit item"),
        };

        // Getting rid of the bad NaN case
        if cred.0.is_nan() {
            panic!("NaN credit found");
        }

        // Refresh the requested item's credit according to hit policy.
        let new_cred = match &self.cache.policy {
            // Refreshes it to its full cost.
            HitPolicy::Lru => label.get_cost(),
            // Does not refresh at all.
            HitPolicy::Fifo => *cred,
            // Refreshes to a random value between current credit and cost.
            HitPolicy::Rand => OrderedFloat(rand::rng().random_range(cred.0..label.get_cost().0)),
            // Refreshes it to half its current credit.
            HitPolicy::Half => (label.get_cost() - *cred) / 2.0,
        };

        // Assigning our new credit to the item.
        *cred = new_cred;
    }

    // Finding the element we want to evict in the case of a tie
    fn tiebreak(&mut self, zeros: Vec<&'a Item>) -> &'a Item {
        // If we only have one item that has 0 credit, we tiebreak according to that item.
        if zeros.len() == 1 {
            return zeros[0];
        }
        // Otherwise, we iterate through our tiebreaking order from front to back, checking if each
        // item we find is in our zeros vector. If we find a candidate in our zeros vector, then
        // that is the element soonest on the tiebreaking order with 0 credit and so we return it.
        for cand in self.tiebreaker.order.iter() {
            if zeros.contains(cand) {
                return cand;
            }
        }
        // If we do not find any items in the zeros vector that are in our tiebreaking order, we
        // have somehow mismanaged our tiebreaking order and we throw an error.
        panic!("Tiebreaking order mismanagement");
    }

    // Evicting an element if we do not have enough space for it
    fn evict(&mut self, size: u32) -> OrderedFloat<f32> {
        // Getting our return value
        let mut pressure = OrderedFloat(0.0);

        // Base case: we have enough space for our item and so we simply return 0
        // because our pressure does not increase when we bring an item into cache.
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
        // Increasing the pressure in relation to the credit of the minimum credit item we just
        // evicted.
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

        // Removing the item it picks from our cache and decreasing the occupied space by the size
        // of the item we just evicted.
        self.cache.contents.remove(evicted);
        self.cache.occupied -= evicted.get_size();

        // Returning our pressure at the end
        pressure + self.evict(size)
    }

    // The function called whenever the Landlord implementation faults on a request.
    fn fault(&mut self, item: &'a Item) -> OrderedFloat<f32> {
        // If the cache has too many items, throw an error.
        if self.cache.occupied > self.cache.size {
            panic!("Cache is overfull");
        }
        // If the cache has empty space, just add the item!
        else if self.cache.occupied + item.get_size() <= self.cache.size {
            // We insert the item into cache at full cost.
            self.cache.contents.insert(item, item.get_cost());
            // We increase the occupied cache/tiebreaker space by our item's size.
            self.cache.occupied += item.get_size();
            self.tiebreaker.occupied += item.get_size();
            // We return pressure 0 because we did not have to evict anything.
            OrderedFloat(0.0)
        }
        // Otherwise, the cache is full and we need evict something
        else {
            // We get the item's size.
            let size = item.get_size();
            // We allow our recursive eviction function to evict items until we have enough space,
            // thereby also getting our pressure.
            let pressure = self.evict(size);
            // We insert our item into cache at full credit.
            self.cache.contents.insert(item, item.get_cost());
            pressure
        }
    }

    // Handle our request
    pub fn request(&mut self, item: &'a Item) -> RequestResult {
        // If our cache contains the requested item, we have a hit!
        if self.cache.contents.contains_key(&item) {
            // We hit on that item, updating its credit according to hit policy.
            self.hit(item);
            // We update our tiebreaking order.
            self.update_tiebreak(item);
            // We return a request result of a hit
            RequestResult::Hit
        }
        // Otherwise, we have a fault :(.
        else {
            // We get the pressure as a result of that fault.
            let pressure = self.fault(item);
            // We update our tiebreaking ordering no matter what.
            self.update_tiebreak(item);
            // We wrap our pressure in a request result of a fault.
            RequestResult::Fault(*pressure)
        }
    }

    // Run our Landlord implementation over the provided trace
    pub fn run(
        trace: VecDeque<&'a Item>,
        suffix_start: u32,
        mut s: Landlord<'a>,
        mut f: Landlord<'a>,
        mut tracker: Tracker,
    ) {
        // For each request in our trace
        for (i, request) in trace.iter().enumerate() {
            // We issue that request to the full trace cache because that one is going to have to
            // service that request no matter what.
            let res = f.request(request);
            // From there, we match on the result
            match res {
                // If it is a hit, we log that the request was a hit with our cost logger and
                // pressure logger.
                RequestResult::Hit => {
                    tracker.log_cost(request, RequestFullOrSuffix::Full(true));
                    tracker.log_pres(0.0, RequestFullOrSuffix::Full(true));
                }
                // If the request was a hi, we log_cost that the full trace cache paid that item's cost
                // and that the pressure went up by whatever amount we wrapped in RequestResult.
                RequestResult::Fault(pressure) => {
                    tracker.log_cost(request, RequestFullOrSuffix::Full(false));
                    tracker.log_pres(pressure, RequestFullOrSuffix::Full(false));
                }
            }
            // If we are not in the suffix yet, we are going to say that S simply paid no cost.
            // This is relevant for when we calculate individual suffix competitive ratios later.
            if i < suffix_start as usize {
                tracker.log_cost(request, RequestFullOrSuffix::Suff(true));
                tracker.log_pres(0.0, RequestFullOrSuffix::Suff(true));
                continue;
            }
            let res = s.request(request);
            // We perform an identical match statement as above but instead we just label that the
            // request results are for suff instead.
            match res {
                RequestResult::Hit => {
                    tracker.log_cost(request, RequestFullOrSuffix::Suff(true));
                    tracker.log_pres(0.0, RequestFullOrSuffix::Suff(true));
                }
                RequestResult::Fault(pressure) => {
                    tracker.log_cost(request, RequestFullOrSuffix::Suff(false));
                    tracker.log_pres(pressure, RequestFullOrSuffix::Suff(false));
                }
            }
        }
    }
}

// Converts our deserialized trace of strings into a trace of items
fn strings_to_items(trace: &TraceInfo) -> VecDeque<&Item> {
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
    let s = Landlord::new(15, TiebreakingPolicy::Lru, HitPolicy::Lru);
    let f = Landlord::new(15, TiebreakingPolicy::Lru, HitPolicy::Lru);
    let tracker = Tracker::new(&item_trace);
    Landlord::run(item_trace, 2, s, f, tracker);
}
