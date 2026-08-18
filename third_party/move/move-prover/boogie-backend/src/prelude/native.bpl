{# Copyright (c) The Diem Core Contributors
   SPDX-License-Identifier: Apache-2.0
#}

{# Vectors
   =======
#}

{% macro vector_module(instance) %}
{%- set S = "'" ~ instance.suffix ~ "'" -%}
{%- set T = instance.name -%}
{%- if options.native_equality and instance.has_native_equality -%}
{# Whole vector has native equality: extensional theory AND canonically
   represented elements (ghost-bearing element types must compare
   element-wise so ghosts are excluded). #}
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
{%- set Table = "Table int (" ~ V ~ ")" -%}
{%- set S = "'" ~ instance.0.suffix ~ "_" ~ instance.1.suffix ~ "'" -%}
{%- set SK = "'" ~ instance.0.suffix ~ "'" -%}
{%- set SV = "'" ~ instance.1.suffix ~ "'" -%}
{%- set ENC = "$EncodeKey'" ~ instance.0.suffix ~ "'" -%}
{#- Enumeration-view names and emission guard, hoisted so the front/back
    templates can state rank facts about their witness key. -#}
{%- set EKA = "$EnumKeyAt'" ~ Type ~ S ~ "'" -%}
{%- set ERK = "$EnumRank'" ~ Type ~ S ~ "'" -%}
{%- set EWF = "$TableWf'" ~ Type ~ S ~ "'" -%}
{%- set HAS_ENUM = impl.fun_spec_key_at != "" and impl.fun_spec_rank != "" and not instance.0.is_bv and not instance.1.is_bv -%}

{%- if impl.has_ghost_carrier %}
{# Ghost carrier: the map value wraps the table so declared ghost fields have
   constructor arguments to live in; `Self` is the carrier (named per
   `boogie_struct_name`). Template plumbing (all expand to nothing / identity
   for ghost-less maps, keeping their output byte-identical):
   - `U`: content unwrap suffix applied to map-typed values at raw table ops;
   - `W1`/`W2`: wrap a table expression into a carrier with the `$gbN` ghost
     values of the enclosing rebuild site;
   - `GH`/`GBD`: havoc statements / local declarations for the `$gbN`s
     (mutation gives the map fresh, unconstrained ghost state; validity of
     outstanding iterators becomes UNDETERMINED — never provable, but not
     provably false either — so uses gated on validity fail closed until a
     new iterator re-establishes it);
   - `TND`: local declarations for blocks using a fresh raw table `t_new`
     (`t_new` stays a raw table — it is wrapped at each use site). #}
{%- set Self = impl.struct_base ~ S -%}
{%- set U = "->$$t" -%}
{%- set W1 = Self ~ "(" -%}
{%- set W2 = impl.gb_args ~ ")" -%}
{%- set SW1 = Self ~ "(" -%}
{%- set SW2 = impl.ghost_preserve_args ~ ")" -%}
{%- set SZ2 = impl.ghost_zero_args ~ ")" -%}
{%- set GH = impl.gb_havoc -%}
{%- set GBD = impl.gb_decls -%}
{%- set TND = "var t: " ~ Self ~ "; var t_new: " ~ Table ~ ";" ~ GBD -%}
{#- The raw-table selector is `$$t`: Move field selectors render as `$<name>`,
    so a `$$`-prefixed name cannot collide with any declared ghost field. -#}
datatype {{Self}} {
    {{Self}}($$t: {{Table}}{%- for g in impl.ghost_args %}, {{g.sel}}: {{g.ty}}{%- endfor %})
}
{%- else %}
{%- set Self = Table -%}
{%- set U = "" -%}
{%- set W1 = "" -%}
{%- set W2 = "" -%}
{%- set SW1 = "" -%}
{%- set SW2 = "" -%}
{%- set SZ2 = "" -%}
{%- set GH = "" -%}
{%- set GBD = "" -%}
{%- set TND = "var t, t_new: " ~ Self ~ ";" -%}
{%- endif %}

{#- Content accessors: value-level functions compare/validate the table
    content; carrier ghosts are excluded from equality (Move equality is the
    quotient over runtime state) and `num` ghosts add no validity constraint. -#}
{%- if impl.has_ghost_carrier -%}
{%- set c1 = "t1->$$t" -%}{%- set c2 = "t2->$$t" -%}{%- set c = "t->$$t" -%}
{%- else -%}
{%- set c1 = "t1" -%}{%- set c2 = "t2" -%}{%- set c = "t" -%}
{%- endif -%}
{#- A ghost-bearing VALUE type must compare with `$IsEqual` rather than raw
    `==`, which would include its ghost constructor arguments. (Carrier
    ghosts on the map itself are excluded via the `c1`/`c2` unwrap above.) -#}
{%- if instance.1.has_ghost -%}
{%- set VEQ = "$IsEqual'" ~ instance.1.suffix ~ "'(GetTable(" ~ c1 ~ ", k), GetTable(" ~ c2 ~ ", k))" -%}
{%- else -%}
{%- set VEQ = "GetTable(" ~ c1 ~ ", k) == GetTable(" ~ c2 ~ ", k)" -%}
{%- endif -%}
{%- if options.native_equality and not impl.has_ghost_carrier and not instance.1.has_ghost -%}
function $IsEqual'{{Type}}{{S}}'(t1: {{Self}}, t2: {{Self}}): bool {
    t1 == t2
}
{%- else -%}
function $IsEqual'{{Type}}{{S}}'(t1: {{Self}}, t2: {{Self}}): bool {
    LenTable({{c1}}) == LenTable({{c2}}) &&
    (forall k: int :: ContainsTable({{c1}}, k) <==> ContainsTable({{c2}}, k)) &&
    (forall k: int :: ContainsTable({{c1}}, k) ==> {{VEQ}}) &&
    (forall k: int :: ContainsTable({{c2}}, k) ==> {{VEQ}})
}
{%- endif %}

// Not inlined.
function $IsValid'{{Type}}{{S}}'(t: {{Self}}): bool {
    $IsValid'u64'(LenTable({{c}})) &&
{%- if HAS_ENUM %}
    {{EWF}}({{c}}) &&
{%- endif %}
    (forall i: int:: ContainsTable({{c}}, i) ==> $IsValid{{SV}}(GetTable({{c}}, i)))
}

{%- if impl.fun_new != "" %}
procedure {:inline 2} {{impl.fun_new}}{{S}}() returns (v: {{Self}}) {{"{"}}{{GBD}}
    {{GH}}v := {{W1}}EmptyTable(){{W2}};
}
{%- endif %}

{%- if impl.fun_new_with_config != "" and not instance.1.is_bv %}
// Empty map with degree configuration. Aborts when a nonzero degree is outside its
// valid range (INNER_MIN_DEGREE=4 / LEAF_MIN_DEGREE=3 / MAX_DEGREE=4096, mirroring
// big_ordered_map constants). ASSUMPTION: the implementation's size-validation abort
// (key/entry serialized size exceeding node limits) is presumed not to fire.
procedure {:inline 2} {{impl.fun_new_with_config}}{{S}}(inner_max_degree: int, leaf_max_degree: int, reuse_slots: bool) returns (v: {{Self}}) {{"{"}}{{GBD}}
    if (inner_max_degree != 0 && (inner_max_degree < 4 || inner_max_degree > 4096)) {
        call $ExecFailureAbort();
        return;
    }
    if (leaf_max_degree != 0 && (leaf_max_degree < 3 || leaf_max_degree > 4096)) {
        call $ExecFailureAbort();
        return;
    }
    {{GH}}v := {{W1}}EmptyTable(){{W2}};
}
{%- endif %}

{%- if impl.fun_destroy_empty != "" %}
procedure {:inline 2} {{impl.fun_destroy_empty}}{{S}}(t: {{Self}}) {
    if (LenTable(t{{U}}) != 0) {
        call $Abort($StdError(1/*INVALID_STATE*/, 102/*ENOT_EMPTY*/));
    }
}
{%- endif %}

{%- if impl.fun_len != "" %}
procedure {:inline 2} {{impl.fun_len}}{{S}}(t: ({{Self}})) returns (l: int) {
    l := LenTable(t{{U}});
}
{%- endif %}

{%- if impl.fun_is_empty != "" %}
procedure {:inline 2} {{impl.fun_is_empty}}{{S}}(t: ({{Self}})) returns (r: bool) {
    r := LenTable(t{{U}}) == 0;
}
{%- endif %}

{%- if impl.fun_has_key != "" %}
procedure {:inline 2} {{impl.fun_has_key}}{{S}}(t: ({{Self}}), k: {{K}}) returns (r: bool) {
    r := ContainsTable(t{{U}}, {{ENC}}(k));
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
    if (ContainsTable(t{{U}}, enc_k)) {
        result := $1_option_Option{{SV}}_Some(GetTable(t{{U}}, enc_k));
    } else {
        result := $1_option_Option{{SV}}_None();
    }
}
{%- endif %}

{%- if impl.fun_borrow_front != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Smallest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_borrow_front}}{{S}}(t: {{Self}}) returns (k: {{K}}, v: {{V}}) {
    if (LenTable(t{{U}}) == 0) {
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
{%- if HAS_ENUM %}
    assume {{EWF}}(t{{U}}) ==> {{ERK}}(t{{U}}, {{ENC}}(k)) == 0;
{%- endif %}
}
{%- endif %}

{%- if impl.fun_borrow_back != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Largest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_borrow_back}}{{S}}(t: {{Self}}) returns (k: {{K}}, v: {{V}}) {
    if (LenTable(t{{U}}) == 0) {
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
{%- if HAS_ENUM %}
    assume {{EWF}}(t{{U}}) ==> {{ERK}}(t{{U}}, {{ENC}}(k)) == LenTable(t{{U}}) - 1;
{%- endif %}
}
{%- endif %}

{%- if impl.fun_front_key != "" and impl.fun_spec_has_key != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Smallest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_front_key}}{{S}}(t: {{Self}}) returns (k: {{K}}) {
    if (LenTable(t{{U}}) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Less());
{%- if HAS_ENUM %}
    assume {{EWF}}(t{{U}}) ==> {{ERK}}(t{{U}}, {{ENC}}(k)) == 0;
{%- endif %}
}
{%- endif %}

{%- if impl.fun_back_key != "" and impl.fun_spec_has_key != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Largest key under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_back_key}}{{S}}(t: {{Self}}) returns (k: {{K}}) {
    if (LenTable(t{{U}}) == 0) {
        call $ExecFailureAbort();
        return;
    }
    assume $IsValid'{{instance.0.suffix}}'(k);
    assume {{impl.fun_spec_has_key}}{{S}}(t, k);
    assume (forall other: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, other)} $IsValid'{{instance.0.suffix}}'(other) ==>
        !$IsEqual'{{instance.0.suffix}}'(other, k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, other) ==>
            $1_cmp_$compare'{{instance.0.suffix}}'(k, other) == $1_cmp_Ordering_Greater());
{%- if HAS_ENUM %}
    assume {{EWF}}(t{{U}}) ==> {{ERK}}(t{{U}}, {{ENC}}(k)) == LenTable(t{{U}}) - 1;
{%- endif %}
}
{%- endif %}

{%- if impl.fun_pop_front != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Remove and return the smallest entry under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_pop_front}}{{S}}(m: $Mutation ({{Self}}))
returns (k: {{K}}, v: {{V}}, m': $Mutation ({{Self}})) {
    var t: {{Self}};{{GBD}}
    t := $Dereference(m);
    if (LenTable(t{{U}}) == 0) {
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
{%- if HAS_ENUM %}
    assume {{EWF}}(t{{U}}) ==> {{ERK}}(t{{U}}, {{ENC}}(k)) == 0;
{%- endif %}
    {{GH}}m' := $UpdateMutation(m, {{W1}}RemoveTable(t{{U}}, {{ENC}}(k)){{W2}});
}
{%- endif %}

{%- if impl.fun_pop_back != "" and impl.fun_spec_has_key != "" and impl.fun_spec_get != "" and instance.0.cmp_available and not instance.1.is_bv %}
// Remove and return the largest entry under `cmp::compare` ordering. Aborts when the map is empty.
procedure {:inline 2} {{impl.fun_pop_back}}{{S}}(m: $Mutation ({{Self}}))
returns (k: {{K}}, v: {{V}}, m': $Mutation ({{Self}})) {
    var t: {{Self}};{{GBD}}
    t := $Dereference(m);
    if (LenTable(t{{U}}) == 0) {
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
{%- if HAS_ENUM %}
    assume {{EWF}}(t{{U}}) ==> {{ERK}}(t{{U}}, {{ENC}}(k)) == LenTable(t{{U}}) - 1;
{%- endif %}
    {{GH}}m' := $UpdateMutation(m, {{W1}}RemoveTable(t{{U}}, {{ENC}}(k)){{W2}});
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
{%- if HAS_ENUM %}
        // The predecessor of a contained key sits one position earlier.
        assume {{EWF}}(t{{U}}) && ContainsTable(t{{U}}, {{ENC}}(key)) ==>
            {{ERK}}(t{{U}}, {{ENC}}(k)) == {{ERK}}(t{{U}}, {{ENC}}(key)) - 1;
{%- endif %}
        result := $1_option_Option{{SK}}_Some(k);
    } else {
{%- if HAS_ENUM %}
        // No predecessor means `key` occupies the first position.
        assume {{EWF}}(t{{U}}) && ContainsTable(t{{U}}, {{ENC}}(key)) ==>
            {{ERK}}(t{{U}}, {{ENC}}(key)) == 0;
{%- endif %}
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
{%- if HAS_ENUM %}
        // The successor of a contained key sits one position later. Minimality
        // above is stated through `cmp::compare` and does not by itself reach
        // the enumeration, so a scan that steps by `next_key` could not
        // otherwise be indexed by position.
        assume {{EWF}}(t{{U}}) && ContainsTable(t{{U}}, {{ENC}}(key)) ==>
            {{ERK}}(t{{U}}, {{ENC}}(k)) == {{ERK}}(t{{U}}, {{ENC}}(key)) + 1;
{%- endif %}
        result := $1_option_Option{{SK}}_Some(k);
    } else {
{%- if HAS_ENUM %}
        // No successor means `key` occupies the last position.
        assume {{EWF}}(t{{U}}) && ContainsTable(t{{U}}, {{ENC}}(key)) ==>
            {{ERK}}(t{{U}}, {{ENC}}(key)) == LenTable(t{{U}}) - 1;
{%- endif %}
        result := $1_option_Option{{SK}}_None();
    }
}
{%- endif %}

{%- if impl.fun_keys != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// All keys in the map as a `vector<K>`. Never aborts. The membership biconditional
// is split into two implications so each direction gets a legal trigger
// ($ContainsVec cannot be a pattern: its inline body is an `exists`).
procedure {:inline 2} {{impl.fun_keys}}{{S}}(t: ({{Self}})) returns (result: Vec ({{K}})) {
    assume $IsValid'vec'{{instance.0.suffix}}''(result);
    assume LenVec(result) == LenTable(t{{U}});
    assume (forall i: int :: {ReadVec(result, i)} InRangeVec(result, i) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, ReadVec(result, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, k) ==> $ContainsVec'{{instance.0.suffix}}'(result, k));
    assume (forall i: int, j: int :: {ReadVec(result, i), ReadVec(result, j)}
        InRangeVec(result, i) ==> InRangeVec(result, j) ==> i != j ==>
        !$IsEqual'{{instance.0.suffix}}'(ReadVec(result, i), ReadVec(result, j)));
{%- if instance.0.cmp_available %}
    // Keys are returned in ascending `cmp::compare` order.
    assume (forall i: int, j: int :: {ReadVec(result, i), ReadVec(result, j)}
        InRangeVec(result, i) ==> InRangeVec(result, j) ==> i < j ==>
        $1_cmp_$compare'{{instance.0.suffix}}'(ReadVec(result, i), ReadVec(result, j)) == $1_cmp_Ordering_Less());
{%- endif %}
{%- if HAS_ENUM %}
    // The returned vector and the enumeration agree position by position:
    // both list the same key set in ascending order. Stated because two
    // independently described ascending listings are not connected
    // otherwise — the solver would have to argue their uniqueness — which is
    // what leaves `keys()`-based code unable to share facts with
    // position-based code.
    assume (forall i: int :: {ReadVec(result, i)} InRangeVec(result, i) ==>
        $IsEqual'{{instance.0.suffix}}'(ReadVec(result, i), {{EKA}}(t{{U}}, i)));
{%- endif %}
}
{%- endif %}

{%- if impl.fun_to_ordered_map != "" and not instance.1.is_bv %}
// Convert to another intrinsic-map type with identical contents. Never aborts.
// Both map types share the `Table int V` representation and the per-K `$EncodeKey`,
// so the conversion is the identity at this level.
procedure {:inline 2} {{impl.fun_to_ordered_map}}{{S}}(t: ({{Self}})) returns (result: ({{Table}})) {
    result := t{{U}};
}
{%- endif %}

{%- if impl.fun_values != "" and not instance.1.is_bv %}
// All values in the map as a `vector<V>`. Never aborts. Only length is promised;
// callers needing value/key correspondence should use `to_vec_pair`.
procedure {:inline 2} {{impl.fun_values}}{{S}}(t: ({{Self}})) returns (result: Vec ({{V}})) {
    assume LenVec(result) == LenTable(t{{U}});
}
{%- endif %}

{%- if impl.fun_to_vec_pair != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// Consume the map, returning keys and values as parallel vectors. Never aborts.
// Key-vector membership mirrors `fun_keys` (split biconditional, see there);
// value-vector is length-only.
procedure {:inline 2} {{impl.fun_to_vec_pair}}{{S}}(t: ({{Self}})) returns (result_keys: Vec ({{K}}), result_values: Vec ({{V}})) {
    assume $IsValid'vec'{{instance.0.suffix}}''(result_keys);
    assume $IsValid'vec'{{instance.1.suffix}}''(result_values);
    assume LenVec(result_keys) == LenTable(t{{U}});
    assume LenVec(result_values) == LenTable(t{{U}});
    assume (forall i: int :: {ReadVec(result_keys, i)} InRangeVec(result_keys, i) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, ReadVec(result_keys, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}(t, k) ==> $ContainsVec'{{instance.0.suffix}}'(result_keys, k));
    assume (forall i: int, j: int :: {ReadVec(result_keys, i), ReadVec(result_keys, j)}
        InRangeVec(result_keys, i) ==> InRangeVec(result_keys, j) ==> i != j ==>
        !$IsEqual'{{instance.0.suffix}}'(ReadVec(result_keys, i), ReadVec(result_keys, j)));
{%- if instance.0.cmp_available %}
    // Keys are returned in ascending `cmp::compare` order.
    assume (forall i: int, j: int :: {ReadVec(result_keys, i), ReadVec(result_keys, j)}
        InRangeVec(result_keys, i) ==> InRangeVec(result_keys, j) ==> i < j ==>
        $1_cmp_$compare'{{instance.0.suffix}}'(ReadVec(result_keys, i), ReadVec(result_keys, j)) == $1_cmp_Ordering_Less());
{%- endif %}
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
    assume LenTable(result{{U}}) == LenVec(keys_arg);
{%- if HAS_ENUM %}
    assume {{EWF}}(result{{U}});
{%- endif %}
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
    {{TND}}
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
    assume LenTable(t_new) == LenTable(t{{U}}) + LenVec(keys_arg);
{%- if HAS_ENUM %}
    assume {{EWF}}(t_new);
{%- endif %}
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) ==> {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, ReadVec(keys_arg, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k)
         || (exists i: int :: i >= 0 && i < LenVec(keys_arg)
             && $IsEqual'{{instance.0.suffix}}'(k, ReadVec(keys_arg, i)))));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        {{impl.fun_spec_get}}{{S}}({{W1}}t_new{{W2}}, ReadVec(keys_arg, i)) == ReadVec(values_arg, i));
    {{GH}}m' := $UpdateMutation(m, {{W1}}t_new{{W2}});
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
    {{TND}}
    t := $Dereference(m);
    if (LenVec(keys_arg) != LenVec(values_arg)) {
        call $ExecFailureAbort();
        return;
    }
    // Exact length needs a distinct-count over `keys_arg`; `>= LenVec(keys_arg)`
    // would be unsound under duplicate input keys, so only these bounds hold.
    assume LenTable(t_new) >= LenTable(t{{U}});
    assume LenTable(t_new) <= LenTable(t{{U}}) + LenVec(keys_arg);
{%- if HAS_ENUM %}
    assume {{EWF}}(t_new);
{%- endif %}
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) ==> {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, ReadVec(keys_arg, i)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k)
         || (exists i: int :: i >= 0 && i < LenVec(keys_arg)
             && $IsEqual'{{instance.0.suffix}}'(k, ReadVec(keys_arg, i)))));
    assume (forall i: int :: {ReadVec(keys_arg, i)} i >= 0 && i < LenVec(keys_arg) ==>
        (forall j: int :: {ReadVec(keys_arg, j)} j > i && j < LenVec(keys_arg) ==>
            !$IsEqual'{{instance.0.suffix}}'(ReadVec(keys_arg, j), ReadVec(keys_arg, i))) ==>
        {{impl.fun_spec_get}}{{S}}({{W1}}t_new{{W2}}, ReadVec(keys_arg, i)) == ReadVec(values_arg, i));
    {{GH}}m' := $UpdateMutation(m, {{W1}}t_new{{W2}});
}
{%- endif %}

