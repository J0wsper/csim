use crate::Item;
use std::collections::VecDeque;

pub struct PressureTracker {
    full_pressure: VecDeque<u32>,
    suffix_pressure: VecDeque<u32>,
}

impl PressureTracker {
    pub fn new() -> Self {
        Self {
            full_pressure: VecDeque::new(),
            suffix_pressure: VecDeque::new(),
        }
    }
}
