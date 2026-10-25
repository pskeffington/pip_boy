//! A bounded in-memory FIFO with explicit overflow accounting.
//! The buffer never spills to disk or opens a network connection.

use std::collections::VecDeque;

#[derive(Debug)]
pub struct SampleBuffer<T> {
    capacity: usize,
    dropped: u64,
    queue: VecDeque<T>,
}

impl<T> SampleBuffer<T> {
    pub fn new(capacity: usize) -> Result<Self, String> {
        if !(1..=65_536).contains(&capacity) {
            return Err("capacity must be within 1..=65536".into());
        }
        Ok(Self {
            capacity,
            dropped: 0,
            queue: VecDeque::with_capacity(capacity),
        })
    }

    pub fn push(&mut self, value: T) {
        if self.queue.len() == self.capacity {
            self.queue.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.queue.push_back(value);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.queue.iter()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_newest_items_without_unbounded_growth() {
        let mut queue = SampleBuffer::new(2).unwrap();
        for value in 1..=5 {
            queue.push(value);
        }
        assert_eq!(queue.iter().copied().collect::<Vec<_>>(), vec![4, 5]);
        assert_eq!(queue.dropped(), 3);
        queue.clear();
        assert!(queue.is_empty());
        assert_eq!(queue.dropped(), 3);
    }

    #[test]
    fn refuses_zero_or_unbounded_capacity() {
        assert!(SampleBuffer::<u8>::new(0).is_err());
        assert!(SampleBuffer::<u8>::new(65_537).is_err());
    }
}
