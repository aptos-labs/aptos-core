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
module MessageBoard::ACLBasedMB{
    use Std::ACL::Self;
    use Std::Event::{Self, EventHandle};
    use Std::Signer;
    use Std::Vector;

    // Error map
    const EACCOUNT_NOT_IN_ACL: u64 = 1;
    const ECANNOT_REMOVE_ADMIN_FROM_ACL: u64 = 2;

    struct ACLBasedMB has key {
        participants: ACL::ACL,
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

    /// create the message board and move the resource to signer
    public fun message_board_init_internal(account: &signer) {
        let board = ACLBasedMB{
            participants: ACL::empty(),
            pinned_post: Vector::empty<u8>()
        };
        ACL::add(&mut board.participants, Signer::address_of(account));
        move_to(account, board);
        move_to(account, MessageChangeEventHandle{
            change_events: Event::new_event_handle<MessageChangeEvent>(account)
        })
    }

    /// init message board
    public(script) fun message_board_init(account: signer) {
        message_board_init_internal(&account);
    }

    public fun view_message(board_addr: address): vector<u8> acquires ACLBasedMB {
        let post = borrow_global<ACLBasedMB>(board_addr).pinned_post;
        copy post
    }

    /// board owner control adding new participants
    public fun add_participant_internal(account: &signer, participant: address) acquires ACLBasedMB {
        let board = borrow_global_mut<ACLBasedMB>(Signer::address_of(account));
        ACL::add(&mut board.participants, participant);
    }

    public(script) fun add_participant(account: signer, participant: address) acquires ACLBasedMB {
        add_participant_internal(&account, participant);
    }

    /// remove a participant from the ACL
    public(script) fun remove_participant(account: signer, participant: address) acquires ACLBasedMB {
        let board = borrow_global_mut<ACLBasedMB>(Signer::address_of(&account));
        assert!(Signer::address_of(&account) != participant, ECANNOT_REMOVE_ADMIN_FROM_ACL);
        ACL::remove(&mut board.participants, participant);
    }

    /// an account publish the message to update the notice
    public fun send_pinned_message_internal(
        account: &signer, board_addr: address, message: vector<u8>
    ) acquires ACLBasedMB, MessageChangeEventHandle {
        let board = borrow_global<ACLBasedMB>(board_addr);
        assert!(ACL::contains(&board.participants, Signer::address_of(account)), EACCOUNT_NOT_IN_ACL);

        let board = borrow_global_mut<ACLBasedMB>(board_addr);
        board.pinned_post = message;

        let send_acct = Signer::address_of(account);
        let event_handle = borrow_global_mut<MessageChangeEventHandle>(board_addr);
        Event::emit_event<MessageChangeEvent>(
            &mut event_handle.change_events,
            MessageChangeEvent{
                message,
                participant: send_acct
            }
        );
    }

    public(script) fun send_pinned_message(
        account: signer, board_addr: address, message: vector<u8>
    ) acquires ACLBasedMB, MessageChangeEventHandle {
        send_pinned_message_internal(&account, board_addr, message);
    }

    /// an account can send events containing message
    public(script) fun send_message_to(
        board_addr: address, message: vector<u8>
    ) acquires MessageChangeEventHandle {
        let event_handle = borrow_global_mut<MessageChangeEventHandle>(board_addr);
        Event::emit_event<MessageChangeEvent>(
            &mut event_handle.change_events,
            MessageChangeEvent{
                message,
                participant: board_addr
            }
        );
    }
}

#[test_only]
module MessageBoard::MessageBoardTests {
    use Std::UnitTest;
    use Std::Vector;
    use Std::Signer;

    use MessageBoard::ACLBasedMB;

    const  HELLO_WORLD: vector<u8> = vector<u8>[150, 145, 154, 154, 157, 040, 167, 157, 162, 154, 144];
    const  BOB_IS_HERE: vector<u8> = vector<u8>[142, 157, 142, 040, 151, 163, 040, 150, 145, 162, 145];

    #[test]
    fun test_init_messageboard() {
        let (alice, _) = create_two_signers();
        ACLBasedMB::message_board_init_internal(&alice);
        ACLBasedMB::send_pinned_message_internal(&alice, Signer::address_of(&alice), HELLO_WORLD);
    }

    #[test]
    fun test_send_pinned_message() {
        let (alice, bob) = create_two_signers();
        ACLBasedMB::message_board_init_internal(&alice);
        ACLBasedMB::add_participant_internal(&alice, Signer::address_of(&bob));
        ACLBasedMB::send_pinned_message_internal(&bob, Signer::address_of(&alice), BOB_IS_HERE);
        let message = ACLBasedMB::view_message(Signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
        let message = ACLBasedMB::view_message(Signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
    }

    #[test]
    public(script) fun test_send_message_v_cap() {
        let (alice, _) = create_two_signers();
        ACLBasedMB::message_board_init_internal(&alice);
        ACLBasedMB::send_message_to(Signer::address_of(&alice), BOB_IS_HERE);
    }

    #[test]
    fun read_message_multiple_times() {
        let (alice, bob) = create_two_signers();
        ACLBasedMB::message_board_init_internal(&alice);
        ACLBasedMB::add_participant_internal(&alice, Signer::address_of(&bob));
        ACLBasedMB::send_pinned_message_internal(&bob, Signer::address_of(&alice), BOB_IS_HERE);
        let message = ACLBasedMB::view_message(Signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
        let message = ACLBasedMB::view_message(Signer::address_of(&alice));
        assert!( message == BOB_IS_HERE, 1);
    }

    #[test]
    #[expected_failure(abort_code = 1)]
    fun test_add_new_participant() {
        let (alice, bob) = create_two_signers();
        ACLBasedMB::message_board_init_internal(&alice);
        ACLBasedMB::send_pinned_message_internal(&bob, Signer::address_of(&alice), BOB_IS_HERE);
    }

    #[test_only]
    fun create_two_signers(): (signer, signer) {
        let signers = &mut UnitTest::create_signers_for_testing(2);
        (Vector::pop_back(signers), Vector::pop_back(signers))
    }
}
