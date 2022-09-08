/// This module demonstrates a basic messageboard using ACL to control the access.
/// Admins can
///     (1) create their messageboard
///     (2) add a partipant to its access control list (ACL)
///     (3) remove a participant from its ACL
/// participant can
///     (1) register for the board
///     (2) send a new message
///
/// The module also emits events for subscribers
///     (1) message change event, this event contains the board, message and message author
module message_board::acl_based_mb {
    use std::acl::Self;
    use std::signer;
    use std::vector;
    use aptos_framework::account;
    use aptos_std::event::{Self, EventHandle};

    // Error map
    const EACCOUNT_NOT_IN_ACL: u64 = 1;
    const ECANNOT_REMOVE_ADMIN_FROM_ACL: u64 = 2;

    struct ACLBasedMB has key {
        participants: acl::ACL,
        pinned_post: vector<u8>
    }

    struct MessageChangeEventHandle has key {
        change_events: EventHandle<MessageChangeEvent>
    }

    /// emit an event from participant account showing the board and the new message
    struct MessageChangeEvent has store, drop {
        message: vector<u8>,
        participant: address
    }

    /// init message board
    public entry fun message_board_init(account: &signer) {
        let board = ACLBasedMB{
            participants: acl::empty(),
            pinned_post: vector::empty<u8>()
        };
        acl::add(&mut board.participants, signer::address_of(account));
        move_to(account, board);
        move_to(account, MessageChangeEventHandle{
            change_events: account::new_event_handle<MessageChangeEvent>(account)
        })
    }

    public fun view_message(board_addr: address): vector<u8> acquires ACLBasedMB {
        let post = borrow_global<ACLBasedMB>(board_addr).pinned_post;
        copy post
    }

    /// board owner control adding new participants
    public entry fun add_participant(account: &signer, participant: address) acquires ACLBasedMB {
        let board = borrow_global_mut<ACLBasedMB>(signer::address_of(account));
        acl::add(&mut board.participants, participant);
    }

    /// remove a participant from the ACL
    public entry fun remove_participant(account: signer, participant: address) acquires ACLBasedMB {
        let board = borrow_global_mut<ACLBasedMB>(signer::address_of(&account));
        assert!(signer::address_of(&account) != participant, ECANNOT_REMOVE_ADMIN_FROM_ACL);
        acl::remove(&mut board.participants, participant);
    }

    /// an account publish the message to update the notice
    public entry fun send_pinned_message(
        account: &signer, board_addr: address, message: vector<u8>
    ) acquires ACLBasedMB, MessageChangeEventHandle {
        let board = borrow_global<ACLBasedMB>(board_addr);
        assert!(acl::contains(&board.participants, signer::address_of(account)), EACCOUNT_NOT_IN_ACL);

        let board = borrow_global_mut<ACLBasedMB>(board_addr);
        board.pinned_post = message;

        let send_acct = signer::address_of(account);
        let event_handle = borrow_global_mut<MessageChangeEventHandle>(board_addr);
        event::emit_event<MessageChangeEvent>(
            &mut event_handle.change_events,
            MessageChangeEvent{
                message,
                participant: send_acct
            }
        );
    }

    /// an account can send events containing message
    public entry fun send_message_to(
        account: signer, board_addr: address, message: vector<u8>
    ) acquires MessageChangeEventHandle {
        let event_handle = borrow_global_mut<MessageChangeEventHandle>(board_addr);
        event::emit_event<MessageChangeEvent>(
            &mut event_handle.change_events,
            MessageChangeEvent{
                message,
                participant: signer::address_of(&account)
            }
        );
    }
}

#[test_only]
module message_board::MessageBoardTests {
    use std::unit_test;
    use std::vector;
    use std::signer;

    use message_board::acl_based_mb;

    const  HELLO_WORLD: vector<u8> = vector<u8>[150, 145, 154, 154, 157, 040, 167, 157, 162, 154, 144];
    const  BOB_IS_HERE: vector<u8> = vector<u8>[142, 157, 142, 040, 151, 163, 040, 150, 145, 162, 145];

    #[test]
    public entry fun test_init_messageboard() {
        let (alice, _) = create_two_signers();
        acl_based_mb::message_board_init(&alice);
        acl_based_mb::send_pinned_message(&alice, signer::address_of(&alice), HELLO_WORLD);
    }

    #[test]
    public entry fun test_send_pinned_message() {
        let (alice, bob) = create_two_signers();
        acl_based_mb::message_board_init(&alice);
        acl_based_mb::add_participant(&alice, signer::address_of(&bob));
        acl_based_mb::send_pinned_message(&bob, signer::address_of(&alice), BOB_IS_HERE);
        let message = acl_based_mb::view_message(signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
        let message = acl_based_mb::view_message(signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
    }

    #[test]
    public entry fun test_send_message_v_cap() {
        let (alice, bob) = create_two_signers();
        acl_based_mb::message_board_init(&alice);
        acl_based_mb::send_message_to(bob, signer::address_of(&alice), BOB_IS_HERE);
    }

    #[test]
    public entry fun read_message_multiple_times() {
        let (alice, bob) = create_two_signers();
        acl_based_mb::message_board_init(&alice);
        acl_based_mb::add_participant(&alice, signer::address_of(&bob));
        acl_based_mb::send_pinned_message(&bob, signer::address_of(&alice), BOB_IS_HERE);
        let message = acl_based_mb::view_message(signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
        let message = acl_based_mb::view_message(signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    public entry fun test_add_new_participant() {
        let (alice, bob) = create_two_signers();
        acl_based_mb::message_board_init(&alice);
        acl_based_mb::send_pinned_message(&bob, signer::address_of(&alice), BOB_IS_HERE);
    }

    #[test_only]
    fun create_two_signers(): (signer, signer) {
        let signers = &mut unit_test::create_signers_for_testing(2);
        let (alice, bob) = (vector::pop_back(signers), vector::pop_back(signers));
        aptos_framework::account::create_account_for_test(signer::address_of(&alice));
        aptos_framework::account::create_account_for_test(signer::address_of(&bob));
        (alice, bob)
    }
}
