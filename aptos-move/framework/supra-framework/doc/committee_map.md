
<a id="0x1_committee_map"></a>

# Module `0x1::committee_map`

Design:
CommitteeInfoStore: store all the committee information, supported operation: add, remove
CommitteeInfo: store all the node information, supported operation: add, remove
NodeInfo: store the node information, supported operation: update
NodeData: store the node information with operator's address
requirements:
1. two committee has different id
2. two indentical node should not be in the same committee


-  [Struct `OwnerCap`](#0x1_committee_map_OwnerCap)
-  [Resource `SupraCommitteeEventHandler`](#0x1_committee_map_SupraCommitteeEventHandler)
-  [Resource `CommitteeInfoStore`](#0x1_committee_map_CommitteeInfoStore)
-  [Struct `CommitteeInfo`](#0x1_committee_map_CommitteeInfo)
-  [Struct `NodeInfo`](#0x1_committee_map_NodeInfo)
-  [Struct `NodeData`](#0x1_committee_map_NodeData)
-  [Struct `AddCommitteeEvent`](#0x1_committee_map_AddCommitteeEvent)
-  [Struct `RemoveCommitteeEvent`](#0x1_committee_map_RemoveCommitteeEvent)
-  [Struct `UpdateCommitteeEvent`](#0x1_committee_map_UpdateCommitteeEvent)
-  [Struct `AddCommitteeMemberEvent`](#0x1_committee_map_AddCommitteeMemberEvent)
-  [Struct `RemoveCommitteeMemberEvent`](#0x1_committee_map_RemoveCommitteeMemberEvent)
-  [Struct `UpdateNodeInfoEvent`](#0x1_committee_map_UpdateNodeInfoEvent)
-  [Struct `CreateCommitteeInfoStoreEvent`](#0x1_committee_map_CreateCommitteeInfoStoreEvent)
-  [Struct `UpdateDkgFlagEvent`](#0x1_committee_map_UpdateDkgFlagEvent)
-  [Constants](#@Constants_0)
-  [Function `does_node_exist`](#0x1_committee_map_does_node_exist)
-  [Function `ensure_node_address_exist`](#0x1_committee_map_ensure_node_address_exist)
-  [Function `create_owner`](#0x1_committee_map_create_owner)
-  [Function `create_committeeInfo_store`](#0x1_committee_map_create_committeeInfo_store)
-  [Function `create_event_handler`](#0x1_committee_map_create_event_handler)
-  [Function `get_committeeInfo_address`](#0x1_committee_map_get_committeeInfo_address)
-  [Function `init_module`](#0x1_committee_map_init_module)
-  [Function `validate_committee_type`](#0x1_committee_map_validate_committee_type)
-  [Function `get_committee_info`](#0x1_committee_map_get_committee_info)
-  [Function `get_committee_ids`](#0x1_committee_map_get_committee_ids)
-  [Function `get_committee_id`](#0x1_committee_map_get_committee_id)
-  [Function `get_node_info`](#0x1_committee_map_get_node_info)
-  [Function `get_committee_id_for_node`](#0x1_committee_map_get_committee_id_for_node)
-  [Function `get_peers_for_node`](#0x1_committee_map_get_peers_for_node)
-  [Function `does_com_have_dkg`](#0x1_committee_map_does_com_have_dkg)
-  [Function `update_dkg_flag`](#0x1_committee_map_update_dkg_flag)
-  [Function `upsert_committee`](#0x1_committee_map_upsert_committee)
-  [Function `upsert_committee_bulk`](#0x1_committee_map_upsert_committee_bulk)
-  [Function `remove_committee`](#0x1_committee_map_remove_committee)
-  [Function `remove_committee_bulk`](#0x1_committee_map_remove_committee_bulk)
-  [Function `upsert_committee_member`](#0x1_committee_map_upsert_committee_member)
-  [Function `upsert_committee_member_bulk`](#0x1_committee_map_upsert_committee_member_bulk)
-  [Function `remove_committee_member`](#0x1_committee_map_remove_committee_member)
-  [Function `find_node_in_committee`](#0x1_committee_map_find_node_in_committee)
-  [Specification](#@Specification_1)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">0x1::capability</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map">0x1::simple_map</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_committee_map_OwnerCap"></a>

## Struct `OwnerCap`

Capability that grants an owner the right to perform action.


<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>dummy_field: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_SupraCommitteeEventHandler"></a>

## Resource `SupraCommitteeEventHandler`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>create: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_CreateCommitteeInfoStoreEvent">committee_map::CreateCommitteeInfoStoreEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_committee: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_AddCommitteeEvent">committee_map::AddCommitteeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>remove_committee: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_RemoveCommitteeEvent">committee_map::RemoveCommitteeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_committee: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_UpdateCommitteeEvent">committee_map::UpdateCommitteeEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>add_committee_member: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_AddCommitteeMemberEvent">committee_map::AddCommitteeMemberEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>remove_committee_member: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_RemoveCommitteeMemberEvent">committee_map::RemoveCommitteeMemberEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_node_info: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_UpdateNodeInfoEvent">committee_map::UpdateNodeInfoEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>update_dkg_flag: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="committee_map.md#0x1_committee_map_UpdateDkgFlagEvent">committee_map::UpdateDkgFlagEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_CommitteeInfoStore"></a>

## Resource `CommitteeInfoStore`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code><a href="committee_map.md#0x1_committee_map">committee_map</a>: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;u64, <a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>node_to_committee_map: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_CommitteeInfo"></a>

## Struct `CommitteeInfo`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfo">CommitteeInfo</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>map: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_SimpleMap">simple_map::SimpleMap</a>&lt;<b>address</b>, <a href="committee_map.md#0x1_committee_map_NodeInfo">committee_map::NodeInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>has_valid_dkg: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>committee_type: u8</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_NodeInfo"></a>

## Struct `NodeInfo`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_NodeInfo">NodeInfo</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_port: u16</code>
</dt>
<dd>

</dd>
<dt>
<code>rpc_port: u16</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_NodeData"></a>

## Struct `NodeData`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>operator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>network_port: u16</code>
</dt>
<dd>

</dd>
<dt>
<code>rpc_port: u16</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_AddCommitteeEvent"></a>

## Struct `AddCommitteeEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_AddCommitteeEvent">AddCommitteeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>committee_info: <a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_RemoveCommitteeEvent"></a>

## Struct `RemoveCommitteeEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_RemoveCommitteeEvent">RemoveCommitteeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>committee_info: <a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_UpdateCommitteeEvent"></a>

## Struct `UpdateCommitteeEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_UpdateCommitteeEvent">UpdateCommitteeEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>old_committee_info: <a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_committee_info: <a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_AddCommitteeMemberEvent"></a>

## Struct `AddCommitteeMemberEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_AddCommitteeMemberEvent">AddCommitteeMemberEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>committee_member: <a href="committee_map.md#0x1_committee_map_NodeInfo">committee_map::NodeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_RemoveCommitteeMemberEvent"></a>

## Struct `RemoveCommitteeMemberEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_RemoveCommitteeMemberEvent">RemoveCommitteeMemberEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>committee_member: <a href="committee_map.md#0x1_committee_map_NodeInfo">committee_map::NodeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_UpdateNodeInfoEvent"></a>

## Struct `UpdateNodeInfoEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_UpdateNodeInfoEvent">UpdateNodeInfoEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>old_node_info: <a href="committee_map.md#0x1_committee_map_NodeInfo">committee_map::NodeInfo</a></code>
</dt>
<dd>

</dd>
<dt>
<code>new_node_info: <a href="committee_map.md#0x1_committee_map_NodeInfo">committee_map::NodeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_CreateCommitteeInfoStoreEvent"></a>

## Struct `CreateCommitteeInfoStoreEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_CreateCommitteeInfoStoreEvent">CreateCommitteeInfoStoreEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>committee_info: <a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_committee_map_UpdateDkgFlagEvent"></a>

## Struct `UpdateDkgFlagEvent`



<pre><code><b>struct</b> <a href="committee_map.md#0x1_committee_map_UpdateDkgFlagEvent">UpdateDkgFlagEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>committee_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>flag_value: bool</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_committee_map_CLAN"></a>



<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_CLAN">CLAN</a>: u8 = 2;
</code></pre>



<a id="0x1_committee_map_FAMILY"></a>

Define the CommitteeType as constants


<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_FAMILY">FAMILY</a>: u8 = 1;
</code></pre>



<a id="0x1_committee_map_INVALID_COMMITTEE_ID"></a>

The committee is not found


<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_ID">INVALID_COMMITTEE_ID</a>: u64 = 6;
</code></pre>



<a id="0x1_committee_map_INVALID_COMMITTEE_NUMBERS"></a>

The number of committee is not equal to the number of committee member


<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>: u64 = 4;
</code></pre>



<a id="0x1_committee_map_INVALID_COMMITTEE_TYPE"></a>

The committee type is invalid


<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_TYPE">INVALID_COMMITTEE_TYPE</a>: u64 = 7;
</code></pre>



<a id="0x1_committee_map_INVALID_NODE_NUMBERS"></a>

The number of nodes in the committee is invalid


<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_INVALID_NODE_NUMBERS">INVALID_NODE_NUMBERS</a>: u64 = 8;
</code></pre>



<a id="0x1_committee_map_NODE_NOT_FOUND"></a>

The node is not found in the committee


<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_NODE_NOT_FOUND">NODE_NOT_FOUND</a>: u64 = 5;
</code></pre>



<a id="0x1_committee_map_SEED_COMMITTEE"></a>



<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_SEED_COMMITTEE">SEED_COMMITTEE</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [115, 117, 112, 114, 97, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 99, 111, 109, 109, 105, 116, 116, 101, 101, 95, 109, 97, 112, 58, 58, 67, 111, 109, 109, 105, 116, 116, 101, 101, 73, 110, 102, 111, 83, 116, 111, 114, 101];
</code></pre>



<a id="0x1_committee_map_TRIBE"></a>



<pre><code><b>const</b> <a href="committee_map.md#0x1_committee_map_TRIBE">TRIBE</a>: u8 = 3;
</code></pre>



<a id="0x1_committee_map_does_node_exist"></a>

## Function `does_node_exist`

Internal - check if the node exists in the committee


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_does_node_exist">does_node_exist</a>(committee: &<a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a>, node_address: <b>address</b>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_does_node_exist">does_node_exist</a>(committee: &<a href="committee_map.md#0x1_committee_map_CommitteeInfo">CommitteeInfo</a>, node_address: <b>address</b>): bool {
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&committee.map, &node_address)
}
</code></pre>



</details>

<a id="0x1_committee_map_ensure_node_address_exist"></a>

## Function `ensure_node_address_exist`

Internal - Assert if the node exists in the committee


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_ensure_node_address_exist">ensure_node_address_exist</a>(committee: &<a href="committee_map.md#0x1_committee_map_CommitteeInfo">committee_map::CommitteeInfo</a>, node_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_ensure_node_address_exist">ensure_node_address_exist</a>(committee: &<a href="committee_map.md#0x1_committee_map_CommitteeInfo">CommitteeInfo</a>, node_address: <b>address</b>) {
    <b>assert</b>!(<a href="committee_map.md#0x1_committee_map_does_node_exist">does_node_exist</a>(committee, node_address), <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_NODE_NOT_FOUND">NODE_NOT_FOUND</a>))
}
</code></pre>



</details>

<a id="0x1_committee_map_create_owner"></a>

## Function `create_owner`

Internal - create OwnerCap


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_create_owner">create_owner</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_create_owner">create_owner</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="../../aptos-stdlib/doc/capability.md#0x1_capability_create">capability::create</a>&lt;<a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a>&gt;(owner_signer, &<a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> {});
}
</code></pre>



</details>

<a id="0x1_committee_map_create_committeeInfo_store"></a>

## Function `create_committeeInfo_store`

Internal - create committeeInfo store functions


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_create_committeeInfo_store">create_committeeInfo_store</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_create_committeeInfo_store">create_committeeInfo_store</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a> {
    <b>let</b> (resource_signer, _) = <a href="account.md#0x1_account_create_resource_account">account::create_resource_account</a>(owner_signer, <a href="committee_map.md#0x1_committee_map_SEED_COMMITTEE">SEED_COMMITTEE</a>);
    <b>move_to</b>(&resource_signer, <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
        <a href="committee_map.md#0x1_committee_map">committee_map</a>: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>(),
        node_to_committee_map: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new">simple_map::new</a>()
    });
    resource_signer
}
</code></pre>



</details>

<a id="0x1_committee_map_create_event_handler"></a>

## Function `create_event_handler`

Internal - create event handler


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_create_event_handler">create_event_handler</a>(resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_create_event_handler">create_event_handler</a>(resource_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(resource_signer, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
        create: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_CreateCommitteeInfoStoreEvent">CreateCommitteeInfoStoreEvent</a>&gt;(resource_signer),
        add_committee: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_AddCommitteeEvent">AddCommitteeEvent</a>&gt;(resource_signer),
        remove_committee: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_RemoveCommitteeEvent">RemoveCommitteeEvent</a>&gt;(resource_signer),
        update_committee: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_UpdateCommitteeEvent">UpdateCommitteeEvent</a>&gt;(resource_signer),
        add_committee_member: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_AddCommitteeMemberEvent">AddCommitteeMemberEvent</a>&gt;(resource_signer),
        remove_committee_member: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_RemoveCommitteeMemberEvent">RemoveCommitteeMemberEvent</a>&gt;(resource_signer),
        update_node_info: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_UpdateNodeInfoEvent">UpdateNodeInfoEvent</a>&gt;(resource_signer),
        update_dkg_flag: new_event_handle&lt;<a href="committee_map.md#0x1_committee_map_UpdateDkgFlagEvent">UpdateDkgFlagEvent</a>&gt;(resource_signer),
    });
}
</code></pre>



</details>

<a id="0x1_committee_map_get_committeeInfo_address"></a>

## Function `get_committeeInfo_address`



<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committeeInfo_address">get_committeeInfo_address</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committeeInfo_address">get_committeeInfo_address</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>): <b>address</b> {
    <a href="account.md#0x1_account_create_resource_address">account::create_resource_address</a>(&<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(owner_signer), <a href="committee_map.md#0x1_committee_map_SEED_COMMITTEE">SEED_COMMITTEE</a>)
}
</code></pre>



</details>

<a id="0x1_committee_map_init_module"></a>

## Function `init_module`

Its Initial function which will be executed automatically while deployed packages


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_init_module">init_module</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_init_module">init_module</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="committee_map.md#0x1_committee_map_create_owner">create_owner</a>(owner_signer);
    <b>let</b> resource_signer = <a href="committee_map.md#0x1_committee_map_create_committeeInfo_store">create_committeeInfo_store</a>(owner_signer);
    <a href="committee_map.md#0x1_committee_map_create_event_handler">create_event_handler</a>(&resource_signer);
}
</code></pre>



</details>

<a id="0x1_committee_map_validate_committee_type"></a>

## Function `validate_committee_type`



<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_validate_committee_type">validate_committee_type</a>(committee_type: u8, num_of_nodes: u64): u8
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="committee_map.md#0x1_committee_map_validate_committee_type">validate_committee_type</a>(committee_type: u8, num_of_nodes: u64): u8 {
    <b>assert</b>!(committee_type &gt;= <a href="committee_map.md#0x1_committee_map_FAMILY">FAMILY</a> && committee_type &lt;= <a href="committee_map.md#0x1_committee_map_TRIBE">TRIBE</a>, <a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_TYPE">INVALID_COMMITTEE_TYPE</a>);
    <b>if</b> (committee_type == <a href="committee_map.md#0x1_committee_map_FAMILY">FAMILY</a>) {
        // f+1, number of nodes in a family committee should be greater than 1
        <b>assert</b>!(num_of_nodes &gt; 1, <a href="committee_map.md#0x1_committee_map_INVALID_NODE_NUMBERS">INVALID_NODE_NUMBERS</a>);
    } <b>else</b> <b>if</b> (committee_type == <a href="committee_map.md#0x1_committee_map_CLAN">CLAN</a>) {
        // 2f+1, number of nodes in a clan committee should be odd and greater than 3
        <b>assert</b>!(num_of_nodes &gt;= 3 && num_of_nodes % 2 == 1, <a href="committee_map.md#0x1_committee_map_INVALID_NODE_NUMBERS">INVALID_NODE_NUMBERS</a>);
    } <b>else</b> {
        // 3f+1, number of nodes in a tribe committee should be in the format of 3f+1 and greater than 4
        <b>assert</b>!(num_of_nodes &gt;= 4 && (num_of_nodes - 1) % 3 == 0, <a href="committee_map.md#0x1_committee_map_INVALID_NODE_NUMBERS">INVALID_NODE_NUMBERS</a>);
    };
    committee_type
}
</code></pre>



</details>

<a id="0x1_committee_map_get_committee_info"></a>

## Function `get_committee_info`

Get the committee's node vector and committee type


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_info">get_committee_info</a>(com_store_addr: <b>address</b>, id: u64): (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="committee_map.md#0x1_committee_map_NodeData">committee_map::NodeData</a>&gt;, u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_info">get_committee_info</a>(com_store_addr: <b>address</b>, id: u64): (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a>&gt;, u8) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_store = <b>borrow_global</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> committee = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &id);
    <b>let</b> (addrs, nodes) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_to_vec_pair">simple_map::to_vec_pair</a>(committee.map);
    <b>let</b> node_data_vec = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a>&gt;();
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&addrs) != 0) {
        <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> addrs);
        <b>let</b> node_info = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> nodes);
        <b>let</b> node_data = <a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a> {
            operator: addr,
            ip_public_address: node_info.ip_public_address,
            node_public_key: node_info.node_public_key,
            network_public_key: node_info.network_public_key,
            cg_public_key: node_info.cg_public_key,
            network_port: node_info.network_port,
            rpc_port: node_info.rpc_port,
        };
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> node_data_vec, node_data);
    };
    (node_data_vec, committee.committee_type)
}
</code></pre>



</details>

<a id="0x1_committee_map_get_committee_ids"></a>

## Function `get_committee_ids`

Get the committee's ids from the store


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_ids">get_committee_ids</a>(com_store_addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_ids">get_committee_ids</a>(com_store_addr: <b>address</b>): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_store = <b>borrow_global</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_keys">simple_map::keys</a>(&committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>)
}
</code></pre>



</details>

<a id="0x1_committee_map_get_committee_id"></a>

## Function `get_committee_id`

Get the committee's id for a single node, only pass the address is okay


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_id">get_committee_id</a>(com_store_addr: <b>address</b>, node_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_id">get_committee_id</a>(
    com_store_addr: <b>address</b>,
    node_address: <b>address</b>
): u64 <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_store = <b>borrow_global</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee_store.node_to_committee_map, &node_address)
}
</code></pre>



</details>

<a id="0x1_committee_map_get_node_info"></a>

## Function `get_node_info`

Get the node's information


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_node_info">get_node_info</a>(com_store_addr: <b>address</b>, id: u64, node_address: <b>address</b>): <a href="committee_map.md#0x1_committee_map_NodeData">committee_map::NodeData</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_node_info">get_node_info</a>(
    com_store_addr: <b>address</b>,
    id: u64,
    node_address: <b>address</b>
): <a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a> <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_store = <b>borrow_global</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> committee = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &id);
    <b>let</b> (addrs, nodes) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_to_vec_pair">simple_map::to_vec_pair</a>(committee.map);
    <b>let</b> (flag, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&addrs, &node_address);
    <b>assert</b>!(flag, <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_NODE_NOT_FOUND">NODE_NOT_FOUND</a>));
    <b>let</b> node_info = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&nodes, index);

    <a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a> {
        operator: node_address,
        ip_public_address: node_info.ip_public_address,
        node_public_key: node_info.node_public_key,
        network_public_key: node_info.network_public_key,
        cg_public_key: node_info.cg_public_key,
        network_port: node_info.network_port,
        rpc_port: node_info.rpc_port,
    }
}
</code></pre>



</details>

<a id="0x1_committee_map_get_committee_id_for_node"></a>

## Function `get_committee_id_for_node`

Get the committee's id for a single node


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_id_for_node">get_committee_id_for_node</a>(com_store_addr: <b>address</b>, node_address: <b>address</b>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_committee_id_for_node">get_committee_id_for_node</a>(
    com_store_addr: <b>address</b>,
    node_address: <b>address</b>
): u64 <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_store = <b>borrow_global</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> id = *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee_store.node_to_committee_map, &node_address);
    id
}
</code></pre>



</details>

<a id="0x1_committee_map_get_peers_for_node"></a>

## Function `get_peers_for_node`

Get a tuple of the node itself and node peers vector for a single node


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_peers_for_node">get_peers_for_node</a>(com_store_addr: <b>address</b>, node_address: <b>address</b>): (<a href="committee_map.md#0x1_committee_map_NodeData">committee_map::NodeData</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="committee_map.md#0x1_committee_map_NodeData">committee_map::NodeData</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_get_peers_for_node">get_peers_for_node</a>(
    com_store_addr: <b>address</b>,
    node_address: <b>address</b>
): (<a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a>, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a>&gt;) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_id = <a href="committee_map.md#0x1_committee_map_get_committee_id_for_node">get_committee_id_for_node</a>(com_store_addr, node_address);
    <b>let</b> this_node = <a href="committee_map.md#0x1_committee_map_get_node_info">get_node_info</a>(com_store_addr, committee_id, node_address);
    <b>let</b> (node_info,_) = <a href="committee_map.md#0x1_committee_map_get_committee_info">get_committee_info</a>(com_store_addr, committee_id);
    <b>let</b> (_, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&node_info, &this_node);
    <b>let</b> self= <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> node_info, index);
    (self, node_info)
}
</code></pre>



</details>

<a id="0x1_committee_map_does_com_have_dkg"></a>

## Function `does_com_have_dkg`

Check if the committee has a valid dkg


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_does_com_have_dkg">does_com_have_dkg</a>(com_store_addr: <b>address</b>, com_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_does_com_have_dkg">does_com_have_dkg</a>(com_store_addr: <b>address</b>, com_id: u64): bool <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_store = <b>borrow_global</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> committee = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &com_id);
    committee.has_valid_dkg
}
</code></pre>



</details>

<a id="0x1_committee_map_update_dkg_flag"></a>

## Function `update_dkg_flag`

Update the dkg flag


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_update_dkg_flag">update_dkg_flag</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, com_id: u64, flag_value: bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_update_dkg_flag">update_dkg_flag</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    com_id: u64,
    flag_value: bool
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    // Only the <a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> can access it
    <b>let</b> _acquire = &<a href="../../aptos-stdlib/doc/capability.md#0x1_capability_acquire">capability::acquire</a>(owner_signer, &<a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> {});
    <b>let</b> committee_store = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> committee = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &com_id);
    committee.has_valid_dkg = flag_value;
    <b>let</b> event_handler = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a>&gt;(<a href="committee_map.md#0x1_committee_map_get_committeeInfo_address">get_committeeInfo_address</a>(owner_signer));
    emit_event(
        &<b>mut</b> event_handler.update_dkg_flag,
        <a href="committee_map.md#0x1_committee_map_UpdateDkgFlagEvent">UpdateDkgFlagEvent</a> {
            committee_id: com_id,
            flag_value
        }
    );
}
</code></pre>



</details>

<a id="0x1_committee_map_upsert_committee"></a>

## Function `upsert_committee`

This function is used to add a new committee to the store


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee">upsert_committee</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, id: u64, node_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, network_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;, rpc_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;, committee_type: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee">upsert_committee</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    id: u64,
    node_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    network_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;,
    rpc_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;,
    committee_type: u8
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    // Assert the length of the <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a> for two are the same
    <b>let</b> node_address_len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node_addresses);
    <b>assert</b>!(
        node_address_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ip_public_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        node_address_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node_public_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        node_address_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&network_public_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        node_address_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&cg_public_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        node_address_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&network_port),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        node_address_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&rpc_port),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    // Only the <a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> can access it
    <b>let</b> _acquire = &<a href="../../aptos-stdlib/doc/capability.md#0x1_capability_acquire">capability::acquire</a>(owner_signer, &<a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> {});
    <b>let</b> committee_store = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> node_infos = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="committee_map.md#0x1_committee_map_NodeInfo">NodeInfo</a>&gt;();
    <b>let</b> node_addresses_for_iteration = node_addresses;
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node_addresses_for_iteration) != 0) {
        <b>let</b> ip_public_address = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> ip_public_address);
        <b>let</b> node_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> node_public_key);
        <b>let</b> network_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> network_public_key);
        <b>let</b> cg_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> cg_public_key);
        <b>let</b> network_port = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> network_port);
        <b>let</b> rpc_port = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> rpc_port);
        <b>let</b> node_info = <a href="committee_map.md#0x1_committee_map_NodeInfo">NodeInfo</a> {
            ip_public_address: <b>copy</b> ip_public_address,
            node_public_key: <b>copy</b> node_public_key,
            network_public_key: <b>copy</b> network_public_key,
            cg_public_key: <b>copy</b> cg_public_key,
            network_port,
            rpc_port,
        };
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> node_infos, node_info);
        // Also <b>update</b> the node_to_committee_map
        <b>let</b> node_address = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> node_addresses_for_iteration);
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&<b>mut</b> committee_store.node_to_committee_map, node_address, id);
    };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> node_infos);
    <b>let</b> committee_info = <a href="committee_map.md#0x1_committee_map_CommitteeInfo">CommitteeInfo</a> {
        map: <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_new_from">simple_map::new_from</a>(node_addresses, node_infos),
        has_valid_dkg: <b>false</b>,
        committee_type: <a href="committee_map.md#0x1_committee_map_validate_committee_type">validate_committee_type</a>(committee_type, node_address_len)
    };
    <b>let</b> event_handler = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a>&gt;(<a href="committee_map.md#0x1_committee_map_get_committeeInfo_address">get_committeeInfo_address</a>(owner_signer));
    <b>let</b> (_, value) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&<b>mut</b> committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, id, committee_info);
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&value)) {
        emit_event(
            &<b>mut</b> event_handler.add_committee,
            <a href="committee_map.md#0x1_committee_map_AddCommitteeEvent">AddCommitteeEvent</a> {
                committee_id: id,
                committee_info: <b>copy</b> committee_info
            }, )
    } <b>else</b> {
        emit_event(
            &<b>mut</b> event_handler.update_committee,
            <a href="committee_map.md#0x1_committee_map_UpdateCommitteeEvent">UpdateCommitteeEvent</a> {
                committee_id: id,
                old_committee_info: <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(value),
                new_committee_info: committee_info
            },
        );
        // Destory the map
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_to_vec_pair">simple_map::to_vec_pair</a>(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(value).map);
    };
}
</code></pre>



