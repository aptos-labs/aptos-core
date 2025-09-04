/// This module demonstrates a basic messageboard using capability to control the access.
/// Admin can
///     (1) create their messageboard
///     (2) offer participants capability to update the pinned message
///     (3) remove the capability from a participant
/// participant can
///     (1) register for the board
///     (2) redeem the offered capability to update pinned message
///     (3) send a new message
///
/// The module also emits two types of events for subscribes
///     (1) message cap update event, this event contains the board address and participant offered capability
///     (2) message change event, this event contains the board, message and message author
module message_board::cap_based_mb {
    use message_board::offer;
    use std::signer;
    use std::vector;
    use velor_framework::event;

    // Error map
    const EACCOUNT_NO_NOTICE_CAP: u64 = 1;
    const EONLY_ADMIN_CAN_REMOVE_NOTICE_CAP: u64 = 2;

    struct CapBasedMB has key {
        pinned_post: vector<u8>
    }

    /// provide the capability to alert the board message
    struct MessageChangeCapability has key, store {
        board: address
    }

    #[event]
    /// emit an event from board acct showing the new participant with posting capability
    struct MessageCapUpdate has store, drop {
        board: address,
        participant: address,
    }

    #[event]
    /// emit an event from participant account showing the board and the new message
    struct MessageChange has store, drop {
        board: address,
        message: vector<u8>,
        participant: address
    }

    /// create the message board and move the resource to signer
    public entry fun message_board_init(account: &signer) {
        let board = CapBasedMB {
            pinned_post: vector::empty<u8>()
        };
        let board_addr = signer::address_of(account);
        move_to(account, board);
        let notice_cap = MessageChangeCapability { board: board_addr };
        move_to(account, notice_cap);
    }

    /// directly view message
    public fun view_message(board_addr: address): vector<u8> acquires CapBasedMB {
        let post = borrow_global<CapBasedMB>(board_addr).pinned_post;
        copy post
    }

    /// board owner controls adding new participants
    public entry fun add_participant(account: &signer, participant: address) {
        let board = signer::address_of(account);
        offer::create(account, MessageChangeCapability { board }, participant);

        event::emit(MessageCapUpdate { board, participant });
    }

    /// claim offered capability
    public entry fun claim_notice_cap(account: &signer, board: address) {
        let notice_cap = offer::redeem<MessageChangeCapability>(
            account, board);
        move_to(account, notice_cap);
    }

    /// remove a participant capability to publish notice
    public entry fun remove_participant(account: signer, participant: address) acquires MessageChangeCapability {
        let cap = borrow_global_mut<MessageChangeCapability>(participant);
        assert!(signer::address_of(&account) == cap.board, EONLY_ADMIN_CAN_REMOVE_NOTICE_CAP);
        let cap = move_from<MessageChangeCapability>(participant);
        let MessageChangeCapability { board: _ } = cap;
    }

    /// only the participant with right capability can publish the message
    public entry fun send_pinned_message(
        account: &signer, board_addr: address, message: vector<u8>
    ) acquires MessageChangeCapability, CapBasedMB {
        let cap = borrow_global<MessageChangeCapability>(signer::address_of(account));
        assert!(cap.board == board_addr, EACCOUNT_NO_NOTICE_CAP);
        let board = borrow_global_mut<CapBasedMB>(board_addr);
        board.pinned_post = message;
        event::emit(
            MessageChange {
                board: board_addr,
                message,
                participant: signer::address_of(account)
            }
        );
    }

    /// an account can send events containing message
    public entry fun send_message_to(account: signer, board_addr: address, message: vector<u8>) {
        event::emit(
            MessageChange {
                board: board_addr,
                message,
                participant: signer::address_of(&account)
            }
        );
    }
}

#[test_only]
module message_board::MessageBoardCapTests {
    use std::unit_test;
    use std::vector;
    use std::signer;
    use message_board::cap_based_mb;


    const HELLO_WORLD: vector<u8> = vector<u8>[150, 145, 154, 154, 157, 040, 167, 157, 162, 154, 144];
    const BOB_IS_HERE: vector<u8> = vector<u8>[142, 157, 142, 040, 151, 163, 040, 150, 145, 162, 145];

    #[test]
    public entry fun test_init_messageboard_v_cap() {
        let (alice, _) = create_two_signers();
        cap_based_mb::message_board_init(&alice);
        let board_addr = signer::address_of(&alice);
        cap_based_mb::send_pinned_message(&alice, board_addr, HELLO_WORLD);
    }

    #[test]
    public entry fun test_send_pinned_message_v_cap() {
        let (alice, bob) = create_two_signers();
        cap_based_mb::message_board_init(&alice);
        cap_based_mb::add_participant(&alice, signer::address_of(&bob));
        cap_based_mb::claim_notice_cap(&bob, signer::address_of(&alice));
        cap_based_mb::send_pinned_message(&bob, signer::address_of(&alice), BOB_IS_HERE);
        let message = cap_based_mb::view_message(signer::address_of(&alice));
        assert!(message == BOB_IS_HERE, 1)
    }

    #[test]
    public entry fun test_send_message_v_cap() {
        let (alice, bob) = create_two_signers();
        cap_based_mb::message_board_init(&alice);
        cap_based_mb::send_message_to(bob, signer::address_of(&alice), BOB_IS_HERE);
    }

    #[test]
    #[expected_failure]
    public entry fun test_add_new_participant_v_cap() {
        let (alice, bob) = create_two_signers();
        cap_based_mb::message_board_init(&alice);
        cap_based_mb::add_participant(&alice, signer::address_of(&bob));
        cap_based_mb::send_pinned_message(&bob, signer::address_of(&alice), BOB_IS_HERE);
    }

    #[test_only]
    fun create_two_signers(): (signer, signer) {
        let signers = &mut unit_test::create_signers_for_testing(2);
        let (alice, bob) = (vector::pop_back(signers), vector::pop_back(signers));
        velor_framework::account::create_account_for_test(signer::address_of(&alice));
        velor_framework::account::create_account_for_test(signer::address_of(&bob));
        (alice, bob)
    }
}
