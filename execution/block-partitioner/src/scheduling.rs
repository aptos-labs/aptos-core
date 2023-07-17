// Copyright © Aptos Foundation

use std::collections::BinaryHeap;
use itertools::Itertools;
use crate::simple_partitioner::SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS;

/// Assign a list of tasks to a set of workers in a way that minimize the longest pole.
pub fn assign_tasks_to_workers(tasks: &Vec<u64>, num_workers: usize) -> (u64, Vec<usize>) {
    assert!(num_workers >= 1);
    let num_tasks = tasks.len();
    let mut cost_tid_pairs: Vec<(u64, usize)> = tasks.iter().enumerate().map(|(tid, cost)| (*cost, tid)).collect();
    cost_tid_pairs.sort_by(|a, b| b.cmp(a));
    let mut worker_prio_heap: BinaryHeap<(u64, usize)> =
        BinaryHeap::from((0..num_workers).map(|wid| (u64::MAX, wid)).collect_vec());
    let mut worker_ids_by_tid = vec![usize::MAX; num_tasks];
    for (cost, tid) in cost_tid_pairs.into_iter() {
        let (availability, worker_id) = worker_prio_heap.pop().unwrap();
        worker_ids_by_tid[tid] = worker_id;
        let new_availability = availability - cost;
        worker_prio_heap.push((new_availability, worker_id));
    }
    let longest_pole = worker_prio_heap
        .into_iter()
        .map(|(a, _i)| u64::MAX - a)
        .max()
        .unwrap();
    (longest_pole, worker_ids_by_tid)
}

#[test]
fn test_assign_tasks_to_workers() {
    let (actual, assignment) = assign_tasks_to_workers(&vec![1, 2, 3, 4, 5], 1);
    assert_eq!(15, actual);
    println!("{:?}", assignment);
    let (actual, assignment) = assign_tasks_to_workers(&vec![1, 2, 3, 4, 5], 2);
    assert_eq!(8, actual);
    println!("{:?}", assignment);
    let (actual, assignment) = assign_tasks_to_workers(&vec![1, 2, 3, 4, 5], 3);
    assert_eq!(5, actual);
    println!("{:?}", assignment);
    let (actual, assignment) = assign_tasks_to_workers(&vec![1, 2, 3, 4, 5], 4);
    assert_eq!(5, actual);
    println!("{:?}", assignment);
    let (actual, assignment) = assign_tasks_to_workers(&vec![1, 2, 3, 4, 5], 5);
    assert_eq!(5, actual);
    println!("{:?}", assignment);
}