</details>

<a id="0x1_committee_map_upsert_committee_bulk"></a>

## Function `upsert_committee_bulk`

Add the committee in bulk


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee_bulk">upsert_committee_bulk</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, node_addresses_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;&gt;, ip_public_address_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, node_public_key_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, network_public_key_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, cg_public_key_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, network_port_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;&gt;, rpc_por_bulkt: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;&gt;, committee_types: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee_bulk">upsert_committee_bulk</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    node_addresses_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;&gt;,
    ip_public_address_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    node_public_key_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    network_public_key_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    cg_public_key_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    network_port_bulk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;&gt;,
    rpc_por_bulkt: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;&gt;,
    committee_types: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    // Assert the length of the <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a> for two are the same
    <b>let</b> ids_len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids);
    <b>assert</b>!(
        ids_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node_addresses_bulk),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        ids_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ip_public_address_bulk),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        ids_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node_public_key_bulk),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        ids_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&network_public_key_bulk),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        ids_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&cg_public_key_bulk),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        ids_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&network_port_bulk),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        ids_len == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&rpc_por_bulkt),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) != 0) {
        <b>let</b> id = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> ids);
        <b>let</b> node_addresses = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> node_addresses_bulk);
        <b>let</b> ip_public_address = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> ip_public_address_bulk);
        <b>let</b> node_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> node_public_key_bulk);
        <b>let</b> network_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> network_public_key_bulk);
        <b>let</b> cg_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> cg_public_key_bulk);
        <b>let</b> network_port = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> network_port_bulk);
        <b>let</b> rpc_port = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> rpc_por_bulkt);
        <b>let</b> committee_type = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> committee_types);
        <a href="committee_map.md#0x1_committee_map_upsert_committee">upsert_committee</a>(
            owner_signer,
            com_store_addr,
            id,
            node_addresses,
            ip_public_address,
            node_public_key,
            network_public_key,
            cg_public_key,
            network_port,
            rpc_port,
            committee_type
        );
    }
}
</code></pre>



