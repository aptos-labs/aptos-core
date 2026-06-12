{# Copyright (c) The Diem Core Contributors
   SPDX-License-Identifier: Apache-2.0
#}

{# Vectors
   =======
#}

{% macro vector_module(instance) %}
{%- set S = "'" ~ instance.suffix ~ "'" -%}
{%- set T = instance.name -%}
{%- if options.native_equality -%}
{# Whole vector has native equality #}
function {:inline} $IsEqual'vec{{S}}'(v1: Vec ({{T}}), v2: Vec ({{T}})): bool {
    v1 == v2
}
{%- else -%}
// Not inlined. It appears faster this way.
function $IsEqual'vec{{S}}'(v1: Vec ({{T}}), v2: Vec ({{T}})): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual{{S}}(ReadVec(v1, i), ReadVec(v2, i)))
}
{%- endif %}

// Not inlined.
function $IsPrefix'vec{{S}}'(v: Vec ({{T}}), prefix: Vec ({{T}})): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual{{S}}(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec{{S}}'(v: Vec ({{T}}), suffix: Vec ({{T}})): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual{{S}}(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec{{S}}'(v: Vec ({{T}})): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid{{S}}(ReadVec(v, i)))
}

{# TODO: there is an issue with existential quantifier instantiation if we use the native
   functions here without the $IsValid'u64' tag.
#}
{%- if false and instance.has_native_equality -%}
{# Vector elements have native equality #}
function {:inline} $ContainsVec{{S}}(v: Vec ({{T}}), e: {{T}}): bool {
    ContainsVec(v, e)
}

function {:inline} $IndexOfVec{{S}}(v: Vec ({{T}}), e: {{T}}): int {
    IndexOfVec(v, e)
}
{% else %}
function {:inline} $ContainsVec{{S}}(v: Vec ({{T}}), e: {{T}}): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual{{S}}(ReadVec(v, i), e))
}

function $IndexOfVec{{S}}(v: Vec ({{T}}), e: {{T}}): int;
axiom (forall v: Vec ({{T}}), e: {{T}}:: {$IndexOfVec{{S}}(v, e)}
    (var i := $IndexOfVec{{S}}(v, e);
     if (!$ContainsVec{{S}}(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual{{S}}(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual{{S}}(ReadVec(v, j), e))));
{% endif %}

function {:inline} $RangeVec{{S}}(v: Vec ({{T}})): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec{{S}}(): Vec ({{T}}) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty{{S}}() returns (v: Vec ({{T}})) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty{{S}}(): Vec ({{T}}) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty{{S}}(v: Vec ({{T}})) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back{{S}}(m: $Mutation (Vec ({{T}})), val: {{T}}) returns (m': $Mutation (Vec ({{T}}))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back{{S}}(v: Vec ({{T}}), val: {{T}}): Vec ({{T}}) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back{{S}}(m: $Mutation (Vec ({{T}}))) returns (e: {{T}}, m': $Mutation (Vec ({{T}}))) {
    var v: Vec ({{T}});
    var len: int;
    v := $Dereference(m);
    len := LenVec(v);
    if (len == 0) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, len-1);
    m' := $UpdateMutation(m, RemoveVec(v));
}

procedure {:inline 1} $1_vector_append{{S}}(m: $Mutation (Vec ({{T}})), other: Vec ({{T}})) returns (m': $Mutation (Vec ({{T}}))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse{{S}}(m: $Mutation (Vec ({{T}}))) returns (m': $Mutation (Vec ({{T}}))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append{{S}}(m: $Mutation (Vec ({{T}})), other: Vec ({{T}})) returns (m': $Mutation (Vec ({{T}}))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse{{S}}(m: $Mutation (Vec ({{T}})), new_len: int) returns (v: (Vec ({{T}})), m': $Mutation (Vec ({{T}}))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    v := ReverseVec(v);
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_trim{{S}}(m: $Mutation (Vec ({{T}})), new_len: int) returns (v: (Vec ({{T}})), m': $Mutation (Vec ({{T}}))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice{{S}}(m: $Mutation (Vec ({{T}})), left: int, right: int) returns (m': $Mutation (Vec ({{T}}))) {
    var left_vec: Vec ({{T}});
    var mid_vec: Vec ({{T}});
    var right_vec: Vec ({{T}});
    var v: Vec ({{T}});
    if (left > right) {
        call $ExecFailureAbort();
        return;
    }
    if (left == right) {
        m' := m;
        return;
    }
    v := $Dereference(m);
    if (!(right >= 0 && right <= LenVec(v))) {
        call $ExecFailureAbort();
        return;
    }
    left_vec := SliceVec(v, 0, left);
    right_vec := SliceVec(v, right, LenVec(v));
    mid_vec := ReverseVec(SliceVec(v, left, right));
    m' := $UpdateMutation(m, ConcatVec(left_vec, ConcatVec(mid_vec, right_vec)));
}

procedure {:inline 1} $1_vector_rotate{{S}}(m: $Mutation (Vec ({{T}})), rot: int) returns (n: int, m': $Mutation (Vec ({{T}}))) {
    var v: Vec ({{T}});
    var len: int;
    var left_vec: Vec ({{T}});
    var right_vec: Vec ({{T}});
    v := $Dereference(m);
    if (!(rot >= 0 && rot <= LenVec(v))) {
        call $ExecFailureAbort();
        return;
    }
    left_vec := SliceVec(v, 0, rot);
    right_vec := SliceVec(v, rot, LenVec(v));
    m' := $UpdateMutation(m, ConcatVec(right_vec, left_vec));
    n := LenVec(v) - rot;
}

procedure {:inline 1} $1_vector_rotate_slice{{S}}(m: $Mutation (Vec ({{T}})), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec ({{T}}))) {
    var left_vec: Vec ({{T}});
    var mid_vec: Vec ({{T}});
    var right_vec: Vec ({{T}});
    var mid_left_vec: Vec ({{T}});
    var mid_right_vec: Vec ({{T}});
    var v: Vec ({{T}});
    v := $Dereference(m);
    if (!(left <= rot && rot <= right)) {
        call $ExecFailureAbort();
        return;
    }
    if (!(right >= 0 && right <= LenVec(v))) {
        call $ExecFailureAbort();
        return;
    }
    v := $Dereference(m);
    left_vec := SliceVec(v, 0, left);
    right_vec := SliceVec(v, right, LenVec(v));
    mid_left_vec := SliceVec(v, left, rot);
    mid_right_vec := SliceVec(v, rot, right);
    mid_vec := ConcatVec(mid_right_vec, mid_left_vec);
    m' := $UpdateMutation(m, ConcatVec(left_vec, ConcatVec(mid_vec, right_vec)));
    n := left + (right - rot);
}

procedure {:inline 1} $1_vector_insert{{S}}(m: $Mutation (Vec ({{T}})), i: int, e: {{T}}) returns (m': $Mutation (Vec ({{T}}))) {
    var left_vec: Vec ({{T}});
    var right_vec: Vec ({{T}});
    var v: Vec ({{T}});
    v := $Dereference(m);
    if (!(i >= 0 && i <= LenVec(v))) {
        call $ExecFailureAbort();
        return;
    }
    if (i == LenVec(v)) {
        m' := $UpdateMutation(m, ExtendVec(v, e));
    } else {
        left_vec := ExtendVec(SliceVec(v, 0, i), e);
        right_vec := SliceVec(v, i, LenVec(v));
        m' := $UpdateMutation(m, ConcatVec(left_vec, right_vec));
    }
}

// `vector::move_range(from, removal_position, length, to, insert_position)` extracts the
// half-open range `[removal_position, removal_position+length)` from `from` and splices it
// into `to` at `insert_position`, shifting `to[insert_position..]` to the right. Move enforces
// that `from` and `to` are distinct (no aliasing of mutable references).
procedure {:inline 1} $1_vector_move_range{{S}}(
    from: $Mutation (Vec ({{T}})),
    removal_position: int,
    length: int,
    to: $Mutation (Vec ({{T}})),
    insert_position: int
) returns (from': $Mutation (Vec ({{T}})), to': $Mutation (Vec ({{T}})))
{
    var from_v: Vec ({{T}});
    var to_v: Vec ({{T}});
    var middle: Vec ({{T}});
    from_v := $Dereference(from);
    to_v := $Dereference(to);
    // The `< 0` checks are defensive — Move's u64 arguments are non-negative by typing,
    // but Boogie ints can be arbitrary so we guard explicitly. Matches the convention
    // used in `$1_vector_insert` above.
    if (removal_position < 0
        || length < 0
        || removal_position + length > LenVec(from_v)
        || insert_position < 0
        || insert_position > LenVec(to_v)) {
        call $ExecFailureAbort();
        return;
    }
    middle := SliceVec(from_v, removal_position, removal_position + length);
    from' := $UpdateMutation(from,
        ConcatVec(SliceVec(from_v, 0, removal_position),
                  SliceVec(from_v, removal_position + length, LenVec(from_v))));
    to' := $UpdateMutation(to,
        ConcatVec(SliceVec(to_v, 0, insert_position),
                  ConcatVec(middle, SliceVec(to_v, insert_position, LenVec(to_v)))));
}

