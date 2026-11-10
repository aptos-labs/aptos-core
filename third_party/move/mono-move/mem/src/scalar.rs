// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Plain-data types that may be loaded from and stored to VM memory.

use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};

/// A plain-data type the VM stores as bytes: the fixed-width integers,
/// byte arrays, [`U256`], [`I256`], and [`AccountAddress`].
///
/// Not `bool`: reading a byte other than `0` or `1` as a `bool` is immediate
/// undefined behavior, so booleans are read as `u8` and checked instead
/// ([`MemAddr::load_bool`](crate::MemAddr::load_bool)). Not pointers: they
/// carry provenance that an integer load would strip
/// ([`MemAddr::load_ptr`](crate::MemAddr::load_ptr)).
///
/// # Safety
///
/// Every bit pattern must be a valid `Self`, values must carry no provenance,
/// `from_le`/`to_le` must convert between host order and the little-endian
/// representation stored in memory, and `ALIGN` must be `align_of::<Self>()`.
pub unsafe trait Scalar: Copy {
    const ALIGN: usize;
    fn from_le(v: Self) -> Self;
    fn to_le(v: Self) -> Self;
}

macro_rules! impl_scalar_int {
    ($($t:ty),* $(,)?) => {$(
        // SAFETY: fixed-width integers: no padding, no provenance, standard
        // byte-order conversions.
        unsafe impl Scalar for $t {
            const ALIGN: usize = std::mem::align_of::<$t>();

            #[inline(always)]
            fn from_le(v: Self) -> Self {
                <$t>::from_le(v)
            }

            #[inline(always)]
            fn to_le(v: Self) -> Self {
                <$t>::to_le(v)
            }
        }
    )*};
}

impl_scalar_int!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

// SAFETY: a byte array: no padding, no provenance, no byte order.
unsafe impl<const N: usize> Scalar for [u8; N] {
    const ALIGN: usize = 1;

    #[inline(always)]
    fn from_le(v: Self) -> Self {
        v
    }

    #[inline(always)]
    fn to_le(v: Self) -> Self {
        v
    }
}

// SAFETY: `U256` is `[u128; 2]` underneath: no padding, no provenance. Its
// native image (`to_ne_bytes`) on a little-endian host is already the LE
// encoding, so the conversions below are identity there and a byte swap
// elsewhere.
unsafe impl Scalar for U256 {
    const ALIGN: usize = std::mem::align_of::<U256>();

    #[inline(always)]
    fn from_le(v: Self) -> Self {
        U256::from_le_bytes(v.repr().to_ne_bytes())
    }

    #[inline(always)]
    fn to_le(v: Self) -> Self {
        ethnum::U256::from_ne_bytes(v.to_le_bytes()).into()
    }
}

// SAFETY: as for `U256`.
unsafe impl Scalar for I256 {
    const ALIGN: usize = std::mem::align_of::<I256>();

    #[inline(always)]
    fn from_le(v: Self) -> Self {
        I256::from_le_bytes(v.repr().to_ne_bytes())
    }

    #[inline(always)]
    fn to_le(v: Self) -> Self {
        ethnum::I256::from_ne_bytes(v.to_le_bytes()).into()
    }
}

// SAFETY: a 32-byte array newtype: no padding, no provenance, no byte order.
unsafe impl Scalar for AccountAddress {
    const ALIGN: usize = std::mem::align_of::<AccountAddress>();

    #[inline(always)]
    fn from_le(v: Self) -> Self {
        v
    }

    #[inline(always)]
    fn to_le(v: Self) -> Self {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemAddr;

    #[test]
    fn move_types_round_trip_as_le_bytes() {
        #[repr(C, align(8))]
        struct Buf([u8; 96]);
        let mut buf = Buf([0; 96]);
        let base = MemAddr::from_ptr(buf.0.as_mut_ptr());
        let u = U256::from_le_bytes(std::array::from_fn(|i| i as u8));
        let i = I256::from_le_bytes(std::array::from_fn(|i| 0xF0 - i as u8));
        let a = AccountAddress::new(std::array::from_fn(|i| 0x40 + i as u8));
        unsafe {
            base.store(u);
            base.add(32).store(i);
            base.add(64).store(a);
            assert_eq!(base.load::<U256>(), u);
            assert_eq!(base.add(32).load::<I256>(), i);
            assert_eq!(base.add(64).load::<AccountAddress>(), a);
        }
        // The stored image is the little-endian encoding regardless of host.
        assert_eq!(&buf.0[..32], &u.to_le_bytes());
        assert_eq!(&buf.0[32..64], &i.to_le_bytes());
        assert_eq!(&buf.0[64..], a.as_ref());
    }
}
