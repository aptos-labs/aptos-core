
<a name="0x1_wasm"></a>

# Module `0x1::wasm`



-  [Struct `WasmProgram`](#0x1_wasm_WasmProgram)
-  [Function `publish_code`](#0x1_wasm_publish_code)
-  [Function `validate_and_annotate_wasm_bytecode`](#0x1_wasm_validate_and_annotate_wasm_bytecode)
-  [Function `execute_bytecode`](#0x1_wasm_execute_bytecode)
-  [Function `execute_code`](#0x1_wasm_execute_code)
-  [Function `execute_code_mutable`](#0x1_wasm_execute_code_mutable)


<pre><code><b>use</b> <a href="table.md#0x1_table">0x1::table</a>;
</code></pre>



<a name="0x1_wasm_WasmProgram"></a>

## Struct `WasmProgram`



<pre><code><b>struct</b> <a href="wasm.md#0x1_wasm_WasmProgram">WasmProgram</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>code: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>globals: <a href="table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a name="0x1_wasm_publish_code"></a>

## Function `publish_code`

Pack a value into the <code>Any</code> representation. Because Any can be stored and dropped, this is
also required from <code>T</code>.


<pre><code><b>public</b> <b>fun</b> <a href="wasm.md#0x1_wasm_publish_code">publish_code</a>(code: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="wasm.md#0x1_wasm_WasmProgram">wasm::WasmProgram</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="wasm.md#0x1_wasm_publish_code">publish_code</a>(code: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="wasm.md#0x1_wasm_WasmProgram">WasmProgram</a> {
    <b>let</b> validated_code = <a href="wasm.md#0x1_wasm_validate_and_annotate_wasm_bytecode">Self::validate_and_annotate_wasm_bytecode</a>(code);
    <a href="wasm.md#0x1_wasm_WasmProgram">WasmProgram</a> {
        code,
        globals: <a href="table.md#0x1_table_new">table::new</a>(),
    }
}
</code></pre>



</details>

<a name="0x1_wasm_validate_and_annotate_wasm_bytecode"></a>

## Function `validate_and_annotate_wasm_bytecode`



<pre><code><b>fun</b> <a href="wasm.md#0x1_wasm_validate_and_annotate_wasm_bytecode">validate_and_annotate_wasm_bytecode</a>(code: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="wasm.md#0x1_wasm_validate_and_annotate_wasm_bytecode">validate_and_annotate_wasm_bytecode</a>(code: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_wasm_execute_bytecode"></a>

## Function `execute_bytecode`



<pre><code><b>fun</b> <a href="wasm.md#0x1_wasm_execute_bytecode">execute_bytecode</a>(program: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, globals: &<a href="table.md#0x1_table_Table">table::Table</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, args: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_mutable: bool): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="wasm.md#0x1_wasm_execute_bytecode">execute_bytecode</a>(program: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, globals: &Table&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, args: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, is_mutable: bool): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>

<a name="0x1_wasm_execute_code"></a>

## Function `execute_code`



<pre><code><b>public</b> <b>fun</b> <a href="wasm.md#0x1_wasm_execute_code">execute_code</a>(program: &<a href="wasm.md#0x1_wasm_WasmProgram">wasm::WasmProgram</a>, args: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="wasm.md#0x1_wasm_execute_code">execute_code</a>(program: &<a href="wasm.md#0x1_wasm_WasmProgram">WasmProgram</a>, args: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="wasm.md#0x1_wasm_execute_bytecode">Self::execute_bytecode</a>(&program.code, &program.globals, args, <b>false</b>)
}
</code></pre>



</details>

<a name="0x1_wasm_execute_code_mutable"></a>

## Function `execute_code_mutable`



<pre><code><b>public</b> <b>fun</b> <a href="wasm.md#0x1_wasm_execute_code_mutable">execute_code_mutable</a>(program: &<b>mut</b> <a href="wasm.md#0x1_wasm_WasmProgram">wasm::WasmProgram</a>, args: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="wasm.md#0x1_wasm_execute_code_mutable">execute_code_mutable</a>(program: &<b>mut</b> <a href="wasm.md#0x1_wasm_WasmProgram">WasmProgram</a>, args: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="wasm.md#0x1_wasm_execute_bytecode">Self::execute_bytecode</a>(&program.code, &program.globals, args, <b>true</b>)
}
</code></pre>



</details>


[move-book]: https://move-language.github.io/move/introduction.html
