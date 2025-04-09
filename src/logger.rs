use crate::Item;
use crate::RequestFullOrSuffix;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};

/// Struct that stores the individual suffix competitive ratio of our items.
#[derive(Debug)]
pub struct IndScr {
    // We store the labels of items instead of references to the items for ease of deserialization.
    full_costs: BTreeMap<String, VecDeque<u32>>,
    suff_costs: BTreeMap<String, VecDeque<u32>>,
}

impl IndScr {
    fn new(trace: &VecDeque<&Item>) -> Self {
        Self {
            full_costs: {
                let mut map = BTreeMap::new();
                for request in trace {
                    if !map.contains_key(&request.label) {
                        map.insert(request.label.clone(), VecDeque::new());
                    }
                }
                map
            },
            suff_costs: {
                let mut map = BTreeMap::new();
                for request in trace {
                    if !map.contains_key(&request.label) {
                        map.insert(request.label.clone(), VecDeque::new());
                    }
                }
                map
            },
        }
    }
}

#[derive(Debug)]
pub struct Tracker {
    full_cost: VecDeque<u32>,
    suff_cost: VecDeque<u32>,
    full_pres: VecDeque<f32>,
    suff_pres: VecDeque<f32>,
    ind_scr: IndScr,
}

impl Tracker {
    // Creates a new tracker instance
    pub fn new(trace: &VecDeque<&Item>) -> Self {
        Self {
            full_cost: VecDeque::new(),
            full_pres: VecDeque::new(),
            suff_cost: VecDeque::new(),
            suff_pres: VecDeque::new(),
            ind_scr: IndScr::new(trace),
        }
    }
    /// Gets the cost that the full cache paid at a particular point in the trace.
    pub fn get_full_cost(&self, index: u32) -> u32 {
        *self
            .full_cost
            .get(index as usize)
            .expect("Full cost index out of bounds")
    }
    /// Gets the cost that the full cache paid from the start of the trace to the specified index.
    pub fn get_full_cost_range(&self, index: u32) -> u32 {
        self.full_cost.range(0..index as usize).sum::<u32>()
    }
    /// Gets the cost that the suffix cache paid from the start of the trace to the specified index.
    pub fn get_suff_cost_range(&self, index: u32) -> u32 {
        self.suff_cost.range(0..index as usize).sum::<u32>()
    }
    /// Gets the cost that the suffix cache paid at a particular point in the trace.
    pub fn get_suff_cost(&self, index: u32) -> u32 {
        *self
            .suff_cost
            .get(index as usize)
            .expect("Suffix index out of bounds")
    }
    /// Gets the suffix competitive ratio at a particular index.
    pub fn get_scr(&self, index: u32) -> f32 {
        if self.full_cost.is_empty() {
            0.0
        } else {
            let proper_index = index as usize;
            let suff_cost_sum = self.suff_cost.range(0..proper_index).sum::<u32>();
            let full_cost_sum = self.full_cost.range(0..proper_index).sum::<u32>();
            suff_cost_sum as f32 / full_cost_sum as f32
        }
    }
    /// Gets the individual suffix competitive ratio for the specified item at a particular index.
    pub fn get_ind_scr(&self, index: u32, item: &Item) -> f32 {
        let item_suff_costs = self
            .ind_scr
            .full_costs
            .get(&item.label)
            .expect("Could not find item in full costs for indindividual SCR logging")
            .range(0..index as usize)
            .sum::<u32>();
        let item_full_costs = self
            .ind_scr
            .full_costs
            .get(&item.label)
            .expect("Could not find item in full costs for indindividual SCR logging")
            .range(0..index as usize)
            .sum::<u32>();
        if item_full_costs == 0 {
            return 0.0;
        }
        item_suff_costs as f32 / item_full_costs as f32
    }
    /// Logs the cost of a particular item at a particular request
    pub fn log_cost(&mut self, item: &Item, request_type: RequestFullOrSuffix) {
        let cost = item.get_cost().0 as u32;
        match request_type {
            RequestFullOrSuffix::Full(is_hit) => {
                let item_costs = self
                    .ind_scr
                    .full_costs
                    .get_mut(&item.label)
                    .expect("Could not find item in full costs for individual SCR logging");
                if is_hit {
                    self.full_cost.push_back(0);
                    item_costs.push_back(0);
                } else {
                    self.full_cost.push_back(cost);
                    item_costs.push_back(cost);
                }
            }
            RequestFullOrSuffix::Suff(is_hit) => {
                let item_costs = self
                    .ind_scr
                    .suff_costs
                    .get_mut(&item.label)
                    .expect("Could not find item in full costs for individual SCR logging");
                if is_hit {
                    self.suff_cost.push_back(0);
                    item_costs.push_back(0);
                } else {
                    self.suff_cost.push_back(cost);
                    item_costs.push_back(cost);
                }
            }
        }
    }
    // Logging for pressure. Much simpler than the cost logging because we do not have to be
    // worried about keeping track of indiviual suffix competitive ratios.
    pub fn log_pres(&mut self, pressure: f32, request_type: RequestFullOrSuffix) {
        match request_type {
            RequestFullOrSuffix::Full(is_hit) => {
                if is_hit {
                    self.full_pres.push_back(0.0);
                } else {
                    self.full_pres.push_back(pressure);
                }
            }
            RequestFullOrSuffix::Suff(is_hit) => {
                if is_hit {
                    self.suff_pres.push_back(0.0);
                } else {
                    self.suff_pres.push_back(pressure);
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PrettyTracker {
    full_costs: VecDeque<u32>,
    suff_costs: VecDeque<u32>,
    full_pres: VecDeque<f32>,
    suff_pres: VecDeque<f32>,
    ind_scr: BTreeMap<String, f32>,
}

impl PrettyTracker {
    pub fn new(tracker: Tracker) -> Self {
        Self {
            full_costs: tracker.full_cost,
            suff_costs: tracker.suff_cost,
            full_pres: tracker.full_pres,
            suff_pres: tracker.suff_pres,
            ind_scr: {
                let mut ind_scrs = BTreeMap::new();
                for label in tracker.ind_scr.suff_costs.iter() {
                    let full_costs = tracker.ind_scr.full_costs.get(label.0).unwrap();
                    let full_costs_sum: u32 = full_costs.iter().sum();
                    if full_costs_sum == 0 {
                        ind_scrs.insert(label.0.to_string(), 0.0);
                    } else {
                        let suff_costs_sum: u32 = label.1.iter().sum();
                        let ind_scr = suff_costs_sum as f32 / full_costs_sum as f32;
                        ind_scrs.insert(label.0.to_string(), ind_scr);
                    }
                }
                ind_scrs
            },
        }
    }
    pub fn ser_logger(&self) -> String {
        toml::to_string(self).unwrap()
    }
}
