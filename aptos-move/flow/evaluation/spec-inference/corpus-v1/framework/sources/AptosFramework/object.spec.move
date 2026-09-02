spec aptos_framework::object {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: It's not possible to create an object twice on the same address.
    /// Criticality: Critical
    /// Implementation: The create_object_internal function includes an assertion to ensure that the object being
    /// created does not already exist at the specified address.
    /// Enforcement: Formally verified via [high-level-req-1](create_object_internal).
    ///
    /// No.: 2
    /// Requirement: Only its owner may transfer an object.
    /// Criticality: Critical
    /// Implementation: The transfer function mandates that the transaction be signed by the owner's address, ensuring
    /// that only the rightful owner may initiate the object transfer.
    /// Enforcement: Audited that it aborts if anyone other than the owner attempts to transfer.
    ///
    /// No.: 3
    /// Requirement: The indirect owner of an object may transfer the object.
    /// Criticality: Medium
    /// Implementation: The owns function evaluates to true when the given address possesses either direct or indirect
    /// ownership of the specified object.
    /// Enforcement: Audited that it aborts if address transferring is not indirect owner.
    ///
    /// No.: 4
    /// Requirement: Objects may never change the address which houses them.
    /// Criticality: Low
    /// Implementation: After creating an object, transfers to another owner may occur. However, the address which
    /// stores the object may not be changed.
    /// Enforcement: This is implied by [high-level-req](high-level requirement 1).
    ///
    /// No.: 5
    /// Requirement: If an ungated transfer is disabled on an object in an indirect ownership chain, a transfer should not
    /// occur.
    /// Criticality: Medium
    /// Implementation: Calling disable_ungated_transfer disables direct transfer, and only TransferRef may trigger
    /// transfers. The transfer_with_ref function is called.
    /// Enforcement: Formally verified via [high-level-req-5](transfer_with_ref).
    ///
    /// No.: 6
    /// Requirement: Object addresses must not overlap with other addresses in different domains.
    /// Criticality: Critical
    /// Implementation: The current addressing scheme with suffixes does not conflict with any existing addresses,
    /// such as resource accounts. The GUID space is explicitly separated to ensure this doesn't happen.
    /// Enforcement: This is true by construction if one correctly ensures the usage of INIT_GUID_CREATION_NUM during
    /// the creation of GUID.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
    }

    // `exists_at` is a native Move operation because bytecode only permits
    // `exists<T>` in T's defining module. Specifications have global
    // visibility, so this predicate has exactly the denotation
    // `exists<T>(object)`. The Boogie backend recognizes this well-known
    // predicate and reads the instantiated resource memory directly.
    spec fun spec_exists_at<T: key>(object: address): bool;

    spec exists_at<T: key>(object: address): bool {
        pragma opaque;
        aborts_if false;
        ensures result == spec_exists_at<T>(object);
    }

    spec address_to_object<T: key>(object: address): Object<T> {
        pragma opaque = true;
        aborts_if !exists<ObjectCore>(object);
        aborts_if !spec_exists_at<T>(object);
        ensures result == Object<T> { inner: object };
    }

    spec create_object(owner_address: address): ConstructorRef {
        use 0x1::transaction_context;
        use 0x1::guid;
        use 0x1::event;
        pragma aborts_if_is_partial;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if exists<ObjectCore>(unique_address);

        ensures exists<ObjectCore>(unique_address);
        ensures global<ObjectCore>(unique_address)
            == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: owner_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: unique_address
                        }
                    }
                }
            };
        ensures result == ConstructorRef { self: unique_address, can_delete: true };
        pragma opaque = true;
        ensures [inferred] exists<ObjectCore>(
            transaction_context::spec_generate_unique_address()
        ) && ObjectCore[transaction_context::spec_generate_unique_address()]
            == ObjectCore {
                guid_creation_num: 1125899906842625,
                owner: owner_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle<TransferEvent> {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: 1125899906842624,
                            addr: transaction_context::spec_generate_unique_address()
                        }
                    }
                }
            } ==>
            result
                == ConstructorRef {
                    self: transaction_context::spec_generate_unique_address(),
                    can_delete: true
                };
        aborts_if [inferred] aborts_of<create_object_internal>(
            owner_address, transaction_context::spec_generate_unique_address(), true
        );
    }

    spec create_sticky_object(owner_address: address): ConstructorRef {
        use 0x1::transaction_context;
        use 0x1::guid;
        use 0x1::event;
        pragma aborts_if_is_partial;

        let unique_address = transaction_context::spec_generate_unique_address();
        aborts_if exists<ObjectCore>(unique_address);

        ensures exists<ObjectCore>(unique_address);
        ensures global<ObjectCore>(unique_address)
            == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: owner_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: unique_address
                        }
                    }
                }
            };
        ensures result == ConstructorRef { self: unique_address, can_delete: false };
        pragma opaque = true;
        ensures [inferred] exists<ObjectCore>(
            transaction_context::spec_generate_unique_address()
        ) && ObjectCore[transaction_context::spec_generate_unique_address()]
            == ObjectCore {
                guid_creation_num: 1125899906842625,
                owner: owner_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle<TransferEvent> {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: 1125899906842624,
                            addr: transaction_context::spec_generate_unique_address()
                        }
                    }
                }
            } ==>
            result
                == ConstructorRef {
                    self: transaction_context::spec_generate_unique_address(),
                    can_delete: false
                };
        aborts_if [inferred] aborts_of<create_object_internal>(
            owner_address, transaction_context::spec_generate_unique_address(), false
        );
    }

    spec create_object_address(source: &address, seed: vector<u8>): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_create_object_address(source, seed);
    }

    spec fun spec_create_user_derived_object_address_impl(source: address, derive_from: address): address;

    spec create_user_derived_object_address_impl(
        source: address, derive_from: address
    ): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result
            == spec_create_user_derived_object_address_impl(source, derive_from);
    }

    spec create_user_derived_object_address(
        source: address, derive_from: address
    ): address {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result
            == spec_create_user_derived_object_address(source, derive_from);
    }

    spec create_guid_object_address(source: address, creation_num: u64): address {
        use 0x1::bcs;
        use 0x1::from_bcs;
        use 0x1::hash;
        use 0x1::guid;
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result
            == spec_create_guid_object_address(source, creation_num);
        pragma aborts_if_is_partial = true;
        ensures [inferred] result
            == result_of<from_bcs::to_address>(
                hash::sha3_256(
                    concat(
                        bcs::to_bytes<guid::ID>(guid::create_id(source, creation_num)),
                        vec(253)
                    )
                )
            );
        aborts_if [inferred] aborts_of<from_bcs::to_address>(
            hash::sha3_256(
                concat(
                    bcs::to_bytes<guid::ID>(guid::create_id(source, creation_num)),
                    vec(253)
                )
            )
        );
    }

    spec object_address<T: key>(self: &Object<T>): address {
        pragma opaque = true;
        aborts_if false;
        ensures result == self.inner;
    }

    spec convert<X: key, Y: key>(self: Object<X>): Object<Y> {
        pragma opaque = true;
        aborts_if !exists<ObjectCore>(self.inner);
        aborts_if !spec_exists_at<Y>(self.inner);
        ensures result == Object<Y> { inner: self.inner };
    }

    spec create_named_object(creator: &signer, seed: vector<u8>): ConstructorRef {
        pragma opaque = true;
        let creator_address = signer::address_of(creator);
        let obj_addr = spec_create_object_address(creator_address, seed);
        modifies ObjectCore[obj_addr];
        aborts_if exists<ObjectCore>(obj_addr);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr)
            == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: creator_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr
                        }
                    }
                }
            };
        ensures result == ConstructorRef { self: obj_addr, can_delete: false };
    }

    spec create_user_derived_object(
        creator_address: address, derive_ref: &DeriveRef
    ): ConstructorRef {
        pragma opaque = true;
        let obj_addr = spec_create_user_derived_object_address(
            creator_address, derive_ref.self
        );
        modifies ObjectCore[obj_addr];
        aborts_if exists<ObjectCore>(obj_addr);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr)
            == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: creator_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr
                        }
                    }
                }
            };
        ensures result == ConstructorRef { self: obj_addr, can_delete: false };
    }

    spec create_object_from_account(creator: &signer): ConstructorRef {
        use 0x1::signer;
        use 0x1::bcs;
        use 0x1::from_bcs;
        use 0x1::hash;
        use 0x1::guid;
        use 0x1::event;
        use 0x1::account;
        use std::features;
        use std::features::DEFAULT_ACCOUNT_RESOURCE;
        let addr = signer::address_of(creator);
        let account_exists_pre = exists<account::Account>(addr);
        let feature_on = features::spec_is_enabled(DEFAULT_ACCOUNT_RESOURCE);

        aborts_if !account_exists_pre && !feature_on;
        aborts_if !account_exists_pre
            && feature_on
            && (addr == @vm_reserved
                || addr == @aptos_framework
                || addr == @aptos_token);
        aborts_if !account_exists_pre
            && feature_on
            && len(bcs::to_bytes(addr)) != 32;

        let creation_num = if (account_exists_pre) {
            global<account::Account>(addr).guid_creation_num
        } else { 2 };
        aborts_if creation_num + 1 >= account::MAX_GUID_CREATION_NUM;

        let guid = guid::GUID { id: guid::ID { creation_num, addr } };
        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures exists<account::Account>(addr);
        ensures global<account::Account>(addr).guid_creation_num == creation_num + 1;
        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr)
            == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: addr,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr
                        }
                    }
                }
            };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = {
                let b = ..S1 |~ result_of<account::create_guid>(creator);
                S1.. |~ result_of<create_object_from_guid>(signer::address_of(creator), b)
            };
            exists<account::Account>(signer::address_of(creator))
                && (
                    account::Account[signer::address_of(creator)].guid_creation_num
                        == account::Account[signer::address_of(creator)].guid_creation_num
                        + 1
                        && (
                            exists<ObjectCore>(
                                from_bcs::deserialize<address>(
                                    hash::sha3_256(
                                        concat(
                                            bcs::to_bytes<guid::GUID>(
                                                guid::GUID {
                                                    id: guid::ID {
                                                        creation_num: account::Account[signer::address_of(
                                                            creator
                                                        )].guid_creation_num,
                                                        addr: signer::address_of(creator)
                                                    }
                                                }
                                            ),
                                            vec(253)
                                        )
                                    )
                                )
                            )
                                && (
                                    ObjectCore[
                                        from_bcs::deserialize<address>(
                                            hash::sha3_256(
                                                concat(
                                                    bcs::to_bytes<guid::GUID>(
                                                        guid::GUID {
                                                            id: guid::ID {
                                                                creation_num: account::Account[signer::address_of(
                                                                    creator
                                                                )].guid_creation_num,
                                                                addr: signer::address_of(
                                                                    creator
                                                                )
                                                            }
                                                        }
                                                    ),
                                                    vec(253)
                                                )
                                            )
                                        )
                                    ] == ObjectCore {
                                        guid_creation_num: 1125899906842625,
                                        owner: signer::address_of(creator),
                                        allow_ungated_transfer: true,
                                        transfer_events: event::EventHandle<TransferEvent> {
                                            counter: 0,
                                            guid: guid::GUID {
                                                id: guid::ID {
                                                    creation_num: 1125899906842624,
                                                    addr: from_bcs::deserialize<address>(
                                                        hash::sha3_256(
                                                            concat(
                                                                bcs::to_bytes<guid::GUID>(
                                                                    guid::GUID {
                                                                        id: guid::ID {
                                                                            creation_num: account::Account[signer::address_of(
                                                                                creator
                                                                            )].guid_creation_num,
                                                                            addr: signer::address_of(
                                                                                creator
                                                                            )
                                                                        }
                                                                    }
                                                                ),
                                                                vec(253)
                                                            )
                                                        )
                                                    )
                                                }
                                            }
                                        }
                                    }
                                        && a
                                            == ConstructorRef {
                                                self: from_bcs::deserialize<address>(
                                                    hash::sha3_256(
                                                        concat(
                                                            bcs::to_bytes<guid::GUID>(
                                                                guid::GUID {
                                                                    id: guid::ID {
                                                                        creation_num: account::Account[signer::address_of(
                                                                            creator
                                                                        )].guid_creation_num,
                                                                        addr: signer::address_of(
                                                                            creator
                                                                        )
                                                                    }
                                                                }
                                                            ),
                                                            vec(253)
                                                        )
                                                    )
                                                ),
                                                can_delete: true
                                            }
                                )
                        )
                ) ==> result == a
        });
        aborts_if [inferred] aborts_of<account::create_guid>(creator);
    }

    spec create_object_from_object(creator: &signer): ConstructorRef {
        use 0x1::signer;
        use 0x1::bcs;
        use 0x1::from_bcs;
        use 0x1::hash;
        use 0x1::guid;
        use 0x1::event;
        aborts_if !exists<ObjectCore>(signer::address_of(creator));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(creator));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;
        let creation_num = object_data.guid_creation_num;
        let addr = signer::address_of(creator);

        let guid = guid::GUID { id: guid::ID { creation_num, addr } };

        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures global<ObjectCore>(addr).guid_creation_num
            == old(global<ObjectCore>(addr)).guid_creation_num + 1;
        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr)
            == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: addr,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr
                        }
                    }
                }
            };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = {
                let b = ..S1 |~ result_of<create_guid>(creator);
                S1.. |~ result_of<create_object_from_guid>(signer::address_of(creator), b)
            };
            ObjectCore[signer::address_of(creator)].guid_creation_num
                == ObjectCore[signer::address_of(creator)].guid_creation_num + 1
                && (
                    exists<ObjectCore>(
                        from_bcs::deserialize<address>(
                            hash::sha3_256(
                                concat(
                                    bcs::to_bytes<guid::GUID>(
                                        guid::GUID {
                                            id: guid::ID {
                                                creation_num: ObjectCore[signer::address_of(
                                                    creator
                                                )].guid_creation_num,
                                                addr: signer::address_of(creator)
                                            }
                                        }
                                    ),
                                    vec(253)
                                )
                            )
                        )
                    )
                        && (
                            ObjectCore[
                                from_bcs::deserialize<address>(
                                    hash::sha3_256(
                                        concat(
                                            bcs::to_bytes<guid::GUID>(
                                                guid::GUID {
                                                    id: guid::ID {
                                                        creation_num: ObjectCore[signer::address_of(
                                                            creator
                                                        )].guid_creation_num,
                                                        addr: signer::address_of(creator)
                                                    }
                                                }
                                            ),
                                            vec(253)
                                        )
                                    )
                                )
                            ] == ObjectCore {
                                guid_creation_num: 1125899906842625,
                                owner: signer::address_of(creator),
                                allow_ungated_transfer: true,
                                transfer_events: event::EventHandle<TransferEvent> {
                                    counter: 0,
                                    guid: guid::GUID {
                                        id: guid::ID {
                                            creation_num: 1125899906842624,
                                            addr: from_bcs::deserialize<address>(
                                                hash::sha3_256(
                                                    concat(
                                                        bcs::to_bytes<guid::GUID>(
                                                            guid::GUID {
                                                                id: guid::ID {
                                                                    creation_num: ObjectCore[signer::address_of(
                                                                        creator
                                                                    )].guid_creation_num,
                                                                    addr: signer::address_of(
                                                                        creator
                                                                    )
                                                                }
                                                            }
                                                        ),
                                                        vec(253)
                                                    )
                                                )
                                            )
                                        }
                                    }
                                }
                            }
                                && a
                                    == ConstructorRef {
                                        self: from_bcs::deserialize<address>(
                                            hash::sha3_256(
                                                concat(
                                                    bcs::to_bytes<guid::GUID>(
                                                        guid::GUID {
                                                            id: guid::ID {
                                                                creation_num: ObjectCore[signer::address_of(
                                                                    creator
                                                                )].guid_creation_num,
                                                                addr: signer::address_of(
                                                                    creator
                                                                )
                                                            }
                                                        }
                                                    ),
                                                    vec(253)
                                                )
                                            )
                                        ),
                                        can_delete: true
                                    }
                        )
                ) ==> result == a
        });
    }

    spec create_object_from_guid(
        creator_address: address, guid: guid::GUID
    ): ConstructorRef {
        use 0x1::bcs;
        use 0x1::from_bcs;
        use 0x1::hash;
        use 0x1::guid;
        use 0x1::event;
        let bytes_spec = bcs::to_bytes(guid);
        let bytes = concat(bytes_spec, vec<u8>(OBJECT_FROM_GUID_ADDRESS_SCHEME));
        let hash_bytes = hash::sha3_256(bytes);
        let obj_addr = from_bcs::deserialize<address>(hash_bytes);
        aborts_if exists<ObjectCore>(obj_addr);
        aborts_if !from_bcs::deserializable<address>(hash_bytes);

        ensures exists<ObjectCore>(obj_addr);
        ensures global<ObjectCore>(obj_addr)
            == ObjectCore {
                guid_creation_num: INIT_GUID_CREATION_NUM + 1,
                owner: creator_address,
                allow_ungated_transfer: true,
                transfer_events: event::EventHandle {
                    counter: 0,
                    guid: guid::GUID {
                        id: guid::ID {
                            creation_num: INIT_GUID_CREATION_NUM,
                            addr: obj_addr
                        }
                    }
                }
            };
        ensures result == ConstructorRef { self: obj_addr, can_delete: true };
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] exists<ObjectCore>(
            from_bcs::deserialize<address>(
                hash::sha3_256(
                    concat(bcs::to_bytes<guid::GUID>(guid), vec(253))
                )
            )
        )
            && (
                ObjectCore[
                    from_bcs::deserialize<address>(
                        hash::sha3_256(
                            concat(bcs::to_bytes<guid::GUID>(guid), vec(253))
                        )
                    )
                ] == ObjectCore {
                    guid_creation_num: 1125899906842625,
                    owner: creator_address,
                    allow_ungated_transfer: true,
                    transfer_events: event::EventHandle<TransferEvent> {
                        counter: 0,
                        guid: guid::GUID {
                            id: guid::ID {
                                creation_num: 1125899906842624,
                                addr: from_bcs::deserialize<address>(
                                    hash::sha3_256(
                                        concat(
                                            bcs::to_bytes<guid::GUID>(guid),
                                            vec(253)
                                        )
                                    )
                                )
                            }
                        }
                    }
                }
                    && ConstructorRef {
                        self: result_of<from_bcs::to_address>(
                            hash::sha3_256(
                                concat(bcs::to_bytes<guid::GUID>(guid), vec(253))
                            )
                        ),
                        can_delete: true
                    } == ConstructorRef {
                        self: from_bcs::deserialize<address>(
                            hash::sha3_256(
                                concat(bcs::to_bytes<guid::GUID>(guid), vec(253))
                            )
                        ),
                        can_delete: true
                    }
            ) ==>
            result
                == ConstructorRef {
                    self: result_of<from_bcs::to_address>(
                        hash::sha3_256(concat(bcs::to_bytes<guid::GUID>(guid), vec(253)))
                    ),
                    can_delete: true
                };
        aborts_if [inferred] aborts_of<create_object_internal>(
            creator_address,
            result_of<from_bcs::to_address>(
                hash::sha3_256(concat(bcs::to_bytes<guid::GUID>(guid), vec(253)))
            ),
            true
        );
        aborts_if [inferred] aborts_of<from_bcs::to_address>(
            hash::sha3_256(concat(bcs::to_bytes<guid::GUID>(guid), vec(253)))
        );
    }

    spec create_sticky_object_at_address(
        owner_address: address, object_address: address
    ): ConstructorRef {
        pragma opaque = true;
        modifies ObjectCore[object_address];
        /// [high-level-req-1]
        aborts_if exists<ObjectCore>(object_address);
        ensures exists<ObjectCore>(object_address);
        ensures global<ObjectCore>(object_address).guid_creation_num
            == INIT_GUID_CREATION_NUM + 1;
        ensures result == ConstructorRef { self: object_address, can_delete: false };
    }

    spec create_object_internal(
        creator_address: address, object: address, can_delete: bool
    ): ConstructorRef {
        pragma opaque = true;
        modifies ObjectCore[object];
        // property 1: Creating an object twice on the same address must never occur.
        /// [high-level-req-1]
        aborts_if exists<ObjectCore>(object);
        ensures exists<ObjectCore>(object);
        // property 6: Object addresses must not overlap with other addresses in different domains.
        ensures global<ObjectCore>(object).guid_creation_num
            == INIT_GUID_CREATION_NUM + 1;
        ensures result == ConstructorRef { self: object, can_delete };
    }

    spec generate_delete_ref(self: &ConstructorRef): DeleteRef {
        aborts_if !self.can_delete;
        ensures result == DeleteRef { self: self.self };
        pragma opaque = true;
        ensures [inferred] self.can_delete ==>
            result == DeleteRef { self: self.self };
        aborts_if [inferred]!self.can_delete;
    }

    spec disable_ungated_transfer(self: &TransferRef) {
        pragma opaque = true;
        modifies ObjectCore[self.self];
        aborts_if !exists<ObjectCore>(self.self);
        ensures global<ObjectCore>(self.self).allow_ungated_transfer == false;
    }

    spec object_from_constructor_ref<T: key>(self: &ConstructorRef): Object<T> {
        pragma opaque = true;
        aborts_if !exists<ObjectCore>(self.self);
        aborts_if !spec_exists_at<T>(self.self);
        ensures result == Object<T> { inner: self.self };
    }

    spec create_guid(object: &signer): guid::GUID {
        use 0x1::signer;
        use 0x1::guid;
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;

        ensures result
            == guid::GUID {
                id: guid::ID {
                    creation_num: object_data.guid_creation_num,
                    addr: signer::address_of(object)
                }
            };
        pragma opaque = true, aborts_if_is_partial = true;
        modifies ObjectCore[signer::address_of(object)];
        ensures [inferred] result_of<guid::create>(
            signer::address_of(object),
            old(ObjectCore[signer::address_of(object)]).guid_creation_num
        ) == guid::GUID {
            id: guid::ID {
                creation_num: ObjectCore[signer::address_of(object)].guid_creation_num,
                addr: signer::address_of(object)
            }
        } ==>
            result
                == result_of<guid::create>(
                    signer::address_of(object),
                    old(ObjectCore[signer::address_of(object)]).guid_creation_num
                );
        ensures [inferred] ObjectCore[signer::address_of(object)].guid_creation_num
            == ObjectCore[signer::address_of(object)].guid_creation_num;
        aborts_if [inferred] aborts_of<guid::create>(
            signer::address_of(object),
            ObjectCore[signer::address_of(object)].guid_creation_num
        );
        aborts_if [inferred]!exists<ObjectCore>(signer::address_of(object));
    }

    spec new_event_handle<T: drop + store>(object: &signer): event::EventHandle<T> {
        use 0x1::signer;
        use 0x1::guid;
        use 0x1::event;
        aborts_if !exists<ObjectCore>(signer::address_of(object));
        //Guid properties
        let object_data = global<ObjectCore>(signer::address_of(object));
        aborts_if object_data.guid_creation_num + 1 > MAX_U64;

        let guid = guid::GUID {
            id: guid::ID {
                creation_num: object_data.guid_creation_num,
                addr: signer::address_of(object)
            }
        };
        ensures result == event::EventHandle<T> { counter: 0, guid };
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] event::new_event_handle<T>(result_of<create_guid>(object))
            == event::EventHandle<T> {
                counter: 0,
                guid: guid::GUID {
                    id: guid::ID {
                        creation_num: ObjectCore[signer::address_of(object)].guid_creation_num,
                        addr: signer::address_of(object)
                    }
                }
            } ==>
            result == event::new_event_handle<T>(result_of<create_guid>(object));
    }

    spec object_from_delete_ref<T: key>(self: &DeleteRef): Object<T> {
        aborts_if !exists<ObjectCore>(self.self);
        aborts_if !spec_exists_at<T>(self.self);
        ensures result == Object<T> { inner: self.self };
        pragma opaque = true;
        ensures [inferred] result == Object<T> { inner: self.self };
        aborts_if [inferred] aborts_of<address_to_object<T>> (self.self);
    }

    spec delete(self: DeleteRef) {
        use 0x1::event;
        aborts_if !exists<ObjectCore>(self.self);
        ensures !exists<ObjectCore>(self.self);
        pragma opaque = true;
        modifies Untransferable[self.self];
        ensures [inferred] S1.. |~(
            ensures_of<event::destroy_handle<TransferEvent>> (
                old(ObjectCore[self.self]).transfer_events
            )
        );
        ensures [inferred](S1 |~ exists<Untransferable>(self.self)) ==>
            (S1.. |~ remove<Untransferable>(self.self));
        ensures [inferred]..S1 |~ remove<ObjectCore>(self.self);
        aborts_if [inferred]!exists<ObjectCore>(self.self);
    }

    spec generate_signer_for_extending {
        use 0x1::create_signer;
        pragma opaque;
        ensures result == spec_generate_signer_for_extending(self);
        ensures [inferred] result == create_signer::spec_create_signer(self.self);
        aborts_if [inferred] false;
    }

    spec set_untransferable(self: &ConstructorRef) {
        pragma opaque = true;
        modifies ObjectCore[self.self];
        modifies Untransferable[self.self];
        aborts_if !exists<ObjectCore>(self.self);
        aborts_if exists<Untransferable>(self.self);
        ensures exists<ObjectCore>(self.self);
        ensures exists<Untransferable>(self.self);
        ensures global<ObjectCore>(self.self).allow_ungated_transfer == false;
    }

    spec enable_ungated_transfer(self: &TransferRef) {
        aborts_if exists<Untransferable>(self.self);
        aborts_if !exists<ObjectCore>(self.self);
        ensures global<ObjectCore>(self.self).allow_ungated_transfer == true;
        pragma opaque = true;
        modifies ObjectCore[self.self];
        ensures [inferred]!old(exists<Untransferable>(self.self)) ==>
            update<ObjectCore>(
                self.self,
                update_field(old(ObjectCore[self.self]), allow_ungated_transfer, true)
            );
        aborts_if [inferred]!exists<Untransferable>(self.self);
        aborts_if [inferred] exists<Untransferable>(self.self);
    }

    spec generate_transfer_ref(self: &ConstructorRef): TransferRef {
        pragma opaque = true;
        aborts_if exists<Untransferable>(self.self);
        ensures result == TransferRef { self: self.self };
    }

    spec generate_linear_transfer_ref(self: &TransferRef): LinearTransferRef {
        aborts_if exists<Untransferable>(self.self);
        aborts_if !exists<ObjectCore>(self.self);
        let owner = global<ObjectCore>(self.self).owner;
        ensures result == LinearTransferRef { self: self.self, owner };
        pragma opaque = true;
        let cse_ = Object<ObjectCore> { inner: self.self };
        ensures [inferred]!exists<Untransferable>(self.self)
            && LinearTransferRef {
                self: self.self,
                owner: result_of<0x1::object::owner<ObjectCore>> (cse_)
            } == LinearTransferRef {
                self: self.self,
                owner: ObjectCore[self.self].owner
            } ==>
            result
                == LinearTransferRef {
                    self: self.self,
                    owner: result_of<0x1::object::owner<ObjectCore>> (cse_)
                };
        aborts_if [inferred]!exists<Untransferable>(self.self)
            && aborts_of<0x1::object::owner<ObjectCore>> (cse_);
        aborts_if [inferred] exists<Untransferable>(self.self);
    }

    spec transfer_with_ref(self: LinearTransferRef, to: address) {
        use 0x1::event;
        aborts_if exists<Untransferable>(self.self);
        let object = global<ObjectCore>(self.self);
        aborts_if !exists<ObjectCore>(self.self);
        /// [high-level-req-5]
        aborts_if object.owner != self.owner;
        ensures global<ObjectCore>(self.self).owner == to;
        pragma opaque = true;
        modifies ObjectCore[self.self];
        ensures [inferred]!exists<Untransferable>(self.self)
            && ObjectCore[self.self].owner == self.owner ==>
            ObjectCore[self.self].owner == to;
        ensures [inferred]!exists<Untransferable>(self.self)
            && ObjectCore[self.self].owner == self.owner ==>
            ensures_of<event::emit<Transfer>> (
                Transfer {
                    object: self.self,
                    from: ObjectCore[self.self].owner,
                    to: to
                }
            );
        aborts_if [inferred] exists<Untransferable>(self.self);
        aborts_if [inferred]!exists<Untransferable>(self.self);
    }

    spec transfer_call(owner: &signer, object: address, to: address) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        aborts_if !exists<ObjectCore>(object);
        aborts_if !global<ObjectCore>(object).allow_ungated_transfer;
        pragma opaque = true;
        modifies ObjectCore[object];
        ensures [inferred] ensures_of<transfer_raw>(owner, object, to);
    }

    spec transfer<T: key>(owner: &signer, object: Object<T>, to: address) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !global<ObjectCore>(object_address).allow_ungated_transfer;
        pragma opaque = true;
        modifies ObjectCore[object.inner];
        ensures [inferred] ensures_of<transfer_raw>(owner, object.inner, to);
    }

    spec transfer_raw(owner: &signer, object: address, to: address) {
        use 0x1::signer;
        use 0x1::event;
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        aborts_if !exists<ObjectCore>(object);
        aborts_if !global<ObjectCore>(object).allow_ungated_transfer;
        pragma opaque = true;
        modifies ObjectCore[object];
        ensures [inferred]({
            let a = S1 |~ global<ObjectCore>(object);
            a.owner != to ==> ObjectCore[object].owner == to
        });
        ensures [inferred](S1 |~ global<ObjectCore>(object)).owner != to ==>
            {
                let a = S1 |~ global<ObjectCore>(object);
                S1.. |~ ensures_of<event::emit<Transfer>> (
                    Transfer { object: object, from: a.owner, to: to }
                )
            };
        ensures [inferred]..S1 |~(
            ensures_of<verify_ungated_and_descendant>(signer::address_of(owner), object)
        );
        aborts_if [inferred](S1 |~ global<ObjectCore>(object)).owner != to
            && {
                let a = S1 |~ global<ObjectCore>(object);
                S1 |~ aborts_of<event::emit<Transfer>> (
                    Transfer { object: object, from: a.owner, to: to }
                )
            };
        aborts_if [inferred] S1 |~(!exists<ObjectCore>(object));
    }

    spec transfer_to_object<O: key, T: key>(
        owner: &signer, object: Object<O>, to: Object<T>
    ) {
        pragma aborts_if_is_partial;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        let owner_address = signer::address_of(owner);
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !global<ObjectCore>(object_address).allow_ungated_transfer;
        pragma opaque = true;
        modifies ObjectCore[object.inner];
        ensures [inferred] ensures_of<transfer<O>> (owner, object, to.inner);
    }

    spec burn<T: key>(owner: &signer, object: Object<T>) {
        use 0x1::signer;
        use 0x1::create_signer;
        pragma aborts_if_is_partial;
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if owner(object) != signer::address_of(owner);
        ensures exists<TombStone>(object_address);
        ensures is_owner(object, signer::address_of(owner));
        pragma opaque = true;
        modifies TombStone[
            signer::address_of(create_signer::spec_create_signer(object.inner))
        ];
        ensures [inferred](..S2 |~ result_of<is_owner<T>> (
            object, signer::address_of(owner)
        )) && (S2 |~ !exists<TombStone>(object.inner)) ==>
            {
                let a =
                    signer::address_of(create_signer::spec_create_signer(object.inner));
                let b = TombStone { original_owner: signer::address_of(owner) };
                S2.. |~ publish<TombStone>(a, b)
            };
        aborts_if [inferred]..S2 |~(
            !result_of<is_owner<T>> (object, signer::address_of(owner))
        );
        aborts_if [inferred](
            ..S2 |~ result_of<is_owner<T>> (object, signer::address_of(owner))
        )
            && (
                (S2 |~ !exists<TombStone>(object.inner))
                    && (
                        S2 |~ exists<TombStone>(
                            signer::address_of(
                                create_signer::spec_create_signer(object.inner)
                            )
                        )
                    )
            );
        aborts_if [inferred](
            ..S2 |~ result_of<is_owner<T>> (object, signer::address_of(owner))
        ) && (S2 |~ exists<TombStone>(object.inner));
        aborts_if [inferred] aborts_of<is_owner<T>> (object, signer::address_of(owner));
    }

    spec burn_object_with_transfer<T: key>(
        owner: &signer, object: Object<T>
    ) {
        pragma aborts_if_is_partial;
        let object_address = object.inner;
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if owner(object) != signer::address_of(owner);
        aborts_if is_burnt(object);
    }

    spec unburn<T: key>(original_owner: &signer, object: Object<T>) {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        let object_address = object.inner;
        modifies TombStone[object_address];
        modifies ObjectCore[object_address];
        aborts_if !exists<ObjectCore>(object_address);
        aborts_if !is_burnt(object);
        let tomb_stone = borrow_global<TombStone>(object_address);
        let original_owner_address = signer::address_of(original_owner);
        let object_current_owner = borrow_global<ObjectCore>(object_address).owner;
        aborts_if object_current_owner != original_owner_address
            && tomb_stone.original_owner != original_owner_address;
        // Keep the direct forms alongside the named predicates so the prover's
        // abort-coverage pass can see the resource facts at the branch site.
        aborts_if exists<ObjectCore>(object_address)
            && !exists<TombStone>(object_address);
        aborts_if exists<ObjectCore>(object_address)
            && exists<TombStone>(object_address)
            && ObjectCore[object_address].owner != signer::address_of(original_owner)
            && (
                ObjectCore[object_address].owner != BURN_ADDRESS
                    || TombStone[object_address].original_owner
                        != signer::address_of(original_owner)
            );
        aborts_if exists<ObjectCore>(object_address)
            && exists<TombStone>(object_address)
            && ObjectCore[object_address].owner == BURN_ADDRESS
            && TombStone[object_address].original_owner
                == signer::address_of(original_owner)
            && ObjectCore[object_address].owner
                != TombStone[object_address].original_owner
            && aborts_of<event::emit<Transfer>> (
                Transfer {
                    object: object_address,
                    from: ObjectCore[object_address].owner,
                    to: TombStone[object_address].original_owner
                }
            );
    }

    spec verify_ungated_and_descendant(
        owner: address, destination: address
    ) {
        use 0x1::error;
        // TODO: Verify the link list loop in verify_ungated_and_descendant
        pragma aborts_if_is_partial;
        pragma unroll = MAXIMUM_OBJECT_NESTING;
        aborts_if !exists<ObjectCore>(destination);
        aborts_if !global<ObjectCore>(destination).allow_ungated_transfer;
        // aborts_if exists i in 0..g_roll:
        //     owner != global<ObjectCore>(destination).owner && !exists<ObjectCore>(get_transfer_address(destination, i));
        // aborts_if exists i in 0..g_roll:
        //     owner != global<ObjectCore>(destination).owner && !global<ObjectCore>(get_transfer_address(destination, i)).allow_ungated_transfer;
        // property 3: The 'indirect' owner of an object may transfer the object.
        // ensures exists i in 0..MAXIMUM_OBJECT_NESTING:
        //     owner == get_transfer_address(destination, i);
        pragma opaque = true;
        aborts_if [inferred = sathard]!exists<ObjectCore>(destination);
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && !ObjectCore[destination].allow_ungated_transfer;
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (
                !ObjectCore[destination].allow_ungated_transfer
                    && aborts_of<error::permission_denied>(3)
            );
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (ObjectCore[destination].allow_ungated_transfer
                && (exists x: address: owner != x));
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (
                ObjectCore[destination].allow_ungated_transfer
                    && (aborts_of<error::out_of_range>(6)
                        && (exists x: address: owner != x))
            );
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (
                ObjectCore[destination].allow_ungated_transfer
                    && (exists x: address: owner != x
                        && !exists<ObjectCore>(x))
            );
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (
                ObjectCore[destination].allow_ungated_transfer
                    && (
                        aborts_of<error::permission_denied>(4)
                            && (exists x: address: owner != x
                                && !exists<ObjectCore>(x))
                    )
            );
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (
                ObjectCore[destination].allow_ungated_transfer
                    && (
                        exists x: address:
                            owner != x
                                && exists<ObjectCore>(x)
                                && !ObjectCore[x].allow_ungated_transfer
                    )
            );
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (
                ObjectCore[destination].allow_ungated_transfer
                    && (
                        aborts_of<error::permission_denied>(3)
                            && (
                                exists x: address:
                                    owner != x
                                        && exists<ObjectCore>(x)
                                        && !ObjectCore[x].allow_ungated_transfer
                            )
                    )
            );
        aborts_if [inferred = sathard] exists<ObjectCore>(destination)
            && (
                ObjectCore[destination].allow_ungated_transfer
                    && (
                        exists x: address:
                            owner != x
                                && exists<ObjectCore>(x)
                                && !exists<ObjectCore>(x)
                    )
            );
    }

    // Helper function for property 3
    // spec fun get_transfer_address(addr: address, roll: u64): address {
    //     let i = roll;
    //     if ( i > 0 )
    //     { get_transfer_address(global<ObjectCore>(addr).owner, i - 1) }
    //     else
    //     { global<ObjectCore>(addr).owner }
    // }

    spec ungated_transfer_allowed<T: key>(self: Object<T>): bool {
        aborts_if !exists<ObjectCore>(self.inner);
        ensures result == global<ObjectCore>(self.inner).allow_ungated_transfer;
        pragma opaque = true;
        ensures [inferred] exists<ObjectCore>(self.inner) ==>
            result == ObjectCore[self.inner].allow_ungated_transfer;
        aborts_if [inferred]!exists<ObjectCore>(self.inner);
    }

    spec is_owner<T: key>(object: Object<T>, owner: address): bool {
        pragma opaque = true;
        aborts_if !exists<ObjectCore>(object.inner);
        ensures result == (global<ObjectCore>(object.inner).owner == owner);
    }

    spec owner<T: key>(self: Object<T>): address {
        pragma opaque = true;
        aborts_if !exists<ObjectCore>(self.inner);
        ensures result == global<ObjectCore>(self.inner).owner;
    }

    spec owns<T: key>(object: Object<T>, owner: address): bool {
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        aborts_if !exists<ObjectCore>(object.inner);
        aborts_if ownership_chain_exists(object.inner, MAXIMUM_OBJECT_NESTING)
            && !ownership_reaches(object.inner, owner, MAXIMUM_OBJECT_NESTING);
        ensures result
            == ownership_reaches(object.inner, owner, MAXIMUM_OBJECT_NESTING);
    }

    spec root_owner<T: key>(self: Object<T>): address {
        pragma opaque = true;
        aborts_if !exists<ObjectCore>(self.inner);
        // The ownership path is deterministic.  Together, these clauses say
        // that the result is exactly the first reachable address that is not
        // an object, while all preceding addresses are ObjectCore values.
        ensures !exists<ObjectCore>(result);
        ensures exists depth: num:
            depth >= 0
                && result == spec_owner_at(self.inner, depth + 1)
                && (
                    forall step in 0..depth + 1:
                        exists<ObjectCore>(spec_owner_at(self.inner, step))
                );
    } proof {
        apply spec_owner_at_zero(self.inner);
        forall depth: num apply spec_owner_at_step(self.inner, depth);
    }

    /// The address reached after following `depth` owner links.  Uses of this
    /// function are guarded by `exists<ObjectCore>` for every dereferenced
    /// prefix address.
    spec fun spec_owner_at(address: address, depth: num): address {
        if (depth == 0) address
        else ObjectCore[spec_owner_at(address, depth - 1)].owner
    }

    /// True when every address that must be dereferenced to follow `depth`
    /// ownership links has an ObjectCore.  The endpoint itself need not be an
    /// object: the loop checks it only after following the preceding links.
    spec fun ownership_chain_exists(address: address, depth: num): bool {
        if (depth == 0) true
        else
            exists<ObjectCore>(address)
                && ownership_chain_exists(ObjectCore[address].owner, depth - 1)
    }

    /// Whether `owner` is encountered within at most `depth` ownership links.
    /// A missing ObjectCore terminates the search with `false`; a chain that
    /// stays object-backed beyond the permitted bound is captured separately
    /// by the `owns` abort condition.
    spec fun ownership_reaches(address: address, owner: address, depth: num): bool {
        if (address == owner) true
        else if (depth == 0 || !exists<ObjectCore>(address)) false
        else ownership_reaches(ObjectCore[address].owner, owner, depth - 1)
    }

    spec lemma spec_owner_at_zero(address: address) {
        ensures spec_owner_at(address, 0) == address;
    }

    spec lemma spec_owner_at_step(address: address, depth: num) {
        ensures depth >= 0 ==> spec_owner_at(address, depth + 1) == ObjectCore[spec_owner_at(
            address, depth
        )].owner;
    }

    // Helper function
    spec fun spec_create_object_address(source: address, seed: vector<u8>): address;

    spec fun spec_create_user_derived_object_address(source: address, derive_from: address): address;

    spec fun spec_create_guid_object_address(source: address, creation_num: u64): address;

    spec fun spec_generate_signer_for_extending(ref: &ExtendRef): signer {
        aptos_framework::create_signer::spec_create_signer(ref.self)
    }

    spec address_from_constructor_ref(
        self: &0x1::object::ConstructorRef
    ): address {
        pragma opaque = true;
        ensures [inferred] result == self.self;
        aborts_if [inferred] false;
    }

    spec address_from_delete_ref(self: &0x1::object::DeleteRef): address {
        pragma opaque = true;
        ensures [inferred] result == self.self;
        aborts_if [inferred] false;
    }

    spec address_from_extend_ref(self: &0x1::object::ExtendRef): address {
        pragma opaque = true;
        ensures [inferred] result == self.self;
        aborts_if [inferred] false;
    }

    spec can_generate_delete_ref(self: &0x1::object::ConstructorRef): bool {
        pragma opaque = true;
        ensures [inferred] result == self.can_delete;
        aborts_if [inferred] false;
    }

    spec create_named_unowned_onchain_signer(
        creator: &signer, name: vector<u8>
    ): 0x1::object::ExtendRef {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred]({
            let a = ..S1 |~ result_of<create_named_object>(creator, name);
            result == generate_extend_ref(a)
        });
        ensures [inferred]({
            let a = ..S1 |~ result_of<create_named_object>(creator, name);
            S1.. |~ ensures_of<transfer_with_constructor_ref>(a, @0x0)
        });
        aborts_if [inferred]({
            let a = ..S1 |~ result_of<create_named_object>(creator, name);
            S1 |~ aborts_of<transfer_with_constructor_ref>(a, @0x0)
        });
    }

    spec create_unique_onchain_signer(): 0x1::object::ExtendRef {
        use 0x1::transaction_context;
        pragma opaque = true;
        ensures [inferred] result
            == ExtendRef {
                self: transaction_context::spec_generate_unique_address()
            };
        aborts_if [inferred] false;
    }

    spec generate_derive_ref(self: &0x1::object::ConstructorRef): 0x1::object::DeriveRef {
        pragma opaque = true;
        ensures [inferred] result == DeriveRef { self: self.self };
        aborts_if [inferred] false;
    }

    spec generate_extend_ref(self: &0x1::object::ConstructorRef): 0x1::object::ExtendRef {
        pragma opaque = true;
        ensures [inferred] result == ExtendRef { self: self.self };
        aborts_if [inferred] false;
    }

    spec generate_signer(self: &0x1::object::ConstructorRef): signer {
        use 0x1::create_signer;
        use 0x1::signer;
        pragma opaque = true;
        ensures [inferred] result == create_signer::spec_create_signer(self.self);
        // State this address relation directly rather than relying on the
        // uninterpreted `spec_create_signer` helper to expose it to callers.
        ensures signer::address_of(result) == self.self;
        aborts_if [inferred] false;
    }

    spec is_burnt<T: key>(self: 0x1::object::Object<T>): bool {
        pragma opaque = true;
        ensures [inferred] result == exists<TombStone>(self.inner);
        aborts_if [inferred] false;
    }

    spec is_object(object: address): bool {
        pragma opaque = true;
        ensures [inferred] result == exists<ObjectCore>(object);
        aborts_if [inferred] false;
    }

    spec is_untransferable<T: key>(self: 0x1::object::Object<T>): bool {
        pragma opaque = true;
        ensures [inferred] result == exists<Untransferable>(self.inner);
        aborts_if [inferred] false;
    }

    spec object_exists<T: key>(object: address): bool {
        pragma opaque = true;
        ensures [inferred] exists<ObjectCore>(object) ==>
            result == spec_exists_at<T>(object);
        ensures [inferred]!exists<ObjectCore>(object) ==> result == false;
        aborts_if [inferred] false;
    }

    spec transfer_with_constructor_ref(
        self: &0x1::object::ConstructorRef, to: address
    ) {
        pragma opaque = true;
        let cse_ = TransferRef { self: self.self };
        ensures [inferred]({
            let a = ..S2 |~ result_of<generate_linear_transfer_ref>(cse_);
            S2.. |~ ensures_of<transfer_with_ref>(a, to)
        });
        aborts_if [inferred]({
            let a = ..S2 |~ result_of<generate_linear_transfer_ref>(cse_);
            S2 |~ aborts_of<transfer_with_ref>(a, to)
        });
        aborts_if [inferred] aborts_of<generate_linear_transfer_ref>(cse_);
        aborts_if [inferred] aborts_of<generate_transfer_ref>(self);
    }
}
