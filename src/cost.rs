use crate::Item;
use std::collections::VecDeque;

pub struct CostTracker {
    full_cost: VecDeque<u32>,
    suff_cost: VecDeque<u32>,
    scr: VecDeque<f32>,
}

impl CostTracker {
    fn new() -> Self {
        Self {
            full_cost: VecDeque::new(),
            suff_cost: VecDeque::new(),
            scr: VecDeque::new(),
        }
    }
    fn get_full_cost(&self, index: u32) -> u32 {
        *self
            .full_cost
            .get(index as usize)
            .expect("Full cost index out of bounds")
    }
    fn get_suff_cost(&self, index: u32) -> u32 {
        *self
            .suff_cost
            .get(index as usize)
            .expect("Suffix index out of bounds")
    }
    fn get_scr(&self) -> f32 {
        if self.full_cost.is_empty() {
            0.0
        } else {
            let suff_cost_sum = self.suff_cost.iter().sum::<u32>() as f32;
            let full_cost_sum = self.full_cost.iter().sum::<u32>() as f32;
            suff_cost_sum / full_cost_sum
        }
    }
    fn log(&mut self, item: &Item, full_fault: bool, suff_fault: bool) {
        if full_fault {
            self.full_cost.push_back(item.get_cost().0 as u32);
        } else {
            self.full_cost.push_back(0);
        }
        if suff_fault {
            self.suff_cost.push_back(item.get_cost().0 as u32);
        } else {
            self.suff_cost.push_back(0);
        }
    }
}