{%- if impl.fun_append != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// Merge `other` into `self`, overwriting on overlapping keys. Never aborts.
// Length: bounded on both sides — exact size depends on overlap, which we don't model.
// Under-specified: value semantics (which of `t`/`other` wins per key) is not
// modeled — would require `forall k :: spec_get(t_new, k) == ...` shape.
procedure {:inline 2} {{impl.fun_append}}{{S}}(m: $Mutation ({{Self}}), other: ({{Self}}))
returns (m': $Mutation ({{Self}})) {
    {{TND}}
    t := $Dereference(m);
    assume LenTable(t_new) >= LenTable(t{{U}});
    assume LenTable(t_new) >= LenTable(other{{U}});
    assume LenTable(t_new) <= LenTable(t{{U}}) + LenTable(other{{U}});
{%- if HAS_ENUM %}
    assume {{EWF}}(t_new);
{%- endif %}
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(other, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k) <==>
            ({{impl.fun_spec_has_key}}{{S}}(t, k) || {{impl.fun_spec_has_key}}{{S}}(other, k))));
    {{GH}}m' := $UpdateMutation(m, {{W1}}t_new{{W2}});
}
{%- endif %}

{%- if impl.fun_append_disjoint != "" and impl.fun_spec_has_key != "" and not instance.1.is_bv %}
// Merge `other` into `self`. Aborts if any key in `other` is already in `self`.
// Under-specified: values from both maps survive in `t_new` (disjoint) but the
// per-key `spec_get(t_new, k) == spec_get(t\|other, k)` mapping is not modeled.
procedure {:inline 2} {{impl.fun_append_disjoint}}{{S}}(m: $Mutation ({{Self}}), other: ({{Self}}))
returns (m': $Mutation ({{Self}})) {
    {{TND}}
    t := $Dereference(m);
    if ((exists k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k), {{impl.fun_spec_has_key}}{{S}}(other, k)} $IsValid'{{instance.0.suffix}}'(k)
            && {{impl.fun_spec_has_key}}{{S}}(t, k) && {{impl.fun_spec_has_key}}{{S}}(other, k))) {
        call $ExecFailureAbort();
        return;
    }
    assume LenTable(t_new) == LenTable(t{{U}}) + LenTable(other{{U}});
{%- if HAS_ENUM %}
    assume {{EWF}}(t_new);
{%- endif %}
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}(other, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k) <==>
            ({{impl.fun_spec_has_key}}{{S}}(t, k) || {{impl.fun_spec_has_key}}{{S}}(other, k))));
    {{GH}}m' := $UpdateMutation(m, {{W1}}t_new{{W2}});
}
{%- endif %}

