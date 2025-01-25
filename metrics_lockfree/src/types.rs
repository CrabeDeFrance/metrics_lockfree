use ahash::{HashMap, HashMapExt};
//use std::collections::HashMap;

pub enum MetricType {
    Counter,
    Gauge,
    CounterWithTags,
}

#[derive(Debug)]
pub struct Tags {
    tags: HashMap<Vec<(String, String)>, usize>,
    next_id: usize,
    max_id: usize,
}

impl Tags {
    pub fn new(max_id: usize) -> Self {
        Self {
            tags: HashMap::new(),
            next_id: 1, // id 0 is for tagless value
            max_id,
        }
    }

    pub fn get(&self, tags: &[(String, String)]) -> Option<usize> {
        if tags.is_empty() {
            return Some(0);
        }
        self.tags.get(tags).copied()
    }

    pub fn insert(&mut self, tags: &[(String, String)]) -> Option<usize> {
        // firstly, check if we already have an id for the tags
        // (due to read/write concurrency issues, it is possible to miss an insert by another thread)
        if let Some(id) = self.tags.get(tags) {
            return Some(*id);
        }

        // stop here if we overflow
        if self.next_id >= self.max_id {
            return None;
        }

        // all good, lets reserve a new id and insert/return it
        let id = self.next_id;
        self.tags.insert(tags.to_vec(), id);
        self.next_id += 1;
        Some(id)
    }

    pub fn tags(&self) -> &HashMap<Vec<(String, String)>, usize> {
        &self.tags
    }
}
