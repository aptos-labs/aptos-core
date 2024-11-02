
<a id="0x1_regex"></a>

# Module `0x1::regex`

A minimum regex implementation. Characters are assumed single-byte.

| Supported grammar | testcase-as-example   |
| ----------------- | --------------------- |
| Bracket list      | <code>bracket_list()</code>      |
| Meta-characters   | <code>meta_characters()</code>   |
| Repeat            | <code>repeat()</code>            |
| Capture group     | <code>capture_group()</code>     |
| OR operation      | <code>or_operator()</code>       |

Implementation notes.
The user-provided regex expression is parsed as a syntax tree (<code><a href="regex.md#0x1_regex_AST">AST</a></code>).
The AST converted to a Thompson NFA (https://en.wikipedia.org/wiki/Thompson%27s_construction).
The NFA is used directly to match the target string.


-  [Struct `Regex`](#0x1_regex_Regex)
-  [Struct `Match`](#0x1_regex_Match)
-  [Struct `AST`](#0x1_regex_AST)
-  [Struct `AstNode`](#0x1_regex_AstNode)
-  [Struct `BracketListParser`](#0x1_regex_BracketListParser)
-  [Struct `NFATransition`](#0x1_regex_NFATransition)
-  [Struct `NFAState`](#0x1_regex_NFAState)
-  [Struct `NFA`](#0x1_regex_NFA)
-  [Struct `MatchSession`](#0x1_regex_MatchSession)
-  [Struct `VisitorState`](#0x1_regex_VisitorState)
-  [Constants](#@Constants_0)
-  [Function `compile`](#0x1_regex_compile)
-  [Function `match`](#0x1_regex_match)
-  [Function `matched_group`](#0x1_regex_matched_group)
-  [Function `ast_from_pattern`](#0x1_regex_ast_from_pattern)
-  [Function `parse_regex`](#0x1_regex_parse_regex)
-  [Function `parse_regex_suffix`](#0x1_regex_parse_regex_suffix)
-  [Function `parse_multichars`](#0x1_regex_parse_multichars)
-  [Function `parse_charset`](#0x1_regex_parse_charset)
-  [Function `parse_quantifier`](#0x1_regex_parse_quantifier)
-  [Function `parse_maybe_range_max`](#0x1_regex_parse_maybe_range_max)
-  [Function `parse_range_max`](#0x1_regex_parse_range_max)
-  [Function `parse_number`](#0x1_regex_parse_number)
-  [Function `parse_char`](#0x1_regex_parse_char)
-  [Function `ast_add_node`](#0x1_regex_ast_add_node)
-  [Function `ast_add_concat_node_smart`](#0x1_regex_ast_add_concat_node_smart)
-  [Function `ast_add_repeat_node_smart`](#0x1_regex_ast_add_repeat_node_smart)
-  [Function `new_bracket_list_parser`](#0x1_regex_new_bracket_list_parser)
-  [Function `bracket_list_parser_update`](#0x1_regex_bracket_list_parser_update)
-  [Function `bracket_list_parser_finish`](#0x1_regex_bracket_list_parser_finish)
-  [Function `nfa_from_ast`](#0x1_regex_nfa_from_ast)
-  [Function `build_nfa_from_ast_node`](#0x1_regex_build_nfa_from_ast_node)
-  [Function `add_empty_state_to_nfa`](#0x1_regex_add_empty_state_to_nfa)
-  [Function `add_epsilon_transition`](#0x1_regex_add_epsilon_transition)
-  [Function `trigger_epsilon_transitions`](#0x1_regex_trigger_epsilon_transitions)
-  [Function `trigger_normal_transitions`](#0x1_regex_trigger_normal_transitions)
-  [Function `update_group_times_on_new_arrival`](#0x1_regex_update_group_times_on_new_arrival)
-  [Function `get_charset`](#0x1_regex_get_charset)
-  [Function `char_triggers_transition`](#0x1_regex_char_triggers_transition)
-  [Function `decode_group_action`](#0x1_regex_decode_group_action)
-  [Function `group_action_begin`](#0x1_regex_group_action_begin)
-  [Function `group_action_end`](#0x1_regex_group_action_end)


<pre><code><b>use</b> <a href="../../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_regex_Regex"></a>

## Struct `Regex`

A compiled regex.


<pre><code><b>struct</b> <a href="regex.md#0x1_regex_Regex">Regex</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>nfa: <a href="regex.md#0x1_regex_NFA">regex::NFA</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_Match"></a>

## Struct `Match`

A match result.


<pre><code><b>struct</b> <a href="regex.md#0x1_regex_Match">Match</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>haystack: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>visitor: <a href="regex.md#0x1_regex_VisitorState">regex::VisitorState</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_AST"></a>

## Struct `AST`

A regex expression parsed as a tree.

An LL(1) grammar is used. Each line below is a production rule.
REGEX ::= MULTICHARS REGEX_SUFFIX
REGEX ::= ( REGEX ) GRPQUANT REGEX_SUFFIX
GRPQUANT ::=
GRPQUANT ::= QUANT
REGEX_SUFFIX ::=
REGEX_SUFFIX ::= REGEX
REGEX_SUFFIX ::= | REGEX
MULTICHARS ::= CHARSET MULTICHARS'
MULTICHARS' ::=
MULTICHARS' ::= QUANT
CHARSET ::= CHAR
CHARSET ::= [ CHAR MORE_CHAR ]
MORE_CHAR ::=
MORE_CHAR ::= CHAR MORE_CHAR
CHAR ::= \ CHAR_SUFFIX
CHAR can be any character except the following [ ] ( ) { } + * ? |
CHAR_SUFFIX can be any character.
QUANT ::= ?
QUANT ::= +
QUANT ::= *
QUANT ::= { NUM MAYBE_RANGE_MAX }
MAYBE_RANGE_MAX ::=
MAYBE_RANGE_MAX ::= , RANGE_MAX
RANGE_MAX ::=
RANGE_MAX ::= NUM
NUM ::= 0 NUM'
NUM ::= 1 NUM'
NUM ::= 2 NUM'
NUM' ::=
NUM' ::= NUM

Useful tool: https://www.cs.princeton.edu/courses/archive/spring20/cos320/LL1/


<pre><code><b>struct</b> <a href="regex.md#0x1_regex_AST">AST</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>root: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>nodes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="regex.md#0x1_regex_AstNode">regex::AstNode</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>num_capture_groups: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_AstNode"></a>

## Struct `AstNode`



<pre><code><b>struct</b> <a href="regex.md#0x1_regex_AstNode">AstNode</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>idx: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>type: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>group_idx: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>charset: u256</code>
</dt>
<dd>

</dd>
<dt>
<code>repeat_min: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>repeat_max: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>child_0: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>child_1: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_BracketListParser"></a>

## Struct `BracketListParser`

Helps compile a bracket list in regex (e.g. <code>[a-z\d_-]</code>) to a character set it matches.


<pre><code><b>struct</b> <a href="regex.md#0x1_regex_BracketListParser">BracketListParser</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>range_state: u8</code>
</dt>
<dd>
 0: nothing like a range
 1: saw a single char to start a range (stored in <code>range_start</code>), expecting a -
 2: saw a range start and -, expecting a single char to finish the range
</dd>
<dt>
<code>range_start: u8</code>
</dt>
<dd>

</dd>
<dt>
<code>accumulated_charset: u256</code>
</dt>
<dd>

</dd>
<dt>
<code>is_negated_set: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>num_chars: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_NFATransition"></a>

## Struct `NFATransition`



<pre><code><b>struct</b> <a href="regex.md#0x1_regex_NFATransition">NFATransition</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>activation: u256</code>
</dt>
<dd>

</dd>
<dt>
<code><b>to</b>: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_NFAState"></a>

## Struct `NFAState`



<pre><code><b>struct</b> <a href="regex.md#0x1_regex_NFAState">NFAState</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>idx: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>group_actions: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>epsilon_transitions: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>normal_transitions: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="regex.md#0x1_regex_NFATransition">regex::NFATransition</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_NFA"></a>

## Struct `NFA`

A [Thompson NFA](https://en.wikipedia.org/wiki/Thompson%27s_construction).


<pre><code><b>struct</b> <a href="regex.md#0x1_regex_NFA">NFA</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>states: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="regex.md#0x1_regex_NFAState">regex::NFAState</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>start: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>end: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>num_groups: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_MatchSession"></a>

## Struct `MatchSession`



<pre><code><b>struct</b> <a href="regex.md#0x1_regex_MatchSession">MatchSession</a> <b>has</b> drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>cur_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>visitors: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="regex.md#0x1_regex_VisitorState">regex::VisitorState</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>last_visit_times: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_regex_VisitorState"></a>

## Struct `VisitorState`



<pre><code><b>struct</b> <a href="regex.md#0x1_regex_VisitorState">VisitorState</a> <b>has</b> <b>copy</b>, drop
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>state_idx: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>group_start_times: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>group_end_times: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_regex_AST_NODE_TYPE__CAPTURE"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_AST_NODE_TYPE__CAPTURE">AST_NODE_TYPE__CAPTURE</a>: u64 = 4;
</code></pre>



<a id="0x1_regex_AST_NODE_TYPE__CHARMATCH"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_AST_NODE_TYPE__CHARMATCH">AST_NODE_TYPE__CHARMATCH</a>: u64 = 6;
</code></pre>



<a id="0x1_regex_AST_NODE_TYPE__CONCAT"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_AST_NODE_TYPE__CONCAT">AST_NODE_TYPE__CONCAT</a>: u64 = 1;
</code></pre>



<a id="0x1_regex_AST_NODE_TYPE__EPSILON"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_AST_NODE_TYPE__EPSILON">AST_NODE_TYPE__EPSILON</a>: u64 = 3;
</code></pre>



<a id="0x1_regex_AST_NODE_TYPE__OR"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_AST_NODE_TYPE__OR">AST_NODE_TYPE__OR</a>: u64 = 2;
</code></pre>



<a id="0x1_regex_AST_NODE_TYPE__REPEAT"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_AST_NODE_TYPE__REPEAT">AST_NODE_TYPE__REPEAT</a>: u64 = 5;
</code></pre>



<a id="0x1_regex_GROUP_ACTION_MASK"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_GROUP_ACTION_MASK">GROUP_ACTION_MASK</a>: u64 = 9223372036854775808;
</code></pre>



<a id="0x1_regex_INF"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_INF">INF</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_regex_NULL"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_NULL">NULL</a>: u64 = 18446744073709551615;
</code></pre>



<a id="0x1_regex_TIME_ZERO"></a>



<pre><code><b>const</b> <a href="regex.md#0x1_regex_TIME_ZERO">TIME_ZERO</a>: u64 = 9223372036854775808;
</code></pre>



<a id="0x1_regex_compile"></a>

## Function `compile`

Try compile a given regex expression.


<pre><code><b>public</b> <b>fun</b> <a href="regex.md#0x1_regex_compile">compile</a>(pattern: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="regex.md#0x1_regex_Regex">regex::Regex</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="regex.md#0x1_regex_compile">compile</a>(pattern: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="regex.md#0x1_regex_Regex">Regex</a>&gt; {
    <b>let</b> maybe_ast = <a href="regex.md#0x1_regex_ast_from_pattern">ast_from_pattern</a>(pattern);
    <b>if</b> (<a href="../../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&maybe_ast)) <b>return</b> <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    <b>let</b> ast = <a href="../../move-stdlib/doc/option.md#0x1_option_extract">option::extract</a>(&<b>mut</b> maybe_ast);
    <b>let</b> nfa = <a href="regex.md#0x1_regex_nfa_from_ast">nfa_from_ast</a>(ast);
    <b>let</b> <a href="regex.md#0x1_regex">regex</a> = <a href="regex.md#0x1_regex_Regex">Regex</a> {  nfa };
    <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(<a href="regex.md#0x1_regex">regex</a>)
}
</code></pre>



</details>

<a id="0x1_regex_match"></a>

## Function `match`

Search a given string for a compiled regex.


<pre><code><b>public</b> <b>fun</b> <a href="regex.md#0x1_regex_match">match</a>(<a href="regex.md#0x1_regex">regex</a>: &<a href="regex.md#0x1_regex_Regex">regex::Regex</a>, s: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="regex.md#0x1_regex_Match">regex::Match</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="regex.md#0x1_regex_match">match</a>(<a href="regex.md#0x1_regex">regex</a>: &<a href="regex.md#0x1_regex_Regex">Regex</a>, s: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="regex.md#0x1_regex_Match">Match</a>&gt; {
    <b>let</b> last_visit_times = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> i = 0;
    <b>let</b> num_nfa_states = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&<a href="regex.md#0x1_regex">regex</a>.nfa.states);
    <b>while</b> (i &lt; num_nfa_states) {
        <b>let</b> visit_time = <b>if</b> (i == <a href="regex.md#0x1_regex">regex</a>.nfa.start) { <a href="regex.md#0x1_regex_TIME_ZERO">TIME_ZERO</a> } <b>else</b> { 0 };
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> last_visit_times, visit_time);
        i = i + 1;
    };
    <b>let</b> init_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&<a href="regex.md#0x1_regex">regex</a>.nfa.states, <a href="regex.md#0x1_regex">regex</a>.nfa.start);

    <b>let</b> all_zeros = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <b>let</b> k = <a href="regex.md#0x1_regex">regex</a>.nfa.num_groups;
    <b>while</b> (k &gt; 0) {
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> all_zeros, 0);
        k = k - 1;
    };

    <b>let</b> initial_visitor = <a href="regex.md#0x1_regex_VisitorState">VisitorState</a> {
        state_idx: <a href="regex.md#0x1_regex">regex</a>.nfa.start,
        group_start_times: all_zeros,
        group_end_times: all_zeros,
    };
    <a href="regex.md#0x1_regex_update_group_times_on_new_arrival">update_group_times_on_new_arrival</a>(&<b>mut</b> initial_visitor, init_state, <a href="regex.md#0x1_regex_TIME_ZERO">TIME_ZERO</a>);

    <b>let</b> session = <a href="regex.md#0x1_regex_MatchSession">MatchSession</a> {
        cur_time: <a href="regex.md#0x1_regex_TIME_ZERO">TIME_ZERO</a>,
        visitors: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[initial_visitor],
        last_visit_times,
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each">vector::for_each</a>(s, |char|{
        <a href="regex.md#0x1_regex_trigger_epsilon_transitions">trigger_epsilon_transitions</a>(&<a href="regex.md#0x1_regex">regex</a>.nfa, &<b>mut</b> session);
        <a href="regex.md#0x1_regex_trigger_normal_transitions">trigger_normal_transitions</a>(&<a href="regex.md#0x1_regex">regex</a>.nfa, &<b>mut</b> session, char);
    });
    <a href="regex.md#0x1_regex_trigger_epsilon_transitions">trigger_epsilon_transitions</a>(&<a href="regex.md#0x1_regex">regex</a>.nfa, &<b>mut</b> session);
    <b>let</b> <a href="regex.md#0x1_regex_MatchSession">MatchSession</a> { cur_time: _cur_time, visitors, last_visit_times: _last_visit_times } = session;
    <b>let</b> (visitor_found, visitor_idx) = <a href="../../move-stdlib/doc/vector.md#0x1_vector_find">vector::find</a>(&visitors, |visitor|{
        <b>let</b> visitor: &<a href="regex.md#0x1_regex_VisitorState">VisitorState</a> = visitor;
        visitor.state_idx == <a href="regex.md#0x1_regex">regex</a>.nfa.end
    });
    <b>if</b> (visitor_found) {
        <b>let</b> visitor = <a href="../../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> visitors, visitor_idx);
        <b>let</b> match = <a href="regex.md#0x1_regex_Match">Match</a> {
            haystack: s,
            visitor,
        };
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(match)
    } <b>else</b> {
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_regex_matched_group"></a>

## Function `matched_group`

Return <code>(captured, begin, end)</code> where,
if the given group was captured at least once during the match,
<code>captured</code> will be <code><b>true</b></code> and <code>[begin, end)</code> will be the position of the last capture;
otherwise, <code>captured</code> will be false and <code>begin, end</code> should be ignored.

See test case <code>capture_group()</code> for detailed examples.


<pre><code><b>public</b> <b>fun</b> <a href="regex.md#0x1_regex_matched_group">matched_group</a>(m: &<a href="regex.md#0x1_regex_Match">regex::Match</a>, group_idx: u64): (bool, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="regex.md#0x1_regex_matched_group">matched_group</a>(m: &<a href="regex.md#0x1_regex_Match">Match</a>, group_idx: u64): (bool, u64, u64) {
    <b>if</b> (group_idx == 0) {
        (<b>true</b>, 0, <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&m.haystack))
    } <b>else</b> <b>if</b> (group_idx &gt; <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&m.visitor.group_start_times)) {
        (<b>false</b>, 0, 0)
    } <b>else</b> {
        <b>let</b> group_start_time = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&m.visitor.group_start_times, group_idx-1);
        <b>let</b> group_end_time = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&m.visitor.group_end_times, group_idx-1);
        <b>if</b> (group_end_time == 0) {
            (<b>false</b>, 0, 0)
        } <b>else</b> {
            (<b>true</b>, group_start_time - <a href="regex.md#0x1_regex_TIME_ZERO">TIME_ZERO</a>, group_end_time - <a href="regex.md#0x1_regex_TIME_ZERO">TIME_ZERO</a>)
        }
    }
}
</code></pre>



</details>

<a id="0x1_regex_ast_from_pattern"></a>

## Function `ast_from_pattern`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_from_pattern">ast_from_pattern</a>(pattern: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="regex.md#0x1_regex_AST">regex::AST</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_from_pattern">ast_from_pattern</a>(pattern: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Option&lt;<a href="regex.md#0x1_regex_AST">AST</a>&gt; {
    <b>let</b> ast = <a href="regex.md#0x1_regex_AST">AST</a> {
        root: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        nodes: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        num_capture_groups: 0,
    };
    <b>let</b> n = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&pattern);
    <b>let</b> cursor = 0;
    <b>let</b> num_capture_groups = 0;
    <b>let</b> (parsed, sub_root_idx) = <a href="regex.md#0x1_regex_parse_regex">parse_regex</a>(&pattern, n, &<b>mut</b> cursor, &<b>mut</b> ast, &<b>mut</b> num_capture_groups);
    <b>if</b> (parsed && cursor == n) {
        ast.root = sub_root_idx;
        ast.num_capture_groups = num_capture_groups;
        <a href="../../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(ast)
    } <b>else</b> {
        <a href="../../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_regex_parse_regex"></a>

## Function `parse_regex`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_regex">parse_regex</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64, ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">regex::AST</a>, num_groups: &<b>mut</b> u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_regex">parse_regex</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64, ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">AST</a>, num_groups: &<b>mut</b> u64): (
    bool, // parsed?
    u64, // <b>if</b> parsed, the index of the representing <a href="regex.md#0x1_regex_AST">AST</a> node?
)  {
    <b>if</b> (*cur &gt;= end) <b>return</b> (<b>false</b>, 0);
    <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
    <b>let</b> (sub_node_0_idx, sub_node_1_idx) = <b>if</b> (token == 40) { // (
        <b>let</b> cur_group_idx = *num_groups;
        *num_groups = *num_groups + 1;
        *cur = *cur + 1;
        <b>let</b> (sub_parsed_0, sub_node_0) = <a href="regex.md#0x1_regex_parse_regex">parse_regex</a>(tokens, end, cur, ast, num_groups);
        <b>if</b> (!sub_parsed_0) <b>return</b> (<b>false</b>, 0);
        <b>if</b> (*cur &gt;= end) <b>return</b> (<b>false</b>, 0);
        <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
        <b>if</b> (token != 41) <b>return</b> (<b>false</b>, 0);   // )
        *cur = *cur + 1;
        <b>let</b> cur_token = <b>if</b> (*cur &lt; end) { *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur) } <b>else</b> { 0 };
        <b>let</b> <b>min</b> = 1;
        <b>let</b> max = 1;
        <b>if</b> (cur_token == 42 || cur_token == 43 || cur_token == 63 || cur_token == 123) { // * + ? {
            <b>let</b> (sub_parsed, sub_min, sub_max) = <a href="regex.md#0x1_regex_parse_quantifier">parse_quantifier</a>(tokens, end, cur);
            <b>if</b> (!sub_parsed) <b>return</b> (<b>false</b>, 0);
            <b>min</b> = sub_min;
            max = sub_max
        };

        <b>let</b> (sub_parsed_1, sub_node_1_idx) = <a href="regex.md#0x1_regex_parse_regex_suffix">parse_regex_suffix</a>(tokens, end, cur, ast, num_groups);
        <b>if</b> (!sub_parsed_1) <b>return</b> (<b>false</b>, 0);
        <b>let</b> cap_node = <a href="regex.md#0x1_regex_AstNode">AstNode</a> {
            idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            type: <a href="regex.md#0x1_regex_AST_NODE_TYPE__CAPTURE">AST_NODE_TYPE__CAPTURE</a>,
            group_idx: cur_group_idx,
            charset: 0,
            repeat_min: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            repeat_max: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            child_0: sub_node_0,
            child_1: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        };
        <b>let</b> cap_node_idx = <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast, cap_node);
        <b>let</b> repeat_node_idx = <a href="regex.md#0x1_regex_ast_add_repeat_node_smart">ast_add_repeat_node_smart</a>(ast, cap_node_idx, <b>min</b>, max);
        (repeat_node_idx, sub_node_1_idx)
    } <b>else</b> {
        <b>let</b> (parsed, charset, <b>min</b>, max) = <a href="regex.md#0x1_regex_parse_multichars">parse_multichars</a>(tokens, end, cur);
        <b>if</b> (!parsed) <b>return</b> (<b>false</b>, 0);
        <b>let</b> (parsed, sub_node_1_idx) = <a href="regex.md#0x1_regex_parse_regex_suffix">parse_regex_suffix</a>(tokens, end, cur, ast, num_groups);
        <b>if</b> (!parsed) <b>return</b> (<b>false</b>, 0);
        <b>let</b> charmatch_node = <a href="regex.md#0x1_regex_AstNode">AstNode</a> {
            idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            type: <a href="regex.md#0x1_regex_AST_NODE_TYPE__CHARMATCH">AST_NODE_TYPE__CHARMATCH</a>,
            group_idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            charset,
            child_0: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            child_1: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            repeat_max: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            repeat_min: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        };
        <b>let</b> charmatch_node_idx = <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast, charmatch_node);
        <b>let</b> repeat_node_idx = <a href="regex.md#0x1_regex_ast_add_repeat_node_smart">ast_add_repeat_node_smart</a>(ast, charmatch_node_idx, <b>min</b>, max);
        (repeat_node_idx, sub_node_1_idx)
    };
    <b>let</b> sub_node_1 = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&ast.nodes, sub_node_1_idx);
    <b>let</b> (ret0, ret1) = <b>if</b> (sub_node_1.type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__OR">AST_NODE_TYPE__OR</a>) {
        // sub_node_0    ,       OR (sub_node_1)     ===&gt;         OR (sub_node_1)
        //                      /  \                             /  \
        //                     /    \                           /    \
        //                    1a    1b                     CONCAT    1b
        //                                                  /  \
        //                                                 /    \
        //                                         sub_node_0   1a
        //
        <b>let</b> new_node_idx = <a href="regex.md#0x1_regex_ast_add_concat_node_smart">ast_add_concat_node_smart</a>(ast, sub_node_0_idx, sub_node_1.child_0);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> ast.nodes, sub_node_1_idx).child_0 = new_node_idx;
        (<b>true</b>, sub_node_1_idx)
    } <b>else</b> {
        <b>let</b> new_node_idx = <a href="regex.md#0x1_regex_ast_add_concat_node_smart">ast_add_concat_node_smart</a>(ast, sub_node_0_idx, sub_node_1_idx);
        (<b>true</b>, new_node_idx)
    };
    (ret0, ret1)
}
</code></pre>



</details>

<a id="0x1_regex_parse_regex_suffix"></a>

## Function `parse_regex_suffix`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_regex_suffix">parse_regex_suffix</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64, ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">regex::AST</a>, num_groups: &<b>mut</b> u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_regex_suffix">parse_regex_suffix</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end :u64, cur: &<b>mut</b> u64, ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">AST</a>, num_groups: &<b>mut</b> u64): (
    bool, // parsed?
    u64, // <b>if</b> parsed, the index of the representing <a href="regex.md#0x1_regex_AST">AST</a> node?
) {
    <b>let</b> cur_token = <b>if</b> (*cur &lt; end) { *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur) } <b>else</b> { 0 };
    <b>if</b> (*cur &gt;= end || cur_token == 41) { // )
        <b>let</b> epsilon_node = <a href="regex.md#0x1_regex_AstNode">AstNode</a> {
            idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            type: <a href="regex.md#0x1_regex_AST_NODE_TYPE__EPSILON">AST_NODE_TYPE__EPSILON</a>,
            group_idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            charset: 0,
            repeat_min: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            repeat_max: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            child_0: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            child_1: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        };
        <b>let</b> new_node_idx = <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast, epsilon_node);
        (<b>true</b>, new_node_idx)
    } <b>else</b> <b>if</b> (cur_token == 124) { // |
        *cur = *cur + 1;
        <b>let</b> (sub_parsed, sub_node_idx) = <a href="regex.md#0x1_regex_parse_regex">parse_regex</a>(tokens, end, cur, ast, num_groups);
        <b>if</b> (!sub_parsed) <b>return</b> (<b>false</b>, 0);
        <b>let</b> epsilon_node = <a href="regex.md#0x1_regex_AstNode">AstNode</a> {
            idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            type: <a href="regex.md#0x1_regex_AST_NODE_TYPE__EPSILON">AST_NODE_TYPE__EPSILON</a>,
            group_idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            charset: 0,
            repeat_min: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            repeat_max: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            child_0: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            child_1: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        };
        <b>let</b> epsilon_node_idx = <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast, epsilon_node);

        <b>let</b> or_node = <a href="regex.md#0x1_regex_AstNode">AstNode</a> {
            idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            type: <a href="regex.md#0x1_regex_AST_NODE_TYPE__OR">AST_NODE_TYPE__OR</a>,
            group_idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            charset: 0,
            repeat_min: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            repeat_max: <a href="regex.md#0x1_regex_NULL">NULL</a>,
            child_0: epsilon_node_idx,
            child_1: sub_node_idx,
        };
        <b>let</b> or_node_idx = <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast, or_node);
        (<b>true</b>, or_node_idx)
    } <b>else</b> {
        <a href="regex.md#0x1_regex_parse_regex">parse_regex</a>(tokens, end, cur, ast, num_groups)
    }
}
</code></pre>



</details>

<a id="0x1_regex_parse_multichars"></a>

## Function `parse_multichars`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_multichars">parse_multichars</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (bool, u256, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_multichars">parse_multichars</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (
    bool, // parsed?
    u256, // <b>if</b> parsed, the charset?
    u64, // <b>if</b> parsed, the minimum times <b>to</b> repeat?
    u64, // <b>if</b> parsed, the maximum times <b>to</b> repeat?
) {
    <b>let</b> (parsed, charset) = <a href="regex.md#0x1_regex_parse_charset">parse_charset</a>(tokens, end, cur);
    <b>if</b> (!parsed) <b>return</b> (<b>false</b>, 0, 0, 0);
    <b>let</b> <b>min</b> = 1;
    <b>let</b> max = 1;
    <b>let</b> cur_token = <b>if</b> (*cur &lt; end) { *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur) } <b>else</b> { 0 };
    <b>if</b> (cur_token == 42 || cur_token == 43 || cur_token == 63 || cur_token == 123) {
        <b>let</b> (sub_parsed, sub_min, sub_max) = <a href="regex.md#0x1_regex_parse_quantifier">parse_quantifier</a>(tokens, end, cur);
        <b>if</b> (!sub_parsed) <b>return</b> (<b>false</b>, 0, 0, 0);
        <b>min</b> = sub_min;
        max = sub_max;
    };
    (<b>true</b>, charset, <b>min</b>, max)
}
</code></pre>



</details>

<a id="0x1_regex_parse_charset"></a>

## Function `parse_charset`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_charset">parse_charset</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (bool, u256)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_charset">parse_charset</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (
    bool, // parsed?
    u256, // <b>if</b> parsed, the charset?
) {
    <b>if</b> (*cur &gt;= end) <b>return</b> (<b>false</b>, 0);
    <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
    <b>let</b> charset = <b>if</b> (token == 91) { // [
        *cur = *cur + 1;
        <b>let</b> bracket_parser = <a href="regex.md#0x1_regex_new_bracket_list_parser">new_bracket_list_parser</a>();
        <b>while</b> (<b>true</b>) {
            <b>if</b> (*cur &gt;= end) <b>return</b> (<b>false</b>, 0);
            <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
            <b>if</b> (token == 93) {  // ]
                *cur = *cur + 1;
                <b>break</b>
            };
            <b>let</b> (parsed, char, escaped) = <a href="regex.md#0x1_regex_parse_char">parse_char</a>(tokens, end, cur);
            <b>if</b> (!parsed) <b>return</b> (<b>false</b>, 0);
            <b>let</b> bracket_parse_error = <a href="regex.md#0x1_regex_bracket_list_parser_update">bracket_list_parser_update</a>(&<b>mut</b> bracket_parser, char, escaped);
            <b>if</b> (bracket_parse_error) <b>return</b> (<b>false</b>, 0);
        };
        <a href="regex.md#0x1_regex_bracket_list_parser_finish">bracket_list_parser_finish</a>(bracket_parser)
    } <b>else</b> {
        <b>let</b> (parsed, char, escaped) = <a href="regex.md#0x1_regex_parse_char">parse_char</a>(tokens, end, cur);
        <b>if</b> (!parsed) <b>return</b> (<b>false</b>, 0);
        <b>let</b> (_, charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(char, escaped);
        charset
    };
    (<b>true</b>, charset)
}
</code></pre>



</details>

<a id="0x1_regex_parse_quantifier"></a>

## Function `parse_quantifier`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_quantifier">parse_quantifier</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (bool, u64, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_quantifier">parse_quantifier</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (
    bool, // parsed?
    u64, // <b>if</b> parsed, the <b>min</b> (inclusive)?
    u64, // <b>if</b> parsed, the max (inclusive)?
) {
    <b>if</b> (*cur == end) <b>return</b> (<b>false</b>, <a href="regex.md#0x1_regex_NULL">NULL</a>, <a href="regex.md#0x1_regex_NULL">NULL</a>);
    <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
    *cur = *cur + 1;
    <b>let</b> (lo, hi) = <b>if</b> (token == 42) { // *
        (0, <a href="regex.md#0x1_regex_INF">INF</a>)
    } <b>else</b> <b>if</b> (token == 43) { // +
        (1, <a href="regex.md#0x1_regex_INF">INF</a>)
    } <b>else</b> <b>if</b> (token == 63) { // ?
        (0, 1)
    } <b>else</b> <b>if</b> (token == 123) { // {
        <b>let</b> (sub_parsed, range_min) = <a href="regex.md#0x1_regex_parse_number">parse_number</a>(tokens, end, cur);
        <b>if</b> (!sub_parsed) <b>return</b> (<b>false</b>, <a href="regex.md#0x1_regex_NULL">NULL</a>, <a href="regex.md#0x1_regex_NULL">NULL</a>);
        <b>let</b> (sub_parsed, has_range_max, range_max_val) = <a href="regex.md#0x1_regex_parse_maybe_range_max">parse_maybe_range_max</a>(tokens, end, cur);
        <b>if</b> (!sub_parsed) <b>return</b> (<b>false</b>, <a href="regex.md#0x1_regex_NULL">NULL</a>, <a href="regex.md#0x1_regex_NULL">NULL</a>);
        <b>if</b> (*cur &gt;= end) <b>return</b> (<b>false</b>, <a href="regex.md#0x1_regex_NULL">NULL</a>, <a href="regex.md#0x1_regex_NULL">NULL</a>);
        <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur); *cur = *cur + 1;
        <b>if</b> (token != 125) <b>return</b> (<b>false</b>, <a href="regex.md#0x1_regex_NULL">NULL</a>, <a href="regex.md#0x1_regex_NULL">NULL</a>); // }

        <b>if</b> (has_range_max) {
            <b>if</b> (range_max_val &lt; range_min) <b>return</b> (<b>false</b>, <a href="regex.md#0x1_regex_NULL">NULL</a>, <a href="regex.md#0x1_regex_NULL">NULL</a>);
            (range_min, range_max_val)
        } <b>else</b> {
            (range_min, range_min)
        }
    } <b>else</b> {
        <b>return</b> (<b>false</b>, <a href="regex.md#0x1_regex_NULL">NULL</a>, <a href="regex.md#0x1_regex_NULL">NULL</a>)
    };

    (<b>true</b>, lo, hi)
}
</code></pre>



</details>

<a id="0x1_regex_parse_maybe_range_max"></a>

## Function `parse_maybe_range_max`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_maybe_range_max">parse_maybe_range_max</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (bool, bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_maybe_range_max">parse_maybe_range_max</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (
    bool, // parsed?
    bool, // <b>if</b> parsed, is there a `range_max`?
    u64, // <b>if</b> parsed and a `range_max` is present, its value?
) {
    <b>if</b> (*cur &gt;= end) <b>return</b> (<b>false</b>, <b>false</b>, 0);
    <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
    <b>if</b> (token == 125) <b>return</b> (<b>true</b>, <b>false</b>, 0); // }
    <b>if</b> (token != 44) <b>return</b> (<b>false</b>, <b>false</b>, 0); // ,
    *cur = *cur + 1;
    <b>let</b> (sub_parsed, range_max_val) = <a href="regex.md#0x1_regex_parse_range_max">parse_range_max</a>(tokens, end, cur);
    <b>if</b> (!sub_parsed) <b>return</b> (<b>false</b>, <b>false</b>, 0);
    (<b>true</b>, <b>true</b>, range_max_val)
}
</code></pre>



</details>

<a id="0x1_regex_parse_range_max"></a>

## Function `parse_range_max`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_range_max">parse_range_max</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_range_max">parse_range_max</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (
    bool, // parsed?
    u64, // <b>if</b> parsed, the range max value?
) {
    <b>if</b> (*cur &gt;= end) <b>return</b> (<b>false</b>, 0);
    <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
    <b>if</b> (token == 125) <b>return</b> (<b>true</b>, <a href="regex.md#0x1_regex_INF">INF</a>); // }
    <b>let</b> (sub_parsed, range_max) = <a href="regex.md#0x1_regex_parse_number">parse_number</a>(tokens, end, cur);
    <b>if</b> (!sub_parsed) <b>return</b> (<b>false</b>, 0);
    (<b>true</b>, range_max)
}
</code></pre>



</details>

<a id="0x1_regex_parse_number"></a>

## Function `parse_number`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_number">parse_number</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_number">parse_number</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (
    bool, // parsed?
    u64, // <b>if</b> parsed, the parsed number.
) {
    <b>let</b> acc = 0;
    <b>while</b> (<b>true</b>) {
        <b>if</b> (*cur &gt;= end) <b>break</b>;
        <b>let</b> new_char = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur);
        <b>if</b> (new_char &lt; 48 || new_char &gt; 57) <b>break</b>;
        acc = acc * 10 + ((new_char <b>as</b> u256) - 48);
        <b>if</b> (acc &gt; 0xffffffffffffffff) <b>return</b> (<b>false</b>, 0);
        *cur = *cur + 1;
    };
    <b>if</b> (acc == 0) <b>return</b> (<b>false</b>, 0);
    (<b>true</b>, (acc <b>as</b> u64))
}
</code></pre>



</details>

<a id="0x1_regex_parse_char"></a>

## Function `parse_char`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_char">parse_char</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (bool, u8, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_parse_char">parse_char</a>(tokens: &<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, end: u64, cur: &<b>mut</b> u64): (
    bool, // parsed?
    u8, // If parsed, the char value?
    bool, // If parsed, is it escaped?
) {
    <b>if</b> (*cur == end) <b>return</b> (<b>false</b>, 0, <b>false</b>);
    <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur); *cur = *cur + 1;
    <b>if</b> (token == 92) { // \
        <b>if</b> (*cur == end) <b>return</b> (<b>false</b>, 0, <b>false</b>);
        <b>let</b> token = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(tokens, *cur); *cur = *cur + 1;
        (<b>true</b>, token, <b>true</b>)
    } <b>else</b> <b>if</b> (
        token == 43 // +
            || token == 42 // *
            || token == 63 // ?
            || token == 91 // [
            || token == 93 // ]
            || token == 40 // (
            || token == 41 // )
            || token == 123 // {
            || token == 124 // |
            || token == 125 // }
    ) {
        (<b>false</b>, 0, <b>false</b>)
    } <b>else</b> {
        (<b>true</b>, token, <b>false</b>)
    }
}
</code></pre>



</details>

<a id="0x1_regex_ast_add_node"></a>

## Function `ast_add_node`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">regex::AST</a>, node: <a href="regex.md#0x1_regex_AstNode">regex::AstNode</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">AST</a>, node: <a href="regex.md#0x1_regex_AstNode">AstNode</a>): u64 {
    <b>let</b> new_node_idx = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&ast.nodes);
    node.idx = new_node_idx;
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> ast.nodes, node);
    new_node_idx
}
</code></pre>



</details>

<a id="0x1_regex_ast_add_concat_node_smart"></a>

## Function `ast_add_concat_node_smart`

Add a CONCAT node for the given 2 child nodes.
If one of the children is a EPSILON node, return the other and avoid creating a new CONCAT node.
The child nodes must have been added.


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_add_concat_node_smart">ast_add_concat_node_smart</a>(ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">regex::AST</a>, child_0_idx: u64, child_1_idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_add_concat_node_smart">ast_add_concat_node_smart</a>(ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">AST</a>, child_0_idx: u64, child_1_idx: u64): u64 {
    <b>if</b> (<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&ast.nodes, child_0_idx).type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__EPSILON">AST_NODE_TYPE__EPSILON</a>) <b>return</b> child_1_idx;
    <b>if</b> (<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&ast.nodes, child_1_idx).type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__EPSILON">AST_NODE_TYPE__EPSILON</a>) <b>return</b> child_0_idx;
    <b>let</b> new_node = <a href="regex.md#0x1_regex_AstNode">AstNode</a> {
        idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        type: <a href="regex.md#0x1_regex_AST_NODE_TYPE__CONCAT">AST_NODE_TYPE__CONCAT</a>,
        group_idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        charset: 0,
        child_0: child_0_idx,
        child_1: child_1_idx,
        repeat_min: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        repeat_max: <a href="regex.md#0x1_regex_NULL">NULL</a>,
    };
    <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast, new_node)
}
</code></pre>



</details>

<a id="0x1_regex_ast_add_repeat_node_smart"></a>

## Function `ast_add_repeat_node_smart`

Add a REPEAT node for a given child node.
If repeat_min == repeat_max == 1, return the child node and avoid creating a new REPEAT node.
If the child is a EPSILON node, return it directly and avoid creating a new REPEAT node..
The child node must have been added.


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_add_repeat_node_smart">ast_add_repeat_node_smart</a>(ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">regex::AST</a>, child_idx: u64, repeat_min: u64, repeat_max: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_ast_add_repeat_node_smart">ast_add_repeat_node_smart</a>(ast: &<b>mut</b> <a href="regex.md#0x1_regex_AST">AST</a>, child_idx: u64, repeat_min: u64, repeat_max: u64): u64 {
    <b>if</b> (repeat_min == 1 && repeat_max == 1 || <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&ast.nodes, child_idx).type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__EPSILON">AST_NODE_TYPE__EPSILON</a>) <b>return</b> child_idx;
    <b>let</b> repeat_node = <a href="regex.md#0x1_regex_AstNode">AstNode</a> {
        idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        type: <a href="regex.md#0x1_regex_AST_NODE_TYPE__REPEAT">AST_NODE_TYPE__REPEAT</a>,
        group_idx: <a href="regex.md#0x1_regex_NULL">NULL</a>,
        repeat_min,
        repeat_max,
        charset: 0,
        child_0: child_idx,
        child_1: <a href="regex.md#0x1_regex_NULL">NULL</a>,
    };
    <a href="regex.md#0x1_regex_ast_add_node">ast_add_node</a>(ast, repeat_node)
}
</code></pre>



</details>

<a id="0x1_regex_new_bracket_list_parser"></a>

## Function `new_bracket_list_parser`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_new_bracket_list_parser">new_bracket_list_parser</a>(): <a href="regex.md#0x1_regex_BracketListParser">regex::BracketListParser</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_new_bracket_list_parser">new_bracket_list_parser</a>(): <a href="regex.md#0x1_regex_BracketListParser">BracketListParser</a> {
    <a href="regex.md#0x1_regex_BracketListParser">BracketListParser</a> {
        range_start: 0,
        range_state: 0,
        accumulated_charset: 0,
        is_negated_set: <b>false</b>,
        num_chars: 0,
    }
}
</code></pre>



</details>

<a id="0x1_regex_bracket_list_parser_update"></a>

## Function `bracket_list_parser_update`

Feed a char to a <code><a href="regex.md#0x1_regex_BracketListParser">BracketListParser</a></code>.
Return whether invalid bracket list is detected.


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_bracket_list_parser_update">bracket_list_parser_update</a>(parser: &<b>mut</b> <a href="regex.md#0x1_regex_BracketListParser">regex::BracketListParser</a>, new_char_val: u8, new_char_is_escaped: bool): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_bracket_list_parser_update">bracket_list_parser_update</a>(parser: &<b>mut</b> <a href="regex.md#0x1_regex_BracketListParser">BracketListParser</a>, new_char_val: u8, new_char_is_escaped: bool): bool {
    parser.num_chars = parser.num_chars + 1;
    <b>if</b> (parser.num_chars == 1 && new_char_val == 94 && !new_char_is_escaped) { // saw a raw ^ at the beginning of the list
        parser.is_negated_set = <b>true</b>;
        <b>return</b> <b>false</b>
    };
    <b>let</b> (new_char_is_meta, new_charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(new_char_val, new_char_is_escaped);
    <b>if</b> (parser.range_state == 0) {
        <b>if</b> (new_char_is_meta) {
            parser.accumulated_charset = parser.accumulated_charset | new_charset;
            <b>false</b>
        } <b>else</b> {
            parser.range_state = 1;
            parser.range_start = new_char_val;
            <b>false</b>
        }
    } <b>else</b> <b>if</b> (parser.range_state == 1) {
        <b>if</b> (new_char_val == 45 && !new_char_is_escaped) { // a raw hyphen arrived!
            parser.range_state = 2;
            <b>false</b>
        } <b>else</b> <b>if</b> (new_char_is_meta) {
            parser.range_state = 0;
            <b>let</b> (_, range_start_as_charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(parser.range_start, <b>false</b>);
            parser.accumulated_charset = parser.accumulated_charset | range_start_as_charset | new_charset;
            <b>false</b>
        } <b>else</b> {
            <b>let</b> (_, range_start_as_charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(parser.range_start, <b>false</b>);
            parser.accumulated_charset = parser.accumulated_charset | range_start_as_charset;
            parser.range_start = new_char_val;
            <b>false</b>
        }
    } <b>else</b> <b>if</b> (parser.range_state == 2) {
        <b>if</b> (new_char_is_meta) {
            parser.range_state = 0;
            <b>let</b> (_, range_start_as_charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(parser.range_start, <b>false</b>);
            <b>let</b> (_, hyphen_as_charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(45, <b>false</b>);
            parser.accumulated_charset = parser.accumulated_charset | range_start_as_charset | hyphen_as_charset | new_charset;
            <b>false</b>
        } <b>else</b> <b>if</b> (new_char_val &gt;= parser.range_start) { // valid range found!
            <b>let</b> equivalent_charset = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff &gt;&gt; (255 - new_char_val + parser.range_start) &lt;&lt; parser.range_start;
            parser.accumulated_charset = parser.accumulated_charset | equivalent_charset;
            parser.range_state = 0;
            <b>false</b>
        } <b>else</b> { // invalid range!
            <b>true</b>
        }
    } <b>else</b> {
        <b>abort</b>(99999)
    }
}
</code></pre>



</details>

<a id="0x1_regex_bracket_list_parser_finish"></a>

## Function `bracket_list_parser_finish`

Conclude a <code><a href="regex.md#0x1_regex_BracketListParser">BracketListParser</a></code> for the aggregated character set to match.


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_bracket_list_parser_finish">bracket_list_parser_finish</a>(parser: <a href="regex.md#0x1_regex_BracketListParser">regex::BracketListParser</a>): u256
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_bracket_list_parser_finish">bracket_list_parser_finish</a>(parser: <a href="regex.md#0x1_regex_BracketListParser">BracketListParser</a>): u256 {
    <b>let</b> (_, range_start_as_charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(parser.range_start, <b>false</b>);
    <b>let</b> (_, hyphen_as_charset) = <a href="regex.md#0x1_regex_get_charset">get_charset</a>(45, <b>false</b>);
    <b>let</b> unfinished_range_as_charset = <b>if</b> (parser.range_state == 1) {
        range_start_as_charset
    } <b>else</b> <b>if</b> (parser.range_state == 2) {
        range_start_as_charset | hyphen_as_charset
    } <b>else</b> {
        0
    };
    parser.accumulated_charset = parser.accumulated_charset | unfinished_range_as_charset;
    <b>if</b> (parser.is_negated_set) {
        0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff ^ parser.accumulated_charset
    } <b>else</b> {
        parser.accumulated_charset
    }
}
</code></pre>



</details>

<a id="0x1_regex_nfa_from_ast"></a>

## Function `nfa_from_ast`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_nfa_from_ast">nfa_from_ast</a>(ast: <a href="regex.md#0x1_regex_AST">regex::AST</a>): <a href="regex.md#0x1_regex_NFA">regex::NFA</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_nfa_from_ast">nfa_from_ast</a>(ast: <a href="regex.md#0x1_regex_AST">AST</a>): <a href="regex.md#0x1_regex_NFA">NFA</a> {
    <b>let</b> nfa = <a href="regex.md#0x1_regex_NFA">NFA</a> { states: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[], start: <a href="regex.md#0x1_regex_NULL">NULL</a>, end: <a href="regex.md#0x1_regex_NULL">NULL</a>, num_groups: ast.num_capture_groups };
    <b>let</b> start = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(&<b>mut</b> nfa);
    <b>let</b> end = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(&<b>mut</b> nfa);
    <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(&ast, ast.root, &<b>mut</b> nfa, start, end);
    nfa.start = start;
    nfa.end = end;
    nfa
}
</code></pre>



</details>

<a id="0x1_regex_build_nfa_from_ast_node"></a>

## Function `build_nfa_from_ast_node`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast: &<a href="regex.md#0x1_regex_AST">regex::AST</a>, node_id: u64, nfa: &<b>mut</b> <a href="regex.md#0x1_regex_NFA">regex::NFA</a>, start: u64, end: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast: &<a href="regex.md#0x1_regex_AST">AST</a>, node_id: u64, nfa: &<b>mut</b> <a href="regex.md#0x1_regex_NFA">NFA</a>, start: u64, end: u64) {
    <b>let</b> node = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&ast.nodes, node_id);
    <b>if</b> (node.type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__CAPTURE">AST_NODE_TYPE__CAPTURE</a>) {
        <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_0, nfa, start, end);
        <b>let</b> start_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> nfa.states, start);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> start_state.group_actions, <a href="regex.md#0x1_regex_group_action_begin">group_action_begin</a>(node.group_idx));
        <b>let</b> end_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> nfa.states, end);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> end_state.group_actions, <a href="regex.md#0x1_regex_group_action_end">group_action_end</a>(node.group_idx));
    } <b>else</b> <b>if</b> (node.type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__CONCAT">AST_NODE_TYPE__CONCAT</a>) {
        <b>let</b> mid = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
        <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_0, nfa, start, mid);
        <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_1, nfa, mid, end);
    } <b>else</b> <b>if</b> (node.type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__OR">AST_NODE_TYPE__OR</a>) {
        <b>let</b> sub_start_0 = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
        <b>let</b> sub_end_0 = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
        <b>let</b> sub_start_1 = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
        <b>let</b> sub_end_1 = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
        <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_0, nfa, sub_start_0, sub_end_0);
        <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_1, nfa, sub_start_1, sub_end_1);
        <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, start, sub_start_0);
        <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, start, sub_start_1);
        <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, sub_end_0, end);
        <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, sub_end_1, end);
    } <b>else</b> <b>if</b> (node.type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__REPEAT">AST_NODE_TYPE__REPEAT</a>) {
        <b>let</b> last = start;
        <b>let</b> i = 0;
        <b>while</b> (i &lt; node.repeat_min) {
            <b>let</b> new_state = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
            <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_0, nfa, last, new_state);
            last = new_state;
            i = i + 1;
        };
        <b>if</b> (node.repeat_max == <a href="regex.md#0x1_regex_INF">INF</a>) {
            <b>let</b> new_state = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
            <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, last, new_state);
            <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_0, nfa, new_state, end);
            <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, new_state, end);
            <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, end, new_state);
        } <b>else</b> {
            <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, last, end);
            <b>while</b> (i &lt; node.repeat_max) {
                <b>let</b> new_state = <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa);
                <a href="regex.md#0x1_regex_build_nfa_from_ast_node">build_nfa_from_ast_node</a>(ast, node.child_0, nfa, last, new_state);
                <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, new_state, end);
                last = new_state;
                i = i + 1;
            };
        };
    } <b>else</b> <b>if</b> (node.type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__CHARMATCH">AST_NODE_TYPE__CHARMATCH</a>) {
        <b>let</b> start_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> nfa.states, start);
        <b>let</b> transition = <a href="regex.md#0x1_regex_NFATransition">NFATransition</a> {
            activation: node.charset,
            <b>to</b>: end,
        };
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> start_state.normal_transitions, transition);
    } <b>else</b> <b>if</b> (node.type == <a href="regex.md#0x1_regex_AST_NODE_TYPE__EPSILON">AST_NODE_TYPE__EPSILON</a>) {
        <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa, start, end);
    } <b>else</b> {
        <b>abort</b>(8880)
    }
}
</code></pre>



