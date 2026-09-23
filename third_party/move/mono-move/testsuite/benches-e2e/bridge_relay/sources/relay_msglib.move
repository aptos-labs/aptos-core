/// Message identity for the relay, shaped after the LayerZero V2 message
/// library.
///
/// Upstream a GUID is a keccak hash of the endpoint ids, the addresses of both
/// ends, and the nonce; here it is a bit-packed `u128`, because hashing every
/// message would move this workload's cost from storage onto compute.
module bench::relay_msglib {
    /// Each GUID field is 32 bits wide.
    const FIELD_MASK: u64 = 0xffffffff;
    const FIELD_MASK_128: u128 = 0xffffffff;

    /// Low 63 bits of a GUID, which is what a slot key is built from.
    const SLOT_MASK: u128 = 0x7fffffffffffffff;

    /// Slots a channel cycles through per nonce stream. Nonces never repeat,
    /// so keying on the nonce itself would make every write a creation and
    /// grow the tree for the length of a run. Folding the nonce into a ring
    /// settles the payload store and the DVN attestations at a fixed size
    /// instead, and a ring this wide holds every message a single transaction
    /// can name, so no transaction wraps its own window.
    const GUID_RING: u64 = 64;

    /// GUID layout, high bits first: source eid, destination eid, channel id,
    /// nonce, each 32 bits wide.
    public fun guid(
        src_eid: u32, dst_eid: u32, channel_id: u64, nonce: u64
    ): u128 {
        ((src_eid as u128) << 96)
            | ((dst_eid as u128) << 64)
            | (((channel_id & FIELD_MASK) as u128) << 32)
            | ((nonce & FIELD_MASK) as u128)
    }

    /// The bits of a GUID a slot key is built from: the channel id, and the
    /// nonce folded into the channel's ring.
    public fun guid_slot_bits(guid: u128): u64 {
        let bits = ((guid & SLOT_MASK) as u64);
        (bits & (FIELD_MASK << 32)) | (bits & (GUID_RING - 1))
    }

    public fun guid_nonce(guid: u128): u64 {
        ((guid & FIELD_MASK_128) as u64)
    }

    public fun guid_channel(guid: u128): u64 {
        (((guid >> 32) & FIELD_MASK_128) as u64)
    }

    #[view]
    public fun guid_ring(): u64 { GUID_RING }
}
