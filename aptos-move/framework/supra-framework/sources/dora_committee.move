/// Design:
/// CommitteeInfoStore: store all the committee information, supported operation: add, remove
/// CommitteeInfo: store all the dora node information, supported operation: add, remove
/// DoraNodeInfo: store the dora node information, supported operation: update
/// NodeData: store the dora node information with operator's address
/// requirements:
/// 1. two committee has different id
/// 2. two indentical node should not be in the same committee
module supra_framework::dora_committee {
    use std::error;
    use std::option;
    use std::signer;
    use std::vector;
    use aptos_std::capability;
    use supra_framework::event::{emit_event, EventHandle};
    use aptos_std::simple_map::{Self, SimpleMap};
    use supra_framework::account::{Self, new_event_handle};

    /// The number of committee is not equal to the number of committee member
    const INVALID_COMMITTEE_NUMBERS: u64 = 4;

    /// The node is not found in the committee
    const NODE_NOT_FOUND: u64 = 5;

    /// The committee is not found
    const INVALID_COMMITTEE_ID: u64 = 6;

    /// The committee type is invalid
    const INVALID_COMMITTEE_TYPE: u64 = 7;

    /// The number of nodes in the committee is invalid
    const INVALID_NODE_NUMBERS: u64 = 8;

    /// Define the CommitteeType as constants
    const FAMILY: u8 = 1;
    const CLAN: u8 = 2;
    const TRIBE: u8 = 3;

    const SEED_COMMITTEE: vector<u8> = b"supra_framework::dora_committee::CommitteeInfoStore";

    /// Capability that grants an owner the right to perform action.
    struct OwnerCap has drop {}

    struct SupraCommitteeEventHandler has key {
        create: EventHandle<CreateCommitteeInfoStoreEvent>,
        add_committee: EventHandle<AddCommitteeEvent>,
        remove_committee: EventHandle<RemoveCommitteeEvent>,
        update_committee: EventHandle<UpdateCommitteeEvent>,
        add_committee_member: EventHandle<AddCommitteeMemberEvent>,
        remove_committee_member: EventHandle<RemoveCommitteeMemberEvent>,
        update_node_info: EventHandle<UpdateNodeInfoEvent>,
        update_dkg_flag: EventHandle<UpdateDkgFlagEvent>,
    }

    struct CommitteeInfoStore has key {
        // map from unique committee id -> CommitteeInfo
        committee_map: SimpleMap<u64, CommitteeInfo>,
        // map from node address to committee to which the node belongs
        node_to_committee_map: SimpleMap<address, u64>,
    }

    struct CommitteeInfo has store, drop, copy {
        // map from node address to DoraNodeInfo which provides other details pertaining to the node
        map: SimpleMap<address, DoraNodeInfo>,
        // a flag that indicates whether the committee already has performed DKG successfully or not, thus indicating whether corresponding public key are valid to be used for threshold signing
        has_valid_dkg: bool,
        committee_type: u8
    }

    struct DoraNodeInfo has store, copy, drop {
        ip_public_address: vector<u8>,
        // public key to be used for voting
        dora_public_key: vector<u8>,
        // public key for secure TLS connection
        network_public_key: vector<u8>,
        cg_public_key: vector<u8>,
        // port number at which the node listens to for connection attempts by peers
        network_port: u16,
        // port number to serve RPC requests
        rpc_port: u16
    }

    struct NodeData has copy, drop {
        operator: address,
        ip_public_address: vector<u8>,
        dora_public_key: vector<u8>,
        network_public_key: vector<u8>,
        cg_public_key: vector<u8>,
        network_port: u16,
        rpc_port: u16
    }

    struct AddCommitteeEvent has store, drop {
        committee_id: u64,
        committee_info: CommitteeInfo
    }

    struct RemoveCommitteeEvent has store, drop {
        committee_id: u64,
        committee_info: CommitteeInfo
    }

    struct UpdateCommitteeEvent has store, drop {
        committee_id: u64,
        old_committee_info: CommitteeInfo,
        new_committee_info: CommitteeInfo
    }

    struct AddCommitteeMemberEvent has store, drop {
        committee_id: u64,
        committee_member: DoraNodeInfo
    }