procedure {:inline 1} $1_vector_length{{S}}(v: Vec ({{T}})) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length{{S}}(v: Vec ({{T}})): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow{{S}}(v: Vec ({{T}}), i: int) returns (dst: {{T}}) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow{{S}}(v: Vec ({{T}}), i: int): {{T}} {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut{{S}}(m: $Mutation (Vec ({{T}})), index: int)
returns (dst: $Mutation ({{T}}), m': $Mutation (Vec ({{T}})))
{
    var v: Vec ({{T}});
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(m->l, ExtendVec(m->p, index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut{{S}}(v: Vec ({{T}}), i: int): {{T}} {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty{{S}}(v: Vec ({{T}})) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap{{S}}(m: $Mutation (Vec ({{T}})), i: int, j: int) returns (m': $Mutation (Vec ({{T}})))
{
    var v: Vec ({{T}});
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap{{S}}(v: Vec ({{T}}), i: int, j: int): Vec ({{T}}) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove{{S}}(m: $Mutation (Vec ({{T}})), i: int) returns (e: {{T}}, m': $Mutation (Vec ({{T}})))
{
    var v: Vec ({{T}});

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove{{S}}(m: $Mutation (Vec ({{T}})), i: int) returns (e: {{T}}, m': $Mutation (Vec ({{T}})))
{
    var len: int;
    var v: Vec ({{T}});

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains{{S}}(v: Vec ({{T}}), e: {{T}}) returns (res: bool)  {
    res := $ContainsVec{{S}}(v, e);
}

procedure {:inline 1}
$1_vector_index_of{{S}}(v: Vec ({{T}}), e: {{T}}) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec{{S}}(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}
{% endmacro vector_module %}

{# Tables
   =======
#}

{% macro table_key_encoding(instance) %}
{%- set K = instance.name -%}
{%- set S = "'" ~ instance.suffix ~ "'" -%}

function $EncodeKey{{S}}(k: {{K}}): int;
axiom (
  forall k1, k2: {{K}} :: {$EncodeKey{{S}}(k1), $EncodeKey{{S}}(k2)}
    $IsEqual{{S}}(k1, k2) <==> $EncodeKey{{S}}(k1) == $EncodeKey{{S}}(k2)
);
{% endmacro table_key_encoding %}


{% macro table_module(impl, instance) %}
{%- set K = instance.0.name -%}
{%- set V = instance.1.name -%}
{%- set Type = impl.struct_name -%}
{%- set Self = "Table int (" ~ V ~ ")" -%}
{%- set S = "'" ~ instance.0.suffix ~ "_" ~ instance.1.suffix ~ "'" -%}
{%- set SK = "'" ~ instance.0.suffix ~ "'" -%}
{%- set SV = "'" ~ instance.1.suffix ~ "'" -%}
{%- set ENC = "$EncodeKey'" ~ instance.0.suffix ~ "'" -%}
{# Iterator type for this K — empty when the map has no `IteratorPtr` companion. #}
{%- set IT = impl.iter_type_prefix ~ SK -%}
{# Iterator-with-path type for this K — empty when no `IteratorPtrWithPath` companion. #}
{%- set IPWP = impl.iter_with_path_type_prefix ~ SK -%}

{%- if options.native_equality -%}
function $IsEqual'{{Type}}{{S}}'(t1: {{Self}}, t2: {{Self}}): bool {
    t1 == t2
}
{%- else -%}
function $IsEqual'{{Type}}{{S}}'(t1: {{Self}}, t2: {{Self}}): bool {
    LenTable(t1) == LenTable(t2) &&
    (forall k: int :: ContainsTable(t1, k) <==> ContainsTable(t2, k)) &&
    (forall k: int :: ContainsTable(t1, k) ==> GetTable(t1, k) == GetTable(t2, k)) &&
    (forall k: int :: ContainsTable(t2, k) ==> GetTable(t1, k) == GetTable(t2, k))
}
{%- endif %}

// Not inlined.
function $IsValid'{{Type}}{{S}}'(t: {{Self}}): bool {
    $IsValid'u64'(LenTable(t)) &&
    (forall i: int:: ContainsTable(t, i) ==> $IsValid{{SV}}(GetTable(t, i)))
}

{%- if impl.fun_new != "" %}
procedure {:inline 2} {{impl.fun_new}}{{S}}() returns (v: {{Self}}) {
    v := EmptyTable();
}
{%- endif %}

{%- if impl.fun_new_with_config != "" %}
// Create an empty map with configured degree limits. Aborts when either degree
// is non-zero and outside the supported [INNER_MIN_DEGREE=4 | LEAF_MIN_DEGREE=3, MAX_DEGREE=4096]
// range; `reuse_slots` does not contribute abort conditions at this level.
procedure {:inline 2} {{impl.fun_new_with_config}}{{S}}(inner_max_degree: int, leaf_max_degree: int, reuse_slots: bool) returns (v: {{Self}}) {
    if (inner_max_degree != 0 && (inner_max_degree < 4 || inner_max_degree > 4096)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 11/*EINVALID_CONFIG_PARAMETER*/));
    } else if (leaf_max_degree != 0 && (leaf_max_degree < 3 || leaf_max_degree > 4096)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 11/*EINVALID_CONFIG_PARAMETER*/));
    } else {
        v := EmptyTable();
    }
}
{%- endif %}

{%- if impl.fun_new_with_reusable != "" %}
// Create an empty map with the reuse-slots policy enabled. The Move source aborts
// when K/V are not constant-serialized-size, but that's a BCS-level property not
// expressible here; we conservatively report aborts_if false to match the existing
// trusted abstract spec.
procedure {:inline 2} {{impl.fun_new_with_reusable}}{{S}}() returns (v: {{Self}}) {
    v := EmptyTable();
}
{%- endif %}

{%- if impl.fun_new_with_type_size_hints != "" %}
// Create an empty map configured against size hints. The Move source asserts
// avg <= max for both key and value, but the existing trusted abstract spec
// reports aborts_if false; we match that here.
procedure {:inline 2} {{impl.fun_new_with_type_size_hints}}{{S}}(avg_key_bytes: int, max_key_bytes: int, avg_value_bytes: int, max_value_bytes: int) returns (v: {{Self}}) {
    v := EmptyTable();
}
{%- endif %}

{%- if impl.fun_destroy_empty != "" %}
procedure {:inline 2} {{impl.fun_destroy_empty}}{{S}}(t: {{Self}}) {
    if (LenTable(t) != 0) {
        call $Abort($StdError(1/*INVALID_STATE*/, 102/*ENOT_EMPTY*/));
    }
}
{%- endif %}

{%- if impl.fun_len != "" %}
procedure {:inline 2} {{impl.fun_len}}{{S}}(t: ({{Self}})) returns (l: int) {
    l := LenTable(t);
}
{%- endif %}

{%- if impl.fun_is_empty != "" %}
procedure {:inline 2} {{impl.fun_is_empty}}{{S}}(t: ({{Self}})) returns (r: bool) {
    r := LenTable(t) == 0;
}
{%- endif %}

{%- if impl.fun_has_key != "" %}
procedure {:inline 2} {{impl.fun_has_key}}{{S}}(t: ({{Self}}), k: {{K}}) returns (r: bool) {
    r := ContainsTable(t, {{ENC}}(k));
}
{%- endif %}

{%- if impl.fun_add_no_override != "" %}
procedure {:inline 2} {{impl.fun_add_no_override}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, v: {{V}}) returns (m': $Mutation({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 100/*EALREADY_EXISTS*/));
    } else {
        m' := $UpdateMutation(m, AddTable(t, enc_k, v));
    }
}
{%- endif %}

{%- if impl.fun_add_override_if_exists != "" %}
procedure {:inline 2} {{impl.fun_add_override_if_exists}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, v: {{V}}) returns (m': $Mutation({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t, enc_k)) {
        m' := $UpdateMutation(m, UpdateTable(t, enc_k, v));
    } else {
        m' := $UpdateMutation(m, AddTable(t, enc_k, v));
    }
}
{%- endif %}

{%- if impl.fun_del_must_exist != "" %}
// Remove the entry at `k`, returning its value. Aborts when `k` is absent. The
// abort code is the prover-internal `$StdError(7, 101)` (INVALID_ARGUMENTS, ENOT_FOUND);
// this is an abstract code shared across all maps bound to this intrinsic. The runtime
// abort code differs per map (e.g. `big_ordered_map::remove` aborts with
// `error::invalid_argument(EKEY_NOT_FOUND=2)`), so user specs MUST NOT rely on
// `aborts_with <literal>` to match the runtime constant — use `aborts_if` (boolean)
// instead. The mismatch is uniform across all map intrinsics (cf. `add_no_override`,
// iter codes) and reflects the prover's category-only abstraction of abort codes.
procedure {:inline 2} {{impl.fun_del_must_exist}}{{S}}(m: $Mutation ({{Self}}), k: {{K}})
returns (v: {{V}}, m': $Mutation({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        v := GetTable(t, enc_k);
        m' := $UpdateMutation(m, RemoveTable(t, enc_k));
    }
}
{%- endif %}

{%- if impl.fun_del_return_key != "" %}
procedure {:inline 2} {{impl.fun_del_return_key}}{{S}}(m: $Mutation ({{Self}}), k: {{K}})
returns (k': {{K}}, v: {{V}}, m': $Mutation({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        k' := k;
        v := GetTable(t, enc_k);
        m' := $UpdateMutation(m, RemoveTable(t, enc_k));
    }
}
{%- endif %}

{%- if impl.fun_borrow != "" %}
procedure {:inline 2} {{impl.fun_borrow}}{{S}}(t: {{Self}}, k: {{K}}) returns (v: {{V}}) {
    var enc_k: int;
    enc_k := {{ENC}}(k);
    if (!ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        v := GetTable(t, {{ENC}}(k));
    }
}
{%- endif %}

{%- if impl.fun_borrow_mut != "" %}
procedure {:inline 2} {{impl.fun_borrow_mut}}{{S}}(m: $Mutation ({{Self}}), k: {{K}})
returns (dst: $Mutation ({{V}}), m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        dst := $Mutation(m->l, ExtendVec(m->p, enc_k), GetTable(t, enc_k));
        m' := m;
    }
}
{%- endif %}

{%- if impl.fun_borrow_mut_with_default != "" %}
procedure {:inline 2} {{impl.fun_borrow_mut_with_default}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, default: {{V}})
returns (dst: $Mutation ({{V}}), m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    var t': {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t, enc_k)) {
        m' := $UpdateMutation(m, AddTable(t, enc_k, default));
        t' := $Dereference(m');
        dst := $Mutation(m'->l, ExtendVec(m'->p, enc_k), GetTable(t', enc_k));
    } else {
        dst := $Mutation(m->l, ExtendVec(m->p, enc_k), GetTable(t, enc_k));
        m' := m;
    }
}
{%- endif %}

{%- if impl.fun_borrow_with_default != "" %}
procedure {:inline 2} {{impl.fun_borrow_with_default}}{{S}}(t: {{Self}}, k: {{K}}, default: {{V}}) returns (v: {{V}}) {
    var enc_k: int;
    enc_k := {{ENC}}(k);
    if (!ContainsTable(t, enc_k)) {
        v := default;
    } else {
        v := GetTable(t, {{ENC}}(k));
    }
}
{%- endif %}

{%- if impl.fun_get != "" and not instance.1.is_type_param and not instance.1.is_bv %}
// Optional lookup: returns Some(v) when k is in the map, None otherwise. Never aborts.
procedure {:inline 2} {{impl.fun_get}}{{S}}(t: {{Self}}, k: {{K}}) returns (result: $1_option_Option{{SV}}) {
    if (ContainsTable(t, {{ENC}}(k))) {
        result := $1_option_Option{{SV}}_Some(GetTable(t, {{ENC}}(k)));
    } else {
        result := $1_option_Option{{SV}}_None();
    }
}
{%- endif %}

{%- if impl.fun_to_vec_pair != "" %}
// Decompose the map into two vectors with parallel positions. Order is unspecified
// because the underlying SMT-array model has no notion of insertion order.
procedure {:inline 2} {{impl.fun_to_vec_pair}}{{S}}(t: {{Self}}) returns (ks: Vec ({{K}}), vs: Vec ({{V}})) {
    assume LenVec(ks) == LenTable(t);
    assume LenVec(vs) == LenTable(t);
    assume (forall i: int :: 0 <= i && i < LenVec(ks) ==>
        ContainsTable(t, {{ENC}}(ReadVec(ks, i))));
    assume (forall k: {{K}} :: ContainsTable(t, {{ENC}}(k)) ==> $ContainsVec{{SK}}(ks, k));
    assume (forall i: int :: 0 <= i && i < LenVec(ks) ==>
        GetTable(t, {{ENC}}(ReadVec(ks, i))) == ReadVec(vs, i));
}
{%- endif %}

{%- if impl.fun_keys != "" %}
// Project all keys of the map into a vector. Order is unspecified. Keys are distinct
// (forced by `LenVec == LenTable` + every position is a Table key).
procedure {:inline 2} {{impl.fun_keys}}{{S}}(t: {{Self}}) returns (ks: Vec ({{K}})) {
    assume LenVec(ks) == LenTable(t);
    assume (forall i: int :: 0 <= i && i < LenVec(ks) ==>
        ContainsTable(t, {{ENC}}(ReadVec(ks, i))));
    assume (forall k: {{K}} :: ContainsTable(t, {{ENC}}(k)) ==> $ContainsVec{{SK}}(ks, k));
}
{%- endif %}

{%- if impl.fun_values != "" %}
// Project all values of the map into a vector. Order is unspecified, but multiplicity
// is preserved: if two distinct keys map to the same value, that value appears twice
// in the result. The local `ks` is a havoc'd witness, constrained to be a permutation
// of the table's keys, and used positionally to pin each `vs` slot to a distinct key's
// value. Callers never see `ks`, so they cannot reason about which permutation; but
// its existence forces `vs` to be the multiset of values of the map's entries.
procedure {:inline 2} {{impl.fun_values}}{{S}}(t: {{Self}}) returns (vs: Vec ({{V}})) {
    var ks: Vec ({{K}});
    assume LenVec(ks) == LenTable(t);
    assume LenVec(vs) == LenTable(t);
    assume (forall i: int :: 0 <= i && i < LenVec(ks) ==>
        ContainsTable(t, {{ENC}}(ReadVec(ks, i))));
    assume (forall k: {{K}} :: ContainsTable(t, {{ENC}}(k)) ==> $ContainsVec{{SK}}(ks, k));
    assume (forall i: int :: 0 <= i && i < LenVec(ks) ==>
        GetTable(t, {{ENC}}(ReadVec(ks, i))) == ReadVec(vs, i));
}
{%- endif %}

{%- if impl.fun_front_key != "" and not instance.0.is_type_param %}
// Smallest key in the map under cmp::compare ordering. Aborts on an empty map. The
// result is a key in the map, and every other key compares Greater.
procedure {:inline 2} {{impl.fun_front_key}}{{S}}(t: {{Self}}) returns (k: {{K}}) {
    if (LenTable(t) == 0) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 3/*EITER_OUT_OF_BOUNDS*/));
    } else {
        assume ContainsTable(t, {{ENC}}(k));
        assume (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(k) ==>
            $1_cmp_$compare{{SK}}(k, k0) == $1_cmp_Ordering_Less());
    }
}
{%- endif %}

{%- if impl.fun_back_key != "" and not instance.0.is_type_param %}
// Largest key in the map under cmp::compare ordering. Aborts on an empty map.
procedure {:inline 2} {{impl.fun_back_key}}{{S}}(t: {{Self}}) returns (k: {{K}}) {
    if (LenTable(t) == 0) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 3/*EITER_OUT_OF_BOUNDS*/));
    } else {
        assume ContainsTable(t, {{ENC}}(k));
        assume (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(k) ==>
            $1_cmp_$compare{{SK}}(k, k0) == $1_cmp_Ordering_Greater());
    }
}
{%- endif %}

{%- if impl.fun_borrow_front != "" and not instance.0.is_type_param %}
// Smallest key in the map together with its value. Aborts on an empty map. The
// Move return type is `(K, &V)`; at the intrinsic boundary the reference is stripped
// (same convention as `map_borrow`).
procedure {:inline 2} {{impl.fun_borrow_front}}{{S}}(t: {{Self}}) returns (k: {{K}}, v: {{V}}) {
    if (LenTable(t) == 0) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 3/*EITER_OUT_OF_BOUNDS*/));
    } else {
        assume ContainsTable(t, {{ENC}}(k));
        assume (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(k) ==>
            $1_cmp_$compare{{SK}}(k, k0) == $1_cmp_Ordering_Less());
        v := GetTable(t, {{ENC}}(k));
    }
}
{%- endif %}

