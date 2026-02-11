/// This module defines a chain of unique structs where each struct contains the previous one
/// plus 7 primitive fields. Since every struct is unique, the DAG-based StructLayoutCache
/// never gets a cache hit, so the total type node count grows linearly.
///
/// Each struct contributes 8 nodes (1 struct node + 7 primitive fields).
/// S64 has 8 * 65 = 520 nodes, which exceeds the 512-node limit.
/// The limit is enforced at runtime when constructing the type layout for move_to.
module addr::exceed_limit {
    use std::signer;

    struct S0 has drop, key, store { f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S1 has drop, key, store { inner: S0, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S2 has drop, key, store { inner: S1, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S3 has drop, key, store { inner: S2, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S4 has drop, key, store { inner: S3, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S5 has drop, key, store { inner: S4, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S6 has drop, key, store { inner: S5, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S7 has drop, key, store { inner: S6, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S8 has drop, key, store { inner: S7, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S9 has drop, key, store { inner: S8, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S10 has drop, key, store { inner: S9, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S11 has drop, key, store { inner: S10, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S12 has drop, key, store { inner: S11, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S13 has drop, key, store { inner: S12, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S14 has drop, key, store { inner: S13, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S15 has drop, key, store { inner: S14, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S16 has drop, key, store { inner: S15, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S17 has drop, key, store { inner: S16, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S18 has drop, key, store { inner: S17, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S19 has drop, key, store { inner: S18, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S20 has drop, key, store { inner: S19, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S21 has drop, key, store { inner: S20, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S22 has drop, key, store { inner: S21, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S23 has drop, key, store { inner: S22, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S24 has drop, key, store { inner: S23, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S25 has drop, key, store { inner: S24, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S26 has drop, key, store { inner: S25, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S27 has drop, key, store { inner: S26, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S28 has drop, key, store { inner: S27, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S29 has drop, key, store { inner: S28, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S30 has drop, key, store { inner: S29, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S31 has drop, key, store { inner: S30, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S32 has drop, key, store { inner: S31, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S33 has drop, key, store { inner: S32, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S34 has drop, key, store { inner: S33, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S35 has drop, key, store { inner: S34, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S36 has drop, key, store { inner: S35, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S37 has drop, key, store { inner: S36, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S38 has drop, key, store { inner: S37, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S39 has drop, key, store { inner: S38, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S40 has drop, key, store { inner: S39, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S41 has drop, key, store { inner: S40, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S42 has drop, key, store { inner: S41, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S43 has drop, key, store { inner: S42, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S44 has drop, key, store { inner: S43, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S45 has drop, key, store { inner: S44, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S46 has drop, key, store { inner: S45, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S47 has drop, key, store { inner: S46, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S48 has drop, key, store { inner: S47, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S49 has drop, key, store { inner: S48, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S50 has drop, key, store { inner: S49, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S51 has drop, key, store { inner: S50, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S52 has drop, key, store { inner: S51, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S53 has drop, key, store { inner: S52, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S54 has drop, key, store { inner: S53, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S55 has drop, key, store { inner: S54, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S56 has drop, key, store { inner: S55, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S57 has drop, key, store { inner: S56, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S58 has drop, key, store { inner: S57, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S59 has drop, key, store { inner: S58, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S60 has drop, key, store { inner: S59, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S61 has drop, key, store { inner: S60, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S62 has drop, key, store { inner: S61, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S63 has drop, key, store { inner: S62, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }
    struct S64 has drop, key, store { inner: S63, f0: u8, f1: u16, f2: u32, f3: u64, f4: u128, f5: bool, f6: address }

    inline fun d(a: address): S0 { S0 { f0: 0, f1: 0, f2: 0, f3: 0, f4: 0, f5: false, f6: a } }
    inline fun w<T>(inner: T, a: address): (T, u8, u16, u32, u64, u128, bool, address) { (inner, 0, 0, 0, 0, 0, false, a) }

    fun init_module(source_account: &signer) {
        let a = signer::address_of(source_account);
        let s0 = d(a);
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s0, a);
        let s1 = S1 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s1, a);
        let s2 = S2 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s2, a);
        let s3 = S3 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s3, a);
        let s4 = S4 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s4, a);
        let s5 = S5 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s5, a);
        let s6 = S6 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s6, a);
        let s7 = S7 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s7, a);
        let s8 = S8 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s8, a);
        let s9 = S9 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s9, a);
        let s10 = S10 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s10, a);
        let s11 = S11 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s11, a);
        let s12 = S12 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s12, a);
        let s13 = S13 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s13, a);
        let s14 = S14 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s14, a);
        let s15 = S15 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s15, a);
        let s16 = S16 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s16, a);
        let s17 = S17 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s17, a);
        let s18 = S18 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s18, a);
        let s19 = S19 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s19, a);
        let s20 = S20 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s20, a);
        let s21 = S21 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s21, a);
        let s22 = S22 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s22, a);
        let s23 = S23 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s23, a);
        let s24 = S24 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s24, a);
        let s25 = S25 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s25, a);
        let s26 = S26 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s26, a);
        let s27 = S27 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s27, a);
        let s28 = S28 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s28, a);
        let s29 = S29 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s29, a);
        let s30 = S30 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s30, a);
        let s31 = S31 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s31, a);
        let s32 = S32 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s32, a);
        let s33 = S33 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s33, a);
        let s34 = S34 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s34, a);
        let s35 = S35 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s35, a);
        let s36 = S36 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s36, a);
        let s37 = S37 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s37, a);
        let s38 = S38 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s38, a);
        let s39 = S39 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s39, a);
        let s40 = S40 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s40, a);
        let s41 = S41 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s41, a);
        let s42 = S42 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s42, a);
        let s43 = S43 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s43, a);
        let s44 = S44 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s44, a);
        let s45 = S45 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s45, a);
        let s46 = S46 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s46, a);
        let s47 = S47 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s47, a);
        let s48 = S48 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s48, a);
        let s49 = S49 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s49, a);
        let s50 = S50 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s50, a);
        let s51 = S51 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s51, a);
        let s52 = S52 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s52, a);
        let s53 = S53 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s53, a);
        let s54 = S54 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s54, a);
        let s55 = S55 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s55, a);
        let s56 = S56 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s56, a);
        let s57 = S57 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s57, a);
        let s58 = S58 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s58, a);
        let s59 = S59 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s59, a);
        let s60 = S60 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s60, a);
        let s61 = S61 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s61, a);
        let s62 = S62 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s62, a);
        let s63 = S63 { inner, f0, f1, f2, f3, f4, f5, f6 };
        let (inner, f0, f1, f2, f3, f4, f5, f6) = w(s63, a);
        let s64 = S64 { inner, f0, f1, f2, f3, f4, f5, f6 };
        move_to(source_account, s64);
    }
}