{%- if impl.fun_trim != "" and not instance.1.is_bv %}
// Split the map at `at`. Retains [0, at) in self, returns [at, len). Aborts if
// `at > len(self)`. The key sets form a set-level partition of the original keys;
// which keys land on which side (the `at` smallest stay) is ordering-dependent
// and not modeled.
procedure {:inline 2} {{impl.fun_trim}}{{S}}(m: $Mutation ({{Self}}), at: int)
returns (result: ({{Self}}), m': $Mutation ({{Self}})) {
    {{TND}}
    t := $Dereference(m);
    if (at > LenTable(t{{U}})) {
        call $ExecFailureAbort();
        return;
    }
    assume LenTable(t_new) == at;
    assume LenTable(result{{U}}) == LenTable(t{{U}}) - at;
{%- if HAS_ENUM %}
    assume {{EWF}}(t_new);
    assume {{EWF}}(result{{U}});
{%- endif %}
{%- if impl.fun_spec_has_key != "" %}
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k) ==> {{impl.fun_spec_has_key}}{{S}}(t, k)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(result, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(result, k) ==> {{impl.fun_spec_has_key}}{{S}}(t, k)));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) ==>
            ({{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k) || {{impl.fun_spec_has_key}}{{S}}(result, k))));
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k), {{impl.fun_spec_has_key}}{{S}}(result, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        !({{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k) && {{impl.fun_spec_has_key}}{{S}}(result, k)));
{%- endif %}
    {{GH}}m' := $UpdateMutation(m, {{W1}}t_new{{W2}});
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
    {{TND}}
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
    assume LenTable(t_new) == LenTable(t{{U}});
{%- if HAS_ENUM %}
    assume {{EWF}}(t_new);
{%- endif %}
    assume !{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, old_key);
    assume {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, new_key);
    assume (forall k: {{K}} :: {{"{"}}{{impl.fun_spec_has_key}}{{S}}(t, k)} {{"{"}}{{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)} $IsValid'{{instance.0.suffix}}'(k) ==>
        !$IsEqual'{{instance.0.suffix}}'(k, old_key) ==>
        !$IsEqual'{{instance.0.suffix}}'(k, new_key) ==>
        ({{impl.fun_spec_has_key}}{{S}}(t, k) == {{impl.fun_spec_has_key}}{{S}}({{W1}}t_new{{W2}}, k)));
    {{GH}}m' := $UpdateMutation(m, {{W1}}t_new{{W2}});
}
{%- endif %}