{%- if impl.fun_borrow_back != "" and not instance.0.is_type_param %}
// Largest key in the map together with its value. Aborts on an empty map.
procedure {:inline 2} {{impl.fun_borrow_back}}{{S}}(t: {{Self}}) returns (k: {{K}}, v: {{V}}) {
    if (LenTable(t) == 0) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 3/*EITER_OUT_OF_BOUNDS*/));
    } else {
        assume ContainsTable(t, {{ENC}}(k));
        assume (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(k) ==>
            $1_cmp_$compare{{SK}}(k, k0) == $1_cmp_Ordering_Greater());
        v := GetTable(t, {{ENC}}(k));
    }
}
{%- endif %}

{%- if impl.fun_pop_front != "" and not instance.0.is_type_param %}
// Remove and return the smallest entry under cmp::compare. Aborts on an empty map.
procedure {:inline 2} {{impl.fun_pop_front}}{{S}}(m: $Mutation ({{Self}}))
returns (k: {{K}}, v: {{V}}, m': $Mutation ({{Self}})) {
    var t: {{Self}};
    t := $Dereference(m);
    if (LenTable(t) == 0) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 3/*EITER_OUT_OF_BOUNDS*/));
    } else {
        assume ContainsTable(t, {{ENC}}(k));
        assume (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(k) ==>
            $1_cmp_$compare{{SK}}(k, k0) == $1_cmp_Ordering_Less());
        v := GetTable(t, {{ENC}}(k));
        m' := $UpdateMutation(m, RemoveTable(t, {{ENC}}(k)));
    }
}
{%- endif %}

