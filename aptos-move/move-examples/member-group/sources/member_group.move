// Update: See the README in the use-member-group branch of aptos-tontine. There I talk
// about how it doesn't really make sense to emit events from a "library" (particularly
// for what is mostly a data structure), as opposed to a "top level" module.

module dport_std::member_group {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_framework::account::new_event_handle;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_std::object::{Self, Object};

    /// The member tried to do something but they weren't in the group, or they tried
    /// to do something against someone else not in the group.
    const E_MEMBER_NOT_IN_GROUP: u64 = 0;

    /// The caller tried to perform an action but did not have the required
    /// privilege level.
    const E_CALLER_LACKING_PRIVILEGES: u64 = 1;

    /// The caller tried to invite someone who has already been invited / is in the
    /// group.
    const E_ALREADY_INVITED_OR_MEMBER: u64 = 2;

    /// The caller tried to join a group they weren't invited to.
    const E_NOT_INVITED: u64 = 3;

    /// The member tried to join but they're already in the group.
    const E_MEMBER_TRIED_TO_JOIN_AGAIN: u64 = 4;

    /// todo don't do all the fancy table init module stuff bc we'll make this not
    /// meant to be read off chain using the table APIs. we could even use a smart
    /// table to make this extra obvious. to read this, we'll use this custom processor
    /// that i'm building that will use the events emitted by this module to build up
    /// the picture of the state of the membership. we still need the key thing i think,
    /// though a complex key type might make the downstream indexer complicated, since
    /// we want to be able to query on key? well postgres supports JSON as a field type,
    /// so who knows, maybe it is fine.

    struct MemberInvitedEvent has store, drop {
        member: address,
    }

    struct MemberJoinedEvent has store, drop {
        member: address,
    }

    struct MemberLeftEvent has store, drop {
        member: address,
    }

    struct MemberRemovedEvent has store, drop {
        member: address,
    }

    struct PrivilegesSetEvent has store, drop {
        member: address,
        new_privilege_level: u8,
    }

    // TODO: Determine if an identifier of some sort is required.

    /// Summary
    /// ---
    ///
    /// This struct provides functionality for tracking membership of a "group".
    ///
    /// Reading data off chain
    /// ---
    /// The information captured in this struct is not sufficient to do off chain
    /// lookups such as:
    ///
    /// - Get all rooms an address is invited to / has joined.
    ///
    /// This is by design. Rather than track this on-chain and using the resource
    /// and / or table APIs to read it, there is a custom processor that uses the
    /// events emitted by the module to determine the global state of group membership.
    ///
    /// Privileges
    /// ---
    ///
    /// The MemberGroup has the notion of privilege levels. The highest privilege
    /// level is 0, followed by 1, then 2, and so on. In the MemberGroup you can
    /// define what level a member must be to have certain privileges. For example, you
    /// could say you must be level 2 to invite other members and level 1 to remove
    /// other members.
    ///
    /// Note that even if a member has the remove member privilege, they can only remove
    /// members of a lower level than themselves. Similarly, members can only promote
    /// other members up to their own privilege level.
    ///
    /// If you're not interested in using these features, just set the required level
    /// for all privileges to 0 and don't touch any of the promotion features. This way
    /// only the creator of the MemberGroup will have the ability to add / remove
    /// members.
    struct MemberGroup has key, store {
        /// This is a map of address, which identifies a member, to MemberInfo.
        ///
        /// You'll see that we use SmartTable here. SmartTable is not easy to access
        /// via the standard node APIs, or even the regular event / table indexer
        /// tables. Instead, you must use the custom processor described above.
        members: SmartTable<address, MemberInfo>,

        /// This is a vec of the keys of that map, which we keep alongside so callers
        /// can iterate through the members if they want.
        members_vec: vector<address>,

        /// The privilege level required to invite other members. See more in the
        /// privileges section above.
        can_invite_privilege_level: u8,

        /// The privilege level required to remove other members. See more in the
        /// privileges section above.
        can_remove_privilege_level: u8,

        // Events emitted for various lifecycle events.
        member_invited_events: EventHandle<MemberInvitedEvent>,
        member_joined_events: EventHandle<MemberJoinedEvent>,
        member_left_events: EventHandle<MemberLeftEvent>,
        member_removed_events: EventHandle<MemberRemovedEvent>,
        privileges_set_events: EventHandle<PrivilegesSetEvent>,
    }

