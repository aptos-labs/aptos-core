/// # Dead Man's Switch Tracker Module
///
/// This module implements a dead man's switch mechanism for trading orders, ensuring that
/// orders are automatically invalidated if a trader's session expires without periodic
/// keep-alive updates. This security feature prevents stale orders from being executed
/// if a trader loses connection or becomes unresponsive.
///
/// ## Overview
///
/// The dead man's switch works by requiring traders to periodically send keep-alive signals.
/// If a trader fails to update their keep-alive state within a specified timeout period,
/// all their orders placed during that session become invalid and can be cancelled.
///
/// ## Key Concepts
///
/// ### Session Management
/// - **Session**: A time-bound period during which a trader's orders are considered valid
/// - **Session Start Time**: The beginning of the current session (when it was started or restarted)
/// - **Expiration Time**: When the current session will expire if not renewed
/// - **Timeout**: The duration for which a keep-alive update remains valid
///
/// ### Order Validation
/// An order is considered valid if:
/// 1. The trader has no keep-alive state set (no dead man's switch enabled), OR
/// 2. The order was created after the current session started, AND
/// 3. The current time is before the session expiration time
///
/// ### Session Lifecycle
///
/// **First Keep-Alive Update:**
/// - Creates a new session with `session_start_time = 0` (all existing orders remain valid)
/// - Sets `expiration_time = current_time + timeout`
///
/// **Subsequent Updates (Before Expiration):**
/// - Extends the current session: `expiration_time = current_time + timeout`
/// - Keeps the same `session_start_time` (existing orders remain valid)
///
/// **Update After Expiration:**
/// - Starts a new session: `session_start_time = current_time`
/// - Sets new `expiration_time = current_time + timeout`
/// - All orders placed before this time are invalidated
///
/// ## Events
///
/// - `KeepAliveUpdateEvent`: Emitted when a trader updates their keep-alive state
/// - `KeepAliveDisabledEvent`: Emitted when a trader disables their keep-alive
/// - `MinKeepAliveTimeUpdatedEvent`: Emitted when the minimum keep-alive time is updated
///
module aptos_experimental::dead_mans_switch_tracker {
    friend aptos_experimental::order_placement;
    friend aptos_experimental::order_operations;
    friend aptos_experimental::market_types;
    friend aptos_experimental::dead_mans_switch_operations;
    use std::option::Option;
    use aptos_std::big_ordered_map::BigOrderedMap;
    use aptos_framework::event;
    use aptos_experimental::order_book_utils;

    /// Error code when the provided keep-alive timeout is shorter than the minimum allowed
    const E_KEEP_ALIVE_TIMEOUT_TOO_SHORT: u64 = 0;

