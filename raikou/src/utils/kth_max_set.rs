use std::collections::BTreeSet;

#[derive(Clone)]
pub struct KthMaxSet<T> {
    max_k: BTreeSet<T>,
    rest: BTreeSet<T>,
    k: usize,
}

impl<T> KthMaxSet<T>
where
    T: Ord,
{
    pub fn new(k: usize) -> Self {
        assert!(k >= 1);
        Self {
            max_k: BTreeSet::new(),
            rest: BTreeSet::new(),
            k,
        }
    }

    pub fn kth_max(&self) -> Option<&T> {
        if self.max_k.len() == self.k {
            self.max_k.first()
        } else {
            None
        }
    }

    pub fn k_max_set(&self) -> &BTreeSet<T> {
        &self.max_k
    }

    pub fn len(&self) -> usize {
        self.max_k.len() + self.rest.len()
    }

    pub fn insert(&mut self, value: T) {
        match self.kth_max() {
            Some(kth_max) if value < *kth_max => {
                self.rest.insert(value);
            },
            _ => {
                self.max_k.insert(value);
                if self.max_k.len() > self.k {
                    self.rest.insert(self.max_k.pop_last().unwrap());
                }
            },
        }
    }
}
