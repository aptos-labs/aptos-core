
<a id="0x7_dead_mans_switch_tracker"></a>

# Module `0x7::dead_mans_switch_tracker`


<a id="@Dead_Man's_Switch_Tracker_Module_0"></a>

## Dead Man's Switch Tracker Module


This module implements a dead man's switch mechanism for trading orders, ensuring that
orders are automatically invalidated if a trader's session expires without periodic
keep-alive updates. This security feature prevents stale orders from being executed
if a trader loses connection or becomes unresponsive.


<a id="@Overview_1"></a>

### Overview


The dead man's switch works by requiring traders to periodically send keep-alive signals.
If a trader fails to update their keep-alive state within a specified timeout period,
all their orders placed during that session become invalid and can be cancelled.


<a id="@Key_Concepts_2"></a>

### Key Concepts



<a id="@Session_Management_3"></a>

#### Session Management

- **Session**: A time-bound period during which a trader's orders are considered valid
- **Session Start Time**: The beginning of the current session (when it was started or restarted)
- **Expiration Time**: When the current session will expire if not renewed
- **Timeout**: The duration for which a keep-alive update remains valid


<a id="@Order_Validation_4"></a>

#### Order Validation

An order is considered valid if:
1. The trader has no keep-alive state set (no dead man's switch enabled), OR
2. The order was created after the current session started, AND
3. The current time is before the session expiration time


<a id="@Session_Lifecycle_5"></a>

#### Session Lifecycle


**First Keep-Alive Update:**
- Creates a new session with <code>session_start_time = 0</code> (all existing orders remain valid)
- Sets <code>expiration_time = current_time + timeout</code>

**Subsequent Updates (Before Expiration):**
- Extends the current session: <code>expiration_time = current_time + timeout</code>
- Keeps the same <code>session_start_time</code> (existing orders remain valid)

**Update After Expiration:**
- Starts a new session: <code>session_start_time = current_time</code>
- Sets new <code>expiration_time = current_time + timeout</code>
- All orders placed before this time are invalidated


<a id="@Events_6"></a>

### Events


- <code><a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveUpdateEvent">KeepAliveUpdateEvent</a></code>: Emitted when a trader updates their keep-alive state
- <code><a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveDisabledEvent">KeepAliveDisabledEvent</a></code>: Emitted when a trader disables their keep-alive
- <code><a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_MinKeepAliveTimeUpdatedEvent">MinKeepAliveTimeUpdatedEvent</a></code>: Emitted when the minimum keep-alive time is updated


-  [Dead Man's Switch Tracker Module](#@Dead_Man's_Switch_Tracker_Module_0)
    -  [Overview](#@Overview_1)
    -  [Key Concepts](#@Key_Concepts_2)
        -  [Session Management](#@Session_Management_3)
        -  [Order Validation](#@Order_Validation_4)
        -  [Session Lifecycle](#@Session_Lifecycle_5)
    -  [Events](#@Events_6)
-  [Enum `KeepAliveUpdateEvent`](#0x7_dead_mans_switch_tracker_KeepAliveUpdateEvent)
-  [Enum `KeepAliveDisabledEvent`](#0x7_dead_mans_switch_tracker_KeepAliveDisabledEvent)
-  [Enum `MinKeepAliveTimeUpdatedEvent`](#0x7_dead_mans_switch_tracker_MinKeepAliveTimeUpdatedEvent)
-  [Struct `KeepAliveState`](#0x7_dead_mans_switch_tracker_KeepAliveState)
-  [Struct `DeadMansSwitchTracker`](#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker)
-  [Constants](#@Constants_7)
-  [Function `new_dead_mans_switch_tracker`](#0x7_dead_mans_switch_tracker_new_dead_mans_switch_tracker)
    -  [Parameters](#@Parameters_8)
    -  [Returns](#@Returns_9)
    -  [Example](#@Example_10)
-  [Function `set_min_keep_alive_time_secs`](#0x7_dead_mans_switch_tracker_set_min_keep_alive_time_secs)
-  [Function `is_order_valid`](#0x7_dead_mans_switch_tracker_is_order_valid)
    -  [Parameters](#@Parameters_11)
    -  [Returns](#@Returns_12)
    -  [Validation Logic](#@Validation_Logic_13)
    -  [Example](#@Example_14)
-  [Function `disable_keep_alive`](#0x7_dead_mans_switch_tracker_disable_keep_alive)
-  [Function `keep_alive`](#0x7_dead_mans_switch_tracker_keep_alive)
    -  [Parameters](#@Parameters_15)
    -  [Special Cases](#@Special_Cases_16)
    -  [Errors](#@Errors_17)
    -  [Effects](#@Effects_18)
    -  [Example](#@Example_19)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map">0x1::big_ordered_map</a>;
<b>use</b> <a href="../../aptos-framework/doc/event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-framework/doc/timestamp.md#0x1_timestamp">0x1::timestamp</a>;
</code></pre>



<a id="0x7_dead_mans_switch_tracker_KeepAliveUpdateEvent"></a>

## Enum `KeepAliveUpdateEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveUpdateEvent">KeepAliveUpdateEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>parent: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>session_start_time_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_time_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_dead_mans_switch_tracker_KeepAliveDisabledEvent"></a>

## Enum `KeepAliveDisabledEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveDisabledEvent">KeepAliveDisabledEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>parent: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>was_registered: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_dead_mans_switch_tracker_MinKeepAliveTimeUpdatedEvent"></a>

## Enum `MinKeepAliveTimeUpdatedEvent`



<pre><code>#[<a href="../../aptos-framework/doc/event.md#0x1_event">event</a>]
enum <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_MinKeepAliveTimeUpdatedEvent">MinKeepAliveTimeUpdatedEvent</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Variants</summary>


<details>
<summary>V1</summary>


<details>
<summary>Fields</summary>


<dl>
<dt>
<code>parent: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>market: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>old_min_keep_alive_time_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>new_min_keep_alive_time_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

</details>

</details>

<a id="0x7_dead_mans_switch_tracker_KeepAliveState"></a>

## Struct `KeepAliveState`



<pre><code><b>struct</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveState">KeepAliveState</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>session_start_time_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>expiration_time_secs: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_dead_mans_switch_tracker_DeadMansSwitchTracker"></a>

## Struct `DeadMansSwitchTracker`



<pre><code><b>struct</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>min_keep_alive_time_secs: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>state: <a href="../../aptos-framework/doc/big_ordered_map.md#0x1_big_ordered_map_BigOrderedMap">big_ordered_map::BigOrderedMap</a>&lt;<b>address</b>, <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveState">dead_mans_switch_tracker::KeepAliveState</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_7"></a>

## Constants


<a id="0x7_dead_mans_switch_tracker_E_KEEP_ALIVE_TIMEOUT_TOO_SHORT"></a>

Error code when the provided keep-alive timeout is shorter than the minimum allowed


<pre><code><b>const</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_E_KEEP_ALIVE_TIMEOUT_TOO_SHORT">E_KEEP_ALIVE_TIMEOUT_TOO_SHORT</a>: u64 = 0;
</code></pre>



<a id="0x7_dead_mans_switch_tracker_new_dead_mans_switch_tracker"></a>

## Function `new_dead_mans_switch_tracker`

Creates a new dead man's switch tracker


<a id="@Parameters_8"></a>

### Parameters

- <code>min_keep_alive_time_secs</code>: Minimum timeout duration that traders must use.
This prevents abuse by forcing traders to set reasonable timeout periods.


<a id="@Returns_9"></a>

### Returns

A new <code><a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a></code> instance with no active sessions


<a id="@Example_10"></a>

### Example

```move
let tracker = new_dead_mans_switch_tracker(60); // 60 second minimum
```


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_new_dead_mans_switch_tracker">new_dead_mans_switch_tracker</a>(min_keep_alive_time_secs: u64): <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_new_dead_mans_switch_tracker">new_dead_mans_switch_tracker</a>(
    min_keep_alive_time_secs: u64
): <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a> {
    <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a> {
        min_keep_alive_time_secs,
        state: <a href="order_book_utils.md#0x7_order_book_utils_new_default_big_ordered_map">order_book_utils::new_default_big_ordered_map</a>()
    }
}
</code></pre>



</details>

<a id="0x7_dead_mans_switch_tracker_set_min_keep_alive_time_secs"></a>

## Function `set_min_keep_alive_time_secs`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_set_min_keep_alive_time_secs">set_min_keep_alive_time_secs</a>(tracker: &<b>mut</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>, parent: <b>address</b>, market: <b>address</b>, min_keep_alive_time_secs: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_set_min_keep_alive_time_secs">set_min_keep_alive_time_secs</a>(
    tracker: &<b>mut</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a>,
    parent: <b>address</b>,
    market: <b>address</b>,
    min_keep_alive_time_secs: u64
) {
    <b>let</b> old_min_keep_alive_time_secs = tracker.min_keep_alive_time_secs;
    tracker.min_keep_alive_time_secs = min_keep_alive_time_secs;
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
        MinKeepAliveTimeUpdatedEvent::V1 {
            parent,
            market,
            old_min_keep_alive_time_secs,
            new_min_keep_alive_time_secs: min_keep_alive_time_secs
        }
    );
}
</code></pre>



</details>

<a id="0x7_dead_mans_switch_tracker_is_order_valid"></a>

## Function `is_order_valid`

Checks if an order is valid based on the dead man's switch state

An order is valid if:
1. No keep-alive state exists for the account (dead man's switch not enabled), OR
2. The order was created after the current session started AND the session hasn't expired


<a id="@Parameters_11"></a>

### Parameters

- <code>tracker</code>: Reference to the dead man's switch tracker
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The trader's address
- <code>order_creation_time_secs</code>: When the order was created (in seconds since epoch)


<a id="@Returns_12"></a>

### Returns

<code><b>true</b></code> if the order is valid, <code><b>false</b></code> if it should be cancelled


<a id="@Validation_Logic_13"></a>

### Validation Logic

```
if no keep-alive state:
return true  // No dead man's switch, all orders valid
if order_creation_time < session_start_time:
return false  // Order from expired session
if current_time > expiration_time:
return false  // Session expired (exclusive of expiration time)
return true  // Order valid
```


<a id="@Example_14"></a>

### Example

```move
let order_time = 1000;
let is_valid = is_order_valid(&tracker, trader_addr, order_time);
if (!is_valid) {
// Cancel the order
}
```


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_is_order_valid">is_order_valid</a>(tracker: &<a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, order_creation_time_secs: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_is_order_valid">is_order_valid</a>(
    tracker: &<a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    order_creation_time_secs: Option&lt;u64&gt;
): bool {
    <b>let</b> itr = tracker.state.internal_find(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>if</b> (itr.iter_is_end(&tracker.state)) {
        // No keep-alive set, so all orders are valid
        <b>return</b> <b>true</b>;
    };
    <b>let</b> current_time = aptos_std::timestamp::now_seconds();
    <b>let</b> order_creation_time_secs =
        <b>if</b> (order_creation_time_secs.is_some()) {
            order_creation_time_secs.destroy_some()
        } <b>else</b> {
            current_time
        };
    <b>let</b> state = itr.iter_borrow(&tracker.state);
    <b>if</b> (state.session_start_time_secs &gt; order_creation_time_secs) {
        // Order was placed before the session started, so it is invalid
        <b>return</b> <b>false</b>;
    };
    state.expiration_time_secs &gt;= current_time
}
</code></pre>



</details>

<a id="0x7_dead_mans_switch_tracker_disable_keep_alive"></a>

## Function `disable_keep_alive`



<pre><code><b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_disable_keep_alive">disable_keep_alive</a>(tracker: &<b>mut</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>, parent: <b>address</b>, market: <b>address</b>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_disable_keep_alive">disable_keep_alive</a>(
    tracker: &<b>mut</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a>,
    parent: <b>address</b>,
    market: <b>address</b>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>
) {
    <b>let</b> removed = tracker.state.remove_or_none(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>let</b> was_registered = removed.is_some();
    <b>if</b> (was_registered) {
        <b>let</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveState">KeepAliveState</a> { session_start_time_secs: _, expiration_time_secs: _ } =
            removed.destroy_some();
    } <b>else</b> {
        removed.destroy_none();
    };
    <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
        KeepAliveDisabledEvent::V1 { parent, market, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, was_registered }
    );
}
</code></pre>



</details>

<a id="0x7_dead_mans_switch_tracker_keep_alive"></a>

## Function `keep_alive`

Updates the keep-alive state for a trader

This is the core function traders call to maintain their session and prevent
their orders from expiring. Behavior depends on the current state:

1. **First Update (No Prior State)**:
- Creates a new session with <code>session_start_time = 0</code>
- All existing orders remain valid
- Sets <code>expiration_time = current_time + timeout_seconds</code>

2. **Update Within Valid Session**:
- Extends the current session
- Updates <code>expiration_time = current_time + timeout_seconds</code>
- Keeps existing <code>session_start_time</code> (orders remain valid)

3. **Update After Session Expired**:
- Starts a new session with <code>session_start_time = current_time</code>
- All orders placed before now are invalidated
- Sets <code>expiration_time = current_time + timeout_seconds</code>


<a id="@Parameters_15"></a>

### Parameters

- <code>tracker</code>: Mutable reference to the dead man's switch tracker
- <code><a href="../../aptos-framework/doc/account.md#0x1_account">account</a></code>: The trader's address
- <code>timeout_seconds</code>: Duration in seconds until the session expires.
Must be >= <code>min_keep_alive_time_secs</code> or 0 to disable.


<a id="@Special_Cases_16"></a>

### Special Cases

- If <code>timeout_seconds == 0</code>: Disables the keep-alive (calls <code>disable_keep_alive</code>)


<a id="@Errors_17"></a>

### Errors

- <code><a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_E_KEEP_ALIVE_TIMEOUT_TOO_SHORT">E_KEEP_ALIVE_TIMEOUT_TOO_SHORT</a></code>: If timeout is less than the minimum and not zero


<a id="@Effects_18"></a>

### Effects

- Updates or creates the trader's keep-alive state
- Emits a <code><a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveUpdateEvent">KeepAliveUpdateEvent</a></code>


<a id="@Example_19"></a>

### Example

```move
// Update with 5 minute timeout
update_keep_alive_state(&mut tracker, trader_addr, 300);

// Disable dead man's switch
update_keep_alive_state(&mut tracker, trader_addr, 0);
```


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_keep_alive">keep_alive</a>(tracker: &<b>mut</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">dead_mans_switch_tracker::DeadMansSwitchTracker</a>, parent: <b>address</b>, market: <b>address</b>, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>, timeout_seconds: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_keep_alive">keep_alive</a>(
    tracker: &<b>mut</b> <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_DeadMansSwitchTracker">DeadMansSwitchTracker</a>,
    parent: <b>address</b>,
    market: <b>address</b>,
    <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>: <b>address</b>,
    timeout_seconds: u64
) {
    <b>if</b> (timeout_seconds == 0) {
        <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_disable_keep_alive">disable_keep_alive</a>(tracker, parent, market, <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
        <b>return</b>;
    };
    <b>assert</b>!(
        timeout_seconds &gt;= tracker.min_keep_alive_time_secs,
        <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_E_KEEP_ALIVE_TIMEOUT_TOO_SHORT">E_KEEP_ALIVE_TIMEOUT_TOO_SHORT</a> // ERROR_KEEP_ALIVE_TIMEOUT_TOO_SHORT
    );
    <b>let</b> current_time = aptos_std::timestamp::now_seconds();
    <b>let</b> expiration_time = current_time + timeout_seconds;
    <b>let</b> itr = tracker.state.internal_find(&<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>);
    <b>if</b> (!itr.iter_is_end(&tracker.state)) {
        <b>let</b> state = itr.iter_borrow_mut(&<b>mut</b> tracker.state);
        <b>if</b> (current_time &gt; state.expiration_time_secs) {
            // Start a new session - this means <a href="../../aptos-framework/../aptos-stdlib/doc/any.md#0x1_any">any</a> order placed before this time is invalidated
            state.session_start_time_secs = current_time;
        };
        // Update existing session
        state.expiration_time_secs = expiration_time;
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            KeepAliveUpdateEvent::V1 {
                parent,
                market,
                <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
                session_start_time_secs: state.session_start_time_secs,
                expiration_time_secs: state.expiration_time_secs
            }
        );
    } <b>else</b> {
        <b>let</b> new_state = <a href="dead_mans_switch_tracker.md#0x7_dead_mans_switch_tracker_KeepAliveState">KeepAliveState</a> {
            session_start_time_secs: 0, // this means that all existing orders are valid
            expiration_time_secs: expiration_time
        };
        tracker.state.add(<a href="../../aptos-framework/doc/account.md#0x1_account">account</a>, new_state);
        <a href="../../aptos-framework/doc/event.md#0x1_event_emit">event::emit</a>(
            KeepAliveUpdateEvent::V1 {
                parent,
                market,
                <a href="../../aptos-framework/doc/account.md#0x1_account">account</a>,
                session_start_time_secs: 0,
                expiration_time_secs: expiration_time
            }
        );
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
