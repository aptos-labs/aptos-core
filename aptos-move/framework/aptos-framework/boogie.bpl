
// ** Expanded prelude

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Basic theory for vectors using arrays. This version of vectors is not extensional.

datatype Vec<T> {
    Vec(v: [int]T, l: int)
}

function {:builtin "MapConst"} MapConstVec<T>(T): [int]T;
function DefaultVecElem<T>(): T;
function {:inline} DefaultVecMap<T>(): [int]T { MapConstVec(DefaultVecElem()) }

function {:inline} EmptyVec<T>(): Vec T {
    Vec(DefaultVecMap(), 0)
}

function {:inline} MakeVec1<T>(v: T): Vec T {
    Vec(DefaultVecMap()[0 := v], 1)
}

function {:inline} MakeVec2<T>(v1: T, v2: T): Vec T {
    Vec(DefaultVecMap()[0 := v1][1 := v2], 2)
}

function {:inline} MakeVec3<T>(v1: T, v2: T, v3: T): Vec T {
    Vec(DefaultVecMap()[0 := v1][1 := v2][2 := v3], 3)
}

function {:inline} MakeVec4<T>(v1: T, v2: T, v3: T, v4: T): Vec T {
    Vec(DefaultVecMap()[0 := v1][1 := v2][2 := v3][3 := v4], 4)
}

function {:inline} ExtendVec<T>(v: Vec T, elem: T): Vec T {
    (var l := v->l;
    Vec(v->v[l := elem], l + 1))
}

function {:inline} ReadVec<T>(v: Vec T, i: int): T {
    v->v[i]
}

function {:inline} LenVec<T>(v: Vec T): int {
    v->l
}

function {:inline} IsEmptyVec<T>(v: Vec T): bool {
    v->l == 0
}

function {:inline} RemoveVec<T>(v: Vec T): Vec T {
    (var l := v->l - 1;
    Vec(v->v[l := DefaultVecElem()], l))
}

function {:inline} RemoveAtVec<T>(v: Vec T, i: int): Vec T {
    (var l := v->l - 1;
    Vec(
        (lambda j: int ::
           if j >= 0 && j < l then
               if j < i then v->v[j] else v->v[j+1]
           else DefaultVecElem()),
        l))
}

function {:inline} ConcatVec<T>(v1: Vec T, v2: Vec T): Vec T {
    (var l1, m1, l2, m2 := v1->l, v1->v, v2->l, v2->v;
    Vec(
        (lambda i: int ::
          if i >= 0 && i < l1 + l2 then
            if i < l1 then m1[i] else m2[i - l1]
          else DefaultVecElem()),
        l1 + l2))
}

function {:inline} ReverseVec<T>(v: Vec T): Vec T {
    (var l := v->l;
    Vec(
        (lambda i: int :: if 0 <= i && i < l then v->v[l - i - 1] else DefaultVecElem()),
        l))
}

function {:inline} SliceVec<T>(v: Vec T, i: int, j: int): Vec T {
    (var m := v->v;
    Vec(
        (lambda k:int ::
          if 0 <= k && k < j - i then
            m[i + k]
          else
            DefaultVecElem()),
        (if j - i < 0 then 0 else j - i)))
}


function {:inline} UpdateVec<T>(v: Vec T, i: int, elem: T): Vec T {
    Vec(v->v[i := elem], v->l)
}

function {:inline} SwapVec<T>(v: Vec T, i: int, j: int): Vec T {
    (var m := v->v;
    Vec(m[i := m[j]][j := m[i]], v->l))
}

function {:inline} ContainsVec<T>(v: Vec T, e: T): bool {
    (var l := v->l;
    (exists i: int :: InRangeVec(v, i) && v->v[i] == e))
}

function IndexOfVec<T>(v: Vec T, e: T): int;
axiom {:ctor "Vec"} (forall<T> v: Vec T, e: T :: {IndexOfVec(v, e)}
    (var i := IndexOfVec(v,e);
     if (!ContainsVec(v, e)) then i == -1
     else InRangeVec(v, i) && ReadVec(v, i) == e &&
        (forall j: int :: j >= 0 && j < i ==> ReadVec(v, j) != e)));

// This function should stay non-inlined as it guards many quantifiers
// over vectors. It appears important to have this uninterpreted for
// quantifier triggering.
function InRangeVec<T>(v: Vec T, i: int): bool {
    i >= 0 && i < LenVec(v)
}

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Boogie model for multisets, based on Boogie arrays. This theory assumes extensional equality for element types.

datatype Multiset<T> {
    Multiset(v: [T]int, l: int)
}

function {:builtin "MapConst"} MapConstMultiset<T>(l: int): [T]int;

function {:inline} EmptyMultiset<T>(): Multiset T {
    Multiset(MapConstMultiset(0), 0)
}

function {:inline} LenMultiset<T>(s: Multiset T): int {
    s->l
}

function {:inline} ExtendMultiset<T>(s: Multiset T, v: T): Multiset T {
    (var len := s->l;
    (var cnt := s->v[v];
    Multiset(s->v[v := (cnt + 1)], len + 1)))
}

// This function returns (s1 - s2). This function assumes that s2 is a subset of s1.
function {:inline} SubtractMultiset<T>(s1: Multiset T, s2: Multiset T): Multiset T {
    (var len1 := s1->l;
    (var len2 := s2->l;
    Multiset((lambda v:T :: s1->v[v]-s2->v[v]), len1-len2)))
}

function {:inline} IsEmptyMultiset<T>(s: Multiset T): bool {
    (s->l == 0) &&
    (forall v: T :: s->v[v] == 0)
}

function {:inline} IsSubsetMultiset<T>(s1: Multiset T, s2: Multiset T): bool {
    (s1->l <= s2->l) &&
    (forall v: T :: s1->v[v] <= s2->v[v])
}

function {:inline} ContainsMultiset<T>(s: Multiset T, v: T): bool {
    s->v[v] > 0
}

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Theory for tables.

// v is the SMT array holding the key-value assignment. e is an array which
// independently determines whether a key is valid or not. l is the length.
//
// Note that even though the program cannot reflect over existence of a key,
// we want the specification to be able to do this, so it can express
// verification conditions like "key has been inserted".
datatype Table <K, V> {
    Table(v: [K]V, e: [K]bool, l: int)
}

// Functions for default SMT arrays. For the table values, we don't care and
// use an uninterpreted function.
function DefaultTableArray<K, V>(): [K]V;
function DefaultTableKeyExistsArray<K>(): [K]bool;
axiom DefaultTableKeyExistsArray() == (lambda i: int :: false);

function {:inline} EmptyTable<K, V>(): Table K V {
    Table(DefaultTableArray(), DefaultTableKeyExistsArray(), 0)
}

function {:inline} GetTable<K,V>(t: Table K V, k: K): V {
    // Notice we do not check whether key is in the table. The result is undetermined if it is not.
    t->v[k]
}

function {:inline} LenTable<K,V>(t: Table K V): int {
    t->l
}


function {:inline} ContainsTable<K,V>(t: Table K V, k: K): bool {
    t->e[k]
}

function {:inline} UpdateTable<K,V>(t: Table K V, k: K, v: V): Table K V {
    Table(t->v[k := v], t->e, t->l)
}

function {:inline} AddTable<K,V>(t: Table K V, k: K, v: V): Table K V {
    // This function has an undetermined result if the key is already in the table
    // (all specification functions have this "partial definiteness" behavior). Thus we can
    // just increment the length.
    Table(t->v[k := v], t->e[k := true], t->l + 1)
}

function {:inline} RemoveTable<K,V>(t: Table K V, k: K): Table K V {
    // Similar as above, we only need to consider the case where the key is in the table.
    Table(t->v, t->e[k := false], t->l - 1)
}

axiom {:ctor "Table"} (forall<K,V> t: Table K V :: {LenTable(t)}
    (exists k: K :: {ContainsTable(t, k)} ContainsTable(t, k)) ==> LenTable(t) >= 1
);
// TODO: we might want to encoder a stronger property that the length of table
// must be more than N given a set of N items. Currently we don't see a need here
// and the above axiom seems to be sufficient.
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// ==================================================================================
// Native object::exists_at

// ==================================================================================
// Intrinsic implementation of aggregator and aggregator factory

datatype $1_aggregator_Aggregator {
    $1_aggregator_Aggregator($handle: int, $key: int, $limit: int, $val: int)
}
function {:inline} $Update'$1_aggregator_Aggregator'_handle(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(x, s->$key, s->$limit, s->$val)
}
function {:inline} $Update'$1_aggregator_Aggregator'_key(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(s->$handle, x, s->$limit, s->$val)
}
function {:inline} $Update'$1_aggregator_Aggregator'_limit(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(s->$handle, s->$key, x, s->$val)
}
function {:inline} $Update'$1_aggregator_Aggregator'_val(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(s->$handle, s->$key, s->$limit, x)
}
function $IsValid'$1_aggregator_Aggregator'(s: $1_aggregator_Aggregator): bool {
    $IsValid'address'(s->$handle)
      && $IsValid'address'(s->$key)
      && $IsValid'u128'(s->$limit)
      && $IsValid'u128'(s->$val)
}
function {:inline} $IsEqual'$1_aggregator_Aggregator'(s1: $1_aggregator_Aggregator, s2: $1_aggregator_Aggregator): bool {
    s1 == s2
}
function {:inline} $1_aggregator_spec_get_limit(s: $1_aggregator_Aggregator): int {
    s->$limit
}
function {:inline} $1_aggregator_spec_get_handle(s: $1_aggregator_Aggregator): int {
    s->$handle
}
function {:inline} $1_aggregator_spec_get_key(s: $1_aggregator_Aggregator): int {
    s->$key
}
function {:inline} $1_aggregator_spec_get_val(s: $1_aggregator_Aggregator): int {
    s->$val
}

function $1_aggregator_spec_read(agg: $1_aggregator_Aggregator): int {
    $1_aggregator_spec_get_val(agg)
}

function $1_aggregator_spec_aggregator_set_val(agg: $1_aggregator_Aggregator, val: int): $1_aggregator_Aggregator {
    $Update'$1_aggregator_Aggregator'_val(agg, val)
}

function $1_aggregator_spec_aggregator_get_val(agg: $1_aggregator_Aggregator): int {
    $1_aggregator_spec_get_val(agg)
}

function $1_aggregator_factory_spec_new_aggregator(limit: int) : $1_aggregator_Aggregator;

axiom (forall limit: int :: {$1_aggregator_factory_spec_new_aggregator(limit)}
    (var agg := $1_aggregator_factory_spec_new_aggregator(limit);
     $1_aggregator_spec_get_limit(agg) == limit));

axiom (forall limit: int :: {$1_aggregator_factory_spec_new_aggregator(limit)}
     (var agg := $1_aggregator_factory_spec_new_aggregator(limit);
     $1_aggregator_spec_aggregator_get_val(agg) == 0));


// ============================================================================================
// Primitive Types

const $MAX_U8: int;
axiom $MAX_U8 == 255;
const $MAX_U16: int;
axiom $MAX_U16 == 65535;
const $MAX_U32: int;
axiom $MAX_U32 == 4294967295;
const $MAX_U64: int;
axiom $MAX_U64 == 18446744073709551615;
const $MAX_U128: int;
axiom $MAX_U128 == 340282366920938463463374607431768211455;
const $MAX_U256: int;
axiom $MAX_U256 == 115792089237316195423570985008687907853269984665640564039457584007913129639935;

// Templates for bitvector operations

function {:bvbuiltin "bvand"} $And'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvor"} $Or'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvxor"} $Xor'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvadd"} $Add'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvsub"} $Sub'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvmul"} $Mul'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvudiv"} $Div'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvurem"} $Mod'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvshl"} $Shl'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvlshr"} $Shr'Bv8'(bv8,bv8) returns(bv8);
function {:bvbuiltin "bvult"} $Lt'Bv8'(bv8,bv8) returns(bool);
function {:bvbuiltin "bvule"} $Le'Bv8'(bv8,bv8) returns(bool);
function {:bvbuiltin "bvugt"} $Gt'Bv8'(bv8,bv8) returns(bool);
function {:bvbuiltin "bvuge"} $Ge'Bv8'(bv8,bv8) returns(bool);

procedure {:inline 1} $AddBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    if ($Lt'Bv8'($Add'Bv8'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Add'Bv8'(src1, src2);
}

procedure {:inline 1} $AddBv8_unchecked(src1: bv8, src2: bv8) returns (dst: bv8)
{
    dst := $Add'Bv8'(src1, src2);
}

procedure {:inline 1} $SubBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    if ($Lt'Bv8'(src1, src2)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Sub'Bv8'(src1, src2);
}

procedure {:inline 1} $MulBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    if ($Lt'Bv8'($Mul'Bv8'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mul'Bv8'(src1, src2);
}

procedure {:inline 1} $DivBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    if (src2 == 0bv8) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Div'Bv8'(src1, src2);
}

procedure {:inline 1} $ModBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    if (src2 == 0bv8) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mod'Bv8'(src1, src2);
}

procedure {:inline 1} $AndBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    dst := $And'Bv8'(src1,src2);
}

procedure {:inline 1} $OrBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    dst := $Or'Bv8'(src1,src2);
}

procedure {:inline 1} $XorBv8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    dst := $Xor'Bv8'(src1,src2);
}

procedure {:inline 1} $LtBv8(src1: bv8, src2: bv8) returns (dst: bool)
{
    dst := $Lt'Bv8'(src1,src2);
}

procedure {:inline 1} $LeBv8(src1: bv8, src2: bv8) returns (dst: bool)
{
    dst := $Le'Bv8'(src1,src2);
}

procedure {:inline 1} $GtBv8(src1: bv8, src2: bv8) returns (dst: bool)
{
    dst := $Gt'Bv8'(src1,src2);
}

procedure {:inline 1} $GeBv8(src1: bv8, src2: bv8) returns (dst: bool)
{
    dst := $Ge'Bv8'(src1,src2);
}

function $IsValid'bv8'(v: bv8): bool {
  $Ge'Bv8'(v,0bv8) && $Le'Bv8'(v,255bv8)
}

function {:inline} $IsEqual'bv8'(x: bv8, y: bv8): bool {
    x == y
}

procedure {:inline 1} $int2bv8(src: int) returns (dst: bv8)
{
    if (src > 255) {
        call $ExecFailureAbort();
        return;
    }
    dst := $int2bv.8(src);
}

procedure {:inline 1} $bv2int8(src: bv8) returns (dst: int)
{
    dst := $bv2int.8(src);
}

function {:builtin "(_ int2bv 8)"} $int2bv.8(i: int) returns (bv8);
function {:builtin "bv2nat"} $bv2int.8(i: bv8) returns (int);

function {:bvbuiltin "bvand"} $And'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvor"} $Or'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvxor"} $Xor'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvadd"} $Add'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvsub"} $Sub'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvmul"} $Mul'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvudiv"} $Div'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvurem"} $Mod'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvshl"} $Shl'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvlshr"} $Shr'Bv16'(bv16,bv16) returns(bv16);
function {:bvbuiltin "bvult"} $Lt'Bv16'(bv16,bv16) returns(bool);
function {:bvbuiltin "bvule"} $Le'Bv16'(bv16,bv16) returns(bool);
function {:bvbuiltin "bvugt"} $Gt'Bv16'(bv16,bv16) returns(bool);
function {:bvbuiltin "bvuge"} $Ge'Bv16'(bv16,bv16) returns(bool);

procedure {:inline 1} $AddBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    if ($Lt'Bv16'($Add'Bv16'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Add'Bv16'(src1, src2);
}

procedure {:inline 1} $AddBv16_unchecked(src1: bv16, src2: bv16) returns (dst: bv16)
{
    dst := $Add'Bv16'(src1, src2);
}

procedure {:inline 1} $SubBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    if ($Lt'Bv16'(src1, src2)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Sub'Bv16'(src1, src2);
}

procedure {:inline 1} $MulBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    if ($Lt'Bv16'($Mul'Bv16'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mul'Bv16'(src1, src2);
}

procedure {:inline 1} $DivBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    if (src2 == 0bv16) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Div'Bv16'(src1, src2);
}

procedure {:inline 1} $ModBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    if (src2 == 0bv16) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mod'Bv16'(src1, src2);
}

procedure {:inline 1} $AndBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    dst := $And'Bv16'(src1,src2);
}

procedure {:inline 1} $OrBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    dst := $Or'Bv16'(src1,src2);
}

procedure {:inline 1} $XorBv16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    dst := $Xor'Bv16'(src1,src2);
}

procedure {:inline 1} $LtBv16(src1: bv16, src2: bv16) returns (dst: bool)
{
    dst := $Lt'Bv16'(src1,src2);
}

procedure {:inline 1} $LeBv16(src1: bv16, src2: bv16) returns (dst: bool)
{
    dst := $Le'Bv16'(src1,src2);
}

procedure {:inline 1} $GtBv16(src1: bv16, src2: bv16) returns (dst: bool)
{
    dst := $Gt'Bv16'(src1,src2);
}

procedure {:inline 1} $GeBv16(src1: bv16, src2: bv16) returns (dst: bool)
{
    dst := $Ge'Bv16'(src1,src2);
}

function $IsValid'bv16'(v: bv16): bool {
  $Ge'Bv16'(v,0bv16) && $Le'Bv16'(v,65535bv16)
}

function {:inline} $IsEqual'bv16'(x: bv16, y: bv16): bool {
    x == y
}

procedure {:inline 1} $int2bv16(src: int) returns (dst: bv16)
{
    if (src > 65535) {
        call $ExecFailureAbort();
        return;
    }
    dst := $int2bv.16(src);
}

procedure {:inline 1} $bv2int16(src: bv16) returns (dst: int)
{
    dst := $bv2int.16(src);
}

function {:builtin "(_ int2bv 16)"} $int2bv.16(i: int) returns (bv16);
function {:builtin "bv2nat"} $bv2int.16(i: bv16) returns (int);

function {:bvbuiltin "bvand"} $And'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvor"} $Or'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvxor"} $Xor'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvadd"} $Add'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvsub"} $Sub'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvmul"} $Mul'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvudiv"} $Div'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvurem"} $Mod'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvshl"} $Shl'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvlshr"} $Shr'Bv32'(bv32,bv32) returns(bv32);
function {:bvbuiltin "bvult"} $Lt'Bv32'(bv32,bv32) returns(bool);
function {:bvbuiltin "bvule"} $Le'Bv32'(bv32,bv32) returns(bool);
function {:bvbuiltin "bvugt"} $Gt'Bv32'(bv32,bv32) returns(bool);
function {:bvbuiltin "bvuge"} $Ge'Bv32'(bv32,bv32) returns(bool);

procedure {:inline 1} $AddBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    if ($Lt'Bv32'($Add'Bv32'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Add'Bv32'(src1, src2);
}

procedure {:inline 1} $AddBv32_unchecked(src1: bv32, src2: bv32) returns (dst: bv32)
{
    dst := $Add'Bv32'(src1, src2);
}

procedure {:inline 1} $SubBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    if ($Lt'Bv32'(src1, src2)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Sub'Bv32'(src1, src2);
}

procedure {:inline 1} $MulBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    if ($Lt'Bv32'($Mul'Bv32'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mul'Bv32'(src1, src2);
}

procedure {:inline 1} $DivBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    if (src2 == 0bv32) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Div'Bv32'(src1, src2);
}

procedure {:inline 1} $ModBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    if (src2 == 0bv32) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mod'Bv32'(src1, src2);
}

procedure {:inline 1} $AndBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    dst := $And'Bv32'(src1,src2);
}

procedure {:inline 1} $OrBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    dst := $Or'Bv32'(src1,src2);
}

procedure {:inline 1} $XorBv32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    dst := $Xor'Bv32'(src1,src2);
}

procedure {:inline 1} $LtBv32(src1: bv32, src2: bv32) returns (dst: bool)
{
    dst := $Lt'Bv32'(src1,src2);
}

procedure {:inline 1} $LeBv32(src1: bv32, src2: bv32) returns (dst: bool)
{
    dst := $Le'Bv32'(src1,src2);
}

procedure {:inline 1} $GtBv32(src1: bv32, src2: bv32) returns (dst: bool)
{
    dst := $Gt'Bv32'(src1,src2);
}

procedure {:inline 1} $GeBv32(src1: bv32, src2: bv32) returns (dst: bool)
{
    dst := $Ge'Bv32'(src1,src2);
}

function $IsValid'bv32'(v: bv32): bool {
  $Ge'Bv32'(v,0bv32) && $Le'Bv32'(v,2147483647bv32)
}

function {:inline} $IsEqual'bv32'(x: bv32, y: bv32): bool {
    x == y
}

procedure {:inline 1} $int2bv32(src: int) returns (dst: bv32)
{
    if (src > 2147483647) {
        call $ExecFailureAbort();
        return;
    }
    dst := $int2bv.32(src);
}

procedure {:inline 1} $bv2int32(src: bv32) returns (dst: int)
{
    dst := $bv2int.32(src);
}

function {:builtin "(_ int2bv 32)"} $int2bv.32(i: int) returns (bv32);
function {:builtin "bv2nat"} $bv2int.32(i: bv32) returns (int);

function {:bvbuiltin "bvand"} $And'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvor"} $Or'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvxor"} $Xor'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvadd"} $Add'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvsub"} $Sub'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvmul"} $Mul'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvudiv"} $Div'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvurem"} $Mod'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvshl"} $Shl'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvlshr"} $Shr'Bv64'(bv64,bv64) returns(bv64);
function {:bvbuiltin "bvult"} $Lt'Bv64'(bv64,bv64) returns(bool);
function {:bvbuiltin "bvule"} $Le'Bv64'(bv64,bv64) returns(bool);
function {:bvbuiltin "bvugt"} $Gt'Bv64'(bv64,bv64) returns(bool);
function {:bvbuiltin "bvuge"} $Ge'Bv64'(bv64,bv64) returns(bool);

procedure {:inline 1} $AddBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    if ($Lt'Bv64'($Add'Bv64'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Add'Bv64'(src1, src2);
}

procedure {:inline 1} $AddBv64_unchecked(src1: bv64, src2: bv64) returns (dst: bv64)
{
    dst := $Add'Bv64'(src1, src2);
}

procedure {:inline 1} $SubBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    if ($Lt'Bv64'(src1, src2)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Sub'Bv64'(src1, src2);
}

procedure {:inline 1} $MulBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    if ($Lt'Bv64'($Mul'Bv64'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mul'Bv64'(src1, src2);
}

procedure {:inline 1} $DivBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    if (src2 == 0bv64) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Div'Bv64'(src1, src2);
}

procedure {:inline 1} $ModBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    if (src2 == 0bv64) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mod'Bv64'(src1, src2);
}

procedure {:inline 1} $AndBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    dst := $And'Bv64'(src1,src2);
}

procedure {:inline 1} $OrBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    dst := $Or'Bv64'(src1,src2);
}

procedure {:inline 1} $XorBv64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    dst := $Xor'Bv64'(src1,src2);
}

procedure {:inline 1} $LtBv64(src1: bv64, src2: bv64) returns (dst: bool)
{
    dst := $Lt'Bv64'(src1,src2);
}

procedure {:inline 1} $LeBv64(src1: bv64, src2: bv64) returns (dst: bool)
{
    dst := $Le'Bv64'(src1,src2);
}

procedure {:inline 1} $GtBv64(src1: bv64, src2: bv64) returns (dst: bool)
{
    dst := $Gt'Bv64'(src1,src2);
}

procedure {:inline 1} $GeBv64(src1: bv64, src2: bv64) returns (dst: bool)
{
    dst := $Ge'Bv64'(src1,src2);
}

function $IsValid'bv64'(v: bv64): bool {
  $Ge'Bv64'(v,0bv64) && $Le'Bv64'(v,18446744073709551615bv64)
}

function {:inline} $IsEqual'bv64'(x: bv64, y: bv64): bool {
    x == y
}

procedure {:inline 1} $int2bv64(src: int) returns (dst: bv64)
{
    if (src > 18446744073709551615) {
        call $ExecFailureAbort();
        return;
    }
    dst := $int2bv.64(src);
}

procedure {:inline 1} $bv2int64(src: bv64) returns (dst: int)
{
    dst := $bv2int.64(src);
}

function {:builtin "(_ int2bv 64)"} $int2bv.64(i: int) returns (bv64);
function {:builtin "bv2nat"} $bv2int.64(i: bv64) returns (int);

function {:bvbuiltin "bvand"} $And'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvor"} $Or'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvxor"} $Xor'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvadd"} $Add'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvsub"} $Sub'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvmul"} $Mul'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvudiv"} $Div'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvurem"} $Mod'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvshl"} $Shl'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvlshr"} $Shr'Bv128'(bv128,bv128) returns(bv128);
function {:bvbuiltin "bvult"} $Lt'Bv128'(bv128,bv128) returns(bool);
function {:bvbuiltin "bvule"} $Le'Bv128'(bv128,bv128) returns(bool);
function {:bvbuiltin "bvugt"} $Gt'Bv128'(bv128,bv128) returns(bool);
function {:bvbuiltin "bvuge"} $Ge'Bv128'(bv128,bv128) returns(bool);

procedure {:inline 1} $AddBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    if ($Lt'Bv128'($Add'Bv128'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Add'Bv128'(src1, src2);
}

procedure {:inline 1} $AddBv128_unchecked(src1: bv128, src2: bv128) returns (dst: bv128)
{
    dst := $Add'Bv128'(src1, src2);
}

procedure {:inline 1} $SubBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    if ($Lt'Bv128'(src1, src2)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Sub'Bv128'(src1, src2);
}

procedure {:inline 1} $MulBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    if ($Lt'Bv128'($Mul'Bv128'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mul'Bv128'(src1, src2);
}

procedure {:inline 1} $DivBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    if (src2 == 0bv128) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Div'Bv128'(src1, src2);
}

procedure {:inline 1} $ModBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    if (src2 == 0bv128) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mod'Bv128'(src1, src2);
}

procedure {:inline 1} $AndBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    dst := $And'Bv128'(src1,src2);
}

procedure {:inline 1} $OrBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    dst := $Or'Bv128'(src1,src2);
}

procedure {:inline 1} $XorBv128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    dst := $Xor'Bv128'(src1,src2);
}

procedure {:inline 1} $LtBv128(src1: bv128, src2: bv128) returns (dst: bool)
{
    dst := $Lt'Bv128'(src1,src2);
}

procedure {:inline 1} $LeBv128(src1: bv128, src2: bv128) returns (dst: bool)
{
    dst := $Le'Bv128'(src1,src2);
}

procedure {:inline 1} $GtBv128(src1: bv128, src2: bv128) returns (dst: bool)
{
    dst := $Gt'Bv128'(src1,src2);
}

procedure {:inline 1} $GeBv128(src1: bv128, src2: bv128) returns (dst: bool)
{
    dst := $Ge'Bv128'(src1,src2);
}

function $IsValid'bv128'(v: bv128): bool {
  $Ge'Bv128'(v,0bv128) && $Le'Bv128'(v,340282366920938463463374607431768211455bv128)
}

function {:inline} $IsEqual'bv128'(x: bv128, y: bv128): bool {
    x == y
}

procedure {:inline 1} $int2bv128(src: int) returns (dst: bv128)
{
    if (src > 340282366920938463463374607431768211455) {
        call $ExecFailureAbort();
        return;
    }
    dst := $int2bv.128(src);
}

procedure {:inline 1} $bv2int128(src: bv128) returns (dst: int)
{
    dst := $bv2int.128(src);
}

function {:builtin "(_ int2bv 128)"} $int2bv.128(i: int) returns (bv128);
function {:builtin "bv2nat"} $bv2int.128(i: bv128) returns (int);

function {:bvbuiltin "bvand"} $And'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvor"} $Or'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvxor"} $Xor'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvadd"} $Add'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvsub"} $Sub'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvmul"} $Mul'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvudiv"} $Div'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvurem"} $Mod'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvshl"} $Shl'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvlshr"} $Shr'Bv256'(bv256,bv256) returns(bv256);
function {:bvbuiltin "bvult"} $Lt'Bv256'(bv256,bv256) returns(bool);
function {:bvbuiltin "bvule"} $Le'Bv256'(bv256,bv256) returns(bool);
function {:bvbuiltin "bvugt"} $Gt'Bv256'(bv256,bv256) returns(bool);
function {:bvbuiltin "bvuge"} $Ge'Bv256'(bv256,bv256) returns(bool);

procedure {:inline 1} $AddBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    if ($Lt'Bv256'($Add'Bv256'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Add'Bv256'(src1, src2);
}

procedure {:inline 1} $AddBv256_unchecked(src1: bv256, src2: bv256) returns (dst: bv256)
{
    dst := $Add'Bv256'(src1, src2);
}

procedure {:inline 1} $SubBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    if ($Lt'Bv256'(src1, src2)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Sub'Bv256'(src1, src2);
}

procedure {:inline 1} $MulBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    if ($Lt'Bv256'($Mul'Bv256'(src1, src2), src1)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mul'Bv256'(src1, src2);
}

procedure {:inline 1} $DivBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    if (src2 == 0bv256) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Div'Bv256'(src1, src2);
}

procedure {:inline 1} $ModBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    if (src2 == 0bv256) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mod'Bv256'(src1, src2);
}

procedure {:inline 1} $AndBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    dst := $And'Bv256'(src1,src2);
}

procedure {:inline 1} $OrBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    dst := $Or'Bv256'(src1,src2);
}

procedure {:inline 1} $XorBv256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    dst := $Xor'Bv256'(src1,src2);
}

procedure {:inline 1} $LtBv256(src1: bv256, src2: bv256) returns (dst: bool)
{
    dst := $Lt'Bv256'(src1,src2);
}

procedure {:inline 1} $LeBv256(src1: bv256, src2: bv256) returns (dst: bool)
{
    dst := $Le'Bv256'(src1,src2);
}

procedure {:inline 1} $GtBv256(src1: bv256, src2: bv256) returns (dst: bool)
{
    dst := $Gt'Bv256'(src1,src2);
}

procedure {:inline 1} $GeBv256(src1: bv256, src2: bv256) returns (dst: bool)
{
    dst := $Ge'Bv256'(src1,src2);
}

function $IsValid'bv256'(v: bv256): bool {
  $Ge'Bv256'(v,0bv256) && $Le'Bv256'(v,115792089237316195423570985008687907853269984665640564039457584007913129639935bv256)
}

function {:inline} $IsEqual'bv256'(x: bv256, y: bv256): bool {
    x == y
}

procedure {:inline 1} $int2bv256(src: int) returns (dst: bv256)
{
    if (src > 115792089237316195423570985008687907853269984665640564039457584007913129639935) {
        call $ExecFailureAbort();
        return;
    }
    dst := $int2bv.256(src);
}

procedure {:inline 1} $bv2int256(src: bv256) returns (dst: int)
{
    dst := $bv2int.256(src);
}

function {:builtin "(_ int2bv 256)"} $int2bv.256(i: int) returns (bv256);
function {:builtin "bv2nat"} $bv2int.256(i: bv256) returns (int);

datatype $Range {
    $Range(lb: int, ub: int)
}

function {:inline} $IsValid'bool'(v: bool): bool {
  true
}

function $IsValid'u8'(v: int): bool {
  v >= 0 && v <= $MAX_U8
}

function $IsValid'u16'(v: int): bool {
  v >= 0 && v <= $MAX_U16
}

function $IsValid'u32'(v: int): bool {
  v >= 0 && v <= $MAX_U32
}

function $IsValid'u64'(v: int): bool {
  v >= 0 && v <= $MAX_U64
}

function $IsValid'u128'(v: int): bool {
  v >= 0 && v <= $MAX_U128
}

function $IsValid'u256'(v: int): bool {
  v >= 0 && v <= $MAX_U256
}

function $IsValid'num'(v: int): bool {
  true
}

function $IsValid'address'(v: int): bool {
  // TODO: restrict max to representable addresses?
  v >= 0
}

function {:inline} $IsValidRange(r: $Range): bool {
   $IsValid'u64'(r->lb) &&  $IsValid'u64'(r->ub)
}

// Intentionally not inlined so it serves as a trigger in quantifiers.
function $InRange(r: $Range, i: int): bool {
   r->lb <= i && i < r->ub
}


function {:inline} $IsEqual'u8'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'u16'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'u32'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'u64'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'u128'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'u256'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'num'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'address'(x: int, y: int): bool {
    x == y
}

function {:inline} $IsEqual'bool'(x: bool, y: bool): bool {
    x == y
}

// ============================================================================================
// Memory

datatype $Location {
    // A global resource location within the statically known resource type's memory,
    // where `a` is an address.
    $Global(a: int),
    // A local location. `i` is the unique index of the local.
    $Local(i: int),
    // The location of a reference outside of the verification scope, for example, a `&mut` parameter
    // of the function being verified. References with these locations don't need to be written back
    // when mutation ends.
    $Param(i: int),
    // The location of an uninitialized mutation. Using this to make sure that the location
    // will not be equal to any valid mutation locations, i.e., $Local, $Global, or $Param.
    $Uninitialized()
}

// A mutable reference which also carries its current value. Since mutable references
// are single threaded in Move, we can keep them together and treat them as a value
// during mutation until the point they are stored back to their original location.
datatype $Mutation<T> {
    $Mutation(l: $Location, p: Vec int, v: T)
}

// Representation of memory for a given type.
datatype $Memory<T> {
    $Memory(domain: [int]bool, contents: [int]T)
}

function {:builtin "MapConst"} $ConstMemoryDomain(v: bool): [int]bool;
function {:builtin "MapConst"} $ConstMemoryContent<T>(v: T): [int]T;
axiom $ConstMemoryDomain(false) == (lambda i: int :: false);
axiom $ConstMemoryDomain(true) == (lambda i: int :: true);


// Dereferences a mutation.
function {:inline} $Dereference<T>(ref: $Mutation T): T {
    ref->v
}

// Update the value of a mutation.
function {:inline} $UpdateMutation<T>(m: $Mutation T, v: T): $Mutation T {
    $Mutation(m->l, m->p, v)
}

function {:inline} $ChildMutation<T1, T2>(m: $Mutation T1, offset: int, v: T2): $Mutation T2 {
    $Mutation(m->l, ExtendVec(m->p, offset), v)
}

// Return true if two mutations share the location and path
function {:inline} $IsSameMutation<T1, T2>(parent: $Mutation T1, child: $Mutation T2 ): bool {
    parent->l == child->l && parent->p == child->p
}

// Return true if the mutation is a parent of a child which was derived with the given edge offset. This
// is used to implement write-back choices.
function {:inline} $IsParentMutation<T1, T2>(parent: $Mutation T1, edge: int, child: $Mutation T2 ): bool {
    parent->l == child->l &&
    (var pp := parent->p;
    (var cp := child->p;
    (var pl := LenVec(pp);
    (var cl := LenVec(cp);
     cl == pl + 1 &&
     (forall i: int:: i >= 0 && i < pl ==> ReadVec(pp, i) ==  ReadVec(cp, i)) &&
     $EdgeMatches(ReadVec(cp, pl), edge)
    ))))
}

// Return true if the mutation is a parent of a child, for hyper edge.
function {:inline} $IsParentMutationHyper<T1, T2>(parent: $Mutation T1, hyper_edge: Vec int, child: $Mutation T2 ): bool {
    parent->l == child->l &&
    (var pp := parent->p;
    (var cp := child->p;
    (var pl := LenVec(pp);
    (var cl := LenVec(cp);
    (var el := LenVec(hyper_edge);
     cl == pl + el &&
     (forall i: int:: i >= 0 && i < pl ==> ReadVec(pp, i) == ReadVec(cp, i)) &&
     (forall i: int:: i >= 0 && i < el ==> $EdgeMatches(ReadVec(cp, pl + i), ReadVec(hyper_edge, i)))
    )))))
}

function {:inline} $EdgeMatches(edge: int, edge_pattern: int): bool {
    edge_pattern == -1 // wildcard
    || edge_pattern == edge
}



function {:inline} $SameLocation<T1, T2>(m1: $Mutation T1, m2: $Mutation T2): bool {
    m1->l == m2->l
}

function {:inline} $HasGlobalLocation<T>(m: $Mutation T): bool {
    (m->l) is $Global
}

function {:inline} $HasLocalLocation<T>(m: $Mutation T, idx: int): bool {
    m->l == $Local(idx)
}

function {:inline} $GlobalLocationAddress<T>(m: $Mutation T): int {
    (m->l)->a
}



// Tests whether resource exists.
function {:inline} $ResourceExists<T>(m: $Memory T, addr: int): bool {
    m->domain[addr]
}

// Obtains Value of given resource.
function {:inline} $ResourceValue<T>(m: $Memory T, addr: int): T {
    m->contents[addr]
}

// Update resource.
function {:inline} $ResourceUpdate<T>(m: $Memory T, a: int, v: T): $Memory T {
    $Memory(m->domain[a := true], m->contents[a := v])
}

// Remove resource.
function {:inline} $ResourceRemove<T>(m: $Memory T, a: int): $Memory T {
    $Memory(m->domain[a := false], m->contents)
}

// Copies resource from memory s to m.
function {:inline} $ResourceCopy<T>(m: $Memory T, s: $Memory T, a: int): $Memory T {
    $Memory(m->domain[a := s->domain[a]],
            m->contents[a := s->contents[a]])
}



// ============================================================================================
// Abort Handling

var $abort_flag: bool;
var $abort_code: int;

function {:inline} $process_abort_code(code: int): int {
    code
}

const $EXEC_FAILURE_CODE: int;
axiom $EXEC_FAILURE_CODE == -1;

// TODO(wrwg): currently we map aborts of native functions like those for vectors also to
//   execution failure. This may need to be aligned with what the runtime actually does.

procedure {:inline 1} $ExecFailureAbort() {
    $abort_flag := true;
    $abort_code := $EXEC_FAILURE_CODE;
}

procedure {:inline 1} $Abort(code: int) {
    $abort_flag := true;
    $abort_code := code;
}

function {:inline} $StdError(cat: int, reason: int): int {
    reason * 256 + cat
}

procedure {:inline 1} $InitVerification() {
    // Set abort_flag to false, and havoc abort_code
    $abort_flag := false;
    havoc $abort_code;
    // Initialize event store
    call $InitEventStore();
}

// ============================================================================================
// Instructions


procedure {:inline 1} $CastU8(src: int) returns (dst: int)
{
    if (src > $MAX_U8) {
        call $ExecFailureAbort();
        return;
    }
    dst := src;
}

procedure {:inline 1} $CastU16(src: int) returns (dst: int)
{
    if (src > $MAX_U16) {
        call $ExecFailureAbort();
        return;
    }
    dst := src;
}

procedure {:inline 1} $CastU32(src: int) returns (dst: int)
{
    if (src > $MAX_U32) {
        call $ExecFailureAbort();
        return;
    }
    dst := src;
}

procedure {:inline 1} $CastU64(src: int) returns (dst: int)
{
    if (src > $MAX_U64) {
        call $ExecFailureAbort();
        return;
    }
    dst := src;
}

procedure {:inline 1} $CastU128(src: int) returns (dst: int)
{
    if (src > $MAX_U128) {
        call $ExecFailureAbort();
        return;
    }
    dst := src;
}

procedure {:inline 1} $CastU256(src: int) returns (dst: int)
{
    if (src > $MAX_U256) {
        call $ExecFailureAbort();
        return;
    }
    dst := src;
}

procedure {:inline 1} $AddU8(src1: int, src2: int) returns (dst: int)
{
    if (src1 + src2 > $MAX_U8) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 + src2;
}

procedure {:inline 1} $AddU16(src1: int, src2: int) returns (dst: int)
{
    if (src1 + src2 > $MAX_U16) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 + src2;
}

procedure {:inline 1} $AddU16_unchecked(src1: int, src2: int) returns (dst: int)
{
    dst := src1 + src2;
}

procedure {:inline 1} $AddU32(src1: int, src2: int) returns (dst: int)
{
    if (src1 + src2 > $MAX_U32) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 + src2;
}

procedure {:inline 1} $AddU32_unchecked(src1: int, src2: int) returns (dst: int)
{
    dst := src1 + src2;
}

procedure {:inline 1} $AddU64(src1: int, src2: int) returns (dst: int)
{
    if (src1 + src2 > $MAX_U64) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 + src2;
}

procedure {:inline 1} $AddU64_unchecked(src1: int, src2: int) returns (dst: int)
{
    dst := src1 + src2;
}

procedure {:inline 1} $AddU128(src1: int, src2: int) returns (dst: int)
{
    if (src1 + src2 > $MAX_U128) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 + src2;
}

procedure {:inline 1} $AddU128_unchecked(src1: int, src2: int) returns (dst: int)
{
    dst := src1 + src2;
}

procedure {:inline 1} $AddU256(src1: int, src2: int) returns (dst: int)
{
    if (src1 + src2 > $MAX_U256) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 + src2;
}

procedure {:inline 1} $AddU256_unchecked(src1: int, src2: int) returns (dst: int)
{
    dst := src1 + src2;
}

procedure {:inline 1} $Sub(src1: int, src2: int) returns (dst: int)
{
    if (src1 < src2) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 - src2;
}

// uninterpreted function to return an undefined value.
function $undefined_int(): int;

// Recursive exponentiation function
// Undefined unless e >=0.  $pow(0,0) is also undefined.
function $pow(n: int, e: int): int {
    if n != 0 && e == 0 then 1
    else if e > 0 then n * $pow(n, e - 1)
    else $undefined_int()
}

function $shl(src1: int, p: int): int {
    src1 * $pow(2, p)
}

function $shlU8(src1: int, p: int): int {
    (src1 * $pow(2, p)) mod 256
}

function $shlU16(src1: int, p: int): int {
    (src1 * $pow(2, p)) mod 65536
}

function $shlU32(src1: int, p: int): int {
    (src1 * $pow(2, p)) mod 4294967296
}

function $shlU64(src1: int, p: int): int {
    (src1 * $pow(2, p)) mod 18446744073709551616
}

function $shlU128(src1: int, p: int): int {
    (src1 * $pow(2, p)) mod 340282366920938463463374607431768211456
}

function $shlU256(src1: int, p: int): int {
    (src1 * $pow(2, p)) mod 115792089237316195423570985008687907853269984665640564039457584007913129639936
}

function $shr(src1: int, p: int): int {
    src1 div $pow(2, p)
}

// We need to know the size of the destination in order to drop bits
// that have been shifted left more than that, so we have $ShlU8/16/32/64/128/256
procedure {:inline 1} $ShlU8(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 8) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shlU8(src1, src2);
}

// Template for cast and shift operations of bitvector types

procedure {:inline 1} $CastBv8to8(src: bv8) returns (dst: bv8)
{
    dst := src;
}


function $shlBv8From8(src1: bv8, src2: bv8) returns (bv8)
{
    $Shl'Bv8'(src1, src2)
}

procedure {:inline 1} $ShlBv8From8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    if ($Ge'Bv8'(src2, 8bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv8'(src1, src2);
}

function $shrBv8From8(src1: bv8, src2: bv8) returns (bv8)
{
    $Shr'Bv8'(src1, src2)
}

procedure {:inline 1} $ShrBv8From8(src1: bv8, src2: bv8) returns (dst: bv8)
{
    if ($Ge'Bv8'(src2, 8bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv8'(src1, src2);
}

procedure {:inline 1} $CastBv16to8(src: bv16) returns (dst: bv8)
{
    if ($Gt'Bv16'(src, 255bv16)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[8:0];
}


function $shlBv8From16(src1: bv8, src2: bv16) returns (bv8)
{
    $Shl'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShlBv8From16(src1: bv8, src2: bv16) returns (dst: bv8)
{
    if ($Ge'Bv16'(src2, 8bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv8'(src1, src2[8:0]);
}

function $shrBv8From16(src1: bv8, src2: bv16) returns (bv8)
{
    $Shr'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShrBv8From16(src1: bv8, src2: bv16) returns (dst: bv8)
{
    if ($Ge'Bv16'(src2, 8bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv8'(src1, src2[8:0]);
}

procedure {:inline 1} $CastBv32to8(src: bv32) returns (dst: bv8)
{
    if ($Gt'Bv32'(src, 255bv32)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[8:0];
}


function $shlBv8From32(src1: bv8, src2: bv32) returns (bv8)
{
    $Shl'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShlBv8From32(src1: bv8, src2: bv32) returns (dst: bv8)
{
    if ($Ge'Bv32'(src2, 8bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv8'(src1, src2[8:0]);
}

function $shrBv8From32(src1: bv8, src2: bv32) returns (bv8)
{
    $Shr'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShrBv8From32(src1: bv8, src2: bv32) returns (dst: bv8)
{
    if ($Ge'Bv32'(src2, 8bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv8'(src1, src2[8:0]);
}

procedure {:inline 1} $CastBv64to8(src: bv64) returns (dst: bv8)
{
    if ($Gt'Bv64'(src, 255bv64)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[8:0];
}


function $shlBv8From64(src1: bv8, src2: bv64) returns (bv8)
{
    $Shl'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShlBv8From64(src1: bv8, src2: bv64) returns (dst: bv8)
{
    if ($Ge'Bv64'(src2, 8bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv8'(src1, src2[8:0]);
}

function $shrBv8From64(src1: bv8, src2: bv64) returns (bv8)
{
    $Shr'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShrBv8From64(src1: bv8, src2: bv64) returns (dst: bv8)
{
    if ($Ge'Bv64'(src2, 8bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv8'(src1, src2[8:0]);
}

procedure {:inline 1} $CastBv128to8(src: bv128) returns (dst: bv8)
{
    if ($Gt'Bv128'(src, 255bv128)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[8:0];
}


function $shlBv8From128(src1: bv8, src2: bv128) returns (bv8)
{
    $Shl'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShlBv8From128(src1: bv8, src2: bv128) returns (dst: bv8)
{
    if ($Ge'Bv128'(src2, 8bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv8'(src1, src2[8:0]);
}

function $shrBv8From128(src1: bv8, src2: bv128) returns (bv8)
{
    $Shr'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShrBv8From128(src1: bv8, src2: bv128) returns (dst: bv8)
{
    if ($Ge'Bv128'(src2, 8bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv8'(src1, src2[8:0]);
}

procedure {:inline 1} $CastBv256to8(src: bv256) returns (dst: bv8)
{
    if ($Gt'Bv256'(src, 255bv256)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[8:0];
}


function $shlBv8From256(src1: bv8, src2: bv256) returns (bv8)
{
    $Shl'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShlBv8From256(src1: bv8, src2: bv256) returns (dst: bv8)
{
    if ($Ge'Bv256'(src2, 8bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv8'(src1, src2[8:0]);
}

function $shrBv8From256(src1: bv8, src2: bv256) returns (bv8)
{
    $Shr'Bv8'(src1, src2[8:0])
}

procedure {:inline 1} $ShrBv8From256(src1: bv8, src2: bv256) returns (dst: bv8)
{
    if ($Ge'Bv256'(src2, 8bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv8'(src1, src2[8:0]);
}

procedure {:inline 1} $CastBv8to16(src: bv8) returns (dst: bv16)
{
    dst := 0bv8 ++ src;
}


function $shlBv16From8(src1: bv16, src2: bv8) returns (bv16)
{
    $Shl'Bv16'(src1, 0bv8 ++ src2)
}

procedure {:inline 1} $ShlBv16From8(src1: bv16, src2: bv8) returns (dst: bv16)
{
    if ($Ge'Bv8'(src2, 16bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv16'(src1, 0bv8 ++ src2);
}

function $shrBv16From8(src1: bv16, src2: bv8) returns (bv16)
{
    $Shr'Bv16'(src1, 0bv8 ++ src2)
}

procedure {:inline 1} $ShrBv16From8(src1: bv16, src2: bv8) returns (dst: bv16)
{
    if ($Ge'Bv8'(src2, 16bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv16'(src1, 0bv8 ++ src2);
}

procedure {:inline 1} $CastBv16to16(src: bv16) returns (dst: bv16)
{
    dst := src;
}


function $shlBv16From16(src1: bv16, src2: bv16) returns (bv16)
{
    $Shl'Bv16'(src1, src2)
}

procedure {:inline 1} $ShlBv16From16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    if ($Ge'Bv16'(src2, 16bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv16'(src1, src2);
}

function $shrBv16From16(src1: bv16, src2: bv16) returns (bv16)
{
    $Shr'Bv16'(src1, src2)
}

procedure {:inline 1} $ShrBv16From16(src1: bv16, src2: bv16) returns (dst: bv16)
{
    if ($Ge'Bv16'(src2, 16bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv16'(src1, src2);
}

procedure {:inline 1} $CastBv32to16(src: bv32) returns (dst: bv16)
{
    if ($Gt'Bv32'(src, 65535bv32)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[16:0];
}


function $shlBv16From32(src1: bv16, src2: bv32) returns (bv16)
{
    $Shl'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShlBv16From32(src1: bv16, src2: bv32) returns (dst: bv16)
{
    if ($Ge'Bv32'(src2, 16bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv16'(src1, src2[16:0]);
}

function $shrBv16From32(src1: bv16, src2: bv32) returns (bv16)
{
    $Shr'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShrBv16From32(src1: bv16, src2: bv32) returns (dst: bv16)
{
    if ($Ge'Bv32'(src2, 16bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv16'(src1, src2[16:0]);
}

procedure {:inline 1} $CastBv64to16(src: bv64) returns (dst: bv16)
{
    if ($Gt'Bv64'(src, 65535bv64)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[16:0];
}


function $shlBv16From64(src1: bv16, src2: bv64) returns (bv16)
{
    $Shl'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShlBv16From64(src1: bv16, src2: bv64) returns (dst: bv16)
{
    if ($Ge'Bv64'(src2, 16bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv16'(src1, src2[16:0]);
}

function $shrBv16From64(src1: bv16, src2: bv64) returns (bv16)
{
    $Shr'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShrBv16From64(src1: bv16, src2: bv64) returns (dst: bv16)
{
    if ($Ge'Bv64'(src2, 16bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv16'(src1, src2[16:0]);
}

procedure {:inline 1} $CastBv128to16(src: bv128) returns (dst: bv16)
{
    if ($Gt'Bv128'(src, 65535bv128)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[16:0];
}


function $shlBv16From128(src1: bv16, src2: bv128) returns (bv16)
{
    $Shl'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShlBv16From128(src1: bv16, src2: bv128) returns (dst: bv16)
{
    if ($Ge'Bv128'(src2, 16bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv16'(src1, src2[16:0]);
}

function $shrBv16From128(src1: bv16, src2: bv128) returns (bv16)
{
    $Shr'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShrBv16From128(src1: bv16, src2: bv128) returns (dst: bv16)
{
    if ($Ge'Bv128'(src2, 16bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv16'(src1, src2[16:0]);
}

procedure {:inline 1} $CastBv256to16(src: bv256) returns (dst: bv16)
{
    if ($Gt'Bv256'(src, 65535bv256)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[16:0];
}


function $shlBv16From256(src1: bv16, src2: bv256) returns (bv16)
{
    $Shl'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShlBv16From256(src1: bv16, src2: bv256) returns (dst: bv16)
{
    if ($Ge'Bv256'(src2, 16bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv16'(src1, src2[16:0]);
}

function $shrBv16From256(src1: bv16, src2: bv256) returns (bv16)
{
    $Shr'Bv16'(src1, src2[16:0])
}

procedure {:inline 1} $ShrBv16From256(src1: bv16, src2: bv256) returns (dst: bv16)
{
    if ($Ge'Bv256'(src2, 16bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv16'(src1, src2[16:0]);
}

procedure {:inline 1} $CastBv8to32(src: bv8) returns (dst: bv32)
{
    dst := 0bv24 ++ src;
}


function $shlBv32From8(src1: bv32, src2: bv8) returns (bv32)
{
    $Shl'Bv32'(src1, 0bv24 ++ src2)
}

procedure {:inline 1} $ShlBv32From8(src1: bv32, src2: bv8) returns (dst: bv32)
{
    if ($Ge'Bv8'(src2, 32bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv32'(src1, 0bv24 ++ src2);
}

function $shrBv32From8(src1: bv32, src2: bv8) returns (bv32)
{
    $Shr'Bv32'(src1, 0bv24 ++ src2)
}

procedure {:inline 1} $ShrBv32From8(src1: bv32, src2: bv8) returns (dst: bv32)
{
    if ($Ge'Bv8'(src2, 32bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv32'(src1, 0bv24 ++ src2);
}

procedure {:inline 1} $CastBv16to32(src: bv16) returns (dst: bv32)
{
    dst := 0bv16 ++ src;
}


function $shlBv32From16(src1: bv32, src2: bv16) returns (bv32)
{
    $Shl'Bv32'(src1, 0bv16 ++ src2)
}

procedure {:inline 1} $ShlBv32From16(src1: bv32, src2: bv16) returns (dst: bv32)
{
    if ($Ge'Bv16'(src2, 32bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv32'(src1, 0bv16 ++ src2);
}

function $shrBv32From16(src1: bv32, src2: bv16) returns (bv32)
{
    $Shr'Bv32'(src1, 0bv16 ++ src2)
}

procedure {:inline 1} $ShrBv32From16(src1: bv32, src2: bv16) returns (dst: bv32)
{
    if ($Ge'Bv16'(src2, 32bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv32'(src1, 0bv16 ++ src2);
}

procedure {:inline 1} $CastBv32to32(src: bv32) returns (dst: bv32)
{
    dst := src;
}


function $shlBv32From32(src1: bv32, src2: bv32) returns (bv32)
{
    $Shl'Bv32'(src1, src2)
}

procedure {:inline 1} $ShlBv32From32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    if ($Ge'Bv32'(src2, 32bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv32'(src1, src2);
}

function $shrBv32From32(src1: bv32, src2: bv32) returns (bv32)
{
    $Shr'Bv32'(src1, src2)
}

procedure {:inline 1} $ShrBv32From32(src1: bv32, src2: bv32) returns (dst: bv32)
{
    if ($Ge'Bv32'(src2, 32bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv32'(src1, src2);
}

procedure {:inline 1} $CastBv64to32(src: bv64) returns (dst: bv32)
{
    if ($Gt'Bv64'(src, 2147483647bv64)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[32:0];
}


function $shlBv32From64(src1: bv32, src2: bv64) returns (bv32)
{
    $Shl'Bv32'(src1, src2[32:0])
}

procedure {:inline 1} $ShlBv32From64(src1: bv32, src2: bv64) returns (dst: bv32)
{
    if ($Ge'Bv64'(src2, 32bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv32'(src1, src2[32:0]);
}

function $shrBv32From64(src1: bv32, src2: bv64) returns (bv32)
{
    $Shr'Bv32'(src1, src2[32:0])
}

procedure {:inline 1} $ShrBv32From64(src1: bv32, src2: bv64) returns (dst: bv32)
{
    if ($Ge'Bv64'(src2, 32bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv32'(src1, src2[32:0]);
}

procedure {:inline 1} $CastBv128to32(src: bv128) returns (dst: bv32)
{
    if ($Gt'Bv128'(src, 2147483647bv128)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[32:0];
}


function $shlBv32From128(src1: bv32, src2: bv128) returns (bv32)
{
    $Shl'Bv32'(src1, src2[32:0])
}

procedure {:inline 1} $ShlBv32From128(src1: bv32, src2: bv128) returns (dst: bv32)
{
    if ($Ge'Bv128'(src2, 32bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv32'(src1, src2[32:0]);
}

function $shrBv32From128(src1: bv32, src2: bv128) returns (bv32)
{
    $Shr'Bv32'(src1, src2[32:0])
}

procedure {:inline 1} $ShrBv32From128(src1: bv32, src2: bv128) returns (dst: bv32)
{
    if ($Ge'Bv128'(src2, 32bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv32'(src1, src2[32:0]);
}

procedure {:inline 1} $CastBv256to32(src: bv256) returns (dst: bv32)
{
    if ($Gt'Bv256'(src, 2147483647bv256)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[32:0];
}


function $shlBv32From256(src1: bv32, src2: bv256) returns (bv32)
{
    $Shl'Bv32'(src1, src2[32:0])
}

procedure {:inline 1} $ShlBv32From256(src1: bv32, src2: bv256) returns (dst: bv32)
{
    if ($Ge'Bv256'(src2, 32bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv32'(src1, src2[32:0]);
}

function $shrBv32From256(src1: bv32, src2: bv256) returns (bv32)
{
    $Shr'Bv32'(src1, src2[32:0])
}

procedure {:inline 1} $ShrBv32From256(src1: bv32, src2: bv256) returns (dst: bv32)
{
    if ($Ge'Bv256'(src2, 32bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv32'(src1, src2[32:0]);
}

procedure {:inline 1} $CastBv8to64(src: bv8) returns (dst: bv64)
{
    dst := 0bv56 ++ src;
}


function $shlBv64From8(src1: bv64, src2: bv8) returns (bv64)
{
    $Shl'Bv64'(src1, 0bv56 ++ src2)
}

procedure {:inline 1} $ShlBv64From8(src1: bv64, src2: bv8) returns (dst: bv64)
{
    if ($Ge'Bv8'(src2, 64bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv64'(src1, 0bv56 ++ src2);
}

function $shrBv64From8(src1: bv64, src2: bv8) returns (bv64)
{
    $Shr'Bv64'(src1, 0bv56 ++ src2)
}

procedure {:inline 1} $ShrBv64From8(src1: bv64, src2: bv8) returns (dst: bv64)
{
    if ($Ge'Bv8'(src2, 64bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv64'(src1, 0bv56 ++ src2);
}

procedure {:inline 1} $CastBv16to64(src: bv16) returns (dst: bv64)
{
    dst := 0bv48 ++ src;
}


function $shlBv64From16(src1: bv64, src2: bv16) returns (bv64)
{
    $Shl'Bv64'(src1, 0bv48 ++ src2)
}

procedure {:inline 1} $ShlBv64From16(src1: bv64, src2: bv16) returns (dst: bv64)
{
    if ($Ge'Bv16'(src2, 64bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv64'(src1, 0bv48 ++ src2);
}

function $shrBv64From16(src1: bv64, src2: bv16) returns (bv64)
{
    $Shr'Bv64'(src1, 0bv48 ++ src2)
}

procedure {:inline 1} $ShrBv64From16(src1: bv64, src2: bv16) returns (dst: bv64)
{
    if ($Ge'Bv16'(src2, 64bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv64'(src1, 0bv48 ++ src2);
}

procedure {:inline 1} $CastBv32to64(src: bv32) returns (dst: bv64)
{
    dst := 0bv32 ++ src;
}


function $shlBv64From32(src1: bv64, src2: bv32) returns (bv64)
{
    $Shl'Bv64'(src1, 0bv32 ++ src2)
}

procedure {:inline 1} $ShlBv64From32(src1: bv64, src2: bv32) returns (dst: bv64)
{
    if ($Ge'Bv32'(src2, 64bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv64'(src1, 0bv32 ++ src2);
}

function $shrBv64From32(src1: bv64, src2: bv32) returns (bv64)
{
    $Shr'Bv64'(src1, 0bv32 ++ src2)
}

procedure {:inline 1} $ShrBv64From32(src1: bv64, src2: bv32) returns (dst: bv64)
{
    if ($Ge'Bv32'(src2, 64bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv64'(src1, 0bv32 ++ src2);
}

procedure {:inline 1} $CastBv64to64(src: bv64) returns (dst: bv64)
{
    dst := src;
}


function $shlBv64From64(src1: bv64, src2: bv64) returns (bv64)
{
    $Shl'Bv64'(src1, src2)
}

procedure {:inline 1} $ShlBv64From64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    if ($Ge'Bv64'(src2, 64bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv64'(src1, src2);
}

function $shrBv64From64(src1: bv64, src2: bv64) returns (bv64)
{
    $Shr'Bv64'(src1, src2)
}

procedure {:inline 1} $ShrBv64From64(src1: bv64, src2: bv64) returns (dst: bv64)
{
    if ($Ge'Bv64'(src2, 64bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv64'(src1, src2);
}

procedure {:inline 1} $CastBv128to64(src: bv128) returns (dst: bv64)
{
    if ($Gt'Bv128'(src, 18446744073709551615bv128)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[64:0];
}


function $shlBv64From128(src1: bv64, src2: bv128) returns (bv64)
{
    $Shl'Bv64'(src1, src2[64:0])
}

procedure {:inline 1} $ShlBv64From128(src1: bv64, src2: bv128) returns (dst: bv64)
{
    if ($Ge'Bv128'(src2, 64bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv64'(src1, src2[64:0]);
}

function $shrBv64From128(src1: bv64, src2: bv128) returns (bv64)
{
    $Shr'Bv64'(src1, src2[64:0])
}

procedure {:inline 1} $ShrBv64From128(src1: bv64, src2: bv128) returns (dst: bv64)
{
    if ($Ge'Bv128'(src2, 64bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv64'(src1, src2[64:0]);
}

procedure {:inline 1} $CastBv256to64(src: bv256) returns (dst: bv64)
{
    if ($Gt'Bv256'(src, 18446744073709551615bv256)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[64:0];
}


function $shlBv64From256(src1: bv64, src2: bv256) returns (bv64)
{
    $Shl'Bv64'(src1, src2[64:0])
}

procedure {:inline 1} $ShlBv64From256(src1: bv64, src2: bv256) returns (dst: bv64)
{
    if ($Ge'Bv256'(src2, 64bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv64'(src1, src2[64:0]);
}

function $shrBv64From256(src1: bv64, src2: bv256) returns (bv64)
{
    $Shr'Bv64'(src1, src2[64:0])
}

procedure {:inline 1} $ShrBv64From256(src1: bv64, src2: bv256) returns (dst: bv64)
{
    if ($Ge'Bv256'(src2, 64bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv64'(src1, src2[64:0]);
}

procedure {:inline 1} $CastBv8to128(src: bv8) returns (dst: bv128)
{
    dst := 0bv120 ++ src;
}


function $shlBv128From8(src1: bv128, src2: bv8) returns (bv128)
{
    $Shl'Bv128'(src1, 0bv120 ++ src2)
}

procedure {:inline 1} $ShlBv128From8(src1: bv128, src2: bv8) returns (dst: bv128)
{
    if ($Ge'Bv8'(src2, 128bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv128'(src1, 0bv120 ++ src2);
}

function $shrBv128From8(src1: bv128, src2: bv8) returns (bv128)
{
    $Shr'Bv128'(src1, 0bv120 ++ src2)
}

procedure {:inline 1} $ShrBv128From8(src1: bv128, src2: bv8) returns (dst: bv128)
{
    if ($Ge'Bv8'(src2, 128bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv128'(src1, 0bv120 ++ src2);
}

procedure {:inline 1} $CastBv16to128(src: bv16) returns (dst: bv128)
{
    dst := 0bv112 ++ src;
}


function $shlBv128From16(src1: bv128, src2: bv16) returns (bv128)
{
    $Shl'Bv128'(src1, 0bv112 ++ src2)
}

procedure {:inline 1} $ShlBv128From16(src1: bv128, src2: bv16) returns (dst: bv128)
{
    if ($Ge'Bv16'(src2, 128bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv128'(src1, 0bv112 ++ src2);
}

function $shrBv128From16(src1: bv128, src2: bv16) returns (bv128)
{
    $Shr'Bv128'(src1, 0bv112 ++ src2)
}

procedure {:inline 1} $ShrBv128From16(src1: bv128, src2: bv16) returns (dst: bv128)
{
    if ($Ge'Bv16'(src2, 128bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv128'(src1, 0bv112 ++ src2);
}

procedure {:inline 1} $CastBv32to128(src: bv32) returns (dst: bv128)
{
    dst := 0bv96 ++ src;
}


function $shlBv128From32(src1: bv128, src2: bv32) returns (bv128)
{
    $Shl'Bv128'(src1, 0bv96 ++ src2)
}

procedure {:inline 1} $ShlBv128From32(src1: bv128, src2: bv32) returns (dst: bv128)
{
    if ($Ge'Bv32'(src2, 128bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv128'(src1, 0bv96 ++ src2);
}

function $shrBv128From32(src1: bv128, src2: bv32) returns (bv128)
{
    $Shr'Bv128'(src1, 0bv96 ++ src2)
}

procedure {:inline 1} $ShrBv128From32(src1: bv128, src2: bv32) returns (dst: bv128)
{
    if ($Ge'Bv32'(src2, 128bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv128'(src1, 0bv96 ++ src2);
}

procedure {:inline 1} $CastBv64to128(src: bv64) returns (dst: bv128)
{
    dst := 0bv64 ++ src;
}


function $shlBv128From64(src1: bv128, src2: bv64) returns (bv128)
{
    $Shl'Bv128'(src1, 0bv64 ++ src2)
}

procedure {:inline 1} $ShlBv128From64(src1: bv128, src2: bv64) returns (dst: bv128)
{
    if ($Ge'Bv64'(src2, 128bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv128'(src1, 0bv64 ++ src2);
}

function $shrBv128From64(src1: bv128, src2: bv64) returns (bv128)
{
    $Shr'Bv128'(src1, 0bv64 ++ src2)
}

procedure {:inline 1} $ShrBv128From64(src1: bv128, src2: bv64) returns (dst: bv128)
{
    if ($Ge'Bv64'(src2, 128bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv128'(src1, 0bv64 ++ src2);
}

procedure {:inline 1} $CastBv128to128(src: bv128) returns (dst: bv128)
{
    dst := src;
}


function $shlBv128From128(src1: bv128, src2: bv128) returns (bv128)
{
    $Shl'Bv128'(src1, src2)
}

procedure {:inline 1} $ShlBv128From128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    if ($Ge'Bv128'(src2, 128bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv128'(src1, src2);
}

function $shrBv128From128(src1: bv128, src2: bv128) returns (bv128)
{
    $Shr'Bv128'(src1, src2)
}

procedure {:inline 1} $ShrBv128From128(src1: bv128, src2: bv128) returns (dst: bv128)
{
    if ($Ge'Bv128'(src2, 128bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv128'(src1, src2);
}

procedure {:inline 1} $CastBv256to128(src: bv256) returns (dst: bv128)
{
    if ($Gt'Bv256'(src, 340282366920938463463374607431768211455bv256)) {
            call $ExecFailureAbort();
            return;
    }
    dst := src[128:0];
}


function $shlBv128From256(src1: bv128, src2: bv256) returns (bv128)
{
    $Shl'Bv128'(src1, src2[128:0])
}

procedure {:inline 1} $ShlBv128From256(src1: bv128, src2: bv256) returns (dst: bv128)
{
    if ($Ge'Bv256'(src2, 128bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv128'(src1, src2[128:0]);
}

function $shrBv128From256(src1: bv128, src2: bv256) returns (bv128)
{
    $Shr'Bv128'(src1, src2[128:0])
}

procedure {:inline 1} $ShrBv128From256(src1: bv128, src2: bv256) returns (dst: bv128)
{
    if ($Ge'Bv256'(src2, 128bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv128'(src1, src2[128:0]);
}

procedure {:inline 1} $CastBv8to256(src: bv8) returns (dst: bv256)
{
    dst := 0bv248 ++ src;
}


function $shlBv256From8(src1: bv256, src2: bv8) returns (bv256)
{
    $Shl'Bv256'(src1, 0bv248 ++ src2)
}

procedure {:inline 1} $ShlBv256From8(src1: bv256, src2: bv8) returns (dst: bv256)
{
    if ($Ge'Bv8'(src2, 256bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv256'(src1, 0bv248 ++ src2);
}

function $shrBv256From8(src1: bv256, src2: bv8) returns (bv256)
{
    $Shr'Bv256'(src1, 0bv248 ++ src2)
}

procedure {:inline 1} $ShrBv256From8(src1: bv256, src2: bv8) returns (dst: bv256)
{
    if ($Ge'Bv8'(src2, 256bv8)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv256'(src1, 0bv248 ++ src2);
}

procedure {:inline 1} $CastBv16to256(src: bv16) returns (dst: bv256)
{
    dst := 0bv240 ++ src;
}


function $shlBv256From16(src1: bv256, src2: bv16) returns (bv256)
{
    $Shl'Bv256'(src1, 0bv240 ++ src2)
}

procedure {:inline 1} $ShlBv256From16(src1: bv256, src2: bv16) returns (dst: bv256)
{
    if ($Ge'Bv16'(src2, 256bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv256'(src1, 0bv240 ++ src2);
}

function $shrBv256From16(src1: bv256, src2: bv16) returns (bv256)
{
    $Shr'Bv256'(src1, 0bv240 ++ src2)
}

procedure {:inline 1} $ShrBv256From16(src1: bv256, src2: bv16) returns (dst: bv256)
{
    if ($Ge'Bv16'(src2, 256bv16)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv256'(src1, 0bv240 ++ src2);
}

procedure {:inline 1} $CastBv32to256(src: bv32) returns (dst: bv256)
{
    dst := 0bv224 ++ src;
}


function $shlBv256From32(src1: bv256, src2: bv32) returns (bv256)
{
    $Shl'Bv256'(src1, 0bv224 ++ src2)
}

procedure {:inline 1} $ShlBv256From32(src1: bv256, src2: bv32) returns (dst: bv256)
{
    if ($Ge'Bv32'(src2, 256bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv256'(src1, 0bv224 ++ src2);
}

function $shrBv256From32(src1: bv256, src2: bv32) returns (bv256)
{
    $Shr'Bv256'(src1, 0bv224 ++ src2)
}

procedure {:inline 1} $ShrBv256From32(src1: bv256, src2: bv32) returns (dst: bv256)
{
    if ($Ge'Bv32'(src2, 256bv32)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv256'(src1, 0bv224 ++ src2);
}

procedure {:inline 1} $CastBv64to256(src: bv64) returns (dst: bv256)
{
    dst := 0bv192 ++ src;
}


function $shlBv256From64(src1: bv256, src2: bv64) returns (bv256)
{
    $Shl'Bv256'(src1, 0bv192 ++ src2)
}

procedure {:inline 1} $ShlBv256From64(src1: bv256, src2: bv64) returns (dst: bv256)
{
    if ($Ge'Bv64'(src2, 256bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv256'(src1, 0bv192 ++ src2);
}

function $shrBv256From64(src1: bv256, src2: bv64) returns (bv256)
{
    $Shr'Bv256'(src1, 0bv192 ++ src2)
}

procedure {:inline 1} $ShrBv256From64(src1: bv256, src2: bv64) returns (dst: bv256)
{
    if ($Ge'Bv64'(src2, 256bv64)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv256'(src1, 0bv192 ++ src2);
}

procedure {:inline 1} $CastBv128to256(src: bv128) returns (dst: bv256)
{
    dst := 0bv128 ++ src;
}


function $shlBv256From128(src1: bv256, src2: bv128) returns (bv256)
{
    $Shl'Bv256'(src1, 0bv128 ++ src2)
}

procedure {:inline 1} $ShlBv256From128(src1: bv256, src2: bv128) returns (dst: bv256)
{
    if ($Ge'Bv128'(src2, 256bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv256'(src1, 0bv128 ++ src2);
}

function $shrBv256From128(src1: bv256, src2: bv128) returns (bv256)
{
    $Shr'Bv256'(src1, 0bv128 ++ src2)
}

procedure {:inline 1} $ShrBv256From128(src1: bv256, src2: bv128) returns (dst: bv256)
{
    if ($Ge'Bv128'(src2, 256bv128)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv256'(src1, 0bv128 ++ src2);
}

procedure {:inline 1} $CastBv256to256(src: bv256) returns (dst: bv256)
{
    dst := src;
}


function $shlBv256From256(src1: bv256, src2: bv256) returns (bv256)
{
    $Shl'Bv256'(src1, src2)
}

procedure {:inline 1} $ShlBv256From256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    if ($Ge'Bv256'(src2, 256bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shl'Bv256'(src1, src2);
}

function $shrBv256From256(src1: bv256, src2: bv256) returns (bv256)
{
    $Shr'Bv256'(src1, src2)
}

procedure {:inline 1} $ShrBv256From256(src1: bv256, src2: bv256) returns (dst: bv256)
{
    if ($Ge'Bv256'(src2, 256bv256)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Shr'Bv256'(src1, src2);
}

procedure {:inline 1} $ShlU16(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 16) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shlU16(src1, src2);
}

procedure {:inline 1} $ShlU32(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 32) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shlU32(src1, src2);
}

procedure {:inline 1} $ShlU64(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 64) {
       call $ExecFailureAbort();
       return;
    }
    dst := $shlU64(src1, src2);
}

procedure {:inline 1} $ShlU128(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 128) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shlU128(src1, src2);
}

procedure {:inline 1} $ShlU256(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    dst := $shlU256(src1, src2);
}

procedure {:inline 1} $Shr(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    dst := $shr(src1, src2);
}

procedure {:inline 1} $ShrU8(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 8) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shr(src1, src2);
}

procedure {:inline 1} $ShrU16(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 16) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shr(src1, src2);
}

procedure {:inline 1} $ShrU32(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 32) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shr(src1, src2);
}

procedure {:inline 1} $ShrU64(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 64) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shr(src1, src2);
}

procedure {:inline 1} $ShrU128(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    if (src2 >= 128) {
        call $ExecFailureAbort();
        return;
    }
    dst := $shr(src1, src2);
}

procedure {:inline 1} $ShrU256(src1: int, src2: int) returns (dst: int)
{
    var res: int;
    // src2 is a u8
    assume src2 >= 0 && src2 < 256;
    dst := $shr(src1, src2);
}

procedure {:inline 1} $MulU8(src1: int, src2: int) returns (dst: int)
{
    if (src1 * src2 > $MAX_U8) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 * src2;
}

procedure {:inline 1} $MulU16(src1: int, src2: int) returns (dst: int)
{
    if (src1 * src2 > $MAX_U16) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 * src2;
}

procedure {:inline 1} $MulU32(src1: int, src2: int) returns (dst: int)
{
    if (src1 * src2 > $MAX_U32) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 * src2;
}

procedure {:inline 1} $MulU64(src1: int, src2: int) returns (dst: int)
{
    if (src1 * src2 > $MAX_U64) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 * src2;
}

procedure {:inline 1} $MulU128(src1: int, src2: int) returns (dst: int)
{
    if (src1 * src2 > $MAX_U128) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 * src2;
}

procedure {:inline 1} $MulU256(src1: int, src2: int) returns (dst: int)
{
    if (src1 * src2 > $MAX_U256) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 * src2;
}

procedure {:inline 1} $Div(src1: int, src2: int) returns (dst: int)
{
    if (src2 == 0) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 div src2;
}

procedure {:inline 1} $Mod(src1: int, src2: int) returns (dst: int)
{
    if (src2 == 0) {
        call $ExecFailureAbort();
        return;
    }
    dst := src1 mod src2;
}

procedure {:inline 1} $ArithBinaryUnimplemented(src1: int, src2: int) returns (dst: int);

procedure {:inline 1} $Lt(src1: int, src2: int) returns (dst: bool)
{
    dst := src1 < src2;
}

procedure {:inline 1} $Gt(src1: int, src2: int) returns (dst: bool)
{
    dst := src1 > src2;
}

procedure {:inline 1} $Le(src1: int, src2: int) returns (dst: bool)
{
    dst := src1 <= src2;
}

procedure {:inline 1} $Ge(src1: int, src2: int) returns (dst: bool)
{
    dst := src1 >= src2;
}

procedure {:inline 1} $And(src1: bool, src2: bool) returns (dst: bool)
{
    dst := src1 && src2;
}

procedure {:inline 1} $Or(src1: bool, src2: bool) returns (dst: bool)
{
    dst := src1 || src2;
}

procedure {:inline 1} $Not(src: bool) returns (dst: bool)
{
    dst := !src;
}

// Pack and Unpack are auto-generated for each type T


// ==================================================================================
// Native Vector

function {:inline} $SliceVecByRange<T>(v: Vec T, r: $Range): Vec T {
    SliceVec(v, r->lb, r->ub)
}

// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `#0`

// Not inlined. It appears faster this way.
function $IsEqual'vec'#0''(v1: Vec (#0), v2: Vec (#0)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'#0'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'#0''(v: Vec (#0), prefix: Vec (#0)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'#0'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'#0''(v: Vec (#0), suffix: Vec (#0)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'#0'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'#0''(v: Vec (#0)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'#0'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'#0'(v: Vec (#0), e: #0): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'#0'(ReadVec(v, i), e))
}

function $IndexOfVec'#0'(v: Vec (#0), e: #0): int;
axiom (forall v: Vec (#0), e: #0:: {$IndexOfVec'#0'(v, e)}
    (var i := $IndexOfVec'#0'(v, e);
     if (!$ContainsVec'#0'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'#0'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'#0'(ReadVec(v, j), e))));


function {:inline} $RangeVec'#0'(v: Vec (#0)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'#0'(): Vec (#0) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'#0'() returns (v: Vec (#0)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'#0'(): Vec (#0) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'#0'(v: Vec (#0)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'#0'(m: $Mutation (Vec (#0)), val: #0) returns (m': $Mutation (Vec (#0))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'#0'(v: Vec (#0), val: #0): Vec (#0) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'#0'(m: $Mutation (Vec (#0))) returns (e: #0, m': $Mutation (Vec (#0))) {
    var v: Vec (#0);
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

procedure {:inline 1} $1_vector_append'#0'(m: $Mutation (Vec (#0)), other: Vec (#0)) returns (m': $Mutation (Vec (#0))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'#0'(m: $Mutation (Vec (#0))) returns (m': $Mutation (Vec (#0))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'#0'(m: $Mutation (Vec (#0)), other: Vec (#0)) returns (m': $Mutation (Vec (#0))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'#0'(m: $Mutation (Vec (#0)), new_len: int) returns (v: (Vec (#0)), m': $Mutation (Vec (#0))) {
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

procedure {:inline 1} $1_vector_trim'#0'(m: $Mutation (Vec (#0)), new_len: int) returns (v: (Vec (#0)), m': $Mutation (Vec (#0))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'#0'(m: $Mutation (Vec (#0)), left: int, right: int) returns (m': $Mutation (Vec (#0))) {
    var left_vec: Vec (#0);
    var mid_vec: Vec (#0);
    var right_vec: Vec (#0);
    var v: Vec (#0);
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

procedure {:inline 1} $1_vector_rotate'#0'(m: $Mutation (Vec (#0)), rot: int) returns (n: int, m': $Mutation (Vec (#0))) {
    var v: Vec (#0);
    var len: int;
    var left_vec: Vec (#0);
    var right_vec: Vec (#0);
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

procedure {:inline 1} $1_vector_rotate_slice'#0'(m: $Mutation (Vec (#0)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (#0))) {
    var left_vec: Vec (#0);
    var mid_vec: Vec (#0);
    var right_vec: Vec (#0);
    var mid_left_vec: Vec (#0);
    var mid_right_vec: Vec (#0);
    var v: Vec (#0);
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

procedure {:inline 1} $1_vector_insert'#0'(m: $Mutation (Vec (#0)), i: int, e: #0) returns (m': $Mutation (Vec (#0))) {
    var left_vec: Vec (#0);
    var right_vec: Vec (#0);
    var v: Vec (#0);
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

procedure {:inline 1} $1_vector_length'#0'(v: Vec (#0)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'#0'(v: Vec (#0)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'#0'(v: Vec (#0), i: int) returns (dst: #0) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'#0'(v: Vec (#0), i: int): #0 {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'#0'(m: $Mutation (Vec (#0)), index: int)
returns (dst: $Mutation (#0), m': $Mutation (Vec (#0)))
{
    var v: Vec (#0);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(m->l, ExtendVec(m->p, index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'#0'(v: Vec (#0), i: int): #0 {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'#0'(v: Vec (#0)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'#0'(m: $Mutation (Vec (#0)), i: int, j: int) returns (m': $Mutation (Vec (#0)))
{
    var v: Vec (#0);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'#0'(v: Vec (#0), i: int, j: int): Vec (#0) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'#0'(m: $Mutation (Vec (#0)), i: int) returns (e: #0, m': $Mutation (Vec (#0)))
{
    var v: Vec (#0);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'#0'(m: $Mutation (Vec (#0)), i: int) returns (e: #0, m': $Mutation (Vec (#0)))
{
    var len: int;
    var v: Vec (#0);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'#0'(v: Vec (#0), e: #0) returns (res: bool)  {
    res := $ContainsVec'#0'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'#0'(v: Vec (#0), e: #0) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'#0'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `u128`

// Not inlined. It appears faster this way.
function $IsEqual'vec'u128''(v1: Vec (int), v2: Vec (int)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'u128'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'u128''(v: Vec (int), prefix: Vec (int)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'u128'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'u128''(v: Vec (int), suffix: Vec (int)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'u128'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'u128''(v: Vec (int)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'u128'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'u128'(v: Vec (int), e: int): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'u128'(ReadVec(v, i), e))
}

function $IndexOfVec'u128'(v: Vec (int), e: int): int;
axiom (forall v: Vec (int), e: int:: {$IndexOfVec'u128'(v, e)}
    (var i := $IndexOfVec'u128'(v, e);
     if (!$ContainsVec'u128'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'u128'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'u128'(ReadVec(v, j), e))));


function {:inline} $RangeVec'u128'(v: Vec (int)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'u128'(): Vec (int) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'u128'() returns (v: Vec (int)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'u128'(): Vec (int) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'u128'(v: Vec (int)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'u128'(m: $Mutation (Vec (int)), val: int) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'u128'(v: Vec (int), val: int): Vec (int) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'u128'(m: $Mutation (Vec (int))) returns (e: int, m': $Mutation (Vec (int))) {
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_append'u128'(m: $Mutation (Vec (int)), other: Vec (int)) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'u128'(m: $Mutation (Vec (int))) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'u128'(m: $Mutation (Vec (int)), other: Vec (int)) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'u128'(m: $Mutation (Vec (int)), new_len: int) returns (v: (Vec (int)), m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_trim'u128'(m: $Mutation (Vec (int)), new_len: int) returns (v: (Vec (int)), m': $Mutation (Vec (int))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'u128'(m: $Mutation (Vec (int)), left: int, right: int) returns (m': $Mutation (Vec (int))) {
    var left_vec: Vec (int);
    var mid_vec: Vec (int);
    var right_vec: Vec (int);
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_rotate'u128'(m: $Mutation (Vec (int)), rot: int) returns (n: int, m': $Mutation (Vec (int))) {
    var v: Vec (int);
    var len: int;
    var left_vec: Vec (int);
    var right_vec: Vec (int);
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

procedure {:inline 1} $1_vector_rotate_slice'u128'(m: $Mutation (Vec (int)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (int))) {
    var left_vec: Vec (int);
    var mid_vec: Vec (int);
    var right_vec: Vec (int);
    var mid_left_vec: Vec (int);
    var mid_right_vec: Vec (int);
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_insert'u128'(m: $Mutation (Vec (int)), i: int, e: int) returns (m': $Mutation (Vec (int))) {
    var left_vec: Vec (int);
    var right_vec: Vec (int);
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_length'u128'(v: Vec (int)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'u128'(v: Vec (int)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'u128'(v: Vec (int), i: int) returns (dst: int) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'u128'(v: Vec (int), i: int): int {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'u128'(m: $Mutation (Vec (int)), index: int)
returns (dst: $Mutation (int), m': $Mutation (Vec (int)))
{
    var v: Vec (int);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(m->l, ExtendVec(m->p, index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'u128'(v: Vec (int), i: int): int {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'u128'(v: Vec (int)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'u128'(m: $Mutation (Vec (int)), i: int, j: int) returns (m': $Mutation (Vec (int)))
{
    var v: Vec (int);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'u128'(v: Vec (int), i: int, j: int): Vec (int) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'u128'(m: $Mutation (Vec (int)), i: int) returns (e: int, m': $Mutation (Vec (int)))
{
    var v: Vec (int);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'u128'(m: $Mutation (Vec (int)), i: int) returns (e: int, m': $Mutation (Vec (int)))
{
    var len: int;
    var v: Vec (int);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'u128'(v: Vec (int), e: int) returns (res: bool)  {
    res := $ContainsVec'u128'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'u128'(v: Vec (int), e: int) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'u128'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `u8`

// Not inlined. It appears faster this way.
function $IsEqual'vec'u8''(v1: Vec (int), v2: Vec (int)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'u8'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'u8''(v: Vec (int), prefix: Vec (int)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'u8'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'u8''(v: Vec (int), suffix: Vec (int)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'u8'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'u8''(v: Vec (int)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'u8'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'u8'(v: Vec (int), e: int): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'u8'(ReadVec(v, i), e))
}

function $IndexOfVec'u8'(v: Vec (int), e: int): int;
axiom (forall v: Vec (int), e: int:: {$IndexOfVec'u8'(v, e)}
    (var i := $IndexOfVec'u8'(v, e);
     if (!$ContainsVec'u8'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'u8'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'u8'(ReadVec(v, j), e))));


function {:inline} $RangeVec'u8'(v: Vec (int)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'u8'(): Vec (int) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'u8'() returns (v: Vec (int)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'u8'(): Vec (int) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'u8'(v: Vec (int)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'u8'(m: $Mutation (Vec (int)), val: int) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'u8'(v: Vec (int), val: int): Vec (int) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'u8'(m: $Mutation (Vec (int))) returns (e: int, m': $Mutation (Vec (int))) {
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_append'u8'(m: $Mutation (Vec (int)), other: Vec (int)) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'u8'(m: $Mutation (Vec (int))) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'u8'(m: $Mutation (Vec (int)), other: Vec (int)) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'u8'(m: $Mutation (Vec (int)), new_len: int) returns (v: (Vec (int)), m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_trim'u8'(m: $Mutation (Vec (int)), new_len: int) returns (v: (Vec (int)), m': $Mutation (Vec (int))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'u8'(m: $Mutation (Vec (int)), left: int, right: int) returns (m': $Mutation (Vec (int))) {
    var left_vec: Vec (int);
    var mid_vec: Vec (int);
    var right_vec: Vec (int);
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_rotate'u8'(m: $Mutation (Vec (int)), rot: int) returns (n: int, m': $Mutation (Vec (int))) {
    var v: Vec (int);
    var len: int;
    var left_vec: Vec (int);
    var right_vec: Vec (int);
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

procedure {:inline 1} $1_vector_rotate_slice'u8'(m: $Mutation (Vec (int)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (int))) {
    var left_vec: Vec (int);
    var mid_vec: Vec (int);
    var right_vec: Vec (int);
    var mid_left_vec: Vec (int);
    var mid_right_vec: Vec (int);
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_insert'u8'(m: $Mutation (Vec (int)), i: int, e: int) returns (m': $Mutation (Vec (int))) {
    var left_vec: Vec (int);
    var right_vec: Vec (int);
    var v: Vec (int);
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

procedure {:inline 1} $1_vector_length'u8'(v: Vec (int)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'u8'(v: Vec (int)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'u8'(v: Vec (int), i: int) returns (dst: int) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'u8'(v: Vec (int), i: int): int {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'u8'(m: $Mutation (Vec (int)), index: int)
returns (dst: $Mutation (int), m': $Mutation (Vec (int)))
{
    var v: Vec (int);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(m->l, ExtendVec(m->p, index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'u8'(v: Vec (int), i: int): int {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'u8'(v: Vec (int)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'u8'(m: $Mutation (Vec (int)), i: int, j: int) returns (m': $Mutation (Vec (int)))
{
    var v: Vec (int);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'u8'(v: Vec (int), i: int, j: int): Vec (int) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'u8'(m: $Mutation (Vec (int)), i: int) returns (e: int, m': $Mutation (Vec (int)))
{
    var v: Vec (int);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'u8'(m: $Mutation (Vec (int)), i: int) returns (e: int, m': $Mutation (Vec (int)))
{
    var len: int;
    var v: Vec (int);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'u8'(v: Vec (int), e: int) returns (res: bool)  {
    res := $ContainsVec'u8'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'u8'(v: Vec (int), e: int) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'u8'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `bv128`

// Not inlined. It appears faster this way.
function $IsEqual'vec'bv128''(v1: Vec (bv128), v2: Vec (bv128)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'bv128'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'bv128''(v: Vec (bv128), prefix: Vec (bv128)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'bv128'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'bv128''(v: Vec (bv128), suffix: Vec (bv128)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'bv128'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'bv128''(v: Vec (bv128)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'bv128'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'bv128'(v: Vec (bv128), e: bv128): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'bv128'(ReadVec(v, i), e))
}

function $IndexOfVec'bv128'(v: Vec (bv128), e: bv128): int;
axiom (forall v: Vec (bv128), e: bv128:: {$IndexOfVec'bv128'(v, e)}
    (var i := $IndexOfVec'bv128'(v, e);
     if (!$ContainsVec'bv128'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'bv128'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'bv128'(ReadVec(v, j), e))));


function {:inline} $RangeVec'bv128'(v: Vec (bv128)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'bv128'(): Vec (bv128) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'bv128'() returns (v: Vec (bv128)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'bv128'(): Vec (bv128) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'bv128'(v: Vec (bv128)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'bv128'(m: $Mutation (Vec (bv128)), val: bv128) returns (m': $Mutation (Vec (bv128))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'bv128'(v: Vec (bv128), val: bv128): Vec (bv128) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'bv128'(m: $Mutation (Vec (bv128))) returns (e: bv128, m': $Mutation (Vec (bv128))) {
    var v: Vec (bv128);
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

procedure {:inline 1} $1_vector_append'bv128'(m: $Mutation (Vec (bv128)), other: Vec (bv128)) returns (m': $Mutation (Vec (bv128))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'bv128'(m: $Mutation (Vec (bv128))) returns (m': $Mutation (Vec (bv128))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'bv128'(m: $Mutation (Vec (bv128)), other: Vec (bv128)) returns (m': $Mutation (Vec (bv128))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'bv128'(m: $Mutation (Vec (bv128)), new_len: int) returns (v: (Vec (bv128)), m': $Mutation (Vec (bv128))) {
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

procedure {:inline 1} $1_vector_trim'bv128'(m: $Mutation (Vec (bv128)), new_len: int) returns (v: (Vec (bv128)), m': $Mutation (Vec (bv128))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'bv128'(m: $Mutation (Vec (bv128)), left: int, right: int) returns (m': $Mutation (Vec (bv128))) {
    var left_vec: Vec (bv128);
    var mid_vec: Vec (bv128);
    var right_vec: Vec (bv128);
    var v: Vec (bv128);
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

procedure {:inline 1} $1_vector_rotate'bv128'(m: $Mutation (Vec (bv128)), rot: int) returns (n: int, m': $Mutation (Vec (bv128))) {
    var v: Vec (bv128);
    var len: int;
    var left_vec: Vec (bv128);
    var right_vec: Vec (bv128);
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

procedure {:inline 1} $1_vector_rotate_slice'bv128'(m: $Mutation (Vec (bv128)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (bv128))) {
    var left_vec: Vec (bv128);
    var mid_vec: Vec (bv128);
    var right_vec: Vec (bv128);
    var mid_left_vec: Vec (bv128);
    var mid_right_vec: Vec (bv128);
    var v: Vec (bv128);
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

procedure {:inline 1} $1_vector_insert'bv128'(m: $Mutation (Vec (bv128)), i: int, e: bv128) returns (m': $Mutation (Vec (bv128))) {
    var left_vec: Vec (bv128);
    var right_vec: Vec (bv128);
    var v: Vec (bv128);
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

procedure {:inline 1} $1_vector_length'bv128'(v: Vec (bv128)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'bv128'(v: Vec (bv128)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'bv128'(v: Vec (bv128), i: int) returns (dst: bv128) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'bv128'(v: Vec (bv128), i: int): bv128 {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'bv128'(m: $Mutation (Vec (bv128)), index: int)
returns (dst: $Mutation (bv128), m': $Mutation (Vec (bv128)))
{
    var v: Vec (bv128);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(m->l, ExtendVec(m->p, index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'bv128'(v: Vec (bv128), i: int): bv128 {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'bv128'(v: Vec (bv128)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'bv128'(m: $Mutation (Vec (bv128)), i: int, j: int) returns (m': $Mutation (Vec (bv128)))
{
    var v: Vec (bv128);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'bv128'(v: Vec (bv128), i: int, j: int): Vec (bv128) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'bv128'(m: $Mutation (Vec (bv128)), i: int) returns (e: bv128, m': $Mutation (Vec (bv128)))
{
    var v: Vec (bv128);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'bv128'(m: $Mutation (Vec (bv128)), i: int) returns (e: bv128, m': $Mutation (Vec (bv128)))
{
    var len: int;
    var v: Vec (bv128);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'bv128'(v: Vec (bv128), e: bv128) returns (res: bool)  {
    res := $ContainsVec'bv128'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'bv128'(v: Vec (bv128), e: bv128) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'bv128'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `bv8`

// Not inlined. It appears faster this way.
function $IsEqual'vec'bv8''(v1: Vec (bv8), v2: Vec (bv8)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'bv8'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'bv8''(v: Vec (bv8), prefix: Vec (bv8)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'bv8'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'bv8''(v: Vec (bv8), suffix: Vec (bv8)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'bv8'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'bv8''(v: Vec (bv8)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'bv8'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'bv8'(v: Vec (bv8), e: bv8): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'bv8'(ReadVec(v, i), e))
}

function $IndexOfVec'bv8'(v: Vec (bv8), e: bv8): int;
axiom (forall v: Vec (bv8), e: bv8:: {$IndexOfVec'bv8'(v, e)}
    (var i := $IndexOfVec'bv8'(v, e);
     if (!$ContainsVec'bv8'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'bv8'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'bv8'(ReadVec(v, j), e))));


function {:inline} $RangeVec'bv8'(v: Vec (bv8)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'bv8'(): Vec (bv8) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'bv8'() returns (v: Vec (bv8)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'bv8'(): Vec (bv8) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'bv8'(v: Vec (bv8)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'bv8'(m: $Mutation (Vec (bv8)), val: bv8) returns (m': $Mutation (Vec (bv8))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'bv8'(v: Vec (bv8), val: bv8): Vec (bv8) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'bv8'(m: $Mutation (Vec (bv8))) returns (e: bv8, m': $Mutation (Vec (bv8))) {
    var v: Vec (bv8);
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

procedure {:inline 1} $1_vector_append'bv8'(m: $Mutation (Vec (bv8)), other: Vec (bv8)) returns (m': $Mutation (Vec (bv8))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'bv8'(m: $Mutation (Vec (bv8))) returns (m': $Mutation (Vec (bv8))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'bv8'(m: $Mutation (Vec (bv8)), other: Vec (bv8)) returns (m': $Mutation (Vec (bv8))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'bv8'(m: $Mutation (Vec (bv8)), new_len: int) returns (v: (Vec (bv8)), m': $Mutation (Vec (bv8))) {
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

procedure {:inline 1} $1_vector_trim'bv8'(m: $Mutation (Vec (bv8)), new_len: int) returns (v: (Vec (bv8)), m': $Mutation (Vec (bv8))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'bv8'(m: $Mutation (Vec (bv8)), left: int, right: int) returns (m': $Mutation (Vec (bv8))) {
    var left_vec: Vec (bv8);
    var mid_vec: Vec (bv8);
    var right_vec: Vec (bv8);
    var v: Vec (bv8);
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

procedure {:inline 1} $1_vector_rotate'bv8'(m: $Mutation (Vec (bv8)), rot: int) returns (n: int, m': $Mutation (Vec (bv8))) {
    var v: Vec (bv8);
    var len: int;
    var left_vec: Vec (bv8);
    var right_vec: Vec (bv8);
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

procedure {:inline 1} $1_vector_rotate_slice'bv8'(m: $Mutation (Vec (bv8)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (bv8))) {
    var left_vec: Vec (bv8);
    var mid_vec: Vec (bv8);
    var right_vec: Vec (bv8);
    var mid_left_vec: Vec (bv8);
    var mid_right_vec: Vec (bv8);
    var v: Vec (bv8);
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

procedure {:inline 1} $1_vector_insert'bv8'(m: $Mutation (Vec (bv8)), i: int, e: bv8) returns (m': $Mutation (Vec (bv8))) {
    var left_vec: Vec (bv8);
    var right_vec: Vec (bv8);
    var v: Vec (bv8);
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

procedure {:inline 1} $1_vector_length'bv8'(v: Vec (bv8)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'bv8'(v: Vec (bv8)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'bv8'(v: Vec (bv8), i: int) returns (dst: bv8) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'bv8'(v: Vec (bv8), i: int): bv8 {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'bv8'(m: $Mutation (Vec (bv8)), index: int)
returns (dst: $Mutation (bv8), m': $Mutation (Vec (bv8)))
{
    var v: Vec (bv8);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(m->l, ExtendVec(m->p, index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'bv8'(v: Vec (bv8), i: int): bv8 {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'bv8'(v: Vec (bv8)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'bv8'(m: $Mutation (Vec (bv8)), i: int, j: int) returns (m': $Mutation (Vec (bv8)))
{
    var v: Vec (bv8);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'bv8'(v: Vec (bv8), i: int, j: int): Vec (bv8) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'bv8'(m: $Mutation (Vec (bv8)), i: int) returns (e: bv8, m': $Mutation (Vec (bv8)))
{
    var v: Vec (bv8);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'bv8'(m: $Mutation (Vec (bv8)), i: int) returns (e: bv8, m': $Mutation (Vec (bv8)))
{
    var len: int;
    var v: Vec (bv8);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'bv8'(v: Vec (bv8), e: bv8) returns (res: bool)  {
    res := $ContainsVec'bv8'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'bv8'(v: Vec (bv8), e: bv8) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'bv8'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ==================================================================================
// Native Table

// ==================================================================================
// Native Hash

// Hash is modeled as an otherwise uninterpreted injection.
// In truth, it is not an injection since the domain has greater cardinality
// (arbitrary length vectors) than the co-domain (vectors of length 32).  But it is
// common to assume in code there are no hash collisions in practice.  Fortunately,
// Boogie is not smart enough to recognized that there is an inconsistency.
// FIXME: If we were using a reliable extensional theory of arrays, and if we could use ==
// instead of $IsEqual, we might be able to avoid so many quantified formulas by
// using a sha2_inverse function in the ensures conditions of Hash_sha2_256 to
// assert that sha2/3 are injections without using global quantified axioms.


function $1_hash_sha2(val: Vec int): Vec int;

// This says that Hash_sha2 is bijective.
axiom (forall v1,v2: Vec int :: {$1_hash_sha2(v1), $1_hash_sha2(v2)}
       $IsEqual'vec'u8''(v1, v2) <==> $IsEqual'vec'u8''($1_hash_sha2(v1), $1_hash_sha2(v2)));

procedure $1_hash_sha2_256(val: Vec int) returns (res: Vec int);
ensures res == $1_hash_sha2(val);     // returns Hash_sha2 Value
ensures $IsValid'vec'u8''(res);    // result is a legal vector of U8s.
ensures LenVec(res) == 32;               // result is 32 bytes.

// Spec version of Move native function.
function {:inline} $1_hash_$sha2_256(val: Vec int): Vec int {
    $1_hash_sha2(val)
}

// similarly for Hash_sha3
function $1_hash_sha3(val: Vec int): Vec int;

axiom (forall v1,v2: Vec int :: {$1_hash_sha3(v1), $1_hash_sha3(v2)}
       $IsEqual'vec'u8''(v1, v2) <==> $IsEqual'vec'u8''($1_hash_sha3(v1), $1_hash_sha3(v2)));

procedure $1_hash_sha3_256(val: Vec int) returns (res: Vec int);
ensures res == $1_hash_sha3(val);     // returns Hash_sha3 Value
ensures $IsValid'vec'u8''(res);    // result is a legal vector of U8s.
ensures LenVec(res) == 32;               // result is 32 bytes.

// Spec version of Move native function.
function {:inline} $1_hash_$sha3_256(val: Vec int): Vec int {
    $1_hash_sha3(val)
}

// ==================================================================================
// Native string

// TODO: correct implementation of strings

procedure {:inline 1} $1_string_internal_check_utf8(x: Vec int) returns (r: bool) {
}

procedure {:inline 1} $1_string_internal_sub_string(x: Vec int, i: int, j: int) returns (r: Vec int) {
}

procedure {:inline 1} $1_string_internal_index_of(x: Vec int, y: Vec int) returns (r: int) {
}

procedure {:inline 1} $1_string_internal_is_char_boundary(x: Vec int, i: int) returns (r: bool) {
}




// ==================================================================================
// Native diem_account

procedure {:inline 1} $1_DiemAccount_create_signer(
  addr: int
) returns (signer: $signer) {
    // A signer is currently identical to an address.
    signer := $signer(addr);
}

procedure {:inline 1} $1_DiemAccount_destroy_signer(
  signer: $signer
) {
  return;
}

// ==================================================================================
// Native account

procedure {:inline 1} $1_Account_create_signer(
  addr: int
) returns (signer: $signer) {
    // A signer is currently identical to an address.
    signer := $signer(addr);
}

// ==================================================================================
// Native Signer

datatype $signer {
    $signer($addr: int)
}
function {:inline} $IsValid'signer'(s: $signer): bool {
    $IsValid'address'(s->$addr)
}
function {:inline} $IsEqual'signer'(s1: $signer, s2: $signer): bool {
    s1 == s2
}

procedure {:inline 1} $1_signer_borrow_address(signer: $signer) returns (res: int) {
    res := signer->$addr;
}

function {:inline} $1_signer_$borrow_address(signer: $signer): int
{
    signer->$addr
}

function $1_signer_is_txn_signer(s: $signer): bool;

function $1_signer_is_txn_signer_addr(a: int): bool;


// ==================================================================================
// Native signature

// Signature related functionality is handled via uninterpreted functions. This is sound
// currently because we verify every code path based on signature verification with
// an arbitrary interpretation.

function $1_Signature_$ed25519_validate_pubkey(public_key: Vec int): bool;
function $1_Signature_$ed25519_verify(signature: Vec int, public_key: Vec int, message: Vec int): bool;

// Needed because we do not have extensional equality:
axiom (forall k1, k2: Vec int ::
    {$1_Signature_$ed25519_validate_pubkey(k1), $1_Signature_$ed25519_validate_pubkey(k2)}
    $IsEqual'vec'u8''(k1, k2) ==> $1_Signature_$ed25519_validate_pubkey(k1) == $1_Signature_$ed25519_validate_pubkey(k2));
axiom (forall s1, s2, k1, k2, m1, m2: Vec int ::
    {$1_Signature_$ed25519_verify(s1, k1, m1), $1_Signature_$ed25519_verify(s2, k2, m2)}
    $IsEqual'vec'u8''(s1, s2) && $IsEqual'vec'u8''(k1, k2) && $IsEqual'vec'u8''(m1, m2)
    ==> $1_Signature_$ed25519_verify(s1, k1, m1) == $1_Signature_$ed25519_verify(s2, k2, m2));


procedure {:inline 1} $1_Signature_ed25519_validate_pubkey(public_key: Vec int) returns (res: bool) {
    res := $1_Signature_$ed25519_validate_pubkey(public_key);
}

procedure {:inline 1} $1_Signature_ed25519_verify(
        signature: Vec int, public_key: Vec int, message: Vec int) returns (res: bool) {
    res := $1_Signature_$ed25519_verify(signature, public_key, message);
}


// ==================================================================================
// Native bcs::serialize


// ==================================================================================
// Native Event module



procedure {:inline 1} $InitEventStore() {
}

// ============================================================================================
// Type Reflection on Type Parameters

datatype $TypeParamInfo {
    $TypeParamBool(),
    $TypeParamU8(),
    $TypeParamU16(),
    $TypeParamU32(),
    $TypeParamU64(),
    $TypeParamU128(),
    $TypeParamU256(),
    $TypeParamAddress(),
    $TypeParamSigner(),
    $TypeParamVector(e: $TypeParamInfo),
    $TypeParamStruct(a: int, m: Vec int, s: Vec int)
}



//==================================
// Begin Translation

function $TypeName(t: $TypeParamInfo): Vec int;
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamBool ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 98][1 := 111][2 := 111][3 := 108], 4)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 98][1 := 111][2 := 111][3 := 108], 4)) ==> t is $TypeParamBool);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamU8 ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 56], 2)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 56], 2)) ==> t is $TypeParamU8);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamU16 ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 54], 3)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 54], 3)) ==> t is $TypeParamU16);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamU32 ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 51][2 := 50], 3)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 51][2 := 50], 3)) ==> t is $TypeParamU32);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamU64 ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 54][2 := 52], 3)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 54][2 := 52], 3)) ==> t is $TypeParamU64);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamU128 ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 50][3 := 56], 4)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 50][3 := 56], 4)) ==> t is $TypeParamU128);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamU256 ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 50][2 := 53][3 := 54], 4)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 50][2 := 53][3 := 54], 4)) ==> t is $TypeParamU256);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamAddress ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 97][1 := 100][2 := 100][3 := 114][4 := 101][5 := 115][6 := 115], 7)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 97][1 := 100][2 := 100][3 := 114][4 := 101][5 := 115][6 := 115], 7)) ==> t is $TypeParamAddress);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamSigner ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 115][1 := 105][2 := 103][3 := 110][4 := 101][5 := 114], 6)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 115][1 := 105][2 := 103][3 := 110][4 := 101][5 := 114], 6)) ==> t is $TypeParamSigner);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamVector ==> $IsEqual'vec'u8''($TypeName(t), ConcatVec(ConcatVec(Vec(DefaultVecMap()[0 := 118][1 := 101][2 := 99][3 := 116][4 := 111][5 := 114][6 := 60], 7), $TypeName(t->e)), Vec(DefaultVecMap()[0 := 62], 1))));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} ($IsPrefix'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 118][1 := 101][2 := 99][3 := 116][4 := 111][5 := 114][6 := 60], 7)) && $IsSuffix'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 62], 1))) ==> t is $TypeParamVector);
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} t is $TypeParamStruct ==> $IsEqual'vec'u8''($TypeName(t), ConcatVec(ConcatVec(ConcatVec(ConcatVec(ConcatVec(Vec(DefaultVecMap()[0 := 48][1 := 120], 2), MakeVec1(t->a)), Vec(DefaultVecMap()[0 := 58][1 := 58], 2)), t->m), Vec(DefaultVecMap()[0 := 58][1 := 58], 2)), t->s)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsPrefix'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 48][1 := 120], 2)) ==> t is $TypeParamVector);


// Given Types for Type Parameters

type #0;
function {:inline} $IsEqual'#0'(x1: #0, x2: #0): bool { x1 == x2 }
function {:inline} $IsValid'#0'(x: #0): bool { true }
var #0_info: $TypeParamInfo;
var #0_$memory: $Memory #0;

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'bool'(b1), $1_from_bcs_deserializable'bool'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u8'(b1), $1_from_bcs_deserializable'u8'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u64'(b1), $1_from_bcs_deserializable'u64'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u128>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u128'(b1), $1_from_bcs_deserializable'u128'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u256'(b1), $1_from_bcs_deserializable'u256'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'address'(b1), $1_from_bcs_deserializable'address'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'signer'(b1), $1_from_bcs_deserializable'signer'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'u8''(b1), $1_from_bcs_deserializable'vec'u8''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<u128>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'u128''(b1), $1_from_bcs_deserializable'vec'u128''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'#0''(b1), $1_from_bcs_deserializable'vec'#0''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <option::Option<u128>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_option_Option'u128''(b1), $1_from_bcs_deserializable'$1_option_Option'u128''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <string::String>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_string_String'(b1), $1_from_bcs_deserializable'$1_string_String'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <features::Features>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_features_Features'(b1), $1_from_bcs_deserializable'$1_features_Features'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_GUID'(b1), $1_from_bcs_deserializable'$1_guid_GUID'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_ID'(b1), $1_from_bcs_deserializable'$1_guid_ID'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<object::TransferEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<fungible_asset::DepositEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<fungible_asset::WithdrawEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<fungible_asset::FrozenEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <aggregator_v2::Aggregator<u128>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_aggregator_v2_Aggregator'u128''(b1), $1_from_bcs_deserializable'$1_aggregator_v2_Aggregator'u128''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ConstructorRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ConstructorRef'(b1), $1_from_bcs_deserializable'$1_object_ConstructorRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::DeleteRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_DeleteRef'(b1), $1_from_bcs_deserializable'$1_object_DeleteRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ExtendRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ExtendRef'(b1), $1_from_bcs_deserializable'$1_object_ExtendRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::Object<fungible_asset::FungibleStore>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_FungibleStore''(b1), $1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_FungibleStore''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::Object<fungible_asset::Metadata>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_Metadata''(b1), $1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_Metadata''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::Object<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_Object'#0''(b1), $1_from_bcs_deserializable'$1_object_Object'#0''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ObjectCore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ObjectCore'(b1), $1_from_bcs_deserializable'$1_object_ObjectCore'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::DepositEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_DepositEvent'(b1), $1_from_bcs_deserializable'$1_fungible_asset_DepositEvent'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::WithdrawEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_WithdrawEvent'(b1), $1_from_bcs_deserializable'$1_fungible_asset_WithdrawEvent'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::TransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_TransferRef'(b1), $1_from_bcs_deserializable'$1_fungible_asset_TransferRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::BurnRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_BurnRef'(b1), $1_from_bcs_deserializable'$1_fungible_asset_BurnRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::ConcurrentSupply>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_ConcurrentSupply'(b1), $1_from_bcs_deserializable'$1_fungible_asset_ConcurrentSupply'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::FrozenEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_FrozenEvent'(b1), $1_from_bcs_deserializable'$1_fungible_asset_FrozenEvent'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::FungibleAsset>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_FungibleAsset'(b1), $1_from_bcs_deserializable'$1_fungible_asset_FungibleAsset'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::FungibleAssetEvents>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_FungibleAssetEvents'(b1), $1_from_bcs_deserializable'$1_fungible_asset_FungibleAssetEvents'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::FungibleStore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_FungibleStore'(b1), $1_from_bcs_deserializable'$1_fungible_asset_FungibleStore'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::Metadata>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_Metadata'(b1), $1_from_bcs_deserializable'$1_fungible_asset_Metadata'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::MintRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_MintRef'(b1), $1_from_bcs_deserializable'$1_fungible_asset_MintRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <fungible_asset::Supply>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_fungible_asset_Supply'(b1), $1_from_bcs_deserializable'$1_fungible_asset_Supply'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'#0'(b1), $1_from_bcs_deserializable'#0'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserialize'bool'(b1), $1_from_bcs_deserialize'bool'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u8'($1_from_bcs_deserialize'u8'(b1), $1_from_bcs_deserialize'u8'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u64'($1_from_bcs_deserialize'u64'(b1), $1_from_bcs_deserialize'u64'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u128>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u128'($1_from_bcs_deserialize'u128'(b1), $1_from_bcs_deserialize'u128'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u256'($1_from_bcs_deserialize'u256'(b1), $1_from_bcs_deserialize'u256'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'address'($1_from_bcs_deserialize'address'(b1), $1_from_bcs_deserialize'address'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'signer'($1_from_bcs_deserialize'signer'(b1), $1_from_bcs_deserialize'signer'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'u8''($1_from_bcs_deserialize'vec'u8''(b1), $1_from_bcs_deserialize'vec'u8''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<u128>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'u128''($1_from_bcs_deserialize'vec'u128''(b1), $1_from_bcs_deserialize'vec'u128''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'#0''($1_from_bcs_deserialize'vec'#0''(b1), $1_from_bcs_deserialize'vec'#0''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <option::Option<u128>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_option_Option'u128''($1_from_bcs_deserialize'$1_option_Option'u128''(b1), $1_from_bcs_deserialize'$1_option_Option'u128''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <string::String>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_string_String'($1_from_bcs_deserialize'$1_string_String'(b1), $1_from_bcs_deserialize'$1_string_String'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <features::Features>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_features_Features'($1_from_bcs_deserialize'$1_features_Features'(b1), $1_from_bcs_deserialize'$1_features_Features'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_GUID'($1_from_bcs_deserialize'$1_guid_GUID'(b1), $1_from_bcs_deserialize'$1_guid_GUID'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_ID'($1_from_bcs_deserialize'$1_guid_ID'(b1), $1_from_bcs_deserialize'$1_guid_ID'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<object::TransferEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_object_TransferEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<fungible_asset::DepositEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_fungible_asset_DepositEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<fungible_asset::WithdrawEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<fungible_asset::FrozenEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <aggregator_v2::Aggregator<u128>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_aggregator_v2_Aggregator'u128''($1_from_bcs_deserialize'$1_aggregator_v2_Aggregator'u128''(b1), $1_from_bcs_deserialize'$1_aggregator_v2_Aggregator'u128''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ConstructorRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ConstructorRef'($1_from_bcs_deserialize'$1_object_ConstructorRef'(b1), $1_from_bcs_deserialize'$1_object_ConstructorRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::DeleteRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_DeleteRef'($1_from_bcs_deserialize'$1_object_DeleteRef'(b1), $1_from_bcs_deserialize'$1_object_DeleteRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ExtendRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ExtendRef'($1_from_bcs_deserialize'$1_object_ExtendRef'(b1), $1_from_bcs_deserialize'$1_object_ExtendRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::Object<fungible_asset::FungibleStore>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_Object'$1_fungible_asset_FungibleStore''($1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_FungibleStore''(b1), $1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_FungibleStore''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::Object<fungible_asset::Metadata>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_Metadata''(b1), $1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_Metadata''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::Object<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_Object'#0''($1_from_bcs_deserialize'$1_object_Object'#0''(b1), $1_from_bcs_deserialize'$1_object_Object'#0''(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ObjectCore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ObjectCore'($1_from_bcs_deserialize'$1_object_ObjectCore'(b1), $1_from_bcs_deserialize'$1_object_ObjectCore'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::DepositEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_DepositEvent'($1_from_bcs_deserialize'$1_fungible_asset_DepositEvent'(b1), $1_from_bcs_deserialize'$1_fungible_asset_DepositEvent'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::WithdrawEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_WithdrawEvent'($1_from_bcs_deserialize'$1_fungible_asset_WithdrawEvent'(b1), $1_from_bcs_deserialize'$1_fungible_asset_WithdrawEvent'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::TransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_TransferRef'($1_from_bcs_deserialize'$1_fungible_asset_TransferRef'(b1), $1_from_bcs_deserialize'$1_fungible_asset_TransferRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::BurnRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_BurnRef'($1_from_bcs_deserialize'$1_fungible_asset_BurnRef'(b1), $1_from_bcs_deserialize'$1_fungible_asset_BurnRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::ConcurrentSupply>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_ConcurrentSupply'($1_from_bcs_deserialize'$1_fungible_asset_ConcurrentSupply'(b1), $1_from_bcs_deserialize'$1_fungible_asset_ConcurrentSupply'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::FrozenEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_FrozenEvent'($1_from_bcs_deserialize'$1_fungible_asset_FrozenEvent'(b1), $1_from_bcs_deserialize'$1_fungible_asset_FrozenEvent'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::FungibleAsset>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_FungibleAsset'($1_from_bcs_deserialize'$1_fungible_asset_FungibleAsset'(b1), $1_from_bcs_deserialize'$1_fungible_asset_FungibleAsset'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::FungibleAssetEvents>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_FungibleAssetEvents'($1_from_bcs_deserialize'$1_fungible_asset_FungibleAssetEvents'(b1), $1_from_bcs_deserialize'$1_fungible_asset_FungibleAssetEvents'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::FungibleStore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_FungibleStore'($1_from_bcs_deserialize'$1_fungible_asset_FungibleStore'(b1), $1_from_bcs_deserialize'$1_fungible_asset_FungibleStore'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::Metadata>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_Metadata'($1_from_bcs_deserialize'$1_fungible_asset_Metadata'(b1), $1_from_bcs_deserialize'$1_fungible_asset_Metadata'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::MintRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_MintRef'($1_from_bcs_deserialize'$1_fungible_asset_MintRef'(b1), $1_from_bcs_deserialize'$1_fungible_asset_MintRef'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <fungible_asset::Supply>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_fungible_asset_Supply'($1_from_bcs_deserialize'$1_fungible_asset_Supply'(b1), $1_from_bcs_deserialize'$1_fungible_asset_Supply'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'#0'($1_from_bcs_deserialize'#0'(b1), $1_from_bcs_deserialize'#0'(b2)))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:8:9+113
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_keccak256(b1), $1_aptos_hash_spec_keccak256(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:13:9+129
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_sha2_512_internal(b1), $1_aptos_hash_spec_sha2_512_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:18:9+129
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_sha3_512_internal(b1), $1_aptos_hash_spec_sha3_512_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:23:9+131
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_ripemd160_internal(b1), $1_aptos_hash_spec_ripemd160_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:28:9+135
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_blake2b_256_internal(b1), $1_aptos_hash_spec_blake2b_256_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/vector.move:146:5+86
function {:inline} $1_vector_$is_empty'u128'(v: Vec (int)): bool {
    $IsEqual'u64'($1_vector_$length'u128'(v), 0)
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:34:10+78
function {:inline} $1_option_spec_none'u128'(): $1_option_Option'u128' {
    $1_option_Option'u128'($EmptyVec'u128'())
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:47:10+89
function {:inline} $1_option_spec_some'u128'(e: int): $1_option_Option'u128' {
    $1_option_Option'u128'(MakeVec1(e))
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:69:10+91
function {:inline} $1_option_spec_is_none'u128'(t: $1_option_Option'u128'): bool {
    $1_vector_$is_empty'u128'(t->$vec)
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:82:10+92
function {:inline} $1_option_spec_is_some'u128'(t: $1_option_Option'u128'): bool {
    !$1_vector_$is_empty'u128'(t->$vec)
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:111:10+78
function {:inline} $1_option_spec_borrow'u128'(t: $1_option_Option'u128'): int {
    ReadVec(t->$vec, 0)
}

// struct option::Option<u128> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:7:5+81
datatype $1_option_Option'u128' {
    $1_option_Option'u128'($vec: Vec (int))
}
function {:inline} $Update'$1_option_Option'u128''_vec(s: $1_option_Option'u128', x: Vec (int)): $1_option_Option'u128' {
    $1_option_Option'u128'(x)
}
function $IsValid'$1_option_Option'u128''(s: $1_option_Option'u128'): bool {
    $IsValid'vec'u128''(s->$vec)
}
function {:inline} $IsEqual'$1_option_Option'u128''(s1: $1_option_Option'u128', s2: $1_option_Option'u128'): bool {
    $IsEqual'vec'u128''(s1->$vec, s2->$vec)}

// fun option::borrow_mut<u128> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+165
procedure {:inline 1} $1_option_borrow_mut'u128'(_$t0: $Mutation ($1_option_Option'u128')) returns ($ret0: $Mutation (int), $ret1: $Mutation ($1_option_Option'u128'))
{
    // declare local variables
    var $t1: $1_option_Option'u128';
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: $Mutation (Vec (int));
    var $t6: int;
    var $t7: $Mutation (int);
    var $t0: $Mutation ($1_option_Option'u128');
    var $temp_0'$1_option_Option'u128'': $1_option_Option'u128';
    var $temp_0'u128': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[t]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+1
    assume {:print "$at(15,5765,5766)"} true;
    $temp_0'$1_option_Option'u128'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'u128''} $temp_0'$1_option_Option'u128'' == $temp_0'$1_option_Option'u128'';

    // $t1 := read_ref($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:25+1
    assume {:print "$at(15,5861,5862)"} true;
    $t1 := $Dereference($t0);

    // $t2 := opaque begin: option::is_some<#0>($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsValid'bool'($t2);

    // assume Eq<bool>($t2, option::spec_is_some<#0>($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsEqual'bool'($t2, $1_option_spec_is_some'u128'($t1));

    // $t2 := opaque end: option::is_some<#0>($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // if ($t2) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(15,5845,5881)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L0:

    // pack_ref_deep($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(15,5845,5881)"} true;

    // destroy($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36

    // $t3 := 262145 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:29+15
    $t3 := 262145;
    assume $IsValid'u64'($t3);

    // trace_abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(15,5845,5881)"} true;
    assume {:print "$track_abort(1,1):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    $t4 := $t3;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:33+1
    assume {:print "$at(15,5915,5916)"} true;
L2:

    // $t5 := borrow_field<option::Option<#0>>.vec($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:28+10
    assume {:print "$at(15,5910,5920)"} true;
    $t5 := $ChildMutation($t0, 0, $Dereference($t0)->$vec);

    // $t6 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:40+1
    $t6 := 0;
    assume $IsValid'u64'($t6);

    // $t7 := vector::borrow_mut<#0>($t5, $t6) on_abort goto L4 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    call $t7,$t5 := $1_vector_borrow_mut'u128'($t5, $t6);
    if ($abort_flag) {
        assume {:print "$at(15,5891,5924)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(1,1):", $t4} $t4 == $t4;
        goto L4;
    }

    // trace_return[0]($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'u128' := $Dereference($t7);
    assume {:print "$track_return(1,1,0):", $temp_0'u128'} $temp_0'u128' == $temp_0'u128';

    // trace_local[t]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'u128'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'u128''} $temp_0'$1_option_Option'u128'' == $temp_0'$1_option_Option'u128'';

    // trace_local[t]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'u128'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'u128''} $temp_0'$1_option_Option'u128'' == $temp_0'$1_option_Option'u128'';

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(15,5929,5930)"} true;
L3:

    // return $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(15,5929,5930)"} true;
    $ret0 := $t7;
    $ret1 := $t0;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
L4:

    // abort($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(15,5929,5930)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:43:5+75
function {:inline} $1_string_$length(s: $1_string_String): int {
    $1_vector_$length'u8'(s->$bytes)
}

// struct string::String at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:13:5+70
datatype $1_string_String {
    $1_string_String($bytes: Vec (int))
}
function {:inline} $Update'$1_string_String'_bytes(s: $1_string_String, x: Vec (int)): $1_string_String {
    $1_string_String(x)
}
function $IsValid'$1_string_String'(s: $1_string_String): bool {
    $IsValid'vec'u8''(s->$bytes)
}
function {:inline} $IsEqual'$1_string_String'(s1: $1_string_String, s2: $1_string_String): bool {
    $IsEqual'vec'u8''(s1->$bytes, s2->$bytes)}

// fun string::length [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:43:5+75
procedure {:inline 1} $1_string_length(_$t0: $1_string_String) returns ($ret0: int)
{
    // declare local variables
    var $t1: Vec (int);
    var $t2: int;
    var $t3: int;
    var $t0: $1_string_String;
    var $temp_0'$1_string_String': $1_string_String;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[s]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:43:5+1
    assume {:print "$at(17,1295,1296)"} true;
    assume {:print "$track_local(2,10,0):", $t0} $t0 == $t0;

    // $t1 := get_field<string::String>.bytes($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:44:24+8
    assume {:print "$at(17,1355,1363)"} true;
    $t1 := $t0->$bytes;

    // $t2 := vector::length<u8>($t1) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:44:9+24
    call $t2 := $1_vector_length'u8'($t1);
    if ($abort_flag) {
        assume {:print "$at(17,1340,1364)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(2,10):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:44:9+24
    assume {:print "$track_return(2,10,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:45:5+1
    assume {:print "$at(17,1369,1370)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:45:5+1
    assume {:print "$at(17,1369,1370)"} true;
    $ret0 := $t2;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:45:5+1
L2:

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:45:5+1
    assume {:print "$at(17,1369,1370)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
function {:inline} $1_signer_$address_of(s: $signer): int {
    $1_signer_$borrow_address(s)
}

// fun signer::address_of [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
procedure {:inline 1} $1_signer_address_of(_$t0: $signer) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t0: $signer;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[s]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+1
    assume {:print "$at(16,395,396)"} true;
    assume {:print "$track_local(3,0,0):", $t0} $t0 == $t0;

    // $t1 := signer::borrow_address($t0) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:10+17
    assume {:print "$at(16,449,466)"} true;
    call $t1 := $1_signer_borrow_address($t0);
    if ($abort_flag) {
        assume {:print "$at(16,449,466)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(3,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:9+18
    assume {:print "$track_return(3,0,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(16,471,472)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(16,471,472)"} true;
    $ret0 := $t1;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
L2:

    // abort($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(16,471,472)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun error::invalid_argument [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:3+76
procedure {:inline 1} $1_error_invalid_argument(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t0: int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[r]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:3+1
    assume {:print "$at(12,3082,3083)"} true;
    assume {:print "$track_local(4,4,0):", $t0} $t0 == $t0;

    // $t1 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:57+16
    $t1 := 1;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(12,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume {:print "$at(12,3126,3156)"} true;

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30

    // trace_return[0]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume {:print "$track_return(4,4,0):", $t3} $t3 == $t3;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:78+1
L1:

    // return $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:78+1
    assume {:print "$at(12,3157,3158)"} true;
    $ret0 := $t3;
    return;

}

// fun error::invalid_state [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:3+70
procedure {:inline 1} $1_error_invalid_state(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t0: int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[r]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:3+1
    assume {:print "$at(12,3232,3233)"} true;
    assume {:print "$track_local(4,5,0):", $t0} $t0 == $t0;

    // $t1 := 3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:54+13
    $t1 := 3;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(12,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume {:print "$at(12,3273,3300)"} true;

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27

    // trace_return[0]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume {:print "$track_return(4,5,0):", $t3} $t3 == $t3;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:72+1
L1:

    // return $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:72+1
    assume {:print "$at(12,3301,3302)"} true;
    $ret0 := $t3;
    return;

}

// fun error::not_found [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+61
procedure {:inline 1} $1_error_not_found(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t0: int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[r]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+1
    assume {:print "$at(12,3461,3462)"} true;
    assume {:print "$track_local(4,6,0):", $t0} $t0 == $t0;

    // $t1 := 6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:49+9
    $t1 := 6;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(12,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$at(12,3497,3520)"} true;

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23

    // trace_return[0]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$track_return(4,6,0):", $t3} $t3 == $t3;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
L1:

    // return $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
    assume {:print "$at(12,3521,3522)"} true;
    $ret0 := $t3;
    return;

}

// fun error::out_of_range [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:3+68
procedure {:inline 1} $1_error_out_of_range(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t0: int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[r]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:3+1
    assume {:print "$at(12,3161,3162)"} true;
    assume {:print "$track_local(4,8,0):", $t0} $t0 == $t0;

    // $t1 := 2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:53+12
    $t1 := 2;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(12,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume {:print "$at(12,3201,3227)"} true;

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26

    // trace_return[0]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume {:print "$track_return(4,8,0):", $t3} $t3 == $t3;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:70+1
L1:

    // return $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:70+1
    assume {:print "$at(12,3228,3229)"} true;
    $ret0 := $t3;
    return;

}

// fun error::permission_denied [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:3+77
procedure {:inline 1} $1_error_permission_denied(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t0: int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[r]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:3+1
    assume {:print "$at(12,3381,3382)"} true;
    assume {:print "$track_local(4,9,0):", $t0} $t0 == $t0;

    // $t1 := 5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:57+17
    $t1 := 5;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(12,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume {:print "$at(12,3425,3456)"} true;

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31

    // trace_return[0]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume {:print "$track_return(4,9,0):", $t3} $t3 == $t3;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:79+1
L1:

    // return $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:79+1
    assume {:print "$at(12,3457,3458)"} true;
    $ret0 := $t3;
    return;

}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.spec.move:38:10+40
function  $1_features_spec_is_enabled(feature: int): bool;
axiom (forall feature: int ::
(var $$res := $1_features_spec_is_enabled(feature);
$IsValid'bool'($$res)));

// struct features::Features at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:342:5+61
datatype $1_features_Features {
    $1_features_Features($features: Vec (bv8))
}
function {:inline} $Update'$1_features_Features'_features(s: $1_features_Features, x: Vec (bv8)): $1_features_Features {
    $1_features_Features(x)
}
function $IsValid'$1_features_Features'(s: $1_features_Features): bool {
    $IsValid'vec'bv8''(s->$features)
}
function {:inline} $IsEqual'$1_features_Features'(s1: $1_features_Features, s2: $1_features_Features): bool {
    $IsEqual'vec'bv8''(s1->$features, s2->$features)}
var $1_features_Features_$memory: $Memory $1_features_Features;

// fun features::concurrent_assets_enabled [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:294:5+108
procedure {:inline 1} $1_features_concurrent_assets_enabled() returns ($ret0: bool)
{
    // declare local variables
    var $t0: int;
    var $t1: bool;
    var $temp_0'bool': bool;

    // bytecode translation starts here
    // $t0 := 37 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:295:20+17
    assume {:print "$at(10,12073,12090)"} true;
    $t0 := 37;
    assume $IsValid'u64'($t0);

    // $t1 := opaque begin: features::is_enabled($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:295:9+29

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:295:9+29
    assume $IsValid'bool'($t1);

    // assume Eq<bool>($t1, features::spec_is_enabled($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:295:9+29
    assume $IsEqual'bool'($t1, $1_features_spec_is_enabled($t0));

    // $t1 := opaque end: features::is_enabled($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:295:9+29

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:295:9+29
    assume {:print "$track_return(5,13,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:296:5+1
    assume {:print "$at(10,12096,12097)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:296:5+1
    assume {:print "$at(10,12096,12097)"} true;
    $ret0 := $t1;
    return;

}

// struct guid::GUID at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:7:5+50
datatype $1_guid_GUID {
    $1_guid_GUID($id: $1_guid_ID)
}
function {:inline} $Update'$1_guid_GUID'_id(s: $1_guid_GUID, x: $1_guid_ID): $1_guid_GUID {
    $1_guid_GUID(x)
}
function $IsValid'$1_guid_GUID'(s: $1_guid_GUID): bool {
    $IsValid'$1_guid_ID'(s->$id)
}
function {:inline} $IsEqual'$1_guid_GUID'(s1: $1_guid_GUID, s2: $1_guid_GUID): bool {
    s1 == s2
}

// struct guid::ID at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:12:5+209
datatype $1_guid_ID {
    $1_guid_ID($creation_num: int, $addr: int)
}
function {:inline} $Update'$1_guid_ID'_creation_num(s: $1_guid_ID, x: int): $1_guid_ID {
    $1_guid_ID(x, s->$addr)
}
function {:inline} $Update'$1_guid_ID'_addr(s: $1_guid_ID, x: int): $1_guid_ID {
    $1_guid_ID(s->$creation_num, x)
}
function $IsValid'$1_guid_ID'(s: $1_guid_ID): bool {
    $IsValid'u64'(s->$creation_num)
      && $IsValid'address'(s->$addr)
}
function {:inline} $IsEqual'$1_guid_ID'(s1: $1_guid_ID, s2: $1_guid_ID): bool {
    s1 == s2
}

// fun guid::create [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:23:5+286
procedure {:inline 1} $1_guid_create(_$t0: int, _$t1: $Mutation (int)) returns ($ret0: $1_guid_GUID, $ret1: $Mutation (int))
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $1_guid_ID;
    var $t8: $1_guid_GUID;
    var $t0: int;
    var $t1: $Mutation (int);
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[addr]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:23:5+1
    assume {:print "$at(130,836,837)"} true;
    assume {:print "$track_local(13,0,0):", $t0} $t0 == $t0;

    // trace_local[creation_num_ref]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:23:5+1
    $temp_0'u64' := $Dereference($t1);
    assume {:print "$track_local(13,0,1):", $temp_0'u64'} $temp_0'u64' == $temp_0'u64';

    // $t3 := read_ref($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:24:28+17
    assume {:print "$at(130,940,957)"} true;
    $t3 := $Dereference($t1);

    // trace_local[creation_num]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:24:13+12
    assume {:print "$track_local(13,0,2):", $t3} $t3 == $t3;

    // $t4 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:25:44+1
    assume {:print "$at(130,1002,1003)"} true;
    $t4 := 1;
    assume $IsValid'u64'($t4);

    // $t5 := +($t3, $t4) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:25:42+1
    call $t5 := $AddU64($t3, $t4);
    if ($abort_flag) {
        assume {:print "$at(130,1000,1001)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(13,0):", $t6} $t6 == $t6;
        goto L2;
    }

    // write_ref($t1, $t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:25:9+36
    $t1 := $UpdateMutation($t1, $t5);

    // $t7 := pack guid::ID($t3, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:27:17+70
    assume {:print "$at(130,1036,1106)"} true;
    $t7 := $1_guid_ID($t3, $t0);

    // $t8 := pack guid::GUID($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    assume {:print "$at(130,1013,1116)"} true;
    $t8 := $1_guid_GUID($t7);

    // trace_return[0]($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    assume {:print "$track_return(13,0,0):", $t8} $t8 == $t8;

    // trace_local[creation_num_ref]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    $temp_0'u64' := $Dereference($t1);
    assume {:print "$track_local(13,0,1):", $temp_0'u64'} $temp_0'u64' == $temp_0'u64';

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(130,1121,1122)"} true;
L1:

    // return $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(130,1121,1122)"} true;
    $ret0 := $t8;
    $ret1 := $t1;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
L2:

    // abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(130,1121,1122)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u8'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u8'(bytes);
$IsValid'u8'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u64'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u64'(bytes);
$IsValid'u64'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u128'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u128'(bytes);
$IsValid'u128'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u256'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u256'(bytes);
$IsValid'u256'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'address'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'address'(bytes);
$IsValid'address'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'signer'(bytes: Vec (int)): $signer;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'signer'(bytes);
$IsValid'signer'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'u8''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'u8''(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'u128''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'u128''(bytes);
$IsValid'vec'u128''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'#0''(bytes: Vec (int)): Vec (#0);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'#0''(bytes);
$IsValid'vec'#0''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_option_Option'u128''(bytes: Vec (int)): $1_option_Option'u128';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_option_Option'u128''(bytes);
$IsValid'$1_option_Option'u128''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_string_String'(bytes: Vec (int)): $1_string_String;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_string_String'(bytes);
$IsValid'$1_string_String'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_features_Features'(bytes: Vec (int)): $1_features_Features;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_features_Features'(bytes);
$IsValid'$1_features_Features'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_GUID'(bytes: Vec (int)): $1_guid_GUID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_GUID'(bytes);
$IsValid'$1_guid_GUID'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_ID'(bytes: Vec (int)): $1_guid_ID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_ID'(bytes);
$IsValid'$1_guid_ID'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_object_TransferEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_object_TransferEvent''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_fungible_asset_DepositEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_fungible_asset_DepositEvent''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_aggregator_v2_Aggregator'u128''(bytes: Vec (int)): $1_aggregator_v2_Aggregator'u128';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_aggregator_v2_Aggregator'u128''(bytes);
$IsValid'$1_aggregator_v2_Aggregator'u128''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ConstructorRef'(bytes: Vec (int)): $1_object_ConstructorRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ConstructorRef'(bytes);
$IsValid'$1_object_ConstructorRef'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_DeleteRef'(bytes: Vec (int)): $1_object_DeleteRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_DeleteRef'(bytes);
$IsValid'$1_object_DeleteRef'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ExtendRef'(bytes: Vec (int)): $1_object_ExtendRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ExtendRef'(bytes);
$IsValid'$1_object_ExtendRef'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_FungibleStore''(bytes: Vec (int)): $1_object_Object'$1_fungible_asset_FungibleStore';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_FungibleStore''(bytes);
$IsValid'$1_object_Object'$1_fungible_asset_FungibleStore''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_Metadata''(bytes: Vec (int)): $1_object_Object'$1_fungible_asset_Metadata';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_Object'$1_fungible_asset_Metadata''(bytes);
$IsValid'$1_object_Object'$1_fungible_asset_Metadata''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_Object'#0''(bytes: Vec (int)): $1_object_Object'#0';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_Object'#0''(bytes);
$IsValid'$1_object_Object'#0''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ObjectCore'(bytes: Vec (int)): $1_object_ObjectCore;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ObjectCore'(bytes);
$IsValid'$1_object_ObjectCore'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_DepositEvent'(bytes: Vec (int)): $1_fungible_asset_DepositEvent;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_DepositEvent'(bytes);
$IsValid'$1_fungible_asset_DepositEvent'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_WithdrawEvent'(bytes: Vec (int)): $1_fungible_asset_WithdrawEvent;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_WithdrawEvent'(bytes);
$IsValid'$1_fungible_asset_WithdrawEvent'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_TransferRef'(bytes: Vec (int)): $1_fungible_asset_TransferRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_TransferRef'(bytes);
$IsValid'$1_fungible_asset_TransferRef'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_BurnRef'(bytes: Vec (int)): $1_fungible_asset_BurnRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_BurnRef'(bytes);
$IsValid'$1_fungible_asset_BurnRef'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_ConcurrentSupply'(bytes: Vec (int)): $1_fungible_asset_ConcurrentSupply;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_ConcurrentSupply'(bytes);
$IsValid'$1_fungible_asset_ConcurrentSupply'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_FrozenEvent'(bytes: Vec (int)): $1_fungible_asset_FrozenEvent;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_FrozenEvent'(bytes);
$IsValid'$1_fungible_asset_FrozenEvent'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_FungibleAsset'(bytes: Vec (int)): $1_fungible_asset_FungibleAsset;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_FungibleAsset'(bytes);
$IsValid'$1_fungible_asset_FungibleAsset'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_FungibleAssetEvents'(bytes: Vec (int)): $1_fungible_asset_FungibleAssetEvents;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_FungibleAssetEvents'(bytes);
$IsValid'$1_fungible_asset_FungibleAssetEvents'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_FungibleStore'(bytes: Vec (int)): $1_fungible_asset_FungibleStore;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_FungibleStore'(bytes);
$IsValid'$1_fungible_asset_FungibleStore'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_Metadata'(bytes: Vec (int)): $1_fungible_asset_Metadata;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_Metadata'(bytes);
$IsValid'$1_fungible_asset_Metadata'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_MintRef'(bytes: Vec (int)): $1_fungible_asset_MintRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_MintRef'(bytes);
$IsValid'$1_fungible_asset_MintRef'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_fungible_asset_Supply'(bytes: Vec (int)): $1_fungible_asset_Supply;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_fungible_asset_Supply'(bytes);
$IsValid'$1_fungible_asset_Supply'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'#0'(bytes: Vec (int)): #0;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'#0'(bytes);
$IsValid'#0'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u8'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u8'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u64'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u64'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u128'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u128'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u256'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u256'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'address'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'address'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'signer'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'signer'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'u8''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'u8''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'u128''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'u128''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'#0''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'#0''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_option_Option'u128''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_option_Option'u128''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_string_String'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_string_String'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_features_Features'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_features_Features'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_GUID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_GUID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_ID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_ID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_aggregator_v2_Aggregator'u128''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_aggregator_v2_Aggregator'u128''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ConstructorRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ConstructorRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_DeleteRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_DeleteRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ExtendRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ExtendRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_FungibleStore''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_FungibleStore''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_Metadata''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_Object'$1_fungible_asset_Metadata''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_Object'#0''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_Object'#0''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ObjectCore'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ObjectCore'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_DepositEvent'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_DepositEvent'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_WithdrawEvent'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_WithdrawEvent'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_TransferRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_TransferRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_BurnRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_BurnRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_ConcurrentSupply'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_ConcurrentSupply'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_FrozenEvent'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_FrozenEvent'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_FungibleAsset'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_FungibleAsset'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_FungibleAssetEvents'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_FungibleAssetEvents'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_FungibleStore'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_FungibleStore'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_Metadata'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_Metadata'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_MintRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_MintRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_fungible_asset_Supply'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_fungible_asset_Supply'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'#0'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'#0'(bytes);
$IsValid'bool'($$res)));

// struct event::EventHandle<object::TransferEvent> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:34:5+224
datatype $1_event_EventHandle'$1_object_TransferEvent' {
    $1_event_EventHandle'$1_object_TransferEvent'($counter: int, $guid: $1_guid_GUID)
}
function {:inline} $Update'$1_event_EventHandle'$1_object_TransferEvent''_counter(s: $1_event_EventHandle'$1_object_TransferEvent', x: int): $1_event_EventHandle'$1_object_TransferEvent' {
    $1_event_EventHandle'$1_object_TransferEvent'(x, s->$guid)
}
function {:inline} $Update'$1_event_EventHandle'$1_object_TransferEvent''_guid(s: $1_event_EventHandle'$1_object_TransferEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_object_TransferEvent' {
    $1_event_EventHandle'$1_object_TransferEvent'(s->$counter, x)
}
function $IsValid'$1_event_EventHandle'$1_object_TransferEvent''(s: $1_event_EventHandle'$1_object_TransferEvent'): bool {
    $IsValid'u64'(s->$counter)
      && $IsValid'$1_guid_GUID'(s->$guid)
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_object_TransferEvent''(s1: $1_event_EventHandle'$1_object_TransferEvent', s2: $1_event_EventHandle'$1_object_TransferEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<fungible_asset::DepositEvent> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:34:5+224
datatype $1_event_EventHandle'$1_fungible_asset_DepositEvent' {
    $1_event_EventHandle'$1_fungible_asset_DepositEvent'($counter: int, $guid: $1_guid_GUID)
}
function {:inline} $Update'$1_event_EventHandle'$1_fungible_asset_DepositEvent''_counter(s: $1_event_EventHandle'$1_fungible_asset_DepositEvent', x: int): $1_event_EventHandle'$1_fungible_asset_DepositEvent' {
    $1_event_EventHandle'$1_fungible_asset_DepositEvent'(x, s->$guid)
}
function {:inline} $Update'$1_event_EventHandle'$1_fungible_asset_DepositEvent''_guid(s: $1_event_EventHandle'$1_fungible_asset_DepositEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_fungible_asset_DepositEvent' {
    $1_event_EventHandle'$1_fungible_asset_DepositEvent'(s->$counter, x)
}
function $IsValid'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(s: $1_event_EventHandle'$1_fungible_asset_DepositEvent'): bool {
    $IsValid'u64'(s->$counter)
      && $IsValid'$1_guid_GUID'(s->$guid)
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(s1: $1_event_EventHandle'$1_fungible_asset_DepositEvent', s2: $1_event_EventHandle'$1_fungible_asset_DepositEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<fungible_asset::WithdrawEvent> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:34:5+224
datatype $1_event_EventHandle'$1_fungible_asset_WithdrawEvent' {
    $1_event_EventHandle'$1_fungible_asset_WithdrawEvent'($counter: int, $guid: $1_guid_GUID)
}
function {:inline} $Update'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''_counter(s: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent', x: int): $1_event_EventHandle'$1_fungible_asset_WithdrawEvent' {
    $1_event_EventHandle'$1_fungible_asset_WithdrawEvent'(x, s->$guid)
}
function {:inline} $Update'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''_guid(s: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_fungible_asset_WithdrawEvent' {
    $1_event_EventHandle'$1_fungible_asset_WithdrawEvent'(s->$counter, x)
}
function $IsValid'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(s: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent'): bool {
    $IsValid'u64'(s->$counter)
      && $IsValid'$1_guid_GUID'(s->$guid)
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(s1: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent', s2: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<fungible_asset::FrozenEvent> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:34:5+224
datatype $1_event_EventHandle'$1_fungible_asset_FrozenEvent' {
    $1_event_EventHandle'$1_fungible_asset_FrozenEvent'($counter: int, $guid: $1_guid_GUID)
}
function {:inline} $Update'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''_counter(s: $1_event_EventHandle'$1_fungible_asset_FrozenEvent', x: int): $1_event_EventHandle'$1_fungible_asset_FrozenEvent' {
    $1_event_EventHandle'$1_fungible_asset_FrozenEvent'(x, s->$guid)
}
function {:inline} $Update'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''_guid(s: $1_event_EventHandle'$1_fungible_asset_FrozenEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_fungible_asset_FrozenEvent' {
    $1_event_EventHandle'$1_fungible_asset_FrozenEvent'(s->$counter, x)
}
function $IsValid'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(s: $1_event_EventHandle'$1_fungible_asset_FrozenEvent'): bool {
    $IsValid'u64'(s->$counter)
      && $IsValid'$1_guid_GUID'(s->$guid)
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(s1: $1_event_EventHandle'$1_fungible_asset_FrozenEvent', s2: $1_event_EventHandle'$1_fungible_asset_FrozenEvent'): bool {
    s1 == s2
}

// fun event::destroy_handle<fungible_asset::DepositEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:78:5+131
procedure {:inline 1} $1_event_destroy_handle'$1_fungible_asset_DepositEvent'(_$t0: $1_event_EventHandle'$1_fungible_asset_DepositEvent') returns ()
{
    // declare local variables
    var $t1: int;
    var $t2: $1_guid_GUID;
    var $t0: $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_DepositEvent'': $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[handle]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:78:5+1
    assume {:print "$at(124,2766,2767)"} true;
    assume {:print "$track_local(15,1,0):", $t0} $t0 == $t0;

    // ($t1, $t2) := unpack event::EventHandle<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:9+38
    assume {:print "$at(124,2843,2881)"} true;
    $t1 := $t0->$counter;
    $t2 := $t0->$guid;

    // destroy($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:44+1

    // destroy($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:35+1

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:80:5+1
    assume {:print "$at(124,2896,2897)"} true;
L1:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:80:5+1
    assume {:print "$at(124,2896,2897)"} true;
    return;

}

// fun event::destroy_handle<fungible_asset::WithdrawEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:78:5+131
procedure {:inline 1} $1_event_destroy_handle'$1_fungible_asset_WithdrawEvent'(_$t0: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent') returns ()
{
    // declare local variables
    var $t1: int;
    var $t2: $1_guid_GUID;
    var $t0: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent'': $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[handle]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:78:5+1
    assume {:print "$at(124,2766,2767)"} true;
    assume {:print "$track_local(15,1,0):", $t0} $t0 == $t0;

    // ($t1, $t2) := unpack event::EventHandle<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:9+38
    assume {:print "$at(124,2843,2881)"} true;
    $t1 := $t0->$counter;
    $t2 := $t0->$guid;

    // destroy($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:44+1

    // destroy($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:35+1

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:80:5+1
    assume {:print "$at(124,2896,2897)"} true;
L1:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:80:5+1
    assume {:print "$at(124,2896,2897)"} true;
    return;

}

// fun event::destroy_handle<fungible_asset::FrozenEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:78:5+131
procedure {:inline 1} $1_event_destroy_handle'$1_fungible_asset_FrozenEvent'(_$t0: $1_event_EventHandle'$1_fungible_asset_FrozenEvent') returns ()
{
    // declare local variables
    var $t1: int;
    var $t2: $1_guid_GUID;
    var $t0: $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_FrozenEvent'': $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[handle]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:78:5+1
    assume {:print "$at(124,2766,2767)"} true;
    assume {:print "$track_local(15,1,0):", $t0} $t0 == $t0;

    // ($t1, $t2) := unpack event::EventHandle<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:9+38
    assume {:print "$at(124,2843,2881)"} true;
    $t1 := $t0->$counter;
    $t2 := $t0->$guid;

    // destroy($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:44+1

    // destroy($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:79:35+1

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:80:5+1
    assume {:print "$at(124,2896,2897)"} true;
L1:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:80:5+1
    assume {:print "$at(124,2896,2897)"} true;
    return;

}

// fun event::new_event_handle<fungible_asset::DepositEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:43:5+165
procedure {:inline 1} $1_event_new_event_handle'$1_fungible_asset_DepositEvent'(_$t0: $1_guid_GUID) returns ($ret0: $1_event_EventHandle'$1_fungible_asset_DepositEvent')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    var $t0: $1_guid_GUID;
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_DepositEvent'': $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[guid]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:43:5+1
    assume {:print "$at(124,1543,1544)"} true;
    assume {:print "$track_local(15,5,0):", $t0} $t0 == $t0;

    // $t1 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:45:22+1
    assume {:print "$at(124,1672,1673)"} true;
    $t1 := 0;
    assume $IsValid'u64'($t1);

    // $t2 := pack event::EventHandle<#0>($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$at(124,1634,1702)"} true;
    $t2 := $1_event_EventHandle'$1_fungible_asset_DepositEvent'($t1, $t0);

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$track_return(15,5,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(124,1707,1708)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(124,1707,1708)"} true;
    $ret0 := $t2;
    return;

}

// fun event::new_event_handle<fungible_asset::WithdrawEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:43:5+165
procedure {:inline 1} $1_event_new_event_handle'$1_fungible_asset_WithdrawEvent'(_$t0: $1_guid_GUID) returns ($ret0: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $t0: $1_guid_GUID;
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent'': $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[guid]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:43:5+1
    assume {:print "$at(124,1543,1544)"} true;
    assume {:print "$track_local(15,5,0):", $t0} $t0 == $t0;

    // $t1 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:45:22+1
    assume {:print "$at(124,1672,1673)"} true;
    $t1 := 0;
    assume $IsValid'u64'($t1);

    // $t2 := pack event::EventHandle<#0>($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$at(124,1634,1702)"} true;
    $t2 := $1_event_EventHandle'$1_fungible_asset_WithdrawEvent'($t1, $t0);

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$track_return(15,5,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(124,1707,1708)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(124,1707,1708)"} true;
    $ret0 := $t2;
    return;

}

// fun event::new_event_handle<fungible_asset::FrozenEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:43:5+165
procedure {:inline 1} $1_event_new_event_handle'$1_fungible_asset_FrozenEvent'(_$t0: $1_guid_GUID) returns ($ret0: $1_event_EventHandle'$1_fungible_asset_FrozenEvent')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $t0: $1_guid_GUID;
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_FrozenEvent'': $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[guid]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:43:5+1
    assume {:print "$at(124,1543,1544)"} true;
    assume {:print "$track_local(15,5,0):", $t0} $t0 == $t0;

    // $t1 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:45:22+1
    assume {:print "$at(124,1672,1673)"} true;
    $t1 := 0;
    assume $IsValid'u64'($t1);

    // $t2 := pack event::EventHandle<#0>($t1, $t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$at(124,1634,1702)"} true;
    $t2 := $1_event_EventHandle'$1_fungible_asset_FrozenEvent'($t1, $t0);

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$track_return(15,5,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(124,1707,1708)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(124,1707,1708)"} true;
    $ret0 := $t2;
    return;

}

// struct aggregator_v2::Aggregator<u128> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:41:5+111
datatype $1_aggregator_v2_Aggregator'u128' {
    $1_aggregator_v2_Aggregator'u128'($value: int, $max_value: int)
}
function {:inline} $Update'$1_aggregator_v2_Aggregator'u128''_value(s: $1_aggregator_v2_Aggregator'u128', x: int): $1_aggregator_v2_Aggregator'u128' {
    $1_aggregator_v2_Aggregator'u128'(x, s->$max_value)
}
function {:inline} $Update'$1_aggregator_v2_Aggregator'u128''_max_value(s: $1_aggregator_v2_Aggregator'u128', x: int): $1_aggregator_v2_Aggregator'u128' {
    $1_aggregator_v2_Aggregator'u128'(s->$value, x)
}
function $IsValid'$1_aggregator_v2_Aggregator'u128''(s: $1_aggregator_v2_Aggregator'u128'): bool {
    $IsValid'u128'(s->$value)
      && $IsValid'u128'(s->$max_value)
}
function {:inline} $IsEqual'$1_aggregator_v2_Aggregator'u128''(s1: $1_aggregator_v2_Aggregator'u128', s2: $1_aggregator_v2_Aggregator'u128'): bool {
    s1 == s2
}

// fun aggregator_v2::add<u128> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:77:5+182
procedure {:inline 1} $1_aggregator_v2_add'u128'(_$t0: $Mutation ($1_aggregator_v2_Aggregator'u128'), _$t1: int) returns ($ret0: $Mutation ($1_aggregator_v2_Aggregator'u128'))
{
    // declare local variables
    var $t2: bool;
    var $t3: bool;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t0: $Mutation ($1_aggregator_v2_Aggregator'u128');
    var $t1: int;
    var $temp_0'$1_aggregator_v2_Aggregator'u128'': $1_aggregator_v2_Aggregator'u128';
    var $temp_0'u128': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[aggregator]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:77:5+1
    assume {:print "$at(92,3937,3938)"} true;
    $temp_0'$1_aggregator_v2_Aggregator'u128'' := $Dereference($t0);
    assume {:print "$track_local(21,0,0):", $temp_0'$1_aggregator_v2_Aggregator'u128''} $temp_0'$1_aggregator_v2_Aggregator'u128'' == $temp_0'$1_aggregator_v2_Aggregator'u128'';

    // trace_local[value]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:77:5+1
    assume {:print "$track_local(21,0,1):", $t1} $t1 == $t1;

    // $t2 := opaque begin: aggregator_v2::try_add<#0>($t0, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    assume {:print "$at(92,4042,4068)"} true;

    // $t3 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    havoc $t3;

    // if ($t3) goto L7 else goto L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    if ($t3) { goto L7; } else { goto L6; }

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
L7:

    // trace_abort($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    assume {:print "$at(92,4042,4068)"} true;
    assume {:print "$track_abort(21,0):", $t4} $t4 == $t4;

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    goto L5;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
L6:

    // $t0 := havoc[mut]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    assume {:print "$at(92,4042,4068)"} true;
    havoc $temp_0'$1_aggregator_v2_Aggregator'u128'';
    $t0 := $UpdateMutation($t0, $temp_0'$1_aggregator_v2_Aggregator'u128'');

    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($Dereference($t0));

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26
    assume $IsValid'bool'($t2);

    // $t2 := opaque end: aggregator_v2::try_add<#0>($t0, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:17+26

    // if ($t2) goto L1 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:9+78
    if ($t2) { goto L1; } else { goto L3; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:9+78
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:9+78
    assume {:print "$at(92,4034,4112)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:65+20
L0:

    // $t5 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:65+20
    assume {:print "$at(92,4090,4110)"} true;
    $t5 := 1;
    assume $IsValid'u64'($t5);

    // $t6 := error::out_of_range($t5) on_abort goto L5 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:45+41
    call $t6 := $1_error_out_of_range($t5);
    if ($abort_flag) {
        assume {:print "$at(92,4070,4111)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(21,0):", $t4} $t4 == $t4;
        goto L5;
    }

    // trace_abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:9+78
    assume {:print "$at(92,4034,4112)"} true;
    assume {:print "$track_abort(21,0):", $t6} $t6 == $t6;

    // $t4 := move($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:9+78
    $t4 := $t6;

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:9+78
    goto L5;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:87+1
L2:

    // trace_local[aggregator]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:87+1
    assume {:print "$at(92,4112,4113)"} true;
    $temp_0'$1_aggregator_v2_Aggregator'u128'' := $Dereference($t0);
    assume {:print "$track_local(21,0,0):", $temp_0'$1_aggregator_v2_Aggregator'u128''} $temp_0'$1_aggregator_v2_Aggregator'u128'' == $temp_0'$1_aggregator_v2_Aggregator'u128'';

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:78:87+1
    goto L4;

    // label L3 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L3:

    // destroy($t0) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L0 at <internal>:1:1+10
    goto L0;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:79:5+1
    assume {:print "$at(92,4118,4119)"} true;
L4:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:79:5+1
    assume {:print "$at(92,4118,4119)"} true;
    $ret0 := $t0;
    return;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:79:5+1
L5:

    // abort($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:79:5+1
    assume {:print "$at(92,4118,4119)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun aggregator_v2::max_value<u128> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:54:5+131
procedure {:inline 1} $1_aggregator_v2_max_value'u128'(_$t0: $1_aggregator_v2_Aggregator'u128') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_aggregator_v2_Aggregator'u128';
    var $temp_0'$1_aggregator_v2_Aggregator'u128'': $1_aggregator_v2_Aggregator'u128';
    var $temp_0'u128': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[aggregator]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:54:5+1
    assume {:print "$at(92,2626,2627)"} true;
    assume {:print "$track_local(21,5,0):", $t0} $t0 == $t0;

    // $t1 := get_field<aggregator_v2::Aggregator<#0>>.max_value($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:55:9+20
    assume {:print "$at(92,2731,2751)"} true;
    $t1 := $t0->$max_value;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:55:9+20
    assume {:print "$track_return(21,5,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:56:5+1
    assume {:print "$at(92,2756,2757)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/aggregator_v2/aggregator_v2.move:56:5+1
    assume {:print "$at(92,2756,2757)"} true;
    $ret0 := $t1;
    return;

}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:7:9+50
function  $1_aptos_hash_spec_keccak256(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_keccak256(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:12:9+58
function  $1_aptos_hash_spec_sha2_512_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_sha2_512_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:17:9+58
function  $1_aptos_hash_spec_sha3_512_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_sha3_512_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:22:9+59
function  $1_aptos_hash_spec_ripemd160_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_ripemd160_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:27:9+61
function  $1_aptos_hash_spec_blake2b_256_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_blake2b_256_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:337:5+95
function {:inline} $1_object_$address_from_constructor_ref(ref: $1_object_ConstructorRef): int {
    ref->$self
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:172:5+278
function {:inline} $1_object_$address_to_object'$1_fungible_asset_FungibleStore'(object: int): $1_object_Object'$1_fungible_asset_FungibleStore' {
    $1_object_Object'$1_fungible_asset_FungibleStore'(object)
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:347:5+93
function {:inline} $1_object_$can_generate_delete_ref(ref: $1_object_ConstructorRef): bool {
    ref->$can_delete
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+91
function {:inline} $1_object_$object_address'$1_fungible_asset_FungibleStore'(object: $1_object_Object'$1_fungible_asset_FungibleStore'): int {
    object->$inner
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+91
function {:inline} $1_object_$object_address'$1_fungible_asset_Metadata'(object: $1_object_Object'$1_fungible_asset_Metadata'): int {
    object->$inner
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+91
function {:inline} $1_object_$object_address'#0'(object: $1_object_Object'#0'): int {
    object->$inner
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:375:5+116
function {:inline} $1_object_$object_from_delete_ref'$1_fungible_asset_FungibleStore'(ref: $1_object_DeleteRef): $1_object_Object'$1_fungible_asset_FungibleStore' {
    $1_object_$address_to_object'$1_fungible_asset_FungibleStore'(ref->$self)
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:54:10+50
function  $1_object_spec_exists_at'$1_fungible_asset_FungibleStore'(object: int): bool;
axiom (forall object: int ::
(var $$res := $1_object_spec_exists_at'$1_fungible_asset_FungibleStore'(object);
$IsValid'bool'($$res)));

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:54:10+50
function  $1_object_spec_exists_at'$1_fungible_asset_Metadata'(object: int): bool;
axiom (forall object: int ::
(var $$res := $1_object_spec_exists_at'$1_fungible_asset_Metadata'(object);
$IsValid'bool'($$res)));

// struct object::ConstructorRef at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:126:5+167
datatype $1_object_ConstructorRef {
    $1_object_ConstructorRef($self: int, $can_delete: bool)
}
function {:inline} $Update'$1_object_ConstructorRef'_self(s: $1_object_ConstructorRef, x: int): $1_object_ConstructorRef {
    $1_object_ConstructorRef(x, s->$can_delete)
}
function {:inline} $Update'$1_object_ConstructorRef'_can_delete(s: $1_object_ConstructorRef, x: bool): $1_object_ConstructorRef {
    $1_object_ConstructorRef(s->$self, x)
}
function $IsValid'$1_object_ConstructorRef'(s: $1_object_ConstructorRef): bool {
    $IsValid'address'(s->$self)
      && $IsValid'bool'(s->$can_delete)
}
function {:inline} $IsEqual'$1_object_ConstructorRef'(s1: $1_object_ConstructorRef, s2: $1_object_ConstructorRef): bool {
    s1 == s2
}

// struct object::DeleteRef at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:133:5+63
datatype $1_object_DeleteRef {
    $1_object_DeleteRef($self: int)
}
function {:inline} $Update'$1_object_DeleteRef'_self(s: $1_object_DeleteRef, x: int): $1_object_DeleteRef {
    $1_object_DeleteRef(x)
}
function $IsValid'$1_object_DeleteRef'(s: $1_object_DeleteRef): bool {
    $IsValid'address'(s->$self)
}
function {:inline} $IsEqual'$1_object_DeleteRef'(s1: $1_object_DeleteRef, s2: $1_object_DeleteRef): bool {
    s1 == s2
}

// struct object::ExtendRef at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:138:5+63
datatype $1_object_ExtendRef {
    $1_object_ExtendRef($self: int)
}
function {:inline} $Update'$1_object_ExtendRef'_self(s: $1_object_ExtendRef, x: int): $1_object_ExtendRef {
    $1_object_ExtendRef(x)
}
function $IsValid'$1_object_ExtendRef'(s: $1_object_ExtendRef): bool {
    $IsValid'address'(s->$self)
}
function {:inline} $IsEqual'$1_object_ExtendRef'(s1: $1_object_ExtendRef, s2: $1_object_ExtendRef): bool {
    s1 == s2
}

// struct object::Object<fungible_asset::FungibleStore> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:121:5+78
datatype $1_object_Object'$1_fungible_asset_FungibleStore' {
    $1_object_Object'$1_fungible_asset_FungibleStore'($inner: int)
}
function {:inline} $Update'$1_object_Object'$1_fungible_asset_FungibleStore''_inner(s: $1_object_Object'$1_fungible_asset_FungibleStore', x: int): $1_object_Object'$1_fungible_asset_FungibleStore' {
    $1_object_Object'$1_fungible_asset_FungibleStore'(x)
}
function $IsValid'$1_object_Object'$1_fungible_asset_FungibleStore''(s: $1_object_Object'$1_fungible_asset_FungibleStore'): bool {
    $IsValid'address'(s->$inner)
}
function {:inline} $IsEqual'$1_object_Object'$1_fungible_asset_FungibleStore''(s1: $1_object_Object'$1_fungible_asset_FungibleStore', s2: $1_object_Object'$1_fungible_asset_FungibleStore'): bool {
    s1 == s2
}

// struct object::Object<fungible_asset::Metadata> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:121:5+78
datatype $1_object_Object'$1_fungible_asset_Metadata' {
    $1_object_Object'$1_fungible_asset_Metadata'($inner: int)
}
function {:inline} $Update'$1_object_Object'$1_fungible_asset_Metadata''_inner(s: $1_object_Object'$1_fungible_asset_Metadata', x: int): $1_object_Object'$1_fungible_asset_Metadata' {
    $1_object_Object'$1_fungible_asset_Metadata'(x)
}
function $IsValid'$1_object_Object'$1_fungible_asset_Metadata''(s: $1_object_Object'$1_fungible_asset_Metadata'): bool {
    $IsValid'address'(s->$inner)
}
function {:inline} $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''(s1: $1_object_Object'$1_fungible_asset_Metadata', s2: $1_object_Object'$1_fungible_asset_Metadata'): bool {
    s1 == s2
}

// struct object::Object<#0> at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:121:5+78
datatype $1_object_Object'#0' {
    $1_object_Object'#0'($inner: int)
}
function {:inline} $Update'$1_object_Object'#0''_inner(s: $1_object_Object'#0', x: int): $1_object_Object'#0' {
    $1_object_Object'#0'(x)
}
function $IsValid'$1_object_Object'#0''(s: $1_object_Object'#0'): bool {
    $IsValid'address'(s->$inner)
}
function {:inline} $IsEqual'$1_object_Object'#0''(s1: $1_object_Object'#0', s2: $1_object_Object'#0'): bool {
    s1 == s2
}

// struct object::ObjectCore at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:94:5+551
datatype $1_object_ObjectCore {
    $1_object_ObjectCore($guid_creation_num: int, $owner: int, $allow_ungated_transfer: bool, $transfer_events: $1_event_EventHandle'$1_object_TransferEvent')
}
function {:inline} $Update'$1_object_ObjectCore'_guid_creation_num(s: $1_object_ObjectCore, x: int): $1_object_ObjectCore {
    $1_object_ObjectCore(x, s->$owner, s->$allow_ungated_transfer, s->$transfer_events)
}
function {:inline} $Update'$1_object_ObjectCore'_owner(s: $1_object_ObjectCore, x: int): $1_object_ObjectCore {
    $1_object_ObjectCore(s->$guid_creation_num, x, s->$allow_ungated_transfer, s->$transfer_events)
}
function {:inline} $Update'$1_object_ObjectCore'_allow_ungated_transfer(s: $1_object_ObjectCore, x: bool): $1_object_ObjectCore {
    $1_object_ObjectCore(s->$guid_creation_num, s->$owner, x, s->$transfer_events)
}
function {:inline} $Update'$1_object_ObjectCore'_transfer_events(s: $1_object_ObjectCore, x: $1_event_EventHandle'$1_object_TransferEvent'): $1_object_ObjectCore {
    $1_object_ObjectCore(s->$guid_creation_num, s->$owner, s->$allow_ungated_transfer, x)
}
function $IsValid'$1_object_ObjectCore'(s: $1_object_ObjectCore): bool {
    $IsValid'u64'(s->$guid_creation_num)
      && $IsValid'address'(s->$owner)
      && $IsValid'bool'(s->$allow_ungated_transfer)
      && $IsValid'$1_event_EventHandle'$1_object_TransferEvent''(s->$transfer_events)
}
function {:inline} $IsEqual'$1_object_ObjectCore'(s1: $1_object_ObjectCore, s2: $1_object_ObjectCore): bool {
    s1 == s2
}
var $1_object_ObjectCore_$memory: $Memory $1_object_ObjectCore;

// struct object::TransferEvent at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:160:5+113
datatype $1_object_TransferEvent {
    $1_object_TransferEvent($object: int, $from: int, $to: int)
}
function {:inline} $Update'$1_object_TransferEvent'_object(s: $1_object_TransferEvent, x: int): $1_object_TransferEvent {
    $1_object_TransferEvent(x, s->$from, s->$to)
}
function {:inline} $Update'$1_object_TransferEvent'_from(s: $1_object_TransferEvent, x: int): $1_object_TransferEvent {
    $1_object_TransferEvent(s->$object, x, s->$to)
}
function {:inline} $Update'$1_object_TransferEvent'_to(s: $1_object_TransferEvent, x: int): $1_object_TransferEvent {
    $1_object_TransferEvent(s->$object, s->$from, x)
}
function $IsValid'$1_object_TransferEvent'(s: $1_object_TransferEvent): bool {
    $IsValid'address'(s->$object)
      && $IsValid'address'(s->$from)
      && $IsValid'address'(s->$to)
}
function {:inline} $IsEqual'$1_object_TransferEvent'(s1: $1_object_TransferEvent, s2: $1_object_TransferEvent): bool {
    s1 == s2
}

// fun object::new_event_handle<fungible_asset::DepositEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:361:5+180
procedure {:inline 1} $1_object_new_event_handle'$1_fungible_asset_DepositEvent'(_$t0: $signer) returns ($ret0: $1_event_EventHandle'$1_fungible_asset_DepositEvent')
{
    // declare local variables
    var $t1: $1_object_ObjectCore;
    var $t2: $1_guid_GUID;
    var $t3: $1_object_ObjectCore;
    var $t4: $1_guid_GUID;
    var $t5: int;
    var $t6: $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    var $t0: $signer;
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_DepositEvent'': $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // assume Identical($t1, global<object::ObjectCore>(signer::$address_of($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:352:9+65
    assume {:print "$at(136,14413,14478)"} true;
    assume ($t1 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // assume Identical($t2, pack guid::GUID(pack guid::ID(select object::ObjectCore.guid_creation_num($t1), signer::$address_of($t0)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:355:9+185
    assume {:print "$at(136,14551,14736)"} true;
    assume ($t2 == $1_guid_GUID($1_guid_ID($t1->$guid_creation_num, $1_signer_$address_of($t0))));

    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:361:5+1
    assume {:print "$at(135,15916,15917)"} true;
    assume {:print "$track_local(55,33,0):", $t0} $t0 == $t0;

    // assume Identical($t3, global<object::ObjectCore>(signer::$address_of($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:336:9+65
    assume {:print "$at(136,13878,13943)"} true;
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // $t4 := object::create_guid($t0) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:33+19
    assume {:print "$at(135,16070,16089)"} true;
    call $t4 := $1_object_create_guid($t0);
    if ($abort_flag) {
        assume {:print "$at(135,16070,16089)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,33):", $t5} $t5 == $t5;
        goto L2;
    }

    // $t6 := event::new_event_handle<#0>($t4) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+44
    call $t6 := $1_event_new_event_handle'$1_fungible_asset_DepositEvent'($t4);
    if ($abort_flag) {
        assume {:print "$at(135,16046,16090)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,33):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_return[0]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+44
    assume {:print "$track_return(55,33,0):", $t6} $t6 == $t6;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
L1:

    // return $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
    $ret0 := $t6;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
L2:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::new_event_handle<fungible_asset::WithdrawEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:361:5+180
procedure {:inline 1} $1_object_new_event_handle'$1_fungible_asset_WithdrawEvent'(_$t0: $signer) returns ($ret0: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent')
{
    // declare local variables
    var $t1: $1_object_ObjectCore;
    var $t2: $1_guid_GUID;
    var $t3: $1_object_ObjectCore;
    var $t4: $1_guid_GUID;
    var $t5: int;
    var $t6: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $t0: $signer;
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent'': $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // assume Identical($t1, global<object::ObjectCore>(signer::$address_of($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:352:9+65
    assume {:print "$at(136,14413,14478)"} true;
    assume ($t1 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // assume Identical($t2, pack guid::GUID(pack guid::ID(select object::ObjectCore.guid_creation_num($t1), signer::$address_of($t0)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:355:9+185
    assume {:print "$at(136,14551,14736)"} true;
    assume ($t2 == $1_guid_GUID($1_guid_ID($t1->$guid_creation_num, $1_signer_$address_of($t0))));

    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:361:5+1
    assume {:print "$at(135,15916,15917)"} true;
    assume {:print "$track_local(55,33,0):", $t0} $t0 == $t0;

    // assume Identical($t3, global<object::ObjectCore>(signer::$address_of($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:336:9+65
    assume {:print "$at(136,13878,13943)"} true;
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // $t4 := object::create_guid($t0) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:33+19
    assume {:print "$at(135,16070,16089)"} true;
    call $t4 := $1_object_create_guid($t0);
    if ($abort_flag) {
        assume {:print "$at(135,16070,16089)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,33):", $t5} $t5 == $t5;
        goto L2;
    }

    // $t6 := event::new_event_handle<#0>($t4) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+44
    call $t6 := $1_event_new_event_handle'$1_fungible_asset_WithdrawEvent'($t4);
    if ($abort_flag) {
        assume {:print "$at(135,16046,16090)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,33):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_return[0]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+44
    assume {:print "$track_return(55,33,0):", $t6} $t6 == $t6;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
L1:

    // return $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
    $ret0 := $t6;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
L2:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::new_event_handle<fungible_asset::FrozenEvent> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:361:5+180
procedure {:inline 1} $1_object_new_event_handle'$1_fungible_asset_FrozenEvent'(_$t0: $signer) returns ($ret0: $1_event_EventHandle'$1_fungible_asset_FrozenEvent')
{
    // declare local variables
    var $t1: $1_object_ObjectCore;
    var $t2: $1_guid_GUID;
    var $t3: $1_object_ObjectCore;
    var $t4: $1_guid_GUID;
    var $t5: int;
    var $t6: $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $t0: $signer;
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_FrozenEvent'': $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // assume Identical($t1, global<object::ObjectCore>(signer::$address_of($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:352:9+65
    assume {:print "$at(136,14413,14478)"} true;
    assume ($t1 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // assume Identical($t2, pack guid::GUID(pack guid::ID(select object::ObjectCore.guid_creation_num($t1), signer::$address_of($t0)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:355:9+185
    assume {:print "$at(136,14551,14736)"} true;
    assume ($t2 == $1_guid_GUID($1_guid_ID($t1->$guid_creation_num, $1_signer_$address_of($t0))));

    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:361:5+1
    assume {:print "$at(135,15916,15917)"} true;
    assume {:print "$track_local(55,33,0):", $t0} $t0 == $t0;

    // assume Identical($t3, global<object::ObjectCore>(signer::$address_of($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:336:9+65
    assume {:print "$at(136,13878,13943)"} true;
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // $t4 := object::create_guid($t0) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:33+19
    assume {:print "$at(135,16070,16089)"} true;
    call $t4 := $1_object_create_guid($t0);
    if ($abort_flag) {
        assume {:print "$at(135,16070,16089)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,33):", $t5} $t5 == $t5;
        goto L2;
    }

    // $t6 := event::new_event_handle<#0>($t4) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+44
    call $t6 := $1_event_new_event_handle'$1_fungible_asset_FrozenEvent'($t4);
    if ($abort_flag) {
        assume {:print "$at(135,16046,16090)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,33):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_return[0]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+44
    assume {:print "$track_return(55,33,0):", $t6} $t6 == $t6;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
L1:

    // return $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
    $ret0 := $t6;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
L2:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(135,16095,16096)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::create_guid [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:354:5+252
procedure {:inline 1} $1_object_create_guid(_$t0: $signer) returns ($ret0: $1_guid_GUID)
{
    // declare local variables
    var $t1: int;
    var $t2: $Mutation ($1_object_ObjectCore);
    var $t3: $1_object_ObjectCore;
    var $t4: int;
    var $t5: int;
    var $t6: $Mutation ($1_object_ObjectCore);
    var $t7: $Mutation (int);
    var $t8: $1_guid_GUID;
    var $t0: $signer;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // assume Identical($t3, global<object::ObjectCore>(signer::$address_of($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:336:9+65
    assume {:print "$at(136,13878,13943)"} true;
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:354:5+1
    assume {:print "$at(135,15621,15622)"} true;
    assume {:print "$track_local(55,7,0):", $t0} $t0 == $t0;

    // $t4 := signer::address_of($t0) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:355:20+26
    assume {:print "$at(135,15714,15740)"} true;
    call $t4 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(135,15714,15740)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_local[addr]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:355:13+4
    assume {:print "$track_local(55,7,1):", $t4} $t4 == $t4;

    // $t6 := borrow_global<object::ObjectCore>($t4) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:356:27+17
    assume {:print "$at(135,15768,15785)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t4)) {
        call $ExecFailureAbort();
    } else {
        $t6 := $Mutation($Global($t4), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t4));
    }
    if ($abort_flag) {
        assume {:print "$at(135,15768,15785)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_local[object_data]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:356:13+11
    $temp_0'$1_object_ObjectCore' := $Dereference($t6);
    assume {:print "$track_local(55,7,2):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t7 := borrow_field<object::ObjectCore>.guid_creation_num($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:28+34
    assume {:print "$at(135,15832,15866)"} true;
    $t7 := $ChildMutation($t6, 0, $Dereference($t6)->$guid_creation_num);

    // $t8 := guid::create($t4, $t7) on_abort goto L2 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:9+54
    call $t8,$t7 := $1_guid_create($t4, $t7);
    if ($abort_flag) {
        assume {:print "$at(135,15813,15867)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(55,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // write_back[Reference($t6).guid_creation_num (u64)]($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:9+54
    $t6 := $UpdateMutation($t6, $Update'$1_object_ObjectCore'_guid_creation_num($Dereference($t6), $Dereference($t7)));

    // write_back[object::ObjectCore@]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:9+54
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t6),
        $Dereference($t6));

    // trace_return[0]($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:9+54
    assume {:print "$track_return(55,7,0):", $t8} $t8 == $t8;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:5+1
    assume {:print "$at(135,15872,15873)"} true;
L1:

    // return $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:5+1
    assume {:print "$at(135,15872,15873)"} true;
    $ret0 := $t8;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:5+1
L2:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:5+1
    assume {:print "$at(135,15872,15873)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::address_from_extend_ref [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:399:5+85
procedure {:inline 1} $1_object_address_from_extend_ref(_$t0: $1_object_ExtendRef) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_ExtendRef;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:399:5+1
    assume {:print "$at(135,17096,17097)"} true;
    assume {:print "$track_local(55,2,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ExtendRef>.self($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:400:9+8
    assume {:print "$at(135,17167,17175)"} true;
    $t1 := $t0->$self;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:400:9+8
    assume {:print "$track_return(55,2,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:401:5+1
    assume {:print "$at(135,17180,17181)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:401:5+1
    assume {:print "$at(135,17180,17181)"} true;
    $ret0 := $t1;
    return;

}

// fun object::address_to_object<fungible_asset::FungibleStore> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:172:5+278
procedure {:inline 1} $1_object_address_to_object'$1_fungible_asset_FungibleStore'(_$t0: int) returns ($ret0: $1_object_Object'$1_fungible_asset_FungibleStore')
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $t0: int;
    var $temp_0'$1_object_Object'$1_fungible_asset_FungibleStore'': $1_object_Object'$1_fungible_asset_FungibleStore';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:172:5+1
    assume {:print "$at(135,7475,7476)"} true;
    assume {:print "$track_local(55,3,0):", $t0} $t0 == $t0;

    // $t1 := exists<object::ObjectCore>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:17+6
    assume {:print "$at(135,7558,7564)"} true;
    $t1 := $ResourceExists($1_object_ObjectCore_$memory, $t0);

    // if ($t1) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    if ($t1) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    assume {:print "$at(135,7550,7627)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:62+22
L0:

    // $t2 := 2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:62+22
    assume {:print "$at(135,7603,7625)"} true;
    $t2 := 2;
    assume $IsValid'u64'($t2);

    // $t3 := error::not_found($t2) on_abort goto L7 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:45+40
    call $t3 := $1_error_not_found($t2);
    if ($abort_flag) {
        assume {:print "$at(135,7586,7626)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(55,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    assume {:print "$at(135,7550,7627)"} true;
    assume {:print "$track_abort(55,3):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    $t4 := $t3;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    goto L7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:30+6
    assume {:print "$at(135,7658,7664)"} true;
L2:

    // $t5 := opaque begin: object::exists_at<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20
    assume {:print "$at(135,7645,7665)"} true;

    // assume WellFormed($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20
    assume $IsValid'bool'($t5);

    // assume Eq<bool>($t5, object::spec_exists_at<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20
    assume $IsEqual'bool'($t5, $1_object_spec_exists_at'$1_fungible_asset_FungibleStore'($t0));

    // $t5 := opaque end: object::exists_at<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20

    // if ($t5) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    if ($t5) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    assume {:print "$at(135,7637,7710)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:56+24
L3:

    // $t6 := 7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:56+24
    assume {:print "$at(135,7684,7708)"} true;
    $t6 := 7;
    assume $IsValid'u64'($t6);

    // $t7 := error::not_found($t6) on_abort goto L7 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:39+42
    call $t7 := $1_error_not_found($t6);
    if ($abort_flag) {
        assume {:print "$at(135,7667,7709)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(55,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    assume {:print "$at(135,7637,7710)"} true;
    assume {:print "$track_abort(55,3):", $t7} $t7 == $t7;

    // $t4 := move($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    $t4 := $t7;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:28+6
    assume {:print "$at(135,7739,7745)"} true;
L5:

    // $t8 := pack object::Object<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:9+27
    assume {:print "$at(135,7720,7747)"} true;
    $t8 := $1_object_Object'$1_fungible_asset_FungibleStore'($t0);

    // trace_return[0]($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:9+27
    assume {:print "$track_return(55,3,0):", $t8} $t8 == $t8;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(135,7752,7753)"} true;
L6:

    // return $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(135,7752,7753)"} true;
    $ret0 := $t8;
    return;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
L7:

    // abort($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(135,7752,7753)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::address_to_object<fungible_asset::Metadata> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:172:5+278
procedure {:inline 1} $1_object_address_to_object'$1_fungible_asset_Metadata'(_$t0: int) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: int;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:172:5+1
    assume {:print "$at(135,7475,7476)"} true;
    assume {:print "$track_local(55,3,0):", $t0} $t0 == $t0;

    // $t1 := exists<object::ObjectCore>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:17+6
    assume {:print "$at(135,7558,7564)"} true;
    $t1 := $ResourceExists($1_object_ObjectCore_$memory, $t0);

    // if ($t1) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    if ($t1) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    assume {:print "$at(135,7550,7627)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:62+22
L0:

    // $t2 := 2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:62+22
    assume {:print "$at(135,7603,7625)"} true;
    $t2 := 2;
    assume $IsValid'u64'($t2);

    // $t3 := error::not_found($t2) on_abort goto L7 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:45+40
    call $t3 := $1_error_not_found($t2);
    if ($abort_flag) {
        assume {:print "$at(135,7586,7626)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(55,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    assume {:print "$at(135,7550,7627)"} true;
    assume {:print "$track_abort(55,3):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    $t4 := $t3;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+77
    goto L7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:30+6
    assume {:print "$at(135,7658,7664)"} true;
L2:

    // $t5 := opaque begin: object::exists_at<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20
    assume {:print "$at(135,7645,7665)"} true;

    // assume WellFormed($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20
    assume $IsValid'bool'($t5);

    // assume Eq<bool>($t5, object::spec_exists_at<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20
    assume $IsEqual'bool'($t5, $1_object_spec_exists_at'$1_fungible_asset_Metadata'($t0));

    // $t5 := opaque end: object::exists_at<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:17+20

    // if ($t5) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    if ($t5) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    assume {:print "$at(135,7637,7710)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:56+24
L3:

    // $t6 := 7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:56+24
    assume {:print "$at(135,7684,7708)"} true;
    $t6 := 7;
    assume $IsValid'u64'($t6);

    // $t7 := error::not_found($t6) on_abort goto L7 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:39+42
    call $t7 := $1_error_not_found($t6);
    if ($abort_flag) {
        assume {:print "$at(135,7667,7709)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(55,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    assume {:print "$at(135,7637,7710)"} true;
    assume {:print "$track_abort(55,3):", $t7} $t7 == $t7;

    // $t4 := move($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    $t4 := $t7;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+73
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:28+6
    assume {:print "$at(135,7739,7745)"} true;
L5:

    // $t8 := pack object::Object<#0>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:9+27
    assume {:print "$at(135,7720,7747)"} true;
    $t8 := $1_object_Object'$1_fungible_asset_Metadata'($t0);

    // trace_return[0]($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:9+27
    assume {:print "$track_return(55,3,0):", $t8} $t8 == $t8;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(135,7752,7753)"} true;
L6:

    // return $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(135,7752,7753)"} true;
    $ret0 := $t8;
    return;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
L7:

    // abort($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(135,7752,7753)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::can_generate_delete_ref [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:347:5+93
procedure {:inline 1} $1_object_can_generate_delete_ref(_$t0: $1_object_ConstructorRef) returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:347:5+1
    assume {:print "$at(135,15424,15425)"} true;
    assume {:print "$track_local(55,5,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.can_delete($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:348:9+14
    assume {:print "$at(135,15497,15511)"} true;
    $t1 := $t0->$can_delete;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:348:9+14
    assume {:print "$track_return(55,5,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:349:5+1
    assume {:print "$at(135,15516,15517)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:349:5+1
    assume {:print "$at(135,15516,15517)"} true;
    $ret0 := $t1;
    return;

}

// fun object::convert<#0, fungible_asset::Metadata> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:220:5+115
procedure {:inline 1} $1_object_convert'#0_$1_fungible_asset_Metadata'(_$t0: $1_object_Object'#0') returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:220:5+1
    assume {:print "$at(135,9573,9574)"} true;
    assume {:print "$track_local(55,6,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:221:30+12
    assume {:print "$at(135,9669,9681)"} true;
    $t1 := $t0->$inner;

    // $t2 := object::address_to_object<#1>($t1) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:221:9+34
    call $t2 := $1_object_address_to_object'$1_fungible_asset_Metadata'($t1);
    if ($abort_flag) {
        assume {:print "$at(135,9648,9682)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(55,6):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:221:9+34
    assume {:print "$track_return(55,6,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:5+1
    assume {:print "$at(135,9687,9688)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:5+1
    assume {:print "$at(135,9687,9688)"} true;
    $ret0 := $t2;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:5+1
L2:

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:5+1
    assume {:print "$at(135,9687,9688)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::generate_signer [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:332:5+96
procedure {:inline 1} $1_object_generate_signer(_$t0: $1_object_ConstructorRef) returns ($ret0: $signer)
{
    // declare local variables
    var $t1: int;
    var $t2: $signer;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:332:5+1
    assume {:print "$at(135,14889,14890)"} true;
    assume {:print "$track_local(55,27,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:333:23+8
    assume {:print "$at(135,14970,14978)"} true;
    $t1 := $t0->$self;

    // $t2 := opaque begin: create_signer::create_signer($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:333:9+23

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:333:9+23
    assume $IsValid'signer'($t2) && $1_signer_is_txn_signer($t2) && $1_signer_is_txn_signer_addr($t2->$addr);

    // assume Eq<address>(signer::$address_of($t2), $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:333:9+23
    assume $IsEqual'address'($1_signer_$address_of($t2), $t1);

    // $t2 := opaque end: create_signer::create_signer($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:333:9+23

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:333:9+23
    assume {:print "$track_return(55,27,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:334:5+1
    assume {:print "$at(135,14984,14985)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:334:5+1
    assume {:print "$at(135,14984,14985)"} true;
    $ret0 := $t2;
    return;

}

// fun object::generate_signer_for_extending [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:394:5+105
procedure {:inline 1} $1_object_generate_signer_for_extending(_$t0: $1_object_ExtendRef) returns ($ret0: $signer)
{
    // declare local variables
    var $t1: int;
    var $t2: $signer;
    var $t0: $1_object_ExtendRef;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:394:5+1
    assume {:print "$at(135,16933,16934)"} true;
    assume {:print "$track_local(55,28,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ExtendRef>.self($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:395:23+8
    assume {:print "$at(135,17023,17031)"} true;
    $t1 := $t0->$self;

    // $t2 := opaque begin: create_signer::create_signer($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:395:9+23

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:395:9+23
    assume $IsValid'signer'($t2) && $1_signer_is_txn_signer($t2) && $1_signer_is_txn_signer_addr($t2->$addr);

    // assume Eq<address>(signer::$address_of($t2), $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:395:9+23
    assume $IsEqual'address'($1_signer_$address_of($t2), $t1);

    // $t2 := opaque end: create_signer::create_signer($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:395:9+23

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:395:9+23
    assume {:print "$track_return(55,28,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume {:print "$at(135,17037,17038)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume {:print "$at(135,17037,17038)"} true;
    $ret0 := $t2;
    return;

}

// fun object::object_address<fungible_asset::FungibleStore> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+91
procedure {:inline 1} $1_object_object_address'$1_fungible_asset_FungibleStore'(_$t0: $1_object_Object'$1_fungible_asset_FungibleStore') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $temp_0'$1_object_Object'$1_fungible_asset_FungibleStore'': $1_object_Object'$1_fungible_asset_FungibleStore';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+1
    assume {:print "$at(135,9436,9437)"} true;
    assume {:print "$track_local(55,34,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+12
    assume {:print "$at(135,9509,9521)"} true;
    $t1 := $t0->$inner;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+12
    assume {:print "$track_return(55,34,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(135,9526,9527)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(135,9526,9527)"} true;
    $ret0 := $t1;
    return;

}

// fun object::object_address<fungible_asset::Metadata> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+91
procedure {:inline 1} $1_object_object_address'$1_fungible_asset_Metadata'(_$t0: $1_object_Object'$1_fungible_asset_Metadata') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+1
    assume {:print "$at(135,9436,9437)"} true;
    assume {:print "$track_local(55,34,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+12
    assume {:print "$at(135,9509,9521)"} true;
    $t1 := $t0->$inner;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+12
    assume {:print "$track_return(55,34,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(135,9526,9527)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(135,9526,9527)"} true;
    $ret0 := $t1;
    return;

}

// fun object::object_address<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+91
procedure {:inline 1} $1_object_object_address'#0'(_$t0: $1_object_Object'#0') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:5+1
    assume {:print "$at(135,9436,9437)"} true;
    assume {:print "$track_local(55,34,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+12
    assume {:print "$at(135,9509,9521)"} true;
    $t1 := $t0->$inner;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+12
    assume {:print "$track_return(55,34,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(135,9526,9527)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(135,9526,9527)"} true;
    $ret0 := $t1;
    return;

}

// fun object::object_from_constructor_ref<fungible_asset::FungibleStore> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:342:5+126
procedure {:inline 1} $1_object_object_from_constructor_ref'$1_fungible_asset_FungibleStore'(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_Object'$1_fungible_asset_FungibleStore')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $t3: int;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_FungibleStore'': $1_object_Object'$1_fungible_asset_FungibleStore';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:342:5+1
    assume {:print "$at(135,15210,15211)"} true;
    assume {:print "$track_local(55,36,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:343:30+8
    assume {:print "$at(135,15321,15329)"} true;
    $t1 := $t0->$self;

    // $t2 := object::address_to_object<#0>($t1) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:343:9+30
    call $t2 := $1_object_address_to_object'$1_fungible_asset_FungibleStore'($t1);
    if ($abort_flag) {
        assume {:print "$at(135,15300,15330)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(55,36):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:343:9+30
    assume {:print "$track_return(55,36,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$at(135,15335,15336)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$at(135,15335,15336)"} true;
    $ret0 := $t2;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
L2:

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$at(135,15335,15336)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::object_from_constructor_ref<fungible_asset::Metadata> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:342:5+126
procedure {:inline 1} $1_object_object_from_constructor_ref'$1_fungible_asset_Metadata'(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:342:5+1
    assume {:print "$at(135,15210,15211)"} true;
    assume {:print "$track_local(55,36,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:343:30+8
    assume {:print "$at(135,15321,15329)"} true;
    $t1 := $t0->$self;

    // $t2 := object::address_to_object<#0>($t1) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:343:9+30
    call $t2 := $1_object_address_to_object'$1_fungible_asset_Metadata'($t1);
    if ($abort_flag) {
        assume {:print "$at(135,15300,15330)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(55,36):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:343:9+30
    assume {:print "$track_return(55,36,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$at(135,15335,15336)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$at(135,15335,15336)"} true;
    $ret0 := $t2;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
L2:

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$at(135,15335,15336)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::object_from_delete_ref<fungible_asset::FungibleStore> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:375:5+116
procedure {:inline 1} $1_object_object_from_delete_ref'$1_fungible_asset_FungibleStore'(_$t0: $1_object_DeleteRef) returns ($ret0: $1_object_Object'$1_fungible_asset_FungibleStore')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $t3: int;
    var $t0: $1_object_DeleteRef;
    var $temp_0'$1_object_DeleteRef': $1_object_DeleteRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_FungibleStore'': $1_object_Object'$1_fungible_asset_FungibleStore';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:375:5+1
    assume {:print "$at(135,16332,16333)"} true;
    assume {:print "$track_local(55,37,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::DeleteRef>.self($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:376:30+8
    assume {:print "$at(135,16433,16441)"} true;
    $t1 := $t0->$self;

    // $t2 := object::address_to_object<#0>($t1) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:376:9+30
    call $t2 := $1_object_address_to_object'$1_fungible_asset_FungibleStore'($t1);
    if ($abort_flag) {
        assume {:print "$at(135,16412,16442)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(55,37):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:376:9+30
    assume {:print "$track_return(55,37,0):", $t2} $t2 == $t2;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:377:5+1
    assume {:print "$at(135,16447,16448)"} true;
L1:

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:377:5+1
    assume {:print "$at(135,16447,16448)"} true;
    $ret0 := $t2;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:377:5+1
L2:

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:377:5+1
    assume {:print "$at(135,16447,16448)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::owns<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:596:5+1145
procedure {:inline 1} $1_object_owns'#0'(_$t0: $1_object_Object'#0', _$t1: int) returns ($ret0: bool)
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: $1_object_ObjectCore;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: bool;
    var $t11: bool;
    var $t12: bool;
    var $t13: bool;
    var $t14: int;
    var $t15: int;
    var $t16: $1_object_ObjectCore;
    var $t17: int;
    var $t18: int;
    var $t19: bool;
    var $t20: int;
    var $t21: int;
    var $t22: int;
    var $t23: bool;
    var $t24: bool;
    var $t25: bool;
    var $t26: $1_object_ObjectCore;
    var $t27: int;
    var $t28: int;
    var $t29: int;
    var $t30: bool;
    var $t31: bool;
    var $t0: $1_object_Object'#0';
    var $t1: int;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'u8': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t5, select object::Object.inner($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:507:9+37
    assume {:print "$at(136,20290,20327)"} true;
    assume ($t5 == $t0->$inner);

    // assume Identical($t6, global<object::ObjectCore>($t5)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:508:9+53
    assume {:print "$at(136,20336,20389)"} true;
    assume ($t6 == $ResourceValue($1_object_ObjectCore_$memory, $t5));

    // assume Identical($t7, select object::ObjectCore.owner($t6)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:509:9+37
    assume {:print "$at(136,20398,20435)"} true;
    assume ($t7 == $t6->$owner);

    // trace_local[object]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:596:5+1
    assume {:print "$at(135,25063,25064)"} true;
    assume {:print "$track_local(55,39,0):", $t0} $t0 == $t0;

    // trace_local[owner]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:596:5+1
    assume {:print "$track_local(55,39,1):", $t1} $t1 == $t1;

    // $t8 := object::object_address<#0>($t0) on_abort goto L16 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:597:31+23
    assume {:print "$at(135,25180,25203)"} true;
    call $t8 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(135,25180,25203)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(55,39):", $t9} $t9 == $t9;
        goto L16;
    }

    // trace_local[current_address]($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:597:13+15
    assume {:print "$track_local(55,39,3):", $t8} $t8 == $t8;

    // $t10 := ==($t8, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:598:29+2
    assume {:print "$at(135,25233,25235)"} true;
    $t10 := $IsEqual'address'($t8, $t1);

    // if ($t10) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:598:9+65
    if ($t10) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:599:20+4
    assume {:print "$at(135,25264,25268)"} true;
L1:

    // $t11 := true at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:599:20+4
    assume {:print "$at(135,25264,25268)"} true;
    $t11 := true;
    assume $IsValid'bool'($t11);

    // trace_return[0]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:599:13+11
    assume {:print "$track_return(55,39,0):", $t11} $t11 == $t11;

    // $t12 := move($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:599:13+11
    $t12 := $t11;

    // goto L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:599:13+11
    goto L15;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:603:32+15
    assume {:print "$at(135,25329,25344)"} true;
L0:

    // $t13 := exists<object::ObjectCore>($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:603:13+6
    assume {:print "$at(135,25310,25316)"} true;
    $t13 := $ResourceExists($1_object_ObjectCore_$memory, $t8);

    // if ($t13) goto L3 else goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:602:9+121
    assume {:print "$at(135,25289,25410)"} true;
    if ($t13) { goto L3; } else { goto L2; }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:602:9+121
L3:

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:602:9+121
    assume {:print "$at(135,25289,25410)"} true;
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:604:30+22
    assume {:print "$at(135,25376,25398)"} true;
L2:

    // $t14 := 2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:604:30+22
    assume {:print "$at(135,25376,25398)"} true;
    $t14 := 2;
    assume $IsValid'u64'($t14);

    // $t15 := error::not_found($t14) on_abort goto L16 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:604:13+40
    call $t15 := $1_error_not_found($t14);
    if ($abort_flag) {
        assume {:print "$at(135,25359,25399)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(55,39):", $t9} $t9 == $t9;
        goto L16;
    }

    // trace_abort($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:602:9+121
    assume {:print "$at(135,25289,25410)"} true;
    assume {:print "$track_abort(55,39):", $t15} $t15 == $t15;

    // $t9 := move($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:602:9+121
    $t9 := $t15;

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:602:9+121
    goto L16;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:607:48+15
    assume {:print "$at(135,25460,25475)"} true;
L4:

    // $t16 := get_global<object::ObjectCore>($t8) on_abort goto L16 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:607:22+13
    assume {:print "$at(135,25434,25447)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t8)) {
        call $ExecFailureAbort();
    } else {
        $t16 := $ResourceValue($1_object_ObjectCore_$memory, $t8);
    }
    if ($abort_flag) {
        assume {:print "$at(135,25434,25447)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(55,39):", $t9} $t9 == $t9;
        goto L16;
    }

    // $t17 := get_field<object::ObjectCore>.owner($t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:608:31+12
    assume {:print "$at(135,25508,25520)"} true;
    $t17 := $t16->$owner;

    // trace_local[current_address#2]($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:608:13+15
    assume {:print "$track_local(55,39,4):", $t17} $t17 == $t17;

    // $t18 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:610:21+1
    assume {:print "$at(135,25543,25544)"} true;
    $t18 := 0;
    assume $IsValid'u8'($t18);

    // trace_local[count]($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:610:13+5
    assume {:print "$track_local(55,39,2):", $t18} $t18 == $t18;

    // label L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:612:13+211
    assume {:print "$at(135,25575,25786)"} true;
L13:

    // assert Lt($t18, 8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:613:17+41
    assume {:print "$at(135,25598,25639)"} true;
    assert {:msg "assert_failed(135,25598,25639): base case of the loop invariant does not hold"}
      ($t18 < 8);

    // assert forall i: num: Range(0, $t18): And(Neq<address>($t1, $t17), exists<object::ObjectCore>($t17)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume {:print "$at(135,25656,25772)"} true;
    assert {:msg "assert_failed(135,25656,25772): base case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t18); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var i := $i_1;
    ((!$IsEqual'address'($t1, $t17) && $ResourceExists($1_object_ObjectCore_$memory, $t17))))));

    // $t4 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t4;

    // assume WellFormed($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'address'($t4);

    // $t19 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t19;

    // assume WellFormed($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'bool'($t19);

    // $t20 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t20;

    // assume WellFormed($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'u8'($t20);

    // $t21 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t21;

    // assume WellFormed($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'u8'($t21);

    // $t22 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t22;

    // assume WellFormed($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'u8'($t22);

    // $t23 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t23;

    // assume WellFormed($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'bool'($t23);

    // $t24 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t24;

    // assume WellFormed($t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'bool'($t24);

    // $t25 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t25;

    // assume WellFormed($t25) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'bool'($t25);

    // $t26 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t26;

    // assume WellFormed($t26) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'$1_object_ObjectCore'($t26);

    // $t27 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    havoc $t27;

    // assume WellFormed($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume $IsValid'address'($t27);

    // trace_local[current_address#2]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume {:print "$info(): enter loop, variable(s) current_address#2 havocked and reassigned"} true;
    assume {:print "$track_local(55,39,4):", $t4} $t4 == $t4;

    // assume Not(AbortFlag()) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume {:print "$info(): loop invariant holds at current state"} true;
    assume !$abort_flag;

    // assume Lt($t18, 8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:613:17+41
    assume {:print "$at(135,25598,25639)"} true;
    assume ($t18 < 8);

    // assume forall i: num: Range(0, $t18): And(Neq<address>($t1, $t4), exists<object::ObjectCore>($t4)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume {:print "$at(135,25656,25772)"} true;
    assume (var $range_0 := $Range(0, $t18); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var i := $i_1;
    ((!$IsEqual'address'($t1, $t4) && $ResourceExists($1_object_ObjectCore_$memory, $t4))))));

    // $t19 := !=($t1, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:617:19+2
    assume {:print "$at(135,25806,25808)"} true;
    $t19 := !$IsEqual'address'($t1, $t4);

    // if ($t19) goto L6 else goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:611:9+634
    assume {:print "$at(135,25554,26188)"} true;
    if ($t19) { goto L6; } else { goto L5; }

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:611:9+634
L6:

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:619:25+5
    assume {:print "$at(135,25862,25867)"} true;
L7:

    // $t20 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:619:33+1
    assume {:print "$at(135,25870,25871)"} true;
    $t20 := 1;
    assume $IsValid'u8'($t20);

    // $t21 := +($t18, $t20) on_abort goto L16 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:619:31+1
    call $t21 := $AddU8($t18, $t20);
    if ($abort_flag) {
        assume {:print "$at(135,25868,25869)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(55,39):", $t9} $t9 == $t9;
        goto L16;
    }

    // $t22 := 8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:29+22
    assume {:print "$at(135,25901,25923)"} true;
    $t22 := 8;
    assume $IsValid'u8'($t22);

    // $t23 := <($t21, $t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:27+1
    call $t23 := $Lt($t21, $t22);

    // if ($t23) goto L9 else goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:13+78
    if ($t23) { goto L9; } else { goto L8; }

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:13+78
L9:

    // goto L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:13+78
    assume {:print "$at(135,25885,25963)"} true;
    goto L10;

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:73+16
L8:

    // $t28 := 6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:73+16
    assume {:print "$at(135,25945,25961)"} true;
    $t28 := 6;
    assume $IsValid'u64'($t28);

    // $t29 := error::out_of_range($t28) on_abort goto L16 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:53+37
    call $t29 := $1_error_out_of_range($t28);
    if ($abort_flag) {
        assume {:print "$at(135,25925,25962)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(55,39):", $t9} $t9 == $t9;
        goto L16;
    }

    // trace_abort($t29) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:13+78
    assume {:print "$at(135,25885,25963)"} true;
    assume {:print "$track_abort(55,39):", $t29} $t29 == $t29;

    // $t9 := move($t29) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:13+78
    $t9 := $t29;

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:620:13+78
    goto L16;

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:621:37+15
    assume {:print "$at(135,26001,26016)"} true;
L10:

    // $t24 := exists<object::ObjectCore>($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:621:18+6
    assume {:print "$at(135,25982,25988)"} true;
    $t24 := $ResourceExists($1_object_ObjectCore_$memory, $t4);

    // $t25 := !($t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:621:17+1
    call $t25 := $Not($t24);

    // if ($t25) goto L12 else goto L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:621:13+86
    if ($t25) { goto L12; } else { goto L11; }

    // label L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:622:24+5
    assume {:print "$at(135,26044,26049)"} true;
L12:

    // $t30 := false at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:622:24+5
    assume {:print "$at(135,26044,26049)"} true;
    $t30 := false;
    assume $IsValid'bool'($t30);

    // trace_return[0]($t30) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:622:17+12
    assume {:print "$track_return(55,39,0):", $t30} $t30 == $t30;

    // $t12 := move($t30) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:622:17+12
    $t12 := $t30;

    // goto L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:622:17+12
    goto L15;

    // label L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:625:52+15
    assume {:print "$at(135,26117,26132)"} true;
L11:

    // $t26 := get_global<object::ObjectCore>($t4) on_abort goto L16 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:625:26+13
    assume {:print "$at(135,26091,26104)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t4)) {
        call $ExecFailureAbort();
    } else {
        $t26 := $ResourceValue($1_object_ObjectCore_$memory, $t4);
    }
    if ($abort_flag) {
        assume {:print "$at(135,26091,26104)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(55,39):", $t9} $t9 == $t9;
        goto L16;
    }

    // $t27 := get_field<object::ObjectCore>.owner($t26) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:626:31+12
    assume {:print "$at(135,26165,26177)"} true;
    $t27 := $t26->$owner;

    // trace_local[current_address#2]($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:626:13+15
    assume {:print "$track_local(55,39,4):", $t27} $t27 == $t27;

    // goto L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:626:43+1
    goto L14;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:628:9+4
    assume {:print "$at(135,26198,26202)"} true;
L5:

    // $t31 := true at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:628:9+4
    assume {:print "$at(135,26198,26202)"} true;
    $t31 := true;
    assume $IsValid'bool'($t31);

    // trace_return[0]($t31) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:628:9+4
    assume {:print "$track_return(55,39,0):", $t31} $t31 == $t31;

    // $t12 := move($t31) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:628:9+4
    $t12 := $t31;

    // goto L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:628:9+4
    goto L15;

    // label L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:628:9+4
    // Loop invariant checking block for the loop started with header: L13
L14:

    // assert Lt($t18, 8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:613:17+41
    assume {:print "$at(135,25598,25639)"} true;
    assert {:msg "assert_failed(135,25598,25639): induction case of the loop invariant does not hold"}
      ($t18 < 8);

    // assert forall i: num: Range(0, $t18): And(Neq<address>($t1, $t27), exists<object::ObjectCore>($t27)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume {:print "$at(135,25656,25772)"} true;
    assert {:msg "assert_failed(135,25656,25772): induction case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t18); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var i := $i_1;
    ((!$IsEqual'address'($t1, $t27) && $ResourceExists($1_object_ObjectCore_$memory, $t27))))));

    // stop() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:614:17+116
    assume false;
    return;

    // label L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:629:5+1
    assume {:print "$at(135,26207,26208)"} true;
L15:

    // return $t12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:629:5+1
    assume {:print "$at(135,26207,26208)"} true;
    $ret0 := $t12;
    return;

    // label L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:629:5+1
L16:

    // abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:629:5+1
    assume {:print "$at(135,26207,26208)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:638:5+174
function {:inline} $1_fungible_asset_$borrow_store_resource'#0'($1_fungible_asset_FungibleStore_$memory: $Memory $1_fungible_asset_FungibleStore, store: $1_object_Object'#0'): $1_fungible_asset_FungibleStore {
    $ResourceValue($1_fungible_asset_FungibleStore_$memory, $1_object_$object_address'#0'(store))
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:323:5+176
function {:inline} $1_fungible_asset_$is_frozen'#0'($1_fungible_asset_FungibleStore_$memory: $Memory $1_fungible_asset_FungibleStore, store: $1_object_Object'#0'): bool {
    ($1_fungible_asset_$store_exists($1_fungible_asset_FungibleStore_$memory, $1_object_$object_address'#0'(store)) && $1_fungible_asset_$borrow_store_resource'#0'($1_fungible_asset_FungibleStore_$memory, store)->$frozen)
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:289:5+90
function {:inline} $1_fungible_asset_$store_exists($1_fungible_asset_FungibleStore_$memory: $Memory $1_fungible_asset_FungibleStore, store: int): bool {
    $ResourceExists($1_fungible_asset_FungibleStore_$memory, store)
}

// spec fun at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:300:5+147
function {:inline} $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory: $Memory $1_fungible_asset_FungibleStore, store: $1_object_Object'#0'): $1_object_Object'$1_fungible_asset_Metadata' {
    $1_fungible_asset_$borrow_store_resource'#0'($1_fungible_asset_FungibleStore_$memory, store)->$metadata
}

// struct fungible_asset::DepositEvent at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:145:5+64
datatype $1_fungible_asset_DepositEvent {
    $1_fungible_asset_DepositEvent($amount: int)
}
function {:inline} $Update'$1_fungible_asset_DepositEvent'_amount(s: $1_fungible_asset_DepositEvent, x: int): $1_fungible_asset_DepositEvent {
    $1_fungible_asset_DepositEvent(x)
}
function $IsValid'$1_fungible_asset_DepositEvent'(s: $1_fungible_asset_DepositEvent): bool {
    $IsValid'u64'(s->$amount)
}
function {:inline} $IsEqual'$1_fungible_asset_DepositEvent'(s1: $1_fungible_asset_DepositEvent, s2: $1_fungible_asset_DepositEvent): bool {
    s1 == s2
}

// struct fungible_asset::WithdrawEvent at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:150:5+65
datatype $1_fungible_asset_WithdrawEvent {
    $1_fungible_asset_WithdrawEvent($amount: int)
}
function {:inline} $Update'$1_fungible_asset_WithdrawEvent'_amount(s: $1_fungible_asset_WithdrawEvent, x: int): $1_fungible_asset_WithdrawEvent {
    $1_fungible_asset_WithdrawEvent(x)
}
function $IsValid'$1_fungible_asset_WithdrawEvent'(s: $1_fungible_asset_WithdrawEvent): bool {
    $IsValid'u64'(s->$amount)
}
function {:inline} $IsEqual'$1_fungible_asset_WithdrawEvent'(s1: $1_fungible_asset_WithdrawEvent, s2: $1_fungible_asset_WithdrawEvent): bool {
    s1 == s2
}

// struct fungible_asset::TransferRef at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:135:5+77
datatype $1_fungible_asset_TransferRef {
    $1_fungible_asset_TransferRef($metadata: $1_object_Object'$1_fungible_asset_Metadata')
}
function {:inline} $Update'$1_fungible_asset_TransferRef'_metadata(s: $1_fungible_asset_TransferRef, x: $1_object_Object'$1_fungible_asset_Metadata'): $1_fungible_asset_TransferRef {
    $1_fungible_asset_TransferRef(x)
}
function $IsValid'$1_fungible_asset_TransferRef'(s: $1_fungible_asset_TransferRef): bool {
    $IsValid'$1_object_Object'$1_fungible_asset_Metadata''(s->$metadata)
}
function {:inline} $IsEqual'$1_fungible_asset_TransferRef'(s1: $1_fungible_asset_TransferRef, s2: $1_fungible_asset_TransferRef): bool {
    s1 == s2
}

// struct fungible_asset::BurnRef at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:140:5+73
datatype $1_fungible_asset_BurnRef {
    $1_fungible_asset_BurnRef($metadata: $1_object_Object'$1_fungible_asset_Metadata')
}
function {:inline} $Update'$1_fungible_asset_BurnRef'_metadata(s: $1_fungible_asset_BurnRef, x: $1_object_Object'$1_fungible_asset_Metadata'): $1_fungible_asset_BurnRef {
    $1_fungible_asset_BurnRef(x)
}
function $IsValid'$1_fungible_asset_BurnRef'(s: $1_fungible_asset_BurnRef): bool {
    $IsValid'$1_object_Object'$1_fungible_asset_Metadata''(s->$metadata)
}
function {:inline} $IsEqual'$1_fungible_asset_BurnRef'(s1: $1_fungible_asset_BurnRef, s2: $1_fungible_asset_BurnRef): bool {
    s1 == s2
}

// struct fungible_asset::ConcurrentSupply at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:80:5+74
datatype $1_fungible_asset_ConcurrentSupply {
    $1_fungible_asset_ConcurrentSupply($current: $1_aggregator_v2_Aggregator'u128')
}
function {:inline} $Update'$1_fungible_asset_ConcurrentSupply'_current(s: $1_fungible_asset_ConcurrentSupply, x: $1_aggregator_v2_Aggregator'u128'): $1_fungible_asset_ConcurrentSupply {
    $1_fungible_asset_ConcurrentSupply(x)
}
function $IsValid'$1_fungible_asset_ConcurrentSupply'(s: $1_fungible_asset_ConcurrentSupply): bool {
    $IsValid'$1_aggregator_v2_Aggregator'u128''(s->$current)
}
function {:inline} $IsEqual'$1_fungible_asset_ConcurrentSupply'(s1: $1_fungible_asset_ConcurrentSupply, s2: $1_fungible_asset_ConcurrentSupply): bool {
    s1 == s2
}
var $1_fungible_asset_ConcurrentSupply_$memory: $Memory $1_fungible_asset_ConcurrentSupply;

// struct fungible_asset::FrozenEvent at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:155:5+64
datatype $1_fungible_asset_FrozenEvent {
    $1_fungible_asset_FrozenEvent($frozen: bool)
}
function {:inline} $Update'$1_fungible_asset_FrozenEvent'_frozen(s: $1_fungible_asset_FrozenEvent, x: bool): $1_fungible_asset_FrozenEvent {
    $1_fungible_asset_FrozenEvent(x)
}
function $IsValid'$1_fungible_asset_FrozenEvent'(s: $1_fungible_asset_FrozenEvent): bool {
    $IsValid'bool'(s->$frozen)
}
function {:inline} $IsEqual'$1_fungible_asset_FrozenEvent'(s1: $1_fungible_asset_FrozenEvent, s2: $1_fungible_asset_FrozenEvent): bool {
    s1 == s2
}

// struct fungible_asset::FungibleAsset at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:123:5+85
datatype $1_fungible_asset_FungibleAsset {
    $1_fungible_asset_FungibleAsset($metadata: $1_object_Object'$1_fungible_asset_Metadata', $amount: int)
}
function {:inline} $Update'$1_fungible_asset_FungibleAsset'_metadata(s: $1_fungible_asset_FungibleAsset, x: $1_object_Object'$1_fungible_asset_Metadata'): $1_fungible_asset_FungibleAsset {
    $1_fungible_asset_FungibleAsset(x, s->$amount)
}
function {:inline} $Update'$1_fungible_asset_FungibleAsset'_amount(s: $1_fungible_asset_FungibleAsset, x: int): $1_fungible_asset_FungibleAsset {
    $1_fungible_asset_FungibleAsset(s->$metadata, x)
}
function $IsValid'$1_fungible_asset_FungibleAsset'(s: $1_fungible_asset_FungibleAsset): bool {
    $IsValid'$1_object_Object'$1_fungible_asset_Metadata''(s->$metadata)
      && $IsValid'u64'(s->$amount)
}
function {:inline} $IsEqual'$1_fungible_asset_FungibleAsset'(s1: $1_fungible_asset_FungibleAsset, s2: $1_fungible_asset_FungibleAsset): bool {
    s1 == s2
}

// struct fungible_asset::FungibleAssetEvents at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:115:5+216
datatype $1_fungible_asset_FungibleAssetEvents {
    $1_fungible_asset_FungibleAssetEvents($deposit_events: $1_event_EventHandle'$1_fungible_asset_DepositEvent', $withdraw_events: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent', $frozen_events: $1_event_EventHandle'$1_fungible_asset_FrozenEvent')
}
function {:inline} $Update'$1_fungible_asset_FungibleAssetEvents'_deposit_events(s: $1_fungible_asset_FungibleAssetEvents, x: $1_event_EventHandle'$1_fungible_asset_DepositEvent'): $1_fungible_asset_FungibleAssetEvents {
    $1_fungible_asset_FungibleAssetEvents(x, s->$withdraw_events, s->$frozen_events)
}
function {:inline} $Update'$1_fungible_asset_FungibleAssetEvents'_withdraw_events(s: $1_fungible_asset_FungibleAssetEvents, x: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent'): $1_fungible_asset_FungibleAssetEvents {
    $1_fungible_asset_FungibleAssetEvents(s->$deposit_events, x, s->$frozen_events)
}
function {:inline} $Update'$1_fungible_asset_FungibleAssetEvents'_frozen_events(s: $1_fungible_asset_FungibleAssetEvents, x: $1_event_EventHandle'$1_fungible_asset_FrozenEvent'): $1_fungible_asset_FungibleAssetEvents {
    $1_fungible_asset_FungibleAssetEvents(s->$deposit_events, s->$withdraw_events, x)
}
function $IsValid'$1_fungible_asset_FungibleAssetEvents'(s: $1_fungible_asset_FungibleAssetEvents): bool {
    $IsValid'$1_event_EventHandle'$1_fungible_asset_DepositEvent''(s->$deposit_events)
      && $IsValid'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent''(s->$withdraw_events)
      && $IsValid'$1_event_EventHandle'$1_fungible_asset_FrozenEvent''(s->$frozen_events)
}
function {:inline} $IsEqual'$1_fungible_asset_FungibleAssetEvents'(s1: $1_fungible_asset_FungibleAssetEvents, s2: $1_fungible_asset_FungibleAssetEvents): bool {
    s1 == s2
}
var $1_fungible_asset_FungibleAssetEvents_$memory: $Memory $1_fungible_asset_FungibleAssetEvents;

// struct fungible_asset::FungibleStore at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:105:5+324
datatype $1_fungible_asset_FungibleStore {
    $1_fungible_asset_FungibleStore($metadata: $1_object_Object'$1_fungible_asset_Metadata', $balance: int, $frozen: bool)
}
function {:inline} $Update'$1_fungible_asset_FungibleStore'_metadata(s: $1_fungible_asset_FungibleStore, x: $1_object_Object'$1_fungible_asset_Metadata'): $1_fungible_asset_FungibleStore {
    $1_fungible_asset_FungibleStore(x, s->$balance, s->$frozen)
}
function {:inline} $Update'$1_fungible_asset_FungibleStore'_balance(s: $1_fungible_asset_FungibleStore, x: int): $1_fungible_asset_FungibleStore {
    $1_fungible_asset_FungibleStore(s->$metadata, x, s->$frozen)
}
function {:inline} $Update'$1_fungible_asset_FungibleStore'_frozen(s: $1_fungible_asset_FungibleStore, x: bool): $1_fungible_asset_FungibleStore {
    $1_fungible_asset_FungibleStore(s->$metadata, s->$balance, x)
}
function $IsValid'$1_fungible_asset_FungibleStore'(s: $1_fungible_asset_FungibleStore): bool {
    $IsValid'$1_object_Object'$1_fungible_asset_Metadata''(s->$metadata)
      && $IsValid'u64'(s->$balance)
      && $IsValid'bool'(s->$frozen)
}
function {:inline} $IsEqual'$1_fungible_asset_FungibleStore'(s1: $1_fungible_asset_FungibleStore, s2: $1_fungible_asset_FungibleStore): bool {
    s1 == s2
}
var $1_fungible_asset_FungibleStore_$memory: $Memory $1_fungible_asset_FungibleStore;

// struct fungible_asset::Metadata at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:86:5+785
datatype $1_fungible_asset_Metadata {
    $1_fungible_asset_Metadata($name: $1_string_String, $symbol: $1_string_String, $decimals: int, $icon_uri: $1_string_String, $project_uri: $1_string_String)
}
function {:inline} $Update'$1_fungible_asset_Metadata'_name(s: $1_fungible_asset_Metadata, x: $1_string_String): $1_fungible_asset_Metadata {
    $1_fungible_asset_Metadata(x, s->$symbol, s->$decimals, s->$icon_uri, s->$project_uri)
}
function {:inline} $Update'$1_fungible_asset_Metadata'_symbol(s: $1_fungible_asset_Metadata, x: $1_string_String): $1_fungible_asset_Metadata {
    $1_fungible_asset_Metadata(s->$name, x, s->$decimals, s->$icon_uri, s->$project_uri)
}
function {:inline} $Update'$1_fungible_asset_Metadata'_decimals(s: $1_fungible_asset_Metadata, x: int): $1_fungible_asset_Metadata {
    $1_fungible_asset_Metadata(s->$name, s->$symbol, x, s->$icon_uri, s->$project_uri)
}
function {:inline} $Update'$1_fungible_asset_Metadata'_icon_uri(s: $1_fungible_asset_Metadata, x: $1_string_String): $1_fungible_asset_Metadata {
    $1_fungible_asset_Metadata(s->$name, s->$symbol, s->$decimals, x, s->$project_uri)
}
function {:inline} $Update'$1_fungible_asset_Metadata'_project_uri(s: $1_fungible_asset_Metadata, x: $1_string_String): $1_fungible_asset_Metadata {
    $1_fungible_asset_Metadata(s->$name, s->$symbol, s->$decimals, s->$icon_uri, x)
}
function $IsValid'$1_fungible_asset_Metadata'(s: $1_fungible_asset_Metadata): bool {
    $IsValid'$1_string_String'(s->$name)
      && $IsValid'$1_string_String'(s->$symbol)
      && $IsValid'u8'(s->$decimals)
      && $IsValid'$1_string_String'(s->$icon_uri)
      && $IsValid'$1_string_String'(s->$project_uri)
}
function {:inline} $IsEqual'$1_fungible_asset_Metadata'(s1: $1_fungible_asset_Metadata, s2: $1_fungible_asset_Metadata): bool {
    $IsEqual'$1_string_String'(s1->$name, s2->$name)
    && $IsEqual'$1_string_String'(s1->$symbol, s2->$symbol)
    && $IsEqual'u8'(s1->$decimals, s2->$decimals)
    && $IsEqual'$1_string_String'(s1->$icon_uri, s2->$icon_uri)
    && $IsEqual'$1_string_String'(s1->$project_uri, s2->$project_uri)}
var $1_fungible_asset_Metadata_$memory: $Memory $1_fungible_asset_Metadata;

// struct fungible_asset::MintRef at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:129:5+73
datatype $1_fungible_asset_MintRef {
    $1_fungible_asset_MintRef($metadata: $1_object_Object'$1_fungible_asset_Metadata')
}
function {:inline} $Update'$1_fungible_asset_MintRef'_metadata(s: $1_fungible_asset_MintRef, x: $1_object_Object'$1_fungible_asset_Metadata'): $1_fungible_asset_MintRef {
    $1_fungible_asset_MintRef(x)
}
function $IsValid'$1_fungible_asset_MintRef'(s: $1_fungible_asset_MintRef): bool {
    $IsValid'$1_object_Object'$1_fungible_asset_Metadata''(s->$metadata)
}
function {:inline} $IsEqual'$1_fungible_asset_MintRef'(s1: $1_fungible_asset_MintRef, s2: $1_fungible_asset_MintRef): bool {
    s1 == s2
}

// struct fungible_asset::Supply at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:73:5+133
datatype $1_fungible_asset_Supply {
    $1_fungible_asset_Supply($current: int, $maximum: $1_option_Option'u128')
}
function {:inline} $Update'$1_fungible_asset_Supply'_current(s: $1_fungible_asset_Supply, x: int): $1_fungible_asset_Supply {
    $1_fungible_asset_Supply(x, s->$maximum)
}
function {:inline} $Update'$1_fungible_asset_Supply'_maximum(s: $1_fungible_asset_Supply, x: $1_option_Option'u128'): $1_fungible_asset_Supply {
    $1_fungible_asset_Supply(s->$current, x)
}
function $IsValid'$1_fungible_asset_Supply'(s: $1_fungible_asset_Supply): bool {
    $IsValid'u128'(s->$current)
      && $IsValid'$1_option_Option'u128''(s->$maximum)
}
function {:inline} $IsEqual'$1_fungible_asset_Supply'(s1: $1_fungible_asset_Supply, s2: $1_fungible_asset_Supply): bool {
    $IsEqual'u128'(s1->$current, s2->$current)
    && $IsEqual'$1_option_Option'u128''(s1->$maximum, s2->$maximum)}
var $1_fungible_asset_Supply_$memory: $Memory $1_fungible_asset_Supply;

// fun fungible_asset::extract [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:519:5+353
procedure {:timeLimit 40} $1_fungible_asset_extract$verify(_$t0: $Mutation ($1_fungible_asset_FungibleAsset), _$t1: int) returns ($ret0: $1_fungible_asset_FungibleAsset, $ret1: $Mutation ($1_fungible_asset_FungibleAsset))
{
    // declare local variables
    var $t2: int;
    var $t3: bool;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: $Mutation (int);
    var $t10: $1_object_Object'$1_fungible_asset_Metadata';
    var $t11: $1_fungible_asset_FungibleAsset;
    var $t0: $Mutation ($1_fungible_asset_FungibleAsset);
    var $t1: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();
    assume $t0->l == $Param(0);

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:519:5+1
    assume {:print "$at(2,21133,21134)"} true;
    assume $IsValid'$1_fungible_asset_FungibleAsset'($Dereference($t0));

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:519:5+1
    assume $IsValid'u64'($t1);

    // trace_local[fungible_asset]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:519:5+1
    $temp_0'$1_fungible_asset_FungibleAsset' := $Dereference($t0);
    assume {:print "$track_local(56,14,0):", $temp_0'$1_fungible_asset_FungibleAsset'} $temp_0'$1_fungible_asset_FungibleAsset' == $temp_0'$1_fungible_asset_FungibleAsset';

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:519:5+1
    assume {:print "$track_local(56,14,1):", $t1} $t1 == $t1;

    // $t2 := get_field<fungible_asset::FungibleAsset>.amount($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:17+21
    assume {:print "$at(2,21234,21255)"} true;
    $t2 := $Dereference($t0)->$amount;

    // $t3 := >=($t2, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:39+2
    call $t3 := $Ge($t2, $t1);

    // if ($t3) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
    if ($t3) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
    assume {:print "$at(2,21226,21314)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
L0:

    // destroy($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
    assume {:print "$at(2,21226,21314)"} true;

    // $t4 := 4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:74+21
    $t4 := 4;
    assume $IsValid'u64'($t4);

    // $t5 := error::invalid_argument($t4) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:50+46
    call $t5 := $1_error_invalid_argument($t4);
    if ($abort_flag) {
        assume {:print "$at(2,21267,21313)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,14):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
    assume {:print "$at(2,21226,21314)"} true;
    assume {:print "$track_abort(56,14):", $t5} $t5 == $t5;

    // $t6 := move($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
    $t6 := $t5;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:520:9+88
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:521:33+14
    assume {:print "$at(2,21348,21362)"} true;
L2:

    // $t7 := get_field<fungible_asset::FungibleAsset>.amount($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:521:33+21
    assume {:print "$at(2,21348,21369)"} true;
    $t7 := $Dereference($t0)->$amount;

    // $t8 := -($t7, $t1) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:521:55+1
    call $t8 := $Sub($t7, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,21370,21371)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,14):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t9 := borrow_field<fungible_asset::FungibleAsset>.amount($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:521:9+21
    $t9 := $ChildMutation($t0, 1, $Dereference($t0)->$amount);

    // write_ref($t9, $t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:521:9+54
    $t9 := $UpdateMutation($t9, $t8);

    // write_back[Reference($t0).amount (u64)]($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:521:9+54
    $t0 := $UpdateMutation($t0, $Update'$1_fungible_asset_FungibleAsset'_amount($Dereference($t0), $Dereference($t9)));

    // trace_local[fungible_asset]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:521:9+54
    $temp_0'$1_fungible_asset_FungibleAsset' := $Dereference($t0);
    assume {:print "$track_local(56,14,0):", $temp_0'$1_fungible_asset_FungibleAsset'} $temp_0'$1_fungible_asset_FungibleAsset' == $temp_0'$1_fungible_asset_FungibleAsset';

    // $t10 := get_field<fungible_asset::FungibleAsset>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:523:23+23
    assume {:print "$at(2,21426,21449)"} true;
    $t10 := $Dereference($t0)->$metadata;

    // $t11 := pack fungible_asset::FungibleAsset($t10, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:522:9+92
    assume {:print "$at(2,21388,21480)"} true;
    $t11 := $1_fungible_asset_FungibleAsset($t10, $t1);

    // trace_return[0]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:522:9+92
    assume {:print "$track_return(56,14,0):", $t11} $t11 == $t11;

    // trace_local[fungible_asset]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:522:9+92
    $temp_0'$1_fungible_asset_FungibleAsset' := $Dereference($t0);
    assume {:print "$track_local(56,14,0):", $temp_0'$1_fungible_asset_FungibleAsset'} $temp_0'$1_fungible_asset_FungibleAsset' == $temp_0'$1_fungible_asset_FungibleAsset';

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:526:5+1
    assume {:print "$at(2,21485,21486)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:526:5+1
    assume {:print "$at(2,21485,21486)"} true;
    assert {:msg "assert_failed(2,21485,21486): function does not abort under this condition"}
      !false;

    // return $t11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:526:5+1
    $ret0 := $t11;
    $ret1 := $t0;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:526:5+1
L4:

    // abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:526:5+1
    assume {:print "$at(2,21485,21486)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun fungible_asset::balance [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:311:5+231
procedure {:timeLimit 40} $1_fungible_asset_balance$verify(_$t0: $1_object_Object'#0') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t4: bool;
    var $t5: int;
    var $t6: $1_fungible_asset_FungibleStore;
    var $t7: int;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'u64': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:311:5+1
    assume {:print "$at(2,12948,12949)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:311:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:311:5+1
    assume {:print "$track_local(56,3,0):", $t0} $t0 == $t0;

    // $t2 := object::object_address<#0>($t0) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:312:26+30
    assume {:print "$at(2,13048,13078)"} true;
    call $t2 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,13048,13078)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,3):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t4 := fungible_asset::store_exists($t2) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:312:13+44
    call $t4 := $1_fungible_asset_store_exists($t2);
    if ($abort_flag) {
        assume {:print "$at(2,13035,13079)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,3):", $t3} $t3 == $t3;
        goto L4;
    }

    // if ($t4) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:312:9+142
    if ($t4) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:313:35+6
    assume {:print "$at(2,13117,13123)"} true;
L1:

    // $t5 := object::object_address<#0>($t0) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:38+29
    assume {:print "$at(2,26657,26686)"} true;
    call $t5 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26657,26686)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,3):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t6 := get_global<fungible_asset::FungibleStore>($t5) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:9+13
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t5)) {
        call $ExecFailureAbort();
    } else {
        $t6 := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t5);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26628,26641)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,3):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t1 := get_field<fungible_asset::FungibleStore>.balance($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:313:13+37
    assume {:print "$at(2,13095,13132)"} true;
    $t1 := $t6->$balance;

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:312:9+142
    assume {:print "$at(2,13031,13173)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:315:13+1
    assume {:print "$at(2,13162,13163)"} true;
L0:

    // $t7 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:315:13+1
    assume {:print "$at(2,13162,13163)"} true;
    $t7 := 0;
    assume $IsValid'u64'($t7);

    // $t1 := $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:312:9+142
    assume {:print "$at(2,13031,13173)"} true;
    $t1 := $t7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:312:9+142
L2:

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:312:9+142
    assume {:print "$at(2,13031,13173)"} true;
    assume {:print "$track_return(56,3,0):", $t1} $t1 == $t1;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:317:5+1
    assume {:print "$at(2,13178,13179)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:317:5+1
    assume {:print "$at(2,13178,13179)"} true;
    assert {:msg "assert_failed(2,13178,13179): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:317:5+1
    $ret0 := $t1;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:317:5+1
L4:

    // assert false at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:229:5+51
    assume {:print "$at(3,11949,12000)"} true;
    assert {:msg "assert_failed(3,11949,12000): abort not covered by any of the `aborts_if` clauses"}
      false;

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:229:5+51
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun fungible_asset::burn [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+328
procedure {:inline 1} $1_fungible_asset_burn(_$t0: $1_fungible_asset_BurnRef, _$t1: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: int;
    var $t6: $1_object_Object'$1_fungible_asset_Metadata';
    var $t7: bool;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t0: $1_fungible_asset_BurnRef;
    var $t1: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_BurnRef': $1_fungible_asset_BurnRef;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume {:print "$at(2,18546,18547)"} true;
    assume {:print "$track_local(56,4,0):", $t0} $t0 == $t0;

    // trace_local[fa]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume {:print "$track_local(56,4,1):", $t1} $t1 == $t1;

    // ($t4, $t5) := unpack fungible_asset::FungibleAsset($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:452:13+67
    assume {:print "$at(2,18644,18711)"} true;
    $t4 := $t1->$metadata;
    $t5 := $t1->$amount;

    // trace_local[amount]($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:454:13+6
    assume {:print "$at(2,18694,18700)"} true;
    assume {:print "$track_local(56,4,2):", $t5} $t5 == $t5;

    // trace_local[metadata]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:453:13+8
    assume {:print "$at(2,18672,18680)"} true;
    assume {:print "$track_local(56,4,3):", $t4} $t4 == $t4;

    // $t6 := get_field<fungible_asset::BurnRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:17+12
    assume {:print "$at(2,18734,18746)"} true;
    $t6 := $t0->$metadata;

    // $t7 := ==($t6, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:30+2
    $t7 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t6, $t4);

    // if ($t7) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    assume {:print "$at(2,18726,18823)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:67+37
L0:

    // $t8 := 13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:67+37
    assume {:print "$at(2,18784,18821)"} true;
    $t8 := 13;
    assume $IsValid'u64'($t8);

    // $t9 := error::invalid_argument($t8) on_abort goto L4 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:43+62
    call $t9 := $1_error_invalid_argument($t8);
    if ($abort_flag) {
        assume {:print "$at(2,18760,18822)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,4):", $t10} $t10 == $t10;
        goto L4;
    }

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    assume {:print "$at(2,18726,18823)"} true;
    assume {:print "$track_abort(56,4):", $t9} $t9 == $t9;

    // $t10 := move($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    $t10 := $t9;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:457:25+9
    assume {:print "$at(2,18849,18858)"} true;
L2:

    // fungible_asset::decrease_supply<fungible_asset::Metadata>($t4, $t5) on_abort goto L4 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:457:9+34
    assume {:print "$at(2,18833,18867)"} true;
    call $1_fungible_asset_decrease_supply'$1_fungible_asset_Metadata'($t4, $t5);
    if ($abort_flag) {
        assume {:print "$at(2,18833,18867)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,4):", $t10} $t10 == $t10;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
    assume {:print "$at(2,18873,18874)"} true;
L3:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
    assume {:print "$at(2,18873,18874)"} true;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
L4:

    // abort($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
    assume {:print "$at(2,18873,18874)"} true;
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun fungible_asset::burn [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+328
procedure {:timeLimit 40} $1_fungible_asset_burn$verify(_$t0: $1_fungible_asset_BurnRef, _$t1: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: int;
    var $t6: $1_object_Object'$1_fungible_asset_Metadata';
    var $t7: bool;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t0: $1_fungible_asset_BurnRef;
    var $t1: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_BurnRef': $1_fungible_asset_BurnRef;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume {:print "$at(2,18546,18547)"} true;
    assume $IsValid'$1_fungible_asset_BurnRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t1);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume {:print "$track_local(56,4,0):", $t0} $t0 == $t0;

    // trace_local[fa]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:451:5+1
    assume {:print "$track_local(56,4,1):", $t1} $t1 == $t1;

    // ($t4, $t5) := unpack fungible_asset::FungibleAsset($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:452:13+67
    assume {:print "$at(2,18644,18711)"} true;
    $t4 := $t1->$metadata;
    $t5 := $t1->$amount;

    // trace_local[amount]($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:454:13+6
    assume {:print "$at(2,18694,18700)"} true;
    assume {:print "$track_local(56,4,2):", $t5} $t5 == $t5;

    // trace_local[metadata]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:453:13+8
    assume {:print "$at(2,18672,18680)"} true;
    assume {:print "$track_local(56,4,3):", $t4} $t4 == $t4;

    // $t6 := get_field<fungible_asset::BurnRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:17+12
    assume {:print "$at(2,18734,18746)"} true;
    $t6 := $t0->$metadata;

    // $t7 := ==($t6, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:30+2
    $t7 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t6, $t4);

    // if ($t7) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    assume {:print "$at(2,18726,18823)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:67+37
L0:

    // $t8 := 13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:67+37
    assume {:print "$at(2,18784,18821)"} true;
    $t8 := 13;
    assume $IsValid'u64'($t8);

    // $t9 := error::invalid_argument($t8) on_abort goto L4 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:43+62
    call $t9 := $1_error_invalid_argument($t8);
    if ($abort_flag) {
        assume {:print "$at(2,18760,18822)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,4):", $t10} $t10 == $t10;
        goto L4;
    }

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    assume {:print "$at(2,18726,18823)"} true;
    assume {:print "$track_abort(56,4):", $t9} $t9 == $t9;

    // $t10 := move($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    $t10 := $t9;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:456:9+97
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:457:25+9
    assume {:print "$at(2,18849,18858)"} true;
L2:

    // fungible_asset::decrease_supply<fungible_asset::Metadata>($t4, $t5) on_abort goto L4 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:457:9+34
    assume {:print "$at(2,18833,18867)"} true;
    call $1_fungible_asset_decrease_supply'$1_fungible_asset_Metadata'($t4, $t5);
    if ($abort_flag) {
        assume {:print "$at(2,18833,18867)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,4):", $t10} $t10 == $t10;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
    assume {:print "$at(2,18873,18874)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
    assume {:print "$at(2,18873,18874)"} true;
    assert {:msg "assert_failed(2,18873,18874): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
L4:

    // abort($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:458:5+1
    assume {:print "$at(2,18873,18874)"} true;
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun fungible_asset::burn_from [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+441
procedure {:timeLimit 40} $1_fungible_asset_burn_from$verify(_$t0: $1_fungible_asset_BurnRef, _$t1: $1_object_Object'#0', _$t2: int) returns ()
{
    // declare local variables
    var $t3: int;
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: $1_object_Object'$1_fungible_asset_Metadata';
    var $t6: int;
    var $t7: bool;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $1_fungible_asset_FungibleStore;
    var $t12: $1_fungible_asset_FungibleAsset;
    var $t0: $1_fungible_asset_BurnRef;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_fungible_asset_BurnRef': $1_fungible_asset_BurnRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume {:print "$at(2,18950,18951)"} true;
    assume $IsValid'$1_fungible_asset_BurnRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume $IsValid'u64'($t2);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume {:print "$track_local(56,5,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume {:print "$track_local(56,5,1):", $t1} $t1 == $t1;

    // trace_local[amount]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:461:5+1
    assume {:print "$track_local(56,5,2):", $t2} $t2 == $t2;

    // $t4 := get_field<fungible_asset::BurnRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:466:24+12
    assume {:print "$at(2,19150,19162)"} true;
    $t4 := $t0->$metadata;

    // $t5 := fungible_asset::store_metadata<#0>($t1) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:29+21
    assume {:print "$at(2,19192,19213)"} true;
    call $t5 := $1_fungible_asset_store_metadata'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,19192,19213)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,5):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t7 := ==($t4, $t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:26+2
    $t7 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t4, $t5);

    // if ($t7) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:9+97
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:9+97
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:9+97
    assume {:print "$at(2,19172,19269)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:9+97
L0:

    // $t8 := 10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:76+28
    assume {:print "$at(2,19239,19267)"} true;
    $t8 := 10;
    assume $IsValid'u64'($t8);

    // $t9 := error::invalid_argument($t8) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:52+53
    call $t9 := $1_error_invalid_argument($t8);
    if ($abort_flag) {
        assume {:print "$at(2,19215,19268)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,5):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:9+97
    assume {:print "$at(2,19172,19269)"} true;
    assume {:print "$track_abort(56,5):", $t9} $t9 == $t9;

    // $t6 := move($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:9+97
    $t6 := $t9;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:467:9+97
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:468:49+6
    assume {:print "$at(2,19319,19325)"} true;
L2:

    // $t10 := object::object_address<#0>($t1) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:468:26+30
    assume {:print "$at(2,19296,19326)"} true;
    call $t10 := $1_object_object_address'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,19296,19326)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,5):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_local[store_addr]($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:468:13+10
    assume {:print "$track_local(56,5,3):", $t10} $t10 == $t10;

    // assume Identical($t11, global<fungible_asset::FungibleStore>($t10)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:431:9+46
    assume {:print "$at(3,17998,18044)"} true;
    assume ($t11 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t10));

    // $t12 := fungible_asset::withdraw_internal($t10, $t2) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:469:19+37
    assume {:print "$at(2,19346,19383)"} true;
    call $t12 := $1_fungible_asset_withdraw_internal($t10, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,19346,19383)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,5):", $t6} $t6 == $t6;
        goto L4;
    }

    // fungible_asset::burn($t0, $t12) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:469:9+48
    call $1_fungible_asset_burn($t0, $t12);
    if ($abort_flag) {
        assume {:print "$at(2,19336,19384)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,5):", $t6} $t6 == $t6;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:470:5+1
    assume {:print "$at(2,19390,19391)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:470:5+1
    assume {:print "$at(2,19390,19391)"} true;
    assert {:msg "assert_failed(2,19390,19391): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:470:5+1
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:470:5+1
L4:

    // abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:470:5+1
    assume {:print "$at(2,19390,19391)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun fungible_asset::amount [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:305:5+68
procedure {:timeLimit 40} $1_fungible_asset_amount$verify(_$t0: $1_fungible_asset_FungibleAsset) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:305:5+1
    assume {:print "$at(2,12820,12821)"} true;
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t0);

    // trace_local[fa]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:305:5+1
    assume {:print "$track_local(56,1,0):", $t0} $t0 == $t0;

    // $t1 := get_field<fungible_asset::FungibleAsset>.amount($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:306:9+9
    assume {:print "$at(2,12873,12882)"} true;
    $t1 := $t0->$amount;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:306:9+9
    assume {:print "$track_return(56,1,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:307:5+1
    assume {:print "$at(2,12887,12888)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:307:5+1
    assume {:print "$at(2,12887,12888)"} true;
    assert {:msg "assert_failed(2,12887,12888): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:307:5+1
    $ret0 := $t1;
    return;

}

// fun fungible_asset::decimals [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:283:5+131
procedure {:timeLimit 40} $1_fungible_asset_decimals$verify(_$t0: $1_object_Object'#0') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: $1_fungible_asset_Metadata;
    var $t4: int;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'u8': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:283:5+1
    assume {:print "$at(2,12091,12092)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::Metadata: ResourceDomain<fungible_asset::Metadata>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:283:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Metadata_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Metadata_$memory, $a_0);
    ($IsValid'$1_fungible_asset_Metadata'($rsc))));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:283:5+1
    assume {:print "$track_local(56,8,0):", $t0} $t0 == $t0;

    // $t1 := object::object_address<#0>($t0) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:627:20+32
    assume {:print "$at(2,26211,26243)"} true;
    call $t1 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26211,26243)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,8):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_global<fungible_asset::Metadata>($t1) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:628:9+13
    assume {:print "$at(2,26253,26266)"} true;
    if (!$ResourceExists($1_fungible_asset_Metadata_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $ResourceValue($1_fungible_asset_Metadata_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26253,26266)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,8):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t4 := get_field<fungible_asset::Metadata>.decimals($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:284:9+44
    assume {:print "$at(2,12172,12216)"} true;
    $t4 := $t3->$decimals;

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:284:9+44
    assume {:print "$track_return(56,8,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:285:5+1
    assume {:print "$at(2,12221,12222)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:285:5+1
    assume {:print "$at(2,12221,12222)"} true;
    assert {:msg "assert_failed(2,12221,12222): function does not abort under this condition"}
      !false;

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:285:5+1
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:285:5+1
L2:

    // abort($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:285:5+1
    assume {:print "$at(2,12221,12222)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun fungible_asset::deposit<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+231
procedure {:inline 1} $1_fungible_asset_deposit'#0'(_$t0: $1_object_Object'#0', _$t1: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $1_fungible_asset_ConcurrentSupply;
    var $t4: bool;
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: $1_object_Object'$1_fungible_asset_Metadata';
    var $t10: $1_object_Object'$1_fungible_asset_Metadata';
    var $t11: int;
    var $t12: $1_fungible_asset_FungibleStore;
    var $t13: int;
    var $t0: $1_object_Object'#0';
    var $t1: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t2, select object::Object.inner($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:326:9+34
    assume {:print "$at(3,15130,15164)"} true;
    assume ($t2 == $t0->$inner);

    // assume Identical($t3, global<fungible_asset::ConcurrentSupply>($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:327:9+55
    assume {:print "$at(3,15173,15228)"} true;
    assume ($t3 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t2));

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume {:print "$at(2,16903,16904)"} true;
    assume {:print "$track_local(56,10,0):", $t0} $t0 == $t0;

    // trace_local[fa]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume {:print "$track_local(56,10,1):", $t1} $t1 == $t1;

    // $t4 := fungible_asset::is_frozen<#0>($t0) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:18+16
    assume {:print "$at(2,17030,17046)"} true;
    call $t4 := $1_fungible_asset_is_frozen'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,17030,17046)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,10):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t6 := !($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:17+1
    call $t6 := $Not($t4);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    assume {:print "$at(2,17021,17090)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:60+16
L0:

    // $t7 := 3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:60+16
    assume {:print "$at(2,17072,17088)"} true;
    $t7 := 3;
    assume $IsValid'u64'($t7);

    // $t8 := error::invalid_argument($t7) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:36+41
    call $t8 := $1_error_invalid_argument($t7);
    if ($abort_flag) {
        assume {:print "$at(2,17048,17089)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,10):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    assume {:print "$at(2,17021,17090)"} true;
    assume {:print "$track_abort(56,10):", $t8} $t8 == $t8;

    // $t5 := move($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    $t5 := $t8;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:412:26+5
    assume {:print "$at(2,17117,17122)"} true;
L2:

    // assume Identical($t9, select fungible_asset::FungibleAsset.metadata($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:413:9+27
    assume {:print "$at(3,17355,17382)"} true;
    assume ($t9 == $t1->$metadata);

    // assume Identical($t10, fungible_asset::$store_metadata<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:414:9+43
    assume {:print "$at(3,17391,17434)"} true;
    assume ($t10 == $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory, $t0));

    // assume Identical($t11, object::$object_address<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:417:9+47
    assume {:print "$at(3,17539,17586)"} true;
    assume ($t11 == $1_object_$object_address'#0'($t0));

    // assume Identical($t12, global<fungible_asset::FungibleStore>($t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:418:9+46
    assume {:print "$at(3,17595,17641)"} true;
    assume ($t12 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t11));

    // assume Identical($t13, select fungible_asset::FungibleAsset.amount($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:419:9+23
    assume {:print "$at(3,17650,17673)"} true;
    assume ($t13 == $t1->$amount);

    // fungible_asset::deposit_internal<#0>($t0, $t1) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:412:9+27
    assume {:print "$at(2,17100,17127)"} true;
    call $1_fungible_asset_deposit_internal'#0'($t0, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,17100,17127)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,10):", $t5} $t5 == $t5;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:413:5+1
    assume {:print "$at(2,17133,17134)"} true;
L3:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:413:5+1
    assume {:print "$at(2,17133,17134)"} true;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:413:5+1
L4:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:413:5+1
    assume {:print "$at(2,17133,17134)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun fungible_asset::deposit [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+231
procedure {:timeLimit 40} $1_fungible_asset_deposit$verify(_$t0: $1_object_Object'#0', _$t1: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $1_fungible_asset_ConcurrentSupply;
    var $t4: bool;
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: $1_object_Object'$1_fungible_asset_Metadata';
    var $t10: $1_object_Object'$1_fungible_asset_Metadata';
    var $t11: int;
    var $t12: $1_fungible_asset_FungibleStore;
    var $t13: int;
    var $t14: $1_fungible_asset_ConcurrentSupply;
    var $t0: $1_object_Object'#0';
    var $t1: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $1_fungible_asset_FungibleStore_$memory#34: $Memory $1_fungible_asset_FungibleStore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume {:print "$at(2,16903,16904)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t1);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume Identical($t2, select object::Object.inner($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:326:9+34
    assume {:print "$at(3,15130,15164)"} true;
    assume ($t2 == $t0->$inner);

    // assume Identical($t3, global<fungible_asset::ConcurrentSupply>($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:327:9+55
    assume {:print "$at(3,15173,15228)"} true;
    assume ($t3 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t2));

    // @34 := save_mem(fungible_asset::FungibleStore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume {:print "$at(2,16903,16904)"} true;
    $1_fungible_asset_FungibleStore_$memory#34 := $1_fungible_asset_FungibleStore_$memory;

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume {:print "$track_local(56,10,0):", $t0} $t0 == $t0;

    // trace_local[fa]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:410:5+1
    assume {:print "$track_local(56,10,1):", $t1} $t1 == $t1;

    // $t4 := fungible_asset::is_frozen<#0>($t0) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:18+16
    assume {:print "$at(2,17030,17046)"} true;
    call $t4 := $1_fungible_asset_is_frozen'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,17030,17046)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,10):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t6 := !($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:17+1
    call $t6 := $Not($t4);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    assume {:print "$at(2,17021,17090)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:60+16
L0:

    // $t7 := 3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:60+16
    assume {:print "$at(2,17072,17088)"} true;
    $t7 := 3;
    assume $IsValid'u64'($t7);

    // $t8 := error::invalid_argument($t7) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:36+41
    call $t8 := $1_error_invalid_argument($t7);
    if ($abort_flag) {
        assume {:print "$at(2,17048,17089)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,10):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    assume {:print "$at(2,17021,17090)"} true;
    assume {:print "$track_abort(56,10):", $t8} $t8 == $t8;

    // $t5 := move($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    $t5 := $t8;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:411:9+69
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:412:26+5
    assume {:print "$at(2,17117,17122)"} true;
L2:

    // assume Identical($t9, select fungible_asset::FungibleAsset.metadata($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:413:9+27
    assume {:print "$at(3,17355,17382)"} true;
    assume ($t9 == $t1->$metadata);

    // assume Identical($t10, fungible_asset::$store_metadata<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:414:9+43
    assume {:print "$at(3,17391,17434)"} true;
    assume ($t10 == $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory, $t0));

    // assume Identical($t11, object::$object_address<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:417:9+47
    assume {:print "$at(3,17539,17586)"} true;
    assume ($t11 == $1_object_$object_address'#0'($t0));

    // assume Identical($t12, global<fungible_asset::FungibleStore>($t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:418:9+46
    assume {:print "$at(3,17595,17641)"} true;
    assume ($t12 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t11));

    // assume Identical($t13, select fungible_asset::FungibleAsset.amount($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:419:9+23
    assume {:print "$at(3,17650,17673)"} true;
    assume ($t13 == $t1->$amount);

    // fungible_asset::deposit_internal<#0>($t0, $t1) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:412:9+27
    assume {:print "$at(2,17100,17127)"} true;
    call $1_fungible_asset_deposit_internal'#0'($t0, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,17100,17127)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,10):", $t5} $t5 == $t5;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:413:5+1
    assume {:print "$at(2,17133,17134)"} true;
L3:

    // assume Identical($t14, global<fungible_asset::ConcurrentSupply>($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:328:9+65
    assume {:print "$at(3,15237,15302)"} true;
    assume ($t14 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t2));

    // assert Not(fungible_asset::$is_frozen[@34]<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:324:9+28
    assume {:print "$at(3,15092,15120)"} true;
    assert {:msg "assert_failed(3,15092,15120): function does not abort under this condition"}
      !$1_fungible_asset_$is_frozen'#0'($1_fungible_asset_FungibleStore_$memory#34, $t0);

    // assert Eq<fungible_asset::ConcurrentSupply>($t14, $t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:330:9+30
    assume {:print "$at(3,15344,15374)"} true;
    assert {:msg "assert_failed(3,15344,15374): post-condition does not hold"}
      $IsEqual'$1_fungible_asset_ConcurrentSupply'($t14, $t3);

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:330:9+30
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:413:5+1
    assume {:print "$at(2,17133,17134)"} true;
L4:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:413:5+1
    assume {:print "$at(2,17133,17134)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun fungible_asset::destroy_zero [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:537:5+206
procedure {:timeLimit 40} $1_fungible_asset_destroy_zero$verify(_$t0: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t0: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:537:5+1
    assume {:print "$at(2,22092,22093)"} true;
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t0);

    // trace_local[fungible_asset]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:537:5+1
    assume {:print "$track_local(56,13,0):", $t0} $t0 == $t0;

    // ($t2, $t3) := unpack fungible_asset::FungibleAsset($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:538:13+37
    assume {:print "$at(2,22161,22198)"} true;
    $t2 := $t0->$metadata;
    $t3 := $t0->$amount;

    // trace_local[amount]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:538:29+6
    assume {:print "$track_local(56,13,1):", $t3} $t3 == $t3;

    // destroy($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:538:47+1

    // $t4 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:27+1
    assume {:print "$at(2,22243,22244)"} true;
    $t4 := 0;
    assume $IsValid'u64'($t4);

    // $t5 := ==($t3, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:24+2
    $t5 := $IsEqual'u64'($t3, $t4);

    // if ($t5) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:9+66
    if ($t5) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:9+66
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:9+66
    assume {:print "$at(2,22225,22291)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:54+19
L0:

    // $t6 := 12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:54+19
    assume {:print "$at(2,22270,22289)"} true;
    $t6 := 12;
    assume $IsValid'u64'($t6);

    // $t7 := error::invalid_argument($t6) on_abort goto L4 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:30+44
    call $t7 := $1_error_invalid_argument($t6);
    if ($abort_flag) {
        assume {:print "$at(2,22246,22290)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,13):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:9+66
    assume {:print "$at(2,22225,22291)"} true;
    assume {:print "$track_abort(56,13):", $t7} $t7 == $t7;

    // $t8 := move($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:9+66
    $t8 := $t7;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:9+66
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:539:75+1
L2:

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:540:5+1
    assume {:print "$at(2,22297,22298)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:540:5+1
    assume {:print "$at(2,22297,22298)"} true;
    assert {:msg "assert_failed(2,22297,22298): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:540:5+1
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:540:5+1
L4:

    // abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:540:5+1
    assume {:print "$at(2,22297,22298)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun fungible_asset::name [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:271:5+127
procedure {:timeLimit 40} $1_fungible_asset_name$verify(_$t0: $1_object_Object'#0') returns ($ret0: $1_string_String)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: $1_fungible_asset_Metadata;
    var $t4: $1_string_String;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_string_String': $1_string_String;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:271:5+1
    assume {:print "$at(2,11671,11672)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::Metadata: ResourceDomain<fungible_asset::Metadata>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:271:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Metadata_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Metadata_$memory, $a_0);
    ($IsValid'$1_fungible_asset_Metadata'($rsc))));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:271:5+1
    assume {:print "$track_local(56,26,0):", $t0} $t0 == $t0;

    // $t1 := object::object_address<#0>($t0) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:627:20+32
    assume {:print "$at(2,26211,26243)"} true;
    call $t1 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26211,26243)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,26):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_global<fungible_asset::Metadata>($t1) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:628:9+13
    assume {:print "$at(2,26253,26266)"} true;
    if (!$ResourceExists($1_fungible_asset_Metadata_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $ResourceValue($1_fungible_asset_Metadata_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26253,26266)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,26):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t4 := get_field<fungible_asset::Metadata>.name($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:272:9+40
    assume {:print "$at(2,11752,11792)"} true;
    $t4 := $t3->$name;

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:272:9+40
    assume {:print "$track_return(56,26,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:273:5+1
    assume {:print "$at(2,11797,11798)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:273:5+1
    assume {:print "$at(2,11797,11798)"} true;
    assert {:msg "assert_failed(2,11797,11798): function does not abort under this condition"}
      !false;

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:273:5+1
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:273:5+1
L2:

    // abort($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:273:5+1
    assume {:print "$at(2,11797,11798)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun fungible_asset::symbol [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:277:5+131
procedure {:timeLimit 40} $1_fungible_asset_symbol$verify(_$t0: $1_object_Object'#0') returns ($ret0: $1_string_String)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: $1_fungible_asset_Metadata;
    var $t4: $1_string_String;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_string_String': $1_string_String;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:277:5+1
    assume {:print "$at(2,11889,11890)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::Metadata: ResourceDomain<fungible_asset::Metadata>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:277:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Metadata_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Metadata_$memory, $a_0);
    ($IsValid'$1_fungible_asset_Metadata'($rsc))));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:277:5+1
    assume {:print "$track_local(56,32,0):", $t0} $t0 == $t0;

    // $t1 := object::object_address<#0>($t0) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:627:20+32
    assume {:print "$at(2,26211,26243)"} true;
    call $t1 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26211,26243)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,32):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_global<fungible_asset::Metadata>($t1) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:628:9+13
    assume {:print "$at(2,26253,26266)"} true;
    if (!$ResourceExists($1_fungible_asset_Metadata_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $ResourceValue($1_fungible_asset_Metadata_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26253,26266)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,32):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t4 := get_field<fungible_asset::Metadata>.symbol($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:278:9+42
    assume {:print "$at(2,11972,12014)"} true;
    $t4 := $t3->$symbol;

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:278:9+42
    assume {:print "$track_return(56,32,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:279:5+1
    assume {:print "$at(2,12019,12020)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:279:5+1
    assume {:print "$at(2,12019,12020)"} true;
    assert {:msg "assert_failed(2,12019,12020): function does not abort under this condition"}
      !false;

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:279:5+1
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:279:5+1
L2:

    // abort($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:279:5+1
    assume {:print "$at(2,12019,12020)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun fungible_asset::merge [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:530:5+349
procedure {:timeLimit 40} $1_fungible_asset_merge$verify(_$t0: $Mutation ($1_fungible_asset_FungibleAsset), _$t1: $1_fungible_asset_FungibleAsset) returns ($ret0: $Mutation ($1_fungible_asset_FungibleAsset))
{
    // declare local variables
    var $t2: int;
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: int;
    var $t5: $1_object_Object'$1_fungible_asset_Metadata';
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: $Mutation (int);
    var $t0: $Mutation ($1_fungible_asset_FungibleAsset);
    var $t1: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();
    assume $t0->l == $Param(0);

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:530:5+1
    assume {:print "$at(2,21696,21697)"} true;
    assume $IsValid'$1_fungible_asset_FungibleAsset'($Dereference($t0));

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:530:5+1
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t1);

    // trace_local[dst_fungible_asset]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:530:5+1
    $temp_0'$1_fungible_asset_FungibleAsset' := $Dereference($t0);
    assume {:print "$track_local(56,21,0):", $temp_0'$1_fungible_asset_FungibleAsset'} $temp_0'$1_fungible_asset_FungibleAsset' == $temp_0'$1_fungible_asset_FungibleAsset';

    // trace_local[src_fungible_asset]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:530:5+1
    assume {:print "$track_local(56,21,1):", $t1} $t1 == $t1;

    // ($t3, $t4) := unpack fungible_asset::FungibleAsset($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:531:13+34
    assume {:print "$at(2,21802,21836)"} true;
    $t3 := $t1->$metadata;
    $t4 := $t1->$amount;

    // trace_local[amount]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:531:39+6
    assume {:print "$track_local(56,21,2):", $t4} $t4 == $t4;

    // $t5 := get_field<fungible_asset::FungibleAsset>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:29+27
    assume {:print "$at(2,21887,21914)"} true;
    $t5 := $Dereference($t0)->$metadata;

    // $t6 := ==($t3, $t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:26+2
    $t6 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t3, $t5);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
    assume {:print "$at(2,21867,21966)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
L0:

    // destroy($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
    assume {:print "$at(2,21867,21966)"} true;

    // $t7 := 6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:82+24
    $t7 := 6;
    assume $IsValid'u64'($t7);

    // $t8 := error::invalid_argument($t7) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:58+49
    call $t8 := $1_error_invalid_argument($t7);
    if ($abort_flag) {
        assume {:print "$at(2,21916,21965)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,21):", $t9} $t9 == $t9;
        goto L4;
    }

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
    assume {:print "$at(2,21867,21966)"} true;
    assume {:print "$track_abort(56,21):", $t8} $t8 == $t8;

    // $t9 := move($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
    $t9 := $t8;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:532:9+99
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:37+18
    assume {:print "$at(2,22004,22022)"} true;
L2:

    // $t10 := get_field<fungible_asset::FungibleAsset>.amount($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:37+25
    assume {:print "$at(2,22004,22029)"} true;
    $t10 := $Dereference($t0)->$amount;

    // $t11 := +($t10, $t4) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:63+1
    call $t11 := $AddU64($t10, $t4);
    if ($abort_flag) {
        assume {:print "$at(2,22030,22031)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,21):", $t9} $t9 == $t9;
        goto L4;
    }

    // $t12 := borrow_field<fungible_asset::FungibleAsset>.amount($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:9+25
    $t12 := $ChildMutation($t0, 1, $Dereference($t0)->$amount);

    // write_ref($t12, $t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:9+62
    $t12 := $UpdateMutation($t12, $t11);

    // write_back[Reference($t0).amount (u64)]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:9+62
    $t0 := $UpdateMutation($t0, $Update'$1_fungible_asset_FungibleAsset'_amount($Dereference($t0), $Dereference($t12)));

    // trace_local[dst_fungible_asset]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:9+62
    $temp_0'$1_fungible_asset_FungibleAsset' := $Dereference($t0);
    assume {:print "$track_local(56,21,0):", $temp_0'$1_fungible_asset_FungibleAsset'} $temp_0'$1_fungible_asset_FungibleAsset' == $temp_0'$1_fungible_asset_FungibleAsset';

    // trace_local[dst_fungible_asset]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:533:71+1
    $temp_0'$1_fungible_asset_FungibleAsset' := $Dereference($t0);
    assume {:print "$track_local(56,21,0):", $temp_0'$1_fungible_asset_FungibleAsset'} $temp_0'$1_fungible_asset_FungibleAsset' == $temp_0'$1_fungible_asset_FungibleAsset';

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:534:5+1
    assume {:print "$at(2,22044,22045)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:534:5+1
    assume {:print "$at(2,22044,22045)"} true;
    assert {:msg "assert_failed(2,22044,22045): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:534:5+1
    $ret0 := $t0;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:534:5+1
L4:

    // abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:534:5+1
    assume {:print "$at(2,22044,22045)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

}

// fun fungible_asset::mint [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+335
procedure {:inline 1} $1_fungible_asset_mint(_$t0: $1_fungible_asset_MintRef, _$t1: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t4: bool;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: $1_object_Object'$1_fungible_asset_Metadata';
    var $t9: int;
    var $t10: $1_fungible_asset_FungibleAsset;
    var $t0: $1_fungible_asset_MintRef;
    var $t1: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_MintRef': $1_fungible_asset_MintRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume {:print "$at(2,17199,17200)"} true;
    assume {:print "$track_local(56,23,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume {:print "$track_local(56,23,1):", $t1} $t1 == $t1;

    // $t3 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:26+1
    assume {:print "$at(2,17319,17320)"} true;
    $t3 := 0;
    assume $IsValid'u64'($t3);

    // $t4 := >($t1, $t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:24+1
    call $t4 := $Gt($t1, $t3);

    // if ($t4) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    if ($t4) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    assume {:print "$at(2,17302,17370)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
L0:

    // $t5 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:53+22
    assume {:print "$at(2,17346,17368)"} true;
    $t5 := 1;
    assume $IsValid'u64'($t5);

    // $t6 := error::invalid_argument($t5) on_abort goto L4 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:29+47
    call $t6 := $1_error_invalid_argument($t5);
    if ($abort_flag) {
        assume {:print "$at(2,17322,17369)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,23):", $t7} $t7 == $t7;
        goto L4;
    }

    // trace_abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    assume {:print "$at(2,17302,17370)"} true;
    assume {:print "$track_abort(56,23):", $t6} $t6 == $t6;

    // $t7 := move($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    $t7 := $t6;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:418:24+3
    assume {:print "$at(2,17395,17398)"} true;
L2:

    // $t8 := get_field<fungible_asset::MintRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:418:24+12
    assume {:print "$at(2,17395,17407)"} true;
    $t8 := $t0->$metadata;

    // trace_local[metadata]($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:418:13+8
    assume {:print "$track_local(56,23,2):", $t8} $t8 == $t8;

    // assume Identical($t9, object::$object_address<fungible_asset::Metadata>($t8)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:445:9+56
    assume {:print "$at(3,18524,18580)"} true;
    assume ($t9 == $1_object_$object_address'$1_fungible_asset_Metadata'($t8));

    // fungible_asset::increase_supply<fungible_asset::Metadata>($t8, $t1) on_abort goto L4 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:419:9+34
    assume {:print "$at(2,17417,17451)"} true;
    call $1_fungible_asset_increase_supply'$1_fungible_asset_Metadata'($t8, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,17417,17451)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,23):", $t7} $t7 == $t7;
        goto L4;
    }

    // $t10 := pack fungible_asset::FungibleAsset($t8, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:421:9+66
    assume {:print "$at(2,17462,17528)"} true;
    $t10 := $1_fungible_asset_FungibleAsset($t8, $t1);

    // trace_return[0]($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:421:9+66
    assume {:print "$track_return(56,23,0):", $t10} $t10 == $t10;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:425:5+1
    assume {:print "$at(2,17533,17534)"} true;
L3:

    // return $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:425:5+1
    assume {:print "$at(2,17533,17534)"} true;
    $ret0 := $t10;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:425:5+1
L4:

    // abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:425:5+1
    assume {:print "$at(2,17533,17534)"} true;
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun fungible_asset::mint [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+335
procedure {:timeLimit 40} $1_fungible_asset_mint$verify(_$t0: $1_fungible_asset_MintRef, _$t1: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t4: bool;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: $1_object_Object'$1_fungible_asset_Metadata';
    var $t9: int;
    var $t10: $1_fungible_asset_FungibleAsset;
    var $t0: $1_fungible_asset_MintRef;
    var $t1: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_MintRef': $1_fungible_asset_MintRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume {:print "$at(2,17199,17200)"} true;
    assume $IsValid'$1_fungible_asset_MintRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume $IsValid'u64'($t1);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume {:print "$track_local(56,23,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:416:5+1
    assume {:print "$track_local(56,23,1):", $t1} $t1 == $t1;

    // $t3 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:26+1
    assume {:print "$at(2,17319,17320)"} true;
    $t3 := 0;
    assume $IsValid'u64'($t3);

    // $t4 := >($t1, $t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:24+1
    call $t4 := $Gt($t1, $t3);

    // if ($t4) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    if ($t4) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    assume {:print "$at(2,17302,17370)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
L0:

    // $t5 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:53+22
    assume {:print "$at(2,17346,17368)"} true;
    $t5 := 1;
    assume $IsValid'u64'($t5);

    // $t6 := error::invalid_argument($t5) on_abort goto L4 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:29+47
    call $t6 := $1_error_invalid_argument($t5);
    if ($abort_flag) {
        assume {:print "$at(2,17322,17369)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,23):", $t7} $t7 == $t7;
        goto L4;
    }

    // trace_abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    assume {:print "$at(2,17302,17370)"} true;
    assume {:print "$track_abort(56,23):", $t6} $t6 == $t6;

    // $t7 := move($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    $t7 := $t6;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:417:9+68
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:418:24+3
    assume {:print "$at(2,17395,17398)"} true;
L2:

    // $t8 := get_field<fungible_asset::MintRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:418:24+12
    assume {:print "$at(2,17395,17407)"} true;
    $t8 := $t0->$metadata;

    // trace_local[metadata]($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:418:13+8
    assume {:print "$track_local(56,23,2):", $t8} $t8 == $t8;

    // assume Identical($t9, object::$object_address<fungible_asset::Metadata>($t8)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:445:9+56
    assume {:print "$at(3,18524,18580)"} true;
    assume ($t9 == $1_object_$object_address'$1_fungible_asset_Metadata'($t8));

    // fungible_asset::increase_supply<fungible_asset::Metadata>($t8, $t1) on_abort goto L4 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:419:9+34
    assume {:print "$at(2,17417,17451)"} true;
    call $1_fungible_asset_increase_supply'$1_fungible_asset_Metadata'($t8, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,17417,17451)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,23):", $t7} $t7 == $t7;
        goto L4;
    }

    // $t10 := pack fungible_asset::FungibleAsset($t8, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:421:9+66
    assume {:print "$at(2,17462,17528)"} true;
    $t10 := $1_fungible_asset_FungibleAsset($t8, $t1);

    // trace_return[0]($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:421:9+66
    assume {:print "$track_return(56,23,0):", $t10} $t10 == $t10;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:425:5+1
    assume {:print "$at(2,17533,17534)"} true;
L3:

    // assert Not(Eq<u64>($t1, 0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:336:9+22
    assume {:print "$at(3,15487,15509)"} true;
    assert {:msg "assert_failed(3,15487,15509): function does not abort under this condition"}
      !$IsEqual'u64'($t1, 0);

    // return $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:336:9+22
    $ret0 := $t10;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:425:5+1
    assume {:print "$at(2,17533,17534)"} true;
L4:

    // abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:425:5+1
    assume {:print "$at(2,17533,17534)"} true;
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun fungible_asset::supply [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:235:5+581
procedure {:timeLimit 40} $1_fungible_asset_supply$verify(_$t0: $1_object_Object'#0') returns ($ret0: $1_option_Option'u128')
{
    // declare local variables
    var $t1: $1_option_Option'u128';
    var $t2: $1_option_Option'u128';
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: bool;
    var $t7: $1_fungible_asset_ConcurrentSupply;
    var $t8: $1_aggregator_v2_Aggregator'u128';
    var $t9: int;
    var $t10: bool;
    var $t11: bool;
    var $t12: $1_fungible_asset_Supply;
    var $t13: int;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_option_Option'u128'': $1_option_Option'u128';
    var $temp_0'address': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:235:5+1
    assume {:print "$at(2,10120,10121)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:235:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:235:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:235:5+1
    assume {:print "$track_local(56,31,0):", $t0} $t0 == $t0;

    // $t4 := object::object_address<#0>($t0) on_abort goto L7 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:236:32+33
    assume {:print "$at(2,10248,10281)"} true;
    call $t4 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,10248,10281)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,31):", $t5} $t5 == $t5;
        goto L7;
    }

    // trace_local[metadata_address]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:236:13+16
    assume {:print "$track_local(56,31,3):", $t4} $t4 == $t4;

    // $t6 := exists<fungible_asset::ConcurrentSupply>($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:237:13+6
    assume {:print "$at(2,10295,10301)"} true;
    $t6 := $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t4);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:237:9+404
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:238:58+16
    assume {:print "$at(2,10398,10414)"} true;
L1:

    // $t7 := get_global<fungible_asset::ConcurrentSupply>($t4) on_abort goto L7 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:238:26+13
    assume {:print "$at(2,10366,10379)"} true;
    if (!$ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t4)) {
        call $ExecFailureAbort();
    } else {
        $t7 := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t4);
    }
    if ($abort_flag) {
        assume {:print "$at(2,10366,10379)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,31):", $t5} $t5 == $t5;
        goto L7;
    }

    // $t8 := get_field<fungible_asset::ConcurrentSupply>.current($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:46+15
    assume {:print "$at(2,10462,10477)"} true;
    $t8 := $t7->$current;

    // $t9 := opaque begin: aggregator_v2::read<u128>($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36

    // $t10 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36
    havoc $t10;

    // if ($t10) goto L9 else goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36
    if ($t10) { goto L9; } else { goto L8; }

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36
L9:

    // trace_abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36
    assume {:print "$at(2,10442,10478)"} true;
    assume {:print "$track_abort(56,31):", $t5} $t5 == $t5;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36
    goto L7;

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36
L8:

    // assume WellFormed($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36
    assume {:print "$at(2,10442,10478)"} true;
    assume $IsValid'u128'($t9);

    // $t9 := opaque end: aggregator_v2::read<u128>($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:26+36

    // $t2 := opaque begin: option::some<u128>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:13+50

    // assume And(WellFormed($t2), Le(Len<u128>(select option::Option.vec($t2)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:13+50
    assume ($IsValid'$1_option_Option'u128''($t2) && (LenVec($t2->$vec) <= 1));

    // assume Eq<option::Option<u128>>($t2, option::spec_some<u128>($t9)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:13+50
    assume $IsEqual'$1_option_Option'u128''($t2, $1_option_spec_some'u128'($t9));

    // $t2 := opaque end: option::some<u128>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:239:13+50

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:237:9+404
    assume {:print "$at(2,10291,10695)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:240:35+16
    assume {:print "$at(2,10514,10530)"} true;
L0:

    // $t11 := exists<fungible_asset::Supply>($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:240:20+6
    assume {:print "$at(2,10499,10505)"} true;
    $t11 := $ResourceExists($1_fungible_asset_Supply_$memory, $t4);

    // if ($t11) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:240:16+200
    if ($t11) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:241:48+16
    assume {:print "$at(2,10582,10598)"} true;
L4:

    // $t12 := get_global<fungible_asset::Supply>($t4) on_abort goto L7 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:241:26+13
    assume {:print "$at(2,10560,10573)"} true;
    if (!$ResourceExists($1_fungible_asset_Supply_$memory, $t4)) {
        call $ExecFailureAbort();
    } else {
        $t12 := $ResourceValue($1_fungible_asset_Supply_$memory, $t4);
    }
    if ($abort_flag) {
        assume {:print "$at(2,10560,10573)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,31):", $t5} $t5 == $t5;
        goto L7;
    }

    // $t13 := get_field<fungible_asset::Supply>.current($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:242:26+14
    assume {:print "$at(2,10626,10640)"} true;
    $t13 := $t12->$current;

    // $t1 := opaque begin: option::some<u128>($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:242:13+28

    // assume And(WellFormed($t1), Le(Len<u128>(select option::Option.vec($t1)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:242:13+28
    assume ($IsValid'$1_option_Option'u128''($t1) && (LenVec($t1->$vec) <= 1));

    // assume Eq<option::Option<u128>>($t1, option::spec_some<u128>($t13)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:242:13+28
    assume $IsEqual'$1_option_Option'u128''($t1, $1_option_spec_some'u128'($t13));

    // $t1 := opaque end: option::some<u128>($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:242:13+28

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:240:16+200
    assume {:print "$at(2,10495,10695)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:244:13+14
    assume {:print "$at(2,10671,10685)"} true;
L3:

    // $t1 := opaque begin: option::none<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:244:13+14
    assume {:print "$at(2,10671,10685)"} true;

    // assume And(WellFormed($t1), Le(Len<u128>(select option::Option.vec($t1)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:244:13+14
    assume ($IsValid'$1_option_Option'u128''($t1) && (LenVec($t1->$vec) <= 1));

    // assume Eq<option::Option<u128>>($t1, option::spec_none<u128>()) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:244:13+14
    assume $IsEqual'$1_option_Option'u128''($t1, $1_option_spec_none'u128'());

    // $t1 := opaque end: option::none<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:244:13+14

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:240:16+200
    assume {:print "$at(2,10495,10695)"} true;
L5:

    // $t2 := $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:237:9+404
    assume {:print "$at(2,10291,10695)"} true;
    $t2 := $t1;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:237:9+404
L2:

    // trace_return[0]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:237:9+404
    assume {:print "$at(2,10291,10695)"} true;
    assume {:print "$track_return(56,31,0):", $t2} $t2 == $t2;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:246:5+1
    assume {:print "$at(2,10700,10701)"} true;
L6:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:246:5+1
    assume {:print "$at(2,10700,10701)"} true;
    assert {:msg "assert_failed(2,10700,10701): function does not abort under this condition"}
      !false;

    // return $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:246:5+1
    $ret0 := $t2;
    return;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:246:5+1
L7:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:246:5+1
    assume {:print "$at(2,10700,10701)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun fungible_asset::transfer [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+260
procedure {:timeLimit 40} $1_fungible_asset_transfer$verify(_$t0: $signer, _$t1: $1_object_Object'#0', _$t2: $1_object_Object'#0', _$t3: int) returns ()
{
    // declare local variables
    var $t4: $1_fungible_asset_FungibleAsset;
    var $t5: int;
    var $t6: $1_fungible_asset_ConcurrentSupply;
    var $t7: int;
    var $t8: $1_object_ObjectCore;
    var $t9: int;
    var $t10: $1_fungible_asset_FungibleStore;
    var $t11: $1_fungible_asset_ConcurrentSupply;
    var $t12: $1_fungible_asset_FungibleAsset;
    var $t13: int;
    var $t14: int;
    var $t15: $1_fungible_asset_ConcurrentSupply;
    var $t16: $1_fungible_asset_ConcurrentSupply;
    var $t0: $signer;
    var $t1: $1_object_Object'#0';
    var $t2: $1_object_Object'#0';
    var $t3: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume {:print "$at(2,14289,14290)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($t0->$addr);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume $IsValid'$1_object_Object'#0''($t2);

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume $IsValid'u64'($t3);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume Identical($t5, select object::Object.inner($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:255:9+27
    assume {:print "$at(3,12527,12554)"} true;
    assume ($t5 == $t1->$inner);

    // assume Identical($t6, global<fungible_asset::ConcurrentSupply>($t5)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:256:9+49
    assume {:print "$at(3,12563,12612)"} true;
    assume ($t6 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t5));

    // trace_local[sender]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume {:print "$at(2,14289,14290)"} true;
    assume {:print "$track_local(56,33,0):", $t0} $t0 == $t0;

    // trace_local[from]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume {:print "$track_local(56,33,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume {:print "$track_local(56,33,2):", $t2} $t2 == $t2;

    // trace_local[amount]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:348:5+1
    assume {:print "$track_local(56,33,3):", $t3} $t3 == $t3;

    // assume Identical($t7, select object::Object.inner($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:299:9+36
    assume {:print "$at(3,14080,14116)"} true;
    assume ($t7 == $t1->$inner);

    // assume Identical($t8, global<object::ObjectCore>($t7)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:300:9+61
    assume {:print "$at(3,14125,14186)"} true;
    assume ($t8 == $ResourceValue($1_object_ObjectCore_$memory, $t7));

    // assume Identical($t9, select object::ObjectCore.owner($t8)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:301:9+37
    assume {:print "$at(3,14195,14232)"} true;
    assume ($t9 == $t8->$owner);

    // assume Identical($t10, global<fungible_asset::FungibleStore>($t7)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:311:9+62
    assume {:print "$at(3,14611,14673)"} true;
    assume ($t10 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t7));

    // assume Identical($t11, global<fungible_asset::ConcurrentSupply>($t7)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:314:9+57
    assume {:print "$at(3,14736,14793)"} true;
    assume ($t11 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t7));

    // $t12 := fungible_asset::withdraw<#0>($t0, $t1, $t3) on_abort goto L2 with $t13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:354:18+30
    assume {:print "$at(2,14487,14517)"} true;
    call $t12 := $1_fungible_asset_withdraw'#0'($t0, $t1, $t3);
    if ($abort_flag) {
        assume {:print "$at(2,14487,14517)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(56,33):", $t13} $t13 == $t13;
        goto L2;
    }

    // trace_local[fa]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:354:13+2
    assume {:print "$track_local(56,33,4):", $t12} $t12 == $t12;

    // assume Identical($t14, select object::Object.inner($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:326:9+34
    assume {:print "$at(3,15130,15164)"} true;
    assume ($t14 == $t2->$inner);

    // assume Identical($t15, global<fungible_asset::ConcurrentSupply>($t14)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:327:9+55
    assume {:print "$at(3,15173,15228)"} true;
    assume ($t15 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t14));

    // fungible_asset::deposit<#0>($t2, $t12) on_abort goto L2 with $t13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:355:9+15
    assume {:print "$at(2,14527,14542)"} true;
    call $1_fungible_asset_deposit'#0'($t2, $t12);
    if ($abort_flag) {
        assume {:print "$at(2,14527,14542)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(56,33):", $t13} $t13 == $t13;
        goto L2;
    }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:356:5+1
    assume {:print "$at(2,14548,14549)"} true;
L1:

    // assume Identical($t16, global<fungible_asset::ConcurrentSupply>($t5)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:257:9+59
    assume {:print "$at(3,12621,12680)"} true;
    assume ($t16 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t5));

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:356:5+1
    assume {:print "$at(2,14548,14549)"} true;
    assert {:msg "assert_failed(2,14548,14549): function does not abort under this condition"}
      !false;

    // assert Eq<fungible_asset::ConcurrentSupply>($t16, $t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:259:9+30
    assume {:print "$at(3,12722,12752)"} true;
    assert {:msg "assert_failed(3,12722,12752): post-condition does not hold"}
      $IsEqual'$1_fungible_asset_ConcurrentSupply'($t16, $t6);

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:259:9+30
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:356:5+1
    assume {:print "$at(2,14548,14549)"} true;
L2:

    // abort($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:356:5+1
    assume {:print "$at(2,14548,14549)"} true;
    $abort_code := $t13;
    $abort_flag := true;
    return;

}

// fun fungible_asset::withdraw<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+426
procedure {:inline 1} $1_fungible_asset_withdraw'#0'(_$t0: $signer, _$t1: $1_object_Object'#0', _$t2: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t3: int;
    var $t4: $1_object_ObjectCore;
    var $t5: int;
    var $t6: $1_fungible_asset_FungibleStore;
    var $t7: $1_fungible_asset_ConcurrentSupply;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $1_object_ObjectCore;
    var $t12: int;
    var $t13: bool;
    var $t14: int;
    var $t15: int;
    var $t16: bool;
    var $t17: bool;
    var $t18: int;
    var $t19: int;
    var $t20: int;
    var $t21: $1_fungible_asset_FungibleStore;
    var $t22: $1_fungible_asset_FungibleAsset;
    var $t0: $signer;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // assume Identical($t3, select object::Object.inner($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:299:9+36
    assume {:print "$at(3,14080,14116)"} true;
    assume ($t3 == $t1->$inner);

    // assume Identical($t4, global<object::ObjectCore>($t3)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:300:9+61
    assume {:print "$at(3,14125,14186)"} true;
    assume ($t4 == $ResourceValue($1_object_ObjectCore_$memory, $t3));

    // assume Identical($t5, select object::ObjectCore.owner($t4)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:301:9+37
    assume {:print "$at(3,14195,14232)"} true;
    assume ($t5 == $t4->$owner);

    // assume Identical($t6, global<fungible_asset::FungibleStore>($t3)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:311:9+62
    assume {:print "$at(3,14611,14673)"} true;
    assume ($t6 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t3));

    // assume Identical($t7, global<fungible_asset::ConcurrentSupply>($t3)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:314:9+57
    assume {:print "$at(3,14736,14793)"} true;
    assume ($t7 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t3));

    // trace_local[owner]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$at(2,16412,16413)"} true;
    assume {:print "$track_local(56,37,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$track_local(56,37,1):", $t1} $t1 == $t1;

    // trace_local[amount]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$track_local(56,37,2):", $t2} $t2 == $t2;

    // $t8 := signer::address_of($t0) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:37+25
    assume {:print "$at(2,16615,16640)"} true;
    call $t8 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,16615,16640)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // assume Identical($t10, select object::Object.inner($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:507:9+37
    assume {:print "$at(136,20290,20327)"} true;
    assume ($t10 == $t1->$inner);

    // assume Identical($t11, global<object::ObjectCore>($t10)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:508:9+53
    assume {:print "$at(136,20336,20389)"} true;
    assume ($t11 == $ResourceValue($1_object_ObjectCore_$memory, $t10));

    // assume Identical($t12, select object::ObjectCore.owner($t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:509:9+37
    assume {:print "$at(136,20398,20435)"} true;
    assume ($t12 == $t11->$owner);

    // $t13 := object::owns<#0>($t1, $t8) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:17+46
    assume {:print "$at(2,16595,16641)"} true;
    call $t13 := $1_object_owns'#0'($t1, $t8);
    if ($abort_flag) {
        assume {:print "$at(2,16595,16641)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // if ($t13) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    if ($t13) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    assume {:print "$at(2,16587,16686)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:90+16
L0:

    // $t14 := 8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:90+16
    assume {:print "$at(2,16668,16684)"} true;
    $t14 := 8;
    assume $IsValid'u64'($t14);

    // $t15 := error::permission_denied($t14) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:65+42
    call $t15 := $1_error_permission_denied($t14);
    if ($abort_flag) {
        assume {:print "$at(2,16643,16685)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // trace_abort($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    assume {:print "$at(2,16587,16686)"} true;
    assume {:print "$track_abort(56,37):", $t15} $t15 == $t15;

    // $t9 := move($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    $t9 := $t15;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    goto L7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:28+5
    assume {:print "$at(2,16715,16720)"} true;
L2:

    // $t16 := fungible_asset::is_frozen<#0>($t1) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:18+16
    assume {:print "$at(2,16705,16721)"} true;
    call $t16 := $1_fungible_asset_is_frozen'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,16705,16721)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // $t17 := !($t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:17+1
    call $t17 := $Not($t16);

    // if ($t17) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    if ($t17) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    assume {:print "$at(2,16696,16765)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:60+16
L3:

    // $t18 := 3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:60+16
    assume {:print "$at(2,16747,16763)"} true;
    $t18 := 3;
    assume $IsValid'u64'($t18);

    // $t19 := error::invalid_argument($t18) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:36+41
    call $t19 := $1_error_invalid_argument($t18);
    if ($abort_flag) {
        assume {:print "$at(2,16723,16764)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // trace_abort($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    assume {:print "$at(2,16696,16765)"} true;
    assume {:print "$track_abort(56,37):", $t19} $t19 == $t19;

    // $t9 := move($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    $t9 := $t19;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:50+6
    assume {:print "$at(2,16816,16822)"} true;
L5:

    // $t20 := object::object_address<#0>($t1) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:27+30
    assume {:print "$at(2,16793,16823)"} true;
    call $t20 := $1_object_object_address'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,16793,16823)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // assume Identical($t21, global<fungible_asset::FungibleStore>($t20)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:431:9+46
    assume {:print "$at(3,17998,18044)"} true;
    assume ($t21 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t20));

    // $t22 := fungible_asset::withdraw_internal($t20, $t2) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:9+57
    assume {:print "$at(2,16775,16832)"} true;
    call $t22 := $1_fungible_asset_withdraw_internal($t20, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,16775,16832)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // trace_return[0]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:9+57
    assume {:print "$track_return(56,37,0):", $t22} $t22 == $t22;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:407:5+1
    assume {:print "$at(2,16837,16838)"} true;
L6:

    // return $t22 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:407:5+1
    assume {:print "$at(2,16837,16838)"} true;
    $ret0 := $t22;
    return;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:407:5+1
L7:

    // abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:407:5+1
    assume {:print "$at(2,16837,16838)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

}

// fun fungible_asset::withdraw [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+426
procedure {:timeLimit 40} $1_fungible_asset_withdraw$verify(_$t0: $signer, _$t1: $1_object_Object'#0', _$t2: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t3: int;
    var $t4: $1_object_ObjectCore;
    var $t5: int;
    var $t6: $1_fungible_asset_FungibleStore;
    var $t7: $1_fungible_asset_ConcurrentSupply;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $1_object_ObjectCore;
    var $t12: int;
    var $t13: bool;
    var $t14: int;
    var $t15: int;
    var $t16: bool;
    var $t17: bool;
    var $t18: int;
    var $t19: int;
    var $t20: int;
    var $t21: $1_fungible_asset_FungibleStore;
    var $t22: $1_fungible_asset_FungibleAsset;
    var $t23: $1_fungible_asset_ConcurrentSupply;
    var $t0: $signer;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $1_object_ObjectCore_$memory#26: $Memory $1_object_ObjectCore;
    var $1_fungible_asset_FungibleStore_$memory#27: $Memory $1_fungible_asset_FungibleStore;
    var $1_fungible_asset_FungibleAssetEvents_$memory#28: $Memory $1_fungible_asset_FungibleAssetEvents;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$at(2,16412,16413)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($t0->$addr);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume $IsValid'u64'($t2);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume Identical($t3, select object::Object.inner($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:299:9+36
    assume {:print "$at(3,14080,14116)"} true;
    assume ($t3 == $t1->$inner);

    // assume Identical($t4, global<object::ObjectCore>($t3)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:300:9+61
    assume {:print "$at(3,14125,14186)"} true;
    assume ($t4 == $ResourceValue($1_object_ObjectCore_$memory, $t3));

    // assume Identical($t5, select object::ObjectCore.owner($t4)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:301:9+37
    assume {:print "$at(3,14195,14232)"} true;
    assume ($t5 == $t4->$owner);

    // assume Identical($t6, global<fungible_asset::FungibleStore>($t3)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:311:9+62
    assume {:print "$at(3,14611,14673)"} true;
    assume ($t6 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t3));

    // assume Identical($t7, global<fungible_asset::ConcurrentSupply>($t3)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:314:9+57
    assume {:print "$at(3,14736,14793)"} true;
    assume ($t7 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t3));

    // @26 := save_mem(object::ObjectCore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$at(2,16412,16413)"} true;
    $1_object_ObjectCore_$memory#26 := $1_object_ObjectCore_$memory;

    // @28 := save_mem(fungible_asset::FungibleAssetEvents) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    $1_fungible_asset_FungibleAssetEvents_$memory#28 := $1_fungible_asset_FungibleAssetEvents_$memory;

    // @27 := save_mem(fungible_asset::FungibleStore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    $1_fungible_asset_FungibleStore_$memory#27 := $1_fungible_asset_FungibleStore_$memory;

    // trace_local[owner]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$track_local(56,37,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$track_local(56,37,1):", $t1} $t1 == $t1;

    // trace_local[amount]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:399:5+1
    assume {:print "$track_local(56,37,2):", $t2} $t2 == $t2;

    // $t8 := signer::address_of($t0) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:37+25
    assume {:print "$at(2,16615,16640)"} true;
    call $t8 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,16615,16640)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // assume Identical($t10, select object::Object.inner($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:507:9+37
    assume {:print "$at(136,20290,20327)"} true;
    assume ($t10 == $t1->$inner);

    // assume Identical($t11, global<object::ObjectCore>($t10)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:508:9+53
    assume {:print "$at(136,20336,20389)"} true;
    assume ($t11 == $ResourceValue($1_object_ObjectCore_$memory, $t10));

    // assume Identical($t12, select object::ObjectCore.owner($t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:509:9+37
    assume {:print "$at(136,20398,20435)"} true;
    assume ($t12 == $t11->$owner);

    // $t13 := object::owns<#0>($t1, $t8) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:17+46
    assume {:print "$at(2,16595,16641)"} true;
    call $t13 := $1_object_owns'#0'($t1, $t8);
    if ($abort_flag) {
        assume {:print "$at(2,16595,16641)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // if ($t13) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    if ($t13) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    assume {:print "$at(2,16587,16686)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:90+16
L0:

    // $t14 := 8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:90+16
    assume {:print "$at(2,16668,16684)"} true;
    $t14 := 8;
    assume $IsValid'u64'($t14);

    // $t15 := error::permission_denied($t14) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:65+42
    call $t15 := $1_error_permission_denied($t14);
    if ($abort_flag) {
        assume {:print "$at(2,16643,16685)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // trace_abort($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    assume {:print "$at(2,16587,16686)"} true;
    assume {:print "$track_abort(56,37):", $t15} $t15 == $t15;

    // $t9 := move($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    $t9 := $t15;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:404:9+99
    goto L7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:28+5
    assume {:print "$at(2,16715,16720)"} true;
L2:

    // $t16 := fungible_asset::is_frozen<#0>($t1) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:18+16
    assume {:print "$at(2,16705,16721)"} true;
    call $t16 := $1_fungible_asset_is_frozen'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,16705,16721)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // $t17 := !($t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:17+1
    call $t17 := $Not($t16);

    // if ($t17) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    if ($t17) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    assume {:print "$at(2,16696,16765)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:60+16
L3:

    // $t18 := 3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:60+16
    assume {:print "$at(2,16747,16763)"} true;
    $t18 := 3;
    assume $IsValid'u64'($t18);

    // $t19 := error::invalid_argument($t18) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:36+41
    call $t19 := $1_error_invalid_argument($t18);
    if ($abort_flag) {
        assume {:print "$at(2,16723,16764)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // trace_abort($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    assume {:print "$at(2,16696,16765)"} true;
    assume {:print "$track_abort(56,37):", $t19} $t19 == $t19;

    // $t9 := move($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    $t9 := $t19;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:405:9+69
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:50+6
    assume {:print "$at(2,16816,16822)"} true;
L5:

    // $t20 := object::object_address<#0>($t1) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:27+30
    assume {:print "$at(2,16793,16823)"} true;
    call $t20 := $1_object_object_address'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,16793,16823)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // assume Identical($t21, global<fungible_asset::FungibleStore>($t20)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:431:9+46
    assume {:print "$at(3,17998,18044)"} true;
    assume ($t21 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t20));

    // $t22 := fungible_asset::withdraw_internal($t20, $t2) on_abort goto L7 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:9+57
    assume {:print "$at(2,16775,16832)"} true;
    call $t22 := $1_fungible_asset_withdraw_internal($t20, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,16775,16832)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,37):", $t9} $t9 == $t9;
        goto L7;
    }

    // trace_return[0]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:406:9+57
    assume {:print "$track_return(56,37,0):", $t22} $t22 == $t22;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:407:5+1
    assume {:print "$at(2,16837,16838)"} true;
L6:

    // assume Identical($t23, global<fungible_asset::ConcurrentSupply>($t3)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:315:9+67
    assume {:print "$at(3,14802,14869)"} true;
    assume ($t23 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t3));

    // assert Not(And(Neq<address>(select object::Object.inner($t1), signer::$address_of[]($t0)), Not(exists[@26]<object::ObjectCore>(select object::Object.inner($t1))))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:303:9+95
    assume {:print "$at(3,14272,14367)"} true;
    assert {:msg "assert_failed(3,14272,14367): function does not abort under this condition"}
      !(!$IsEqual'address'($t1->$inner, $1_signer_$address_of($t0)) && !$ResourceExists($1_object_ObjectCore_$memory#26, $t1->$inner));

    // assert Not(Not(exists[@27]<fungible_asset::FungibleStore>($t3))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:304:9+52
    assume {:print "$at(3,14376,14428)"} true;
    assert {:msg "assert_failed(3,14376,14428): function does not abort under this condition"}
      !!$ResourceExists($1_fungible_asset_FungibleStore_$memory#27, $t3);

    // assert Not(Not(exists[@28]<fungible_asset::FungibleAssetEvents>($t3))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:305:9+58
    assume {:print "$at(3,14437,14495)"} true;
    assert {:msg "assert_failed(3,14437,14495): function does not abort under this condition"}
      !!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory#28, $t3);

    // assert Not(Eq<u64>($t2, 0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:307:9+24
    assume {:print "$at(3,14505,14529)"} true;
    assert {:msg "assert_failed(3,14505,14529): function does not abort under this condition"}
      !$IsEqual'u64'($t2, 0);

    // assert Not(fungible_asset::$is_frozen[@27]<#0>($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:309:9+29
    assume {:print "$at(3,14572,14601)"} true;
    assert {:msg "assert_failed(3,14572,14601): function does not abort under this condition"}
      !$1_fungible_asset_$is_frozen'#0'($1_fungible_asset_FungibleStore_$memory#27, $t1);

    // assert Not(Lt(select fungible_asset::FungibleStore.balance($t6), $t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:312:9+44
    assume {:print "$at(3,14682,14726)"} true;
    assert {:msg "assert_failed(3,14682,14726): function does not abort under this condition"}
      !($t6->$balance < $t2);

    // assert Eq<fungible_asset::ConcurrentSupply>($t23, $t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:317:9+30
    assume {:print "$at(3,14911,14941)"} true;
    assert {:msg "assert_failed(3,14911,14941): post-condition does not hold"}
      $IsEqual'$1_fungible_asset_ConcurrentSupply'($t23, $t7);

    // return $t22 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:317:9+30
    $ret0 := $t22;
    return;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:407:5+1
    assume {:print "$at(2,16837,16838)"} true;
L7:

    // abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:407:5+1
    assume {:print "$at(2,16837,16838)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

}

// fun fungible_asset::zero [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:511:5+173
procedure {:timeLimit 40} $1_fungible_asset_zero$verify(_$t0: $1_object_Object'#0') returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t1: $1_object_Object'$1_fungible_asset_Metadata';
    var $t2: int;
    var $t3: int;
    var $t4: $1_fungible_asset_FungibleAsset;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:511:5+1
    assume {:print "$at(2,20871,20872)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:511:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:511:5+1
    assume {:print "$track_local(56,40,0):", $t0} $t0 == $t0;

    // $t1 := object::convert<#0, fungible_asset::Metadata>($t0) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:513:23+25
    assume {:print "$at(2,20979,21004)"} true;
    call $t1 := $1_object_convert'#0_$1_fungible_asset_Metadata'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,20979,21004)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,40):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:514:21+1
    assume {:print "$at(2,21026,21027)"} true;
    $t3 := 0;
    assume $IsValid'u64'($t3);

    // $t4 := pack fungible_asset::FungibleAsset($t1, $t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:512:9+97
    assume {:print "$at(2,20941,21038)"} true;
    $t4 := $1_fungible_asset_FungibleAsset($t1, $t3);

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:512:9+97
    assume {:print "$track_return(56,40,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:516:5+1
    assume {:print "$at(2,21043,21044)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:516:5+1
    assume {:print "$at(2,21043,21044)"} true;
    assert {:msg "assert_failed(2,21043,21044): function does not abort under this condition"}
      !false;

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:516:5+1
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:516:5+1
L2:

    // abort($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:516:5+1
    assume {:print "$at(2,21043,21044)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun fungible_asset::generate_transfer_ref [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:228:5+207
procedure {:timeLimit 40} $1_fungible_asset_generate_transfer_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_fungible_asset_TransferRef)
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t4: $1_fungible_asset_TransferRef;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $1_object_ObjectCore_$memory#44: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:228:5+1
    assume {:print "$at(2,9836,9837)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:228:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume Identical($t1, object::$address_from_constructor_ref($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:191:9+80
    assume {:print "$at(3,10954,11034)"} true;
    assume ($t1 == $1_object_$address_from_constructor_ref($t0));

    // @44 := save_mem(object::ObjectCore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:228:5+1
    assume {:print "$at(2,9836,9837)"} true;
    $1_object_ObjectCore_$memory#44 := $1_object_ObjectCore_$memory;

    // trace_local[constructor_ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:228:5+1
    assume {:print "$track_local(56,17,0):", $t0} $t0 == $t0;

    // $t2 := object::object_from_constructor_ref<fungible_asset::Metadata>($t0) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:229:24+62
    assume {:print "$at(2,9941,10003)"} true;
    call $t2 := $1_object_object_from_constructor_ref'$1_fungible_asset_Metadata'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,9941,10003)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,17):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t4 := pack fungible_asset::TransferRef($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:230:9+24
    assume {:print "$at(2,10013,10037)"} true;
    $t4 := $1_fungible_asset_TransferRef($t2);

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:230:9+24
    assume {:print "$track_return(56,17,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:231:5+1
    assume {:print "$at(2,10042,10043)"} true;
L1:

    // assert Not(Not(exists[@44]<object::ObjectCore>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:192:9+59
    assume {:print "$at(3,11043,11102)"} true;
    assert {:msg "assert_failed(3,11043,11102): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#44, $t1);

    // assert Not(Not(object::spec_exists_at[]<fungible_asset::Metadata>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:193:9+65
    assume {:print "$at(3,11111,11176)"} true;
    assert {:msg "assert_failed(3,11111,11176): function does not abort under this condition"}
      !!$1_object_spec_exists_at'$1_fungible_asset_Metadata'($t1);

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:193:9+65
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:231:5+1
    assume {:print "$at(2,10042,10043)"} true;
L2:

    // assert Or(Not(exists[@44]<object::ObjectCore>($t1)), Not(object::spec_exists_at[]<fungible_asset::Metadata>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:190:5+312
    assume {:print "$at(3,10870,11182)"} true;
    assert {:msg "assert_failed(3,10870,11182): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#44, $t1) || !$1_object_spec_exists_at'$1_fungible_asset_Metadata'($t1));

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:190:5+312
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun fungible_asset::transfer_with_ref [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+312
procedure {:timeLimit 40} $1_fungible_asset_transfer_with_ref$verify(_$t0: $1_fungible_asset_TransferRef, _$t1: $1_object_Object'#0', _$t2: $1_object_Object'#0', _$t3: int) returns ()
{
    // declare local variables
    var $t4: $1_fungible_asset_FungibleAsset;
    var $t5: $1_fungible_asset_FungibleAsset;
    var $t6: int;
    var $t0: $1_fungible_asset_TransferRef;
    var $t1: $1_object_Object'#0';
    var $t2: $1_object_Object'#0';
    var $t3: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume {:print "$at(2,20407,20408)"} true;
    assume $IsValid'$1_fungible_asset_TransferRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume $IsValid'$1_object_Object'#0''($t2);

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume $IsValid'u64'($t3);

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // trace_local[transfer_ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume {:print "$track_local(56,35,0):", $t0} $t0 == $t0;

    // trace_local[from]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume {:print "$track_local(56,35,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume {:print "$track_local(56,35,2):", $t2} $t2 == $t2;

    // trace_local[amount]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:499:5+1
    assume {:print "$track_local(56,35,3):", $t3} $t3 == $t3;

    // $t5 := fungible_asset::withdraw_with_ref<#0>($t0, $t1, $t3) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:505:18+45
    assume {:print "$at(2,20619,20664)"} true;
    call $t5 := $1_fungible_asset_withdraw_with_ref'#0'($t0, $t1, $t3);
    if ($abort_flag) {
        assume {:print "$at(2,20619,20664)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,35):", $t6} $t6 == $t6;
        goto L2;
    }

    // trace_local[fa]($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:505:13+2
    assume {:print "$track_local(56,35,4):", $t5} $t5 == $t5;

    // fungible_asset::deposit_with_ref<#0>($t0, $t2, $t5) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:506:9+38
    assume {:print "$at(2,20674,20712)"} true;
    call $1_fungible_asset_deposit_with_ref'#0'($t0, $t2, $t5);
    if ($abort_flag) {
        assume {:print "$at(2,20674,20712)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,35):", $t6} $t6 == $t6;
        goto L2;
    }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:507:5+1
    assume {:print "$at(2,20718,20719)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:507:5+1
    assume {:print "$at(2,20718,20719)"} true;
    assert {:msg "assert_failed(2,20718,20719): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:507:5+1
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:507:5+1
L2:

    // abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:507:5+1
    assume {:print "$at(2,20718,20719)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun fungible_asset::add_fungibility [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1828
procedure {:timeLimit 40} $1_fungible_asset_add_fungibility$verify(_$t0: $1_object_ConstructorRef, _$t1: $1_option_Option'u128', _$t2: $1_string_String, _$t3: $1_string_String, _$t4: int, _$t5: $1_string_String, _$t6: $1_string_String) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t7: $signer;
    var $t8: $1_aggregator_v2_Aggregator'u128';
    var $t9: $signer;
    var $t10: $signer;
    var $t11: bool;
    var $t12: int;
    var $t13: bool;
    var $t14: int;
    var $t15: bool;
    var $t16: int;
    var $t17: int;
    var $t18: $signer;
    var $t19: int;
    var $t20: int;
    var $t21: bool;
    var $t22: int;
    var $t23: int;
    var $t24: int;
    var $t25: int;
    var $t26: bool;
    var $t27: int;
    var $t28: int;
    var $t29: int;
    var $t30: bool;
    var $t31: int;
    var $t32: int;
    var $t33: int;
    var $t34: int;
    var $t35: bool;
    var $t36: int;
    var $t37: int;
    var $t38: int;
    var $t39: int;
    var $t40: bool;
    var $t41: int;
    var $t42: int;
    var $t43: $1_fungible_asset_Metadata;
    var $t44: bool;
    var $t45: $1_option_Option'u128';
    var $t46: bool;
    var $t47: bool;
    var $t48: $Mutation ($1_option_Option'u128');
    var $t49: int;
    var $t50: $1_option_Option'u128';
    var $t51: bool;
    var $t52: bool;
    var $t53: $1_fungible_asset_ConcurrentSupply;
    var $t54: int;
    var $t55: $1_option_Option'u128';
    var $t56: $1_fungible_asset_Supply;
    var $t57: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_object_ConstructorRef;
    var $t1: $1_option_Option'u128';
    var $t2: $1_string_String;
    var $t3: $1_string_String;
    var $t4: int;
    var $t5: $1_string_String;
    var $t6: $1_string_String;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'$1_option_Option'u128'': $1_option_Option'u128';
    var $temp_0'$1_string_String': $1_string_String;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $temp_0'u8': int;
    var $1_object_ObjectCore_$memory#45: $Memory $1_object_ObjectCore;
    var $1_fungible_asset_Metadata_$memory#46: $Memory $1_fungible_asset_Metadata;
    var $1_fungible_asset_ConcurrentSupply_$memory#47: $Memory $1_fungible_asset_ConcurrentSupply;
    var $1_fungible_asset_Supply_$memory#48: $Memory $1_fungible_asset_Supply;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;
    $t4 := _$t4;
    $t5 := _$t5;
    $t6 := _$t6;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$at(2,6917,6918)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // assume And(WellFormed($t1), Le(Len<u128>(select option::Option.vec($t1)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume ($IsValid'$1_option_Option'u128''($t1) && (LenVec($t1->$vec) <= 1));

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume $IsValid'$1_string_String'($t2);

    // assume WellFormed($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume $IsValid'$1_string_String'($t3);

    // assume WellFormed($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume $IsValid'u8'($t4);

    // assume WellFormed($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume $IsValid'$1_string_String'($t5);

    // assume WellFormed($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume $IsValid'$1_string_String'($t6);

    // assume forall $rsc: features::Features: ResourceDomain<features::Features>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_features_Features_$memory, $a_0)}(var $rsc := $ResourceValue($1_features_Features_$memory, $a_0);
    ($IsValid'$1_features_Features'($rsc))));

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Metadata: ResourceDomain<fungible_asset::Metadata>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Metadata_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Metadata_$memory, $a_0);
    ($IsValid'$1_fungible_asset_Metadata'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // assume Identical($t12, object::$address_from_constructor_ref($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:165:9+80
    assume {:print "$at(3,9392,9472)"} true;
    assume ($t12 == $1_object_$address_from_constructor_ref($t0));

    // @45 := save_mem(object::ObjectCore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$at(2,6917,6918)"} true;
    $1_object_ObjectCore_$memory#45 := $1_object_ObjectCore_$memory;

    // @47 := save_mem(fungible_asset::ConcurrentSupply) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    $1_fungible_asset_ConcurrentSupply_$memory#47 := $1_fungible_asset_ConcurrentSupply_$memory;

    // @46 := save_mem(fungible_asset::Metadata) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    $1_fungible_asset_Metadata_$memory#46 := $1_fungible_asset_Metadata_$memory;

    // @48 := save_mem(fungible_asset::Supply) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    $1_fungible_asset_Supply_$memory#48 := $1_fungible_asset_Supply_$memory;

    // trace_local[constructor_ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$track_local(56,0,0):", $t0} $t0 == $t0;

    // trace_local[maximum_supply]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$track_local(56,0,1):", $t1} $t1 == $t1;

    // trace_local[name]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$track_local(56,0,2):", $t2} $t2 == $t2;

    // trace_local[symbol]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$track_local(56,0,3):", $t3} $t3 == $t3;

    // trace_local[decimals]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$track_local(56,0,4):", $t4} $t4 == $t4;

    // trace_local[icon_uri]($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$track_local(56,0,5):", $t5} $t5 == $t5;

    // trace_local[project_uri]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:166:5+1
    assume {:print "$track_local(56,0,6):", $t6} $t6 == $t6;

    // $t13 := object::can_generate_delete_ref($t0) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:18+48
    assume {:print "$at(2,7191,7239)"} true;
    call $t13 := $1_object_can_generate_delete_ref($t0);
    if ($abort_flag) {
        assume {:print "$at(2,7191,7239)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // $t15 := !($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:17+1
    call $t15 := $Not($t13);

    // if ($t15) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:9+105
    if ($t15) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:9+105
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:9+105
    assume {:print "$at(2,7182,7287)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:9+105
L0:

    // $t16 := 18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:92+20
    assume {:print "$at(2,7265,7285)"} true;
    $t16 := 18;
    assume $IsValid'u64'($t16);

    // $t17 := error::invalid_argument($t16) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:68+45
    call $t17 := $1_error_invalid_argument($t16);
    if ($abort_flag) {
        assume {:print "$at(2,7241,7286)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_abort($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:9+105
    assume {:print "$at(2,7182,7287)"} true;
    assume {:print "$track_abort(56,0):", $t17} $t17 == $t17;

    // $t14 := move($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:9+105
    $t14 := $t17;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:175:9+105
    goto L25;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:176:63+15
    assume {:print "$at(2,7351,7366)"} true;
L2:

    // $t18 := object::generate_signer($t0) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:176:39+40
    assume {:print "$at(2,7327,7367)"} true;
    call $t18 := $1_object_generate_signer($t0);
    if ($abort_flag) {
        assume {:print "$at(2,7327,7367)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_local[metadata_object_signer]($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:176:13+22
    assume {:print "$track_local(56,0,10):", $t18} $t18 == $t18;

    // $t19 := string::length($t2) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:17+21
    assume {:print "$at(2,7385,7406)"} true;
    call $t19 := $1_string_length($t2);
    if ($abort_flag) {
        assume {:print "$at(2,7385,7406)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // $t20 := 32 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:42+15
    $t20 := 32;
    assume $IsValid'u64'($t20);

    // $t21 := <=($t19, $t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:39+2
    call $t21 := $Le($t19, $t20);

    // if ($t21) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:9+86
    if ($t21) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:9+86
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:9+86
    assume {:print "$at(2,7377,7463)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:9+86
L3:

    // $t22 := 15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:79+14
    assume {:print "$at(2,7447,7461)"} true;
    $t22 := 15;
    assume $IsValid'u64'($t22);

    // $t23 := error::out_of_range($t22) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:59+35
    call $t23 := $1_error_out_of_range($t22);
    if ($abort_flag) {
        assume {:print "$at(2,7427,7462)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_abort($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:9+86
    assume {:print "$at(2,7377,7463)"} true;
    assume {:print "$track_abort(56,0):", $t23} $t23 == $t23;

    // $t14 := move($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:9+86
    $t14 := $t23;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:177:9+86
    goto L25;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:32+7
    assume {:print "$at(2,7496,7503)"} true;
L5:

    // $t24 := string::length($t3) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:17+23
    assume {:print "$at(2,7481,7504)"} true;
    call $t24 := $1_string_length($t3);
    if ($abort_flag) {
        assume {:print "$at(2,7481,7504)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // $t25 := 10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:44+17
    $t25 := 10;
    assume $IsValid'u64'($t25);

    // $t26 := <=($t24, $t25) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:41+2
    call $t26 := $Le($t24, $t25);

    // if ($t26) goto L7 else goto L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:9+92
    if ($t26) { goto L7; } else { goto L6; }

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:9+92
L7:

    // goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:9+92
    assume {:print "$at(2,7473,7565)"} true;
    goto L8;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:9+92
L6:

    // $t27 := 16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:83+16
    assume {:print "$at(2,7547,7563)"} true;
    $t27 := 16;
    assume $IsValid'u64'($t27);

    // $t28 := error::out_of_range($t27) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:63+37
    call $t28 := $1_error_out_of_range($t27);
    if ($abort_flag) {
        assume {:print "$at(2,7527,7564)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_abort($t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:9+92
    assume {:print "$at(2,7473,7565)"} true;
    assume {:print "$track_abort(56,0):", $t28} $t28 == $t28;

    // $t14 := move($t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:9+92
    $t14 := $t28;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:178:9+92
    goto L25;

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:17+8
    assume {:print "$at(2,7583,7591)"} true;
L8:

    // $t29 := 32 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:29+12
    assume {:print "$at(2,7595,7607)"} true;
    $t29 := 32;
    assume $IsValid'u8'($t29);

    // $t30 := <=($t4, $t29) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:26+2
    call $t30 := $Le($t4, $t29);

    // if ($t30) goto L10 else goto L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:9+75
    if ($t30) { goto L10; } else { goto L9; }

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:9+75
L10:

    // goto L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:9+75
    assume {:print "$at(2,7575,7650)"} true;
    goto L11;

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:9+75
L9:

    // $t31 := 17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:63+19
    assume {:print "$at(2,7629,7648)"} true;
    $t31 := 17;
    assume $IsValid'u64'($t31);

    // $t32 := error::out_of_range($t31) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:43+40
    call $t32 := $1_error_out_of_range($t31);
    if ($abort_flag) {
        assume {:print "$at(2,7609,7649)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_abort($t32) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:9+75
    assume {:print "$at(2,7575,7650)"} true;
    assume {:print "$track_abort(56,0):", $t32} $t32 == $t32;

    // $t14 := move($t32) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:9+75
    $t14 := $t32;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:179:9+75
    goto L25;

    // label L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:32+9
    assume {:print "$at(2,7683,7692)"} true;
L11:

    // $t33 := string::length($t5) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:17+25
    assume {:print "$at(2,7668,7693)"} true;
    call $t33 := $1_string_length($t5);
    if ($abort_flag) {
        assume {:print "$at(2,7668,7693)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // $t34 := 512 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:46+14
    $t34 := 512;
    assume $IsValid'u64'($t34);

    // $t35 := <=($t33, $t34) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:43+2
    call $t35 := $Le($t33, $t34);

    // if ($t35) goto L13 else goto L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:9+88
    if ($t35) { goto L13; } else { goto L12; }

    // label L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:9+88
L13:

    // goto L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:9+88
    assume {:print "$at(2,7660,7748)"} true;
    goto L14;

    // label L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:9+88
L12:

    // $t36 := 19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:82+13
    assume {:print "$at(2,7733,7746)"} true;
    $t36 := 19;
    assume $IsValid'u64'($t36);

    // $t37 := error::out_of_range($t36) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:62+34
    call $t37 := $1_error_out_of_range($t36);
    if ($abort_flag) {
        assume {:print "$at(2,7713,7747)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_abort($t37) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:9+88
    assume {:print "$at(2,7660,7748)"} true;
    assume {:print "$track_abort(56,0):", $t37} $t37 == $t37;

    // $t14 := move($t37) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:9+88
    $t14 := $t37;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:180:9+88
    goto L25;

    // label L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:32+12
    assume {:print "$at(2,7781,7793)"} true;
L14:

    // $t38 := string::length($t6) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:17+28
    assume {:print "$at(2,7766,7794)"} true;
    call $t38 := $1_string_length($t6);
    if ($abort_flag) {
        assume {:print "$at(2,7766,7794)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // $t39 := 512 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:49+14
    $t39 := 512;
    assume $IsValid'u64'($t39);

    // $t40 := <=($t38, $t39) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:46+2
    call $t40 := $Le($t38, $t39);

    // if ($t40) goto L16 else goto L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:9+91
    if ($t40) { goto L16; } else { goto L15; }

    // label L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:9+91
L16:

    // goto L17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:9+91
    assume {:print "$at(2,7758,7849)"} true;
    goto L17;

    // label L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:9+91
L15:

    // $t41 := 19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:85+13
    assume {:print "$at(2,7834,7847)"} true;
    $t41 := 19;
    assume $IsValid'u64'($t41);

    // $t42 := error::out_of_range($t41) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:65+34
    call $t42 := $1_error_out_of_range($t41);
    if ($abort_flag) {
        assume {:print "$at(2,7814,7848)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_abort($t42) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:9+91
    assume {:print "$at(2,7758,7849)"} true;
    assume {:print "$track_abort(56,0):", $t42} $t42 == $t42;

    // $t14 := move($t42) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:9+91
    $t14 := $t42;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:181:9+91
    goto L25;

    // label L17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:182:17+22
    assume {:print "$at(2,7867,7889)"} true;
L17:

    // $t43 := pack fungible_asset::Metadata($t2, $t3, $t4, $t5, $t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:183:13+151
    assume {:print "$at(2,7903,8054)"} true;
    $t43 := $1_fungible_asset_Metadata($t2, $t3, $t4, $t5, $t6);

    // move_to<fungible_asset::Metadata>($t43, $t18) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:182:9+7
    assume {:print "$at(2,7859,7866)"} true;
    if ($ResourceExists($1_fungible_asset_Metadata_$memory, $t18->$addr)) {
        call $ExecFailureAbort();
    } else {
        $1_fungible_asset_Metadata_$memory := $ResourceUpdate($1_fungible_asset_Metadata_$memory, $t18->$addr, $t43);
    }
    if ($abort_flag) {
        assume {:print "$at(2,7859,7866)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // $t44 := features::concurrent_assets_enabled() on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:192:13+37
    assume {:print "$at(2,8079,8116)"} true;
    call $t44 := $1_features_concurrent_assets_enabled();
    if ($abort_flag) {
        assume {:print "$at(2,8079,8116)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // if ($t44) goto L19 else goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:192:9+591
    if ($t44) { goto L19; } else { goto L18; }

    // label L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:193:45+15
    assume {:print "$at(2,8164,8179)"} true;
L19:

    // $t45 := copy($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:193:45+15
    assume {:print "$at(2,8164,8179)"} true;
    $t45 := $t1;

    // $t46 := opaque begin: option::is_none<u128>($t45) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:193:29+32

    // assume WellFormed($t46) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:193:29+32
    assume $IsValid'bool'($t46);

    // assume Eq<bool>($t46, option::spec_is_none<u128>($t45)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:193:29+32
    assume $IsEqual'bool'($t46, $1_option_spec_is_none'u128'($t45));

    // $t46 := opaque end: option::is_none<u128>($t45) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:193:29+32

    // trace_local[unlimited]($t46) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:193:17+9
    assume {:print "$track_local(56,0,11):", $t46} $t46 == $t46;

    // if ($t46) goto L21 else goto L20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:195:26+215
    assume {:print "$at(2,8270,8485)"} true;
    if ($t46) { goto L21; } else { goto L20; }

    // label L21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
    assume {:print "$at(2,8307,8351)"} true;
L21:

    // $t8 := opaque begin: aggregator_v2::create_unbounded_aggregator<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
    assume {:print "$at(2,8307,8351)"} true;

    // $t47 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
    havoc $t47;

    // if ($t47) goto L27 else goto L26 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
    if ($t47) { goto L27; } else { goto L26; }

    // label L27 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
L27:

    // trace_abort($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
    assume {:print "$at(2,8307,8351)"} true;
    assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
    goto L25;

    // label L26 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
L26:

    // assume WellFormed($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44
    assume {:print "$at(2,8307,8351)"} true;
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($t8);

    // $t8 := opaque end: aggregator_v2::create_unbounded_aggregator<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:196:21+44

    // goto L22 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:195:26+215
    assume {:print "$at(2,8270,8485)"} true;
    goto L22;

    // label L20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:70+19
    assume {:print "$at(2,8446,8465)"} true;
L20:

    // $t48 := borrow_local($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:70+19
    assume {:print "$at(2,8446,8465)"} true;
    $t48 := $Mutation($Local(1), EmptyVec(), $t1);

    // $t49 := opaque begin: option::extract<u128>($t48) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36

    // $t50 := read_ref($t48) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    $t50 := $Dereference($t48);

    // assume Identical($t51, option::spec_is_none<u128>($t48)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume ($t51 == $1_option_spec_is_none'u128'($Dereference($t48)));

    // if ($t51) goto L29 else goto L32 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    if ($t51) { goto L29; } else { goto L32; }

    // label L29 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
L29:

    // assume And(option::spec_is_none<u128>($t48), Eq(262145, $t14)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume {:print "$at(2,8430,8466)"} true;
    assume ($1_option_spec_is_none'u128'($Dereference($t48)) && $IsEqual'num'(262145, $t14));

    // trace_abort($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume {:print "$at(2,8430,8466)"} true;
    assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    goto L25;

    // label L28 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
L28:

    // $t48 := havoc[mut]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume {:print "$at(2,8430,8466)"} true;
    havoc $temp_0'$1_option_Option'u128'';
    $t48 := $UpdateMutation($t48, $temp_0'$1_option_Option'u128'');

    // assume And(WellFormed($t48), Le(Len<u128>(select option::Option.vec($t48)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume ($IsValid'$1_option_Option'u128''($Dereference($t48)) && (LenVec($Dereference($t48)->$vec) <= 1));

    // assume WellFormed($t49) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume $IsValid'u128'($t49);

    // assume Eq<u128>($t49, option::spec_borrow<u128>($t50)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume $IsEqual'u128'($t49, $1_option_spec_borrow'u128'($t50));

    // assume option::spec_is_none<u128>($t48) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume $1_option_spec_is_none'u128'($Dereference($t48));

    // $t49 := opaque end: option::extract<u128>($t48) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36

    // assert Le(Len<u128>(select option::Option.vec($t48)), 1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    // data invariant at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    assume {:print "$at(15,530,554)"} true;
    assert {:msg "assert_failed(15,530,554): data invariant does not hold"}
      (LenVec($Dereference($t48)->$vec) <= 1);

    // write_back[LocalRoot($t1)@]($t48) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume {:print "$at(2,8430,8466)"} true;
    $t1 := $Dereference($t48);

    // trace_local[maximum_supply]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:54+36
    assume {:print "$track_local(56,0,1):", $t1} $t1 == $t1;

    // $t8 := opaque begin: aggregator_v2::create_aggregator<u128>($t49) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70

    // $t52 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70
    havoc $t52;

    // if ($t52) goto L31 else goto L30 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70
    if ($t52) { goto L31; } else { goto L30; }

    // label L31 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70
L31:

    // trace_abort($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70
    assume {:print "$at(2,8397,8467)"} true;
    assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;

    // goto L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70
    goto L25;

    // label L30 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70
L30:

    // assume WellFormed($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70
    assume {:print "$at(2,8397,8467)"} true;
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($t8);

    // $t8 := opaque end: aggregator_v2::create_aggregator<u128>($t49) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:198:21+70

    // label L22 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:194:21+22
    assume {:print "$at(2,8202,8224)"} true;
L22:

    // $t53 := pack fungible_asset::ConcurrentSupply($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:194:45+274
    assume {:print "$at(2,8226,8500)"} true;
    $t53 := $1_fungible_asset_ConcurrentSupply($t8);

    // move_to<fungible_asset::ConcurrentSupply>($t53, $t18) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:194:13+7
    if ($ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t18->$addr)) {
        call $ExecFailureAbort();
    } else {
        $1_fungible_asset_ConcurrentSupply_$memory := $ResourceUpdate($1_fungible_asset_ConcurrentSupply_$memory, $t18->$addr, $t53);
    }
    if ($abort_flag) {
        assume {:print "$at(2,8194,8201)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // goto L23 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:200:15+1
    assume {:print "$at(2,8501,8502)"} true;
    goto L23;

    // label L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:202:21+22
    assume {:print "$at(2,8540,8562)"} true;
L18:

    // $t54 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:203:26+1
    assume {:print "$at(2,8598,8599)"} true;
    $t54 := 0;
    assume $IsValid'u128'($t54);

    // $t55 := move($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:204:26+14
    assume {:print "$at(2,8626,8640)"} true;
    $t55 := $t1;

    // $t56 := pack fungible_asset::Supply($t54, $t55) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:202:45+90
    assume {:print "$at(2,8564,8654)"} true;
    $t56 := $1_fungible_asset_Supply($t54, $t55);

    // move_to<fungible_asset::Supply>($t56, $t18) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:202:13+7
    if ($ResourceExists($1_fungible_asset_Supply_$memory, $t18->$addr)) {
        call $ExecFailureAbort();
    } else {
        $1_fungible_asset_Supply_$memory := $ResourceUpdate($1_fungible_asset_Supply_$memory, $t18->$addr, $t56);
    }
    if ($abort_flag) {
        assume {:print "$at(2,8532,8539)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // label L23 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:208:55+15
    assume {:print "$at(2,8723,8738)"} true;
L23:

    // $t57 := object::object_from_constructor_ref<fungible_asset::Metadata>($t0) on_abort goto L25 with $t14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:208:9+62
    assume {:print "$at(2,8677,8739)"} true;
    call $t57 := $1_object_object_from_constructor_ref'$1_fungible_asset_Metadata'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,8677,8739)"} true;
        $t14 := $abort_code;
        assume {:print "$track_abort(56,0):", $t14} $t14 == $t14;
        goto L25;
    }

    // trace_return[0]($t57) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:208:9+62
    assume {:print "$track_return(56,0,0):", $t57} $t57 == $t57;

    // label L24 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:209:5+1
    assume {:print "$at(2,8744,8745)"} true;
L24:

    // assert Not(object::$can_generate_delete_ref[]($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:159:9+61
    assume {:print "$at(3,9024,9085)"} true;
    assert {:msg "assert_failed(3,9024,9085): function does not abort under this condition"}
      !$1_object_$can_generate_delete_ref($t0);

    // assert Not(Gt(string::$length[]($t2), 32)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:160:9+51
    assume {:print "$at(3,9094,9145)"} true;
    assert {:msg "assert_failed(3,9094,9145): function does not abort under this condition"}
      !($1_string_$length($t2) > 32);

    // assert Not(Gt(string::$length[]($t3), 10)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:161:9+55
    assume {:print "$at(3,9154,9209)"} true;
    assert {:msg "assert_failed(3,9154,9209): function does not abort under this condition"}
      !($1_string_$length($t3) > 10);

    // assert Not(Gt($t4, 32)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:162:9+36
    assume {:print "$at(3,9218,9254)"} true;
    assert {:msg "assert_failed(3,9218,9254): function does not abort under this condition"}
      !($t4 > 32);

    // assert Not(Gt(string::$length[]($t5), 512)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:163:9+54
    assume {:print "$at(3,9263,9317)"} true;
    assert {:msg "assert_failed(3,9263,9317): function does not abort under this condition"}
      !($1_string_$length($t5) > 512);

    // assert Not(Gt(string::$length[]($t6), 512)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:164:9+57
    assume {:print "$at(3,9326,9383)"} true;
    assert {:msg "assert_failed(3,9326,9383): function does not abort under this condition"}
      !($1_string_$length($t6) > 512);

    // assert Not(Not(exists[@45]<object::ObjectCore>($t12))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:166:9+59
    assume {:print "$at(3,9481,9540)"} true;
    assert {:msg "assert_failed(3,9481,9540): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#45, $t12);

    // assert Not(Not(object::spec_exists_at[]<fungible_asset::Metadata>($t12))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:167:9+65
    assume {:print "$at(3,9549,9614)"} true;
    assert {:msg "assert_failed(3,9549,9614): function does not abort under this condition"}
      !!$1_object_spec_exists_at'$1_fungible_asset_Metadata'($t12);

    // assert Not(exists[@46]<fungible_asset::Metadata>($t12)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:168:9+48
    assume {:print "$at(3,9623,9671)"} true;
    assert {:msg "assert_failed(3,9623,9671): function does not abort under this condition"}
      !$ResourceExists($1_fungible_asset_Metadata_$memory#46, $t12);

    // assert Not(And(features::spec_is_enabled[](37), exists[@47]<fungible_asset::ConcurrentSupply>($t12))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:169:9+116
    assume {:print "$at(3,9680,9796)"} true;
    assert {:msg "assert_failed(3,9680,9796): function does not abort under this condition"}
      !($1_features_spec_is_enabled(37) && $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory#47, $t12));

    // assert Not(And(Not(features::spec_is_enabled[](37)), exists[@48]<fungible_asset::Supply>($t12))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:170:9+107
    assume {:print "$at(3,9805,9912)"} true;
    assert {:msg "assert_failed(3,9805,9912): function does not abort under this condition"}
      !(!$1_features_spec_is_enabled(37) && $ResourceExists($1_fungible_asset_Supply_$memory#48, $t12));

    // assert exists<fungible_asset::Metadata>($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:173:9+46
    assume {:print "$at(3,9953,9999)"} true;
    assert {:msg "assert_failed(3,9953,9999): post-condition does not hold"}
      $ResourceExists($1_fungible_asset_Metadata_$memory, $t12);

    // assert Implies(features::spec_is_enabled(37), exists<fungible_asset::ConcurrentSupply>($t12)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:174:9+115
    assume {:print "$at(3,10008,10123)"} true;
    assert {:msg "assert_failed(3,10008,10123): post-condition does not hold"}
      ($1_features_spec_is_enabled(37) ==> $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t12));

    // assert Implies(Not(features::spec_is_enabled(37)), exists<fungible_asset::Supply>($t12)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:175:9+106
    assume {:print "$at(3,10132,10238)"} true;
    assert {:msg "assert_failed(3,10132,10238): post-condition does not hold"}
      (!$1_features_spec_is_enabled(37) ==> $ResourceExists($1_fungible_asset_Supply_$memory, $t12));

    // return $t57 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:175:9+106
    $ret0 := $t57;
    return;

    // label L25 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:209:5+1
    assume {:print "$at(2,8744,8745)"} true;
L25:

    // abort($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:209:5+1
    assume {:print "$at(2,8744,8745)"} true;
    $abort_code := $t14;
    $abort_flag := true;
    return;

    // label L32 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L32:

    // destroy($t48) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L28 at <internal>:1:1+10
    goto L28;

}

// fun fungible_asset::asset_metadata [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:327:5+91
procedure {:timeLimit 40} $1_fungible_asset_asset_metadata$verify(_$t0: $1_fungible_asset_FungibleAsset) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:327:5+1
    assume {:print "$at(2,13533,13534)"} true;
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t0);

    // trace_local[fa]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:327:5+1
    assume {:print "$track_local(56,2,0):", $t0} $t0 == $t0;

    // $t1 := get_field<fungible_asset::FungibleAsset>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:328:9+11
    assume {:print "$at(2,13607,13618)"} true;
    $t1 := $t0->$metadata;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:328:9+11
    assume {:print "$track_return(56,2,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:329:5+1
    assume {:print "$at(2,13623,13624)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:329:5+1
    assume {:print "$at(2,13623,13624)"} true;
    assert {:msg "assert_failed(2,13623,13624): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:329:5+1
    $ret0 := $t1;
    return;

}

// fun fungible_asset::burn_ref_metadata [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:342:5+90
procedure {:timeLimit 40} $1_fungible_asset_burn_ref_metadata$verify(_$t0: $1_fungible_asset_BurnRef) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_fungible_asset_BurnRef;
    var $temp_0'$1_fungible_asset_BurnRef': $1_fungible_asset_BurnRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:342:5+1
    assume {:print "$at(2,14023,14024)"} true;
    assume $IsValid'$1_fungible_asset_BurnRef'($t0);

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:342:5+1
    assume {:print "$track_local(56,6,0):", $t0} $t0 == $t0;

    // $t1 := get_field<fungible_asset::BurnRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:343:9+12
    assume {:print "$at(2,14095,14107)"} true;
    $t1 := $t0->$metadata;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:343:9+12
    assume {:print "$track_return(56,6,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:344:5+1
    assume {:print "$at(2,14112,14113)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:344:5+1
    assume {:print "$at(2,14112,14113)"} true;
    assert {:msg "assert_failed(2,14112,14113): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:344:5+1
    $ret0 := $t1;
    return;

}

// fun fungible_asset::create_store [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+774
procedure {:timeLimit 40} $1_fungible_asset_create_store$verify(_$t0: $1_object_ConstructorRef, _$t1: $1_object_Object'#0') returns ($ret0: $1_object_Object'$1_fungible_asset_FungibleStore')
{
    // declare local variables
    var $t2: $signer;
    var $t3: $signer;
    var $t4: int;
    var $t5: $signer;
    var $t6: int;
    var $t7: $1_object_Object'$1_fungible_asset_Metadata';
    var $t8: int;
    var $t9: bool;
    var $t10: $1_fungible_asset_FungibleStore;
    var $t11: $1_object_ObjectCore;
    var $t12: $1_guid_GUID;
    var $t13: $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    var $t14: $1_object_ObjectCore;
    var $t15: $1_guid_GUID;
    var $t16: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $t17: $1_object_ObjectCore;
    var $t18: $1_guid_GUID;
    var $t19: $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $t20: $1_fungible_asset_FungibleAssetEvents;
    var $t21: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $t0: $1_object_ConstructorRef;
    var $t1: $1_object_Object'#0';
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'$1_fungible_asset_FungibleStore'': $1_object_Object'$1_fungible_asset_FungibleStore';
    var $temp_0'signer': $signer;
    var $1_fungible_asset_FungibleStore_$memory#49: $Memory $1_fungible_asset_FungibleStore;
    var $1_object_ObjectCore_$memory#50: $Memory $1_object_ObjectCore;
    var $1_fungible_asset_FungibleAssetEvents_$memory#51: $Memory $1_fungible_asset_FungibleAssetEvents;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume {:print "$at(2,14730,14731)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume Identical($t4, object::$address_from_constructor_ref($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:269:9+80
    assume {:print "$at(3,12974,13054)"} true;
    assume ($t4 == $1_object_$address_from_constructor_ref($t0));

    // @50 := save_mem(object::ObjectCore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume {:print "$at(2,14730,14731)"} true;
    $1_object_ObjectCore_$memory#50 := $1_object_ObjectCore_$memory;

    // @51 := save_mem(fungible_asset::FungibleAssetEvents) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    $1_fungible_asset_FungibleAssetEvents_$memory#51 := $1_fungible_asset_FungibleAssetEvents_$memory;

    // @49 := save_mem(fungible_asset::FungibleStore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    $1_fungible_asset_FungibleStore_$memory#49 := $1_fungible_asset_FungibleStore_$memory;

    // trace_local[constructor_ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume {:print "$track_local(56,7,0):", $t0} $t0 == $t0;

    // trace_local[metadata]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:360:5+1
    assume {:print "$track_local(56,7,1):", $t1} $t1 == $t1;

    // $t5 := object::generate_signer($t0) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:364:26+40
    assume {:print "$at(2,14890,14930)"} true;
    call $t5 := $1_object_generate_signer($t0);
    if ($abort_flag) {
        assume {:print "$at(2,14890,14930)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // trace_local[store_obj]($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:364:13+9
    assume {:print "$track_local(56,7,3):", $t5} $t5 == $t5;

    // $t7 := object::convert<#0, fungible_asset::Metadata>($t1) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:366:23+25
    assume {:print "$at(2,14997,15022)"} true;
    call $t7 := $1_object_convert'#0_$1_fungible_asset_Metadata'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,14997,15022)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // $t8 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:367:22+1
    assume {:print "$at(2,15045,15046)"} true;
    $t8 := 0;
    assume $IsValid'u64'($t8);

    // $t9 := false at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:368:21+5
    assume {:print "$at(2,15068,15073)"} true;
    $t9 := false;
    assume $IsValid'bool'($t9);

    // $t10 := pack fungible_asset::FungibleStore($t7, $t8, $t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:365:28+125
    assume {:print "$at(2,14959,15084)"} true;
    $t10 := $1_fungible_asset_FungibleStore($t7, $t8, $t9);

    // move_to<fungible_asset::FungibleStore>($t10, $t5) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:365:9+7
    if ($ResourceExists($1_fungible_asset_FungibleStore_$memory, $t5->$addr)) {
        call $ExecFailureAbort();
    } else {
        $1_fungible_asset_FungibleStore_$memory := $ResourceUpdate($1_fungible_asset_FungibleStore_$memory, $t5->$addr, $t10);
    }
    if ($abort_flag) {
        assume {:print "$at(2,14940,14947)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // assume Identical($t11, global<object::ObjectCore>(signer::$address_of($t5))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:352:9+65
    assume {:print "$at(136,14413,14478)"} true;
    assume ($t11 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t5)));

    // assume Identical($t12, pack guid::GUID(pack guid::ID(select object::ObjectCore.guid_creation_num($t11), signer::$address_of($t5)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:355:9+185
    assume {:print "$at(136,14551,14736)"} true;
    assume ($t12 == $1_guid_GUID($1_guid_ID($t11->$guid_creation_num, $1_signer_$address_of($t5))));

    // $t13 := object::new_event_handle<fungible_asset::DepositEvent>($t5) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:372:33+49
    assume {:print "$at(2,15180,15229)"} true;
    call $t13 := $1_object_new_event_handle'$1_fungible_asset_DepositEvent'($t5);
    if ($abort_flag) {
        assume {:print "$at(2,15180,15229)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // assume Identical($t14, global<object::ObjectCore>(signer::$address_of($t5))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:352:9+65
    assume {:print "$at(136,14413,14478)"} true;
    assume ($t14 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t5)));

    // assume Identical($t15, pack guid::GUID(pack guid::ID(select object::ObjectCore.guid_creation_num($t14), signer::$address_of($t5)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:355:9+185
    assume {:print "$at(136,14551,14736)"} true;
    assume ($t15 == $1_guid_GUID($1_guid_ID($t14->$guid_creation_num, $1_signer_$address_of($t5))));

    // $t16 := object::new_event_handle<fungible_asset::WithdrawEvent>($t5) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:373:34+50
    assume {:print "$at(2,15264,15314)"} true;
    call $t16 := $1_object_new_event_handle'$1_fungible_asset_WithdrawEvent'($t5);
    if ($abort_flag) {
        assume {:print "$at(2,15264,15314)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // assume Identical($t17, global<object::ObjectCore>(signer::$address_of($t5))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:352:9+65
    assume {:print "$at(136,14413,14478)"} true;
    assume ($t17 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t5)));

    // assume Identical($t18, pack guid::GUID(pack guid::ID(select object::ObjectCore.guid_creation_num($t17), signer::$address_of($t5)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:355:9+185
    assume {:print "$at(136,14551,14736)"} true;
    assume ($t18 == $1_guid_GUID($1_guid_ID($t17->$guid_creation_num, $1_signer_$address_of($t5))));

    // $t19 := object::new_event_handle<fungible_asset::FrozenEvent>($t5) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:374:32+48
    assume {:print "$at(2,15347,15395)"} true;
    call $t19 := $1_object_new_event_handle'$1_fungible_asset_FrozenEvent'($t5);
    if ($abort_flag) {
        assume {:print "$at(2,15347,15395)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // $t20 := pack fungible_asset::FungibleAssetEvents($t13, $t16, $t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:371:13+284
    assume {:print "$at(2,15126,15410)"} true;
    $t20 := $1_fungible_asset_FungibleAssetEvents($t13, $t16, $t19);

    // move_to<fungible_asset::FungibleAssetEvents>($t20, $t5) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:370:9+7
    assume {:print "$at(2,15095,15102)"} true;
    if ($ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t5->$addr)) {
        call $ExecFailureAbort();
    } else {
        $1_fungible_asset_FungibleAssetEvents_$memory := $ResourceUpdate($1_fungible_asset_FungibleAssetEvents_$memory, $t5->$addr, $t20);
    }
    if ($abort_flag) {
        assume {:print "$at(2,15095,15102)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // $t21 := object::object_from_constructor_ref<fungible_asset::FungibleStore>($t0) on_abort goto L2 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:378:9+67
    assume {:print "$at(2,15431,15498)"} true;
    call $t21 := $1_object_object_from_constructor_ref'$1_fungible_asset_FungibleStore'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,15431,15498)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,7):", $t6} $t6 == $t6;
        goto L2;
    }

    // trace_return[0]($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:378:9+67
    assume {:print "$track_return(56,7,0):", $t21} $t21 == $t21;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:379:5+1
    assume {:print "$at(2,15503,15504)"} true;
L1:

    // assert Not(exists[@49]<fungible_asset::FungibleStore>($t4)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:270:9+53
    assume {:print "$at(3,13063,13116)"} true;
    assert {:msg "assert_failed(3,13063,13116): function does not abort under this condition"}
      !$ResourceExists($1_fungible_asset_FungibleStore_$memory#49, $t4);

    // assert Not(Not(exists[@50]<object::ObjectCore>(select object::Object.inner($t1)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:271:9+54
    assume {:print "$at(3,13125,13179)"} true;
    assert {:msg "assert_failed(3,13125,13179): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#50, $t1->$inner);

    // assert Not(exists[@51]<fungible_asset::FungibleAssetEvents>($t4)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:272:9+59
    assume {:print "$at(3,13188,13247)"} true;
    assert {:msg "assert_failed(3,13188,13247): function does not abort under this condition"}
      !$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory#51, $t4);

    // assert exists<fungible_asset::FungibleStore>($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:274:9+51
    assume {:print "$at(3,13257,13308)"} true;
    assert {:msg "assert_failed(3,13257,13308): post-condition does not hold"}
      $ResourceExists($1_fungible_asset_FungibleStore_$memory, $t4);

    // assert exists<fungible_asset::FungibleAssetEvents>($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:275:9+57
    assume {:print "$at(3,13317,13374)"} true;
    assert {:msg "assert_failed(3,13317,13374): post-condition does not hold"}
      $ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t4);

    // assert Eq<u64>(select fungible_asset::FungibleStore.balance(global<fungible_asset::FungibleStore>($t4)), 0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:278:9+71
    assume {:print "$at(3,13415,13486)"} true;
    assert {:msg "assert_failed(3,13415,13486): post-condition does not hold"}
      $IsEqual'u64'($ResourceValue($1_fungible_asset_FungibleStore_$memory, $t4)->$balance, 0);

    // return $t21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:278:9+71
    $ret0 := $t21;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:379:5+1
    assume {:print "$at(2,15503,15504)"} true;
L2:

    // abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:379:5+1
    assume {:print "$at(2,15503,15504)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun fungible_asset::decrease_supply<fungible_asset::Metadata> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1075
procedure {:inline 1} $1_fungible_asset_decrease_supply'$1_fungible_asset_Metadata'(_$t0: $1_object_Object'$1_fungible_asset_Metadata', _$t1: int) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $Mutation ($1_fungible_asset_Supply);
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: bool;
    var $t11: $Mutation ($1_fungible_asset_ConcurrentSupply);
    var $t12: $Mutation ($1_aggregator_v2_Aggregator'u128');
    var $t13: int;
    var $t14: bool;
    var $t15: bool;
    var $t16: int;
    var $t17: int;
    var $t18: bool;
    var $t19: bool;
    var $t20: int;
    var $t21: int;
    var $t22: $Mutation ($1_fungible_asset_Supply);
    var $t23: int;
    var $t24: int;
    var $t25: bool;
    var $t26: int;
    var $t27: int;
    var $t28: int;
    var $t29: int;
    var $t30: int;
    var $t31: int;
    var $t32: int;
    var $t33: $Mutation (int);
    var $t0: $1_object_Object'$1_fungible_asset_Metadata';
    var $t1: int;
    var $temp_0'$1_aggregator_v2_Aggregator'u128'': $1_aggregator_v2_Aggregator'u128';
    var $temp_0'$1_fungible_asset_Supply': $1_fungible_asset_Supply;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume {:print "$at(2,25000,25001)"} true;
    assume {:print "$track_local(56,9,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume {:print "$track_local(56,9,1):", $t1} $t1 == $t1;

    // $t4 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:27+1
    assume {:print "$at(2,25125,25126)"} true;
    $t4 := 0;
    assume $IsValid'u64'($t4);

    // $t5 := !=($t1, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:24+2
    $t5 := !$IsEqual'u64'($t1, $t4);

    // if ($t5) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    if ($t5) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    assume {:print "$at(2,25107,25176)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
L0:

    // $t6 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:54+22
    assume {:print "$at(2,25152,25174)"} true;
    $t6 := 1;
    assume $IsValid'u64'($t6);

    // $t7 := error::invalid_argument($t6) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:30+47
    call $t7 := $1_error_invalid_argument($t6);
    if ($abort_flag) {
        assume {:print "$at(2,25128,25175)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    assume {:print "$at(2,25107,25176)"} true;
    assume {:print "$track_abort(56,9):", $t7} $t7 == $t7;

    // $t8 := move($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    $t8 := $t7;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    goto L18;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:602:55+8
    assume {:print "$at(2,25232,25240)"} true;
L2:

    // $t9 := object::object_address<#0>($t0) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:602:32+32
    assume {:print "$at(2,25209,25241)"} true;
    call $t9 := $1_object_object_address'$1_fungible_asset_Metadata'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,25209,25241)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_local[metadata_address]($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:602:13+16
    assume {:print "$track_local(56,9,2):", $t9} $t9 == $t9;

    // $t10 := exists<fungible_asset::ConcurrentSupply>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:13+6
    assume {:print "$at(2,25256,25262)"} true;
    $t10 := $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t9);

    // if ($t10) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    if ($t10) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:605:62+16
    assume {:print "$at(2,25363,25379)"} true;
L4:

    // $t11 := borrow_global<fungible_asset::ConcurrentSupply>($t9) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:605:26+17
    assume {:print "$at(2,25327,25344)"} true;
    if (!$ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t9)) {
        call $ExecFailureAbort();
    } else {
        $t11 := $Mutation($Global($t9), EmptyVec(), $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t9));
    }
    if ($abort_flag) {
        assume {:print "$at(2,25327,25344)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t12 := borrow_field<fungible_asset::ConcurrentSupply>.current($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:40+19
    assume {:print "$at(2,25443,25462)"} true;
    $t12 := $ChildMutation($t11, 0, $Dereference($t11)->$current);

    // $t13 := (u128)($t1) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:61+16
    call $t13 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,25464,25480)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t14 := opaque begin: aggregator_v2::try_sub<u128>($t12, $t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61

    // $t15 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    havoc $t15;

    // if ($t15) goto L21 else goto L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    if ($t15) { goto L21; } else { goto L19; }

    // label L20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
L20:

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume {:print "$at(2,25420,25481)"} true;
    assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    goto L18;

    // label L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
L19:

    // $t12 := havoc[mut]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume {:print "$at(2,25420,25481)"} true;
    havoc $temp_0'$1_aggregator_v2_Aggregator'u128'';
    $t12 := $UpdateMutation($t12, $temp_0'$1_aggregator_v2_Aggregator'u128'');

    // assume WellFormed($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($Dereference($t12));

    // assume WellFormed($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume $IsValid'bool'($t14);

    // $t14 := opaque end: aggregator_v2::try_sub<u128>($t12, $t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61

    // write_back[Reference($t11).current (aggregator_v2::Aggregator<u128>)]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    $t11 := $UpdateMutation($t11, $Update'$1_fungible_asset_ConcurrentSupply'_current($Dereference($t11), $Dereference($t12)));

    // write_back[fungible_asset::ConcurrentSupply@]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    $1_fungible_asset_ConcurrentSupply_$memory := $ResourceUpdate($1_fungible_asset_ConcurrentSupply_$memory, $GlobalLocationAddress($t11),
        $Dereference($t11));

    // if ($t14) goto L6 else goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    assume {:print "$at(2,25395,25551)"} true;
    if ($t14) { goto L6; } else { goto L5; }

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
L6:

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    assume {:print "$at(2,25395,25551)"} true;
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:609:37+17
    assume {:print "$at(2,25519,25536)"} true;
L5:

    // $t16 := 20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:609:37+17
    assume {:print "$at(2,25519,25536)"} true;
    $t16 := 20;
    assume $IsValid'u64'($t16);

    // $t17 := error::out_of_range($t16) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:609:17+38
    call $t17 := $1_error_out_of_range($t16);
    if ($abort_flag) {
        assume {:print "$at(2,25499,25537)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    assume {:print "$at(2,25395,25551)"} true;
    assume {:print "$track_abort(56,9):", $t17} $t17 == $t17;

    // $t8 := move($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    $t8 := $t17;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    goto L18;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    assume {:print "$at(2,25252,26069)"} true;
L7:

    // goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    assume {:print "$at(2,25252,26069)"} true;
    goto L8;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:611:35+16
    assume {:print "$at(2,25587,25603)"} true;
L3:

    // $t18 := exists<fungible_asset::Supply>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:611:20+6
    assume {:print "$at(2,25572,25578)"} true;
    $t18 := $ResourceExists($1_fungible_asset_Supply_$memory, $t9);

    // if ($t18) goto L10 else goto L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:611:16+501
    if ($t18) { goto L10; } else { goto L9; }

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:36+16
    assume {:print "$at(2,25643,25659)"} true;
L10:

    // $t19 := exists<fungible_asset::Supply>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:21+6
    assume {:print "$at(2,25628,25634)"} true;
    $t19 := $ResourceExists($1_fungible_asset_Supply_$memory, $t9);

    // if ($t19) goto L12 else goto L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    if ($t19) { goto L12; } else { goto L11; }

    // label L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
L12:

    // goto L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    assume {:print "$at(2,25620,25698)"} true;
    goto L13;

    // label L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:72+17
L11:

    // $t20 := 21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:72+17
    assume {:print "$at(2,25679,25696)"} true;
    $t20 := 21;
    assume $IsValid'u64'($t20);

    // $t21 := error::not_found($t20) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:55+35
    call $t21 := $1_error_not_found($t20);
    if ($abort_flag) {
        assume {:print "$at(2,25662,25697)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    assume {:print "$at(2,25620,25698)"} true;
    assume {:print "$track_abort(56,9):", $t21} $t21 == $t21;

    // $t8 := move($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    $t8 := $t21;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    goto L18;

    // label L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:613:52+16
    assume {:print "$at(2,25751,25767)"} true;
L13:

    // $t22 := borrow_global<fungible_asset::Supply>($t9) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:613:26+17
    assume {:print "$at(2,25725,25742)"} true;
    if (!$ResourceExists($1_fungible_asset_Supply_$memory, $t9)) {
        call $ExecFailureAbort();
    } else {
        $t22 := $Mutation($Global($t9), EmptyVec(), $ResourceValue($1_fungible_asset_Supply_$memory, $t9));
    }
    if ($abort_flag) {
        assume {:print "$at(2,25725,25742)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_local[supply#1]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:613:17+6
    $temp_0'$1_fungible_asset_Supply' := $Dereference($t22);
    assume {:print "$track_local(56,9,3):", $temp_0'$1_fungible_asset_Supply'} $temp_0'$1_fungible_asset_Supply' == $temp_0'$1_fungible_asset_Supply';

    // $t23 := get_field<fungible_asset::Supply>.current($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:615:17+14
    assume {:print "$at(2,25807,25821)"} true;
    $t23 := $Dereference($t22)->$current;

    // $t24 := (u128)($t1) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:615:35+16
    call $t24 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,25825,25841)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t25 := >=($t23, $t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:615:32+2
    call $t25 := $Ge($t23, $t24);

    // if ($t25) goto L15 else goto L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;
    if ($t25) { goto L15; } else { goto L14; }

    // label L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
L15:

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;
    goto L16;

    // label L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
L14:

    // pack_ref_deep($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;

    // destroy($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130

    // $t26 := 20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:616:38+17
    assume {:print "$at(2,25880,25897)"} true;
    $t26 := 20;
    assume $IsValid'u64'($t26);

    // $t27 := error::invalid_state($t26) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:616:17+39
    call $t27 := $1_error_invalid_state($t26);
    if ($abort_flag) {
        assume {:print "$at(2,25859,25898)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;
    assume {:print "$track_abort(56,9):", $t27} $t27 == $t27;

    // $t8 := move($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    $t8 := $t27;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    goto L18;

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:45+17
    assume {:print "$at(2,26039,26056)"} true;
L9:

    // $t28 := 21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:45+17
    assume {:print "$at(2,26039,26056)"} true;
    $t28 := 21;
    assume $IsValid'u64'($t28);

    // $t29 := error::not_found($t28) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:28+35
    call $t29 := $1_error_not_found($t28);
    if ($abort_flag) {
        assume {:print "$at(2,26022,26057)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t29) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:13+51
    assume {:print "$at(2,26007,26058)"} true;
    assume {:print "$track_abort(56,9):", $t29} $t29 == $t29;

    // $t8 := move($t29) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:13+51
    $t8 := $t29;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:13+51
    goto L18;

    // label L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:30+6
    assume {:print "$at(2,25943,25949)"} true;
L16:

    // $t30 := get_field<fungible_asset::Supply>.current($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:30+14
    assume {:print "$at(2,25943,25957)"} true;
    $t30 := $Dereference($t22)->$current;

    // $t31 := (u128)($t1) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:47+16
    call $t31 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,25960,25976)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t32 := -($t30, $t31) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:45+1
    call $t32 := $Sub($t30, $t31);
    if ($abort_flag) {
        assume {:print "$at(2,25958,25959)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t33 := borrow_field<fungible_asset::Supply>.current($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+14
    $t33 := $ChildMutation($t22, 0, $Dereference($t22)->$current);

    // write_ref($t33, $t32) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+50
    $t33 := $UpdateMutation($t33, $t32);

    // write_back[Reference($t22).current (u128)]($t33) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+50
    $t22 := $UpdateMutation($t22, $Update'$1_fungible_asset_Supply'_current($Dereference($t22), $Dereference($t33)));

    // pack_ref_deep($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+50

    // write_back[fungible_asset::Supply@]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+50
    $1_fungible_asset_Supply_$memory := $ResourceUpdate($1_fungible_asset_Supply_$memory, $GlobalLocationAddress($t22),
        $Dereference($t22));

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    assume {:print "$at(2,25252,26069)"} true;
L8:

    // label L17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
    assume {:print "$at(2,26074,26075)"} true;
L17:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
    assume {:print "$at(2,26074,26075)"} true;
    return;

    // label L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
L18:

    // abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
    assume {:print "$at(2,26074,26075)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

    // label L21 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L21:

    // destroy($t11) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L20 at <internal>:1:1+10
    goto L20;

}

// fun fungible_asset::decrease_supply [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1075
procedure {:timeLimit 40} $1_fungible_asset_decrease_supply$verify(_$t0: $1_object_Object'#0', _$t1: int) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $Mutation ($1_fungible_asset_Supply);
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: bool;
    var $t11: $Mutation ($1_fungible_asset_ConcurrentSupply);
    var $t12: $Mutation ($1_aggregator_v2_Aggregator'u128');
    var $t13: int;
    var $t14: bool;
    var $t15: bool;
    var $t16: int;
    var $t17: int;
    var $t18: bool;
    var $t19: bool;
    var $t20: int;
    var $t21: int;
    var $t22: $Mutation ($1_fungible_asset_Supply);
    var $t23: int;
    var $t24: int;
    var $t25: bool;
    var $t26: int;
    var $t27: int;
    var $t28: int;
    var $t29: int;
    var $t30: int;
    var $t31: int;
    var $t32: int;
    var $t33: $Mutation (int);
    var $t0: $1_object_Object'#0';
    var $t1: int;
    var $temp_0'$1_aggregator_v2_Aggregator'u128'': $1_aggregator_v2_Aggregator'u128';
    var $temp_0'$1_fungible_asset_Supply': $1_fungible_asset_Supply;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume {:print "$at(2,25000,25001)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume $IsValid'u64'($t1);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume {:print "$track_local(56,9,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:600:5+1
    assume {:print "$track_local(56,9,1):", $t1} $t1 == $t1;

    // $t4 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:27+1
    assume {:print "$at(2,25125,25126)"} true;
    $t4 := 0;
    assume $IsValid'u64'($t4);

    // $t5 := !=($t1, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:24+2
    $t5 := !$IsEqual'u64'($t1, $t4);

    // if ($t5) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    if ($t5) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    assume {:print "$at(2,25107,25176)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
L0:

    // $t6 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:54+22
    assume {:print "$at(2,25152,25174)"} true;
    $t6 := 1;
    assume $IsValid'u64'($t6);

    // $t7 := error::invalid_argument($t6) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:30+47
    call $t7 := $1_error_invalid_argument($t6);
    if ($abort_flag) {
        assume {:print "$at(2,25128,25175)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    assume {:print "$at(2,25107,25176)"} true;
    assume {:print "$track_abort(56,9):", $t7} $t7 == $t7;

    // $t8 := move($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    $t8 := $t7;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:601:9+69
    goto L18;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:602:55+8
    assume {:print "$at(2,25232,25240)"} true;
L2:

    // $t9 := object::object_address<#0>($t0) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:602:32+32
    assume {:print "$at(2,25209,25241)"} true;
    call $t9 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,25209,25241)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_local[metadata_address]($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:602:13+16
    assume {:print "$track_local(56,9,2):", $t9} $t9 == $t9;

    // $t10 := exists<fungible_asset::ConcurrentSupply>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:13+6
    assume {:print "$at(2,25256,25262)"} true;
    $t10 := $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t9);

    // if ($t10) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    if ($t10) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:605:62+16
    assume {:print "$at(2,25363,25379)"} true;
L4:

    // $t11 := borrow_global<fungible_asset::ConcurrentSupply>($t9) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:605:26+17
    assume {:print "$at(2,25327,25344)"} true;
    if (!$ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t9)) {
        call $ExecFailureAbort();
    } else {
        $t11 := $Mutation($Global($t9), EmptyVec(), $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t9));
    }
    if ($abort_flag) {
        assume {:print "$at(2,25327,25344)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t12 := borrow_field<fungible_asset::ConcurrentSupply>.current($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:40+19
    assume {:print "$at(2,25443,25462)"} true;
    $t12 := $ChildMutation($t11, 0, $Dereference($t11)->$current);

    // $t13 := (u128)($t1) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:61+16
    call $t13 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,25464,25480)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t14 := opaque begin: aggregator_v2::try_sub<u128>($t12, $t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61

    // $t15 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    havoc $t15;

    // if ($t15) goto L21 else goto L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    if ($t15) { goto L21; } else { goto L19; }

    // label L20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
L20:

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume {:print "$at(2,25420,25481)"} true;
    assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    goto L18;

    // label L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
L19:

    // $t12 := havoc[mut]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume {:print "$at(2,25420,25481)"} true;
    havoc $temp_0'$1_aggregator_v2_Aggregator'u128'';
    $t12 := $UpdateMutation($t12, $temp_0'$1_aggregator_v2_Aggregator'u128'');

    // assume WellFormed($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($Dereference($t12));

    // assume WellFormed($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    assume $IsValid'bool'($t14);

    // $t14 := opaque end: aggregator_v2::try_sub<u128>($t12, $t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61

    // write_back[Reference($t11).current (aggregator_v2::Aggregator<u128>)]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    $t11 := $UpdateMutation($t11, $Update'$1_fungible_asset_ConcurrentSupply'_current($Dereference($t11), $Dereference($t12)));

    // write_back[fungible_asset::ConcurrentSupply@]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:608:17+61
    $1_fungible_asset_ConcurrentSupply_$memory := $ResourceUpdate($1_fungible_asset_ConcurrentSupply_$memory, $GlobalLocationAddress($t11),
        $Dereference($t11));

    // if ($t14) goto L6 else goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    assume {:print "$at(2,25395,25551)"} true;
    if ($t14) { goto L6; } else { goto L5; }

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
L6:

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    assume {:print "$at(2,25395,25551)"} true;
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:609:37+17
    assume {:print "$at(2,25519,25536)"} true;
L5:

    // $t16 := 20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:609:37+17
    assume {:print "$at(2,25519,25536)"} true;
    $t16 := 20;
    assume $IsValid'u64'($t16);

    // $t17 := error::out_of_range($t16) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:609:17+38
    call $t17 := $1_error_out_of_range($t16);
    if ($abort_flag) {
        assume {:print "$at(2,25499,25537)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    assume {:print "$at(2,25395,25551)"} true;
    assume {:print "$track_abort(56,9):", $t17} $t17 == $t17;

    // $t8 := move($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    $t8 := $t17;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:607:13+156
    goto L18;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    assume {:print "$at(2,25252,26069)"} true;
L7:

    // goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    assume {:print "$at(2,25252,26069)"} true;
    goto L8;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:611:35+16
    assume {:print "$at(2,25587,25603)"} true;
L3:

    // $t18 := exists<fungible_asset::Supply>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:611:20+6
    assume {:print "$at(2,25572,25578)"} true;
    $t18 := $ResourceExists($1_fungible_asset_Supply_$memory, $t9);

    // if ($t18) goto L10 else goto L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:611:16+501
    if ($t18) { goto L10; } else { goto L9; }

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:36+16
    assume {:print "$at(2,25643,25659)"} true;
L10:

    // $t19 := exists<fungible_asset::Supply>($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:21+6
    assume {:print "$at(2,25628,25634)"} true;
    $t19 := $ResourceExists($1_fungible_asset_Supply_$memory, $t9);

    // if ($t19) goto L12 else goto L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    if ($t19) { goto L12; } else { goto L11; }

    // label L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
L12:

    // goto L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    assume {:print "$at(2,25620,25698)"} true;
    goto L13;

    // label L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:72+17
L11:

    // $t20 := 21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:72+17
    assume {:print "$at(2,25679,25696)"} true;
    $t20 := 21;
    assume $IsValid'u64'($t20);

    // $t21 := error::not_found($t20) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:55+35
    call $t21 := $1_error_not_found($t20);
    if ($abort_flag) {
        assume {:print "$at(2,25662,25697)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    assume {:print "$at(2,25620,25698)"} true;
    assume {:print "$track_abort(56,9):", $t21} $t21 == $t21;

    // $t8 := move($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    $t8 := $t21;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:612:13+78
    goto L18;

    // label L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:613:52+16
    assume {:print "$at(2,25751,25767)"} true;
L13:

    // $t22 := borrow_global<fungible_asset::Supply>($t9) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:613:26+17
    assume {:print "$at(2,25725,25742)"} true;
    if (!$ResourceExists($1_fungible_asset_Supply_$memory, $t9)) {
        call $ExecFailureAbort();
    } else {
        $t22 := $Mutation($Global($t9), EmptyVec(), $ResourceValue($1_fungible_asset_Supply_$memory, $t9));
    }
    if ($abort_flag) {
        assume {:print "$at(2,25725,25742)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_local[supply#1]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:613:17+6
    $temp_0'$1_fungible_asset_Supply' := $Dereference($t22);
    assume {:print "$track_local(56,9,3):", $temp_0'$1_fungible_asset_Supply'} $temp_0'$1_fungible_asset_Supply' == $temp_0'$1_fungible_asset_Supply';

    // $t23 := get_field<fungible_asset::Supply>.current($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:615:17+14
    assume {:print "$at(2,25807,25821)"} true;
    $t23 := $Dereference($t22)->$current;

    // $t24 := (u128)($t1) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:615:35+16
    call $t24 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,25825,25841)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t25 := >=($t23, $t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:615:32+2
    call $t25 := $Ge($t23, $t24);

    // if ($t25) goto L15 else goto L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;
    if ($t25) { goto L15; } else { goto L14; }

    // label L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
L15:

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;
    goto L16;

    // label L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
L14:

    // assert Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($t22))), 1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    // data invariant at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    assume {:print "$at(15,530,554)"} true;
    assert {:msg "assert_failed(15,530,554): data invariant does not hold"}
      (LenVec($Dereference($t22)->$maximum->$vec) <= 1);

    // destroy($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;

    // $t26 := 20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:616:38+17
    assume {:print "$at(2,25880,25897)"} true;
    $t26 := 20;
    assume $IsValid'u64'($t26);

    // $t27 := error::invalid_state($t26) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:616:17+39
    call $t27 := $1_error_invalid_state($t26);
    if ($abort_flag) {
        assume {:print "$at(2,25859,25898)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    assume {:print "$at(2,25782,25912)"} true;
    assume {:print "$track_abort(56,9):", $t27} $t27 == $t27;

    // $t8 := move($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    $t8 := $t27;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:614:13+130
    goto L18;

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:45+17
    assume {:print "$at(2,26039,26056)"} true;
L9:

    // $t28 := 21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:45+17
    assume {:print "$at(2,26039,26056)"} true;
    $t28 := 21;
    assume $IsValid'u64'($t28);

    // $t29 := error::not_found($t28) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:28+35
    call $t29 := $1_error_not_found($t28);
    if ($abort_flag) {
        assume {:print "$at(2,26022,26057)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // trace_abort($t29) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:13+51
    assume {:print "$at(2,26007,26058)"} true;
    assume {:print "$track_abort(56,9):", $t29} $t29 == $t29;

    // $t8 := move($t29) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:13+51
    $t8 := $t29;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:620:13+51
    goto L18;

    // label L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:30+6
    assume {:print "$at(2,25943,25949)"} true;
L16:

    // $t30 := get_field<fungible_asset::Supply>.current($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:30+14
    assume {:print "$at(2,25943,25957)"} true;
    $t30 := $Dereference($t22)->$current;

    // $t31 := (u128)($t1) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:47+16
    call $t31 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,25960,25976)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t32 := -($t30, $t31) on_abort goto L18 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:45+1
    call $t32 := $Sub($t30, $t31);
    if ($abort_flag) {
        assume {:print "$at(2,25958,25959)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,9):", $t8} $t8 == $t8;
        goto L18;
    }

    // $t33 := borrow_field<fungible_asset::Supply>.current($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+14
    $t33 := $ChildMutation($t22, 0, $Dereference($t22)->$current);

    // write_ref($t33, $t32) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+50
    $t33 := $UpdateMutation($t33, $t32);

    // write_back[Reference($t22).current (u128)]($t33) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+50
    $t22 := $UpdateMutation($t22, $Update'$1_fungible_asset_Supply'_current($Dereference($t22), $Dereference($t33)));

    // assert Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($t22))), 1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    // data invariant at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    assume {:print "$at(15,530,554)"} true;
    assert {:msg "assert_failed(15,530,554): data invariant does not hold"}
      (LenVec($Dereference($t22)->$maximum->$vec) <= 1);

    // write_back[fungible_asset::Supply@]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:618:13+50
    assume {:print "$at(2,25926,25976)"} true;
    $1_fungible_asset_Supply_$memory := $ResourceUpdate($1_fungible_asset_Supply_$memory, $GlobalLocationAddress($t22),
        $Dereference($t22));

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:604:9+817
    assume {:print "$at(2,25252,26069)"} true;
L8:

    // label L17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
    assume {:print "$at(2,26074,26075)"} true;
L17:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
    assume {:print "$at(2,26074,26075)"} true;
    assert {:msg "assert_failed(2,26074,26075): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
    return;

    // label L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
L18:

    // abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:622:5+1
    assume {:print "$at(2,26074,26075)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

    // label L21 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L21:

    // destroy($t11) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L20 at <internal>:1:1+10
    goto L20;

}

// fun fungible_asset::deposit_internal<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+687
procedure {:inline 1} $1_fungible_asset_deposit_internal'#0'(_$t0: $1_object_Object'#0', _$t1: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $Mutation ($1_fungible_asset_FungibleStore);
    var $t5: int;
    var $t6: $1_object_Object'$1_fungible_asset_Metadata';
    var $t7: $1_object_Object'$1_fungible_asset_Metadata';
    var $t8: $1_object_Object'$1_fungible_asset_Metadata';
    var $t9: int;
    var $t10: $1_fungible_asset_FungibleStore;
    var $t11: int;
    var $t12: $1_object_Object'$1_fungible_asset_Metadata';
    var $t13: int;
    var $t14: int;
    var $t15: bool;
    var $t16: $1_object_Object'$1_fungible_asset_Metadata';
    var $t17: int;
    var $t18: bool;
    var $t19: int;
    var $t20: int;
    var $t21: int;
    var $t22: $Mutation ($1_fungible_asset_FungibleStore);
    var $t23: int;
    var $t24: int;
    var $t25: $Mutation (int);
    var $t26: $Mutation ($1_fungible_asset_FungibleAssetEvents);
    var $t27: $Mutation ($1_event_EventHandle'$1_fungible_asset_DepositEvent');
    var $t28: $1_fungible_asset_DepositEvent;
    var $t0: $1_object_Object'#0';
    var $t1: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleStore': $1_fungible_asset_FungibleStore;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t7, select fungible_asset::FungibleAsset.metadata($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:413:9+27
    assume {:print "$at(3,17355,17382)"} true;
    assume ($t7 == $t1->$metadata);

    // assume Identical($t8, fungible_asset::$store_metadata<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:414:9+43
    assume {:print "$at(3,17391,17434)"} true;
    assume ($t8 == $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory, $t0));

    // assume Identical($t9, object::$object_address<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:417:9+47
    assume {:print "$at(3,17539,17586)"} true;
    assume ($t9 == $1_object_$object_address'#0'($t0));

    // assume Identical($t10, global<fungible_asset::FungibleStore>($t9)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:418:9+46
    assume {:print "$at(3,17595,17641)"} true;
    assume ($t10 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t9));

    // assume Identical($t11, select fungible_asset::FungibleAsset.amount($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:419:9+23
    assume {:print "$at(3,17650,17673)"} true;
    assume ($t11 == $t1->$amount);

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume {:print "$at(2,22304,22305)"} true;
    assume {:print "$track_local(56,11,0):", $t0} $t0 == $t0;

    // trace_local[fa]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume {:print "$track_local(56,11,1):", $t1} $t1 == $t1;

    // ($t12, $t13) := unpack fungible_asset::FungibleAsset($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:543:13+34
    assume {:print "$at(2,22428,22462)"} true;
    $t12 := $t1->$metadata;
    $t13 := $t1->$amount;

    // trace_local[amount]($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:543:39+6
    assume {:print "$track_local(56,11,2):", $t13} $t13 == $t13;

    // trace_local[metadata]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:543:29+8
    assume {:print "$track_local(56,11,3):", $t12} $t12 == $t12;

    // $t14 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:23+1
    assume {:print "$at(2,22491,22492)"} true;
    $t14 := 0;
    assume $IsValid'u64'($t14);

    // $t15 := ==($t13, $t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:20+2
    $t15 := $IsEqual'u64'($t13, $t14);

    // if ($t15) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:9+23
    if ($t15) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:26+6
L1:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:26+6
    assume {:print "$at(2,22494,22500)"} true;
    goto L5;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:546:45+5
    assume {:print "$at(2,22547,22552)"} true;
L0:

    // $t16 := fungible_asset::store_metadata<#0>($t0) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:546:30+21
    assume {:print "$at(2,22532,22553)"} true;
    call $t16 := $1_fungible_asset_store_metadata'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,22532,22553)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_local[store_metadata]($t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:546:13+14
    assume {:print "$track_local(56,11,6):", $t16} $t16 == $t16;

    // $t18 := ==($t12, $t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:26+2
    assume {:print "$at(2,22580,22582)"} true;
    $t18 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t12, $t16);

    // if ($t18) goto L3 else goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    if ($t18) { goto L3; } else { goto L2; }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
L3:

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    assume {:print "$at(2,22563,22659)"} true;
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:69+34
L2:

    // $t19 := 11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:69+34
    assume {:print "$at(2,22623,22657)"} true;
    $t19 := 11;
    assume $IsValid'u64'($t19);

    // $t20 := error::invalid_argument($t19) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:45+59
    call $t20 := $1_error_invalid_argument($t19);
    if ($abort_flag) {
        assume {:print "$at(2,22599,22658)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_abort($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    assume {:print "$at(2,22563,22659)"} true;
    assume {:print "$track_abort(56,11):", $t20} $t20 == $t20;

    // $t17 := move($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    $t17 := $t20;

    // goto L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    goto L6;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:548:49+6
    assume {:print "$at(2,22709,22715)"} true;
L4:

    // $t21 := object::object_address<#0>($t0) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:548:26+30
    assume {:print "$at(2,22686,22716)"} true;
    call $t21 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,22686,22716)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_local[store_addr]($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:548:13+10
    assume {:print "$track_local(56,11,5):", $t21} $t21 == $t21;

    // $t22 := borrow_global<fungible_asset::FungibleStore>($t21) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:549:21+17
    assume {:print "$at(2,22738,22755)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t21)) {
        call $ExecFailureAbort();
    } else {
        $t22 := $Mutation($Global($t21), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t21));
    }
    if ($abort_flag) {
        assume {:print "$at(2,22738,22755)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_local[store#1]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:549:13+5
    $temp_0'$1_fungible_asset_FungibleStore' := $Dereference($t22);
    assume {:print "$track_local(56,11,4):", $temp_0'$1_fungible_asset_FungibleStore'} $temp_0'$1_fungible_asset_FungibleStore' == $temp_0'$1_fungible_asset_FungibleStore';

    // $t23 := get_field<fungible_asset::FungibleStore>.balance($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:25+13
    assume {:print "$at(2,22808,22821)"} true;
    $t23 := $Dereference($t22)->$balance;

    // $t24 := +($t23, $t13) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:39+1
    call $t24 := $AddU64($t23, $t13);
    if ($abort_flag) {
        assume {:print "$at(2,22822,22823)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // $t25 := borrow_field<fungible_asset::FungibleStore>.balance($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+13
    $t25 := $ChildMutation($t22, 1, $Dereference($t22)->$balance);

    // write_ref($t25, $t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+38
    $t25 := $UpdateMutation($t25, $t24);

    // write_back[Reference($t22).balance (u64)]($t25) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+38
    $t22 := $UpdateMutation($t22, $Update'$1_fungible_asset_FungibleStore'_balance($Dereference($t22), $Dereference($t25)));

    // write_back[fungible_asset::FungibleStore@]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+38
    $1_fungible_asset_FungibleStore_$memory := $ResourceUpdate($1_fungible_asset_FungibleStore_$memory, $GlobalLocationAddress($t22),
        $Dereference($t22));

    // $t26 := borrow_global<fungible_asset::FungibleAssetEvents>($t21) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:552:22+17
    assume {:print "$at(2,22854,22871)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t21)) {
        call $ExecFailureAbort();
    } else {
        $t26 := $Mutation($Global($t21), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $t21));
    }
    if ($abort_flag) {
        assume {:print "$at(2,22854,22871)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // $t27 := borrow_field<fungible_asset::FungibleAssetEvents>.deposit_events($t26) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:27+26
    assume {:print "$at(2,22932,22958)"} true;
    $t27 := $ChildMutation($t26, 0, $Dereference($t26)->$deposit_events);

    // $t28 := pack fungible_asset::DepositEvent($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:55+23
    $t28 := $1_fungible_asset_DepositEvent($t13);

    // opaque begin: event::emit_event<fungible_asset::DepositEvent>($t27, $t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70

    // opaque end: event::emit_event<fungible_asset::DepositEvent>($t27, $t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70

    // write_back[Reference($t26).deposit_events (event::EventHandle<fungible_asset::DepositEvent>)]($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70
    $t26 := $UpdateMutation($t26, $Update'$1_fungible_asset_FungibleAssetEvents'_deposit_events($Dereference($t26), $Dereference($t27)));

    // write_back[fungible_asset::FungibleAssetEvents@]($t26) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70
    $1_fungible_asset_FungibleAssetEvents_$memory := $ResourceUpdate($1_fungible_asset_FungibleAssetEvents_$memory, $GlobalLocationAddress($t26),
        $Dereference($t26));

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:554:5+1
    assume {:print "$at(2,22990,22991)"} true;
L5:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:554:5+1
    assume {:print "$at(2,22990,22991)"} true;
    return;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:554:5+1
L6:

    // abort($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:554:5+1
    assume {:print "$at(2,22990,22991)"} true;
    $abort_code := $t17;
    $abort_flag := true;
    return;

}

// fun fungible_asset::deposit_internal [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+687
procedure {:timeLimit 40} $1_fungible_asset_deposit_internal$verify(_$t0: $1_object_Object'#0', _$t1: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $Mutation ($1_fungible_asset_FungibleStore);
    var $t5: int;
    var $t6: $1_object_Object'$1_fungible_asset_Metadata';
    var $t7: $1_object_Object'$1_fungible_asset_Metadata';
    var $t8: $1_object_Object'$1_fungible_asset_Metadata';
    var $t9: int;
    var $t10: $1_fungible_asset_FungibleStore;
    var $t11: int;
    var $t12: $1_object_Object'$1_fungible_asset_Metadata';
    var $t13: int;
    var $t14: int;
    var $t15: bool;
    var $t16: $1_object_Object'$1_fungible_asset_Metadata';
    var $t17: int;
    var $t18: bool;
    var $t19: int;
    var $t20: int;
    var $t21: int;
    var $t22: $Mutation ($1_fungible_asset_FungibleStore);
    var $t23: int;
    var $t24: int;
    var $t25: $Mutation (int);
    var $t26: $Mutation ($1_fungible_asset_FungibleAssetEvents);
    var $t27: $Mutation ($1_event_EventHandle'$1_fungible_asset_DepositEvent');
    var $t28: $1_fungible_asset_DepositEvent;
    var $t29: int;
    var $t0: $1_object_Object'#0';
    var $t1: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleStore': $1_fungible_asset_FungibleStore;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume {:print "$at(2,22304,22305)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t1);

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume Identical($t7, select fungible_asset::FungibleAsset.metadata($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:413:9+27
    assume {:print "$at(3,17355,17382)"} true;
    assume ($t7 == $t1->$metadata);

    // assume Identical($t8, fungible_asset::$store_metadata<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:414:9+43
    assume {:print "$at(3,17391,17434)"} true;
    assume ($t8 == $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory, $t0));

    // assume Identical($t9, object::$object_address<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:417:9+47
    assume {:print "$at(3,17539,17586)"} true;
    assume ($t9 == $1_object_$object_address'#0'($t0));

    // assume Identical($t10, global<fungible_asset::FungibleStore>($t9)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:418:9+46
    assume {:print "$at(3,17595,17641)"} true;
    assume ($t10 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t9));

    // assume Identical($t11, select fungible_asset::FungibleAsset.amount($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:419:9+23
    assume {:print "$at(3,17650,17673)"} true;
    assume ($t11 == $t1->$amount);

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume {:print "$at(2,22304,22305)"} true;
    assume {:print "$track_local(56,11,0):", $t0} $t0 == $t0;

    // trace_local[fa]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:542:5+1
    assume {:print "$track_local(56,11,1):", $t1} $t1 == $t1;

    // ($t12, $t13) := unpack fungible_asset::FungibleAsset($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:543:13+34
    assume {:print "$at(2,22428,22462)"} true;
    $t12 := $t1->$metadata;
    $t13 := $t1->$amount;

    // trace_local[amount]($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:543:39+6
    assume {:print "$track_local(56,11,2):", $t13} $t13 == $t13;

    // trace_local[metadata]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:543:29+8
    assume {:print "$track_local(56,11,3):", $t12} $t12 == $t12;

    // $t14 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:23+1
    assume {:print "$at(2,22491,22492)"} true;
    $t14 := 0;
    assume $IsValid'u64'($t14);

    // $t15 := ==($t13, $t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:20+2
    $t15 := $IsEqual'u64'($t13, $t14);

    // if ($t15) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:9+23
    if ($t15) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:26+6
L1:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:544:26+6
    assume {:print "$at(2,22494,22500)"} true;
    goto L5;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:546:45+5
    assume {:print "$at(2,22547,22552)"} true;
L0:

    // $t16 := fungible_asset::store_metadata<#0>($t0) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:546:30+21
    assume {:print "$at(2,22532,22553)"} true;
    call $t16 := $1_fungible_asset_store_metadata'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,22532,22553)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_local[store_metadata]($t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:546:13+14
    assume {:print "$track_local(56,11,6):", $t16} $t16 == $t16;

    // $t18 := ==($t12, $t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:26+2
    assume {:print "$at(2,22580,22582)"} true;
    $t18 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t12, $t16);

    // if ($t18) goto L3 else goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    if ($t18) { goto L3; } else { goto L2; }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
L3:

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    assume {:print "$at(2,22563,22659)"} true;
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:69+34
L2:

    // $t19 := 11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:69+34
    assume {:print "$at(2,22623,22657)"} true;
    $t19 := 11;
    assume $IsValid'u64'($t19);

    // $t20 := error::invalid_argument($t19) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:45+59
    call $t20 := $1_error_invalid_argument($t19);
    if ($abort_flag) {
        assume {:print "$at(2,22599,22658)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_abort($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    assume {:print "$at(2,22563,22659)"} true;
    assume {:print "$track_abort(56,11):", $t20} $t20 == $t20;

    // $t17 := move($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    $t17 := $t20;

    // goto L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:547:9+96
    goto L6;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:548:49+6
    assume {:print "$at(2,22709,22715)"} true;
L4:

    // $t21 := object::object_address<#0>($t0) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:548:26+30
    assume {:print "$at(2,22686,22716)"} true;
    call $t21 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,22686,22716)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_local[store_addr]($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:548:13+10
    assume {:print "$track_local(56,11,5):", $t21} $t21 == $t21;

    // $t22 := borrow_global<fungible_asset::FungibleStore>($t21) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:549:21+17
    assume {:print "$at(2,22738,22755)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t21)) {
        call $ExecFailureAbort();
    } else {
        $t22 := $Mutation($Global($t21), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t21));
    }
    if ($abort_flag) {
        assume {:print "$at(2,22738,22755)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // trace_local[store#1]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:549:13+5
    $temp_0'$1_fungible_asset_FungibleStore' := $Dereference($t22);
    assume {:print "$track_local(56,11,4):", $temp_0'$1_fungible_asset_FungibleStore'} $temp_0'$1_fungible_asset_FungibleStore' == $temp_0'$1_fungible_asset_FungibleStore';

    // $t23 := get_field<fungible_asset::FungibleStore>.balance($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:25+13
    assume {:print "$at(2,22808,22821)"} true;
    $t23 := $Dereference($t22)->$balance;

    // $t24 := +($t23, $t13) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:39+1
    call $t24 := $AddU64($t23, $t13);
    if ($abort_flag) {
        assume {:print "$at(2,22822,22823)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // $t25 := borrow_field<fungible_asset::FungibleStore>.balance($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+13
    $t25 := $ChildMutation($t22, 1, $Dereference($t22)->$balance);

    // write_ref($t25, $t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+38
    $t25 := $UpdateMutation($t25, $t24);

    // write_back[Reference($t22).balance (u64)]($t25) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+38
    $t22 := $UpdateMutation($t22, $Update'$1_fungible_asset_FungibleStore'_balance($Dereference($t22), $Dereference($t25)));

    // write_back[fungible_asset::FungibleStore@]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:550:9+38
    $1_fungible_asset_FungibleStore_$memory := $ResourceUpdate($1_fungible_asset_FungibleStore_$memory, $GlobalLocationAddress($t22),
        $Dereference($t22));

    // $t26 := borrow_global<fungible_asset::FungibleAssetEvents>($t21) on_abort goto L6 with $t17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:552:22+17
    assume {:print "$at(2,22854,22871)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t21)) {
        call $ExecFailureAbort();
    } else {
        $t26 := $Mutation($Global($t21), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $t21));
    }
    if ($abort_flag) {
        assume {:print "$at(2,22854,22871)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(56,11):", $t17} $t17 == $t17;
        goto L6;
    }

    // $t27 := borrow_field<fungible_asset::FungibleAssetEvents>.deposit_events($t26) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:27+26
    assume {:print "$at(2,22932,22958)"} true;
    $t27 := $ChildMutation($t26, 0, $Dereference($t26)->$deposit_events);

    // $t28 := pack fungible_asset::DepositEvent($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:55+23
    $t28 := $1_fungible_asset_DepositEvent($t13);

    // opaque begin: event::emit_event<fungible_asset::DepositEvent>($t27, $t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70

    // opaque end: event::emit_event<fungible_asset::DepositEvent>($t27, $t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70

    // write_back[Reference($t26).deposit_events (event::EventHandle<fungible_asset::DepositEvent>)]($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70
    $t26 := $UpdateMutation($t26, $Update'$1_fungible_asset_FungibleAssetEvents'_deposit_events($Dereference($t26), $Dereference($t27)));

    // write_back[fungible_asset::FungibleAssetEvents@]($t26) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:553:9+70
    $1_fungible_asset_FungibleAssetEvents_$memory := $ResourceUpdate($1_fungible_asset_FungibleAssetEvents_$memory, $GlobalLocationAddress($t26),
        $Dereference($t26));

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:554:5+1
    assume {:print "$at(2,22990,22991)"} true;
L5:

    // assume Identical($t29, select fungible_asset::FungibleStore.balance(global<fungible_asset::FungibleStore>($t9))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:420:9+67
    assume {:print "$at(3,17682,17749)"} true;
    assume ($t29 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t9)->$balance);

    // assert Not(And(Neq<u64>($t11, 0), Neq<object::Object<fungible_asset::Metadata>>($t7, $t8))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:416:9+54
    assume {:print "$at(3,17476,17530)"} true;
    assert {:msg "assert_failed(3,17476,17530): function does not abort under this condition"}
      !(!$IsEqual'u64'($t11, 0) && !$IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t7, $t8));

    // assert Eq<u64>($t29, Add(select fungible_asset::FungibleStore.balance($t10), $t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:421:9+48
    assume {:print "$at(3,17758,17806)"} true;
    assert {:msg "assert_failed(3,17758,17806): post-condition does not hold"}
      $IsEqual'u64'($t29, ($t10->$balance + $t11));

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:421:9+48
    return;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:554:5+1
    assume {:print "$at(2,22990,22991)"} true;
L6:

    // abort($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:554:5+1
    assume {:print "$at(2,22990,22991)"} true;
    $abort_code := $t17;
    $abort_flag := true;
    return;

}

// fun fungible_asset::deposit_with_ref<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+358
procedure {:inline 1} $1_fungible_asset_deposit_with_ref'#0'(_$t0: $1_fungible_asset_TransferRef, _$t1: $1_object_Object'#0', _$t2: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: $1_object_Object'$1_fungible_asset_Metadata';
    var $t10: $1_object_Object'$1_fungible_asset_Metadata';
    var $t11: int;
    var $t12: $1_fungible_asset_FungibleStore;
    var $t13: int;
    var $t0: $1_fungible_asset_TransferRef;
    var $t1: $1_object_Object'#0';
    var $t2: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume {:print "$at(2,19957,19958)"} true;
    assume {:print "$track_local(56,12,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume {:print "$track_local(56,12,1):", $t1} $t1 == $t1;

    // trace_local[fa]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume {:print "$track_local(56,12,2):", $t2} $t2 == $t2;

    // $t3 := get_field<fungible_asset::TransferRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:492:13+12
    assume {:print "$at(2,20154,20166)"} true;
    $t3 := $t0->$metadata;

    // $t4 := get_field<fungible_asset::FungibleAsset>.metadata($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:492:29+11
    $t4 := $t2->$metadata;

    // $t5 := ==($t3, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:492:26+2
    $t5 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t3, $t4);

    // if ($t5) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    assume {:print "$at(2,20133,20271)"} true;
    if ($t5) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    assume {:print "$at(2,20133,20271)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:493:37+41
    assume {:print "$at(2,20219,20260)"} true;
L0:

    // $t6 := 2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:493:37+41
    assume {:print "$at(2,20219,20260)"} true;
    $t6 := 2;
    assume $IsValid'u64'($t6);

    // $t7 := error::invalid_argument($t6) on_abort goto L4 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:493:13+66
    call $t7 := $1_error_invalid_argument($t6);
    if ($abort_flag) {
        assume {:print "$at(2,20195,20261)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,12):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    assume {:print "$at(2,20133,20271)"} true;
    assume {:print "$track_abort(56,12):", $t7} $t7 == $t7;

    // $t8 := move($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    $t8 := $t7;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:495:26+5
    assume {:print "$at(2,20298,20303)"} true;
L2:

    // assume Identical($t9, select fungible_asset::FungibleAsset.metadata($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:413:9+27
    assume {:print "$at(3,17355,17382)"} true;
    assume ($t9 == $t2->$metadata);

    // assume Identical($t10, fungible_asset::$store_metadata<#0>($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:414:9+43
    assume {:print "$at(3,17391,17434)"} true;
    assume ($t10 == $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory, $t1));

    // assume Identical($t11, object::$object_address<#0>($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:417:9+47
    assume {:print "$at(3,17539,17586)"} true;
    assume ($t11 == $1_object_$object_address'#0'($t1));

    // assume Identical($t12, global<fungible_asset::FungibleStore>($t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:418:9+46
    assume {:print "$at(3,17595,17641)"} true;
    assume ($t12 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t11));

    // assume Identical($t13, select fungible_asset::FungibleAsset.amount($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:419:9+23
    assume {:print "$at(3,17650,17673)"} true;
    assume ($t13 == $t2->$amount);

    // fungible_asset::deposit_internal<#0>($t1, $t2) on_abort goto L4 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:495:9+27
    assume {:print "$at(2,20281,20308)"} true;
    call $1_fungible_asset_deposit_internal'#0'($t1, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,20281,20308)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,12):", $t8} $t8 == $t8;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:496:5+1
    assume {:print "$at(2,20314,20315)"} true;
L3:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:496:5+1
    assume {:print "$at(2,20314,20315)"} true;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:496:5+1
L4:

    // abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:496:5+1
    assume {:print "$at(2,20314,20315)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun fungible_asset::deposit_with_ref [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+358
procedure {:timeLimit 40} $1_fungible_asset_deposit_with_ref$verify(_$t0: $1_fungible_asset_TransferRef, _$t1: $1_object_Object'#0', _$t2: $1_fungible_asset_FungibleAsset) returns ()
{
    // declare local variables
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: $1_object_Object'$1_fungible_asset_Metadata';
    var $t10: $1_object_Object'$1_fungible_asset_Metadata';
    var $t11: int;
    var $t12: $1_fungible_asset_FungibleStore;
    var $t13: int;
    var $t0: $1_fungible_asset_TransferRef;
    var $t1: $1_object_Object'#0';
    var $t2: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume {:print "$at(2,19957,19958)"} true;
    assume $IsValid'$1_fungible_asset_TransferRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t2);

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume {:print "$track_local(56,12,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume {:print "$track_local(56,12,1):", $t1} $t1 == $t1;

    // trace_local[fa]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:486:5+1
    assume {:print "$track_local(56,12,2):", $t2} $t2 == $t2;

    // $t3 := get_field<fungible_asset::TransferRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:492:13+12
    assume {:print "$at(2,20154,20166)"} true;
    $t3 := $t0->$metadata;

    // $t4 := get_field<fungible_asset::FungibleAsset>.metadata($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:492:29+11
    $t4 := $t2->$metadata;

    // $t5 := ==($t3, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:492:26+2
    $t5 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t3, $t4);

    // if ($t5) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    assume {:print "$at(2,20133,20271)"} true;
    if ($t5) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    assume {:print "$at(2,20133,20271)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:493:37+41
    assume {:print "$at(2,20219,20260)"} true;
L0:

    // $t6 := 2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:493:37+41
    assume {:print "$at(2,20219,20260)"} true;
    $t6 := 2;
    assume $IsValid'u64'($t6);

    // $t7 := error::invalid_argument($t6) on_abort goto L4 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:493:13+66
    call $t7 := $1_error_invalid_argument($t6);
    if ($abort_flag) {
        assume {:print "$at(2,20195,20261)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,12):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    assume {:print "$at(2,20133,20271)"} true;
    assume {:print "$track_abort(56,12):", $t7} $t7 == $t7;

    // $t8 := move($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    $t8 := $t7;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:491:9+138
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:495:26+5
    assume {:print "$at(2,20298,20303)"} true;
L2:

    // assume Identical($t9, select fungible_asset::FungibleAsset.metadata($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:413:9+27
    assume {:print "$at(3,17355,17382)"} true;
    assume ($t9 == $t2->$metadata);

    // assume Identical($t10, fungible_asset::$store_metadata<#0>($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:414:9+43
    assume {:print "$at(3,17391,17434)"} true;
    assume ($t10 == $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory, $t1));

    // assume Identical($t11, object::$object_address<#0>($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:417:9+47
    assume {:print "$at(3,17539,17586)"} true;
    assume ($t11 == $1_object_$object_address'#0'($t1));

    // assume Identical($t12, global<fungible_asset::FungibleStore>($t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:418:9+46
    assume {:print "$at(3,17595,17641)"} true;
    assume ($t12 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t11));

    // assume Identical($t13, select fungible_asset::FungibleAsset.amount($t2)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:419:9+23
    assume {:print "$at(3,17650,17673)"} true;
    assume ($t13 == $t2->$amount);

    // fungible_asset::deposit_internal<#0>($t1, $t2) on_abort goto L4 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:495:9+27
    assume {:print "$at(2,20281,20308)"} true;
    call $1_fungible_asset_deposit_internal'#0'($t1, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,20281,20308)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,12):", $t8} $t8 == $t8;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:496:5+1
    assume {:print "$at(2,20314,20315)"} true;
L3:

    // assert Not(Neq<object::Object<fungible_asset::Metadata>>(select fungible_asset::TransferRef.metadata($t0), select fungible_asset::FungibleAsset.metadata($t2))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:382:9+40
    assume {:print "$at(3,16525,16565)"} true;
    assert {:msg "assert_failed(3,16525,16565): function does not abort under this condition"}
      !!$IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t0->$metadata, $t2->$metadata);

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:382:9+40
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:496:5+1
    assume {:print "$at(2,20314,20315)"} true;
L4:

    // abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:496:5+1
    assume {:print "$at(2,20314,20315)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun fungible_asset::generate_burn_ref [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:220:5+195
procedure {:timeLimit 40} $1_fungible_asset_generate_burn_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_fungible_asset_BurnRef)
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t4: $1_fungible_asset_BurnRef;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_fungible_asset_BurnRef': $1_fungible_asset_BurnRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $1_object_ObjectCore_$memory#43: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:220:5+1
    assume {:print "$at(2,9386,9387)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:220:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume Identical($t1, object::$address_from_constructor_ref($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:185:9+80
    assume {:print "$at(3,10636,10716)"} true;
    assume ($t1 == $1_object_$address_from_constructor_ref($t0));

    // @43 := save_mem(object::ObjectCore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:220:5+1
    assume {:print "$at(2,9386,9387)"} true;
    $1_object_ObjectCore_$memory#43 := $1_object_ObjectCore_$memory;

    // trace_local[constructor_ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:220:5+1
    assume {:print "$track_local(56,15,0):", $t0} $t0 == $t0;

    // $t2 := object::object_from_constructor_ref<fungible_asset::Metadata>($t0) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:221:24+62
    assume {:print "$at(2,9483,9545)"} true;
    call $t2 := $1_object_object_from_constructor_ref'$1_fungible_asset_Metadata'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,9483,9545)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,15):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t4 := pack fungible_asset::BurnRef($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:222:9+20
    assume {:print "$at(2,9555,9575)"} true;
    $t4 := $1_fungible_asset_BurnRef($t2);

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:222:9+20
    assume {:print "$track_return(56,15,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:223:5+1
    assume {:print "$at(2,9580,9581)"} true;
L1:

    // assert Not(Not(exists[@43]<object::ObjectCore>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:186:9+59
    assume {:print "$at(3,10725,10784)"} true;
    assert {:msg "assert_failed(3,10725,10784): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#43, $t1);

    // assert Not(Not(object::spec_exists_at[]<fungible_asset::Metadata>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:187:9+65
    assume {:print "$at(3,10793,10858)"} true;
    assert {:msg "assert_failed(3,10793,10858): function does not abort under this condition"}
      !!$1_object_spec_exists_at'$1_fungible_asset_Metadata'($t1);

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:187:9+65
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:223:5+1
    assume {:print "$at(2,9580,9581)"} true;
L2:

    // assert Or(Not(exists[@43]<object::ObjectCore>($t1)), Not(object::spec_exists_at[]<fungible_asset::Metadata>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:184:5+304
    assume {:print "$at(3,10560,10864)"} true;
    assert {:msg "assert_failed(3,10560,10864): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#43, $t1) || !$1_object_spec_exists_at'$1_fungible_asset_Metadata'($t1));

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:184:5+304
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun fungible_asset::generate_mint_ref [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:213:5+195
procedure {:timeLimit 40} $1_fungible_asset_generate_mint_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_fungible_asset_MintRef)
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'$1_fungible_asset_Metadata';
    var $t3: int;
    var $t4: $1_fungible_asset_MintRef;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_fungible_asset_MintRef': $1_fungible_asset_MintRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $1_object_ObjectCore_$memory#42: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:213:5+1
    assume {:print "$at(2,8968,8969)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:213:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume Identical($t1, object::$address_from_constructor_ref($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:179:9+80
    assume {:print "$at(3,10326,10406)"} true;
    assume ($t1 == $1_object_$address_from_constructor_ref($t0));

    // @42 := save_mem(object::ObjectCore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:213:5+1
    assume {:print "$at(2,8968,8969)"} true;
    $1_object_ObjectCore_$memory#42 := $1_object_ObjectCore_$memory;

    // trace_local[constructor_ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:213:5+1
    assume {:print "$track_local(56,16,0):", $t0} $t0 == $t0;

    // $t2 := object::object_from_constructor_ref<fungible_asset::Metadata>($t0) on_abort goto L2 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:214:24+62
    assume {:print "$at(2,9065,9127)"} true;
    call $t2 := $1_object_object_from_constructor_ref'$1_fungible_asset_Metadata'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,9065,9127)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,16):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t4 := pack fungible_asset::MintRef($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:215:9+20
    assume {:print "$at(2,9137,9157)"} true;
    $t4 := $1_fungible_asset_MintRef($t2);

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:215:9+20
    assume {:print "$track_return(56,16,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:216:5+1
    assume {:print "$at(2,9162,9163)"} true;
L1:

    // assert Not(Not(exists[@42]<object::ObjectCore>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:180:9+59
    assume {:print "$at(3,10415,10474)"} true;
    assert {:msg "assert_failed(3,10415,10474): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#42, $t1);

    // assert Not(Not(object::spec_exists_at[]<fungible_asset::Metadata>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:181:9+65
    assume {:print "$at(3,10483,10548)"} true;
    assert {:msg "assert_failed(3,10483,10548): function does not abort under this condition"}
      !!$1_object_spec_exists_at'$1_fungible_asset_Metadata'($t1);

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:181:9+65
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:216:5+1
    assume {:print "$at(2,9162,9163)"} true;
L2:

    // assert Or(Not(exists[@42]<object::ObjectCore>($t1)), Not(object::spec_exists_at[]<fungible_asset::Metadata>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:178:5+304
    assume {:print "$at(3,10250,10554)"} true;
    assert {:msg "assert_failed(3,10250,10554): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#42, $t1) || !$1_object_spec_exists_at'$1_fungible_asset_Metadata'($t1));

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:178:5+304
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun fungible_asset::increase_supply<fungible_asset::Metadata> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1143
procedure {:inline 1} $1_fungible_asset_increase_supply'$1_fungible_asset_Metadata'(_$t0: $1_object_Object'$1_fungible_asset_Metadata', _$t1: int) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $Mutation ($1_fungible_asset_Supply);
    var $t4: int;
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: bool;
    var $t12: $Mutation ($1_fungible_asset_ConcurrentSupply);
    var $t13: $Mutation ($1_aggregator_v2_Aggregator'u128');
    var $t14: int;
    var $t15: bool;
    var $t16: bool;
    var $t17: int;
    var $t18: int;
    var $t19: bool;
    var $t20: $Mutation ($1_fungible_asset_Supply);
    var $t21: $1_option_Option'u128';
    var $t22: bool;
    var $t23: $Mutation ($1_option_Option'u128');
    var $t24: $Mutation (int);
    var $t25: int;
    var $t26: int;
    var $t27: int;
    var $t28: int;
    var $t29: bool;
    var $t30: int;
    var $t31: int;
    var $t32: int;
    var $t33: int;
    var $t34: int;
    var $t35: int;
    var $t36: int;
    var $t37: $Mutation (int);
    var $t0: $1_object_Object'$1_fungible_asset_Metadata';
    var $t1: int;
    var $temp_0'$1_aggregator_v2_Aggregator'u128'': $1_aggregator_v2_Aggregator'u128';
    var $temp_0'$1_fungible_asset_Supply': $1_fungible_asset_Supply;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t4, object::$object_address<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:445:9+56
    assume {:print "$at(3,18524,18580)"} true;
    assume ($t4 == $1_object_$object_address'$1_fungible_asset_Metadata'($t0));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume {:print "$at(2,23791,23792)"} true;
    assume {:print "$track_local(56,18,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume {:print "$track_local(56,18,1):", $t1} $t1 == $t1;

    // $t5 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:27+1
    assume {:print "$at(2,23916,23917)"} true;
    $t5 := 0;
    assume $IsValid'u64'($t5);

    // $t6 := !=($t1, $t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:24+2
    $t6 := !$IsEqual'u64'($t1, $t5);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    assume {:print "$at(2,23898,23967)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
L0:

    // $t7 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:54+22
    assume {:print "$at(2,23943,23965)"} true;
    $t7 := 1;
    assume $IsValid'u64'($t7);

    // $t8 := error::invalid_argument($t7) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:30+47
    call $t8 := $1_error_invalid_argument($t7);
    if ($abort_flag) {
        assume {:print "$at(2,23919,23966)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    assume {:print "$at(2,23898,23967)"} true;
    assume {:print "$track_abort(56,18):", $t8} $t8 == $t8;

    // $t9 := move($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    $t9 := $t8;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    goto L18;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:576:55+8
    assume {:print "$at(2,24023,24031)"} true;
L2:

    // $t10 := object::object_address<#0>($t0) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:576:32+32
    assume {:print "$at(2,24000,24032)"} true;
    call $t10 := $1_object_object_address'$1_fungible_asset_Metadata'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,24000,24032)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_local[metadata_address]($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:576:13+16
    assume {:print "$track_local(56,18,2):", $t10} $t10 == $t10;

    // $t11 := exists<fungible_asset::ConcurrentSupply>($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:13+6
    assume {:print "$at(2,24047,24053)"} true;
    $t11 := $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t10);

    // if ($t11) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    if ($t11) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:579:62+16
    assume {:print "$at(2,24154,24170)"} true;
L4:

    // $t12 := borrow_global<fungible_asset::ConcurrentSupply>($t10) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:579:26+17
    assume {:print "$at(2,24118,24135)"} true;
    if (!$ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t12 := $Mutation($Global($t10), EmptyVec(), $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t10));
    }
    if ($abort_flag) {
        assume {:print "$at(2,24118,24135)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t13 := borrow_field<fungible_asset::ConcurrentSupply>.current($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:40+19
    assume {:print "$at(2,24233,24252)"} true;
    $t13 := $ChildMutation($t12, 0, $Dereference($t12)->$current);

    // $t14 := (u128)($t1) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:61+16
    call $t14 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,24254,24270)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t15 := opaque begin: aggregator_v2::try_add<u128>($t13, $t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61

    // $t16 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    havoc $t16;

    // if ($t16) goto L21 else goto L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    if ($t16) { goto L21; } else { goto L19; }

    // label L20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
L20:

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume {:print "$at(2,24210,24271)"} true;
    assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    goto L18;

    // label L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
L19:

    // $t13 := havoc[mut]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume {:print "$at(2,24210,24271)"} true;
    havoc $temp_0'$1_aggregator_v2_Aggregator'u128'';
    $t13 := $UpdateMutation($t13, $temp_0'$1_aggregator_v2_Aggregator'u128'');

    // assume WellFormed($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($Dereference($t13));

    // assume WellFormed($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume $IsValid'bool'($t15);

    // $t15 := opaque end: aggregator_v2::try_add<u128>($t13, $t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61

    // write_back[Reference($t12).current (aggregator_v2::Aggregator<u128>)]($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    $t12 := $UpdateMutation($t12, $Update'$1_fungible_asset_ConcurrentSupply'_current($Dereference($t12), $Dereference($t13)));

    // write_back[fungible_asset::ConcurrentSupply@]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    $1_fungible_asset_ConcurrentSupply_$memory := $ResourceUpdate($1_fungible_asset_ConcurrentSupply_$memory, $GlobalLocationAddress($t12),
        $Dereference($t12));

    // if ($t15) goto L6 else goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    assume {:print "$at(2,24185,24344)"} true;
    if ($t15) { goto L6; } else { goto L5; }

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
L6:

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    assume {:print "$at(2,24185,24344)"} true;
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:582:37+20
    assume {:print "$at(2,24309,24329)"} true;
L5:

    // $t17 := 5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:582:37+20
    assume {:print "$at(2,24309,24329)"} true;
    $t17 := 5;
    assume $IsValid'u64'($t17);

    // $t18 := error::out_of_range($t17) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:582:17+41
    call $t18 := $1_error_out_of_range($t17);
    if ($abort_flag) {
        assume {:print "$at(2,24289,24330)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    assume {:print "$at(2,24185,24344)"} true;
    assume {:print "$track_abort(56,18):", $t18} $t18 == $t18;

    // $t9 := move($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    $t9 := $t18;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    goto L18;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    assume {:print "$at(2,24043,24928)"} true;
L7:

    // goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    assume {:print "$at(2,24043,24928)"} true;
    goto L8;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:584:35+16
    assume {:print "$at(2,24380,24396)"} true;
L3:

    // $t19 := exists<fungible_asset::Supply>($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:584:20+6
    assume {:print "$at(2,24365,24371)"} true;
    $t19 := $ResourceExists($1_fungible_asset_Supply_$memory, $t10);

    // if ($t19) goto L10 else goto L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:584:16+567
    if ($t19) { goto L10; } else { goto L9; }

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:585:52+16
    assume {:print "$at(2,24452,24468)"} true;
L10:

    // $t20 := borrow_global<fungible_asset::Supply>($t10) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:585:26+17
    assume {:print "$at(2,24426,24443)"} true;
    if (!$ResourceExists($1_fungible_asset_Supply_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t20 := $Mutation($Global($t10), EmptyVec(), $ResourceValue($1_fungible_asset_Supply_$memory, $t10));
    }
    if ($abort_flag) {
        assume {:print "$at(2,24426,24443)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_local[supply#1]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:585:17+6
    $temp_0'$1_fungible_asset_Supply' := $Dereference($t20);
    assume {:print "$track_local(56,18,3):", $temp_0'$1_fungible_asset_Supply'} $temp_0'$1_fungible_asset_Supply' == $temp_0'$1_fungible_asset_Supply';

    // $t21 := get_field<fungible_asset::Supply>.maximum($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:33+15
    assume {:print "$at(2,24503,24518)"} true;
    $t21 := $Dereference($t20)->$maximum;

    // $t22 := opaque begin: option::is_some<u128>($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32

    // assume WellFormed($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32
    assume $IsValid'bool'($t22);

    // assume Eq<bool>($t22, option::spec_is_some<u128>($t21)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32
    assume $IsEqual'bool'($t22, $1_option_spec_is_some'u128'($t21));

    // $t22 := opaque end: option::is_some<u128>($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32

    // if ($t22) goto L12 else goto L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:13+288
    if ($t22) { goto L12; } else { goto L11; }

    // label L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:52+6
    assume {:print "$at(2,24574,24580)"} true;
L12:

    // $t23 := borrow_field<fungible_asset::Supply>.maximum($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:47+19
    assume {:print "$at(2,24569,24588)"} true;
    $t23 := $ChildMutation($t20, 1, $Dereference($t20)->$maximum);

    // $t24 := option::borrow_mut<u128>($t23) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:28+39
    call $t24,$t23 := $1_option_borrow_mut'u128'($t23);
    if ($abort_flag) {
        assume {:print "$at(2,24550,24589)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t25 := read_ref($t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:27+40
    $t25 := $Dereference($t24);

    // write_back[Reference($t20).maximum (option::Option<u128>)]($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:27+40
    $t20 := $UpdateMutation($t20, $Update'$1_fungible_asset_Supply'_maximum($Dereference($t20), $Dereference($t23)));

    // $t26 := get_field<fungible_asset::Supply>.current($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:27+14
    assume {:print "$at(2,24642,24656)"} true;
    $t26 := $Dereference($t20)->$current;

    // $t27 := -($t25, $t26) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:25+1
    call $t27 := $Sub($t25, $t26);
    if ($abort_flag) {
        assume {:print "$at(2,24640,24641)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t28 := (u128)($t1) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:45+16
    call $t28 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,24660,24676)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t29 := >=($t27, $t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:42+2
    call $t29 := $Ge($t27, $t28);

    // if ($t29) goto L14 else goto L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    if ($t29) { goto L14; } else { goto L13; }

    // label L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
L14:

    // goto L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    goto L15;

    // label L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
L13:

    // pack_ref_deep($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;

    // write_back[fungible_asset::Supply@]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    $1_fungible_asset_Supply_$memory := $ResourceUpdate($1_fungible_asset_Supply_$memory, $GlobalLocationAddress($t20),
        $Dereference($t20));

    // destroy($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150

    // $t30 := 5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:590:41+20
    assume {:print "$at(2,24718,24738)"} true;
    $t30 := 5;
    assume $IsValid'u64'($t30);

    // $t31 := error::out_of_range($t30) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:590:21+41
    call $t31 := $1_error_out_of_range($t30);
    if ($abort_flag) {
        assume {:print "$at(2,24698,24739)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t31) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    assume {:print "$track_abort(56,18):", $t31} $t31 == $t31;

    // $t9 := move($t31) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    $t9 := $t31;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    goto L18;

    // label L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
L15:

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    goto L16;

    // label L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:13+288
    assume {:print "$at(2,24483,24771)"} true;
L11:

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:13+288
    assume {:print "$at(2,24483,24771)"} true;
    goto L16;

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:45+17
    assume {:print "$at(2,24898,24915)"} true;
L9:

    // $t32 := 21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:45+17
    assume {:print "$at(2,24898,24915)"} true;
    $t32 := 21;
    assume $IsValid'u64'($t32);

    // $t33 := error::not_found($t32) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:28+35
    call $t33 := $1_error_not_found($t32);
    if ($abort_flag) {
        assume {:print "$at(2,24881,24916)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t33) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:13+51
    assume {:print "$at(2,24866,24917)"} true;
    assume {:print "$track_abort(56,18):", $t33} $t33 == $t33;

    // $t9 := move($t33) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:13+51
    $t9 := $t33;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:13+51
    goto L18;

    // label L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:30+6
    assume {:print "$at(2,24802,24808)"} true;
L16:

    // $t34 := get_field<fungible_asset::Supply>.current($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:30+14
    assume {:print "$at(2,24802,24816)"} true;
    $t34 := $Dereference($t20)->$current;

    // $t35 := (u128)($t1) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:47+16
    call $t35 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,24819,24835)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t36 := +($t34, $t35) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:45+1
    call $t36 := $AddU128($t34, $t35);
    if ($abort_flag) {
        assume {:print "$at(2,24817,24818)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t37 := borrow_field<fungible_asset::Supply>.current($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+14
    $t37 := $ChildMutation($t20, 0, $Dereference($t20)->$current);

    // write_ref($t37, $t36) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+50
    $t37 := $UpdateMutation($t37, $t36);

    // write_back[Reference($t20).current (u128)]($t37) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+50
    $t20 := $UpdateMutation($t20, $Update'$1_fungible_asset_Supply'_current($Dereference($t20), $Dereference($t37)));

    // pack_ref_deep($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+50

    // write_back[fungible_asset::Supply@]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+50
    $1_fungible_asset_Supply_$memory := $ResourceUpdate($1_fungible_asset_Supply_$memory, $GlobalLocationAddress($t20),
        $Dereference($t20));

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    assume {:print "$at(2,24043,24928)"} true;
L8:

    // label L17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:597:5+1
    assume {:print "$at(2,24933,24934)"} true;
L17:

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:597:5+1
    assume {:print "$at(2,24933,24934)"} true;
    return;

    // label L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:597:5+1
L18:

    // abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:597:5+1
    assume {:print "$at(2,24933,24934)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

    // label L21 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L21:

    // destroy($t12) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L20 at <internal>:1:1+10
    goto L20;

}

// fun fungible_asset::increase_supply [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1143
procedure {:timeLimit 40} $1_fungible_asset_increase_supply$verify(_$t0: $1_object_Object'#0', _$t1: int) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $Mutation ($1_fungible_asset_Supply);
    var $t4: int;
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: bool;
    var $t12: $Mutation ($1_fungible_asset_ConcurrentSupply);
    var $t13: $Mutation ($1_aggregator_v2_Aggregator'u128');
    var $t14: int;
    var $t15: bool;
    var $t16: bool;
    var $t17: int;
    var $t18: int;
    var $t19: bool;
    var $t20: $Mutation ($1_fungible_asset_Supply);
    var $t21: $1_option_Option'u128';
    var $t22: bool;
    var $t23: $Mutation ($1_option_Option'u128');
    var $t24: $Mutation (int);
    var $t25: int;
    var $t26: int;
    var $t27: int;
    var $t28: int;
    var $t29: bool;
    var $t30: int;
    var $t31: int;
    var $t32: int;
    var $t33: int;
    var $t34: int;
    var $t35: int;
    var $t36: int;
    var $t37: $Mutation (int);
    var $t0: $1_object_Object'#0';
    var $t1: int;
    var $temp_0'$1_aggregator_v2_Aggregator'u128'': $1_aggregator_v2_Aggregator'u128';
    var $temp_0'$1_fungible_asset_Supply': $1_fungible_asset_Supply;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    var $1_fungible_asset_ConcurrentSupply_$memory#37: $Memory $1_fungible_asset_ConcurrentSupply;
    var $1_fungible_asset_Supply_$memory#38: $Memory $1_fungible_asset_Supply;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume {:print "$at(2,23791,23792)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume $IsValid'u64'($t1);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // assume Identical($t4, object::$object_address<#0>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:445:9+56
    assume {:print "$at(3,18524,18580)"} true;
    assume ($t4 == $1_object_$object_address'#0'($t0));

    // @37 := save_mem(fungible_asset::ConcurrentSupply) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume {:print "$at(2,23791,23792)"} true;
    $1_fungible_asset_ConcurrentSupply_$memory#37 := $1_fungible_asset_ConcurrentSupply_$memory;

    // @38 := save_mem(fungible_asset::Supply) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    $1_fungible_asset_Supply_$memory#38 := $1_fungible_asset_Supply_$memory;

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume {:print "$track_local(56,18,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:574:5+1
    assume {:print "$track_local(56,18,1):", $t1} $t1 == $t1;

    // $t5 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:27+1
    assume {:print "$at(2,23916,23917)"} true;
    $t5 := 0;
    assume $IsValid'u64'($t5);

    // $t6 := !=($t1, $t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:24+2
    $t6 := !$IsEqual'u64'($t1, $t5);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    assume {:print "$at(2,23898,23967)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
L0:

    // $t7 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:54+22
    assume {:print "$at(2,23943,23965)"} true;
    $t7 := 1;
    assume $IsValid'u64'($t7);

    // $t8 := error::invalid_argument($t7) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:30+47
    call $t8 := $1_error_invalid_argument($t7);
    if ($abort_flag) {
        assume {:print "$at(2,23919,23966)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    assume {:print "$at(2,23898,23967)"} true;
    assume {:print "$track_abort(56,18):", $t8} $t8 == $t8;

    // $t9 := move($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    $t9 := $t8;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:575:9+69
    goto L18;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:576:55+8
    assume {:print "$at(2,24023,24031)"} true;
L2:

    // $t10 := object::object_address<#0>($t0) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:576:32+32
    assume {:print "$at(2,24000,24032)"} true;
    call $t10 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,24000,24032)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_local[metadata_address]($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:576:13+16
    assume {:print "$track_local(56,18,2):", $t10} $t10 == $t10;

    // $t11 := exists<fungible_asset::ConcurrentSupply>($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:13+6
    assume {:print "$at(2,24047,24053)"} true;
    $t11 := $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t10);

    // if ($t11) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    if ($t11) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:579:62+16
    assume {:print "$at(2,24154,24170)"} true;
L4:

    // $t12 := borrow_global<fungible_asset::ConcurrentSupply>($t10) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:579:26+17
    assume {:print "$at(2,24118,24135)"} true;
    if (!$ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t12 := $Mutation($Global($t10), EmptyVec(), $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t10));
    }
    if ($abort_flag) {
        assume {:print "$at(2,24118,24135)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t13 := borrow_field<fungible_asset::ConcurrentSupply>.current($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:40+19
    assume {:print "$at(2,24233,24252)"} true;
    $t13 := $ChildMutation($t12, 0, $Dereference($t12)->$current);

    // $t14 := (u128)($t1) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:61+16
    call $t14 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,24254,24270)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t15 := opaque begin: aggregator_v2::try_add<u128>($t13, $t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61

    // $t16 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    havoc $t16;

    // if ($t16) goto L21 else goto L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    if ($t16) { goto L21; } else { goto L19; }

    // label L20 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
L20:

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume {:print "$at(2,24210,24271)"} true;
    assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    goto L18;

    // label L19 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
L19:

    // $t13 := havoc[mut]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume {:print "$at(2,24210,24271)"} true;
    havoc $temp_0'$1_aggregator_v2_Aggregator'u128'';
    $t13 := $UpdateMutation($t13, $temp_0'$1_aggregator_v2_Aggregator'u128'');

    // assume WellFormed($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($Dereference($t13));

    // assume WellFormed($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    assume $IsValid'bool'($t15);

    // $t15 := opaque end: aggregator_v2::try_add<u128>($t13, $t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61

    // write_back[Reference($t12).current (aggregator_v2::Aggregator<u128>)]($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    $t12 := $UpdateMutation($t12, $Update'$1_fungible_asset_ConcurrentSupply'_current($Dereference($t12), $Dereference($t13)));

    // write_back[fungible_asset::ConcurrentSupply@]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:581:17+61
    $1_fungible_asset_ConcurrentSupply_$memory := $ResourceUpdate($1_fungible_asset_ConcurrentSupply_$memory, $GlobalLocationAddress($t12),
        $Dereference($t12));

    // if ($t15) goto L6 else goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    assume {:print "$at(2,24185,24344)"} true;
    if ($t15) { goto L6; } else { goto L5; }

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
L6:

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    assume {:print "$at(2,24185,24344)"} true;
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:582:37+20
    assume {:print "$at(2,24309,24329)"} true;
L5:

    // $t17 := 5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:582:37+20
    assume {:print "$at(2,24309,24329)"} true;
    $t17 := 5;
    assume $IsValid'u64'($t17);

    // $t18 := error::out_of_range($t17) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:582:17+41
    call $t18 := $1_error_out_of_range($t17);
    if ($abort_flag) {
        assume {:print "$at(2,24289,24330)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    assume {:print "$at(2,24185,24344)"} true;
    assume {:print "$track_abort(56,18):", $t18} $t18 == $t18;

    // $t9 := move($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    $t9 := $t18;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:580:13+159
    goto L18;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    assume {:print "$at(2,24043,24928)"} true;
L7:

    // goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    assume {:print "$at(2,24043,24928)"} true;
    goto L8;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:584:35+16
    assume {:print "$at(2,24380,24396)"} true;
L3:

    // $t19 := exists<fungible_asset::Supply>($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:584:20+6
    assume {:print "$at(2,24365,24371)"} true;
    $t19 := $ResourceExists($1_fungible_asset_Supply_$memory, $t10);

    // if ($t19) goto L10 else goto L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:584:16+567
    if ($t19) { goto L10; } else { goto L9; }

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:585:52+16
    assume {:print "$at(2,24452,24468)"} true;
L10:

    // $t20 := borrow_global<fungible_asset::Supply>($t10) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:585:26+17
    assume {:print "$at(2,24426,24443)"} true;
    if (!$ResourceExists($1_fungible_asset_Supply_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t20 := $Mutation($Global($t10), EmptyVec(), $ResourceValue($1_fungible_asset_Supply_$memory, $t10));
    }
    if ($abort_flag) {
        assume {:print "$at(2,24426,24443)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_local[supply#1]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:585:17+6
    $temp_0'$1_fungible_asset_Supply' := $Dereference($t20);
    assume {:print "$track_local(56,18,3):", $temp_0'$1_fungible_asset_Supply'} $temp_0'$1_fungible_asset_Supply' == $temp_0'$1_fungible_asset_Supply';

    // $t21 := get_field<fungible_asset::Supply>.maximum($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:33+15
    assume {:print "$at(2,24503,24518)"} true;
    $t21 := $Dereference($t20)->$maximum;

    // $t22 := opaque begin: option::is_some<u128>($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32

    // assume WellFormed($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32
    assume $IsValid'bool'($t22);

    // assume Eq<bool>($t22, option::spec_is_some<u128>($t21)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32
    assume $IsEqual'bool'($t22, $1_option_spec_is_some'u128'($t21));

    // $t22 := opaque end: option::is_some<u128>($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:17+32

    // if ($t22) goto L12 else goto L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:13+288
    if ($t22) { goto L12; } else { goto L11; }

    // label L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:52+6
    assume {:print "$at(2,24574,24580)"} true;
L12:

    // $t23 := borrow_field<fungible_asset::Supply>.maximum($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:47+19
    assume {:print "$at(2,24569,24588)"} true;
    $t23 := $ChildMutation($t20, 1, $Dereference($t20)->$maximum);

    // $t24 := option::borrow_mut<u128>($t23) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:28+39
    call $t24,$t23 := $1_option_borrow_mut'u128'($t23);
    if ($abort_flag) {
        assume {:print "$at(2,24550,24589)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t25 := read_ref($t24) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:27+40
    $t25 := $Dereference($t24);

    // write_back[Reference($t20).maximum (option::Option<u128>)]($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:587:27+40
    $t20 := $UpdateMutation($t20, $Update'$1_fungible_asset_Supply'_maximum($Dereference($t20), $Dereference($t23)));

    // $t26 := get_field<fungible_asset::Supply>.current($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:27+14
    assume {:print "$at(2,24642,24656)"} true;
    $t26 := $Dereference($t20)->$current;

    // $t27 := -($t25, $t26) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:25+1
    call $t27 := $Sub($t25, $t26);
    if ($abort_flag) {
        assume {:print "$at(2,24640,24641)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t28 := (u128)($t1) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:45+16
    call $t28 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,24660,24676)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t29 := >=($t27, $t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:589:42+2
    call $t29 := $Ge($t27, $t28);

    // if ($t29) goto L14 else goto L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    if ($t29) { goto L14; } else { goto L13; }

    // label L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
L14:

    // goto L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    goto L15;

    // label L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
L13:

    // assert Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($t20))), 1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    // data invariant at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    assume {:print "$at(15,530,554)"} true;
    assert {:msg "assert_failed(15,530,554): data invariant does not hold"}
      (LenVec($Dereference($t20)->$maximum->$vec) <= 1);

    // write_back[fungible_asset::Supply@]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    $1_fungible_asset_Supply_$memory := $ResourceUpdate($1_fungible_asset_Supply_$memory, $GlobalLocationAddress($t20),
        $Dereference($t20));

    // destroy($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150

    // $t30 := 5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:590:41+20
    assume {:print "$at(2,24718,24738)"} true;
    $t30 := 5;
    assume $IsValid'u64'($t30);

    // $t31 := error::out_of_range($t30) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:590:21+41
    call $t31 := $1_error_out_of_range($t30);
    if ($abort_flag) {
        assume {:print "$at(2,24698,24739)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t31) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    assume {:print "$track_abort(56,18):", $t31} $t31 == $t31;

    // $t9 := move($t31) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    $t9 := $t31;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    goto L18;

    // label L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
L15:

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:588:17+150
    assume {:print "$at(2,24607,24757)"} true;
    goto L16;

    // label L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:13+288
    assume {:print "$at(2,24483,24771)"} true;
L11:

    // goto L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:586:13+288
    assume {:print "$at(2,24483,24771)"} true;
    goto L16;

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:45+17
    assume {:print "$at(2,24898,24915)"} true;
L9:

    // $t32 := 21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:45+17
    assume {:print "$at(2,24898,24915)"} true;
    $t32 := 21;
    assume $IsValid'u64'($t32);

    // $t33 := error::not_found($t32) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:28+35
    call $t33 := $1_error_not_found($t32);
    if ($abort_flag) {
        assume {:print "$at(2,24881,24916)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // trace_abort($t33) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:13+51
    assume {:print "$at(2,24866,24917)"} true;
    assume {:print "$track_abort(56,18):", $t33} $t33 == $t33;

    // $t9 := move($t33) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:13+51
    $t9 := $t33;

    // goto L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:595:13+51
    goto L18;

    // label L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:30+6
    assume {:print "$at(2,24802,24808)"} true;
L16:

    // $t34 := get_field<fungible_asset::Supply>.current($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:30+14
    assume {:print "$at(2,24802,24816)"} true;
    $t34 := $Dereference($t20)->$current;

    // $t35 := (u128)($t1) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:47+16
    call $t35 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,24819,24835)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t36 := +($t34, $t35) on_abort goto L18 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:45+1
    call $t36 := $AddU128($t34, $t35);
    if ($abort_flag) {
        assume {:print "$at(2,24817,24818)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,18):", $t9} $t9 == $t9;
        goto L18;
    }

    // $t37 := borrow_field<fungible_asset::Supply>.current($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+14
    $t37 := $ChildMutation($t20, 0, $Dereference($t20)->$current);

    // write_ref($t37, $t36) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+50
    $t37 := $UpdateMutation($t37, $t36);

    // write_back[Reference($t20).current (u128)]($t37) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+50
    $t20 := $UpdateMutation($t20, $Update'$1_fungible_asset_Supply'_current($Dereference($t20), $Dereference($t37)));

    // assert Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($t20))), 1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    // data invariant at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    assume {:print "$at(15,530,554)"} true;
    assert {:msg "assert_failed(15,530,554): data invariant does not hold"}
      (LenVec($Dereference($t20)->$maximum->$vec) <= 1);

    // write_back[fungible_asset::Supply@]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:593:13+50
    assume {:print "$at(2,24785,24835)"} true;
    $1_fungible_asset_Supply_$memory := $ResourceUpdate($1_fungible_asset_Supply_$memory, $GlobalLocationAddress($t20),
        $Dereference($t20));

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:578:9+885
    assume {:print "$at(2,24043,24928)"} true;
L8:

    // label L17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:597:5+1
    assume {:print "$at(2,24933,24934)"} true;
L17:

    // assert Not(Eq<u64>($t1, 0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:444:9+22
    assume {:print "$at(3,18493,18515)"} true;
    assert {:msg "assert_failed(3,18493,18515): function does not abort under this condition"}
      !$IsEqual'u64'($t1, 0);

    // assert Not(And(Not(exists[@37]<fungible_asset::ConcurrentSupply>($t4)), Not(exists[@38]<fungible_asset::Supply>($t4)))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:447:9+91
    assume {:print "$at(3,18590,18681)"} true;
    assert {:msg "assert_failed(3,18590,18681): function does not abort under this condition"}
      !(!$ResourceExists($1_fungible_asset_ConcurrentSupply_$memory#37, $t4) && !$ResourceExists($1_fungible_asset_Supply_$memory#38, $t4));

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:447:9+91
    return;

    // label L18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:597:5+1
    assume {:print "$at(2,24933,24934)"} true;
L18:

    // abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:597:5+1
    assume {:print "$at(2,24933,24934)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

    // label L21 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L21:

    // destroy($t12) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L20 at <internal>:1:1+10
    goto L20;

}

// fun fungible_asset::is_frozen<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:323:5+176
procedure {:inline 1} $1_fungible_asset_is_frozen'#0'(_$t0: $1_object_Object'#0') returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: bool;
    var $t5: int;
    var $t6: $1_fungible_asset_FungibleStore;
    var $t7: bool;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:323:5+1
    assume {:print "$at(2,13351,13352)"} true;
    assume {:print "$track_local(56,19,0):", $t0} $t0 == $t0;

    // $t2 := object::object_address<#0>($t0) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:22+30
    assume {:print "$at(2,13450,13480)"} true;
    call $t2 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,13450,13480)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t4 := fungible_asset::store_exists($t2) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+44
    call $t4 := $1_fungible_asset_store_exists($t2);
    if ($abort_flag) {
        assume {:print "$at(2,13437,13481)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // if ($t4) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    if ($t4) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:79+6
L1:

    // $t5 := object::object_address<#0>($t0) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:38+29
    assume {:print "$at(2,26657,26686)"} true;
    call $t5 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26657,26686)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t6 := get_global<fungible_asset::FungibleStore>($t5) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:9+13
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t5)) {
        call $ExecFailureAbort();
    } else {
        $t6 := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t5);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26628,26641)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t1 := get_field<fungible_asset::FungibleStore>.frozen($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:57+36
    assume {:print "$at(2,13485,13521)"} true;
    $t1 := $t6->$frozen;

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
L0:

    // $t7 := false at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    assume {:print "$at(2,13437,13521)"} true;
    $t7 := false;
    assume $IsValid'bool'($t7);

    // $t1 := $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    $t1 := $t7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
L2:

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    assume {:print "$at(2,13437,13521)"} true;
    assume {:print "$track_return(56,19,0):", $t1} $t1 == $t1;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
    assume {:print "$at(2,13526,13527)"} true;
L3:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
    assume {:print "$at(2,13526,13527)"} true;
    $ret0 := $t1;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
L4:

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
    assume {:print "$at(2,13526,13527)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun fungible_asset::is_frozen [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:323:5+176
procedure {:timeLimit 40} $1_fungible_asset_is_frozen$verify(_$t0: $1_object_Object'#0') returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: bool;
    var $t5: int;
    var $t6: $1_fungible_asset_FungibleStore;
    var $t7: bool;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:323:5+1
    assume {:print "$at(2,13351,13352)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:323:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:323:5+1
    assume {:print "$track_local(56,19,0):", $t0} $t0 == $t0;

    // $t2 := object::object_address<#0>($t0) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:22+30
    assume {:print "$at(2,13450,13480)"} true;
    call $t2 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,13450,13480)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t4 := fungible_asset::store_exists($t2) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+44
    call $t4 := $1_fungible_asset_store_exists($t2);
    if ($abort_flag) {
        assume {:print "$at(2,13437,13481)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // if ($t4) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    if ($t4) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:79+6
L1:

    // $t5 := object::object_address<#0>($t0) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:38+29
    assume {:print "$at(2,26657,26686)"} true;
    call $t5 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26657,26686)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t6 := get_global<fungible_asset::FungibleStore>($t5) on_abort goto L4 with $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:9+13
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t5)) {
        call $ExecFailureAbort();
    } else {
        $t6 := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t5);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26628,26641)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(56,19):", $t3} $t3 == $t3;
        goto L4;
    }

    // $t1 := get_field<fungible_asset::FungibleStore>.frozen($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:57+36
    assume {:print "$at(2,13485,13521)"} true;
    $t1 := $t6->$frozen;

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
L0:

    // $t7 := false at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    assume {:print "$at(2,13437,13521)"} true;
    $t7 := false;
    assume $IsValid'bool'($t7);

    // $t1 := $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    $t1 := $t7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
L2:

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:324:9+84
    assume {:print "$at(2,13437,13521)"} true;
    assume {:print "$track_return(56,19,0):", $t1} $t1 == $t1;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
    assume {:print "$at(2,13526,13527)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
    assume {:print "$at(2,13526,13527)"} true;
    assert {:msg "assert_failed(2,13526,13527): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
    $ret0 := $t1;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:325:5+1
L4:

    // assert false at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:232:5+54
    assume {:print "$at(3,12006,12060)"} true;
    assert {:msg "assert_failed(3,12006,12060): abort not covered by any of the `aborts_if` clauses"}
      false;

    // abort($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:232:5+54
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun fungible_asset::maximum [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:251:5+723
procedure {:timeLimit 40} $1_fungible_asset_maximum$verify(_$t0: $1_object_Object'#0') returns ($ret0: $1_option_Option'u128')
{
    // declare local variables
    var $t1: $1_option_Option'u128';
    var $t2: $1_option_Option'u128';
    var $t3: $1_option_Option'u128';
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: bool;
    var $t9: $1_fungible_asset_ConcurrentSupply;
    var $t10: $1_aggregator_v2_Aggregator'u128';
    var $t11: int;
    var $t12: int;
    var $t13: bool;
    var $t14: bool;
    var $t15: $1_fungible_asset_Supply;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_option_Option'u128'': $1_option_Option'u128';
    var $temp_0'address': int;
    var $temp_0'u128': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:251:5+1
    assume {:print "$at(2,10859,10860)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:251:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:251:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[metadata]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:251:5+1
    assume {:print "$track_local(56,20,0):", $t0} $t0 == $t0;

    // $t6 := object::object_address<#0>($t0) on_abort goto L10 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:252:32+33
    assume {:print "$at(2,10988,11021)"} true;
    call $t6 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,10988,11021)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,20):", $t7} $t7 == $t7;
        goto L10;
    }

    // trace_local[metadata_address]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:252:13+16
    assume {:print "$track_local(56,20,5):", $t6} $t6 == $t6;

    // $t8 := exists<fungible_asset::ConcurrentSupply>($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:253:13+6
    assume {:print "$at(2,11035,11041)"} true;
    $t8 := $ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t6);

    // if ($t8) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:253:9+545
    if ($t8) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:254:58+16
    assume {:print "$at(2,11138,11154)"} true;
L1:

    // $t9 := get_global<fungible_asset::ConcurrentSupply>($t6) on_abort goto L10 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:254:26+13
    assume {:print "$at(2,11106,11119)"} true;
    if (!$ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t6)) {
        call $ExecFailureAbort();
    } else {
        $t9 := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t6);
    }
    if ($abort_flag) {
        assume {:print "$at(2,11106,11119)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,20):", $t7} $t7 == $t7;
        goto L10;
    }

    // $t10 := get_field<fungible_asset::ConcurrentSupply>.current($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:255:54+15
    assume {:print "$at(2,11210,11225)"} true;
    $t10 := $t9->$current;

    // $t11 := aggregator_v2::max_value<u128>($t10) on_abort goto L10 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:255:29+41
    call $t11 := $1_aggregator_v2_max_value'u128'($t10);
    if ($abort_flag) {
        assume {:print "$at(2,11185,11226)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,20):", $t7} $t7 == $t7;
        goto L10;
    }

    // trace_local[max_value]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:255:17+9
    assume {:print "$track_local(56,20,4):", $t11} $t11 == $t11;

    // $t12 := 340282366920938463463374607431768211455 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:256:30+8
    assume {:print "$at(2,11257,11265)"} true;
    $t12 := 340282366920938463463374607431768211455;
    assume $IsValid'u128'($t12);

    // $t13 := ==($t11, $t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:256:27+2
    $t13 := $IsEqual'u128'($t11, $t12);

    // if ($t13) goto L3 else goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:256:13+134
    if ($t13) { goto L3; } else { goto L2; }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:257:17+14
    assume {:print "$at(2,11285,11299)"} true;
L3:

    // $t1 := opaque begin: option::none<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:257:17+14
    assume {:print "$at(2,11285,11299)"} true;

    // assume And(WellFormed($t1), Le(Len<u128>(select option::Option.vec($t1)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:257:17+14
    assume ($IsValid'$1_option_Option'u128''($t1) && (LenVec($t1->$vec) <= 1));

    // assume Eq<option::Option<u128>>($t1, option::spec_none<u128>()) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:257:17+14
    assume $IsEqual'$1_option_Option'u128''($t1, $1_option_spec_none'u128'());

    // $t1 := opaque end: option::none<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:257:17+14

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:256:13+134
    assume {:print "$at(2,11240,11374)"} true;
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:259:30+9
    assume {:print "$at(2,11350,11359)"} true;
L2:

    // $t1 := opaque begin: option::some<u128>($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:259:17+23
    assume {:print "$at(2,11337,11360)"} true;

    // assume And(WellFormed($t1), Le(Len<u128>(select option::Option.vec($t1)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:259:17+23
    assume ($IsValid'$1_option_Option'u128''($t1) && (LenVec($t1->$vec) <= 1));

    // assume Eq<option::Option<u128>>($t1, option::spec_some<u128>($t11)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:259:17+23
    assume $IsEqual'$1_option_Option'u128''($t1, $1_option_spec_some'u128'($t11));

    // $t1 := opaque end: option::some<u128>($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:259:17+23

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:256:13+134
    assume {:print "$at(2,11240,11374)"} true;
L4:

    // $t3 := $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:253:9+545
    assume {:print "$at(2,11031,11576)"} true;
    $t3 := $t1;

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:253:9+545
    goto L5;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:261:35+16
    assume {:print "$at(2,11409,11425)"} true;
L0:

    // $t14 := exists<fungible_asset::Supply>($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:261:20+6
    assume {:print "$at(2,11394,11400)"} true;
    $t14 := $ResourceExists($1_fungible_asset_Supply_$memory, $t6);

    // if ($t14) goto L7 else goto L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:261:16+186
    if ($t14) { goto L7; } else { goto L6; }

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:262:48+16
    assume {:print "$at(2,11477,11493)"} true;
L7:

    // $t15 := get_global<fungible_asset::Supply>($t6) on_abort goto L10 with $t7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:262:26+13
    assume {:print "$at(2,11455,11468)"} true;
    if (!$ResourceExists($1_fungible_asset_Supply_$memory, $t6)) {
        call $ExecFailureAbort();
    } else {
        $t15 := $ResourceValue($1_fungible_asset_Supply_$memory, $t6);
    }
    if ($abort_flag) {
        assume {:print "$at(2,11455,11468)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(56,20):", $t7} $t7 == $t7;
        goto L10;
    }

    // $t2 := get_field<fungible_asset::Supply>.maximum($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:263:13+14
    assume {:print "$at(2,11508,11522)"} true;
    $t2 := $t15->$maximum;

    // goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:261:16+186
    assume {:print "$at(2,11390,11576)"} true;
    goto L8;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:265:13+14
    assume {:print "$at(2,11552,11566)"} true;
L6:

    // $t2 := opaque begin: option::none<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:265:13+14
    assume {:print "$at(2,11552,11566)"} true;

    // assume And(WellFormed($t2), Le(Len<u128>(select option::Option.vec($t2)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:265:13+14
    assume ($IsValid'$1_option_Option'u128''($t2) && (LenVec($t2->$vec) <= 1));

    // assume Eq<option::Option<u128>>($t2, option::spec_none<u128>()) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:265:13+14
    assume $IsEqual'$1_option_Option'u128''($t2, $1_option_spec_none'u128'());

    // $t2 := opaque end: option::none<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:265:13+14

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:261:16+186
    assume {:print "$at(2,11390,11576)"} true;
L8:

    // $t3 := $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:253:9+545
    assume {:print "$at(2,11031,11576)"} true;
    $t3 := $t2;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:253:9+545
L5:

    // trace_return[0]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:253:9+545
    assume {:print "$at(2,11031,11576)"} true;
    assume {:print "$track_return(56,20,0):", $t3} $t3 == $t3;

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:267:5+1
    assume {:print "$at(2,11581,11582)"} true;
L9:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:267:5+1
    assume {:print "$at(2,11581,11582)"} true;
    assert {:msg "assert_failed(2,11581,11582): function does not abort under this condition"}
      !false;

    // return $t3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:267:5+1
    $ret0 := $t3;
    return;

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:267:5+1
L10:

    // assert false at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:200:5+63
    assume {:print "$at(3,11293,11356)"} true;
    assert {:msg "assert_failed(3,11293,11356): abort not covered by any of the `aborts_if` clauses"}
      false;

    // abort($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:200:5+63
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun fungible_asset::metadata_from_asset [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:294:5+96
procedure {:timeLimit 40} $1_fungible_asset_metadata_from_asset$verify(_$t0: $1_fungible_asset_FungibleAsset) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:294:5+1
    assume {:print "$at(2,12451,12452)"} true;
    assume $IsValid'$1_fungible_asset_FungibleAsset'($t0);

    // trace_local[fa]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:294:5+1
    assume {:print "$track_local(56,22,0):", $t0} $t0 == $t0;

    // $t1 := get_field<fungible_asset::FungibleAsset>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:295:9+11
    assume {:print "$at(2,12530,12541)"} true;
    $t1 := $t0->$metadata;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:295:9+11
    assume {:print "$track_return(56,22,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:296:5+1
    assume {:print "$at(2,12546,12547)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:296:5+1
    assume {:print "$at(2,12546,12547)"} true;
    assert {:msg "assert_failed(2,12546,12547): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:296:5+1
    $ret0 := $t1;
    return;

}

// fun fungible_asset::mint_ref_metadata [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:332:5+90
procedure {:timeLimit 40} $1_fungible_asset_mint_ref_metadata$verify(_$t0: $1_fungible_asset_MintRef) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_fungible_asset_MintRef;
    var $temp_0'$1_fungible_asset_MintRef': $1_fungible_asset_MintRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:332:5+1
    assume {:print "$at(2,13693,13694)"} true;
    assume $IsValid'$1_fungible_asset_MintRef'($t0);

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:332:5+1
    assume {:print "$track_local(56,24,0):", $t0} $t0 == $t0;

    // $t1 := get_field<fungible_asset::MintRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:333:9+12
    assume {:print "$at(2,13765,13777)"} true;
    $t1 := $t0->$metadata;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:333:9+12
    assume {:print "$track_return(56,24,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:334:5+1
    assume {:print "$at(2,13782,13783)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:334:5+1
    assume {:print "$at(2,13782,13783)"} true;
    assert {:msg "assert_failed(2,13782,13783): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:334:5+1
    $ret0 := $t1;
    return;

}

// fun fungible_asset::mint_to [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+197
procedure {:timeLimit 40} $1_fungible_asset_mint_to$verify(_$t0: $1_fungible_asset_MintRef, _$t1: $1_object_Object'#0', _$t2: int) returns ()
{
    // declare local variables
    var $t3: $1_fungible_asset_FungibleAsset;
    var $t4: int;
    var $t5: int;
    var $t6: $1_fungible_asset_ConcurrentSupply;
    var $t0: $1_fungible_asset_MintRef;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_fungible_asset_MintRef': $1_fungible_asset_MintRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume {:print "$at(2,17622,17623)"} true;
    assume $IsValid'$1_fungible_asset_MintRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume $IsValid'u64'($t2);

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume {:print "$track_local(56,25,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume {:print "$track_local(56,25,1):", $t1} $t1 == $t1;

    // trace_local[amount]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:428:5+1
    assume {:print "$track_local(56,25,2):", $t2} $t2 == $t2;

    // $t3 := fungible_asset::mint($t0, $t2) on_abort goto L2 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:430:24+17
    assume {:print "$at(2,17794,17811)"} true;
    call $t3 := $1_fungible_asset_mint($t0, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,17794,17811)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(56,25):", $t4} $t4 == $t4;
        goto L2;
    }

    // assume Identical($t5, select object::Object.inner($t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:326:9+34
    assume {:print "$at(3,15130,15164)"} true;
    assume ($t5 == $t1->$inner);

    // assume Identical($t6, global<fungible_asset::ConcurrentSupply>($t5)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:327:9+55
    assume {:print "$at(3,15173,15228)"} true;
    assume ($t6 == $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $t5));

    // fungible_asset::deposit<#0>($t1, $t3) on_abort goto L2 with $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:430:9+33
    assume {:print "$at(2,17779,17812)"} true;
    call $1_fungible_asset_deposit'#0'($t1, $t3);
    if ($abort_flag) {
        assume {:print "$at(2,17779,17812)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(56,25):", $t4} $t4 == $t4;
        goto L2;
    }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:431:5+1
    assume {:print "$at(2,17818,17819)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:431:5+1
    assume {:print "$at(2,17818,17819)"} true;
    assert {:msg "assert_failed(2,17818,17819): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:431:5+1
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:431:5+1
L2:

    // abort($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:431:5+1
    assume {:print "$at(2,17818,17819)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun fungible_asset::remove_store [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:382:5+725
procedure {:timeLimit 40} $1_fungible_asset_remove_store$verify(_$t0: $1_object_DeleteRef) returns ()
{
    // declare local variables
    var $t1: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $t2: int;
    var $t3: int;
    var $t4: $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $t5: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $t6: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $t7: int;
    var $t8: $1_object_Object'$1_fungible_asset_FungibleStore';
    var $t9: int;
    var $t10: int;
    var $t11: $1_fungible_asset_FungibleStore;
    var $t12: $1_object_Object'$1_fungible_asset_Metadata';
    var $t13: int;
    var $t14: bool;
    var $t15: int;
    var $t16: bool;
    var $t17: int;
    var $t18: int;
    var $t19: $1_fungible_asset_FungibleAssetEvents;
    var $t20: $1_event_EventHandle'$1_fungible_asset_DepositEvent';
    var $t21: $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $t22: $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $t0: $1_object_DeleteRef;
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_FrozenEvent'': $1_event_EventHandle'$1_fungible_asset_FrozenEvent';
    var $temp_0'$1_event_EventHandle'$1_fungible_asset_WithdrawEvent'': $1_event_EventHandle'$1_fungible_asset_WithdrawEvent';
    var $temp_0'$1_object_DeleteRef': $1_object_DeleteRef;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    var $1_fungible_asset_FungibleStore_$memory#41: $Memory $1_fungible_asset_FungibleStore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:382:5+1
    assume {:print "$at(2,15606,15607)"} true;
    assume $IsValid'$1_object_DeleteRef'($t0);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:382:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:382:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:382:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume Identical($t6, object::$object_from_delete_ref<fungible_asset::FungibleStore>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:284:9+70
    assume {:print "$at(3,13588,13658)"} true;
    assume ($t6 == $1_object_$object_from_delete_ref'$1_fungible_asset_FungibleStore'($t0));

    // assume Identical($t7, object::$object_address<fungible_asset::FungibleStore>($t6)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:285:9+41
    assume {:print "$at(3,13667,13708)"} true;
    assume ($t7 == $1_object_$object_address'$1_fungible_asset_FungibleStore'($t6));

    // @41 := save_mem(fungible_asset::FungibleStore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:382:5+1
    assume {:print "$at(2,15606,15607)"} true;
    $1_fungible_asset_FungibleStore_$memory#41 := $1_fungible_asset_FungibleStore_$memory;

    // trace_local[delete_ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:382:5+1
    assume {:print "$track_local(56,27,0):", $t0} $t0 == $t0;

    // $t8 := object::object_from_delete_ref<fungible_asset::FungibleStore>($t0) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:383:22+57
    assume {:print "$at(2,15721,15778)"} true;
    call $t8 := $1_object_object_from_delete_ref'$1_fungible_asset_FungibleStore'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,15721,15778)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // $t10 := object::object_address<fungible_asset::FungibleStore>($t8) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:384:20+29
    assume {:print "$at(2,15799,15828)"} true;
    call $t10 := $1_object_object_address'$1_fungible_asset_FungibleStore'($t8);
    if ($abort_flag) {
        assume {:print "$at(2,15799,15828)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // trace_local[addr]($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:384:13+4
    assume {:print "$track_local(56,27,2):", $t10} $t10 == $t10;

    // $t11 := move_from<fungible_asset::FungibleStore>($t10) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:386:15+9
    assume {:print "$at(2,15906,15915)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t11 := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t10);
        $1_fungible_asset_FungibleStore_$memory := $ResourceRemove($1_fungible_asset_FungibleStore_$memory, $t10);
    }
    if ($abort_flag) {
        assume {:print "$at(2,15906,15915)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // ($t12, $t13, $t14) := unpack fungible_asset::FungibleStore($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:385:13+49
    assume {:print "$at(2,15842,15891)"} true;
    $t12 := $t11->$metadata;
    $t13 := $t11->$balance;
    $t14 := $t11->$frozen;

    // destroy($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:385:59+1

    // trace_local[balance]($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:385:42+7
    assume {:print "$track_local(56,27,3):", $t13} $t13 == $t13;

    // destroy($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:385:39+1

    // $t15 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:28+1
    assume {:print "$at(2,15965,15966)"} true;
    $t15 := 0;
    assume $IsValid'u64'($t15);

    // $t16 := ==($t13, $t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:25+2
    $t16 := $IsEqual'u64'($t13, $t15);

    // if ($t16) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:9+69
    if ($t16) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:9+69
    assume {:print "$at(2,15946,16015)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:56+20
L0:

    // $t17 := 14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:56+20
    assume {:print "$at(2,15993,16013)"} true;
    $t17 := 14;
    assume $IsValid'u64'($t17);

    // $t18 := error::permission_denied($t17) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:31+46
    call $t18 := $1_error_permission_denied($t17);
    if ($abort_flag) {
        assume {:print "$at(2,15968,16014)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // trace_abort($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:9+69
    assume {:print "$at(2,15946,16015)"} true;
    assume {:print "$track_abort(56,27):", $t18} $t18 == $t18;

    // $t9 := move($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:9+69
    $t9 := $t18;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:387:9+69
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:392:44+4
    assume {:print "$at(2,16178,16182)"} true;
L2:

    // $t19 := move_from<fungible_asset::FungibleAssetEvents>($t10) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:392:13+9
    assume {:print "$at(2,16147,16156)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t19 := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $t10);
        $1_fungible_asset_FungibleAssetEvents_$memory := $ResourceRemove($1_fungible_asset_FungibleAssetEvents_$memory, $t10);
    }
    if ($abort_flag) {
        assume {:print "$at(2,16147,16156)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // ($t20, $t21, $t22) := unpack fungible_asset::FungibleAssetEvents($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:388:13+115
    assume {:print "$at(2,16029,16144)"} true;
    $t20 := $t19->$deposit_events;
    $t21 := $t19->$withdraw_events;
    $t22 := $t19->$frozen_events;

    // trace_local[frozen_events]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:391:13+13
    assume {:print "$at(2,16120,16133)"} true;
    assume {:print "$track_local(56,27,4):", $t22} $t22 == $t22;

    // trace_local[withdraw_events]($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:390:13+15
    assume {:print "$at(2,16091,16106)"} true;
    assume {:print "$track_local(56,27,5):", $t21} $t21 == $t21;

    // event::destroy_handle<fungible_asset::DepositEvent>($t20) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:393:9+37
    assume {:print "$at(2,16193,16230)"} true;
    call $1_event_destroy_handle'$1_fungible_asset_DepositEvent'($t20);
    if ($abort_flag) {
        assume {:print "$at(2,16193,16230)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // event::destroy_handle<fungible_asset::WithdrawEvent>($t21) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:394:9+38
    assume {:print "$at(2,16240,16278)"} true;
    call $1_event_destroy_handle'$1_fungible_asset_WithdrawEvent'($t21);
    if ($abort_flag) {
        assume {:print "$at(2,16240,16278)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // event::destroy_handle<fungible_asset::FrozenEvent>($t22) on_abort goto L4 with $t9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:395:9+36
    assume {:print "$at(2,16288,16324)"} true;
    call $1_event_destroy_handle'$1_fungible_asset_FrozenEvent'($t22);
    if ($abort_flag) {
        assume {:print "$at(2,16288,16324)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(56,27):", $t9} $t9 == $t9;
        goto L4;
    }

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:396:5+1
    assume {:print "$at(2,16330,16331)"} true;
L3:

    // assert Not(Not(exists[@41]<fungible_asset::FungibleStore>($t7))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:286:9+39
    assume {:print "$at(3,13717,13756)"} true;
    assert {:msg "assert_failed(3,13717,13756): function does not abort under this condition"}
      !!$ResourceExists($1_fungible_asset_FungibleStore_$memory#41, $t7);

    // assert Not(Neq<u64>(select fungible_asset::FungibleStore.balance(global[@41]<fungible_asset::FungibleStore>($t7)), 0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:288:9+58
    assume {:print "$at(3,13797,13855)"} true;
    assert {:msg "assert_failed(3,13797,13855): function does not abort under this condition"}
      !!$IsEqual'u64'($ResourceValue($1_fungible_asset_FungibleStore_$memory#41, $t7)->$balance, 0);

    // assert Not(exists<fungible_asset::FungibleStore>($t7)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:289:9+37
    assume {:print "$at(3,13864,13901)"} true;
    assert {:msg "assert_failed(3,13864,13901): post-condition does not hold"}
      !$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t7);

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:289:9+37
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:396:5+1
    assume {:print "$at(2,16330,16331)"} true;
L4:

    // abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:396:5+1
    assume {:print "$at(2,16330,16331)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

}

// fun fungible_asset::set_frozen_flag [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+597
procedure {:timeLimit 40} $1_fungible_asset_set_frozen_flag$verify(_$t0: $1_fungible_asset_TransferRef, _$t1: $1_object_Object'#0', _$t2: bool) returns ()
{
    // declare local variables
    var $t3: int;
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: $1_object_Object'$1_fungible_asset_Metadata';
    var $t6: int;
    var $t7: bool;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $Mutation ($1_fungible_asset_FungibleStore);
    var $t12: $Mutation (bool);
    var $t13: $Mutation ($1_fungible_asset_FungibleAssetEvents);
    var $t14: $Mutation ($1_event_EventHandle'$1_fungible_asset_FrozenEvent');
    var $t15: $1_fungible_asset_FrozenEvent;
    var $t0: $1_fungible_asset_TransferRef;
    var $t1: $1_object_Object'#0';
    var $t2: bool;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume {:print "$at(2,17912,17913)"} true;
    assume $IsValid'$1_fungible_asset_TransferRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume $IsValid'bool'($t2);

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume {:print "$track_local(56,28,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume {:print "$track_local(56,28,1):", $t1} $t1 == $t1;

    // trace_local[frozen]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:434:5+1
    assume {:print "$track_local(56,28,2):", $t2} $t2 == $t2;

    // $t4 := get_field<fungible_asset::TransferRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:440:13+12
    assume {:print "$at(2,18104,18116)"} true;
    $t4 := $t0->$metadata;

    // $t5 := fungible_asset::store_metadata<#0>($t1) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:440:29+21
    call $t5 := $1_fungible_asset_store_metadata'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,18120,18141)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,28):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t7 := ==($t4, $t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:440:26+2
    $t7 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t4, $t5);

    // if ($t7) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:439:9+140
    assume {:print "$at(2,18083,18223)"} true;
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:439:9+140
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:439:9+140
    assume {:print "$at(2,18083,18223)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:441:37+32
    assume {:print "$at(2,18179,18211)"} true;
L0:

    // $t8 := 9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:441:37+32
    assume {:print "$at(2,18179,18211)"} true;
    $t8 := 9;
    assume $IsValid'u64'($t8);

    // $t9 := error::invalid_argument($t8) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:441:13+57
    call $t9 := $1_error_invalid_argument($t8);
    if ($abort_flag) {
        assume {:print "$at(2,18155,18212)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,28):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:439:9+140
    assume {:print "$at(2,18083,18223)"} true;
    assume {:print "$track_abort(56,28):", $t9} $t9 == $t9;

    // $t6 := move($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:439:9+140
    $t6 := $t9;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:439:9+140
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:443:49+6
    assume {:print "$at(2,18273,18279)"} true;
L2:

    // $t10 := object::object_address<#0>($t1) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:443:26+30
    assume {:print "$at(2,18250,18280)"} true;
    call $t10 := $1_object_object_address'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,18250,18280)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,28):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_local[store_addr]($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:443:13+10
    assume {:print "$track_local(56,28,3):", $t10} $t10 == $t10;

    // $t11 := borrow_global<fungible_asset::FungibleStore>($t10) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:444:9+17
    assume {:print "$at(2,18290,18307)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t11 := $Mutation($Global($t10), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t10));
    }
    if ($abort_flag) {
        assume {:print "$at(2,18290,18307)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,28):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t12 := borrow_field<fungible_asset::FungibleStore>.frozen($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:444:9+51
    $t12 := $ChildMutation($t11, 2, $Dereference($t11)->$frozen);

    // write_ref($t12, $t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:444:9+60
    $t12 := $UpdateMutation($t12, $t2);

    // write_back[Reference($t11).frozen (bool)]($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:444:9+60
    $t11 := $UpdateMutation($t11, $Update'$1_fungible_asset_FungibleStore'_frozen($Dereference($t11), $Dereference($t12)));

    // write_back[fungible_asset::FungibleStore@]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:444:9+60
    $1_fungible_asset_FungibleStore_$memory := $ResourceUpdate($1_fungible_asset_FungibleStore_$memory, $GlobalLocationAddress($t11),
        $Dereference($t11));

    // $t13 := borrow_global<fungible_asset::FungibleAssetEvents>($t10) on_abort goto L4 with $t6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:446:22+17
    assume {:print "$at(2,18374,18391)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t10)) {
        call $ExecFailureAbort();
    } else {
        $t13 := $Mutation($Global($t10), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $t10));
    }
    if ($abort_flag) {
        assume {:print "$at(2,18374,18391)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(56,28):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t14 := borrow_field<fungible_asset::FungibleAssetEvents>.frozen_events($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:447:27+25
    assume {:print "$at(2,18452,18477)"} true;
    $t14 := $ChildMutation($t13, 2, $Dereference($t13)->$frozen_events);

    // $t15 := pack fungible_asset::FrozenEvent($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:447:54+22
    $t15 := $1_fungible_asset_FrozenEvent($t2);

    // opaque begin: event::emit_event<fungible_asset::FrozenEvent>($t14, $t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:447:9+68

    // opaque end: event::emit_event<fungible_asset::FrozenEvent>($t14, $t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:447:9+68

    // write_back[Reference($t13).frozen_events (event::EventHandle<fungible_asset::FrozenEvent>)]($t14) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:447:9+68
    $t13 := $UpdateMutation($t13, $Update'$1_fungible_asset_FungibleAssetEvents'_frozen_events($Dereference($t13), $Dereference($t14)));

    // write_back[fungible_asset::FungibleAssetEvents@]($t13) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:447:9+68
    $1_fungible_asset_FungibleAssetEvents_$memory := $ResourceUpdate($1_fungible_asset_FungibleAssetEvents_$memory, $GlobalLocationAddress($t13),
        $Dereference($t13));

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:448:5+1
    assume {:print "$at(2,18508,18509)"} true;
L3:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:448:5+1
    assume {:print "$at(2,18508,18509)"} true;
    assert {:msg "assert_failed(2,18508,18509): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:448:5+1
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:448:5+1
L4:

    // abort($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:448:5+1
    assume {:print "$at(2,18508,18509)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun fungible_asset::store_exists [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:289:5+90
procedure {:inline 1} $1_fungible_asset_store_exists(_$t0: int) returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:289:5+1
    assume {:print "$at(2,12309,12310)"} true;
    assume {:print "$track_local(56,29,0):", $t0} $t0 == $t0;

    // $t1 := exists<fungible_asset::FungibleStore>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:290:9+6
    assume {:print "$at(2,12365,12371)"} true;
    $t1 := $ResourceExists($1_fungible_asset_FungibleStore_$memory, $t0);

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:290:9+28
    assume {:print "$track_return(56,29,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:291:5+1
    assume {:print "$at(2,12398,12399)"} true;
L1:

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:291:5+1
    assume {:print "$at(2,12398,12399)"} true;
    $ret0 := $t1;
    return;

}

// fun fungible_asset::store_exists [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:289:5+90
procedure {:timeLimit 40} $1_fungible_asset_store_exists$verify(_$t0: int) returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:289:5+1
    assume {:print "$at(2,12309,12310)"} true;
    assume $IsValid'address'($t0);

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:289:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:289:5+1
    assume {:print "$track_local(56,29,0):", $t0} $t0 == $t0;

    // $t1 := exists<fungible_asset::FungibleStore>($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:290:9+6
    assume {:print "$at(2,12365,12371)"} true;
    $t1 := $ResourceExists($1_fungible_asset_FungibleStore_$memory, $t0);

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:290:9+28
    assume {:print "$track_return(56,29,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:291:5+1
    assume {:print "$at(2,12398,12399)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:291:5+1
    assume {:print "$at(2,12398,12399)"} true;
    assert {:msg "assert_failed(2,12398,12399): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:291:5+1
    $ret0 := $t1;
    return;

}

// fun fungible_asset::store_metadata<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:300:5+147
procedure {:inline 1} $1_fungible_asset_store_metadata'#0'(_$t0: $1_object_Object'#0') returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: $1_fungible_asset_FungibleStore;
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:300:5+1
    assume {:print "$at(2,12612,12613)"} true;
    assume {:print "$track_local(56,30,0):", $t0} $t0 == $t0;

    // $t1 := object::object_address<#0>($t0) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:38+29
    assume {:print "$at(2,26657,26686)"} true;
    call $t1 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26657,26686)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,30):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_global<fungible_asset::FungibleStore>($t1) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:9+13
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26628,26641)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,30):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t4 := get_field<fungible_asset::FungibleStore>.metadata($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:301:9+38
    assume {:print "$at(2,12715,12753)"} true;
    $t4 := $t3->$metadata;

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:301:9+38
    assume {:print "$track_return(56,30,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
    assume {:print "$at(2,12758,12759)"} true;
L1:

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
    assume {:print "$at(2,12758,12759)"} true;
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
L2:

    // abort($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
    assume {:print "$at(2,12758,12759)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun fungible_asset::store_metadata [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:300:5+147
procedure {:timeLimit 40} $1_fungible_asset_store_metadata$verify(_$t0: $1_object_Object'#0') returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: $1_fungible_asset_FungibleStore;
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:300:5+1
    assume {:print "$at(2,12612,12613)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:300:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // trace_local[store]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:300:5+1
    assume {:print "$track_local(56,30,0):", $t0} $t0 == $t0;

    // $t1 := object::object_address<#0>($t0) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:38+29
    assume {:print "$at(2,26657,26686)"} true;
    call $t1 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26657,26686)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,30):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_global<fungible_asset::FungibleStore>($t1) on_abort goto L2 with $t2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:639:9+13
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,26628,26641)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(56,30):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t4 := get_field<fungible_asset::FungibleStore>.metadata($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:301:9+38
    assume {:print "$at(2,12715,12753)"} true;
    $t4 := $t3->$metadata;

    // trace_return[0]($t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:301:9+38
    assume {:print "$track_return(56,30,0):", $t4} $t4 == $t4;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
    assume {:print "$at(2,12758,12759)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
    assume {:print "$at(2,12758,12759)"} true;
    assert {:msg "assert_failed(2,12758,12759): function does not abort under this condition"}
      !false;

    // return $t4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
    $ret0 := $t4;
    return;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
L2:

    // abort($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:302:5+1
    assume {:print "$at(2,12758,12759)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun fungible_asset::transfer_ref_metadata [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:337:5+98
procedure {:timeLimit 40} $1_fungible_asset_transfer_ref_metadata$verify(_$t0: $1_fungible_asset_TransferRef) returns ($ret0: $1_object_Object'$1_fungible_asset_Metadata')
{
    // declare local variables
    var $t1: $1_object_Object'$1_fungible_asset_Metadata';
    var $t0: $1_fungible_asset_TransferRef;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:337:5+1
    assume {:print "$at(2,13856,13857)"} true;
    assume $IsValid'$1_fungible_asset_TransferRef'($t0);

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:337:5+1
    assume {:print "$track_local(56,34,0):", $t0} $t0 == $t0;

    // $t1 := get_field<fungible_asset::TransferRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:338:9+12
    assume {:print "$at(2,13936,13948)"} true;
    $t1 := $t0->$metadata;

    // trace_return[0]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:338:9+12
    assume {:print "$track_return(56,34,0):", $t1} $t1 == $t1;

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:339:5+1
    assume {:print "$at(2,13953,13954)"} true;
L1:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:339:5+1
    assume {:print "$at(2,13953,13954)"} true;
    assert {:msg "assert_failed(2,13953,13954): function does not abort under this condition"}
      !false;

    // return $t1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:339:5+1
    $ret0 := $t1;
    return;

}

// fun fungible_asset::upgrade_to_concurrent [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:642:5+1043
procedure {:timeLimit 40} $1_fungible_asset_upgrade_to_concurrent$verify(_$t0: $1_object_ExtendRef) returns ()
{
    // declare local variables
    var $t1: $1_aggregator_v2_Aggregator'u128';
    var $t2: int;
    var $t3: $1_option_Option'u128';
    var $t4: int;
    var $t5: $signer;
    var $t6: $1_fungible_asset_ConcurrentSupply;
    var $t7: int;
    var $t8: int;
    var $t9: $signer;
    var $t10: bool;
    var $t11: int;
    var $t12: int;
    var $t13: bool;
    var $t14: int;
    var $t15: int;
    var $t16: $1_fungible_asset_Supply;
    var $t17: int;
    var $t18: $1_option_Option'u128';
    var $t19: $1_option_Option'u128';
    var $t20: bool;
    var $t21: bool;
    var $t22: $Mutation ($1_option_Option'u128');
    var $t23: int;
    var $t24: $1_option_Option'u128';
    var $t25: bool;
    var $t26: bool;
    var $t27: $Mutation ($1_fungible_asset_ConcurrentSupply);
    var $t28: $Mutation ($1_aggregator_v2_Aggregator'u128');
    var $t29: $1_fungible_asset_ConcurrentSupply;
    var $t0: $1_object_ExtendRef;
    var $temp_0'$1_fungible_asset_ConcurrentSupply': $1_fungible_asset_ConcurrentSupply;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    var $temp_0'$1_option_Option'u128'': $1_option_Option'u128';
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    var $temp_0'u128': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:642:5+1
    assume {:print "$at(2,26699,26700)"} true;
    assume $IsValid'$1_object_ExtendRef'($t0);

    // assume forall $rsc: features::Features: ResourceDomain<features::Features>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:642:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_features_Features_$memory, $a_0)}(var $rsc := $ResourceValue($1_features_Features_$memory, $a_0);
    ($IsValid'$1_features_Features'($rsc))));

    // assume forall $rsc: fungible_asset::ConcurrentSupply: ResourceDomain<fungible_asset::ConcurrentSupply>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:642:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_ConcurrentSupply_$memory, $a_0);
    ($IsValid'$1_fungible_asset_ConcurrentSupply'($rsc))));

    // assume forall $rsc: fungible_asset::Supply: ResourceDomain<fungible_asset::Supply>(): And(WellFormed($rsc), Le(Len<u128>(select option::Option.vec(select fungible_asset::Supply.maximum($rsc))), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:642:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_Supply_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_Supply_$memory, $a_0);
    (($IsValid'$1_fungible_asset_Supply'($rsc) && (LenVec($rsc->$maximum->$vec) <= 1)))));

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:642:5+1
    assume {:print "$track_local(56,36,0):", $t0} $t0 == $t0;

    // $t7 := object::address_from_extend_ref($t0) on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:645:39+36
    assume {:print "$at(2,26820,26856)"} true;
    call $t7 := $1_object_address_from_extend_ref($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26820,26856)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // trace_local[metadata_object_address]($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:645:13+23
    assume {:print "$track_local(56,36,4):", $t7} $t7 == $t7;

    // $t9 := object::generate_signer_for_extending($t0) on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:646:38+42
    assume {:print "$at(2,26895,26937)"} true;
    call $t9 := $1_object_generate_signer_for_extending($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26895,26937)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // trace_local[metadata_object_signer]($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:646:13+22
    assume {:print "$track_local(56,36,5):", $t9} $t9 == $t9;

    // $t10 := features::concurrent_assets_enabled() on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:17+37
    assume {:print "$at(2,26955,26992)"} true;
    call $t10 := $1_features_concurrent_assets_enabled();
    if ($abort_flag) {
        assume {:print "$at(2,26955,26992)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // if ($t10) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:9+103
    if ($t10) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:9+103
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:9+103
    assume {:print "$at(2,26947,27050)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:80+30
L0:

    // $t11 := 22 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:80+30
    assume {:print "$at(2,27018,27048)"} true;
    $t11 := 22;
    assume $IsValid'u64'($t11);

    // $t12 := error::invalid_argument($t11) on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:56+55
    call $t12 := $1_error_invalid_argument($t11);
    if ($abort_flag) {
        assume {:print "$at(2,26994,27049)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // trace_abort($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:9+103
    assume {:print "$at(2,26947,27050)"} true;
    assume {:print "$track_abort(56,36):", $t12} $t12 == $t12;

    // $t8 := move($t12) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:9+103
    $t8 := $t12;

    // goto L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:647:9+103
    goto L10;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:32+23
    assume {:print "$at(2,27083,27106)"} true;
L2:

    // $t13 := exists<fungible_asset::Supply>($t7) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:17+6
    assume {:print "$at(2,27068,27074)"} true;
    $t13 := $ResourceExists($1_fungible_asset_Supply_$memory, $t7);

    // if ($t13) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:9+85
    if ($t13) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:9+85
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:9+85
    assume {:print "$at(2,27060,27145)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:75+17
L3:

    // $t14 := 21 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:75+17
    assume {:print "$at(2,27126,27143)"} true;
    $t14 := 21;
    assume $IsValid'u64'($t14);

    // $t15 := error::not_found($t14) on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:58+35
    call $t15 := $1_error_not_found($t14);
    if ($abort_flag) {
        assume {:print "$at(2,27109,27144)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // trace_abort($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:9+85
    assume {:print "$at(2,27060,27145)"} true;
    assume {:print "$track_abort(56,36):", $t15} $t15 == $t15;

    // $t8 := move($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:9+85
    $t8 := $t15;

    // goto L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:648:9+85
    goto L10;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:652:31+23
    assume {:print "$at(2,27240,27263)"} true;
L5:

    // $t16 := move_from<fungible_asset::Supply>($t7) on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:652:13+9
    assume {:print "$at(2,27222,27231)"} true;
    if (!$ResourceExists($1_fungible_asset_Supply_$memory, $t7)) {
        call $ExecFailureAbort();
    } else {
        $t16 := $ResourceValue($1_fungible_asset_Supply_$memory, $t7);
        $1_fungible_asset_Supply_$memory := $ResourceRemove($1_fungible_asset_Supply_$memory, $t7);
    }
    if ($abort_flag) {
        assume {:print "$at(2,27222,27231)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // ($t17, $t18) := unpack fungible_asset::Supply($t16) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:649:13+60
    assume {:print "$at(2,27159,27219)"} true;
    $t17 := $t16->$current;
    $t18 := $t16->$maximum;

    // $t3 := $t18 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:651:13+7
    assume {:print "$at(2,27201,27208)"} true;
    $t3 := $t18;

    // trace_local[maximum]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:651:13+7
    assume {:print "$track_local(56,36,3):", $t3} $t3 == $t3;

    // trace_local[current]($t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:650:13+7
    assume {:print "$at(2,27180,27187)"} true;
    assume {:print "$track_local(56,36,2):", $t17} $t17 == $t17;

    // $t19 := copy($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:654:41+8
    assume {:print "$at(2,27307,27315)"} true;
    $t19 := $t3;

    // $t20 := opaque begin: option::is_none<u128>($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:654:25+25

    // assume WellFormed($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:654:25+25
    assume $IsValid'bool'($t20);

    // assume Eq<bool>($t20, option::spec_is_none<u128>($t19)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:654:25+25
    assume $IsEqual'bool'($t20, $1_option_spec_is_none'u128'($t19));

    // $t20 := opaque end: option::is_none<u128>($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:654:25+25

    // if ($t20) goto L7 else goto L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:656:22+204
    assume {:print "$at(2,27379,27583)"} true;
    if ($t20) { goto L7; } else { goto L6; }

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
    assume {:print "$at(2,27412,27456)"} true;
L7:

    // $t1 := opaque begin: aggregator_v2::create_unbounded_aggregator<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
    assume {:print "$at(2,27412,27456)"} true;

    // $t21 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
    havoc $t21;

    // if ($t21) goto L12 else goto L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
    if ($t21) { goto L12; } else { goto L11; }

    // label L12 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
L12:

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
    assume {:print "$at(2,27412,27456)"} true;
    assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;

    // goto L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
    goto L10;

    // label L11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
L11:

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44
    assume {:print "$at(2,27412,27456)"} true;
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($t1);

    // $t1 := opaque end: aggregator_v2::create_unbounded_aggregator<u128>() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:657:17+44

    // goto L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:656:22+204
    assume {:print "$at(2,27379,27583)"} true;
    goto L8;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:66+12
    assume {:print "$at(2,27555,27567)"} true;
L6:

    // $t22 := borrow_local($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:66+12
    assume {:print "$at(2,27555,27567)"} true;
    $t22 := $Mutation($Local(3), EmptyVec(), $t3);

    // $t23 := opaque begin: option::extract<u128>($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29

    // $t24 := read_ref($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    $t24 := $Dereference($t22);

    // assume Identical($t25, option::spec_is_none<u128>($t22)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume ($t25 == $1_option_spec_is_none'u128'($Dereference($t22)));

    // if ($t25) goto L14 else goto L17 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    if ($t25) { goto L14; } else { goto L17; }

    // label L14 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
L14:

    // assume And(option::spec_is_none<u128>($t22), Eq(262145, $t8)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume {:print "$at(2,27539,27568)"} true;
    assume ($1_option_spec_is_none'u128'($Dereference($t22)) && $IsEqual'num'(262145, $t8));

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume {:print "$at(2,27539,27568)"} true;
    assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;

    // goto L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    goto L10;

    // label L13 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
L13:

    // $t22 := havoc[mut]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume {:print "$at(2,27539,27568)"} true;
    havoc $temp_0'$1_option_Option'u128'';
    $t22 := $UpdateMutation($t22, $temp_0'$1_option_Option'u128'');

    // assume And(WellFormed($t22), Le(Len<u128>(select option::Option.vec($t22)), 1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume ($IsValid'$1_option_Option'u128''($Dereference($t22)) && (LenVec($Dereference($t22)->$vec) <= 1));

    // assume WellFormed($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume $IsValid'u128'($t23);

    // assume Eq<u128>($t23, option::spec_borrow<u128>($t24)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume $IsEqual'u128'($t23, $1_option_spec_borrow'u128'($t24));

    // assume option::spec_is_none<u128>($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume $1_option_spec_is_none'u128'($Dereference($t22));

    // $t23 := opaque end: option::extract<u128>($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29

    // assert Le(Len<u128>(select option::Option.vec($t22)), 1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    // data invariant at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:13:9+24
    assume {:print "$at(15,530,554)"} true;
    assert {:msg "assert_failed(15,530,554): data invariant does not hold"}
      (LenVec($Dereference($t22)->$vec) <= 1);

    // write_back[LocalRoot($t3)@]($t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume {:print "$at(2,27539,27568)"} true;
    $t3 := $Dereference($t22);

    // trace_local[maximum]($t3) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:50+29
    assume {:print "$track_local(56,36,3):", $t3} $t3 == $t3;

    // $t1 := opaque begin: aggregator_v2::create_aggregator<u128>($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63

    // $t26 := havoc[val]() at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63
    havoc $t26;

    // if ($t26) goto L16 else goto L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63
    if ($t26) { goto L16; } else { goto L15; }

    // label L16 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63
L16:

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63
    assume {:print "$at(2,27506,27569)"} true;
    assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;

    // goto L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63
    goto L10;

    // label L15 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63
L15:

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63
    assume {:print "$at(2,27506,27569)"} true;
    assume $IsValid'$1_aggregator_v2_Aggregator'u128''($t1);

    // $t1 := opaque end: aggregator_v2::create_aggregator<u128>($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:660:17+63

    // label L8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:656:22+204
    assume {:print "$at(2,27379,27583)"} true;
L8:

    // $t6 := pack fungible_asset::ConcurrentSupply($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:655:22+255
    assume {:print "$at(2,27339,27594)"} true;
    $t6 := $1_fungible_asset_ConcurrentSupply($t1);

    // trace_local[supply]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:655:13+6
    assume {:print "$track_local(56,36,6):", $t6} $t6 == $t6;

    // $t27 := borrow_local($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:664:33+6
    assume {:print "$at(2,27661,27667)"} true;
    $t27 := $Mutation($Local(6), EmptyVec(), $t6);

    // $t28 := borrow_field<fungible_asset::ConcurrentSupply>.current($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:664:28+19
    $t28 := $ChildMutation($t27, 0, $Dereference($t27)->$current);

    // aggregator_v2::add<u128>($t28, $t17) on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:664:9+48
    call $t28 := $1_aggregator_v2_add'u128'($t28, $t17);
    if ($abort_flag) {
        assume {:print "$at(2,27637,27685)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // write_back[Reference($t27).current (aggregator_v2::Aggregator<u128>)]($t28) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:664:9+48
    $t27 := $UpdateMutation($t27, $Update'$1_fungible_asset_ConcurrentSupply'_current($Dereference($t27), $Dereference($t28)));

    // write_back[LocalRoot($t6)@]($t27) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:664:9+48
    $t6 := $Dereference($t27);

    // trace_local[supply]($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:664:9+48
    assume {:print "$track_local(56,36,6):", $t6} $t6 == $t6;

    // $t29 := move($t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:665:42+6
    assume {:print "$at(2,27728,27734)"} true;
    $t29 := $t6;

    // move_to<fungible_asset::ConcurrentSupply>($t29, $t9) on_abort goto L10 with $t8 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:665:9+7
    if ($ResourceExists($1_fungible_asset_ConcurrentSupply_$memory, $t9->$addr)) {
        call $ExecFailureAbort();
    } else {
        $1_fungible_asset_ConcurrentSupply_$memory := $ResourceUpdate($1_fungible_asset_ConcurrentSupply_$memory, $t9->$addr, $t29);
    }
    if ($abort_flag) {
        assume {:print "$at(2,27695,27702)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(56,36):", $t8} $t8 == $t8;
        goto L10;
    }

    // label L9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:666:5+1
    assume {:print "$at(2,27741,27742)"} true;
L9:

    // assert Not(false) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:666:5+1
    assume {:print "$at(2,27741,27742)"} true;
    assert {:msg "assert_failed(2,27741,27742): function does not abort under this condition"}
      !false;

    // return () at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:666:5+1
    return;

    // label L10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:666:5+1
L10:

    // abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:666:5+1
    assume {:print "$at(2,27741,27742)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

    // label L17 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L17:

    // destroy($t22) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L13 at <internal>:1:1+10
    goto L13;

}

// fun fungible_asset::withdraw_internal [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+667
procedure {:inline 1} $1_fungible_asset_withdraw_internal(_$t0: int, _$t1: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t2: $Mutation ($1_fungible_asset_FungibleAssetEvents);
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $Mutation ($1_fungible_asset_FungibleStore);
    var $t5: $1_fungible_asset_FungibleStore;
    var $t6: int;
    var $t7: bool;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $Mutation ($1_fungible_asset_FungibleStore);
    var $t12: int;
    var $t13: bool;
    var $t14: int;
    var $t15: int;
    var $t16: int;
    var $t17: int;
    var $t18: $Mutation (int);
    var $t19: $Mutation ($1_fungible_asset_FungibleAssetEvents);
    var $t20: $1_object_Object'$1_fungible_asset_Metadata';
    var $t21: $Mutation ($1_event_EventHandle'$1_fungible_asset_WithdrawEvent');
    var $t22: $1_fungible_asset_WithdrawEvent;
    var $t23: $1_fungible_asset_FungibleAsset;
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAssetEvents': $1_fungible_asset_FungibleAssetEvents;
    var $temp_0'$1_fungible_asset_FungibleStore': $1_fungible_asset_FungibleStore;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t5, global<fungible_asset::FungibleStore>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:431:9+46
    assume {:print "$at(3,17998,18044)"} true;
    assume ($t5 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t0));

    // trace_local[store_addr]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume {:print "$at(2,23058,23059)"} true;
    assume {:print "$track_local(56,38,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume {:print "$track_local(56,38,1):", $t1} $t1 == $t1;

    // $t6 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:27+1
    assume {:print "$at(2,23224,23225)"} true;
    $t6 := 0;
    assume $IsValid'u64'($t6);

    // $t7 := !=($t1, $t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:24+2
    $t7 := !$IsEqual'u64'($t1, $t6);

    // if ($t7) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    assume {:print "$at(2,23206,23275)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:54+22
L0:

    // $t8 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:54+22
    assume {:print "$at(2,23251,23273)"} true;
    $t8 := 1;
    assume $IsValid'u64'($t8);

    // $t9 := error::invalid_argument($t8) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:30+47
    call $t9 := $1_error_invalid_argument($t8);
    if ($abort_flag) {
        assume {:print "$at(2,23227,23274)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    assume {:print "$at(2,23206,23275)"} true;
    assume {:print "$track_abort(56,38):", $t9} $t9 == $t9;

    // $t10 := move($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    $t10 := $t9;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    goto L7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:562:54+10
    assume {:print "$at(2,23330,23340)"} true;
L2:

    // $t11 := borrow_global<fungible_asset::FungibleStore>($t0) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:562:21+17
    assume {:print "$at(2,23297,23314)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t11 := $Mutation($Global($t0), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t0));
    }
    if ($abort_flag) {
        assume {:print "$at(2,23297,23314)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_local[store]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:562:13+5
    $temp_0'$1_fungible_asset_FungibleStore' := $Dereference($t11);
    assume {:print "$track_local(56,38,4):", $temp_0'$1_fungible_asset_FungibleStore'} $temp_0'$1_fungible_asset_FungibleStore' == $temp_0'$1_fungible_asset_FungibleStore';

    // $t12 := get_field<fungible_asset::FungibleStore>.balance($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:17+13
    assume {:print "$at(2,23359,23372)"} true;
    $t12 := $Dereference($t11)->$balance;

    // $t13 := >=($t12, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:31+2
    call $t13 := $Ge($t12, $t1);

    // if ($t13) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    if ($t13) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    assume {:print "$at(2,23351,23431)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
L3:

    // destroy($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    assume {:print "$at(2,23351,23431)"} true;

    // $t14 := 4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:66+21
    $t14 := 4;
    assume $IsValid'u64'($t14);

    // $t15 := error::invalid_argument($t14) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:42+46
    call $t15 := $1_error_invalid_argument($t14);
    if ($abort_flag) {
        assume {:print "$at(2,23384,23430)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_abort($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    assume {:print "$at(2,23351,23431)"} true;
    assume {:print "$track_abort(56,38):", $t15} $t15 == $t15;

    // $t10 := move($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    $t10 := $t15;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:25+5
    assume {:print "$at(2,23457,23462)"} true;
L5:

    // $t16 := get_field<fungible_asset::FungibleStore>.balance($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:25+13
    assume {:print "$at(2,23457,23470)"} true;
    $t16 := $Dereference($t11)->$balance;

    // $t17 := -($t16, $t1) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:39+1
    call $t17 := $Sub($t16, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,23471,23472)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // $t18 := borrow_field<fungible_asset::FungibleStore>.balance($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:9+13
    $t18 := $ChildMutation($t11, 1, $Dereference($t11)->$balance);

    // write_ref($t18, $t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:9+38
    $t18 := $UpdateMutation($t18, $t17);

    // write_back[Reference($t11).balance (u64)]($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:9+38
    $t11 := $UpdateMutation($t11, $Update'$1_fungible_asset_FungibleStore'_balance($Dereference($t11), $Dereference($t18)));

    // $t19 := borrow_global<fungible_asset::FungibleAssetEvents>($t0) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:566:22+17
    assume {:print "$at(2,23503,23520)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t19 := $Mutation($Global($t0), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $t0));
    }
    if ($abort_flag) {
        assume {:print "$at(2,23503,23520)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_local[events]($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:566:13+6
    $temp_0'$1_fungible_asset_FungibleAssetEvents' := $Dereference($t19);
    assume {:print "$track_local(56,38,2):", $temp_0'$1_fungible_asset_FungibleAssetEvents'} $temp_0'$1_fungible_asset_FungibleAssetEvents' == $temp_0'$1_fungible_asset_FungibleAssetEvents';

    // $t20 := get_field<fungible_asset::FungibleStore>.metadata($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:567:24+14
    assume {:print "$at(2,23578,23592)"} true;
    $t20 := $Dereference($t11)->$metadata;

    // write_back[fungible_asset::FungibleStore@]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:567:24+14
    $1_fungible_asset_FungibleStore_$memory := $ResourceUpdate($1_fungible_asset_FungibleStore_$memory, $GlobalLocationAddress($t11),
        $Dereference($t11));

    // trace_local[metadata]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:567:13+8
    assume {:print "$track_local(56,38,3):", $t20} $t20 == $t20;

    // $t21 := borrow_field<fungible_asset::FungibleAssetEvents>.withdraw_events($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:27+27
    assume {:print "$at(2,23620,23647)"} true;
    $t21 := $ChildMutation($t19, 1, $Dereference($t19)->$withdraw_events);

    // $t22 := pack fungible_asset::WithdrawEvent($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:56+24
    $t22 := $1_fungible_asset_WithdrawEvent($t1);

    // opaque begin: event::emit_event<fungible_asset::WithdrawEvent>($t21, $t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72

    // opaque end: event::emit_event<fungible_asset::WithdrawEvent>($t21, $t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72

    // write_back[Reference($t19).withdraw_events (event::EventHandle<fungible_asset::WithdrawEvent>)]($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72
    $t19 := $UpdateMutation($t19, $Update'$1_fungible_asset_FungibleAssetEvents'_withdraw_events($Dereference($t19), $Dereference($t21)));

    // write_back[fungible_asset::FungibleAssetEvents@]($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72
    $1_fungible_asset_FungibleAssetEvents_$memory := $ResourceUpdate($1_fungible_asset_FungibleAssetEvents_$memory, $GlobalLocationAddress($t19),
        $Dereference($t19));

    // $t23 := pack fungible_asset::FungibleAsset($t20, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:570:9+34
    assume {:print "$at(2,23685,23719)"} true;
    $t23 := $1_fungible_asset_FungibleAsset($t20, $t1);

    // trace_return[0]($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:570:9+34
    assume {:print "$track_return(56,38,0):", $t23} $t23 == $t23;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:571:5+1
    assume {:print "$at(2,23724,23725)"} true;
L6:

    // return $t23 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:571:5+1
    assume {:print "$at(2,23724,23725)"} true;
    $ret0 := $t23;
    return;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:571:5+1
L7:

    // abort($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:571:5+1
    assume {:print "$at(2,23724,23725)"} true;
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun fungible_asset::withdraw_internal [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+667
procedure {:timeLimit 40} $1_fungible_asset_withdraw_internal$verify(_$t0: int, _$t1: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t2: $Mutation ($1_fungible_asset_FungibleAssetEvents);
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $Mutation ($1_fungible_asset_FungibleStore);
    var $t5: $1_fungible_asset_FungibleStore;
    var $t6: int;
    var $t7: bool;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $Mutation ($1_fungible_asset_FungibleStore);
    var $t12: int;
    var $t13: bool;
    var $t14: int;
    var $t15: int;
    var $t16: int;
    var $t17: int;
    var $t18: $Mutation (int);
    var $t19: $Mutation ($1_fungible_asset_FungibleAssetEvents);
    var $t20: $1_object_Object'$1_fungible_asset_Metadata';
    var $t21: $Mutation ($1_event_EventHandle'$1_fungible_asset_WithdrawEvent');
    var $t22: $1_fungible_asset_WithdrawEvent;
    var $t23: $1_fungible_asset_FungibleAsset;
    var $t24: int;
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_FungibleAssetEvents': $1_fungible_asset_FungibleAssetEvents;
    var $temp_0'$1_fungible_asset_FungibleStore': $1_fungible_asset_FungibleStore;
    var $temp_0'$1_object_Object'$1_fungible_asset_Metadata'': $1_object_Object'$1_fungible_asset_Metadata';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    var $1_fungible_asset_FungibleStore_$memory#18: $Memory $1_fungible_asset_FungibleStore;
    var $1_fungible_asset_FungibleAssetEvents_$memory#19: $Memory $1_fungible_asset_FungibleAssetEvents;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume {:print "$at(2,23058,23059)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume $IsValid'u64'($t1);

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // assume Identical($t5, global<fungible_asset::FungibleStore>($t0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:431:9+46
    assume {:print "$at(3,17998,18044)"} true;
    assume ($t5 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t0));

    // @19 := save_mem(fungible_asset::FungibleAssetEvents) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume {:print "$at(2,23058,23059)"} true;
    $1_fungible_asset_FungibleAssetEvents_$memory#19 := $1_fungible_asset_FungibleAssetEvents_$memory;

    // @18 := save_mem(fungible_asset::FungibleStore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    $1_fungible_asset_FungibleStore_$memory#18 := $1_fungible_asset_FungibleStore_$memory;

    // trace_local[store_addr]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume {:print "$track_local(56,38,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:557:5+1
    assume {:print "$track_local(56,38,1):", $t1} $t1 == $t1;

    // $t6 := 0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:27+1
    assume {:print "$at(2,23224,23225)"} true;
    $t6 := 0;
    assume $IsValid'u64'($t6);

    // $t7 := !=($t1, $t6) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:24+2
    $t7 := !$IsEqual'u64'($t1, $t6);

    // if ($t7) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    assume {:print "$at(2,23206,23275)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:54+22
L0:

    // $t8 := 1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:54+22
    assume {:print "$at(2,23251,23273)"} true;
    $t8 := 1;
    assume $IsValid'u64'($t8);

    // $t9 := error::invalid_argument($t8) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:30+47
    call $t9 := $1_error_invalid_argument($t8);
    if ($abort_flag) {
        assume {:print "$at(2,23227,23274)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_abort($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    assume {:print "$at(2,23206,23275)"} true;
    assume {:print "$track_abort(56,38):", $t9} $t9 == $t9;

    // $t10 := move($t9) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    $t10 := $t9;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:561:9+69
    goto L7;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:562:54+10
    assume {:print "$at(2,23330,23340)"} true;
L2:

    // $t11 := borrow_global<fungible_asset::FungibleStore>($t0) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:562:21+17
    assume {:print "$at(2,23297,23314)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleStore_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t11 := $Mutation($Global($t0), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t0));
    }
    if ($abort_flag) {
        assume {:print "$at(2,23297,23314)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_local[store]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:562:13+5
    $temp_0'$1_fungible_asset_FungibleStore' := $Dereference($t11);
    assume {:print "$track_local(56,38,4):", $temp_0'$1_fungible_asset_FungibleStore'} $temp_0'$1_fungible_asset_FungibleStore' == $temp_0'$1_fungible_asset_FungibleStore';

    // $t12 := get_field<fungible_asset::FungibleStore>.balance($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:17+13
    assume {:print "$at(2,23359,23372)"} true;
    $t12 := $Dereference($t11)->$balance;

    // $t13 := >=($t12, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:31+2
    call $t13 := $Ge($t12, $t1);

    // if ($t13) goto L4 else goto L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    if ($t13) { goto L4; } else { goto L3; }

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
L4:

    // goto L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    assume {:print "$at(2,23351,23431)"} true;
    goto L5;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
L3:

    // destroy($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    assume {:print "$at(2,23351,23431)"} true;

    // $t14 := 4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:66+21
    $t14 := 4;
    assume $IsValid'u64'($t14);

    // $t15 := error::invalid_argument($t14) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:42+46
    call $t15 := $1_error_invalid_argument($t14);
    if ($abort_flag) {
        assume {:print "$at(2,23384,23430)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_abort($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    assume {:print "$at(2,23351,23431)"} true;
    assume {:print "$track_abort(56,38):", $t15} $t15 == $t15;

    // $t10 := move($t15) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    $t10 := $t15;

    // goto L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:563:9+80
    goto L7;

    // label L5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:25+5
    assume {:print "$at(2,23457,23462)"} true;
L5:

    // $t16 := get_field<fungible_asset::FungibleStore>.balance($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:25+13
    assume {:print "$at(2,23457,23470)"} true;
    $t16 := $Dereference($t11)->$balance;

    // $t17 := -($t16, $t1) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:39+1
    call $t17 := $Sub($t16, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,23471,23472)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // $t18 := borrow_field<fungible_asset::FungibleStore>.balance($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:9+13
    $t18 := $ChildMutation($t11, 1, $Dereference($t11)->$balance);

    // write_ref($t18, $t17) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:9+38
    $t18 := $UpdateMutation($t18, $t17);

    // write_back[Reference($t11).balance (u64)]($t18) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:564:9+38
    $t11 := $UpdateMutation($t11, $Update'$1_fungible_asset_FungibleStore'_balance($Dereference($t11), $Dereference($t18)));

    // $t19 := borrow_global<fungible_asset::FungibleAssetEvents>($t0) on_abort goto L7 with $t10 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:566:22+17
    assume {:print "$at(2,23503,23520)"} true;
    if (!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t19 := $Mutation($Global($t0), EmptyVec(), $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $t0));
    }
    if ($abort_flag) {
        assume {:print "$at(2,23503,23520)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(56,38):", $t10} $t10 == $t10;
        goto L7;
    }

    // trace_local[events]($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:566:13+6
    $temp_0'$1_fungible_asset_FungibleAssetEvents' := $Dereference($t19);
    assume {:print "$track_local(56,38,2):", $temp_0'$1_fungible_asset_FungibleAssetEvents'} $temp_0'$1_fungible_asset_FungibleAssetEvents' == $temp_0'$1_fungible_asset_FungibleAssetEvents';

    // $t20 := get_field<fungible_asset::FungibleStore>.metadata($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:567:24+14
    assume {:print "$at(2,23578,23592)"} true;
    $t20 := $Dereference($t11)->$metadata;

    // write_back[fungible_asset::FungibleStore@]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:567:24+14
    $1_fungible_asset_FungibleStore_$memory := $ResourceUpdate($1_fungible_asset_FungibleStore_$memory, $GlobalLocationAddress($t11),
        $Dereference($t11));

    // trace_local[metadata]($t20) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:567:13+8
    assume {:print "$track_local(56,38,3):", $t20} $t20 == $t20;

    // $t21 := borrow_field<fungible_asset::FungibleAssetEvents>.withdraw_events($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:27+27
    assume {:print "$at(2,23620,23647)"} true;
    $t21 := $ChildMutation($t19, 1, $Dereference($t19)->$withdraw_events);

    // $t22 := pack fungible_asset::WithdrawEvent($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:56+24
    $t22 := $1_fungible_asset_WithdrawEvent($t1);

    // opaque begin: event::emit_event<fungible_asset::WithdrawEvent>($t21, $t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72

    // opaque end: event::emit_event<fungible_asset::WithdrawEvent>($t21, $t22) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72

    // write_back[Reference($t19).withdraw_events (event::EventHandle<fungible_asset::WithdrawEvent>)]($t21) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72
    $t19 := $UpdateMutation($t19, $Update'$1_fungible_asset_FungibleAssetEvents'_withdraw_events($Dereference($t19), $Dereference($t21)));

    // write_back[fungible_asset::FungibleAssetEvents@]($t19) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:568:9+72
    $1_fungible_asset_FungibleAssetEvents_$memory := $ResourceUpdate($1_fungible_asset_FungibleAssetEvents_$memory, $GlobalLocationAddress($t19),
        $Dereference($t19));

    // $t23 := pack fungible_asset::FungibleAsset($t20, $t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:570:9+34
    assume {:print "$at(2,23685,23719)"} true;
    $t23 := $1_fungible_asset_FungibleAsset($t20, $t1);

    // trace_return[0]($t23) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:570:9+34
    assume {:print "$track_return(56,38,0):", $t23} $t23 == $t23;

    // label L6 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:571:5+1
    assume {:print "$at(2,23724,23725)"} true;
L6:

    // assume Identical($t24, select fungible_asset::FungibleStore.balance(global<fungible_asset::FungibleStore>($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:437:9+67
    assume {:print "$at(3,18245,18312)"} true;
    assume ($t24 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t0)->$balance);

    // assert Not(Eq<u64>($t1, 0)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:429:9+24
    assume {:print "$at(3,17964,17988)"} true;
    assert {:msg "assert_failed(3,17964,17988): function does not abort under this condition"}
      !$IsEqual'u64'($t1, 0);

    // assert Not(Lt(select fungible_asset::FungibleStore.balance($t5), $t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:433:9+35
    assume {:print "$at(3,18086,18121)"} true;
    assert {:msg "assert_failed(3,18086,18121): function does not abort under this condition"}
      !($t5->$balance < $t1);

    // assert Not(Not(exists[@18]<fungible_asset::FungibleStore>($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:434:9+45
    assume {:print "$at(3,18130,18175)"} true;
    assert {:msg "assert_failed(3,18130,18175): function does not abort under this condition"}
      !!$ResourceExists($1_fungible_asset_FungibleStore_$memory#18, $t0);

    // assert Not(Not(exists[@19]<fungible_asset::FungibleAssetEvents>($t0))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:435:9+51
    assume {:print "$at(3,18184,18235)"} true;
    assert {:msg "assert_failed(3,18184,18235): function does not abort under this condition"}
      !!$ResourceExists($1_fungible_asset_FungibleAssetEvents_$memory#19, $t0);

    // assert Eq<u64>($t24, Sub(select fungible_asset::FungibleStore.balance($t5), $t1)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:438:9+48
    assume {:print "$at(3,18321,18369)"} true;
    assert {:msg "assert_failed(3,18321,18369): post-condition does not hold"}
      $IsEqual'u64'($t24, ($t5->$balance - $t1));

    // return $t23 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:438:9+48
    $ret0 := $t23;
    return;

    // label L7 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:571:5+1
    assume {:print "$at(2,23724,23725)"} true;
L7:

    // abort($t10) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:571:5+1
    assume {:print "$at(2,23724,23725)"} true;
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun fungible_asset::withdraw_with_ref<#0> [baseline] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+399
procedure {:inline 1} $1_fungible_asset_withdraw_with_ref'#0'(_$t0: $1_fungible_asset_TransferRef, _$t1: $1_object_Object'#0', _$t2: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: $1_fungible_asset_FungibleStore;
    var $t11: $1_fungible_asset_FungibleAsset;
    var $t0: $1_fungible_asset_TransferRef;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume {:print "$at(2,19481,19482)"} true;
    assume {:print "$track_local(56,39,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume {:print "$track_local(56,39,1):", $t1} $t1 == $t1;

    // trace_local[amount]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume {:print "$track_local(56,39,2):", $t2} $t2 == $t2;

    // $t3 := get_field<fungible_asset::TransferRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:479:13+12
    assume {:print "$at(2,19688,19700)"} true;
    $t3 := $t0->$metadata;

    // $t4 := fungible_asset::store_metadata<#0>($t1) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:479:29+21
    call $t4 := $1_fungible_asset_store_metadata'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,19704,19725)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t6 := ==($t3, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:479:26+2
    $t6 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t3, $t4);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    assume {:print "$at(2,19667,19807)"} true;
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    assume {:print "$at(2,19667,19807)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:480:37+32
    assume {:print "$at(2,19763,19795)"} true;
L0:

    // $t7 := 9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:480:37+32
    assume {:print "$at(2,19763,19795)"} true;
    $t7 := 9;
    assume $IsValid'u64'($t7);

    // $t8 := error::invalid_argument($t7) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:480:13+57
    call $t8 := $1_error_invalid_argument($t7);
    if ($abort_flag) {
        assume {:print "$at(2,19739,19796)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    assume {:print "$at(2,19667,19807)"} true;
    assume {:print "$track_abort(56,39):", $t8} $t8 == $t8;

    // $t5 := move($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    $t5 := $t8;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:50+6
    assume {:print "$at(2,19858,19864)"} true;
L2:

    // $t9 := object::object_address<#0>($t1) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:27+30
    assume {:print "$at(2,19835,19865)"} true;
    call $t9 := $1_object_object_address'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,19835,19865)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // assume Identical($t10, global<fungible_asset::FungibleStore>($t9)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:431:9+46
    assume {:print "$at(3,17998,18044)"} true;
    assume ($t10 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t9));

    // $t11 := fungible_asset::withdraw_internal($t9, $t2) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:9+57
    assume {:print "$at(2,19817,19874)"} true;
    call $t11 := $1_fungible_asset_withdraw_internal($t9, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,19817,19874)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_return[0]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:9+57
    assume {:print "$track_return(56,39,0):", $t11} $t11 == $t11;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:483:5+1
    assume {:print "$at(2,19879,19880)"} true;
L3:

    // return $t11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:483:5+1
    assume {:print "$at(2,19879,19880)"} true;
    $ret0 := $t11;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:483:5+1
L4:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:483:5+1
    assume {:print "$at(2,19879,19880)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun fungible_asset::withdraw_with_ref [verification] at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+399
procedure {:timeLimit 40} $1_fungible_asset_withdraw_with_ref$verify(_$t0: $1_fungible_asset_TransferRef, _$t1: $1_object_Object'#0', _$t2: int) returns ($ret0: $1_fungible_asset_FungibleAsset)
{
    // declare local variables
    var $t3: $1_object_Object'$1_fungible_asset_Metadata';
    var $t4: $1_object_Object'$1_fungible_asset_Metadata';
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: $1_fungible_asset_FungibleStore;
    var $t11: $1_fungible_asset_FungibleAsset;
    var $t0: $1_fungible_asset_TransferRef;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_fungible_asset_FungibleAsset': $1_fungible_asset_FungibleAsset;
    var $temp_0'$1_fungible_asset_TransferRef': $1_fungible_asset_TransferRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'u64': int;
    var $1_fungible_asset_FungibleStore_$memory#32: $Memory $1_fungible_asset_FungibleStore;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume {:print "$at(2,19481,19482)"} true;
    assume $IsValid'$1_fungible_asset_TransferRef'($t0);

    // assume WellFormed($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume $IsValid'u64'($t2);

    // assume forall $rsc: fungible_asset::FungibleAssetEvents: ResourceDomain<fungible_asset::FungibleAssetEvents>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleAssetEvents_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleAssetEvents'($rsc))));

    // assume forall $rsc: fungible_asset::FungibleStore: ResourceDomain<fungible_asset::FungibleStore>(): WellFormed($rsc) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0)}(var $rsc := $ResourceValue($1_fungible_asset_FungibleStore_$memory, $a_0);
    ($IsValid'$1_fungible_asset_FungibleStore'($rsc))));

    // @32 := save_mem(fungible_asset::FungibleStore) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    $1_fungible_asset_FungibleStore_$memory#32 := $1_fungible_asset_FungibleStore_$memory;

    // trace_local[ref]($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume {:print "$track_local(56,39,0):", $t0} $t0 == $t0;

    // trace_local[store]($t1) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume {:print "$track_local(56,39,1):", $t1} $t1 == $t1;

    // trace_local[amount]($t2) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:473:5+1
    assume {:print "$track_local(56,39,2):", $t2} $t2 == $t2;

    // $t3 := get_field<fungible_asset::TransferRef>.metadata($t0) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:479:13+12
    assume {:print "$at(2,19688,19700)"} true;
    $t3 := $t0->$metadata;

    // $t4 := fungible_asset::store_metadata<#0>($t1) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:479:29+21
    call $t4 := $1_fungible_asset_store_metadata'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,19704,19725)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t6 := ==($t3, $t4) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:479:26+2
    $t6 := $IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t3, $t4);

    // if ($t6) goto L1 else goto L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    assume {:print "$at(2,19667,19807)"} true;
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
L1:

    // goto L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    assume {:print "$at(2,19667,19807)"} true;
    goto L2;

    // label L0 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:480:37+32
    assume {:print "$at(2,19763,19795)"} true;
L0:

    // $t7 := 9 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:480:37+32
    assume {:print "$at(2,19763,19795)"} true;
    $t7 := 9;
    assume $IsValid'u64'($t7);

    // $t8 := error::invalid_argument($t7) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:480:13+57
    call $t8 := $1_error_invalid_argument($t7);
    if ($abort_flag) {
        assume {:print "$at(2,19739,19796)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    assume {:print "$at(2,19667,19807)"} true;
    assume {:print "$track_abort(56,39):", $t8} $t8 == $t8;

    // $t5 := move($t8) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    $t5 := $t8;

    // goto L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:478:9+140
    goto L4;

    // label L2 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:50+6
    assume {:print "$at(2,19858,19864)"} true;
L2:

    // $t9 := object::object_address<#0>($t1) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:27+30
    assume {:print "$at(2,19835,19865)"} true;
    call $t9 := $1_object_object_address'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,19835,19865)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // assume Identical($t10, global<fungible_asset::FungibleStore>($t9)) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:431:9+46
    assume {:print "$at(3,17998,18044)"} true;
    assume ($t10 == $ResourceValue($1_fungible_asset_FungibleStore_$memory, $t9));

    // $t11 := fungible_asset::withdraw_internal($t9, $t2) on_abort goto L4 with $t5 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:9+57
    assume {:print "$at(2,19817,19874)"} true;
    call $t11 := $1_fungible_asset_withdraw_internal($t9, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,19817,19874)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(56,39):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_return[0]($t11) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:482:9+57
    assume {:print "$track_return(56,39,0):", $t11} $t11 == $t11;

    // label L3 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:483:5+1
    assume {:print "$at(2,19879,19880)"} true;
L3:

    // assert Not(Neq<object::Object<fungible_asset::Metadata>>(select fungible_asset::TransferRef.metadata($t0), fungible_asset::$store_metadata[@32]<#0>($t1))) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:371:9+50
    assume {:print "$at(3,16262,16312)"} true;
    assert {:msg "assert_failed(3,16262,16312): function does not abort under this condition"}
      !!$IsEqual'$1_object_Object'$1_fungible_asset_Metadata''($t0->$metadata, $1_fungible_asset_$store_metadata'#0'($1_fungible_asset_FungibleStore_$memory#32, $t1));

    // return $t11 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.spec.move:371:9+50
    $ret0 := $t11;
    return;

    // label L4 at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:483:5+1
    assume {:print "$at(2,19879,19880)"} true;
L4:

    // abort($t5) at /Users/aalok/aptos-core/aptos-move/framework/aptos-framework/sources/fungible_asset.move:483:5+1
    assume {:print "$at(2,19879,19880)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}