    struct MemberInfo has store, drop {
        /// What time the invite expires. If this is zero, the invite never expires.
        /// If the member joins, this gets set to zero.
        invite_expires_secs: u64,

        /// If the member has accepted the invite and joined, this will be true.
        joined: bool,

        /// The privilege level of the member. See the comment for MemberGroup to
        /// learn more about privileges.
        privilege_level: u8,
    }

    public fun new_object(creator: &signer): Object<MemberGroup> {
        let constructor_ref = &object::create_object_from_account(creator);
        let object_signer = &object::generate_signer(constructor_ref);
        let group = new(creator);
        move_to(object_signer, group);
        object::object_from_constructor_ref(constructor_ref)
    }

    fun new(creator: &signer): MemberGroup {
        let creator_addr = signer::address_of(creator);

        let group = MemberGroup {
            members: smart_table::new(),
            members_vec: vector::empty(),
            can_invite_privilege_level: 0,
            can_remove_privilege_level: 0,
            member_invited_events: new_event_handle<MemberInvitedEvent>(creator),
            member_joined_events: new_event_handle<MemberJoinedEvent>(creator),
            member_left_events: new_event_handle<MemberLeftEvent>(creator),
            member_removed_events: new_event_handle<MemberRemovedEvent>(creator),
            privileges_set_events: new_event_handle<PrivilegesSetEvent>(creator),
        };

        add_member(
            &mut group,
            creator_addr,
            MemberInfo {
                invite_expires_secs: 0,
                joined: true,
                privilege_level: 0,
            }
        );

        event::emit_event(
            &mut group.member_joined_events,
            MemberJoinedEvent { member: creator_addr }
        );

        event::emit_event(
            &mut group.privileges_set_events,
            PrivilegesSetEvent { member: creator_addr, new_privilege_level: 0 }
        );

        group
    }

    fun add_member(group: &mut MemberGroup, member: address, member_info: MemberInfo) {
        smart_table::add(
            &mut group.members,
            member,
            member_info,
        );
        vector::push_back(&mut group.members_vec, member);
    }

    fun remove_member(group: &mut MemberGroup, member: address) {
        smart_table::remove(
            &mut group.members,
            member,
        );
        let (_, i) = vector::index_of(&group.members_vec, &member);
        vector::swap_remove(&mut group.members_vec, i);
    }

    /// Set the privilege level at which a member can invite other members.
    public fun set_can_invite_privilege_level(caller: &signer, group: &mut MemberGroup, new_level: u8) {
        let caller_addr = signer::address_of(caller);
        assert_is_root(group, caller_addr);
        group.can_invite_privilege_level = new_level;
    }

    /// Set the privilege level at which a member can remove other members.
    public fun set_can_remove_privilege_level(caller: &signer, group: &mut MemberGroup, new_level: u8) {
        let caller_addr = signer::address_of(caller);
        assert_is_root(group, caller_addr);
        group.can_remove_privilege_level = new_level;
    }

    public fun members(group: &MemberGroup): &vector<address> {
        &group.members_vec
    }

    public fun member_in_group(group: &MemberGroup, member: address): bool {
        smart_table::contains(&group.members, member)
    }

    public fun assert_member_in_group(group: &MemberGroup, member: address) {
        assert!(
            member_in_group(group, member),
            error::invalid_state(E_MEMBER_NOT_IN_GROUP),
        );
    }

    fun assert_has_privilege_level(group: &MemberGroup, member: address, level: u8) {
        assert_member_in_group(group, member);

        // Assert the caller has privileges at or stronger than the given level.
        let info = smart_table::borrow(&group.members, member);
        assert!(
            info.privilege_level <= level,
            error::invalid_state(E_CALLER_LACKING_PRIVILEGES),
        );
    }

    fun assert_is_root(group: &MemberGroup, member: address) {
        assert_has_privilege_level(group, member, 0);
    }