{%- if impl.fun_pop_back != "" and not instance.0.is_type_param %}
// Remove and return the largest entry under cmp::compare. Aborts on an empty map.
procedure {:inline 2} {{impl.fun_pop_back}}{{S}}(m: $Mutation ({{Self}}))
returns (k: {{K}}, v: {{V}}, m': $Mutation ({{Self}})) {
    var t: {{Self}};
    t := $Dereference(m);
    if (LenTable(t) == 0) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 3/*EITER_OUT_OF_BOUNDS*/));
    } else {
        assume ContainsTable(t, {{ENC}}(k));
        assume (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(k) ==>
            $1_cmp_$compare{{SK}}(k, k0) == $1_cmp_Ordering_Greater());
        v := GetTable(t, {{ENC}}(k));
        m' := $UpdateMutation(m, RemoveTable(t, {{ENC}}(k)));
    }
}
{%- endif %}

{%- if impl.fun_prev_key != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Largest key strictly less than `key` under cmp::compare. Returns Some when one
// exists, None otherwise. Never aborts.
procedure {:inline 2} {{impl.fun_prev_key}}{{S}}(t: {{Self}}, key: {{K}})
returns (result: $1_option_Option{{SK}}) {
    var has_prev: bool;
    var prev_k: {{K}};
    has_prev := (exists k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) &&
        $1_cmp_$compare{{SK}}(k0, key) == $1_cmp_Ordering_Less());
    if (!has_prev) {
        result := $1_option_Option{{SK}}_None();
    } else {
        assume ContainsTable(t, {{ENC}}(prev_k));
        assume $1_cmp_$compare{{SK}}(prev_k, key) == $1_cmp_Ordering_Less();
        assume (forall k0: {{K}} ::
            ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(prev_k) &&
            $1_cmp_$compare{{SK}}(k0, key) == $1_cmp_Ordering_Less() ==>
            $1_cmp_$compare{{SK}}(prev_k, k0) == $1_cmp_Ordering_Greater());
        result := $1_option_Option{{SK}}_Some(prev_k);
    }
}
{%- endif %}

