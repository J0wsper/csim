use crate::Item;
use crate::RequestFullOrSuffix;
use std::collections::{BTreeMap, VecDeque};

pub struct IndScr<'a> {
    full_costs: BTreeMap<&'a Item, VecDeque<u32>>,
    suff_costs: BTreeMap<&'a Item, VecDeque<u32>>,
}

impl<'a> IndScr<'a> {
    fn new(trace: &VecDeque<&'a Item>) -> Self {
        Self {
            full_costs: {
                let mut map = BTreeMap::new();
                for request in trace {
                    if !map.contains_key(request) {
                        map.insert(*request, VecDeque::new());
                    }
                }
                map
            },
            suff_costs: {
                let mut map = BTreeMap::new();
                for request in trace {
                    if !map.contains_key(request) {
                        map.insert(*request, VecDeque::new());
                    }
                }
                map
            },
        }
    }
}

pub struct Tracker<'a> {
    full_cost: VecDeque<u32>,
    full_pres: VecDeque<f32>,
    suff_cost: VecDeque<u32>,
    suff_pres: VecDeque<f32>,
    ind_scr: IndScr<'a>,
}

impl<'a> Tracker<'a> {
    pub fn new(trace: &VecDeque<&'a Item>) -> Self {
        Self {
            full_cost: VecDeque::new(),
            full_pres: VecDeque::new(),
            suff_cost: VecDeque::new(),
            suff_pres: VecDeque::new(),
            ind_scr: IndScr::new(trace),
        }
    }
    pub fn get_full_cost(&self, index: u32) -> u32 {
        *self
            .full_cost
            .get(index as usize)
            .expect("Full cost index out of bounds")
    }
    pub fn get_full_cost_range(&self, index: u32) -> u32 {
        self.full_cost.range(0..index as usize).sum::<u32>()
    }
    pub fn get_suff_cost_range(&self, index: u32) -> u32 {
        self.suff_cost.range(0..index as usize).sum::<u32>()
    }
    pub fn get_suff_cost(&self, index: u32) -> u32 {
        *self
            .suff_cost
            .get(index as usize)
            .expect("Suffix index out of bounds")
    }
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
    pub fn get_ind_scr(&self, index: u32, item: &'a Item) -> f32 {
        let item_suff_costs = self
            .ind_scr
            .full_costs
            .get(item)
            .expect("Could not find item in full costs for indindividual SCR logging")
            .range(0..index as usize)
            .sum::<u32>();
        let item_full_costs = self
            .ind_scr
            .full_costs
            .get(item)
            .expect("Could not find item in full costs for indindividual SCR logging")
            .range(0..index as usize)
            .sum::<u32>();
        if item_full_costs == 0 {
            return 0.0;
        }
        item_suff_costs as f32 / item_full_costs as f32
    }
    pub fn log_cost(&mut self, item: &Item, request_type: RequestFullOrSuffix) {
        let cost = item.get_cost().0 as u32;
        match request_type {
            RequestFullOrSuffix::Full(is_hit) => {
                let item_costs = self
                    .ind_scr
                    .full_costs
                    .get_mut(item)
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
                    .get_mut(item)
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
    // Much simpler than the cost logging.
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