{%- if impl.fun_add_no_override != "" %}
procedure {:inline 2} {{impl.fun_add_no_override}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, v: {{V}}) returns (m': $Mutation({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t{{U}}, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 100/*EALREADY_EXISTS*/));
    } else {
{%- if HAS_ENUM %}
        // Insertion keeps the table cardinality-consistent. Stated here rather
        // than as a preservation axiom because an axiom triggered on
        // `AddTable` cannot fire: the function is `{:inline}` and its
        // expansion is a constructor term with two array stores.
        assume {{EWF}}(t{{U}}) ==> {{EWF}}(AddTable(t{{U}}, enc_k, v));
{%- endif %}
        {{GH}}m' := $UpdateMutation(m, {{W1}}AddTable(t{{U}}, enc_k, v){{W2}});
    }
}
{%- endif %}

{%- if impl.fun_add_override_if_exists != "" %}
procedure {:inline 2} {{impl.fun_add_override_if_exists}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, v: {{V}}) returns (m': $Mutation({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t{{U}}, enc_k)) {
        {#- Existing key: an in-place value replacement, not a structural
            mutation — ghosts (the validity slot) are preserved. -#}
        m' := $UpdateMutation(m, {{SW1}}UpdateTable(t{{U}}, enc_k, v){{SW2}});
    } else {
{%- if HAS_ENUM %}
        // Insertion keeps the table cardinality-consistent. Stated here rather
        // than as a preservation axiom because an axiom triggered on
        // `AddTable` cannot fire: the function is `{:inline}` and its
        // expansion is a constructor term with two array stores.
        assume {{EWF}}(t{{U}}) ==> {{EWF}}(AddTable(t{{U}}, enc_k, v));
{%- endif %}
        {{GH}}m' := $UpdateMutation(m, {{W1}}AddTable(t{{U}}, enc_k, v){{W2}});
    }
}
{%- endif %}

