module 0x42::constants {
    const ENABLED: bool = true;
    const OWNER: address = @0x42;
    const NAME: vector<u8> = b"constants";
    const LIMIT: u8 = 200;
    const BIG: u128 = 1 << 100;

    spec fun int2bv_u64(value: u64): u64 {
        int2bv(value + 1)
    }

    spec fun int2bv_u128(value: u128): u128 {
        int2bv(value + 1)
    }

    spec fun int2bv_and_u64(left: u64, right: u64): u64 {
        int2bv(left) & right
    }

    public fun owner(): address {
        OWNER
    }

    public fun enabled(): bool {
        ENABLED
    }

    public fun name(): vector<u8> {
        NAME
    }

    public fun within(x: u8): bool {
        x < LIMIT
    }
    spec within {
        ensures result == (x < LIMIT);
    }

    public fun big(): u128 {
        BIG
    }
}
