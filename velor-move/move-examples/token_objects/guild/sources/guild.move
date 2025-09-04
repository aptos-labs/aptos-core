/// This module implements the guild and member tokens.
/// The flow works as follows:
/// 1. When the admin (i.e., the contract owner) publishes this module,
///   - a guild collection manager object is created,
///   - a guild collection is created with the signer of the guild collection manager, and
///   - a whitelist for guild masters is created.
/// 2. The admin adds guild masters to the whitelist.
/// 3. A whitelisted guild master calls `mint_guild` to mint a guild token,
///    and creates a member collection for the guild.
/// 4. A guild master with a guild token calls `mint_member` to mint
///    a member token for a user who is a guild member.
/// 5. A guild master burns (or revokes) a member token.
module guild::guild {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use velor_framework::object::{Self, Object};
    use velor_std::smart_vector::{Self, SmartVector};
    use velor_token_objects::collection;
    use velor_token_objects::token;

    /// The provided signer is not the admin
    const ENOT_ADMIN: u64 = 1;
    /// The provided signer is not the owner
    const ENOT_OWNER: u64 = 2;
    /// The provided signer is not a guild master
    const ENOT_GUILD_MASTER: u64 = 3;

    /// The guild token collection name
    const GUILD_COLLECTION_NAME: vector<u8> = b"Guild Collection Name";
    /// The guild collection description
    const GUILD_COLLECTION_DESCRIPTION: vector<u8> = b"Guild Collection Description";
    /// The guild collection URI
    const GUILD_COLLECTION_URI: vector<u8> = b"https://guild.collection.uri";

    /// Published under the contract owner's account.
    struct Config has key {
        /// Whitelist of guild masters.
        whitelist: SmartVector<address>,
        /// `extend_ref` of the guild collection manager object. Used to obtain its signer.
        extend_ref: object::ExtendRef,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Guild token
    struct GuildToken has key {
        /// Used to get the signer of the token
        extend_ref: object::ExtendRef,
        /// Member collection name
        member_collection_name: String,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Member token
    struct MemberToken has key {
        /// Belonging guild
        guild: Object<GuildToken>,
        /// Used to burn
        burn_ref: token::BurnRef,
    }

    /// Initializes the module, creating the manager object, the guild token collection and the whitelist.
    fun init_module(sender: &signer) acquires Config {
        // Create the guild collection manager object to use it to autonomously
        // manage the guild collection (e.g., create the collection and mint tokens).
        let constructor_ref = object::create_object(signer::address_of(sender));
        let extend_ref = object::generate_extend_ref(&constructor_ref);

        // Publish the config resource.
        move_to(sender, Config { whitelist: smart_vector::new(), extend_ref});

        // Create the guild collection.
        create_guild_collection(&guild_collection_manager_signer());
    }

    #[view]
    /// Returns the guild token address by name
    public fun guild_token_address(guild_token_name: String): address acquires Config {
        token::create_token_address(&guild_collection_manager_address(), &string::utf8(GUILD_COLLECTION_NAME), &guild_token_name)
    }

    #[view]
    /// Returns the guild token address by name
    public fun member_token_address(guild_token: Object<GuildToken>, member_token_name: String): address acquires GuildToken {
        let guild_token_addr = object::object_address(&guild_token);
        let member_collection_name = &borrow_global<GuildToken>(guild_token_addr).member_collection_name;
        token::create_token_address(&guild_token_addr, member_collection_name, &member_token_name)
    }

    /// Adds a guild master to the whitelist. This function allows the admin to add a guild master
    /// to the whitelist.
    public entry fun whitelist_guild_master(admin: &signer, guild_master: address) acquires Config {
        assert!(signer::address_of(admin) == guild_collection_manager_owner(), ENOT_ADMIN);
        let config = borrow_global_mut<Config>(@guild);
        smart_vector::push_back(&mut config.whitelist, guild_master);
    }

    /// Mints a guild token, and creates a new associated member collection.
    /// This function allows a whitelisted guild master to mint a new guild token.
    public entry fun mint_guild(
        guild_master: &signer,
        description: String,
        name: String,
        uri: String,
        member_collection_name: String,
        member_collection_description: String,
        member_collection_uri: String,
    ) acquires Config {
        // Checks if the guild master is whitelisted.
        let guild_master_addr = signer::address_of(guild_master);
        assert!(is_whitelisted(guild_master_addr), ENOT_GUILD_MASTER);

        // The collection name is used to locate the collection object and to create a new token object.
        let collection = string::utf8(GUILD_COLLECTION_NAME);
        // Creates the guild token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        // TODO: Switch to `create_token` once it is available.
        let constructor_ref = token::create_named_token(
            &guild_collection_manager_signer(),
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let extend_ref = object::generate_extend_ref(&constructor_ref);

        // Transfers the token to the guild master.
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, guild_master_addr);


        // Publishes the GuildToken resource with the refs.
        let guild_token = GuildToken {
            extend_ref,
            member_collection_name,
        };
        move_to(&object_signer, guild_token);

        // Creates a member collection which is associated to the guild token.
        create_member_collection(&object_signer, member_collection_name, member_collection_description, member_collection_uri);
    }

