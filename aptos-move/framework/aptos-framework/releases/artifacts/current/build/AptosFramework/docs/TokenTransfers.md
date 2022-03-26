
<a name="0x1_TokenTransfers"></a>

# Module `0x1::TokenTransfers`

This module provides the foundation for transferring of Tokens


-  [Resource `TokenTransfers`](#0x1_TokenTransfers_TokenTransfers)
-  [Function `initialize_token_transfers`](#0x1_TokenTransfers_initialize_token_transfers)
-  [Function `transfer_to`](#0x1_TokenTransfers_transfer_to)
-  [Function `receive_from`](#0x1_TokenTransfers_receive_from)
-  [Function `stop_transfer_to`](#0x1_TokenTransfers_stop_transfer_to)
-  [Function `create_token`](#0x1_TokenTransfers_create_token)


<pre><code><b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII">0x1::ASCII</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID">0x1::GUID</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option">0x1::Option</a>;
<b>use</b> <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer">0x1::Signer</a>;
<b>use</b> <a href="Table.md#0x1_Table">0x1::Table</a>;
<b>use</b> <a href="Token.md#0x1_Token">0x1::Token</a>;
</code></pre>



<a name="0x1_TokenTransfers_TokenTransfers"></a>

## Resource `TokenTransfers`



<pre><code><b>struct</b> <a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a>&lt;TokenType: <b>copy</b>, drop, store&gt; <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>pending_transfers: <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<b>address</b>, <a href="Table.md#0x1_Table_Table">Table::Table</a>&lt;<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, <a href="Token.md#0x1_Token_Token">Token::Token</a>&lt;TokenType&gt;&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_TokenTransfers_initialize_token_transfers"></a>

## Function `initialize_token_transfers`



<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_initialize_token_transfers">initialize_token_transfers</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(account: &signer)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_initialize_token_transfers">initialize_token_transfers</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(account: &signer) {
    <b>move_to</b>(
        account,
        <a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a>&lt;TokenType&gt; {
            pending_transfers: <a href="Table.md#0x1_Table_create">Table::create</a>&lt;<b>address</b>, <a href="Table.md#0x1_Table">Table</a>&lt;ID, <a href="Token.md#0x1_Token">Token</a>&lt;TokenType&gt;&gt;&gt;(),
        }
    )
}
</code></pre>



</details>

<a name="0x1_TokenTransfers_transfer_to"></a>

## Function `transfer_to`



<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_transfer_to">transfer_to</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(sender: &signer, receiver: <b>address</b>, token_id: &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>, amount: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_transfer_to">transfer_to</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    sender: &signer,
    receiver: <b>address</b>,
    token_id: &ID,
    amount: u64,
) <b>acquires</b> <a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a> {
    <b>let</b> sender_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(sender);
    <b>let</b> pending_transfers =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a>&lt;TokenType&gt;&gt;(sender_addr).pending_transfers;
    <b>if</b> (!<a href="Table.md#0x1_Table_contains_key">Table::contains_key</a>(pending_transfers, &receiver)) {
        <a href="Table.md#0x1_Table_insert">Table::insert</a>(pending_transfers, receiver, <a href="Table.md#0x1_Table_create">Table::create</a>())
    };
    <b>let</b> addr_pending_transfers = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(pending_transfers, &receiver);

    <b>let</b> token = <a href="Token.md#0x1_Token_withdraw_token">Token::withdraw_token</a>&lt;TokenType&gt;(sender, token_id, amount);
    <b>let</b> token_id = <a href="Token.md#0x1_Token_token_id">Token::token_id</a>(&token);
    <b>if</b> (<a href="Table.md#0x1_Table_contains_key">Table::contains_key</a>(addr_pending_transfers, token_id)) {
        <b>let</b> dst_token = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(addr_pending_transfers, token_id);
        <a href="Token.md#0x1_Token_merge_token">Token::merge_token</a>(token, dst_token)
    } <b>else</b> {
        <a href="Table.md#0x1_Table_insert">Table::insert</a>(addr_pending_transfers, *token_id, token)
    }
}
</code></pre>



</details>

<a name="0x1_TokenTransfers_receive_from"></a>

## Function `receive_from`



<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_receive_from">receive_from</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(receiver: &signer, sender: <b>address</b>, token_id: &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_receive_from">receive_from</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    receiver: &signer,
    sender: <b>address</b>,
    token_id: &ID,
) <b>acquires</b> <a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a> {
    <b>let</b> receiver_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(receiver);
    <b>let</b> pending_transfers =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a>&lt;TokenType&gt;&gt;(sender).pending_transfers;
    <b>let</b> pending_tokens = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(pending_transfers, &receiver_addr);
    <b>let</b> (_id, token) = <a href="Table.md#0x1_Table_remove">Table::remove</a>(pending_tokens, token_id);

    <b>if</b> (<a href="Table.md#0x1_Table_count">Table::count</a>(pending_tokens) == 0) {
        <b>let</b> (_id, real_pending_transfers) = <a href="Table.md#0x1_Table_remove">Table::remove</a>(pending_transfers, &receiver_addr);
        <a href="Table.md#0x1_Table_destroy_empty">Table::destroy_empty</a>(real_pending_transfers)
    };

    <a href="Token.md#0x1_Token_deposit_token">Token::deposit_token</a>(receiver, token)
}
</code></pre>



</details>

<a name="0x1_TokenTransfers_stop_transfer_to"></a>

## Function `stop_transfer_to`



<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_stop_transfer_to">stop_transfer_to</a>&lt;TokenType: <b>copy</b>, drop, store&gt;(sender: &signer, receiver: <b>address</b>, token_id: &<a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_stop_transfer_to">stop_transfer_to</a>&lt;TokenType: <b>copy</b> + drop + store&gt;(
    sender: &signer,
    receiver: <b>address</b>,
    token_id: &ID,
) <b>acquires</b> <a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a> {
    <b>let</b> sender_addr = <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Signer.md#0x1_Signer_address_of">Signer::address_of</a>(sender);
    <b>let</b> pending_transfers =
        &<b>mut</b> <b>borrow_global_mut</b>&lt;<a href="TokenTransfers.md#0x1_TokenTransfers">TokenTransfers</a>&lt;TokenType&gt;&gt;(sender_addr).pending_transfers;
    <b>let</b> pending_tokens = <a href="Table.md#0x1_Table_borrow_mut">Table::borrow_mut</a>(pending_transfers, &receiver);
    <b>let</b> (_id, token) = <a href="Table.md#0x1_Table_remove">Table::remove</a>(pending_tokens, token_id);

    <b>if</b> (<a href="Table.md#0x1_Table_count">Table::count</a>(pending_tokens) == 0) {
        <b>let</b> (_id, real_pending_transfers) = <a href="Table.md#0x1_Table_remove">Table::remove</a>(pending_transfers, &receiver);
        <a href="Table.md#0x1_Table_destroy_empty">Table::destroy_empty</a>(real_pending_transfers)
    };

    <a href="Token.md#0x1_Token_deposit_token">Token::deposit_token</a>(sender, token)
}
</code></pre>



</details>

<a name="0x1_TokenTransfers_create_token"></a>

## Function `create_token`



<pre><code><b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_create_token">create_token</a>(creator: &signer, amount: u64): <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/GUID.md#0x1_GUID_ID">GUID::ID</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="TokenTransfers.md#0x1_TokenTransfers_create_token">create_token</a>(creator: &signer, amount: u64): ID {
    <b>use</b> Std::ASCII;
    <b>use</b> Std::Option;

    <b>let</b> collection_id = <a href="Token.md#0x1_Token_create_collection">Token::create_collection</a>&lt;u64&gt;(
        creator,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"Collection: Hello, World"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"Hello, World"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"https://aptos.dev"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/Option.md#0x1_Option_none">Option::none</a>(),
    );
    <a href="Token.md#0x1_Token_create_token">Token::create_token</a>&lt;u64&gt;(
        creator,
        *&collection_id,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"<a href="Token.md#0x1_Token">Token</a>: Hello, <a href="Token.md#0x1_Token">Token</a>"),
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"Hello, <a href="Token.md#0x1_Token">Token</a>"),
        amount,
        <a href="../../../../../../../aptos-framework/releases/artifacts/current/build/MoveStdlib/docs/ASCII.md#0x1_ASCII_string">ASCII::string</a>(b"https://aptos.dev"),
        0,
    )
}
</code></pre>



</details>
