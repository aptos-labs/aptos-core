module 0xbeef::VersionModule {
    use std::signer;
    use std::debug;

    // Enum with 64 variants, each carrying a u64 value.
    // The max_struct_variants limit is 64, meaning up to 64 variants are allowed.
    enum Versions has copy, drop, store {
        V0(u64),
        V1(u64),
        V2(u64),
        V3(u64),
        V4(u64),
        V5(u64),
        V6(u64),
        V7(u64),
        V8(u64),
        V9(u64),
        V10(u64),
        V11(u64),
        V12(u64),
        V13(u64),
        V14(u64),
        V15(u64),
        V16(u64),
        V17(u64),
        V18(u64),
        V19(u64),
        V20(u64),
        V21(u64),
        V22(u64),
        V23(u64),
        V24(u64),
        V25(u64),
        V26(u64),
        V27(u64),
        V28(u64),
        V29(u64),
        V30(u64),
        V31(u64),
        V32(u64),
        V33(u64),
        V34(u64),
        V35(u64),
        V36(u64),
        V37(u64),
        V38(u64),
        V39(u64),
        V40(u64),
        V41(u64),
        V42(u64),
        V43(u64),
        V44(u64),
        V45(u64),
        V46(u64),
        V47(u64),
        V48(u64),
        V49(u64),
        V50(u64),
        V51(u64),
        V52(u64),
        V53(u64),
        V54(u64),
        V55(u64),
        V56(u64),
        V57(u64),
        V58(u64),
        V59(u64),
        V60(u64),
        V61(u64),
        V62(u64),
        V63(u64),
    }

    // Resource that holds a version.
    struct VersionHolder has key, store {
        version: Versions,
    }

    /// Stores the provided version under the caller's account.
    public entry fun store_version(account: &signer) {
        let version = Versions::V1(1);

        // Store the version
        move_to(account, VersionHolder { version });
    }

    /// Retrieves the stored version from the given account.
    public entry fun get_version(account: &signer) acquires VersionHolder {
        let holder = borrow_global<VersionHolder>(signer::address_of(account));
        debug::print(&holder.version);
    }
}