</details>

<a id="0x1_committee_map_remove_committee"></a>

## Function `remove_committee`

Remove the committee from the store


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_remove_committee">remove_committee</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_remove_committee">remove_committee</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    id: u64
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    // Only the <a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> can access it
    <b>let</b> _acquire = &<a href="../../aptos-stdlib/doc/capability.md#0x1_capability_acquire">capability::acquire</a>(owner_signer, &<a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> {});

    <b>let</b> committee_store = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>assert</b>!(
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &id),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_ID">INVALID_COMMITTEE_ID</a>)
    );
    <b>let</b> (id, committee_info) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &id);
    // Also remove the node_to_committee_map
    <b>let</b> (addrs, _) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_to_vec_pair">simple_map::to_vec_pair</a>(committee_info.map);
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&addrs) != 0) {
        <b>let</b> addr = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> addrs);
        <b>assert</b>!(
            <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&committee_store.node_to_committee_map, &addr),
            <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_NODE_NOT_FOUND">NODE_NOT_FOUND</a>)
        );
        <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> committee_store.node_to_committee_map, &addr);
    };
    <b>let</b> event_handler = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a>&gt;(<a href="committee_map.md#0x1_committee_map_get_committeeInfo_address">get_committeeInfo_address</a>(owner_signer));
    emit_event(
        &<b>mut</b> event_handler.remove_committee,
        <a href="committee_map.md#0x1_committee_map_RemoveCommitteeEvent">RemoveCommitteeEvent</a> {
            committee_id: id,
            committee_info: committee_info
        }, )
}
</code></pre>



