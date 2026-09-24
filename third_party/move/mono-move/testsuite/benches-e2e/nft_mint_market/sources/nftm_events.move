/// Marketplace events, shaped after the ones Topaz emits on Aptos.
module bench::nftm_events {
    use aptos_framework::event;

    #[event]
    struct Mint has drop, store {
        collection_index: u64,
        token: address,
        owner: address,
    }

    #[event]
    struct List has drop, store {
        token: address,
        seller: address,
        price: u64,
    }

    #[event]
    struct Cancel has drop, store {
        token: address,
        seller: address,
    }

    #[event]
    struct Sale has drop, store {
        token: address,
        seller: address,
        buyer: address,
        price: u64,
        commission: u64,
        royalty: u64,
    }

    #[event]
    struct OfferPlaced has drop, store {
        offer: address,
        buyer: address,
        token: address,
        collection_index: u64,
        price: u64,
    }

    #[event]
    struct OfferClosed has drop, store {
        offer: address,
        buyer: address,
        refunded: bool,
    }

    public fun emit_mint(collection_index: u64, token: address, owner: address) {
        event::emit(Mint { collection_index, token, owner });
    }

    public fun emit_list(token: address, seller: address, price: u64) {
        event::emit(List { token, seller, price });
    }

    public fun emit_cancel(token: address, seller: address) {
        event::emit(Cancel { token, seller });
    }

    public fun emit_sale(
        token: address,
        seller: address,
        buyer: address,
        price: u64,
        commission: u64,
        royalty: u64,
    ) {
        event::emit(
            Sale { token, seller, buyer, price, commission, royalty });
    }

    public fun emit_offer_placed(
        offer: address,
        buyer: address,
        token: address,
        collection_index: u64,
        price: u64,
    ) {
        event::emit(
            OfferPlaced { offer, buyer, token, collection_index, price });
    }

    public fun emit_offer_closed(offer: address, buyer: address, refunded: bool) {
        event::emit(OfferClosed { offer, buyer, refunded });
    }
}