{%- if impl.fun_upsert != "" and not instance.1.is_bv %}
// Insert (k, v) or update v if k already maps. Returns the previous value (if any) as
// `Option<V>`. Never aborts.
procedure {:inline 2} {{impl.fun_upsert}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, v: {{V}})
returns (prev_v: $1_option_Option{{SV}}, m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t{{U}}, enc_k)) {
        {#- Existing key: an in-place value replacement, not a structural
            mutation — ghosts (the validity slot) are preserved. -#}
        prev_v := $1_option_Option{{SV}}_Some(GetTable(t{{U}}, enc_k));
        m' := $UpdateMutation(m, {{SW1}}UpdateTable(t{{U}}, enc_k, v){{SW2}});
    } else {
        prev_v := $1_option_Option{{SV}}_None();
{%- if HAS_ENUM %}
        // Insertion keeps the table cardinality-consistent. Stated here rather
        // than as a preservation axiom because an axiom triggered on
        // `AddTable` cannot fire: the function is `{:inline}` and its
        // expansion is a constructor term with two array stores.
        assume {{EWF}}(t{{U}}) ==> {{EWF}}(AddTable(t{{U}}, enc_k, v));
{%- endif %}
        {{GH}}m' := $UpdateMutation(m, {{W1}}AddTable(t{{U}}, enc_k, v){{W2}});
    }
}
{%- endif %}

{%- if impl.fun_del_must_exist != "" %}
procedure {:inline 2} {{impl.fun_del_must_exist}}{{S}}(m: $Mutation ({{Self}}), k: {{K}})
returns (v: {{V}}, m': $Mutation({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t{{U}}, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        v := GetTable(t{{U}}, enc_k);
        {{GH}}m' := $UpdateMutation(m, {{W1}}RemoveTable(t{{U}}, enc_k){{W2}});
    }
}
{%- endif %}