</details>

<a id="0x1_committee_map_remove_committee_bulk"></a>

## Function `remove_committee_bulk`

Remove the committee in bulk


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_remove_committee_bulk">remove_committee_bulk</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_remove_committee_bulk">remove_committee_bulk</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) != 0) {
        <b>let</b> id = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> ids);
        <a href="committee_map.md#0x1_committee_map_remove_committee">remove_committee</a>(owner_signer, com_store_addr, id);
    }
}
</code></pre>



</details>

<a id="0x1_committee_map_upsert_committee_member"></a>

## Function `upsert_committee_member`

Upsert the node to the committee


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee_member">upsert_committee_member</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, id: u64, node_address: <b>address</b>, ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, network_port: u16, rpc_port: u16)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee_member">upsert_committee_member</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    id: u64,
    node_address: <b>address</b>,
    ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    network_port: u16,
    rpc_port: u16,
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    // Only the <a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> can access it
    <b>let</b> _acquire = &<a href="../../aptos-stdlib/doc/capability.md#0x1_capability_acquire">capability::acquire</a>(owner_signer, &<a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> {});

    <b>let</b> committee_store = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> committee = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &id);
    <b>let</b> node_info = <a href="committee_map.md#0x1_committee_map_NodeInfo">NodeInfo</a> {
        ip_public_address: <b>copy</b> ip_public_address,
        node_public_key: <b>copy</b> node_public_key,
        network_public_key: <b>copy</b> network_public_key,
        cg_public_key: <b>copy</b> cg_public_key,
        network_port: network_port,
        rpc_port: rpc_port,
    };
    <b>let</b> event_handler = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a>&gt;(<a href="committee_map.md#0x1_committee_map_get_committeeInfo_address">get_committeeInfo_address</a>(owner_signer));
    <b>if</b> (!<a href="committee_map.md#0x1_committee_map_does_node_exist">does_node_exist</a>(committee, node_address)) {
        emit_event(
            &<b>mut</b> event_handler.add_committee_member,
            <a href="committee_map.md#0x1_committee_map_AddCommitteeMemberEvent">AddCommitteeMemberEvent</a> {
                committee_id: id,
                committee_member: node_info
            })
    } <b>else</b> {
        emit_event(
            &<b>mut</b> event_handler.update_node_info,
            <a href="committee_map.md#0x1_committee_map_UpdateNodeInfoEvent">UpdateNodeInfoEvent</a> {
                committee_id: id,
                old_node_info: *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee.map, &node_address),
                new_node_info: node_info
            })
    };
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&<b>mut</b> committee.map, node_address, node_info);
    // Also <b>update</b> the node_to_committee_map
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_upsert">simple_map::upsert</a>(&<b>mut</b> committee_store.node_to_committee_map, node_address, id);
}
</code></pre>