    struct RemoveCommitteeMemberEvent has store, drop {
        committee_id: u64,
        committee_member: DoraNodeInfo
    }

    struct UpdateNodeInfoEvent has store, drop {
        committee_id: u64,
        old_node_info: DoraNodeInfo,
        new_node_info: DoraNodeInfo
    }

    struct CreateCommitteeInfoStoreEvent has store, drop {
        committee_id: u64,
        committee_info: CommitteeInfo
    }

    struct UpdateDkgFlagEvent has store, drop {
        committee_id: u64,
        flag_value: bool
    }

    /// Internal - check if the node exists in the committee
    fun does_node_exist(committee: &CommitteeInfo, node_address: address): bool {
        simple_map::contains_key(&committee.map, &node_address)
    }
    /// Internal - Assert if the node exists in the committee
    fun ensure_node_address_exist(committee: &CommitteeInfo, node_address: address) {
        assert!(does_node_exist(committee, node_address), error::invalid_argument(NODE_NOT_FOUND))
    }

    /// Internal - create OwnerCap
    fun create_owner(owner_signer: &signer) {
        capability::create<OwnerCap>(owner_signer, &OwnerCap {});
    }

    /// Internal - create committeeInfo store functions
    fun create_committeeInfo_store(owner_signer: &signer): signer {
        let (resource_signer, _) = account::create_resource_account(owner_signer, SEED_COMMITTEE);
        move_to(&resource_signer, CommitteeInfoStore {
            committee_map: simple_map::new(),
            node_to_committee_map: simple_map::new()
        });
        resource_signer
    }

    /// Internal - create event handler
    fun create_event_handler(resource_signer: &signer) {
        move_to(resource_signer, SupraCommitteeEventHandler {
            create: new_event_handle<CreateCommitteeInfoStoreEvent>(resource_signer),
            add_committee: new_event_handle<AddCommitteeEvent>(resource_signer),
            remove_committee: new_event_handle<RemoveCommitteeEvent>(resource_signer),
            update_committee: new_event_handle<UpdateCommitteeEvent>(resource_signer),
            add_committee_member: new_event_handle<AddCommitteeMemberEvent>(resource_signer),
            remove_committee_member: new_event_handle<RemoveCommitteeMemberEvent>(resource_signer),
            update_node_info: new_event_handle<UpdateNodeInfoEvent>(resource_signer),
            update_dkg_flag: new_event_handle<UpdateDkgFlagEvent>(resource_signer),
        });
    }

    fun get_committeeInfo_address(owner_signer: &signer): address {
        account::create_resource_address(&signer::address_of(owner_signer), SEED_COMMITTEE)
    }

    /// Its Initial function which will be executed automatically while deployed packages
    fun init_module(owner_signer: &signer) {
        create_owner(owner_signer);
        let resource_signer = create_committeeInfo_store(owner_signer);
        create_event_handler(&resource_signer);
    }

    // Function to validate the committee type from an integer
    fun validate_committee_type(committee_type: u8, num_of_nodes: u64): u8 {
        assert!(committee_type >= FAMILY && committee_type <= TRIBE, INVALID_COMMITTEE_TYPE);
        if (committee_type == FAMILY) {
            // f+1, number of nodes in a family committee should be greater than 1
            assert!(num_of_nodes > 1, INVALID_NODE_NUMBERS);
        } else if (committee_type == CLAN) {
            // 2f+1, number of nodes in a clan committee should be odd and greater than 3
            assert!(num_of_nodes >= 3 && num_of_nodes % 2 == 1, INVALID_NODE_NUMBERS);
        } else {
            // 3f+1, number of nodes in a tribe committee should be in the format of 3f+1 and greater than 4
            assert!(num_of_nodes >= 4 && (num_of_nodes - 1) % 3 == 0, INVALID_NODE_NUMBERS);
        };
        committee_type
    }

