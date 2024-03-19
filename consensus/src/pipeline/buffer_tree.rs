use crate::pipeline::hashable::Hashable;
use aptos_crypto::HashValue;
use std::collections::HashMap;

pub struct LinkedItem<T: Hashable> {
    // use option so we don't need T to be cloneable
    elem: Option<T>,
    // index is for find_element_by_key to have a starting position (similar to find_element)
    index: u64,
    next: Option<HashValue>,
}

pub type Cursor = Option<HashValue>;

/// BufferTree implements a tree structure of buffer items
/// It supports push_back, pop_front, and lookup by HashValue
pub struct BufferTree<T: Hashable> {
    map: HashMap<HashValue, LinkedItem<T>>,
    tails: Vec<Cursor>, // Stores tails for each parent node
    count: u64,
    root: Cursor, // Points to the root of the tree
}

impl<T: Hashable> BufferTree<T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            tails: Vec::new(),
            count: 0,
            root: None,
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn root_cursor(&self) -> &Cursor {
        &self.root
    }

    pub fn push_back(&mut self, elem: T, parent: Cursor) {
        self.count = self.count.checked_add(1).unwrap();
        let t_hash = elem.hash();
        self.map.insert(t_hash, LinkedItem {
            elem: Some(elem),
            index: self.count,
            next: None,
        });
        let parent_index = parent.as_ref().map_or(0, |_| self.map.get(&parent.unwrap()).unwrap().index as usize);
        if parent_index >= self.tails.len() {
            self.tails.resize(parent_index + 1, None);
        }
        if let Some(tail) = self.tails[parent_index] {
            self.map.get_mut(&tail).unwrap().next = Some(t_hash);
        } else {
            self.tails[parent_index] = Some(t_hash);
        }
        if self.root.is_none() {
            self.root = Some(t_hash);
        }
    }

    pub fn pop_front(&mut self, parent: Cursor) -> Option<T> {
        let parent_index = parent.as_ref().map_or(0, |_| self.map.get(&parent.unwrap()).unwrap().index as usize);
        if parent_index < self.tails.len() {
            let mut item = self.map.remove(&self.tails[parent_index].unwrap()).unwrap();
            let elem = item.elem.take();
            self.tails[parent_index] = item.next;
            if self.root == Some(self.tails[parent_index].unwrap()) {
                self.root = None; // Empty
            }
            elem
        } else {
            None
        }
    }

    // utils - assuming item is not None
    pub fn get_next(&self, cursor: &Cursor) -> Cursor {
        self.map.get(cursor.as_ref().unwrap()).unwrap().next
    }

    pub fn get(&self, cursor: &Cursor) -> &T {
        self.map
            .get(cursor.as_ref().unwrap())
            .unwrap()
            .elem
            .as_ref()
            .unwrap()
    }

    pub fn set(&mut self, cursor: &Cursor, new_val: T) {
        self.map
            .get_mut(cursor.as_ref().unwrap())
            .unwrap()
            .elem
            .replace(new_val);
    }

    pub fn take(&mut self, cursor: &Cursor) -> T {
        self.map
            .get_mut(cursor.as_ref().unwrap())
            .unwrap()
            .elem
            .take()
            .unwrap()
    }

    pub fn exist(&self, cursor: &Cursor) -> bool {
        cursor.map_or(false, |key| self.map.contains_key(&key))
    }

    /// find_elem returns the first item non-prior to `cursor` that compare(item) is true
    /// if no such item exists, the function returns None
    pub fn find_elem_from<F: Fn(&T) -> bool>(&self, cursor: Cursor, compare: F) -> Cursor {
        let mut current = cursor;
        if !self.exist(&cursor) {
            return None;
        }
        while current.is_some() {
            if compare(self.get(&current)) {
                return current;
            }
            current = self.get_next(&current);
        }
        None
    }

    /// we make sure that the element found by the key is after `cursor`
    /// if `cursor` is None, this function returns None (same as find_elem)
    pub fn find_elem_by_key(&self, cursor: Cursor, key: HashValue) -> Cursor {
        let cursor_order = self.map.get(cursor.as_ref()?).unwrap().index;
        let item = self.map.get(&key)?;
        if item.index >= cursor_order {
            Some(key)
        } else {
            None
        }
    }
}