// Copyright © Aptos Foundation

use std::marker::PhantomData;
use rayon::iter::ParallelExtend;
use rayon::prelude::ParallelIterator;

pub struct PartitionWrapper<'a, T: Send, C: ParallelExtend<T>>{
    underlying: &'a C,
    phantom: PhantomData<T>,
}

pub trait PartitionTo: ParallelIterator {
    fn partition_to<A, B, P>(self, predicate: P, a: &A, b: &B) -> (A, B)
    where
        A: Default + Send + ParallelExtend<Self::Item>,
        B: Default + Send + ParallelExtend<Self::Item>,
        P: Fn(&Self::Item) -> bool + Sync + Send
    {
        let a_wrapper = PartitionWrapper {
            underlying: a,
            phantom: PhantomData,
        };
        let b_wrapper = PartitionWrapper {
            underlying: b,
            phantom: PhantomData,
        };


    }
}
