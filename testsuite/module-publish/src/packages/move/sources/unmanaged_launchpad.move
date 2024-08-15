module unmanaged_launchpad::unmanaged_launchpad {
    use aptos_framework::object::Object;

    use aptos_token_objects::collection::Collection;

    #[randomness]
    entry fun mint(
        user: &signer,
        collection: Object<Collection>,
    ) {

    }
}