    /// Set the privilege level of another member. A caller can only set the level of
    /// a member with a strictly less powerful level than them.
    public fun set_member_privilege_level(caller: &signer, group: &mut MemberGroup, member: address, new_level: u8) {
        let caller_addr = signer::address_of(caller);

        // Assert the caller is a member.
        assert_member_in_group(group, caller_addr);

        // Assert the member in question is a member.
        assert_member_in_group(group, member);

        // Confirm that the caller is of a higher privilege level than the member whose
        // level they're trying to change.
        let caller_level = {
            smart_table::borrow(&group.members, caller_addr).privilege_level
        };
        let member_info = smart_table::borrow_mut(&mut group.members, member);
        assert!(
            caller_level < member_info.privilege_level,
            error::invalid_state(E_CALLER_LACKING_PRIVILEGES),
        );

        // Update the level of the member.
        member_info.privilege_level = new_level;
    }

    /// Invite someone to the group. This invite can be set to expire by setting
    /// expiration_secs. If this behaviour is not desired, set expiration_secs to zero.
    public fun invite(
        group: &mut MemberGroup,
        caller: &signer,
        invitee: address,
        level: u8,
        expiration_secs: u64,
    ) {
        let caller_addr = signer::address_of(caller);

        // Assert the caller has the required privilege level.
        let required_level = group.can_invite_privilege_level;
        assert_has_privilege_level(group, caller_addr, required_level);

        // Assert the invitee is not already invited / a member.
        assert!(!smart_table::contains(&group.members, invitee), error::invalid_state(E_ALREADY_INVITED_OR_MEMBER));

        // Assert the level they're being added at is not more powerful than the caller.
        let caller_level = smart_table::borrow(&group.members, caller_addr).privilege_level;
        assert!(
            caller_level <= level,
            error::invalid_state(E_CALLER_LACKING_PRIVILEGES),
        );

        // Invite the member.
        add_member(
            group,
            invitee,
            MemberInfo {
                invite_expires_secs: expiration_secs,
                joined: false,
                privilege_level: level,
            }
        );

        event::emit_event(
            &mut group.member_invited_events,
            MemberInvitedEvent { member: invitee }
        );

        event::emit_event(
            &mut group.privileges_set_events,
            PrivilegesSetEvent { member: invitee, new_privilege_level: level }
        );

    }

    /// Accept an invite to join a group.
    public fun join(
        group: &mut MemberGroup,
        caller: &signer,
    ) {
        let caller_addr = signer::address_of(caller);

        // Assert the caller has been invited to the group.
        assert!(smart_table::contains(&group.members, caller_addr), error::invalid_state(E_NOT_INVITED));

        // Assert the caller has not already joined the group.
        let member_info = smart_table::borrow_mut(&mut group.members, caller_addr);
        assert!(!member_info.joined, error::invalid_state(E_MEMBER_TRIED_TO_JOIN_AGAIN));

        // Join the group.
        member_info.joined = true;

        event::emit_event(
            &mut group.member_joined_events,
            MemberJoinedEvent { member: caller_addr }
        );
    }

    /// Leave a group. Note: It is possible that doing this will put the group in a
    /// state where no one has certain privileges anymore. We do not check for this
    /// right now.
    public fun leave(
        group: &mut MemberGroup,
        caller: &signer,
    ) {
        let caller_addr = signer::address_of(caller);

        // Assert the caller is in the group.
        assert_member_in_group(group, caller_addr);

        // Leave the group.
        remove_member(
            group,
            caller_addr,
        );

        event::emit_event(
            &mut group.member_left_events,
            MemberLeftEvent { member: caller_addr }
        );
    }

    /// Remove someone from the group.
    public fun remove(
        group: &mut MemberGroup,
        caller: &signer,
        member: address,
    ) {
        let caller_addr = signer::address_of(caller);

        // Assert the caller has the required privilege level.
        let required_level = group.can_remove_privilege_level;
        assert_has_privilege_level(group, caller_addr, required_level);

        // Confirm that the caller is of a higher privilege level than the member they
        // are trying to remove.
        let caller_level = {
            smart_table::borrow(&group.members, caller_addr).privilege_level
        };
        let member_info = smart_table::borrow_mut(&mut group.members, member);
        assert!(
            caller_level < member_info.privilege_level,
            error::invalid_state(E_CALLER_LACKING_PRIVILEGES),
        );

        // Remove them from the group.
        remove_member(
            group,
            member,
        );

        event::emit_event(
            &mut group.member_removed_events,
            MemberRemovedEvent { member }
        );
    }
}
