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

{%- if impl.fun_new_with_config != "" and not instance.1.is_bv %}
// Empty map with degree configuration. Aborts when a nonzero degree is outside its
// valid range (INNER_MIN_DEGREE=4 / LEAF_MIN_DEGREE=3 / MAX_DEGREE=4096, mirroring
// big_ordered_map constants). ASSUMPTION: the implementation's size-validation abort
// (key/entry serialized size exceeding node limits) is presumed not to fire.
procedure {:inline 2} {{impl.fun_new_with_config}}{{S}}(inner_max_degree: int, leaf_max_degree: int, reuse_slots: bool) returns (v: {{Self}}) {
    if (inner_max_degree != 0 && (inner_max_degree < 4 || inner_max_degree > 4096)) {
        call $ExecFailureAbort();
        return;
    }
    if (leaf_max_degree != 0 && (leaf_max_degree < 3 || leaf_max_degree > 4096)) {
        call $ExecFailureAbort();
        return;
    }
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

{# Emission gates used by the templates below (documented once here):
   - `cmp_available`: `$1_cmp_$compare'K'` only exists in the prelude when K appears
     in a cmp instantiation; ordering templates must not reference it otherwise.
   - `not instance.1.is_bv`: skips the speculative bit-vector twin instances
     (`add_prelude` duplicates every numeric-valued map instance with a bv value
     representation, usage or not). For Option-returning templates emission there
     would fail Boogie name resolution (`is_bv` is a Boogie-level tag, not a Move
     type, so no registration path mints e.g. `Option'bv64'`); for the rest it
     would only bloat every shard's prelude. Bit-vector-classified maps are not
     supported through these roles. #}
{%- if impl.fun_get != "" and not instance.1.is_bv %}
// Read-only lookup. Returns `Some(value)` when `k` is in the map, `None` otherwise.
// Never aborts.
procedure {:inline 2} {{impl.fun_get}}{{S}}(t: ({{Self}}), k: {{K}}) returns (result: $1_option_Option{{SV}}) {
    var enc_k: int;
    enc_k := {{ENC}}(k);
    if (ContainsTable(t, enc_k)) {
        result := $1_option_Option{{SV}}_Some(GetTable(t, enc_k));
    } else {
        result := $1_option_Option{{SV}}_None();
    }
}
{%- endif %}

{%- if impl.fun_borrow_front != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Smallest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_borrow_front}}{{S}}(t: {{Self}}) returns (k: {{K}}, v: {{V}}) {
    if (LenTable(t) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume $IsValid'{{instance.1.suffix}}'(v);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume v == {{impl.fun_spec_get}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Less());
}
{%- endif %}

{%- if impl.fun_borrow_back != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Largest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_borrow_back}}{{S}}(t: {{Self}}) returns (k: {{K}}, v: {{V}}) {
    if (LenTable(t) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume $IsValid'{{instance.1.suffix}}'(v);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume v == {{impl.fun_spec_get}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Greater());
}
{%- endif %}

{%- if impl.fun_front_key != "" and impl.fun_spec_has_key != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Smallest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_front_key}}{{S}}(t: {{Self}}) returns (k: {{K}}) {
    if (LenTable(t) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Less());
}
{%- endif %}

{%- if impl.fun_back_key != "" and impl.fun_spec_has_key != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Largest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_back_key}}{{S}}(t: {{Self}}) returns (k: {{K}}) {
    if (LenTable(t) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Greater());
}
{%- endif %}