</details>

<a id="0x1_committee_map_upsert_committee_member_bulk"></a>

## Function `upsert_committee_member_bulk`

Upsert nodes to the committee


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee_member_bulk">upsert_committee_member_bulk</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, node_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;, ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, network_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;, rpc_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_upsert_committee_member_bulk">upsert_committee_member_bulk</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    node_addresses: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;,
    ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    network_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;,
    rpc_port: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    // Assert the length of the <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a> for two are the same
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node_addresses),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ip_public_address),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&node_public_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&network_public_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&cg_public_key),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&network_port),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) == <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&rpc_port),
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="committee_map.md#0x1_committee_map_INVALID_COMMITTEE_NUMBERS">INVALID_COMMITTEE_NUMBERS</a>)
    );
    <b>while</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ids) != 0) {
        <b>let</b> id = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> ids);
        <b>let</b> node_address = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> node_addresses);
        <b>let</b> ip_public_address = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> ip_public_address);
        <b>let</b> node_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> node_public_key);
        <b>let</b> network_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> network_public_key);
        <b>let</b> cg_public_key = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> cg_public_key);
        <b>let</b> network_port = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> network_port);
        <b>let</b> rpc_port = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_pop_back">vector::pop_back</a>(&<b>mut</b> rpc_port);
        <a href="committee_map.md#0x1_committee_map_upsert_committee_member">upsert_committee_member</a>(
            owner_signer,
            com_store_addr,
            id,
            node_address,
            ip_public_address,
            node_public_key,
            network_public_key,
            cg_public_key,
            network_port,
            rpc_port
        );
    }
}
</code></pre>



