/// Per-channel nonces, keyed the way LayerZero V2 keys a channel: the pair of
/// endpoint ids plus the sender and receiver OApp addresses.
///
/// The outbound nonce counts what has been sent and the inbound nonce what has
/// been consumed, whether by a delivery or by a skip. Their difference is the
/// queue depth every mix branch clamps against.
module bench::relay_channel {
    use std::signer;
    use aptos_std::table::{Self, Table};

    /// Only the package address may open or configure channels.
    const E_NOT_BENCH: u64 = 1;
    /// Channel table has not been created yet.
    const E_NOT_INITIALIZED: u64 = 2;

    struct Channel has store, drop {
        src_eid: u32,
        dst_eid: u32,
        sender: address,
        receiver: address,
        outbound_nonce: u64,
        inbound_nonce: u64,
        skipped: u64,
    }

    struct Channels has key {
        channels: Table<u64, Channel>,
        /// Channels the setup declared, which is the modulus a caller-supplied
        /// id is folded into.
        n_channels: u64,
        src_eid: u32,
        dst_eid: u32,
    }

    public fun initialize(
        admin: &signer, n_channels: u64, src_eid: u32, dst_eid: u32
    ) acquires Channels {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        if (!exists<Channels>(@bench)) {
            move_to(
                admin,
                Channels {
                    channels: table::new(),
                    n_channels,
                    src_eid,
                    dst_eid,
                },
            );
            return
        };
        let channels = borrow_global_mut<Channels>(@bench);
        channels.n_channels = n_channels;
        channels.src_eid = src_eid;
        channels.dst_eid = dst_eid;
    }

    public fun open(
        admin: &signer, channel_id: u64, receiver: address
    ) acquires Channels {
        assert!(signer::address_of(admin) == @bench, E_NOT_BENCH);
        assert!(exists<Channels>(@bench), E_NOT_INITIALIZED);
        let channels = borrow_global_mut<Channels>(@bench);
        if (table::contains(&channels.channels, channel_id)) return;
        let src_eid = channels.src_eid;
        let dst_eid = channels.dst_eid;
        table::add(
            &mut channels.channels,
            channel_id,
            Channel {
                src_eid,
                dst_eid,
                sender: @bench,
                receiver,
                outbound_nonce: 0,
                inbound_nonce: 0,
                skipped: 0,
            },
        );
    }

    /// Point the channel at the account that is about to use it. A no-op on a
    /// channel that was never opened, since the mix folds ids rather than
    /// rejecting them.
    public fun attach(channel_id: u64, sender: address) acquires Channels {
        if (!is_open(channel_id)) return;
        table::borrow_mut(
            &mut borrow_global_mut<Channels>(@bench).channels, channel_id
        ).sender = sender;
    }

    /// Take the next outbound nonce. Nonces count from one, matching the
    /// upstream endpoint.
    public fun next_outbound(channel_id: u64): u64 acquires Channels {
        if (!is_open(channel_id)) return 0;
        let channel = table::borrow_mut(
            &mut borrow_global_mut<Channels>(@bench).channels, channel_id);
        channel.outbound_nonce = channel.outbound_nonce + 1;
        channel.outbound_nonce
    }

    /// Consume the next inbound nonce and return it, or zero when the channel
    /// has nothing in flight.
    public fun advance_inbound(channel_id: u64): u64 acquires Channels {
        if (!is_open(channel_id)) return 0;
        let channel = table::borrow_mut(
            &mut borrow_global_mut<Channels>(@bench).channels, channel_id);
        if (channel.inbound_nonce >= channel.outbound_nonce) return 0;
        channel.inbound_nonce = channel.inbound_nonce + 1;
        channel.inbound_nonce
    }

    public fun note_skipped(channel_id: u64, n: u64) acquires Channels {
        if (!is_open(channel_id)) return;
        let channel = table::borrow_mut(
            &mut borrow_global_mut<Channels>(@bench).channels, channel_id);
        channel.skipped = channel.skipped + n;
    }

    #[view]
    public fun is_open(channel_id: u64): bool acquires Channels {
        if (!exists<Channels>(@bench)) return false;
        table::contains(&borrow_global<Channels>(@bench).channels, channel_id)
    }

    #[view]
    /// Channels the setup declared, zero before initialization.
    public fun declared_count(): u64 acquires Channels {
        if (!exists<Channels>(@bench)) return 0;
        borrow_global<Channels>(@bench).n_channels
    }

    #[view]
    public fun eids(): (u32, u32) acquires Channels {
        if (!exists<Channels>(@bench)) return (0, 0);
        let channels = borrow_global<Channels>(@bench);
        (channels.src_eid, channels.dst_eid)
    }

    #[view]
    public fun outbound(channel_id: u64): u64 acquires Channels {
        if (!is_open(channel_id)) return 0;
        table::borrow(
            &borrow_global<Channels>(@bench).channels, channel_id
        ).outbound_nonce
    }

    #[view]
    public fun inbound(channel_id: u64): u64 acquires Channels {
        if (!is_open(channel_id)) return 0;
        table::borrow(
            &borrow_global<Channels>(@bench).channels, channel_id
        ).inbound_nonce
    }

    #[view]
    /// Messages sent but not yet delivered or skipped.
    public fun outstanding(channel_id: u64): u64 acquires Channels {
        if (!is_open(channel_id)) return 0;
        let channel = table::borrow(
            &borrow_global<Channels>(@bench).channels, channel_id);
        channel.outbound_nonce - channel.inbound_nonce
    }

    #[view]
    public fun skipped(channel_id: u64): u64 acquires Channels {
        if (!is_open(channel_id)) return 0;
        table::borrow(
            &borrow_global<Channels>(@bench).channels, channel_id).skipped
    }

    #[view]
    public fun sender(channel_id: u64): address acquires Channels {
        if (!is_open(channel_id)) return @0x0;
        table::borrow(
            &borrow_global<Channels>(@bench).channels, channel_id).sender
    }

    #[view]
    public fun receiver(channel_id: u64): address acquires Channels {
        if (!is_open(channel_id)) return @0x0;
        table::borrow(
            &borrow_global<Channels>(@bench).channels, channel_id).receiver
    }
}