</details>

<a id="0x1_regex_add_empty_state_to_nfa"></a>

## Function `add_empty_state_to_nfa`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa: &<b>mut</b> <a href="regex.md#0x1_regex_NFA">regex::NFA</a>): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_add_empty_state_to_nfa">add_empty_state_to_nfa</a>(nfa: &<b>mut</b> <a href="regex.md#0x1_regex_NFA">NFA</a>): u64 {
    <b>let</b> ret = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&nfa.states);
    <b>let</b> new_state = <a href="regex.md#0x1_regex_NFAState">NFAState</a> {
        idx: ret,
        group_actions: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        epsilon_transitions: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
        normal_transitions: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[],
    };
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> nfa.states, new_state);
    ret
}
</code></pre>



</details>

<a id="0x1_regex_add_epsilon_transition"></a>

## Function `add_epsilon_transition`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa: &<b>mut</b> <a href="regex.md#0x1_regex_NFA">regex::NFA</a>, from: u64, <b>to</b>: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_add_epsilon_transition">add_epsilon_transition</a>(nfa: &<b>mut</b> <a href="regex.md#0x1_regex_NFA">NFA</a>, from: u64, <b>to</b>: u64) {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> nfa.states, from).epsilon_transitions, <b>to</b>);
}
</code></pre>



