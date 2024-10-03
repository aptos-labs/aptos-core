
<a id="0x815_m"></a>

# Module `0x815::m`



-  [Enum Resource `CommonFields`](#0x815_m_CommonFields)
-  [Enum `CommonFieldsVector`](#0x815_m_CommonFieldsVector)
-  [Function `t9_common_field`](#0x815_m_t9_common_field)
-  [Function `test_data_invariant`](#0x815_m_test_data_invariant)
-  [Function `test_match_ref`](#0x815_m_test_match_ref)
-  [Function `test_enum_vector`](#0x815_m_test_enum_vector)


<pre><code></code></pre>



<a id="0x815_m_CommonFields"></a>

## Enum Resource `CommonFields`



<pre><code>enum <a href="enum.md#0x815_m_CommonFields">CommonFields</a> <b>has</b> <b>copy</b>, drop, key
</code></pre>



##### Variants


##### Foo


##### Fields


<dl>
<dt>
<code>x: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>y: u8</code>
</dt>
<dd>

</dd>
</dl>


##### Bar


##### Fields


<dl>
<dt>
<code>x: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>y: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>z: u32</code>
</dt>
<dd>

</dd>
</dl>


##### Specification



<pre><code><b>invariant</b> self.x &gt; 20;
<b>invariant</b> (self is CommonFields::Bar) ==&gt; self.z &gt; 10;
</code></pre>



<a id="0x815_m_CommonFieldsVector"></a>

## Enum `CommonFieldsVector`



<pre><code>enum <a href="enum.md#0x815_m_CommonFieldsVector">CommonFieldsVector</a> <b>has</b> drop
</code></pre>



##### Variants


##### Foo


##### Fields


<dl>
<dt>
<code>x: <a href="">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


##### Bar


##### Fields


<dl>
<dt>
<code>x: <a href="">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>y: <a href="">vector</a>&lt;<a href="enum.md#0x815_m_CommonFields">m::CommonFields</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


<a id="0x815_m_t9_common_field"></a>

## Function `t9_common_field`



<pre><code><b>fun</b> <a href="enum.md#0x815_m_t9_common_field">t9_common_field</a>(): u64
</code></pre>



##### Implementation


<pre><code><b>fun</b> <a href="enum.md#0x815_m_t9_common_field">t9_common_field</a>(): u64 {
    <b>let</b> common = CommonFields::Bar {
        x: 30,
        y: 40,
        z: 50
    };
    common.x = 15; // <b>struct</b> <b>invariant</b> fails
    common.x
}
</code></pre>



<a id="0x815_m_test_data_invariant"></a>

## Function `test_data_invariant`



<pre><code><b>fun</b> <a href="enum.md#0x815_m_test_data_invariant">test_data_invariant</a>()
</code></pre>



##### Implementation


<pre><code><b>fun</b> <a href="enum.md#0x815_m_test_data_invariant">test_data_invariant</a>() {
    <b>let</b> common = CommonFields::Bar {
        x: 30,
        y: 40,
        z: 50
    };
    <b>let</b> CommonFields::Bar {x: _x, y: _y, z} = &<b>mut</b> common;
    *z = 9; // <b>struct</b> <b>invariant</b> fails
}
</code></pre>



<a id="0x815_m_test_match_ref"></a>

## Function `test_match_ref`



<pre><code><b>fun</b> <a href="enum.md#0x815_m_test_match_ref">test_match_ref</a>(): u64
</code></pre>



##### Implementation


<pre><code><b>fun</b> <a href="enum.md#0x815_m_test_match_ref">test_match_ref</a>(): u64 {
    <b>let</b> common = CommonFields::Bar {
        x: 30,
        y: 40,
        z: 50
    };
    match (&common) {
        Foo {x, y: _} =&gt; *x,
        Bar {x, y: _, z: _ } =&gt; *x + 1
    }
}
</code></pre>



##### Specification



<pre><code><b>ensures</b> result == 31;
</code></pre>



<a id="0x815_m_test_enum_vector"></a>

## Function `test_enum_vector`



<pre><code><b>fun</b> <a href="enum.md#0x815_m_test_enum_vector">test_enum_vector</a>()
</code></pre>



##### Implementation


<pre><code><b>fun</b> <a href="enum.md#0x815_m_test_enum_vector">test_enum_vector</a>() {
    <b>let</b> _common_vector_1 = CommonFieldsVector::Foo {
        x: <a href="">vector</a>[2]
    };
    <b>let</b> _common_fields = CommonFields::Bar {
        x: 30,
        y: 40,
        z: 50
    };
    <b>let</b> _common_vector_2 = CommonFieldsVector::Bar {
        x: <a href="">vector</a>[2],
        y: <a href="">vector</a>[_common_fields]
    };
    <b>spec</b> {
        <b>assert</b> _common_vector_1.x != _common_vector_2.x; // this fails
        <b>assert</b> _common_vector_2.y[0] == CommonFields::Bar {
            x: 30,
            y: 40,
            z: 50
        };
    };
    <b>let</b> _common_vector_3 = CommonFieldsVector::Bar {
        x: <a href="">vector</a>[2],
        y: <a href="">vector</a>[_common_fields]
    };
    <b>spec</b> {
        <b>assert</b> _common_vector_2.x == _common_vector_3.x;
        <b>assert</b> _common_vector_2 == _common_vector_3;
    };

}
</code></pre>