{%- if impl.fun_pop_front != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Remove and return the smallest entry under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_pop_front}}{{S}}(m: $Mutation ({{Self}}))
returns (k: {{K}}, v: {{V}}, m': $Mutation ({{Self}})) {
    var t: {{Self}};
    t := $Dereference(m);
    if (LenTable(t) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume $IsValid'{{instance.1.suffix}}'(v);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume v == {{impl.fun_spec_get}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Less());
    m' := $UpdateMutation(m, RemoveTable(t, {{ENC}}(k)));
}
{%- endif %}

{%- if impl.fun_pop_back != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Remove and return the largest entry under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_pop_back}}{{S}}(m: $Mutation ({{Self}}))
returns (k: {{K}}, v: {{V}}, m': $Mutation ({{Self}})) {
    var t: {{Self}};
    t := $Dereference(m);
    if (LenTable(t) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume $IsValid'{{instance.1.suffix}}'(v);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume v == {{impl.fun_spec_get}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Greater());
    m' := $UpdateMutation(m, RemoveTable(t, {{ENC}}(k)));
}
{%- endif %}

{%- if impl.fun_prev_key != "" and impl.fun_spec_has_key != "" and not instance.0.is_bv and instance.0.cmp_available %}
// Largest key strictly less than `key` under `cmp::compare`, wrapped in `Option<K>`
// (None when no such key exists). Never aborts.
procedure {:inline 2} {{impl.fun_prev_key}}{{S}}(t: {{Self}}, key: {{K}}) returns (result: $1_option_Option{{SK}}) {
    var k: {{K}};
    if ((exists k_p: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k_p)} $IsValid'{{instance.0.suffix}}'(k_p)
            && {{impl.fun_spec_has_key}}{{S}}(t, k_p)
            && $1_cmp_$compare'{{instance.0.suffix}}'(k_p, key) == $1_cmp_Ordering_Less())) {
        assume $IsValid'{{instance.0.suffix}}'(k);
        assume {{impl.fun_spec_has_key}}{{S}}(t, k);
        assume $1_cmp_$compare'{{instance.0.suffix}}'(k, key) == $1_cmp_Ordering_Less();
        // k is the *largest* such predecessor: any other in-map k_p that is also
        // < key must satisfy k > k_p.
        assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
            !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
            {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(other, key) == $1_cmp_Ordering_Less() ==>
                $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Greater());
        result := $1_option_Option{{SK}}_Some(k);
    } else {
        result := $1_option_Option{{SK}}_None();
    }
}
{%- endif %}

{%- if impl.fun_next_key != "" and impl.fun_spec_has_key != "" and not instance.0.is_bv and instance.0.cmp_available %}
// Smallest key strictly greater than `key` under `cmp::compare`, wrapped in `Option<K>`
// (None when no such key exists). Never aborts.
procedure {:inline 2} {{impl.fun_next_key}}{{S}}(t: {{Self}}, key: {{K}}) returns (result: $1_option_Option{{SK}}) {
    var k: {{K}};
    if ((exists k_p: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k_p)} $IsValid'{{instance.0.suffix}}'(k_p)
            && {{impl.fun_spec_has_key}}{{S}}(t, k_p)
            && $1_cmp_$compare'{{instance.0.suffix}}'(k_p, key) == $1_cmp_Ordering_Greater())) {
        assume $IsValid'{{instance.0.suffix}}'(k);
        assume {{impl.fun_spec_has_key}}{{S}}(t, k);
        assume $1_cmp_$compare'{{instance.0.suffix}}'(k, key) == $1_cmp_Ordering_Greater();
        // k is the *smallest* such successor: any other in-map k_p that is also
        // > key must satisfy k < k_p.
        assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
            !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
            {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(other, key) == $1_cmp_Ordering_Greater() ==>
                $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Less());
        result := $1_option_Option{{SK}}_Some(k);
    } else {
        result := $1_option_Option{{SK}}_None();
    }
}
{%- endif %}