</details>

<a id="0x1_regex_trigger_epsilon_transitions"></a>

## Function `trigger_epsilon_transitions`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_trigger_epsilon_transitions">trigger_epsilon_transitions</a>(nfa: &<a href="regex.md#0x1_regex_NFA">regex::NFA</a>, sess: &<b>mut</b> <a href="regex.md#0x1_regex_MatchSession">regex::MatchSession</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_trigger_epsilon_transitions">trigger_epsilon_transitions</a>(nfa: &<a href="regex.md#0x1_regex_NFA">NFA</a>, sess: &<b>mut</b> <a href="regex.md#0x1_regex_MatchSession">MatchSession</a>) {
    <b>let</b> num_active_visitors = <a href="../../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&sess.visitors);
    <b>let</b> i = 0;
    <b>while</b> (i &lt; num_active_visitors) {
        <b>let</b> cur_visitor = *<a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&sess.visitors, i);
        <b>let</b> cur_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&nfa.states, cur_visitor.state_idx);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&cur_state.epsilon_transitions, |next_state_idx|{
            <b>let</b> next_state_idx: u64 = *next_state_idx;
            <b>let</b> next_state_last_visit = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> sess.last_visit_times, next_state_idx);
            <b>if</b> (*next_state_last_visit &lt; sess.cur_time) {
                *next_state_last_visit = sess.cur_time;
                <b>let</b> new_visitor = cur_visitor;
                new_visitor.state_idx = next_state_idx;
                <b>let</b> next_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&nfa.states, next_state_idx);
                <a href="regex.md#0x1_regex_update_group_times_on_new_arrival">update_group_times_on_new_arrival</a>(&<b>mut</b> new_visitor, next_state, sess.cur_time);
                <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> sess.visitors, new_visitor);
                num_active_visitors = num_active_visitors + 1;
            }
        });
        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x1_regex_trigger_normal_transitions"></a>

