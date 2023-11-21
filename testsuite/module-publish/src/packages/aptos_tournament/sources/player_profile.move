module tournament::player_profile {
    use std::signer;
    friend tournament::tournament_manager;

   // will be either used for soulbound token for Player profile or for the Account itself
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct PlayerProfile has key {
        wins: u64,
        losses: u64,
        loot_collected: u64,
        loot_lost: u64,
    }

    public fun init(
        player: &signer,
    ) {
        let player_addr = signer::address_of(player);
        if (!exists<PlayerProfile>(player_addr)) {
            move_to(
                player,
                PlayerProfile {
                    wins: 0,
                    losses: 0,
                    loot_collected: 0,
                    loot_lost: 0,
                }
            );
        };
    }

    public(friend) fun record_profile_win(
        player: address,
        add_to_loot_collected: u64,
    ) acquires PlayerProfile {
        let player_profile = borrow_global_mut<PlayerProfile>(player);
        player_profile.loot_collected = player_profile.loot_collected + add_to_loot_collected;
        player_profile.wins = player_profile.wins + 1;
    }

    public(friend) fun record_profile_loss(
        player: address,
        add_to_loot_lost: u64,
    ) acquires PlayerProfile {
        let player_profile = borrow_global_mut<PlayerProfile>(player);
        player_profile.loot_lost = player_profile.loot_lost + add_to_loot_lost;
        player_profile.losses = player_profile.losses + 1;
    }

    struct ViewPlayerProfile has key {
        wins: u64,
        losses: u64,
        loot_collected: u64,
        loot_lost: u64,
    }

    #[view]
    public fun view_player_profile(player_profile_address: address): ViewPlayerProfile acquires PlayerProfile {
        let player_profile = borrow_global<PlayerProfile>(player_profile_address);
        ViewPlayerProfile {
            wins: player_profile.wins,
            losses: player_profile.losses,
            loot_collected: player_profile.loot_collected,
            loot_lost: player_profile.loot_lost,
        }
    }
}
