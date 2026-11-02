/// Byte-level decode of a Switchboard-shaped report batch: a flat blob of
/// fixed-width big-endian records, taken apart one byte at a time.
///
/// This is the interpreter-side knob of the workload. A batch costs a fixed
/// number of storage writes but `depth` times as many shifts and byte loads,
/// so raising `depth` moves cost into the interpreter without touching the
/// read or write set.
///
/// A batch arrives as the exact bytes the publisher signed, so the signature
/// is checked against them with nothing rebuilt on chain. Those bytes are a
/// 32-byte domain prefix, the BCS length of the records, then the records
/// themselves. A record is 48 bytes: `feed_id: u64 | price: u128 | conf: u64 |
/// ts: u64`, then eight reserved bytes. Reads past the end of the blob yield
/// zero, so a truncated record decodes rather than aborting.
module bench::orc_decode {
    use std::vector;

    const RECORD_LEN: u64 = 48;

    const OFF_FEED: u64 = 0;
    const OFF_PRICE: u64 = 8;
    const OFF_CONF: u64 = 24;
    const OFF_TS: u64 = 32;

    /// Domain separator the signer prepends.
    const PREFIX_LEN: u64 = 32;

    /// Bytes a BCS length may take. Stopping here bounds the walk over a
    /// malformed header.
    const MAX_LEN_BYTES: u64 = 10;

    public fun record_len(): u64 {
        RECORD_LEN
    }

    /// Where the records start. A truncated header yields an offset past the
    /// end of `data`, which leaves the batch empty instead of aborting.
    public fun payload_offset(data: &vector<u8>): u64 {
        let i = PREFIX_LEN;
        let read = 0;
        while (read < MAX_LEN_BYTES) {
            let byte = byte_at(data, i);
            i = i + 1;
            read = read + 1;
            if (byte < 128) break;
        };
        i
    }

    /// Records in `data`, counting a trailing partial record as a whole one.
    public fun n_records(data: &vector<u8>): u64 {
        let offset = payload_offset(data);
        let len = vector::length(data);
        if (len <= offset) return 0;
        (len - offset + RECORD_LEN - 1) / RECORD_LEN
    }

    /// Byte `i` of `data`, or zero past the end.
    public fun byte_at(data: &vector<u8>, i: u64): u8 {
        if (i < vector::length(data)) *vector::borrow(data, i) else 0
    }

    public fun u64_at(data: &vector<u8>, offset: u64): u64 {
        let acc = 0u64;
        let i = 0;
        while (i < 8) {
            acc = (acc << 8) | (byte_at(data, offset + i) as u64);
            i = i + 1;
        };
        acc
    }

    public fun u128_at(data: &vector<u8>, offset: u64): u128 {
        let acc = 0u128;
        let i = 0;
        while (i < 16) {
            acc = (acc << 8) | (byte_at(data, offset + i) as u128);
            i = i + 1;
        };
        acc
    }

    /// Decode record `index` as `(feed_id, price, conf, ts)`, repeating the
    /// decode `depth` times. A zero depth decodes nothing and yields zeros.
    public fun decode_record(
        data: &vector<u8>, index: u64, depth: u64
    ): (u64, u128, u64, u64) {
        let base = payload_offset(data) + index * RECORD_LEN;
        let feed_id = 0;
        let price = 0u128;
        let conf = 0;
        let ts = 0;
        let d = 0;
        while (d < depth) {
            feed_id = u64_at(data, base + OFF_FEED);
            price = u128_at(data, base + OFF_PRICE);
            conf = u64_at(data, base + OFF_CONF);
            ts = u64_at(data, base + OFF_TS);
            d = d + 1;
        };
        (feed_id, price, conf, ts)
    }
}
