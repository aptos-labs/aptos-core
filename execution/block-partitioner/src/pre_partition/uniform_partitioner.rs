// Copyright © Aptos Foundation

#[cfg(test)]
use crate::test_utils::P2PBlockGenerator;
use crate::{pre_partition::PrePartitioner, v2::types::PrePartitionedTxnIdx};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
#[cfg(test)]
use rand::thread_rng;

/// Evenly divide txns. Example: processing txns 0..11 results in [[0,1,2,3],[4,5,6,7],[8,9,10]].
pub struct UniformPartitioner {}

impl PrePartitioner for UniformPartitioner {
    fn pre_partition(
        &self,
        transactions: &[AnalyzedTransaction],
        num_shards: usize,
    ) -> Vec<Vec<PrePartitionedTxnIdx>> {
        let num_items = transactions.len();
        let num_chunks = num_shards;
        let num_chunks_with_overflow = num_items % num_chunks;
        let chunk_size = num_items / num_chunks;
        let mut ret = Vec::with_capacity(num_chunks);
        let mut next_chunk_start = 0;
        for chunk_id in 0..num_chunks {
            let extra = if chunk_id < num_chunks_with_overflow {
                1
            } else {
                0
            };
            let next_chunk_end = next_chunk_start + chunk_size + extra;
            let chunk: Vec<usize> = (next_chunk_start..next_chunk_end).collect();
            next_chunk_start = next_chunk_end;
            ret.push(chunk);
        }
        ret
    }
}

#[test]
fn test_uniform_partitioner() {
    let block_gen = P2PBlockGenerator::new(10);
    let mut rng = thread_rng();
    let txns = block_gen.rand_block(&mut rng, 18);
    let partitioner = UniformPartitioner {};
    let actual = partitioner.pre_partition(txns.as_slice(), 5);
    assert_eq!(
        vec![4, 4, 4, 3, 3],
        actual.iter().map(|v| v.len()).collect::<Vec<usize>>()
    );
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());

    let actual = partitioner.pre_partition(txns.as_slice(), 3);
    assert_eq!(
        vec![6, 6, 6],
        actual.iter().map(|v| v.len()).collect::<Vec<usize>>()
    );
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());
}
