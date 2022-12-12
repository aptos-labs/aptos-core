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

    use nfts_as_accounts::token;

    const ENOT_A_HERO: u64 = 1;
    const ENOT_A_WEAPON: u64 = 2;
    const ENOT_A_GEM: u64 = 3;

    struct OnChainConfig has key {
        collection: String,
        mutability_config: token::MutabilityConfig,
        royalty: token::Royalty,
    }

    struct Hero has key {
        armor: Option<ArmorRef>,
        gender: String,
        race: String,
        shield: Option<ShieldRef>,
        weapon: Option<WeaponRef>,
    }

    struct HeroRef has store {
        inner: token::TokenRef,
    }

    struct Armor has key {
        defense: u64,
        gem: Option<GemRef>,
        weight: u64,
    }

    struct ArmorRef has store {
        inner: token::TokenRef,
    }

    struct Gem has key {
        attack_modifier: u64,
        defense_modifier: u64,
        magic_attribute: String,
    }

    struct GemRef has store {
        inner: token::TokenRef,
    }

    struct Shield has key {
        defense: u64,
        gem: Option<GemRef>,
        weight: u64,
    }

    struct ShieldRef has store {
        inner: token::TokenRef,
    }

    struct Weapon has key {
        attack: u64,
        gem: Option<GemRef>,
        weapon_type: String,
        weight: u64,
    }

    struct WeaponRef has store {
        inner: token::TokenRef,
    }

    fun init_module(account: &signer) {
        let on_chain_config = OnChainConfig {
            collection: string::utf8(b"Hero Quest!"),
            mutability_config: token::create_mutability_config(true, true, true),
            royalty: token::create_royalty(0, 0, signer::address_of(account)),
        };
        move_to(account, on_chain_config);
    }

    fun create_token(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
    ): token::TokenRef acquires OnChainConfig {
        let on_chain_config = borrow_global<OnChainConfig>(signer::address_of(creator));
        token::create_token(
            creator, 
            *&on_chain_config.collection,
            description,
            *&on_chain_config.mutability_config,
            name,
            *&on_chain_config.royalty,
            uri,
        )
    }

    public fun create_hero(
        creator: &signer, 
        description: String,
        gender: String,
        name: String,
        race: String,
        uri: String,
    ): HeroRef acquires OnChainConfig {
        let token_ref = create_token(creator, description, name, uri);

        let hero = Hero {
            armor: option::none(),
            gender,
            race,
            shield: option::none(),
            weapon: option::none(),
        };

        let token_account = token::token_signer(creator, &token_ref);
        move_to(&token_account, hero);
        HeroRef { inner: token_ref }
    }

    public fun hero_ref_to_token_ref(hero_ref: HeroRef): token::TokenRef {
        let HeroRef { inner } = hero_ref;
        inner
    }

    public fun hero_ref_from_token_ref(token_ref: token::TokenRef): HeroRef {
        let token_addr = token::token_addr_from_ref(&token_ref);
        assert!(exists<Hero>(token_addr), ENOT_A_HERO);
        HeroRef { inner: token_ref }
    }

    public fun hero_addr(hero_ref: &HeroRef): address {
        token::token_addr_from_ref(&hero_ref.inner)
    }

    public fun hero_equip_weapon(hero_ref: &HeroRef, weapon_ref: WeaponRef) acquires Hero {
        let hero_addr = hero_addr(hero_ref);
        let hero = borrow_global_mut<Hero>(hero_addr);
        option::fill(&mut hero.weapon, weapon_ref);
    }

    public fun create_weapon(
        creator: &signer, 
        attack: u64,
        description: String,
        name: String,
        uri: String,
        weapon_type: String,
        weight: u64,
    ): WeaponRef acquires OnChainConfig {
        let token_ref = create_token(creator, description, name, uri);

        let weapon = Weapon {
            attack,
            gem: option::none(),
            weapon_type,
            weight,
        };

        let token_account = token::token_signer(creator, &token_ref);
        move_to(&token_account, weapon);
        WeaponRef { inner: token_ref }
    }

    public fun weapon_addr(weapon_ref: &WeaponRef): address {
        token::token_addr_from_ref(&weapon_ref.inner)
    }

    public fun weapon_ref_to_token_ref(weapon_ref: WeaponRef): token::TokenRef {
        let WeaponRef { inner } = weapon_ref;
        inner
    }

    public fun weapon_ref_from_token_ref(token_ref: token::TokenRef): WeaponRef {
        let token_addr = token::token_addr_from_ref(&token_ref);
        assert!(exists<Weapon>(token_addr), ENOT_A_WEAPON);
        WeaponRef { inner: token_ref }
    }

    public fun weapon_equip_gem(weapon_ref: &WeaponRef, gem_ref: GemRef) acquires Weapon {
        let weapon_addr = weapon_addr(weapon_ref);
        let weapon = borrow_global_mut<Weapon>(weapon_addr);
        option::fill(&mut weapon.gem, gem_ref);
    }

    public fun create_gem(
        creator: &signer, 
        attack_modifier: u64,
        defense_modifier: u64,
        description: String,
        magic_attribute: String,
        name: String,
        uri: String,
    ): GemRef acquires OnChainConfig {
        let token_ref = create_token(creator, description, name, uri);

        let gem = Gem {
            attack_modifier,
            defense_modifier,
            magic_attribute,
        };

        let token_account = token::token_signer(creator, &token_ref);
        move_to(&token_account, gem);
        GemRef { inner: token_ref }
    }

    public fun gem_addr(gem_ref: &GemRef): address {
        token::token_addr_from_ref(&gem_ref.inner)
    }

    public fun gem_ref_to_token_ref(gem_ref: GemRef): token::TokenRef {
        let GemRef { inner } = gem_ref;
        inner
    }

    public fun gem_ref_from_token_ref(token_ref: token::TokenRef): GemRef {
        let token_addr = token::token_addr_from_ref(&token_ref);
        assert!(exists<Gem>(token_addr), ENOT_A_GEM);
        GemRef { inner: token_ref }
    }

    #[test_only]

    use nfts_as_accounts::token_store;
    #[test(account = @0x3)]
    fun test_hero_with_gem_weapon(account: &signer) acquires Hero, OnChainConfig, Weapon {
        init_module(account);
        let hero_ref = create_hero(
            account,
            string::utf8(b"The best hero ever!"),
            string::utf8(b"Male"),
            string::utf8(b"Wukong"),
            string::utf8(b"Monkey God"),
            string::utf8(b""),
        );

        let weapon_ref = create_weapon(
            account,
            32,
            string::utf8(b"A magical staff!"),
            string::utf8(b"Ruyi Jingu Bang"),
            string::utf8(b""),
            string::utf8(b"staff"),
            15,
        );

        let gem_ref = create_gem(
            account,
            32,
            32,
            string::utf8(b"Beautiful specimen!"),
            string::utf8(b"earth"),
            string::utf8(b"jade"),
            string::utf8(b""),
        );

        let weapon_addr = weapon_addr(&weapon_ref);
        let gem_addr = gem_addr(&gem_ref);

        token_store::init(account);        
        token_store::store(account, weapon_ref_to_token_ref(weapon_ref));
        token_store::store(account, gem_ref_to_token_ref(gem_ref));

        let weapon_ref = weapon_ref_from_token_ref(token_store::take(account, weapon_addr));
        let gem_ref = gem_ref_from_token_ref(token_store::take(account, gem_addr));

        weapon_equip_gem(&weapon_ref, gem_ref);
        token_store::store(account, weapon_ref_to_token_ref(weapon_ref));

        let weapon_ref = weapon_ref_from_token_ref(token_store::take(account, weapon_addr));
        hero_equip_weapon(&hero_ref, weapon_ref);

        token_store::store(account, hero_ref_to_token_ref(hero_ref));
    }
}
