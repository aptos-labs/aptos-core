// Copyright © Aptos Foundation

use std::collections::BinaryHeap;
use itertools::Itertools;

pub fn assign_tasks_to_workers(mut tasks: Vec<usize>, num_workers: usize) -> (usize, Vec<usize>) {
    assert!(num_workers >= 1);
    tasks.sort_by(|a, b| b.cmp(a));
    let mut worker_prio_heap: BinaryHeap<(usize, usize)> =
        BinaryHeap::from((0..num_workers).map(|wid| (usize::MAX, wid)).collect_vec());
    let mut worker_ids_by_tid = Vec::with_capacity(tasks.len());
    for task in tasks.iter() {
        let (availability, worker_id) = worker_prio_heap.pop().unwrap();
        worker_ids_by_tid.push(worker_id);
        let new_availability = availability - task;
        worker_prio_heap.push((new_availability, worker_id));
    }
    let longest_pole = worker_prio_heap
        .into_iter()
        .map(|(a, _i)| usize::MAX - a)
        .max()
        .unwrap();
    (longest_pole, worker_ids_by_tid)
}

#[test]
fn test_assign_tasks_to_workers() {
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 1);
    assert_eq!(15, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 2);
    assert_eq!(8, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 3);
    assert_eq!(5, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 4);
    assert_eq!(5, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 5);
    assert_eq!(5, actual);
}
