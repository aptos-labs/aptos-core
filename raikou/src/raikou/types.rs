use crate::framework::NodeId;
use std::marker::PhantomData;

pub type Round = i64; // Round number.
pub type BatchSN = i64; // Sequence number of a batch.
pub type Prefix = usize;

pub type Txn = ();
