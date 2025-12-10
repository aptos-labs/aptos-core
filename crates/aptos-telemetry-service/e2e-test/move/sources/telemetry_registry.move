/// Test module for Telemetry Service E2E testing
/// Implements a simple allowlist registry for custom contract authentication
///
/// This contract allows a single admin to manage a list of authorized addresses
/// that can authenticate with the telemetry service.
module telemetry_deployer::telemetry_registry {
    use std::signer;
    use std::vector;
    use aptos_framework::event;

    /// Error codes
    const ENOT_AUTHORIZED: u64 = 1;
    const EALREADY_REGISTERED: u64 = 2;
    const ENOT_FOUND: u64 = 3;

    /// Member information stored on-chain
    struct Member has copy, drop, store {
        address: address,
        ip_address: vector<u8>,  // String representation like "127.0.0.1"
        port: vector<u8>,         // String representation like "8080"
        bls_public_key: vector<u8>,  // Hex-encoded BLS key
        failure_domain: vector<u8>,  // e.g., "dc_us_east"
    }

    /// Registry resource storing all authorized members
    struct Registry has key {
        admin: address,
        members: vector<Member>,
    }

    // Event emitted when a member is added
    #[event]
    struct MemberAddedEvent has drop, store {
        address: address,
        timestamp: u64,
    }

    // Event emitted when a member is removed
    #[event]
    struct MemberRemovedEvent has drop, store {
        address: address,
        timestamp: u64,
    }

    /// Initialize the registry (called once by the deployer)
    public entry fun initialize(account: &signer) {
        let account_addr = signer::address_of(account);

        // Only allow initialization once
        assert!(!exists<Registry>(account_addr), EALREADY_REGISTERED);

        move_to(account, Registry {
            admin: account_addr,
            members: vector::empty(),
        });
    }

    /// Add a new member to the registry (admin only)
    public entry fun add_member(
        admin: &signer,
        member_address: address,
        ip_address: vector<u8>,
        port: vector<u8>,
        bls_public_key: vector<u8>,
        failure_domain: vector<u8>,
    ) acquires Registry {
        let registry = borrow_global_mut<Registry>(signer::address_of(admin));

        // Verify admin
        assert!(signer::address_of(admin) == registry.admin, ENOT_AUTHORIZED);

        // Check if member already exists
        let i = 0;
        let len = vector::length(&registry.members);
        while (i < len) {
            let member = vector::borrow(&registry.members, i);
            assert!(member.address != member_address, EALREADY_REGISTERED);
            i = i + 1;
        };

        // Add new member
        let member = Member {
            address: member_address,
            ip_address,
            port,
            bls_public_key,
            failure_domain,
        };
        vector::push_back(&mut registry.members, member);

        // Emit event
        event::emit(MemberAddedEvent {
            address: member_address,
            timestamp: aptos_framework::timestamp::now_seconds(),
        });
    }

    /// Remove a member from the registry (admin only)
    public entry fun remove_member(
        admin: &signer,
        member_address: address,
    ) acquires Registry {
        let registry = borrow_global_mut<Registry>(signer::address_of(admin));

        // Verify admin
        assert!(signer::address_of(admin) == registry.admin, ENOT_AUTHORIZED);

        // Find and remove member
        let i = 0;
        let len = vector::length(&registry.members);
        let found = false;
        while (i < len) {
            let member = vector::borrow(&registry.members, i);
            if (member.address == member_address) {
                vector::remove(&mut registry.members, i);
                found = true;
                break
            };
            i = i + 1;
        };

        assert!(found, ENOT_FOUND);

        // Emit event
        event::emit(MemberRemovedEvent {
            address: member_address,
            timestamp: aptos_framework::timestamp::now_seconds(),
        });
    }

    // View function to get all members (used by telemetry service)
    // Returns vector of Member structs containing address, ip, port, etc.
    #[view]
    public fun get_all_members(registry_address: address): vector<Member> acquires Registry {
        let registry = borrow_global<Registry>(registry_address);
        *&registry.members  // Return a copy of the members vector
    }

    // View function to check if an address is authorized
    #[view]
    public fun is_member(registry_address: address, member_address: address): bool acquires Registry {
        let registry = borrow_global<Registry>(registry_address);
        let i = 0;
        let len = vector::length(&registry.members);
        while (i < len) {
            let member = vector::borrow(&registry.members, i);
            if (member.address == member_address) {
                return true
            };
            i = i + 1;
        };
        false
    }

    // View function to get member count
    #[view]
    public fun member_count(registry_address: address): u64 acquires Registry {
        let registry = borrow_global<Registry>(registry_address);
        vector::length(&registry.members)
    }
}
