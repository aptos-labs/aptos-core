/// This demonstrates how to make a composable NFT on Aptos. We take a Hero that can have optional
/// weapons, armor, and a shield. Each of these can optionally have a gem. These can be added or
/// removed and seamlessly treated as Tokens or their richer type.
///
/// todo(@davidiw):
/// * complete armor and shield
/// * add accessors to everything
/// * make it possible to get a reference to the various ref types so that the items can remain
///   as is without requiring decomposing
/// * better error handling around if an item is already equipped
/// * add the ability to remove weapons and gems
module nfts_as_accounts::composable_example {
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};

    use nfts_as_accounts::token::{Self, TokenRef};

    const ENOT_A_HERO: u64 = 1;
    const ENOT_A_WEAPON: u64 = 2;
    const ENOT_A_GEM: u64 = 3;

    struct OnChainConfig has key {
        collection: String,
        mutability_config: token::MutabilityConfig,
        royalty: token::Royalty,
    }

    struct Hero has store {
        armor: Option<TokenRef<Armor>>,
        gender: String,
        race: String,
        shield: Option<TokenRef<Shield>>,
        weapon: Option<TokenRef<Weapon>>,
    }

    struct Armor has store {
        defense: u64,
        gem: Option<TokenRef<Gem>>,
        weight: u64,
    }

    struct Gem has store {
        attack_modifier: u64,
        defense_modifier: u64,
        magic_attribute: String,
    }

    struct Shield has store {
        defense: u64,
        gem: Option<TokenRef<Gem>>,
        weight: u64,
    }

    struct Weapon has store {
        attack: u64,
        gem: Option<TokenRef<Gem>>,
        weapon_type: String,
        weight: u64,
    }

    fun init_module(account: &signer) {
        let on_chain_config = OnChainConfig {
            collection: string::utf8(b"Hero Quest!"),
            mutability_config: token::create_mutability_config(true, true, true),
            royalty: token::create_royalty(0, 0, signer::address_of(account)),
        };
        move_to(account, on_chain_config);
    }

    fun create_token<Data: store>(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
        data: Data,
    ): TokenRef<Data> acquires OnChainConfig {
        let on_chain_config = borrow_global<OnChainConfig>(signer::address_of(creator));
        token::create_token(
            creator, 
            *&on_chain_config.collection,
            description,
            *&on_chain_config.mutability_config,
            name,
            *&on_chain_config.royalty,
            uri,
            data,
        )
    }

    public fun create_hero(
        creator: &signer, 
        description: String,
        gender: String,
        name: String,
        race: String,
        uri: String,
    ): TokenRef<Hero> acquires OnChainConfig {
        let hero = Hero {
            armor: option::none(),
            gender,
            race,
            shield: option::none(),
            weapon: option::none(),
        };
        create_token(creator, description, name, uri, hero)
    }

    public fun hero_equip_weapon(hero: &TokenRef<Hero>, weapon: TokenRef<Weapon>) {
        let hero_data = token::take_data(hero);
        option::fill(&mut hero_data.weapon, weapon);
        token::set_data(hero, hero_data);
    }

    public fun create_weapon(
        creator: &signer, 
        attack: u64,
        description: String,
        name: String,
        uri: String,
        weapon_type: String,
        weight: u64,
    ): TokenRef<Weapon> acquires OnChainConfig {
        let weapon = Weapon {
            attack,
            gem: option::none(),
            weapon_type,
            weight,
        };

        create_token(creator, description, name, uri, weapon)
    }

    public fun weapon_equip_gem(weapon: &TokenRef<Weapon>, gem: TokenRef<Gem>) {
        let weapon_data = token::take_data(weapon);
        option::fill(&mut weapon_data.gem, gem);
        token::set_data(weapon, weapon_data);
    }

    public fun create_gem(
        creator: &signer, 
        attack_modifier: u64,
        defense_modifier: u64,
        description: String,
        magic_attribute: String,
        name: String,
        uri: String,
    ): TokenRef<Gem> acquires OnChainConfig {
        let gem = Gem {
            attack_modifier,
            defense_modifier,
            magic_attribute,
        };

        create_token(creator, description, name, uri, gem)
    }

    #[test_only]
    use nfts_as_accounts::token_store;

    #[test(account = @0x3)]
    fun test_hero_with_gem_weapon(account: &signer) acquires OnChainConfig {
        init_module(account);
        let hero = create_hero(
            account,
            string::utf8(b"The best hero ever!"),
            string::utf8(b"Male"),
            string::utf8(b"Wukong"),
            string::utf8(b"Monkey God"),
            string::utf8(b""),
        );

        let weapon = create_weapon(
            account,
            32,
            string::utf8(b"A magical staff!"),
            string::utf8(b"Ruyi Jingu Bang"),
            string::utf8(b""),
            string::utf8(b"staff"),
            15,
        );

        let gem = create_gem(
            account,
            32,
            32,
            string::utf8(b"Beautiful specimen!"),
            string::utf8(b"earth"),
            string::utf8(b"jade"),
            string::utf8(b""),
        );

        let weapon_addr = token::token_addr_from_ref(&weapon);
        let gem_addr = token::token_addr_from_ref(&gem);

        token_store::init(account);        
        token_store::store(account, weapon);
        token_store::store(account, gem);

        let weapon = token_store::take(account, weapon_addr);
        let gem = token_store::take(account, gem_addr);

        weapon_equip_gem(&weapon, gem);
        token_store::store(account, weapon);

        let weapon = token_store::take(account, weapon_addr);
        hero_equip_weapon(&hero, weapon);

        token_store::store(account, hero);
    }
}