{%- if impl.fun_keys != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// All keys in the map as a `vector<K>`. Never aborts. The membership biconditional
// is split into two implications so each direction gets a legal trigger
// ($ContainsVec cannot be a pattern: its inline body is an `exists`).
procedure {:inline 2} {{impl.fun_keys}}{{S}}(t: ({{Self}})) returns (result: Vec ({{K}})) {
    assume LenVec(result) == LenTable(t);
    assume (forall i: int :: {ReadVec(result, i)} InRangeVec(result, i) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, ReadVec(result, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, k) ==> $ContainsVec'{{instance.0.suffix}}'(result, k));
    assume (forall i: int, j: int :: {ReadVec(result, i), ReadVec(result, j)}
        InRangeVec(result, i) ==> InRangeVec(result, j) ==> i != j ==>
        !$IsEqual'{{instance.0.suffix}}'(ReadVec(result, i), ReadVec(result, j)));
}
{%- endif %}

{%- if impl.fun_to_ordered_map != "" and not instance.1.is_bv %}
// Convert to another intrinsic-map type with identical contents. Never aborts.
// Both map types share the `Table int V` representation and the per-K `$EncodeKey`,
// so the conversion is the identity at this level.
procedure {:inline 2} {{impl.fun_to_ordered_map}}{{S}}(t: ({{Self}})) returns (result: ({{Self}})) {
    result := t;
}
{%- endif %}

{%- if impl.fun_values != "" and not instance.1.is_bv %}
// All values in the map as a `vector<V>`. Never aborts. Only length is promised;
// callers needing value/key correspondence should use `to_vec_pair`.
procedure {:inline 2} {{impl.fun_values}}{{S}}(t: ({{Self}})) returns (result: Vec ({{V}})) {
    assume LenVec(result) == LenTable(t);
}
{%- endif %}

{%- if impl.fun_to_vec_pair != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// Consume the map, returning keys and values as parallel vectors. Never aborts.
// Key-vector membership mirrors `fun_keys` (split biconditional, see there);
// value-vector is length-only.
procedure {:inline 2} {{impl.fun_to_vec_pair}}{{S}}(t: ({{Self}})) returns (result_keys: Vec ({{K}}), result_values: Vec ({{V}})) {
    assume LenVec(result_keys) == LenTable(t);
    assume LenVec(result_values) == LenTable(t);
    assume (forall i: int :: {ReadVec(result_keys, i)} InRangeVec(result_keys, i) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, ReadVec(result_keys, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, k) ==> $ContainsVec'{{instance.0.suffix}}'(result_keys, k));
    assume (forall i: int, j: int :: {ReadVec(result_keys, i), ReadVec(result_keys, j)}
        InRangeVec(result_keys, i) ==> InRangeVec(result_keys, j) ==> i != j ==>
        !$IsEqual'{{instance.0.suffix}}'(ReadVec(result_keys, i), ReadVec(result_keys, j)));
}
{%- endif %}

{%- if impl.fun_new_from != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and not instance.1.is_bv %}
// Build a map from parallel key/value vectors. Aborts when lengths differ or any
// key appears more than once.
procedure {:inline 2} {{impl.fun_new_from}}{{S}}(keys_arg: Vec ({{K}}), values_arg: Vec ({{V}})) returns (result: ({{Self}})) {
    if (LenVec(keys_arg) != LenVec(values_arg)) {
        call $ExecFailureAbort();
        return;
    }
    if ((exists i: int, j: int :: {ReadVec(keys_arg, i), ReadVec(keys_arg, j)} i >= 0 && i < LenVec(keys_arg) && j >= 0 && j < LenVec(keys_arg)
            && i != j && $IsEqual'{{instance.0.suffix}}'(ReadVec(keys_arg, i), ReadVec(keys_arg, j)))) {
        call $ExecFailureAbort();
        return;
    }
    assume LenTable(result) == LenVec(keys_arg);
    assume (forall i: int :: {ReadVec(keys_arg, i)} InRangeVec(keys_arg, i) ==>
        {{impl.fun_spec_has_key}}{{S}}(result, ReadVec(keys_arg, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(result, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}(result, k) ==> $ContainsVec'{{instance.0.suffix}}'(keys_arg, k));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        {{impl.fun_spec_get}}{{S}}(result, ReadVec(keys_arg, i)) == ReadVec(values_arg, i));
}
{%- endif %}

{%- if impl.fun_add_all != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and not instance.1.is_bv %}
// Add multiple key/value pairs. Aborts on length mismatch, any input key already
// present, or duplicates among input keys. Values at input keys are set from
// `values_arg`; values at pre-existing keys are unconstrained (a `forall k ::
// spec_get(t, k) == spec_get(t_new, k)` shape would violate trigger discipline).
procedure {:inline 2} {{impl.fun_add_all}}{{S}}(m: $Mutation ({{Self}}), keys_arg: Vec ({{K}}), values_arg: Vec ({{V}}))
returns (m': $Mutation ({{Self}})) {
    var t, t_new: {{Self}};
    t := $Dereference(m);
    if (LenVec(keys_arg) != LenVec(values_arg)) {
        call $ExecFailureAbort();
        return;
    }
    if ((exists i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) && {{impl.fun_spec_has_key}}{{S}}(t, ReadVec(keys_arg, i)))) {
        call $ExecFailureAbort();
        return;
    }
    if ((exists i: int, j: int :: {ReadVec(keys_arg, i), ReadVec(keys_arg, j)} i >= 0 && i < LenVec(keys_arg) && j >= 0 && j < LenVec(keys_arg)
            && i != j && $IsEqual'{{instance.0.suffix}}'(ReadVec(keys_arg, i), ReadVec(keys_arg, j)))) {
        call $ExecFailureAbort();
        return;
    }
    assume LenTable(t_new) == LenTable(t) + LenVec(keys_arg);
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) ==> {{impl.fun_spec_has_key}}{{S}}(t_new, k)));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        {{impl.fun_spec_has_key}}{{S}}(t_new, ReadVec(keys_arg, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t_new, k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k)
         || (exists i: int :: i >= 0 && i < LenVec(keys_arg)
             && $IsEqual'{{instance.0.suffix}}'(k, ReadVec(keys_arg, i)))));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        {{impl.fun_spec_get}}{{S}}(t_new, ReadVec(keys_arg, i)) == ReadVec(values_arg, i));
    m' := $UpdateMutation(m, t_new);
}
{%- endif %}