## Function `trigger_normal_transitions`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_trigger_normal_transitions">trigger_normal_transitions</a>(nfa: &<a href="regex.md#0x1_regex_NFA">regex::NFA</a>, sess: &<b>mut</b> <a href="regex.md#0x1_regex_MatchSession">regex::MatchSession</a>, char: u8)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_trigger_normal_transitions">trigger_normal_transitions</a>(nfa: &<a href="regex.md#0x1_regex_NFA">NFA</a>, sess: &<b>mut</b> <a href="regex.md#0x1_regex_MatchSession">MatchSession</a>, char: u8) {
    sess.cur_time = sess.cur_time + 1;
    <b>let</b> new_visitors = <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>[];
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&sess.visitors, |visitor|{
        <b>let</b> visitor: &<a href="regex.md#0x1_regex_VisitorState">VisitorState</a> = visitor;
        <b>let</b> cur_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&nfa.states, visitor.state_idx);
        <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&cur_state.normal_transitions, |transition|{
            <b>let</b> transition: &<a href="regex.md#0x1_regex_NFATransition">NFATransition</a> = transition;
            <b>let</b> next_state_last_visit = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> sess.last_visit_times, transition.<b>to</b>);
            <b>if</b> (<a href="regex.md#0x1_regex_char_triggers_transition">char_triggers_transition</a>(char, transition) && *next_state_last_visit &lt; sess.cur_time) {
                *next_state_last_visit = sess.cur_time;
                <b>let</b> new_visitor = *visitor;
                new_visitor.state_idx = transition.<b>to</b>;
                <b>let</b> next_state = <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&nfa.states, transition.<b>to</b>);
                <a href="regex.md#0x1_regex_update_group_times_on_new_arrival">update_group_times_on_new_arrival</a>(&<b>mut</b> new_visitor, next_state, sess.cur_time);
                <a href="../../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> new_visitors, new_visitor);
            }
        });
    });
    sess.visitors = new_visitors;
}
</code></pre>