</details>

<a id="0x1_committee_map_remove_committee_member"></a>

## Function `remove_committee_member`

Remove the node from the committee


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_remove_committee_member">remove_committee_member</a>(owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, com_store_addr: <b>address</b>, id: u64, node_address: <b>address</b>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="committee_map.md#0x1_committee_map_remove_committee_member">remove_committee_member</a>(
    owner_signer: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    com_store_addr: <b>address</b>,
    id: u64,
    node_address: <b>address</b>
) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>, <a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a> {
    // Only the <a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> <a href="../../aptos-stdlib/doc/capability.md#0x1_capability">capability</a> can access it
    <b>let</b> _acquire = &<a href="../../aptos-stdlib/doc/capability.md#0x1_capability_acquire">capability::acquire</a>(owner_signer, &<a href="committee_map.md#0x1_committee_map_OwnerCap">OwnerCap</a> {});

    <b>let</b> committee_store = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_addr);
    <b>let</b> committee = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow_mut">simple_map::borrow_mut</a>(&<b>mut</b> committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &id);
    <a href="committee_map.md#0x1_committee_map_ensure_node_address_exist">ensure_node_address_exist</a>(committee, node_address);
    <b>let</b> (_, node_info) = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> committee.map, &node_address);
    <b>let</b> event_handler = <b>borrow_global_mut</b>&lt;<a href="committee_map.md#0x1_committee_map_SupraCommitteeEventHandler">SupraCommitteeEventHandler</a>&gt;(<a href="committee_map.md#0x1_committee_map_get_committeeInfo_address">get_committeeInfo_address</a>(owner_signer));
    emit_event(
        &<b>mut</b> event_handler.remove_committee_member,
        <a href="committee_map.md#0x1_committee_map_RemoveCommitteeMemberEvent">RemoveCommitteeMemberEvent</a> {
            committee_id: id,
            committee_member: <a href="committee_map.md#0x1_committee_map_NodeInfo">NodeInfo</a> {
                ip_public_address: node_info.ip_public_address,
                node_public_key: node_info.node_public_key,
                network_public_key: node_info.network_public_key,
                cg_public_key: node_info.cg_public_key,
                network_port: node_info.network_port,
                rpc_port: node_info.rpc_port,
            }
        }
    );
    // Remove the node from the node_to_committee_map
    <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_remove">simple_map::remove</a>(&<b>mut</b> committee_store.node_to_committee_map, &node_address);
}
</code></pre>