{%- if impl.fun_upsert_all != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and not instance.1.is_bv %}
// Upsert multiple key/value pairs. Aborts only on length mismatch. Post-state
// key set = pre-existing keys ∪ input keys (no phantoms). Value assignment
// respects last-write-wins under duplicate input keys: for each index `i`, if no
// later index `j` carries the same key, `spec_get(t_new, keys[i]) == values[i]`.
// Values at pre-existing keys not in `keys_arg` are unconstrained (would need
// trigger-unsafe `forall k :: spec_get` shape).
procedure {:inline 2} {{impl.fun_upsert_all}}{{S}}(m: $Mutation ({{Self}}), keys_arg: Vec ({{K}}), values_arg: Vec ({{V}}))
returns (m': $Mutation ({{Self}})) {
    var t, t_new: {{Self}};
    t := $Dereference(m);
    if (LenVec(keys_arg) != LenVec(values_arg)) {
        call $ExecFailureAbort();
        return;
    }
    // Exact length needs a distinct-count over `keys_arg`; `>= LenVec(keys_arg)`
    // would be unsound under duplicate input keys, so only these bounds hold.
    assume LenTable(t_new) >= LenTable(t);
    assume LenTable(t_new) <= LenTable(t) + LenVec(keys_arg);
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) ==> {{impl.fun_spec_has_key}}{{S}}(t_new, k)));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        {{impl.fun_spec_has_key}}{{S}}(t_new, ReadVec(keys_arg, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t_new, k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k)
         || (exists i: int :: i >= 0 && i < LenVec(keys_arg)
             && $IsEqual'{{instance.0.suffix}}'(k, ReadVec(keys_arg, i)))));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        (forall j: int :: {ReadVec(keys_arg, j)} j > i && j < LenVec(keys_arg) ==>
            !$IsEqual'{{instance.0.suffix}}'(ReadVec(keys_arg, j), ReadVec(keys_arg, i))) ==>
        {{impl.fun_spec_get}}{{S}}(t_new, ReadVec(keys_arg, i)) == ReadVec(values_arg, i));
    m' := $UpdateMutation(m, t_new);
}
{%- endif %}

{%- if impl.fun_append != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// Merge `other` into `self`, overwriting on overlapping keys. Never aborts.
// Length: bounded on both sides — exact size depends on overlap, which we don't model.
// Under-specified: value semantics (which of `t`/`other` wins per key) is not
// modeled — would require `forall k :: spec_get(t_new, k) == ...` shape.
procedure {:inline 2} {{impl.fun_append}}{{S}}(m: $Mutation ({{Self}}), other: ({{Self}}))
returns (m': $Mutation ({{Self}})) {
    var t, t_new: {{Self}};
    t := $Dereference(m);
    assume LenTable(t_new) >= LenTable(t);
    assume LenTable(t_new) >= LenTable(other);
    assume LenTable(t_new) <= LenTable(t) + LenTable(other);
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(other, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t_new, k) <==>
            ({{impl.fun_spec_has_key}}{{S}}(t, k) || {{impl.fun_spec_has_key}}{{S}}(other, k))));
    m' := $UpdateMutation(m, t_new);
}
{%- endif %}

{%- if impl.fun_append_disjoint != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// Merge `other` into `self`. Aborts if any key in `other` is already in `self`.
// Under-specified: values from both maps survive in `t_new` (disjoint) but the
// per-key `spec_get(t_new, k) == spec_get(t\|other, k)` mapping is not modeled.
procedure {:inline 2} {{impl.fun_append_disjoint}}{{S}}(m: $Mutation ({{Self}}), other: ({{Self}}))
returns (m': $Mutation ({{Self}})) {
    var t, t_new: {{Self}};
    t := $Dereference(m);
    if ((exists k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k), {{impl.fun_spec_has_key}}{{S}}(other, k)} $IsValid'{{instance.0.suffix}}'(k)
            && {{impl.fun_spec_has_key}}{{S}}(t, k) && {{impl.fun_spec_has_key}}{{S}}(other, k))) {
        call $ExecFailureAbort();
        return;
    }
    assume LenTable(t_new) == LenTable(t) + LenTable(other);
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(other, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t_new, k) <==>
            ({{impl.fun_spec_has_key}}{{S}}(t, k) || {{impl.fun_spec_has_key}}{{S}}(other, k))));
    m' := $UpdateMutation(m, t_new);
}
{%- endif %}

