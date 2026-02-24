/// This module defines a chain of unique structs where each struct contains the previous one
/// plus 7 primitive fields. Since every struct is unique, the DAG-based StructLayoutCache
/// never gets a cache hit, so the total type node count grows linearly.
///
/// Each struct contributes 8 nodes (1 struct node + 7 primitive fields).
/// S64 has 8 * 65 = 520 nodes, which exceeds the 512-node limit.
/// The limit is enforced at runtime when constructing the type layout for bcs::to_bytes.
module 0xbeef::test {

    struct S0 has drop { f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S1 has drop { inner: S0, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S2 has drop { inner: S1, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S3 has drop { inner: S2, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S4 has drop { inner: S3, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S5 has drop { inner: S4, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S6 has drop { inner: S5, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S7 has drop { inner: S6, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S8 has drop { inner: S7, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S9 has drop { inner: S8, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S10 has drop { inner: S9, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S11 has drop { inner: S10, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S12 has drop { inner: S11, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S13 has drop { inner: S12, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S14 has drop { inner: S13, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S15 has drop { inner: S14, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S16 has drop { inner: S15, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S17 has drop { inner: S16, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S18 has drop { inner: S17, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S19 has drop { inner: S18, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S20 has drop { inner: S19, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S21 has drop { inner: S20, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S22 has drop { inner: S21, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S23 has drop { inner: S22, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S24 has drop { inner: S23, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S25 has drop { inner: S24, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S26 has drop { inner: S25, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S27 has drop { inner: S26, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S28 has drop { inner: S27, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S29 has drop { inner: S28, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S30 has drop { inner: S29, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S31 has drop { inner: S30, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S32 has drop { inner: S31, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S33 has drop { inner: S32, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S34 has drop { inner: S33, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S35 has drop { inner: S34, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S36 has drop { inner: S35, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S37 has drop { inner: S36, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S38 has drop { inner: S37, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S39 has drop { inner: S38, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S40 has drop { inner: S39, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S41 has drop { inner: S40, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S42 has drop { inner: S41, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S43 has drop { inner: S42, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S44 has drop { inner: S43, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S45 has drop { inner: S44, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S46 has drop { inner: S45, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S47 has drop { inner: S46, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S48 has drop { inner: S47, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S49 has drop { inner: S48, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S50 has drop { inner: S49, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S51 has drop { inner: S50, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S52 has drop { inner: S51, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S53 has drop { inner: S52, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S54 has drop { inner: S53, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S55 has drop { inner: S54, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S56 has drop { inner: S55, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S57 has drop { inner: S56, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S58 has drop { inner: S57, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S59 has drop { inner: S58, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S60 has drop { inner: S59, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S61 has drop { inner: S60, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S62 has drop { inner: S61, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S63 has drop { inner: S62, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S64 has drop { inner: S63, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }

    use std::bcs;
    use std::vector;

    public entry fun run() {
        bcs::to_bytes<vector<S64>>(&vector::empty());
    }
}