    #[view]
    /// Get the committee's dora node vector and committee type
    public fun get_committee_info(com_store_addr: address, id: u64): (vector<NodeData>, u8) acquires CommitteeInfoStore {
        let committee_store = borrow_global<CommitteeInfoStore>(com_store_addr);
        let committee = simple_map::borrow(&committee_store.committee_map, &id);
        let (addrs, dora_nodes) = simple_map::to_vec_pair(committee.map);
        let node_data_vec = vector::empty<NodeData>();
        while (vector::length(&addrs) > 0) {
            let addr = vector::pop_back(&mut addrs);
            let dora_node_info = vector::pop_back(&mut dora_nodes);
            let node_data = NodeData {
                operator: addr,
                ip_public_address: dora_node_info.ip_public_address,
                dora_public_key: dora_node_info.dora_public_key,
                network_public_key: dora_node_info.network_public_key,
                cg_public_key: dora_node_info.cg_public_key,
                network_port: dora_node_info.network_port,
                rpc_port: dora_node_info.rpc_port,
            };
            vector::push_back(&mut node_data_vec, node_data);
        };
        (node_data_vec, committee.committee_type)
    }

    #[view]
    /// Get the committee's ids from the store
    public fun get_committee_ids(com_store_addr: address): vector<u64> acquires CommitteeInfoStore {
        let committee_store = borrow_global<CommitteeInfoStore>(com_store_addr);
        simple_map::keys(&committee_store.committee_map)
    }

    #[view]
    /// Get the committee's id for a single node, only pass the address is okay
    public fun get_committee_id(
        com_store_addr: address,
        node_address: address
    ): u64 acquires CommitteeInfoStore {
        let committee_store = borrow_global<CommitteeInfoStore>(com_store_addr);
        *simple_map::borrow(&committee_store.node_to_committee_map, &node_address)
    }

    #[view]
    /// Get the node's information
    public fun get_node_info(
        com_store_addr: address,
        id: u64,
        node_address: address
    ): NodeData acquires CommitteeInfoStore {
        let committee_store = borrow_global<CommitteeInfoStore>(com_store_addr);
        let committee = simple_map::borrow(&committee_store.committee_map, &id);
        let (addrs, dora_nodes) = simple_map::to_vec_pair(committee.map);
        let (flag, index) = vector::index_of(&addrs, &node_address);
        assert!(flag, error::invalid_argument(NODE_NOT_FOUND));
        let dora_node_info = vector::borrow(&dora_nodes, index);

        NodeData {
            operator: node_address,
            ip_public_address: dora_node_info.ip_public_address,
            dora_public_key: dora_node_info.dora_public_key,
            network_public_key: dora_node_info.network_public_key,
            cg_public_key: dora_node_info.cg_public_key,
            network_port: dora_node_info.network_port,
            rpc_port: dora_node_info.rpc_port,
        }
    }

    #[view]
    /// Get the committee's id for a single node
    public fun get_committee_id_for_node(
        com_store_addr: address,
        node_address: address
    ): u64 acquires CommitteeInfoStore {
        let committee_store = borrow_global<CommitteeInfoStore>(com_store_addr);
        let id = *simple_map::borrow(&committee_store.node_to_committee_map, &node_address);
        id
    }

    #[view]
    /// Get a tuple of the node itself and dora node peers vector for a single node
    public fun get_peers_for_node(
        com_store_addr: address,
        node_address: address
    ): (NodeData, vector<NodeData>) acquires CommitteeInfoStore {
        let committee_id = get_committee_id_for_node(com_store_addr, node_address);
        let this_node = get_node_info(com_store_addr, committee_id, node_address);
        let (dora_node_info,_) = get_committee_info(com_store_addr, committee_id);
        let (_, index) = vector::index_of(&dora_node_info, &this_node);
        let self= vector::remove(&mut dora_node_info, index);
        (self, dora_node_info)
    }

    #[view]
    /// Check if the committee has a valid dkg
    public fun does_com_have_dkg(com_store_addr: address, com_id: u64): bool acquires CommitteeInfoStore {
        let committee_store = borrow_global<CommitteeInfoStore>(com_store_addr);
        let committee = simple_map::borrow(&committee_store.committee_map, &com_id);
        committee.has_valid_dkg
    }

    /// Update the dkg flag
    public fun update_dkg_flag(
        com_store_addr: address,
        owner_signer: &signer,
        com_id: u64,
        flag_value: bool
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        // Only the OwnerCap capability can access it
        let _acquire = &capability::acquire(owner_signer, &OwnerCap {});
        let committee_store = borrow_global_mut<CommitteeInfoStore>(com_store_addr);
        let committee = simple_map::borrow_mut(&mut committee_store.committee_map, &com_id);
        committee.has_valid_dkg = flag_value;
        let event_handler = borrow_global_mut<SupraCommitteeEventHandler>(get_committeeInfo_address(owner_signer));
        emit_event(
            &mut event_handler.update_dkg_flag,
            UpdateDkgFlagEvent {
                committee_id: com_id,
                flag_value
            }
        );
    }