</details>

<a id="0x1_regex_update_group_times_on_new_arrival"></a>

## Function `update_group_times_on_new_arrival`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_update_group_times_on_new_arrival">update_group_times_on_new_arrival</a>(visitor: &<b>mut</b> <a href="regex.md#0x1_regex_VisitorState">regex::VisitorState</a>, new_state: &<a href="regex.md#0x1_regex_NFAState">regex::NFAState</a>, visit_time: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_update_group_times_on_new_arrival">update_group_times_on_new_arrival</a>(visitor: &<b>mut</b> <a href="regex.md#0x1_regex_VisitorState">VisitorState</a>, new_state: &<a href="regex.md#0x1_regex_NFAState">NFAState</a>, visit_time: u64) {
    <a href="../../move-stdlib/doc/vector.md#0x1_vector_for_each_ref">vector::for_each_ref</a>(&new_state.group_actions, |action|{
        <b>let</b> (group_idx, is_end) = <a href="regex.md#0x1_regex_decode_group_action">decode_group_action</a>(*action);
        <b>let</b> record = <b>if</b> (is_end) {
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> visitor.group_end_times, group_idx)
        } <b>else</b> {
            <a href="../../move-stdlib/doc/vector.md#0x1_vector_borrow_mut">vector::borrow_mut</a>(&<b>mut</b> visitor.group_start_times, group_idx)
        };
        *record = visit_time;
    });
}
</code></pre>