</details>

<a id="0x1_committee_map_find_node_in_committee"></a>

## Function `find_node_in_committee`

Find the node in the committee


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_find_node_in_committee">find_node_in_committee</a>(com_store_add: <b>address</b>, id: u64, node_address: <b>address</b>): (bool, <a href="committee_map.md#0x1_committee_map_NodeData">committee_map::NodeData</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="committee_map.md#0x1_committee_map_find_node_in_committee">find_node_in_committee</a>(
    com_store_add: <b>address</b>,
    id: u64,
    node_address: <b>address</b>
): (bool, <a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a>) <b>acquires</b> <a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a> {
    <b>let</b> committee_store = <b>borrow_global</b>&lt;<a href="committee_map.md#0x1_committee_map_CommitteeInfoStore">CommitteeInfoStore</a>&gt;(com_store_add);
    <b>let</b> committee = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee_store.<a href="committee_map.md#0x1_committee_map">committee_map</a>, &id);
    <b>let</b> flag = <a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_contains_key">simple_map::contains_key</a>(&committee.map, &node_address);
    <b>if</b> (!flag) {
        <b>return</b> (<b>false</b>, <a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a> {
            operator: <b>copy</b> node_address,
            ip_public_address: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            node_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            network_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            cg_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            network_port: 0,
            rpc_port: 0,
        })
    } <b>else</b> {
        <b>let</b> node_info = *<a href="../../aptos-stdlib/doc/simple_map.md#0x1_simple_map_borrow">simple_map::borrow</a>(&committee.map, &node_address);
        (<b>true</b>, <a href="committee_map.md#0x1_committee_map_NodeData">NodeData</a> {
            operator: <b>copy</b> node_address,
            ip_public_address: node_info.ip_public_address,
            node_public_key: node_info.node_public_key,
            network_public_key: node_info.network_public_key,
            cg_public_key: node_info.cg_public_key,
            network_port: node_info.network_port,
            rpc_port: node_info.rpc_port,
        })
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>false</b>;
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