{%- if impl.fun_remove_or_none != "" and not instance.1.is_bv %}
// Remove the entry at `k` if present. Returns `Some(prev_value)` on hit, `None` on miss.
// Never aborts.
procedure {:inline 2} {{impl.fun_remove_or_none}}{{S}}(m: $Mutation ({{Self}}), k: {{K}})
returns (result: $1_option_Option{{SV}}, m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (ContainsTable(t{{U}}, enc_k)) {
        result := $1_option_Option{{SV}}_Some(GetTable(t{{U}}, enc_k));
        {{GH}}m' := $UpdateMutation(m, {{W1}}RemoveTable(t{{U}}, enc_k){{W2}});
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
    var t: {{Self}};{{GBD}}
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t{{U}}, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        k' := k;
        v := GetTable(t{{U}}, enc_k);
        {{GH}}m' := $UpdateMutation(m, {{W1}}RemoveTable(t{{U}}, enc_k){{W2}});
    }
}
{%- endif %}

{%- if impl.fun_borrow != "" %}
procedure {:inline 2} {{impl.fun_borrow}}{{S}}(t: {{Self}}, k: {{K}}) returns (v: {{V}}) {
    var enc_k: int;
    enc_k := {{ENC}}(k);
    if (!ContainsTable(t{{U}}, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        v := GetTable(t{{U}}, {{ENC}}(k));
    }
}
{%- endif %}

{%- if impl.fun_borrow_mut != "" %}
procedure {:inline 2} {{impl.fun_borrow_mut}}{{S}}(m: $Mutation ({{Self}}), k: {{K}})
returns (dst: $Mutation ({{V}}), m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t{{U}}, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        dst := $Mutation(m->l, ExtendVec(m->p, enc_k), GetTable(t{{U}}, enc_k));
        m' := m;
    }
}
{%- endif %}

{%- if impl.fun_borrow_mut_with_default != "" %}
procedure {:inline 2} {{impl.fun_borrow_mut_with_default}}{{S}}(m: $Mutation ({{Self}}), k: {{K}}, default: {{V}})
returns (dst: $Mutation ({{V}}), m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    var t': {{Self}};
    enc_k := {{ENC}}(k);
    t := $Dereference(m);
    if (!ContainsTable(t{{U}}, enc_k)) {
{%- if HAS_ENUM %}
        // Same reason as the other insertion sites: an axiom triggered on
        // `AddTable` cannot fire, so well-formedness has to be carried by an
        // assume or the enumeration axioms stay gated off past this call.
        assume {{EWF}}(t{{U}}) ==> {{EWF}}(AddTable(t{{U}}, enc_k, default));
{%- endif %}
        {{GH}}m' := $UpdateMutation(m, {{W1}}AddTable(t{{U}}, enc_k, default){{W2}});
        t' := $Dereference(m');
        dst := $Mutation(m'->l, ExtendVec(m'->p, enc_k), GetTable(t'{{U}}, enc_k));
    } else {
        dst := $Mutation(m->l, ExtendVec(m->p, enc_k), GetTable(t{{U}}, enc_k));
        m' := m;
    }
}
{%- endif %}

{%- if impl.fun_borrow_with_default != "" %}
procedure {:inline 2} {{impl.fun_borrow_with_default}}{{S}}(t: {{Self}}, k: {{K}}, default: {{V}}) returns (v: {{V}}) {
    var enc_k: int;
    enc_k := {{ENC}}(k);
    if (!ContainsTable(t{{U}}, enc_k)) {
        v := default;
    } else {
        v := GetTable(t{{U}}, {{ENC}}(k));
    }
}
{%- endif %}