</details>

<a id="0x1_regex_get_charset"></a>

## Function `get_charset`

Given a char in regex, return whether it is considered a meta-character and the charset it matches.


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_get_charset">get_charset</a>(char: u8, escaped: bool): (bool, u256)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_get_charset">get_charset</a>(char: u8, escaped: bool): (bool, u256) {
    <b>if</b> (escaped && char == 119) { // \w <b>to</b> match a word char, equivalent <b>to</b> [a-zA-Z0-9_]
        (<b>true</b>, 10633823849912963253799171395480977408)
    } <b>else</b> <b>if</b> (escaped && char == 115) { // \s <b>to</b> match a space char, equivalent <b>to</b> [ \n\r\t\f]
        (<b>true</b>, 4294981120)
    } <b>else</b> <b>if</b> (escaped && char == 100) { // \d <b>to</b> match a digit char, equivalent <b>to</b> [0-9]
        (<b>true</b>, 287948901175001088)
    } <b>else</b> <b>if</b> (!escaped && char == 46) { // . <b>to</b> match a char
        (<b>true</b>, 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff)
    } <b>else</b> {
        (<b>false</b>, 1 &lt;&lt; char)
    }
}
</code></pre>



</details>

<a id="0x1_regex_char_triggers_transition"></a>

## Function `char_triggers_transition`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_char_triggers_transition">char_triggers_transition</a>(char: u8, transition: &<a href="regex.md#0x1_regex_NFATransition">regex::NFATransition</a>): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="regex.md#0x1_regex_char_triggers_transition">char_triggers_transition</a>(char: u8, transition: &<a href="regex.md#0x1_regex_NFATransition">NFATransition</a>): bool { (transition.activation & (1&lt;&lt;char)) &gt; 0 }
</code></pre>