    /// This function is used to add a new committee to the store
    public entry fun upsert_committee(
        com_store_addr: address,
        owner_signer: &signer,
        id: u64,
        node_addresses: vector<address>,
        ip_public_address: vector<vector<u8>>,
        dora_public_key: vector<vector<u8>>,
        network_public_key: vector<vector<u8>>,
        cg_public_key: vector<vector<u8>>,
        network_port: vector<u16>,
        rpc_port: vector<u16>,
        committee_type: u8
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        // Assert the length of the vector for two are the same
        let node_address_len = vector::length(&node_addresses);
        assert!(
            node_address_len == vector::length(&ip_public_address),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            node_address_len == vector::length(&dora_public_key),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            node_address_len == vector::length(&network_public_key),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            node_address_len == vector::length(&cg_public_key),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            node_address_len == vector::length(&network_port),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            node_address_len == vector::length(&rpc_port),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        // Only the OwnerCap capability can access it
        let _acquire = &capability::acquire(owner_signer, &OwnerCap {});
        let committee_store = borrow_global_mut<CommitteeInfoStore>(com_store_addr);
        let dora_node_info = vector::empty<DoraNodeInfo>();
        let node_addresses_for_iteration = node_addresses;
        while (vector::length(&node_addresses_for_iteration) > 0) {
            let ip_public_address = vector::pop_back(&mut ip_public_address);
            let dora_public_key = vector::pop_back(&mut dora_public_key);
            let network_public_key = vector::pop_back(&mut network_public_key);
            let cg_public_key = vector::pop_back(&mut cg_public_key);
            let network_port = vector::pop_back(&mut network_port);
            let rpc_port = vector::pop_back(&mut rpc_port);
            let dora_node = DoraNodeInfo {
                ip_public_address: copy ip_public_address,
                dora_public_key: copy dora_public_key,
                network_public_key: copy network_public_key,
                cg_public_key: copy cg_public_key,
                network_port,
                rpc_port,
            };
            vector::push_back(&mut dora_node_info, dora_node);
            // Also update the node_to_committee_map
            let node_address = vector::pop_back(&mut node_addresses_for_iteration);
            simple_map::upsert(&mut committee_store.node_to_committee_map, node_address, id);
        };
        vector::reverse(&mut dora_node_info);
        let committee_info = CommitteeInfo {
            map: simple_map::new_from(node_addresses, dora_node_info),
            has_valid_dkg: false,
            committee_type: validate_committee_type(committee_type, node_address_len)
        };
        let event_handler = borrow_global_mut<SupraCommitteeEventHandler>(get_committeeInfo_address(owner_signer));
        let (_, value) = simple_map::upsert(&mut committee_store.committee_map, id, committee_info);
        if (option::is_none(&value)) {
            emit_event(
                &mut event_handler.add_committee,
                AddCommitteeEvent {
                    committee_id: id,
                    committee_info: copy committee_info
                }, )
        } else {
            emit_event(
                &mut event_handler.update_committee,
                UpdateCommitteeEvent {
                    committee_id: id,
                    old_committee_info: option::destroy_some(value),
                    new_committee_info: committee_info
                },
            );
            // Destory the map
            simple_map::to_vec_pair(option::destroy_some(value).map);
        };
    }

    /// Add the committee in bulk
    public entry fun upsert_committee_bulk(
        com_store_addr: address,
        owner_signer: &signer,
        ids: vector<u64>,
        node_addresses_bulk: vector<vector<address>>,
        ip_public_address_bulk: vector<vector<vector<u8>>>,
        dora_public_key_bulk: vector<vector<vector<u8>>>,
        network_public_key_bulk: vector<vector<vector<u8>>>,
        cg_public_key_bulk: vector<vector<vector<u8>>>,
        network_port_bulk: vector<vector<u16>>,
        rpc_por_bulkt: vector<vector<u16>>,
        committee_types: vector<u8>
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        // Assert the length of the vector for two are the same
        let ids_len = vector::length(&ids);
        assert!(
            ids_len == vector::length(&node_addresses_bulk),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            ids_len == vector::length(&ip_public_address_bulk),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            ids_len == vector::length(&dora_public_key_bulk),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            ids_len == vector::length(&network_public_key_bulk),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            ids_len == vector::length(&cg_public_key_bulk),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            ids_len == vector::length(&network_port_bulk),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            ids_len == vector::length(&rpc_por_bulkt),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        while (vector::length(&ids) > 0) {
            let id = vector::pop_back(&mut ids);
            let node_addresses = vector::pop_back(&mut node_addresses_bulk);
            let ip_public_address = vector::pop_back(&mut ip_public_address_bulk);
            let dora_public_key = vector::pop_back(&mut dora_public_key_bulk);
            let network_public_key = vector::pop_back(&mut network_public_key_bulk);
            let cg_public_key = vector::pop_back(&mut cg_public_key_bulk);
            let network_port = vector::pop_back(&mut network_port_bulk);
            let rpc_port = vector::pop_back(&mut rpc_por_bulkt);
            let committee_type = vector::pop_back(&mut committee_types);
            upsert_committee(
                com_store_addr,
                owner_signer,
                id,
                node_addresses,
                ip_public_address,
                dora_public_key,
                network_public_key,
                cg_public_key,
                network_port,
                rpc_port,
                committee_type
            );
        }
    }

    /// Remove the committee from the store
    public entry fun remove_committee(
        com_store_addr: address,
        owner_signer: &signer,
        id: u64
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        // Only the OwnerCap capability can access it
        let _acquire = &capability::acquire(owner_signer, &OwnerCap {});

        let committee_store = borrow_global_mut<CommitteeInfoStore>(com_store_addr);
        assert!(
            simple_map::contains_key(&committee_store.committee_map, &id),
            error::invalid_argument(INVALID_COMMITTEE_ID)
        );
        let (id, committee_info) = simple_map::remove(&mut committee_store.committee_map, &id);
        // Also remove the node_to_committee_map
        let (addrs, _) = simple_map::to_vec_pair(committee_info.map);
        while (vector::length(&addrs) > 0) {
            let addr = vector::pop_back(&mut addrs);
            assert!(
                simple_map::contains_key(&committee_store.node_to_committee_map, &addr),
                error::invalid_argument(NODE_NOT_FOUND)
            );
            simple_map::remove(&mut committee_store.node_to_committee_map, &addr);
        };
        let event_handler = borrow_global_mut<SupraCommitteeEventHandler>(get_committeeInfo_address(owner_signer));
        emit_event(
            &mut event_handler.remove_committee,
            RemoveCommitteeEvent {
                committee_id: id,
                committee_info: committee_info
            }, )
    }

    /// Remove the committee in bulk
    public entry fun remove_committee_bulk(
        com_store_addr: address,
        owner_signer: &signer,
        ids: vector<u64>
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        while (vector::length(&ids) > 0) {
            let id = vector::pop_back(&mut ids);
            remove_committee(com_store_addr, owner_signer, id);
        }
    }

    /// Upsert the node to the committee
    public entry fun upsert_committee_member(
        com_store_addr: address,
        owner_signer: &signer,
        id: u64,
        node_address: address,
        ip_public_address: vector<u8>,
        dora_public_key: vector<u8>,
        network_public_key: vector<u8>,
        cg_public_key: vector<u8>,
        network_port: u16,
        rpc_port: u16,
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        // Only the OwnerCap capability can access it
        let _acquire = &capability::acquire(owner_signer, &OwnerCap {});

        let committee_store = borrow_global_mut<CommitteeInfoStore>(com_store_addr);
        let committee = simple_map::borrow_mut(&mut committee_store.committee_map, &id);
        let dora_node_info = DoraNodeInfo {
            ip_public_address: copy ip_public_address,
            dora_public_key: copy dora_public_key,
            network_public_key: copy network_public_key,
            cg_public_key: copy cg_public_key,
            network_port: network_port,
            rpc_port: rpc_port,
        };
        let event_handler = borrow_global_mut<SupraCommitteeEventHandler>(get_committeeInfo_address(owner_signer));
        if (!does_node_exist(committee, node_address)) {
            emit_event(
                &mut event_handler.add_committee_member,
                AddCommitteeMemberEvent {
                    committee_id: id,
                    committee_member: dora_node_info
                })
        } else {
            emit_event(
                &mut event_handler.update_node_info,
                UpdateNodeInfoEvent {
                    committee_id: id,
                    old_node_info: *simple_map::borrow(&committee.map, &node_address),
                    new_node_info: dora_node_info
                })
        };
        simple_map::upsert(&mut committee.map, node_address, dora_node_info);
        // Also update the node_to_committee_map
        simple_map::upsert(&mut committee_store.node_to_committee_map, node_address, id);
    }

    /// Upsert dora nodes to the committee
    public entry fun upsert_committee_member_bulk(
        com_store_addr: address,
        owner_signer: &signer,
        ids: vector<u64>,
        node_addresses: vector<address>,
        ip_public_address: vector<vector<u8>>,
        dora_public_key: vector<vector<u8>>,
        network_public_key: vector<vector<u8>>,
        cg_public_key: vector<vector<u8>>,
        network_port: vector<u16>,
        rpc_port: vector<u16>
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        // Assert the length of the vector for two are the same
        assert!(
            vector::length(&ids) == vector::length(&node_addresses),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            vector::length(&ids) == vector::length(&ip_public_address),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            vector::length(&ids) == vector::length(&dora_public_key),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            vector::length(&ids) == vector::length(&network_public_key),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            vector::length(&ids) == vector::length(&cg_public_key),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            vector::length(&ids) == vector::length(&network_port),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        assert!(
            vector::length(&ids) == vector::length(&rpc_port),
            error::invalid_argument(INVALID_COMMITTEE_NUMBERS)
        );
        while (vector::length(&ids) > 0) {
            let id = vector::pop_back(&mut ids);
            let node_address = vector::pop_back(&mut node_addresses);
            let ip_public_address = vector::pop_back(&mut ip_public_address);
            let dora_public_key = vector::pop_back(&mut dora_public_key);
            let network_public_key = vector::pop_back(&mut network_public_key);
            let cg_public_key = vector::pop_back(&mut cg_public_key);
            let network_port = vector::pop_back(&mut network_port);
            let rpc_port = vector::pop_back(&mut rpc_port);
            upsert_committee_member(
                com_store_addr,
                owner_signer,
                id,
                node_address,
                ip_public_address,
                dora_public_key,
                network_public_key,
                cg_public_key,
                network_port,
                rpc_port
            );
        }
    }

    /// Remove the node from the committee
    public entry fun remove_committee_member(
        com_store_addr: address,
        owner_signer: &signer,
        id: u64,
        node_address: address
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        // Only the OwnerCap capability can access it
        let _acquire = &capability::acquire(owner_signer, &OwnerCap {});

        let committee_store = borrow_global_mut<CommitteeInfoStore>(com_store_addr);
        let committee = simple_map::borrow_mut(&mut committee_store.committee_map, &id);
        ensure_node_address_exist(committee, node_address);
        let (_, node_info) = simple_map::remove(&mut committee.map, &node_address);
        let event_handler = borrow_global_mut<SupraCommitteeEventHandler>(get_committeeInfo_address(owner_signer));
        emit_event(
            &mut event_handler.remove_committee_member,
            RemoveCommitteeMemberEvent {
                committee_id: id,
                committee_member: DoraNodeInfo {
                    ip_public_address: node_info.ip_public_address,
                    dora_public_key: node_info.dora_public_key,
                    network_public_key: node_info.network_public_key,
                    cg_public_key: node_info.cg_public_key,
                    network_port: node_info.network_port,
                    rpc_port: node_info.rpc_port,
                }
            }
        );
        // Remove the node from the node_to_committee_map
        simple_map::remove(&mut committee_store.node_to_committee_map, &node_address);
    }

    /// Find the node in the committee
    public fun find_node_in_committee(
        com_store_add: address,
        id: u64,
        node_address: address
    ): (bool, NodeData) acquires CommitteeInfoStore {
        let committee_store = borrow_global<CommitteeInfoStore>(com_store_add);
        let committee = simple_map::borrow(&committee_store.committee_map, &id);
        let flag = simple_map::contains_key(&committee.map, &node_address);
        if (!flag) {
            return (false, NodeData {
                operator: copy node_address,
                ip_public_address: vector::empty(),
                dora_public_key: vector::empty(),
                network_public_key: vector::empty(),
                cg_public_key: vector::empty(),
                network_port: 0,
                rpc_port: 0,
            })
        } else {
            let dora_node_info = *simple_map::borrow(&committee.map, &node_address);
            (true, NodeData {
                operator: copy node_address,
                ip_public_address: dora_node_info.ip_public_address,
                dora_public_key: dora_node_info.dora_public_key,
                network_public_key: dora_node_info.network_public_key,
                cg_public_key: dora_node_info.cg_public_key,
                network_port: dora_node_info.network_port,
                rpc_port: dora_node_info.rpc_port,
            })
        }
    }

    #[test_only]
    fun set_up_test(owner_signer: &signer) {
        account::create_account_for_test(signer::address_of(owner_signer));
        init_module(owner_signer);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_committee_operations(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        // Add node to the committee
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        remove_committee(resource_address, owner_signer, 1);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_upsert_committee_member(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        upsert_committee_member(
            resource_address,
            owner_signer,
            1,
            @0x1,
            vector[123],
            vector[123],
            vector[123],
            vector[123],
            123,
            123
        );
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_remove_committee_member(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        upsert_committee_member(
            resource_address,
            owner_signer,
            1,
            @0x1,
            vector[123],
            vector[123],
            vector[123],
            vector[123],
            123,
            123
        );
        remove_committee_member(resource_address, owner_signer, 1, @0x1);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_upsert_committee_bulk(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee_bulk(
            resource_address,
            owner_signer,
            vector[1, 1],
            vector[vector[@0x1, @0x2], vector[@0x1, @0x2]],
            vector[vector[vector[123], vector[124]], vector[vector[125], vector[126]]],
            vector[vector[vector[123], vector[124]], vector[vector[125], vector[126]]],
            vector[vector[vector[123], vector[124]], vector[vector[125], vector[126]]],
            vector[vector[vector[123], vector[124]], vector[vector[125], vector[126]]],
            vector[vector[123, 124], vector[125, 126]],
            vector[vector[123, 124], vector[125, 126]],
            vector[1,1]
        );
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_remove_committee_bulk(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        remove_committee_bulk(resource_address, owner_signer, vector[1]);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_upsert_committee_member_bulk(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        upsert_committee_member_bulk(
            resource_address,
            owner_signer,
            vector[1],
            vector[@0x1],
            vector[vector[123]],
            vector[vector[123]],
            vector[vector[123]],
            vector[vector[123]],
            vector[123],
            vector[123]
        );
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_update_dkg_flag(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        update_dkg_flag(resource_address, owner_signer, 1, true);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_does_com_have_dkg(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        let flag = does_com_have_dkg(resource_address, 1);
        assert!(flag == false, 0);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_find_node_in_committee(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        let (flag, node_data) = find_node_in_committee(resource_address, 1, @0x1);
        assert!(flag == true, 0);
        assert!(node_data.operator == @0x1, 1);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_get_committee_info(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        let (node_data,_) = get_committee_info(resource_address, 1);
        assert!(vector::length(&node_data) == 2, 0);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_get_committee_ids(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        let ids = get_committee_ids(resource_address);
        assert!(vector::length(&ids) == 1, 0);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_get_committee_id_for_node(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        let id = get_committee_id_for_node(resource_address, @0x1);
        assert!(id == 1, 0);
    }

    #[test(owner_signer = @0xCEFEF)]
    public entry fun test_get_peers_for_node(
        owner_signer: &signer
    ) acquires CommitteeInfoStore, SupraCommitteeEventHandler {
        set_up_test(owner_signer);
        let resource_address = account::create_resource_address(&@0xCEFEF, SEED_COMMITTEE);
        upsert_committee(
            resource_address,
            owner_signer,
            1,
            vector[@0x1, @0x2],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[vector[123], vector[123]],
            vector[123, 123],
            vector[123, 123],
            1
        );
        let (_,peers) = get_peers_for_node(resource_address, @0x1);
        assert!(vector::length(&peers) == 1, 0);
    }
}