{#- The iterator enum is either parameterized by the key (a keyed iterator) or
    unparameterized (a position-based one); its Boogie name follows suit. -#}
{%- if impl.iter_ptr_generic -%}
{%- set ITER = impl.iter_ptr_prefix ~ "'" ~ instance.0.suffix ~ "'" -%}
{%- else -%}
{%- set ITER = impl.iter_ptr_prefix -%}
{%- endif -%}

{%- if impl.fun_iter_borrow_mut != "" %}
// Mutable borrow of the value at the iterator's position. The returned
// mutation's path extends the map's path with the encoded key (a Table index
// edge), so caller write-back goes through UpdateTable instead of concrete map
// internals. The constant-value-size requirement is presumed not to fire (see
// the size presumption in the map's spec).
procedure {:inline 2} {{impl.fun_iter_borrow_mut}}{{S}}(self: {{ITER}}, m: $Mutation ({{Self}}))
returns (dst: $Mutation ({{V}}), m': $Mutation ({{Self}})) {
    var enc_k: int;
    var t: {{Self}};{{GBD}}
    t := $Dereference(m);
{%- if impl.iter_is_index %}
{#- A position-based iterator names no key, so the key comes from the
    enumeration: position i holds key_at(t, i). Aborts on the end iterator or
    an out-of-range position, which is what a stale iterator degrades to. -#}
    if (!(self is {{ITER}}_{{impl.iter_variant}})
        || self->{{impl.iter_key_sel}} < 0
        || self->{{impl.iter_key_sel}} >= LenTable(t{{U}})) {
        call $ExecFailureAbort();
    } else {
{%- if HAS_ENUM %}
        enc_k := {{ENC}}({{EKA}}(t{{U}}, self->{{impl.iter_key_sel}}));
{%- else %}
        // The enumeration is not rendered for this instance (bv-flagged key or
        // value), so a position cannot be resolved to a key at all. Fail here
        // rather than model the borrow: an unconstrained key would leave this
        // procedure able to abort on the containment check below while the
        // map's abort predicate says otherwise, and the two must agree.
        // Unreachable calls still verify, so this only bites real uses.
        assert false;
        enc_k := 0;
{%- endif %}
{%- else %}
{#- A keyed iterator carries its key; aborts on the end iterator or an absent
    key (stale iterator). -#}
    if (!(self is {{ITER}}_{{impl.iter_variant}})) {
        call $ExecFailureAbort();
    } else {
        enc_k := {{ENC}}(self->{{impl.iter_key_sel}});
{%- endif %}
        if (!ContainsTable(t{{U}}, enc_k)) {
            call $ExecFailureAbort();
        } else {
            dst := $Mutation(m->l, ExtendVec(m->p, enc_k), GetTable(t{{U}}, enc_k));
            m' := m;
        }
    }
}
{%- endif %}

{%- if impl.fun_spec_len != "" %}
function {:inline} {{impl.fun_spec_len}}{{S}}(t: ({{Self}})): int {
    LenTable(t{{U}})
}
{%- endif %}

{%- if impl.fun_spec_is_empty != "" %}
function {:inline} {{impl.fun_spec_is_empty}}{{S}}(t: ({{Self}})): bool {
    LenTable(t{{U}}) == 0
}
{%- endif %}

{%- if impl.fun_spec_has_key != "" %}
function {:inline} {{impl.fun_spec_has_key}}{{S}}(t: ({{Self}}), k: {{K}}): bool {
    ContainsTable(t{{U}}, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_set != "" %}
function {:inline} {{impl.fun_spec_set}}{{S}}(t: {{Self}}, k: {{K}}, v: {{V}}): {{Self}} {
    (var enc_k := {{ENC}}(k);
    if (ContainsTable(t{{U}}, enc_k)) then
        {{SW1}}UpdateTable(t{{U}}, enc_k, v){{SW2}}
    else
        {{SW1}}AddTable(t{{U}}, enc_k, v){{SW2}})
}
{%- endif %}

{%- if impl.fun_spec_del != "" %}
function {:inline} {{impl.fun_spec_del}}{{S}}(t: {{Self}}, k: {{K}}): {{Self}} {
    {{SW1}}RemoveTable(t{{U}}, {{ENC}}(k)){{SW2}}
}
{%- endif %}

{%- if impl.fun_spec_get != "" %}
function {:inline} {{impl.fun_spec_get}}{{S}}(t: {{Self}}, k: {{K}}): {{V}} {
    GetTable(t{{U}}, {{ENC}}(k))
}
{%- endif %}

{%- if HAS_ENUM %}
// Enumeration view: `key_at(t, i)` is the i-th smallest key under
// `cmp::compare` (a representative of its `$IsEqual` class), `rank(t, k)` its
// inverse on contained keys. `rank` is defined over the encoded key, so equal
// keys get equal ranks. Names are impl-qualified to avoid collisions when two
// map impls share a (K, V) instance.
function {{EKA}}(t: {{Table}}, i: int): {{K}};
function {{ERK}}(t: {{Table}}, ek: int): int;
function {:inline} {{impl.fun_spec_key_at}}{{S}}(t: {{Self}}, i: int): {{K}} {
    {{EKA}}(t{{U}}, i)
}
function {:inline} {{impl.fun_spec_rank}}{{S}}(t: {{Self}}, k: {{K}}): int {
    {{ERK}}(t{{U}}, {{ENC}}(k))
}
// The axioms below quantify over ALL table values, but the `Table` datatype
// also admits cardinality-inconsistent triples (more contained keys than
// `l`), on which an in-range rank bijection cannot exist. So every statement
// about the enumeration — these axioms and the border rank facts in the
// front/back templates alike — guards on a well-formedness predicate, and no
// rank claim is ever made about a table that may be inconsistent.
//
// Well-formedness holds for the empty table (a ground fact, since
// `EmptyTable` is inline and so this carries no quantifier), is assumed for
// program values through `$IsValid` and at the sites that havoc a fresh
// table, and is carried across value writes and removals by the two axioms
// below. Chained mutations rely on that: the second of two successive pops
// needs the once-removed table to be well formed.
//
// Insertion is carried by an `assume` in each `AddTable` template instead of
// a third axiom here, because an axiom triggered on `AddTable` cannot fire:
// the function is `{:inline}` and its expansion is a constructor term with
// two array stores, which never matches (measured).
//
// Only the RANK side of each mutation needs an axiom. The corresponding
// `key_at` facts — that a removal splices its position out, and an insertion
// splices one in — follow from the rank shift together with the bijection
// below, confirmed by deleting hand-written splice axioms for both and
// watching every position-asserting test still verify.
// Impl-qualified like the enumeration functions, so non-enum programs emit
// nothing.
function {{EWF}}(t: {{Table}}): bool;
axiom {{EWF}}(EmptyTable());
axiom (forall t: {{Table}}, ek: int, v: {{V}} :: {UpdateTable(t, ek, v)}
    {{EWF}}(t) ==> {{EWF}}(UpdateTable(t, ek, v)));
axiom (forall t: {{Table}}, ek: int :: {RemoveTable(t, ek)}
    {{EWF}}(t) && ContainsTable(t, ek) ==> {{EWF}}(RemoveTable(t, ek)));
// Every enumerated position holds a contained key, and rank inverts key_at.
// One axiom rather than two: both facts share the trigger and the guard, so
// splitting them would double instantiations on the commonest pattern.
axiom (forall t: {{Table}}, i: int :: {{"{"}}{{EKA}}(t, i)}
    {{EWF}}(t) && 0 <= i && i < LenTable(t) ==>
        ContainsTable(t, {{ENC}}({{EKA}}(t, i)))
        && {{ERK}}(t, {{ENC}}({{EKA}}(t, i))) == i);
{%- if instance.0.cmp_available %}
// Strictly ascending under `cmp::compare`; emitted only when a cmp
// instantiation for the key type exists in this run.
axiom (forall t: {{Table}}, i: int, j: int :: {{"{"}}{{EKA}}(t, i), {{EKA}}(t, j)}
    {{EWF}}(t) && 0 <= i && i < j && j < LenTable(t) ==>
        $1_cmp_$compare'{{instance.0.suffix}}'({{EKA}}(t, i), {{EKA}}(t, j)) == $1_cmp_Ordering_Less());
{%- endif %}
// A contained key's rank is in range and key_at inverts it (up to $IsEqual).
axiom (forall t: {{Table}}, k: {{K}} :: {{"{"}}{{ERK}}(t, {{ENC}}(k))}
    {{EWF}}(t) && ContainsTable(t, {{ENC}}(k)) ==>
        0 <= {{ERK}}(t, {{ENC}}(k)) && {{ERK}}(t, {{ENC}}(k)) < LenTable(t)
        && $IsEqual'{{instance.0.suffix}}'({{EKA}}(t, {{ERK}}(t, {{ENC}}(k))), k));
// Removal shift: a surviving key's rank drops by one exactly when it was
// above the removed key.
axiom (forall t: {{Table}}, ek: int, ek2: int :: {{"{"}}{{ERK}}(RemoveTable(t, ek), ek2)}
    {{EWF}}(t) && ContainsTable(t, ek) && ContainsTable(t, ek2) && ek2 != ek ==>
        {{ERK}}(RemoveTable(t, ek), ek2) ==
            (if {{ERK}}(t, ek2) < {{ERK}}(t, ek) then {{ERK}}(t, ek2) else {{ERK}}(t, ek2) - 1));

// A value write leaves the key set and the length alone, so it leaves ranks
// alone. Without this, every position fact is lost across `iter_borrow_mut`,
// `iter_modify` and an upsert of an existing key, since all of them write
// back through `UpdateTable` — which is what a loop that mutates values
// while traversing needs to carry its invariant. Preserving `key_at` needs
// no separate rule: rank preservation plus the bijection above pins it.
axiom (forall t: {{Table}}, ek: int, v: {{V}}, ek2: int :: {{"{"}}{{ERK}}(UpdateTable(t, ek, v), ek2)}
    {{EWF}}(t) ==> {{ERK}}(UpdateTable(t, ek, v), ek2) == {{ERK}}(t, ek2));
