spec velor_framework::event {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Each event handle possesses a distinct and unique GUID.
    /// Criticality: Critical
    /// Implementation: The new_event_handle function creates an EventHandle object with a unique GUID, ensuring
    /// distinct identification.
    /// Enforcement: Audited: GUIDs are created in guid::create. Each time the function is called, it increments creation_num_ref. Multiple calls to the function will result in distinct GUID values.
    ///
    /// No.: 2
    /// Requirement: Unable to publish two events with the same GUID & sequence number.
    /// Criticality: Critical
    /// Implementation: Two events may either have the same GUID with a different counter or the same counter with a
    /// different GUID.
    /// Enforcement: This is implied by [high-level-req](high-level requirement 1).
    ///
    /// No.: 3
    /// Requirement: Event native functions respect normal Move rules around object creation and destruction.
    /// Criticality: Critical
    /// Implementation: Must follow the same rules and principles that apply to object creation and destruction in Move
    /// when using event native functions.
    /// Enforcement: The native functions of this module have been manually audited.
    ///
    /// No.: 4
    /// Requirement: Counter increases monotonically between event emissions
    /// Criticality: Medium
    /// Implementation: With each event emission, the emit_event function increments the counter of the EventHandle by
    /// one.
    /// Enforcement: Formally verified in the post condition of [high-level-req-4](emit_event).
    ///
    /// No.: 5
    /// Requirement: For a given EventHandle, it should always be possible to: (1) return the GUID associated with this
    /// EventHandle, (2) return the current counter associated with this EventHandle, and (3) destroy the handle.
    /// Criticality: Low
    /// Implementation: The following functions should not abort if EventHandle exists: guid(), counter(),
    /// destroy_handle().
    /// Enforcement: Formally verified via [high-level-req-5.1](guid), [high-level-req-5.2](counter) and [high-level-req-5.3](destroy_handle).
    /// </high-level-req>
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec emit_event {
        pragma opaque;
        aborts_if [abstract] false;
        /// [high-level-req-4]
        ensures [concrete] handle_ref.counter == old(handle_ref.counter) + 1;
    }

    spec emit {
        pragma opaque;
    }

    /// Native function use opaque.
    spec write_module_event_to_store<T: drop + store>(msg: T) {
        pragma opaque;
    }

    /// Native function use opaque.
    spec write_to_event_store<T: drop + store>(guid: vector<u8>, count: u64, msg: T) {
        pragma opaque;
    }

    spec guid {
        /// [high-level-req-5.1]
        aborts_if false;
    }

    spec counter {
        /// [high-level-req-5.2]
        aborts_if false;
    }

    spec destroy_handle {
        /// [high-level-req-5.3]
        aborts_if false;
    }
}