    // Event emitted when a trader updates their keep-alive state
    // Fields:
    // - parent: The parent address (DEX identifier)
    // - market: The market address
    // - account: The trader's address
    // - session_start_time_secs: When the current session started (0 if first update)
    // - expiration_time_secs: When the session will expire
    #[event]
    enum KeepAliveUpdateEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            account: address,
            session_start_time_secs: u64,
            expiration_time_secs: u64
        }
    }

    // Event emitted when a trader disables their keep-alive (opts out of dead man's switch)
    // Fields:
    // - parent: The parent address (DEX identifier)
    // - market: The market address
    // - account: The trader's address
    // - was_registered: Whether the user was actually registered in the tracker
    #[event]
    enum KeepAliveDisabledEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            account: address,
            was_registered: bool
        }
    }

    // Event emitted when the minimum keep-alive time is updated
    // Fields:
    // - parent: The parent address (DEX identifier)
    // - market: The market address
    // - old_min_keep_alive_time_secs: The previous minimum keep-alive time in seconds
    // - new_min_keep_alive_time_secs: The new minimum keep-alive time in seconds
    #[event]
    enum MinKeepAliveTimeUpdatedEvent has drop, copy, store {
        V1 {
            parent: address,
            market: address,
            old_min_keep_alive_time_secs: u64,
            new_min_keep_alive_time_secs: u64
        }
    }

    // Stores the keep-alive state for a single trader
    // Fields:
    // - session_start_time_secs: Timestamp when the current session started.
    //   Orders created before this time are considered invalid.
    //   Set to 0 on first keep-alive to allow all existing orders.
    // - expiration_time_secs: Timestamp when the current session expires.
    //   If current time is after this (strictly greater), the session is expired.
    struct KeepAliveState has store {
        session_start_time_secs: u64,
        expiration_time_secs: u64
    }

    // Main tracker for managing dead man's switch state across all traders
    // Fields:
    // - min_keep_alive_time_secs: Minimum allowed timeout duration for keep-alive updates.
    //   Prevents traders from setting excessively short timeouts.
    // - state: Map of trader addresses to their keep-alive state
    struct DeadMansSwitchTracker has store {
        min_keep_alive_time_secs: u64,
        state: BigOrderedMap<address, KeepAliveState>
    }

    /// Creates a new dead man's switch tracker
    ///
    /// # Parameters
    /// - `min_keep_alive_time_secs`: Minimum timeout duration that traders must use.
    ///   This prevents abuse by forcing traders to set reasonable timeout periods.
    ///
    /// # Returns
    /// A new `DeadMansSwitchTracker` instance with no active sessions
    ///
    /// # Example
    /// ```move
    /// let tracker = new_dead_mans_switch_tracker(60); // 60 second minimum
    /// ```
    public(friend) fun new_dead_mans_switch_tracker(
        min_keep_alive_time_secs: u64
    ): DeadMansSwitchTracker {
        DeadMansSwitchTracker {
            min_keep_alive_time_secs,
            state: order_book_utils::new_default_big_ordered_map()
        }
    }

    public(friend) fun set_min_keep_alive_time_secs(
        tracker: &mut DeadMansSwitchTracker,
        parent: address,
        market: address,
        min_keep_alive_time_secs: u64
    ) {
        let old_min_keep_alive_time_secs = tracker.min_keep_alive_time_secs;
        tracker.min_keep_alive_time_secs = min_keep_alive_time_secs;
        event::emit(
            MinKeepAliveTimeUpdatedEvent::V1 {
                parent,
                market,
                old_min_keep_alive_time_secs,
                new_min_keep_alive_time_secs: min_keep_alive_time_secs
            }
        );
    }

    /// Checks if an order is valid based on the dead man's switch state
    ///
    /// An order is valid if:
    /// 1. No keep-alive state exists for the account (dead man's switch not enabled), OR
    /// 2. The order was created after the current session started AND the session hasn't expired
    ///
    /// # Parameters
    /// - `tracker`: Reference to the dead man's switch tracker
    /// - `account`: The trader's address
    /// - `order_creation_time_secs`: When the order was created (in seconds since epoch)
    ///
    /// # Returns
    /// `true` if the order is valid, `false` if it should be cancelled
    ///
    /// # Validation Logic
    /// ```
    /// if no keep-alive state:
    ///     return true  // No dead man's switch, all orders valid
    /// if order_creation_time < session_start_time:
    ///     return false  // Order from expired session
    /// if current_time > expiration_time:
    ///     return false  // Session expired (exclusive of expiration time)
    /// return true  // Order valid
    /// ```
    ///
    /// # Example
    /// ```move
    /// let order_time = 1000;
    /// let is_valid = is_order_valid(&tracker, trader_addr, order_time);
    /// if (!is_valid) {
    ///     // Cancel the order
    /// }
    /// ```
    public fun is_order_valid(
        tracker: &DeadMansSwitchTracker,
        account: address,
        order_creation_time_secs: Option<u64>
    ): bool {
        let itr = tracker.state.internal_find(&account);
        if (itr.iter_is_end(&tracker.state)) {
            // No keep-alive set, so all orders are valid
            return true;
        };
        let current_time = aptos_std::timestamp::now_seconds();
        let order_creation_time_secs =
            if (order_creation_time_secs.is_some()) {
                order_creation_time_secs.destroy_some()
            } else {
                current_time
            };
        let state = itr.iter_borrow(&tracker.state);
        if (state.session_start_time_secs > order_creation_time_secs) {
            // Order was placed before the session started, so it is invalid
            return false;
        };
        state.expiration_time_secs >= current_time
    }

    fun disable_keep_alive(
        tracker: &mut DeadMansSwitchTracker,
        parent: address,
        market: address,
        account: address
    ) {
        let removed = tracker.state.remove_or_none(&account);
        let was_registered = removed.is_some();
        if (was_registered) {
            let KeepAliveState { session_start_time_secs: _, expiration_time_secs: _ } =
                removed.destroy_some();
        } else {
            removed.destroy_none();
        };
        event::emit(
            KeepAliveDisabledEvent::V1 { parent, market, account, was_registered }
        );
    }

    /// Updates the keep-alive state for a trader
    ///
    /// This is the core function traders call to maintain their session and prevent
    /// their orders from expiring. Behavior depends on the current state:
    ///
    /// 1. **First Update (No Prior State)**:
    ///    - Creates a new session with `session_start_time = 0`
    ///    - All existing orders remain valid
    ///    - Sets `expiration_time = current_time + timeout_seconds`
    ///
    /// 2. **Update Within Valid Session**:
    ///    - Extends the current session
    ///    - Updates `expiration_time = current_time + timeout_seconds`
    ///    - Keeps existing `session_start_time` (orders remain valid)
    ///
    /// 3. **Update After Session Expired**:
    ///    - Starts a new session with `session_start_time = current_time`
    ///    - All orders placed before now are invalidated
    ///    - Sets `expiration_time = current_time + timeout_seconds`
    ///
    /// # Parameters
    /// - `tracker`: Mutable reference to the dead man's switch tracker
    /// - `account`: The trader's address
    /// - `timeout_seconds`: Duration in seconds until the session expires.
    ///   Must be >= `min_keep_alive_time_secs` or 0 to disable.
    ///
    /// # Special Cases
    /// - If `timeout_seconds == 0`: Disables the keep-alive (calls `disable_keep_alive`)
    ///
    /// # Errors
    /// - `E_KEEP_ALIVE_TIMEOUT_TOO_SHORT`: If timeout is less than the minimum and not zero
    ///
    /// # Effects
    /// - Updates or creates the trader's keep-alive state
    /// - Emits a `KeepAliveUpdateEvent`
    ///
    /// # Example
    /// ```move
    /// // Update with 5 minute timeout
    /// update_keep_alive_state(&mut tracker, trader_addr, 300);
    ///
    /// // Disable dead man's switch
    /// update_keep_alive_state(&mut tracker, trader_addr, 0);
    /// ```
    public(friend) fun keep_alive(
        tracker: &mut DeadMansSwitchTracker,
        parent: address,
        market: address,
        account: address,
        timeout_seconds: u64
    ) {
        if (timeout_seconds == 0) {
            disable_keep_alive(tracker, parent, market, account);
            return;
        };
        assert!(
            timeout_seconds >= tracker.min_keep_alive_time_secs,
            E_KEEP_ALIVE_TIMEOUT_TOO_SHORT // ERROR_KEEP_ALIVE_TIMEOUT_TOO_SHORT
        );
        let current_time = aptos_std::timestamp::now_seconds();
        let expiration_time = current_time + timeout_seconds;
        let itr = tracker.state.internal_find(&account);
        if (!itr.iter_is_end(&tracker.state)) {
            let state = itr.iter_borrow_mut(&mut tracker.state);
            if (current_time > state.expiration_time_secs) {
                // Start a new session - this means any order placed before this time is invalidated
                state.session_start_time_secs = current_time;
            };
            // Update existing session
            state.expiration_time_secs = expiration_time;
            event::emit(
                KeepAliveUpdateEvent::V1 {
                    parent,
                    market,
                    account,
                    session_start_time_secs: state.session_start_time_secs,
                    expiration_time_secs: state.expiration_time_secs
                }
            );
        } else {
            let new_state = KeepAliveState {
                session_start_time_secs: 0, // this means that all existing orders are valid
                expiration_time_secs: expiration_time
            };
            tracker.state.add(account, new_state);
            event::emit(
                KeepAliveUpdateEvent::V1 {
                    parent,
                    market,
                    account,
                    session_start_time_secs: 0,
                    expiration_time_secs: expiration_time
                }
            );
        }
    }

    #[test_only]
    public fun destroy_tracker(tracker: DeadMansSwitchTracker) {
        let DeadMansSwitchTracker { min_keep_alive_time_secs: _, state } = tracker;
        state.destroy(
            |v| {
                let KeepAliveState { session_start_time_secs: _, expiration_time_secs: _ } =
                    v;
            }
        );
    }

    #[test_only]
    public fun new_dead_mans_switch_tracker_for_test(
        min_keep_alive_time_secs: u64
    ): DeadMansSwitchTracker {
        new_dead_mans_switch_tracker(min_keep_alive_time_secs)
    }

    #[test_only]
    public fun update_keep_alive_state_for_test(
        tracker: &mut DeadMansSwitchTracker,
        account: address,
        timeout_seconds: u64
    ) {
        keep_alive(tracker, @0x0, @0x0, account, timeout_seconds)
    }

    #[test_only]
    public fun disable_keep_alive_for_test(
        tracker: &mut DeadMansSwitchTracker, account: address
    ) {
        disable_keep_alive(tracker, @0x0, @0x0, account)
    }
}