    /// Mints a member token. This function mints a new member token and transfers it to the
    /// `receiver` address.
    public entry fun mint_member(
        guild_master: &signer,
        guild_token: Object<GuildToken>,
        description: String,
        name: String,
        uri: String,
        receiver: address,
    ) acquires GuildToken {
        // Checks if the guild master is the owner of the guild token.
        assert!(object::owner(guild_token) == signer::address_of(guild_master), ENOT_OWNER);

        let guild = borrow_global<GuildToken>(object::object_address(&guild_token));
        let guild_token_object_signer = object::generate_signer_for_extending(&guild.extend_ref);
        // Creates the member token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        let constructor_ref = token::create_named_token(
            &guild_token_object_signer,
            guild.member_collection_name,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let burn_ref = token::generate_burn_ref(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);

        // Transfers the token to the `soul_bound_to` address
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, receiver);

        // Publishes the MemberToken resource with the refs.
        let member_token = MemberToken {
            guild: guild_token,
            burn_ref,
        };
        move_to(&object_signer, member_token);
    }

    /// Burns a member token.
    public entry fun burn_member(
        guild_master: &signer,
        token: Object<MemberToken>,
    ) acquires MemberToken {
        let belonging_guild = borrow_global<MemberToken>(object::object_address(&token)).guild;
        assert!(object::owner(belonging_guild) == signer::address_of(guild_master), ENOT_OWNER);
        let member_token = move_from<MemberToken>(object::object_address(&token));
        let MemberToken {
            guild: _,
            burn_ref,
        } = member_token;
        token::burn(burn_ref);
    }

    /// Returns the signer of the guild collection manager object.
    fun guild_collection_manager_signer(): signer acquires Config {
        let manager = borrow_global<Config>(@guild);
        object::generate_signer_for_extending(&manager.extend_ref)
    }

    /// Returns the signer of the guild collection manager object.
    fun guild_collection_manager_owner(): address acquires Config {
        let manager = borrow_global<Config>(@guild);
        let manager_addr = object::address_from_extend_ref(&manager.extend_ref);
        object::owner(object::address_to_object<object::ObjectCore>(manager_addr))
    }

    /// Returns the address of the guild collection manager object.
    fun guild_collection_manager_address(): address acquires Config {
        let manager = borrow_global<Config>(@guild);
        object::address_from_extend_ref(&manager.extend_ref)
    }

    /// Creates the guild collection. This function creates a collection with unlimited supply using
    /// the module constants for description, name, and URI, defined above. The royalty configuration
    /// is skipped in this collection for simplicity.
    fun create_guild_collection(admin: &signer) {
        // Constructs the strings from the bytes.
        let description = string::utf8(GUILD_COLLECTION_DESCRIPTION);
        let name = string::utf8(GUILD_COLLECTION_NAME);
        let uri = string::utf8(GUILD_COLLECTION_URI);

        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            admin,
            description,
            name,
            option::none(),
            uri,
        );
    }

    /// Creates the member collection. This function creates a collection with unlimited supply using
    /// the module constants for description, name, and URI, defined above. The royalty configuration
    /// is skipped in this collection for simplicity.
    fun create_member_collection(guild_token_object_signer: &signer, name: String, description: String, uri: String) {
        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            guild_token_object_signer,
            description,
            name,
            option::none(),
            uri,
        );
    }

    inline fun is_whitelisted(guild_master: address): bool {
        let whitelist = &borrow_global<Config>(@guild).whitelist;
        smart_vector::contains(whitelist, &guild_master)
    }

    #[test(admin = @guild, guild_master = @0x456, user = @0x789)]
    public fun test_guild(admin: &signer, guild_master: &signer, user: address) acquires GuildToken, MemberToken, Config, Config {
        // This test assumes that the creator's address is equal to @token_objects.
        assert!(signer::address_of(admin) == @guild, 0);

        // -----------------------------------
        // Admin creates the guild collection.
        // -----------------------------------
        init_module(admin);

        // ---------------------------------------------
        // Admin adds the guild master to the whitelist.
        // ---------------------------------------------
        whitelist_guild_master(admin, signer::address_of(guild_master));

        // ------------------------------------------
        // Guild master mints a guild token.
        // ------------------------------------------
        mint_guild(
            guild_master,
            string::utf8(b"Guild Token #1 Description"),
            string::utf8(b"Guild Token #1"),
            string::utf8(b"Guild Token #1 URI"),
            string::utf8(b"Member Collection #1"),
            string::utf8(b"Member Collection #1 Description"),
            string::utf8(b"Member Collection #1 URI"),
        );

        // -------------------------------------------
        // Guild master mints a member token for User.
        // -------------------------------------------
        let token_name = string::utf8(b"Member Token #1");
        let token_description = string::utf8(b"Member Token #1 Description");
        let token_uri = string::utf8(b"Member Token #1 URI");
        let guild_token_addr = guild_token_address(string::utf8(b"Guild Token #1"));
        let guild_token = object::address_to_object<GuildToken>(guild_token_addr);
        // Creates the member token for User.
        mint_member(
            guild_master,
            guild_token,
            token_description,
            token_name,
            token_uri,
            user,
        );

        // ------------------------------------------------
        // Guild master burns the member token of the User.
        // ------------------------------------------------
        let member_token_addr = member_token_address(guild_token, token_name);
        burn_member(guild_master, object::address_to_object<MemberToken>(member_token_addr));
    }
}