{%- if impl.fun_trim != "" and not instance.1.is_bv %}
// Split the map at `at`. Retains [0, at) in self, returns [at, len). Aborts if
// `at > len(self)`. The key sets form a set-level partition of the original keys;
// which keys land on which side (the `at` smallest stay) is ordering-dependent
// and not modeled.
procedure {:inline 2} {{impl.fun_trim}}{{S}}(m: $Mutation ({{Self}}), at: int)
returns (result: ({{Self}}), m': $Mutation ({{Self}})) {
    var t, t_new: {{Self}};
    t := $Dereference(m);
    if (at > LenTable(t)) {
        call $ExecFailureAbort();
        return;
    }
    assume LenTable(t_new) == at;
    assume LenTable(result) == LenTable(t) - at;
{%- if impl.fun_spec_has_key != "" %}
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t_new, k) ==> {{impl.fun_spec_has_key}}{{S}}(t, k)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(result, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(result, k) ==> {{impl.fun_spec_has_key}}{{S}}(t, k)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) ==>
            ({{impl.fun_spec_has_key}}{{S}}(t_new, k) || {{impl.fun_spec_has_key}}{{S}}(result, k))));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k), {{impl.fun_spec_has_key}}{{S}}(result, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        !({{impl.fun_spec_has_key}}{{S}}(t_new, k) && {{impl.fun_spec_has_key}}{{S}}(result, k)));
{%- endif %}
    m' := $UpdateMutation(m, t_new);
}
{%- endif %}

{%- if impl.fun_replace_key_inplace != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// Rename `old_key` to `new_key`, keeping the entry's position. Aborts when
// `old_key` is absent, and nondeterministically to model the Move-level abort on
// `new_key` violating the surrounding `cmp::compare<K>` order — callers must
// establish that precondition to conclude success. On success, membership is
// modeled: `old_key` is gone (if distinct from `new_key`), `new_key` is present,
// all other keys unchanged. Value at `new_key` is not modeled (would need
// trigger-unsafe `forall k :: spec_get` shape).
procedure {:inline 2} {{impl.fun_replace_key_inplace}}{{S}}(m: $Mutation ({{Self}}), old_key: {{K}}, new_key: {{K}})
returns (m': $Mutation ({{Self}})) {
    var t, t_new: {{Self}};
    var may_abort_on_order: bool;
    t := $Dereference(m);
    if (!{{impl.fun_spec_has_key}}{{S}}(t, old_key)) {
        call $ExecFailureAbort();
        return;
    }
    if ($IsEqual'{{instance.0.suffix}}'(old_key, new_key)) {
        m' := m;
        return;
    }
    if (may_abort_on_order) {
        call $ExecFailureAbort();
        return;
    }
    assume LenTable(t_new) == LenTable(t);
    assume !{{impl.fun_spec_has_key}}{{S}}(t_new, old_key);
    assume {{impl.fun_spec_has_key}}{{S}}(t_new, new_key);
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t_new, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        !$IsEqual'{{instance.0.suffix}}'(k, old_key) ==>
        !$IsEqual'{{instance.0.suffix}}'(k, new_key) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) == {{impl.fun_spec_has_key}}{{S}}(t_new, k)));
    m' := $UpdateMutation(m, t_new);
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

{%- if impl.fun_upsert != "" and not instance.1.is_bv %}
// Insert (k, v) or update v if k already maps. Returns the previous value (if any) as
// `Option<V>`. Never aborts.
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

{%- if impl.fun_del_must_exist != "" %}
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

{%- if impl.fun_remove_or_none != "" and not instance.1.is_bv %}
// Remove the entry at `k` if present. Returns `Some(prev_value)` on hit, `None` on miss.
// Never aborts.
procedure {:inline 2} {{impl.fun_remove_or_none}}{{S}}(m: $Mutation ({{Self}}), k: {{K}})
returns (result: $1_option_Option{{SV}}, m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};
    enc_k := {{ENC}}(k);
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