// Insertion shift, the mirror of removal: an existing key's rank rises by one
// exactly when the inserted key lands at or before it. Quantified over keys
// rather than raw integers, for the same reason the insertion is: only encoded
// keys can enter the support.
axiom (forall t: {{Table}}, k: {{K}}, v: {{V}}, ek2: int ::
        {{"{"}}{{ERK}}(AddTable(t, {{ENC}}(k), v), ek2)}
    {{EWF}}(t) && !ContainsTable(t, {{ENC}}(k)) && ContainsTable(t, ek2) ==>
        {{ERK}}(AddTable(t, {{ENC}}(k), v), ek2) ==
            (if {{ERK}}(t, ek2) < {{ERK}}(AddTable(t, {{ENC}}(k), v), {{ENC}}(k))
             then {{ERK}}(t, ek2) else {{ERK}}(t, ek2) + 1));

{%- endif %}

{%- if impl.fun_spec_new != "" %}
function {:inline} {{impl.fun_spec_new}}{{S}}(): {{Self}} {
    {{SW1}}EmptyTable(){{SZ2}}
}
{%- endif %}

{%- if impl.fun_spec_aborts_destroy_empty != "" %}
function {:inline} {{impl.fun_spec_aborts_destroy_empty}}{{S}}(t: {{Self}}): bool {
    LenTable(t{{U}}) != 0
}
{%- endif %}

{%- if impl.fun_spec_aborts_add != "" %}
function {:inline} {{impl.fun_spec_aborts_add}}{{S}}(t: {{Self}}, k: {{K}}, v: {{V}}): bool {
    ContainsTable(t{{U}}, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_aborts_del != "" %}
function {:inline} {{impl.fun_spec_aborts_del}}{{S}}(t: {{Self}}, k: {{K}}): bool {
    !ContainsTable(t{{U}}, {{ENC}}(k))
}
{%- endif %}

{%- if impl.fun_spec_aborts_borrow != "" %}
function {:inline} {{impl.fun_spec_aborts_borrow}}{{S}}(t: {{Self}}, k: {{K}}): bool {
    !ContainsTable(t{{U}}, {{ENC}}(k))
}
{%- endif %}

{#- Only emitted for a NATIVE-bound predicate (a defined spec fun is emitted
    by regular spec-function translation). The body mirrors the
    `iter_borrow_mut` procedure's abort behavior exactly, per flavor: end
    iterator or absent key when keyed, end iterator or out-of-range position
    when position-based. -#}
{%- if impl.fun_spec_aborts_iter_borrow_mut != "" and impl.fun_iter_borrow_mut != "" %}
function {:inline} {{impl.fun_spec_aborts_iter_borrow_mut}}{{S}}(self: {{ITER}}, t: {{Self}}): bool {
    !(self is {{ITER}}_{{impl.iter_variant}})
{%- if impl.iter_is_index %}
        || self->{{impl.iter_key_sel}} < 0
        || self->{{impl.iter_key_sel}} >= LenTable(t{{U}})
{%- else %}
        || !ContainsTable(t{{U}}, {{ENC}}(self->{{impl.iter_key_sel}}))
{%- endif %}
}
{%- endif %}

{#- Iterator-validity predicates over the hidden `$$validity` slots: an
    iterator is valid iff its slot matches the map's current one (structural
    mutations havoc the map's slot; see the carrier plumbing above), and two
    map states preserve iterators iff their slots agree. -#}
{%- if impl.fun_spec_iter_valid != "" and impl.has_ghost_carrier %}
function {:inline} {{impl.fun_spec_iter_valid}}{{S}}(it: {{impl.iter_valid_prefix}}{% if impl.iter_valid_generic %}'{{instance.0.suffix}}'{% endif %}, t: {{Self}}): bool {
    it->$$validity == t->$$validity
}
{%- endif %}

{%- if impl.fun_spec_leaf_iter_valid != "" and impl.has_ghost_carrier %}
function {:inline} {{impl.fun_spec_leaf_iter_valid}}{{S}}(it: {{impl.leaf_iter_valid_prefix}}{% if impl.leaf_iter_valid_generic %}'{{instance.0.suffix}}'{% endif %}, t: {{Self}}): bool {
    it->$$validity == t->$$validity
}
{%- endif %}

{%- if impl.fun_spec_iter_preserved != "" and impl.has_ghost_carrier %}
function {:inline} {{impl.fun_spec_iter_preserved}}{{S}}(t1: {{Self}}, t2: {{Self}}): bool {
    t1->$$validity == t2->$$validity
}
{%- endif %}

{%- if impl.fun_spec_leaf_offset != "" %}
// Position of a leaf walker in the enumeration. Left uninterpreted: unlike the
// validity predicates there is nothing in the table representation to read it
// from, so the map's own spec supplies the meaning — where the walk starts, how
// each step advances it, and that it reaches the map's length at the end.
function {{impl.fun_spec_leaf_offset}}{{S}}(it: {{impl.leaf_offset_prefix}}{% if impl.leaf_offset_generic %}'{{instance.0.suffix}}'{% endif %}, t: {{Self}}): int;
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
