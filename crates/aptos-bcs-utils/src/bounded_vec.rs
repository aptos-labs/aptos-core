// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use serde::{
    de::{Error, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, marker::PhantomData, ops::Deref};

/// A vector that rejects an oversized sequence before deserializing its elements.
///
/// The serialized representation is identical to `Vec<T>`. In particular, BCS
/// provides the declared sequence length through `SeqAccess::size_hint()`, which
/// lets this type reject an oversized vector before any attacker-controlled
/// element deserialization runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedVec<T, const MAX_LEN: usize>(Vec<T>);

impl<T, const MAX_LEN: usize> BoundedVec<T, MAX_LEN> {
    pub fn new(inner: Vec<T>) -> anyhow::Result<Self> {
        anyhow::ensure!(
            inner.len() <= MAX_LEN,
            "vector length {} exceeds maximum {}",
            inner.len(),
            MAX_LEN
        );
        Ok(Self(inner))
    }

    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T, const MAX_LEN: usize> Deref for BoundedVec<T, MAX_LEN> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Serialize, const MAX_LEN: usize> Serialize for BoundedVec<T, MAX_LEN> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>, const MAX_LEN: usize> Deserialize<'de> for BoundedVec<T, MAX_LEN> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BoundedVecVisitor<T, const MAX_LEN: usize>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>, const MAX_LEN: usize> Visitor<'de>
            for BoundedVecVisitor<T, MAX_LEN>
        {
            type Value = BoundedVec<T, MAX_LEN>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a sequence with at most {} elements", MAX_LEN)
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                if let Some(len) = sequence.size_hint()
                    && len > MAX_LEN
                {
                    return Err(A::Error::custom(format_args!(
                        "sequence length {} exceeds maximum {}",
                        len, MAX_LEN
                    )));
                }

                let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(MAX_LEN));
                while let Some(value) = sequence.next_element()? {
                    if values.len() == MAX_LEN {
                        return Err(A::Error::custom(format_args!(
                            "sequence length exceeds maximum {}",
                            MAX_LEN
                        )));
                    }
                    values.push(value);
                }
                Ok(BoundedVec(values))
            }
        }

        deserializer.deserialize_seq(BoundedVecVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Error;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ELEMENT_DESERIALIZATIONS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug)]
    struct CountingElement;

    impl<'de> Deserialize<'de> for CountingElement {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            ELEMENT_DESERIALIZATIONS.fetch_add(1, Ordering::Relaxed);
            u8::deserialize(deserializer)
                .map(|_| Self)
                .map_err(D::Error::custom)
        }
    }

    #[test]
    fn bcs_wire_format_matches_vec() {
        let values = vec![1u64, 2, 3];
        let bounded = BoundedVec::<_, 3>::new(values.clone()).unwrap();
        assert_eq!(
            bcs::to_bytes(&values).unwrap(),
            bcs::to_bytes(&bounded).unwrap()
        );
    }

    #[test]
    fn accepts_sequence_at_limit() {
        let bytes = bcs::to_bytes(&vec![1u64, 2, 3]).unwrap();
        let decoded = bcs::from_bytes::<BoundedVec<u64, 3>>(&bytes).unwrap();
        assert_eq!(&*decoded, &[1, 2, 3]);
    }

    #[test]
    fn rejects_oversized_bcs_sequence_before_elements() {
        ELEMENT_DESERIALIZATIONS.store(0, Ordering::Relaxed);
        let bytes = bcs::to_bytes(&vec![1u8, 2, 3]).unwrap();
        let error = bcs::from_bytes::<BoundedVec<CountingElement, 2>>(&bytes).unwrap_err();

        assert!(error.to_string().contains("exceeds maximum 2"));
        assert_eq!(ELEMENT_DESERIALIZATIONS.load(Ordering::Relaxed), 0);
    }
}