{%- if impl.fun_next_key != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Smallest key strictly greater than `key` under cmp::compare. Returns Some when one
// exists, None otherwise. Never aborts.
procedure {:inline 2} {{impl.fun_next_key}}{{S}}(t: {{Self}}, key: {{K}})
returns (result: $1_option_Option{{SK}}) {
    var has_next: bool;
    var next_k: {{K}};
    has_next := (exists k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) &&
        $1_cmp_$compare{{SK}}(k0, key) == $1_cmp_Ordering_Greater());
    if (!has_next) {
        result := $1_option_Option{{SK}}_None();
    } else {
        assume ContainsTable(t, {{ENC}}(next_k));
        assume $1_cmp_$compare{{SK}}(next_k, key) == $1_cmp_Ordering_Greater();
        assume (forall k0: {{K}} ::
            ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(next_k) &&
            $1_cmp_$compare{{SK}}(k0, key) == $1_cmp_Ordering_Greater() ==>
            $1_cmp_$compare{{SK}}(next_k, k0) == $1_cmp_Ordering_Less());
        result := $1_option_Option{{SK}}_Some(next_k);
    }
}
{%- endif %}

{%- if impl.fun_remove_or_none != "" and not instance.1.is_type_param and not instance.1.is_bv %}
// Remove the entry at `key` if present. Returns Some(prev_value) on hit, None on miss.
// Never aborts.
procedure {:inline 2} {{impl.fun_remove_or_none}}{{S}}(m: $Mutation ({{Self}}), key: {{K}})
returns (result: $1_option_Option{{SV}}, m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(key);
    t := $Dereference(m);
    if (ContainsTable(t, enc_k)) {
        result := $1_option_Option{{SV}}_Some(GetTable(t, enc_k));
        m' := $UpdateMutation(m, RemoveTable(t, enc_k));
    } else {
        result := $1_option_Option{{SV}}_None();
        m' := m;
    }
}
{%- endif %}

