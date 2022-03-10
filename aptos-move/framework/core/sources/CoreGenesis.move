module CoreFramework::CoreGenesis {
    use CoreFramework::ChainId;
    use CoreFramework::Block;
    use CoreFramework::Reconfiguration;
    use CoreFramework::Timestamp;

    /// This can only be called once successfully, since after the first call time will have started.
    public fun init(core_resource_account: &signer, chain_id: u8) {
        ChainId::initialize(core_resource_account, chain_id);
        Reconfiguration::initialize(core_resource_account);
        Block::initialize_block_metadata(core_resource_account);
        Timestamp::set_time_has_started(core_resource_account);
    }
}
