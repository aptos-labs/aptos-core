
<a id="0x7_large_packages_v2"></a>

# Module `0x7::large_packages_v2`


<a id="@Aptos_Large_Packages_Framework_v2_0"></a>

## Aptos Large Packages Framework v2


This module provides a framework for uploading large packages to the Aptos network with
separate uploader and publisher roles. One user can upload chunks and another can publish.


<a id="@Key_Features_1"></a>

### Key Features

- **Separated Roles**: Allows one address to upload chunks and another to publish
- **Proposal System**: Uses unique proposal IDs to track staged packages
- **Flexible Publishing**: Supports account, object, and object upgrade publishing


<a id="@Workflow_2"></a>

### Workflow

1. **Uploader stages chunks**: Call <code>stage_code_chunk</code> multiple times with proposal ID
2. **Publisher completes**: Call one of the publish functions to deploy the package
3. **Cleanup**: Either party can cleanup canceled proposals


<a id="@Security_Notes_3"></a>

### Security Notes

- Only the designated publisher can complete a proposal
- Proposals are tightly coupled to both uploader and publisher addresses
- Proposals can be canceled by either the uploader or publisher


-  [Aptos Large Packages Framework v2](#@Aptos_Large_Packages_Framework_v2_0)
    -  [Key Features](#@Key_Features_1)
    -  [Workflow](#@Workflow_2)
    -  [Security Notes](#@Security_Notes_3)
-  [Resource `StagingArea`](#0x7_large_packages_v2_StagingArea)
-  [Struct `ProposalKey`](#0x7_large_packages_v2_ProposalKey)
-  [Struct `ProposalData`](#0x7_large_packages_v2_ProposalData)
-  [Constants](#@Constants_4)
-  [Function `init_module`](#0x7_large_packages_v2_init_module)
-  [Function `stage_code_chunk`](#0x7_large_packages_v2_stage_code_chunk)
-  [Function `publish_to_account`](#0x7_large_packages_v2_publish_to_account)
-  [Function `publish_to_object`](#0x7_large_packages_v2_publish_to_object)
-  [Function `upgrade_object_code`](#0x7_large_packages_v2_upgrade_object_code)
-  [Function `cleanup_proposal`](#0x7_large_packages_v2_cleanup_proposal)
-  [Function `stage_code_chunk_internal`](#0x7_large_packages_v2_stage_code_chunk_internal)
-  [Function `remove_proposal_as_publisher`](#0x7_large_packages_v2_remove_proposal_as_publisher)
-  [Function `assemble_module_code`](#0x7_large_packages_v2_assemble_module_code)
-  [Function `destroy_proposal_data`](#0x7_large_packages_v2_destroy_proposal_data)
-  [Function `proposal_exists`](#0x7_large_packages_v2_proposal_exists)
-  [Function `get_proposal_metadata_size`](#0x7_large_packages_v2_get_proposal_metadata_size)
-  [Function `get_proposal_module_count`](#0x7_large_packages_v2_get_proposal_module_count)
-  [Function `get_proposal_module_size`](#0x7_large_packages_v2_get_proposal_module_size)
-  [Function `get_proposal_total_code_size`](#0x7_large_packages_v2_get_proposal_total_code_size)
-  [Function `get_proposal_summary`](#0x7_large_packages_v2_get_proposal_summary)


<pre><code><b>use</b> <a href="../../aptos-framework/doc/code.md#0x1_code">0x1::code</a>;
<b>use</b> <a href="../../aptos-framework/doc/object.md#0x1_object">0x1::object</a>;
<b>use</b> <a href="../../aptos-framework/doc/object_code_deployment.md#0x1_object_code_deployment">0x1::object_code_deployment</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x7_large_packages_v2_StagingArea"></a>

## Resource `StagingArea`

Global staging area that stores all proposals


<pre><code><b>struct</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>proposals: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;<a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">large_packages_v2::ProposalKey</a>, <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">large_packages_v2::ProposalData</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_large_packages_v2_ProposalKey"></a>

## Struct `ProposalKey`

Unique key for each proposal


<pre><code><b>struct</b> <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>uploader: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>publisher: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>proposal_id: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x7_large_packages_v2_ProposalData"></a>

## Struct `ProposalData`

Data for a staged package proposal


<pre><code><b>struct</b> <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">ProposalData</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>metadata_serialized: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code><a href="../../aptos-framework/doc/code.md#0x1_code">code</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_4"></a>

## Constants


<a id="0x7_large_packages_v2_ECODE_MISMATCH"></a>

Code indices and code chunks should be the same length


<pre><code><b>const</b> <a href="large_packages_v2.md#0x7_large_packages_v2_ECODE_MISMATCH">ECODE_MISMATCH</a>: u64 = 1;
</code></pre>



<a id="0x7_large_packages_v2_ENOT_AUTHORIZED_CLEANUP"></a>

Only uploader or publisher can cleanup proposal


<pre><code><b>const</b> <a href="large_packages_v2.md#0x7_large_packages_v2_ENOT_AUTHORIZED_CLEANUP">ENOT_AUTHORIZED_CLEANUP</a>: u64 = 4;
</code></pre>



<a id="0x7_large_packages_v2_ENOT_AUTHORIZED_PUBLISHER"></a>

Only the designated publisher can publish this proposal


<pre><code><b>const</b> <a href="large_packages_v2.md#0x7_large_packages_v2_ENOT_AUTHORIZED_PUBLISHER">ENOT_AUTHORIZED_PUBLISHER</a>: u64 = 2;
</code></pre>



<a id="0x7_large_packages_v2_EPROPOSAL_NOT_FOUND"></a>

Proposal does not exist


<pre><code><b>const</b> <a href="large_packages_v2.md#0x7_large_packages_v2_EPROPOSAL_NOT_FOUND">EPROPOSAL_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x7_large_packages_v2_init_module"></a>

## Function `init_module`

Initialize the module with global staging area


<pre><code><b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_init_module">init_module</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_init_module">init_module</a>(framework: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <b>move_to</b>(framework, <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
        proposals: <a href="../../aptos-framework/../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>&lt;<a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a>, <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">ProposalData</a>&gt;(),
    });
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_stage_code_chunk"></a>

## Function `stage_code_chunk`

Stage code chunks for a proposal (can be called by any uploader)


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_stage_code_chunk">stage_code_chunk</a>(uploader: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, publisher: <b>address</b>, proposal_id: u64, metadata_chunk: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, code_indices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;, code_chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_stage_code_chunk">stage_code_chunk</a>(
    uploader: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    publisher: <b>address</b>,
    proposal_id: u64,
    metadata_chunk: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    code_indices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;,
    code_chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <a href="large_packages_v2.md#0x7_large_packages_v2_stage_code_chunk_internal">stage_code_chunk_internal</a>(
        uploader,
        publisher,
        proposal_id,
        metadata_chunk,
        code_indices,
        code_chunks
    );
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_publish_to_account"></a>

## Function `publish_to_account`

Publisher publishes the staged package to their account


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_publish_to_account">publish_to_account</a>(publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, uploader: <b>address</b>, proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_publish_to_account">publish_to_account</a>(
    publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    uploader: <b>address</b>,
    proposal_id: u64
) <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> proposal_data = <a href="large_packages_v2.md#0x7_large_packages_v2_remove_proposal_as_publisher">remove_proposal_as_publisher</a>(publisher, uploader, proposal_id);
    <b>let</b> (metadata_serialized, <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>) = <a href="large_packages_v2.md#0x7_large_packages_v2_destroy_proposal_data">destroy_proposal_data</a>(proposal_data);
    <a href="../../aptos-framework/doc/code.md#0x1_code_publish_package_txn">code::publish_package_txn</a>(publisher, metadata_serialized, <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>);
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_publish_to_object"></a>

## Function `publish_to_object`

Publisher publishes the staged package to a new object


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_publish_to_object">publish_to_object</a>(publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, uploader: <b>address</b>, proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_publish_to_object">publish_to_object</a>(
    publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    uploader: <b>address</b>,
    proposal_id: u64
) <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> proposal_data = <a href="large_packages_v2.md#0x7_large_packages_v2_remove_proposal_as_publisher">remove_proposal_as_publisher</a>(publisher, uploader, proposal_id);
    <b>let</b> (metadata_serialized, <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>) = <a href="large_packages_v2.md#0x7_large_packages_v2_destroy_proposal_data">destroy_proposal_data</a>(proposal_data);
    <a href="../../aptos-framework/doc/object_code_deployment.md#0x1_object_code_deployment_publish">object_code_deployment::publish</a>(
        publisher,
        metadata_serialized,
        <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>
    );
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_upgrade_object_code"></a>

## Function `upgrade_object_code`

Publisher upgrades an existing object code


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_upgrade_object_code">upgrade_object_code</a>(publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, uploader: <b>address</b>, proposal_id: u64, code_object: <a href="../../aptos-framework/doc/object.md#0x1_object_Object">object::Object</a>&lt;<a href="../../aptos-framework/doc/code.md#0x1_code_PackageRegistry">code::PackageRegistry</a>&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_upgrade_object_code">upgrade_object_code</a>(
    publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    uploader: <b>address</b>,
    proposal_id: u64,
    code_object: Object&lt;PackageRegistry&gt;
) <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> proposal_data = <a href="large_packages_v2.md#0x7_large_packages_v2_remove_proposal_as_publisher">remove_proposal_as_publisher</a>(publisher, uploader, proposal_id);
    <b>let</b> (metadata_serialized, <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>) = <a href="large_packages_v2.md#0x7_large_packages_v2_destroy_proposal_data">destroy_proposal_data</a>(proposal_data);
    <a href="../../aptos-framework/doc/object_code_deployment.md#0x1_object_code_deployment_upgrade">object_code_deployment::upgrade</a>(
        publisher,
        metadata_serialized,
        <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>,
        code_object
    );
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_cleanup_proposal"></a>

## Function `cleanup_proposal`

Cancel and cleanup a proposal (can be called by uploader or publisher)


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_cleanup_proposal">cleanup_proposal</a>(caller: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, uploader: <b>address</b>, publisher: <b>address</b>, proposal_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_cleanup_proposal">cleanup_proposal</a>(
    caller: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    uploader: <b>address</b>,
    publisher: <b>address</b>,
    proposal_id: u64
) <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> caller_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(caller);
    <b>assert</b>!(
        caller_addr == uploader || caller_addr == publisher,
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="large_packages_v2.md#0x7_large_packages_v2_ENOT_AUTHORIZED_CLEANUP">ENOT_AUTHORIZED_CLEANUP</a>)
    );

    <b>let</b> staging_area = &<b>mut</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader, publisher, proposal_id };

    <b>if</b> (staging_area.proposals.contains(key)) {
        <b>let</b> proposal_data = staging_area.proposals.remove(key);
        <a href="large_packages_v2.md#0x7_large_packages_v2_destroy_proposal_data">destroy_proposal_data</a>(proposal_data);
    };
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_stage_code_chunk_internal"></a>

## Function `stage_code_chunk_internal`

Internal function to stage code chunks


<pre><code><b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_stage_code_chunk_internal">stage_code_chunk_internal</a>(uploader: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, publisher: <b>address</b>, proposal_id: u64, metadata_chunk: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, code_indices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;, code_chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_stage_code_chunk_internal">stage_code_chunk_internal</a>(
    uploader: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    publisher: <b>address</b>,
    proposal_id: u64,
    metadata_chunk: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    code_indices: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u16&gt;,
    code_chunks: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
) {
    <b>assert</b>!(
        code_indices.length() == code_chunks.length(),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_invalid_argument">error::invalid_argument</a>(<a href="large_packages_v2.md#0x7_large_packages_v2_ECODE_MISMATCH">ECODE_MISMATCH</a>)
    );

    <b>let</b> uploader_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(uploader);
    <b>let</b> staging_area = &<b>mut</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader: uploader_addr, publisher, proposal_id };

    // Create new proposal <b>if</b> it doesn't exist
    <b>if</b> (!staging_area.proposals.contains(key)) {
        staging_area.proposals.add(key, <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">ProposalData</a> {
            metadata_serialized: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>: <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        });
    };

    <b>let</b> proposal_data = staging_area.proposals.borrow_mut(key);

    // Append metadata <b>if</b> provided
    <b>if</b> (!metadata_chunk.is_empty()) {
        proposal_data.metadata_serialized.append(metadata_chunk);
    };

    // Add or append <a href="../../aptos-framework/doc/code.md#0x1_code">code</a> chunks
    <b>let</b> code_chunks_len = code_chunks.length();
    for (i in 0..code_chunks_len) {
        <b>let</b> inner_code = code_chunks[i];
        <b>let</b> idx = (code_indices[i] <b>as</b> u64);

        // Ensure the <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a> is large enough
        <b>while</b> (proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>.length() &lt;= idx) {
            proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>.push_back(<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]);
        };

        // Append <b>to</b> the <a href="../../aptos-framework/doc/code.md#0x1_code">code</a> at the given index
        proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>[idx].append(inner_code);
    };
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_remove_proposal_as_publisher"></a>

## Function `remove_proposal_as_publisher`

Remove a proposal as the publisher


<pre><code><b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_remove_proposal_as_publisher">remove_proposal_as_publisher</a>(publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, uploader: <b>address</b>, proposal_id: u64): <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">large_packages_v2::ProposalData</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_remove_proposal_as_publisher">remove_proposal_as_publisher</a>(
    publisher: &<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    uploader: <b>address</b>,
    proposal_id: u64
): <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">ProposalData</a> {
    <b>let</b> publisher_addr = <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(publisher);
    <b>let</b> staging_area = &<b>mut</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader, publisher: publisher_addr, proposal_id };

    <b>assert</b>!(
        staging_area.proposals.contains(key),
        <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="large_packages_v2.md#0x7_large_packages_v2_EPROPOSAL_NOT_FOUND">EPROPOSAL_NOT_FOUND</a>)
    );

    staging_area.proposals.remove(key)
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_assemble_module_code"></a>

## Function `assemble_module_code`

Assemble the module code from chunks


<pre><code><b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_assemble_module_code">assemble_module_code</a>(proposal_data: &<a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">large_packages_v2::ProposalData</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_assemble_module_code">assemble_module_code</a>(proposal_data: &<a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">ProposalData</a>): <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_destroy_proposal_data"></a>

## Function `destroy_proposal_data`

Destroy proposal data and clean up resources


<pre><code><b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_destroy_proposal_data">destroy_proposal_data</a>(proposal_data: <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">large_packages_v2::ProposalData</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_destroy_proposal_data">destroy_proposal_data</a>(proposal_data: <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">ProposalData</a>): (<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-framework/../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;) {
    <b>let</b> <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalData">ProposalData</a> {
        metadata_serialized,
        <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>,
    } = proposal_data;
    (metadata_serialized, <a href="../../aptos-framework/doc/code.md#0x1_code">code</a>)
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_proposal_exists"></a>

## Function `proposal_exists`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_proposal_exists">proposal_exists</a>(uploader: <b>address</b>, publisher: <b>address</b>, proposal_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_proposal_exists">proposal_exists</a>(
    uploader: <b>address</b>,
    publisher: <b>address</b>,
    proposal_id: u64
): bool <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> staging_area = &<a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader, publisher, proposal_id };
    staging_area.proposals.contains(key)
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_get_proposal_metadata_size"></a>

## Function `get_proposal_metadata_size`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_metadata_size">get_proposal_metadata_size</a>(uploader: <b>address</b>, publisher: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_metadata_size">get_proposal_metadata_size</a>(
    uploader: <b>address</b>,
    publisher: <b>address</b>,
    proposal_id: u64
): u64 <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> staging_area = &<a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader, publisher, proposal_id };

    <b>if</b> (staging_area.proposals.contains(key)) {
        <b>let</b> proposal_data = staging_area.proposals.borrow(key);
        proposal_data.metadata_serialized.length()
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_get_proposal_module_count"></a>

## Function `get_proposal_module_count`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_module_count">get_proposal_module_count</a>(uploader: <b>address</b>, publisher: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_module_count">get_proposal_module_count</a>(
    uploader: <b>address</b>,
    publisher: <b>address</b>,
    proposal_id: u64
): u64 <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> staging_area = &<a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader, publisher, proposal_id };

    <b>if</b> (staging_area.proposals.contains(key)) {
        <b>let</b> proposal_data = staging_area.proposals.borrow(key);
        proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>.length() + 1
    } <b>else</b> {
        0
    }
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_get_proposal_module_size"></a>

## Function `get_proposal_module_size`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_module_size">get_proposal_module_size</a>(uploader: <b>address</b>, publisher: <b>address</b>, proposal_id: u64, module_index: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_module_size">get_proposal_module_size</a>(
    uploader: <b>address</b>,
    publisher: <b>address</b>,
    proposal_id: u64,
    module_index: u64
): u64 <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> staging_area = &<a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader, publisher, proposal_id };

    <b>if</b> (!staging_area.proposals.contains(key)) {
        <b>return</b> 0
    };

    <b>let</b> proposal_data = staging_area.proposals.borrow(key);
    <b>if</b> (module_index &gt;= proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>.length()) {
        <b>return</b> 0
    };

    proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>[module_index].length()
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_get_proposal_total_code_size"></a>

## Function `get_proposal_total_code_size`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_total_code_size">get_proposal_total_code_size</a>(uploader: <b>address</b>, publisher: <b>address</b>, proposal_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_total_code_size">get_proposal_total_code_size</a>(
    uploader: <b>address</b>,
    publisher: <b>address</b>,
    proposal_id: u64
): u64 <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> staging_area = &<a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a>[@aptos_experimental];
    <b>let</b> key = <a href="large_packages_v2.md#0x7_large_packages_v2_ProposalKey">ProposalKey</a> { uploader, publisher, proposal_id };

    <b>if</b> (!staging_area.proposals.contains(key)) {
        <b>return</b> 0
    };

    <b>let</b> proposal_data = staging_area.proposals.borrow(key);
    <b>let</b> total_size = 0u64;
    <b>let</b> code_len = proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>.length();

    for (i in 0..code_len) {
        total_size += proposal_data.<a href="../../aptos-framework/doc/code.md#0x1_code">code</a>[i].length();
    };

    total_size
}
</code></pre>



</details>

<a id="0x7_large_packages_v2_get_proposal_summary"></a>

## Function `get_proposal_summary`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_summary">get_proposal_summary</a>(uploader: <b>address</b>, publisher: <b>address</b>, proposal_id: u64): (bool, u64, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_summary">get_proposal_summary</a>(
    uploader: <b>address</b>,
    publisher: <b>address</b>,
    proposal_id: u64
): (bool, u64, u64, u64) <b>acquires</b> <a href="large_packages_v2.md#0x7_large_packages_v2_StagingArea">StagingArea</a> {
    <b>let</b> <b>exists</b> = <a href="large_packages_v2.md#0x7_large_packages_v2_proposal_exists">proposal_exists</a>(uploader, publisher, proposal_id);
    <b>if</b> (!<b>exists</b>) {
        <b>return</b> (<b>false</b>, 0, 0, 0)
    };

    <b>let</b> metadata_size = <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_metadata_size">get_proposal_metadata_size</a>(uploader, publisher, proposal_id);
    <b>let</b> module_count = <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_module_count">get_proposal_module_count</a>(uploader, publisher, proposal_id);
    <b>let</b> total_code_size = <a href="large_packages_v2.md#0x7_large_packages_v2_get_proposal_total_code_size">get_proposal_total_code_size</a>(uploader, publisher, proposal_id);

    (<b>exists</b>, metadata_size, module_count, total_code_size)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