{# ====== Iterator API. ====== #}
{# These blocks only render when the map type has a companion IteratorPtr struct       #}
{# (resolved into `iter_type_prefix`) and the instance is concrete. The Some constructor #}
{# may carry implementation fields beyond the key (e.g. BigOrderedMap's node_index +    #}
{# child_iter); we never read them in spec, so we havoc a local of the iter type and  #}
{# constrain only its variant + key. The other fields stay unconstrained.              #}
{# Staleness: only "cached key was removed" is modeled. A stale cached position whose  #}
{# key still happens to exist after a rebalance is NOT caught.                         #}
{# TODO: model node_index/child_iter explicitly to close this gap.                     #}

{%- if impl.fun_iter_new_begin != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Iterator at the smallest key, or End on an empty map. Never aborts.
procedure {:inline 2} {{impl.fun_iter_new_begin}}{{S}}(t: {{Self}}) returns (result: {{IT}}) {
    var some_it: {{IT}};
    var k_min: {{K}};
    if (LenTable(t) == 0) {
        result := {{IT}}_End();
    } else {
        assume ContainsTable(t, {{ENC}}(k_min));
        assume (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(k_min) ==>
            $1_cmp_$compare{{SK}}(k_min, k0) == $1_cmp_Ordering_Less());
        assume some_it is {{IT}}_Some;
        assume some_it->$key_Some == k_min;
        result := some_it;
    }
}
{%- endif %}

{%- if impl.fun_iter_new_end != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// End-sentinel iterator. Never aborts.
procedure {:inline 2} {{impl.fun_iter_new_end}}{{S}}(t: {{Self}}) returns (result: {{IT}}) {
    result := {{IT}}_End();
}
{%- endif %}

{%- if impl.fun_iter_is_end != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// True iff the iterator is the End sentinel. Never aborts.
procedure {:inline 2} {{impl.fun_iter_is_end}}{{S}}(it: {{IT}}, t: {{Self}}) returns (r: bool) {
    r := it is {{IT}}_End;
}
{%- endif %}

{%- if impl.fun_iter_is_begin != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// True iff the iterator points to the begin position: either at the smallest key, or
// is the End sentinel on an empty map. Never aborts.
procedure {:inline 2} {{impl.fun_iter_is_begin}}{{S}}(it: {{IT}}, t: {{Self}}) returns (r: bool) {
    if (it is {{IT}}_End) {
        r := LenTable(t) == 0;
    } else {
        r := ContainsTable(t, {{ENC}}(it->$key_Some))
            && (forall k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(it->$key_Some) ==>
                $1_cmp_$compare{{SK}}(it->$key_Some, k0) == $1_cmp_Ordering_Less());
    }
}
{%- endif %}

{%- if impl.fun_iter_borrow_key != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Read the iterator's key. Aborts when at End.
procedure {:inline 2} {{impl.fun_iter_borrow_key}}{{S}}(it: {{IT}}) returns (k: {{K}}) {
    if (it is {{IT}}_End) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 3/*EITER_OUT_OF_BOUNDS*/));
    } else {
        k := it->$key_Some;
    }
}
{%- endif %}

{# iter_borrow / iter_next / iter_prev templates intentionally omitted: sound modeling   #}
{# requires tracking node_index/child_iter alongside the cached key. Maps binding these  #}
{# roles must either provide a custom opaque spec or wait for that modeling to land.     #}

{%- if impl.fun_internal_find != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Iterator at the given key if present, end-sentinel otherwise. Never aborts.
procedure {:inline 2} {{impl.fun_internal_find}}{{S}}(t: {{Self}}, key: {{K}}) returns (result: {{IT}}) {
    var some_it: {{IT}};
    if (ContainsTable(t, {{ENC}}(key))) {
        assume some_it is {{IT}}_Some;
        assume some_it->$key_Some == key;
        result := some_it;
    } else {
        result := {{IT}}_End();
    }
}
{%- endif %}

{%- if impl.fun_internal_lower_bound != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Iterator at the smallest key K >= input under cmp::compare. End if no such key
// exists (every key in the map is strictly less than `key`). Never aborts.
procedure {:inline 2} {{impl.fun_internal_lower_bound}}{{S}}(t: {{Self}}, key: {{K}}) returns (result: {{IT}}) {
    var has_ge: bool;
    var lb_k: {{K}};
    var some_it: {{IT}};
    has_ge := (exists k0: {{K}} :: ContainsTable(t, {{ENC}}(k0)) &&
        $1_cmp_$compare{{SK}}(k0, key) != $1_cmp_Ordering_Less());
    if (!has_ge) {
        result := {{IT}}_End();
    } else {
        assume ContainsTable(t, {{ENC}}(lb_k));
        assume $1_cmp_$compare{{SK}}(lb_k, key) != $1_cmp_Ordering_Less();
        assume (forall k0: {{K}} ::
            ContainsTable(t, {{ENC}}(k0)) && {{ENC}}(k0) != {{ENC}}(lb_k) &&
            $1_cmp_$compare{{SK}}(k0, key) != $1_cmp_Ordering_Less() ==>
            $1_cmp_$compare{{SK}}(lb_k, k0) != $1_cmp_Ordering_Greater());
        assume some_it is {{IT}}_Some;
        assume some_it->$key_Some == lb_k;
        result := some_it;
    }
}
{%- endif %}

{# ====== IteratorPtrWithPath API. ====== #}
{# IteratorPtrWithPath<K> is a single-constructor struct wrapping an IteratorPtr<K> and #}
{# an implementation-only `path: vector<u64>`. The `$path` field is never read in spec, #}
{# so we havoc a local of the IPWP type and constrain only the `$iterator` field. #}

{%- if impl.fun_internal_find_with_path != "" and impl.iter_type_prefix != "" and impl.iter_with_path_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Iterator-with-path at the given key if present, end-sentinel otherwise. Never aborts.
procedure {:inline 2} {{impl.fun_internal_find_with_path}}{{S}}(t: {{Self}}, key: {{K}}) returns (result: {{IPWP}}) {
    var iter: {{IT}};
    var some_it: {{IT}};
    var ipwp: {{IPWP}};
    if (ContainsTable(t, {{ENC}}(key))) {
        assume some_it is {{IT}}_Some;
        assume some_it->$key_Some == key;
        iter := some_it;
    } else {
        iter := {{IT}}_End();
    }
    assume ipwp->$iterator == iter;
    result := ipwp;
}
{%- endif %}

{%- if impl.fun_iter_with_path_get_iter != "" and impl.iter_type_prefix != "" and impl.iter_with_path_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
// Project the wrapped IteratorPtr<K> from an IteratorPtrWithPath<K>. Never aborts.
procedure {:inline 2} {{impl.fun_iter_with_path_get_iter}}{{S}}(self: {{IPWP}}) returns (result: {{IT}}) {
    result := self->$iterator;
}
{%- endif %}

{# iter_remove template intentionally omitted for the same reason as iter_borrow above. #}

{%- if impl.fun_new_from != "" %}
// True iff `keys` contains two distinct positions with equal encoded keys.
function {:inline} {{impl.fun_new_from}}_HasDup{{S}}(keys: Vec ({{K}})): bool {
    (exists i: int, j: int ::
        0 <= i && i < LenVec(keys) && 0 <= j && j < LenVec(keys) && i != j &&
        {{ENC}}(ReadVec(keys, i)) == {{ENC}}(ReadVec(keys, j)))
}
// Build a Table from two parallel vectors. Aborts when lengths differ or the keys
// contain a duplicate.
procedure {:inline 2} {{impl.fun_new_from}}{{S}}(keys: Vec ({{K}}), values: Vec ({{V}})) returns (t: {{Self}}) {
    // The length-mismatch branch originates in `vector::zip` and aborts with
    // `EVECTORS_LENGTH_MISMATCH = 0x20002` (std::error category OUT_OF_RANGE = 2,
    // reason 2). The duplicate-key branch originates in the inner `add` and aborts
    // with the map's own `EKEY_ALREADY_EXISTS` (std::error category INVALID_ARGUMENT,
    // here approximated as EALREADY_EXISTS in the prover-internal `$StdError` scheme
    // matching how the existing `add_no_override` intrinsic encodes it).
    if (LenVec(keys) != LenVec(values)) {
        call $Abort($StdError(2/*OUT_OF_RANGE*/, 2/*EVECTORS_LENGTH_MISMATCH*/));
    } else if ({{impl.fun_new_from}}_HasDup{{S}}(keys)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 100/*EALREADY_EXISTS*/));
    } else {
        assume LenTable(t) == LenVec(keys);
        assume (forall k: {{K}} :: ContainsTable(t, {{ENC}}(k)) <==> $ContainsVec{{SK}}(keys, k));
        assume (forall i: int :: 0 <= i && i < LenVec(keys) ==>
            GetTable(t, {{ENC}}(ReadVec(keys, i))) == ReadVec(values, i));
    }
}
{%- endif %}

{%- if impl.fun_upsert != "" and not instance.0.is_type_param and not instance.1.is_type_param and not instance.1.is_bv %}
// Insert (k, v) or update v if k already maps. Returns the previous value (if any) wrapped
// in std::option::Option<V>. Never aborts.
procedure {:inline 2} {{impl.fun_upsert}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, v: {{V}})
returns (prev_v: $1_option_Option{{SV}}, m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t, enc_k)) {
        prev_v := $1_option_Option{{SV}}_Some(GetTable(t, enc_k));
        m' := $UpdateMutation(m, UpdateTable(t, enc_k, v));
    } else {
        prev_v := $1_option_Option{{SV}}_None();
        m' := $UpdateMutation(m, AddTable(t, enc_k, v));
    }
}
{%- endif %}

{%- if impl.fun_upsert_kv != "" and not instance.0.is_bv and not instance.1.is_bv %}
// Like upsert, but additionally returns the displaced key as Option<K>. Under the
// $EncodeKey-as-injection model the stored key is $IsEqual to the input key, so we
// return the input key.
procedure {:inline 2} {{impl.fun_upsert_kv}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, v: {{V}})
returns (prev_k: $1_option_Option{{SK}}, prev_v: $1_option_Option{{SV}}, m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t, enc_k)) {
        prev_k := $1_option_Option{{SK}}_Some(k);
        prev_v := $1_option_Option{{SV}}_Some(GetTable(t, enc_k));
        m' := $UpdateMutation(m, UpdateTable(t, enc_k, v));
    } else {
        prev_k := $1_option_Option{{SK}}_None();
        prev_v := $1_option_Option{{SV}}_None();
        m' := $UpdateMutation(m, AddTable(t, enc_k, v));
    }
}
{%- endif %}

{%- if impl.fun_spec_len != "" %}
function {:inline} {{impl.fun_spec_len}}{{S}}(t: ({{Self}})): int {
    LenTable(t)
}
{%- endif %}

{%- if impl.fun_spec_is_empty != "" %}
function {:inline} {{impl.fun_spec_is_empty}}{{S}}(t: ({{Self}})): bool {
    LenTable(t) == 0
}
{%- endif %}

{%- if impl.fun_spec_has_key != "" %}
function {:inline} {{impl.fun_spec_has_key}}{{S}}(t: ({{Self}}), k: {{K}}): bool {
    ContainsTable(t, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_set != "" %}
function {:inline} {{impl.fun_spec_set}}{{S}}(t: {{Self}}, k: {{K}}, v: {{V}}): {{Self}} {
    (var enc_k := {{ENC}}(k);
    if (ContainsTable(t, enc_k)) then
        UpdateTable(t, enc_k, v)
    else
        AddTable(t, enc_k, v))
}
{%- endif %}

{%- if impl.fun_spec_del != "" %}
function {:inline} {{impl.fun_spec_del}}{{S}}(t: {{Self}}, k: {{K}}): {{Self}} {
    RemoveTable(t, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_get != "" %}
function {:inline} {{impl.fun_spec_get}}{{S}}(t: {{Self}}, k: {{K}}): {{V}} {
    GetTable(t, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_new != "" %}
function {:inline} {{impl.fun_spec_new}}{{S}}(): {{Self}} {
    EmptyTable()
}
{%- endif %}

{%- if impl.fun_spec_aborts_destroy_empty != "" %}
function {:inline} {{impl.fun_spec_aborts_destroy_empty}}{{S}}(t: {{Self}}): bool {
    LenTable(t) != 0
}
{%- endif %}

{%- if impl.fun_spec_aborts_add != "" %}
function {:inline} {{impl.fun_spec_aborts_add}}{{S}}(t: {{Self}}, k: {{K}}, v: {{V}}): bool {
    ContainsTable(t, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_aborts_del != "" %}
function {:inline} {{impl.fun_spec_aborts_del}}{{S}}(t: {{Self}}, k: {{K}}): bool {
    !ContainsTable(t, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_aborts_borrow != "" %}
function {:inline} {{impl.fun_spec_aborts_borrow}}{{S}}(t: {{Self}}, k: {{K}}): bool {
    !ContainsTable(t, {{ENC}}(k))
}
{%- endif %}

{# Abort guards for the templates added in the iter / order-key / new_from / config family. #}
{# Each body must match the corresponding procedure's abort guard exactly.                  #}

{%- if impl.fun_spec_aborts_new_from != "" %}
function {:inline} {{impl.fun_spec_aborts_new_from}}{{S}}(keys: Vec ({{K}}), values: Vec ({{V}})): bool {
    LenVec(keys) != LenVec(values) ||
    (exists i: int, j: int ::
        0 <= i && i < LenVec(keys) && 0 <= j && j < LenVec(keys) && i != j &&
        {{ENC}}(ReadVec(keys, i)) == {{ENC}}(ReadVec(keys, j)))
}
{%- endif %}

{%- if impl.fun_spec_aborts_new_with_config != "" %}
function {:inline} {{impl.fun_spec_aborts_new_with_config}}{{S}}(inner_max_degree: int, leaf_max_degree: int, reuse_slots: bool): bool {
    (inner_max_degree != 0 && (inner_max_degree < 4 || inner_max_degree > 4096)) ||
    (leaf_max_degree != 0 && (leaf_max_degree < 3 || leaf_max_degree > 4096))
}
{%- endif %}

{%- if impl.fun_spec_aborts_empty_map != "" %}
function {:inline} {{impl.fun_spec_aborts_empty_map}}{{S}}(t: {{Self}}): bool {
    LenTable(t) == 0
}
{%- endif %}

{%- if impl.fun_spec_aborts_iter_borrow_key != "" and impl.iter_type_prefix != "" and not instance.0.is_type_param and not instance.0.is_bv %}
function {:inline} {{impl.fun_spec_aborts_iter_borrow_key}}{{S}}(it: {{IT}}): bool {
    it is {{IT}}_End
}
{%- endif %}

{# Abort-spec inlines for iter_oob / iter_prev / iter_remove intentionally omitted,    #}
{# matching the omitted procedure templates above.                                      #}

{% endmacro table_module %}


{# BCS
   ====
#}

{% macro bcs_module(instance) %}
{%- set S = "'" ~ instance.suffix ~ "'" -%}
{%- set T = instance.name -%}
// Serialize is modeled as an uninterpreted function, with an additional
// axiom to say it's an injection.

function $1_bcs_serialize{{S}}(v: {{T}}): Vec int;

axiom (forall v1, v2: {{T}} :: {$1_bcs_serialize{{S}}(v1), $1_bcs_serialize{{S}}(v2)}
   $IsEqual{{S}}(v1, v2) <==> $IsEqual'vec'u8''($1_bcs_serialize{{S}}(v1), $1_bcs_serialize{{S}}(v2)));

// This says that serialize returns a non-empty vec<u8>
{% if options.serialize_bound == 0 %}
axiom (forall v: {{T}} :: {$1_bcs_serialize{{S}}(v)}
     ( var r := $1_bcs_serialize{{S}}(v); $IsValid'vec'u8''(r) && LenVec(r) > 0 ));
{% else %}
axiom (forall v: {{T}} :: {$1_bcs_serialize{{S}}(v)}
     ( var r := $1_bcs_serialize{{S}}(v); $IsValid'vec'u8''(r) && LenVec(r) > 0 &&
                            LenVec(r) <= {{options.serialize_bound}} ));
{% endif %}

procedure $1_bcs_to_bytes{{S}}(v: {{T}}) returns (res: Vec int);
ensures res == $1_bcs_serialize{{S}}(v);

function {:inline} $1_bcs_$to_bytes{{S}}(v: {{T}}): Vec int {
    $1_bcs_serialize{{S}}(v)
}

{% if S == "'address'" -%}
// Serialized addresses should have the same length.
const $serialized_address_len: int;
// Serialized addresses should have the same length
axiom (forall v: int :: {$1_bcs_serialize'address'(v)}
     ( var r := $1_bcs_serialize'address'(v); LenVec(r) == $serialized_address_len));
{% endif %}
{% endmacro hash_module %}


{# FROM_BCS
   ====
#}

{% macro from_bcs_module(instance) %}
{%- set S = "'" ~ instance.suffix ~ "'" -%}
{%- set T = instance.name -%}

procedure $1_from_bcs_from_bytes{{S}}(v: Vec int) returns (res: {{T}});

function $1_from_bcs_$from_bytes{{S}}(v: Vec int): {{T}};
axiom (forall v: Vec int :: {$1_from_bcs_deserialize{{S}}(v)}
     ( var r := $1_from_bcs_$from_bytes{{S}}(v); r == $1_from_bcs_deserialize{{S}}(v) ));

{% endmacro from_bcs_module %}


{# Event Module
   ============
#}

{% macro event_module(instance) %}
{%- set S = "'" ~ instance.suffix ~ "'" -%}
{%- set T = instance.name -%}

// Map type specific handle to universal one.
type $1_event_EventHandle{{S}} = $1_event_EventHandle;

function {:inline} $IsEqual'$1_event_EventHandle{{S}}'(a: $1_event_EventHandle{{S}}, b: $1_event_EventHandle{{S}}): bool {
    a == b
}

function $IsValid'$1_event_EventHandle{{S}}'(h: $1_event_EventHandle{{S}}): bool {
    true
}

// Embed event `{{T}}` into universal $EventRep
function {:constructor} $ToEventRep{{S}}(e: {{T}}): $EventRep;
axiom (forall v1, v2: {{T}} :: {$ToEventRep{{S}}(v1), $ToEventRep{{S}}(v2)}
    $IsEqual{{S}}(v1, v2) <==> $ToEventRep{{S}}(v1) == $ToEventRep{{S}}(v2));

// Creates a new event handle. This ensures each time it is called that a unique new abstract event handler is
// returned.
// TODO: we should check (and abort with the right code) if no generator exists for the signer.
procedure {:inline 1} $1_event_new_event_handle{{S}}(signer: $signer) returns (res: $1_event_EventHandle{{S}}) {
    assume $1_event_EventHandles[res] == false;
    $1_event_EventHandles := $1_event_EventHandles[res := true];
}

// This boogie procedure is the model of `emit_event`. This model abstracts away the `counter` behavior, thus not
// mutating (or increasing) `counter`.
procedure {:inline 1} $1_event_emit_event{{S}}(handle_mut: $Mutation $1_event_EventHandle{{S}}, msg: {{T}})
returns (res: $Mutation $1_event_EventHandle{{S}}) {
    var handle: $1_event_EventHandle{{S}};
    handle := $Dereference(handle_mut);
    $es := $ExtendEventStore{{S}}($es, handle, msg);
    res := handle_mut;
}

procedure {:inline 1} $1_event_guid{{S}}(handle_ref: $1_event_EventHandle{{S}})
returns (res: int) {
    // TODO: temporarily mocked. The return type needs to be fixed.
    res := 0;
}

procedure {:inline 1} $1_event_counter{{S}}(handle_ref: $1_event_EventHandle{{S}})
returns (res: int) {
    // TODO: temporarily mocked.
    res := 0;
}

procedure {:inline 1} $1_event_destroy_handle{{S}}(handle: $1_event_EventHandle{{S}}) {
}

function {:inline} $ExtendEventStore{{S}}(
        es: $EventStore, handle: $1_event_EventHandle{{S}}, msg: {{T}}): $EventStore {
    (var stream := es->streams[handle];
    (var stream_new := ExtendMultiset(stream, $ToEventRep{{S}}(msg));
    $EventStore(es->counter+1, es->streams[handle := stream_new])))
}

function {:inline} $CondExtendEventStore{{S}}(
        es: $EventStore, handle: $1_event_EventHandle{{S}}, msg: {{T}}, cond: bool): $EventStore {
    if cond then
        $ExtendEventStore{{S}}(es, handle, msg)
    else
        es
}
{% endmacro event_module %}
