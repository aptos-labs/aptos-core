// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::thread;

pub struct ConcurrentBlockingVector<T> {
    elems: Vec<(Sender<T>, Receiver<T>, Mutex<Arc<Option<T>>>)>
}

impl<T> ConcurrentBlockingVector<T> {
    pub fn new(num_elems: usize) -> Self {
        let mut elems = Vec::new();
        for _ in 0..num_elems {
            let (tx, rx) = unbounded();
            elems.push((tx, rx, Mutex::new(Arc::new(None))));
        }
        Self {
            elems
        }
    }

    pub fn get_elem(&self, idx: usize) -> Arc<Option<T>> {
        if self.elems.len() <= idx {
            panic!("index out of bound");
        }
        let lock = &mut *(self.elems[idx].2.lock().unwrap());
        if lock.is_none() {
            *lock = Arc::new(Some(self.elems[idx].1.recv().unwrap()));
        }
        lock.clone()
    }

    pub fn set_elem(&self, idx: usize, value: T) {
        if self.elems.len() <= idx {
            panic!("index out of bound");
        }
        let tx =  &self.elems[idx].0;
        tx.send(value).expect("Send failed");
    }
}

#[test]
fn test_concurrent_blocking_vector() {
    let v = Arc::new(ConcurrentBlockingVector::new(2));
    let v_clone = v.clone();
    let handle = thread::spawn(move || {
        assert_eq!(v_clone.get_elem(0), Arc::new(Some(1)));
        assert_eq!(v_clone.get_elem(0), Arc::new(Some(1)));
    });

    v.set_elem(0, 1);
    v.set_elem(1, 2);
    assert_eq!(v.get_elem(1), Arc::new(Some(2)));
    assert_eq!(v.get_elem(1), Arc::new(Some(2)));

    handle.join().expect("thread panicked");
}

/*
use std::ops::{Index};

#[derive(Debug, Clone)]
pub struct Vec3 {
    idx: Vec<usize>,
    e: Vec<(i32, i32)>,
}

impl<Idx> std::ops::Index<Idx> for Vec3
    where
        Idx: std::slice::SliceIndex<[usize]>, {
    type Output = Idx::Output;
    fn index(&self, i: Idx) -> &Self::Output  {
        let x = &self.idx[i];
        x
        //&self.e[idx].0
    }
}

fn func<T: std::fmt::Debug>(arr: &[T]) {
    println!("{}; {:?}", arr.len(), arr[0]);
}

#[test]
fn test_1() {
    let point = Vec3 { idx: vec![0, 1, 2], e: vec![(0, 1), (1, 2), (3, 4)] };
    println!("{}", point[2]);
    func(&point[0..]);
}


use std::ops::Index;

#[derive(Debug, Clone)]
pub struct Vec3 {
    e: [f32; 3],
    v: Vec<usize>,
}

impl Index<usize> for Vec3 {
    type Output = usize;
    fn index(&self, i: usize) -> & usize {
        &self.v[i]
    }
}

impl Vec3 {
    fn len(&self) -> usize {
        self.v.len()
    }
}

fn func<T: Index<usize>>(arr: &T) {
    println!("; {}", arr[0]);
}

#[test]
fn test_2() {
    let point = Vec3 { e: [0.0, 1.0, 3.0], v: vec![0, 1, 2, 3, 4, 5] };
    let z = point[4];
    println!("{}", z);
    func(&point);
}*/