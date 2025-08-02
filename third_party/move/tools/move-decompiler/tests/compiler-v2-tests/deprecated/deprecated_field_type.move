module 0x42::event {

    #[deprecated]
    /// A handle for an event such that:
    /// 1. Other modules can emit events to this handle.
    /// 2. Storage can use this handle to prove the total number of events that happened in the past.
    struct EventHandle<phantom T: drop + store> has store {
        /// Total number of events emitted to this event stream.
        counter: u64,
        /// A globally unique ID for this event stream.
        guid: u64,
    }

}

module 0x41::coin {
    use 0x42::event::EventHandle;

    struct Coin<phantom T> has store { }
    struct CoinType has key {}
    struct DepositEvent has drop, store {}
    struct WithdrawEvent has drop, store {}

    struct CoinStore<phantom CoinType> has key {
        coin: Coin<CoinType>,
        frozen: bool,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
    }
}