</details>

<a id="0x1_regex_decode_group_action"></a>

## Function `decode_group_action`



<pre><code><b>fun</b> <a href="regex.md#0x1_regex_decode_group_action">decode_group_action</a>(group_action: u64): (u64, bool)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="regex.md#0x1_regex_decode_group_action">decode_group_action</a>(group_action: u64): (u64, bool) {
    <b>let</b> group_idx = group_action & (<a href="regex.md#0x1_regex_GROUP_ACTION_MASK">GROUP_ACTION_MASK</a> - 1);
    <b>let</b> is_end = (group_action & <a href="regex.md#0x1_regex_GROUP_ACTION_MASK">GROUP_ACTION_MASK</a>) &gt; 0;
    (group_idx, is_end)
}
</code></pre>



</details>

<a id="0x1_regex_group_action_begin"></a>

## Function `group_action_begin`

Create a "Group begin" action for a give group.


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_group_action_begin">group_action_begin</a>(group_idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="regex.md#0x1_regex_group_action_begin">group_action_begin</a>(group_idx: u64): u64 { group_idx }
</code></pre>



</details>

<a id="0x1_regex_group_action_end"></a>

## Function `group_action_end`

Create a "Group end" action for a give group.


<pre><code><b>fun</b> <a href="regex.md#0x1_regex_group_action_end">group_action_end</a>(group_idx: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code>inline <b>fun</b> <a href="regex.md#0x1_regex_group_action_end">group_action_end</a>(group_idx: u64): u64 { group_idx + (1 &lt;&lt; 63) }
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
