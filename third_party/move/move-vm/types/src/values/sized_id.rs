// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

/// Represents a unique 32-bit identifier used for values which also stores their
/// serialized size (u32::MAX at most). Can be stored as a single 64-bit unsigned
/// integer.
/// TODO[agg_v2](cleanup): consolidate DelayedFiledID and this implementation!
#[derive(Debug, Copy, Clone)]
pub struct SizedID {
    // Unique identifier for a value.
    id: u32,
    // Exact number of bytes a serialized value will take.
    serialized_size: u32,
}

const NUM_BITS_FOR_SERIALIZED_SIZE: usize = 32;

impl SizedID {
    pub fn new(id: u32, serialized_size: u32) -> Self {
        Self {
            id,
            serialized_size,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn serialized_size(&self) -> u32 {
        self.serialized_size
    }
}

impl From<u64> for SizedID {
    fn from(value: u64) -> Self {
        let id = value >> NUM_BITS_FOR_SERIALIZED_SIZE;
        let serialized_size = value & ((1u64 << NUM_BITS_FOR_SERIALIZED_SIZE) - 1);
        Self {
            id: id as u32,
            serialized_size: serialized_size as u32,
        }
    }
}

impl From<SizedID> for u64 {
    fn from(sized_id: SizedID) -> Self {
        let id = (sized_id.id as u64) << NUM_BITS_FOR_SERIALIZED_SIZE;
        id | sized_id.serialized_size as u64
    }
}

#[cfg(test)]
mod test {
    use crate::values::SizedID;

    macro_rules! assert_sized_id_roundtrip {
        ($start_value:expr) => {
            let sized_id: SizedID = $start_value.into();
            let end_value: u64 = sized_id.into();
            assert_eq!($start_value, end_value)
        };
    }

    #[test]
    fn test_sized_id_from_u64() {
        assert_sized_id_roundtrip!(0u64);
        assert_sized_id_roundtrip!(123456789u64);
        assert_sized_id_roundtrip!(u64::MAX);
    }
}
