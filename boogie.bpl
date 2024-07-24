
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

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'bool'(b1), $1_from_bcs_deserializable'bool'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u64'(b1), $1_from_bcs_deserializable'u64'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u256'(b1), $1_from_bcs_deserializable'u256'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'address'(b1), $1_from_bcs_deserializable'address'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'signer'(b1), $1_from_bcs_deserializable'signer'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'u8''(b1), $1_from_bcs_deserializable'vec'u8''(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_GUID'(b1), $1_from_bcs_deserializable'$1_guid_GUID'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_ID'(b1), $1_from_bcs_deserializable'$1_guid_ID'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<object::TransferEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ConstructorRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ConstructorRef'(b1), $1_from_bcs_deserializable'$1_object_ConstructorRef'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ExtendRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ExtendRef'(b1), $1_from_bcs_deserializable'$1_object_ExtendRef'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ObjectCore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ObjectCore'(b1), $1_from_bcs_deserializable'$1_object_ObjectCore'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <state::State>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$cafe_state_State'(b1), $1_from_bcs_deserializable'$cafe_state_State'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <state::Management>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$cafe_state_Management'(b1), $1_from_bcs_deserializable'$cafe_state_Management'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'#0'(b1), $1_from_bcs_deserializable'#0'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserialize'bool'(b1), $1_from_bcs_deserialize'bool'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u64'($1_from_bcs_deserialize'u64'(b1), $1_from_bcs_deserialize'u64'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u256'($1_from_bcs_deserialize'u256'(b1), $1_from_bcs_deserialize'u256'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'address'($1_from_bcs_deserialize'address'(b1), $1_from_bcs_deserialize'address'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'signer'($1_from_bcs_deserialize'signer'(b1), $1_from_bcs_deserialize'signer'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'u8''($1_from_bcs_deserialize'vec'u8''(b1), $1_from_bcs_deserialize'vec'u8''(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_GUID'($1_from_bcs_deserialize'$1_guid_GUID'(b1), $1_from_bcs_deserialize'$1_guid_GUID'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_ID'($1_from_bcs_deserialize'$1_guid_ID'(b1), $1_from_bcs_deserialize'$1_guid_ID'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<object::TransferEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_object_TransferEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ConstructorRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ConstructorRef'($1_from_bcs_deserialize'$1_object_ConstructorRef'(b1), $1_from_bcs_deserialize'$1_object_ConstructorRef'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ExtendRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ExtendRef'($1_from_bcs_deserialize'$1_object_ExtendRef'(b1), $1_from_bcs_deserialize'$1_object_ExtendRef'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ObjectCore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ObjectCore'($1_from_bcs_deserialize'$1_object_ObjectCore'(b1), $1_from_bcs_deserialize'$1_object_ObjectCore'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <state::State>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$cafe_state_State'($1_from_bcs_deserialize'$cafe_state_State'(b1), $1_from_bcs_deserialize'$cafe_state_State'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <state::Management>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$cafe_state_Management'($1_from_bcs_deserialize'$cafe_state_Management'(b1), $1_from_bcs_deserialize'$cafe_state_Management'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'#0'($1_from_bcs_deserialize'#0'(b1), $1_from_bcs_deserialize'#0'(b2)))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:8:9+113
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_keccak256(b1), $1_aptos_hash_spec_keccak256(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:13:9+129
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_sha2_512_internal(b1), $1_aptos_hash_spec_sha2_512_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:18:9+129
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_sha3_512_internal(b1), $1_aptos_hash_spec_sha3_512_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:23:9+131
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_ripemd160_internal(b1), $1_aptos_hash_spec_ripemd160_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:28:9+135
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_blake2b_256_internal(b1), $1_aptos_hash_spec_blake2b_256_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
function {:inline} $1_signer_$address_of(s: $signer): int {
    $1_signer_$borrow_address(s)
}

// fun signer::address_of [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
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
    // trace_local[s]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+1
    assume {:print "$at(14,395,396)"} true;
    assume {:print "$track_local(3,0,0):", $t0} $t0 == $t0;

    // $t1 := signer::borrow_address($t0) on_abort goto L2 with $t2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:10+17
    assume {:print "$at(14,449,466)"} true;
    call $t1 := $1_signer_borrow_address($t0);
    if ($abort_flag) {
        assume {:print "$at(14,449,466)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(3,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_return[0]($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:9+18
    assume {:print "$track_return(3,0,0):", $t1} $t1 == $t1;

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(14,471,472)"} true;
L1:

    // return $t1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(14,471,472)"} true;
    $ret0 := $t1;
    return;

    // label L2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
L2:

    // abort($t2) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(14,471,472)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun error::already_exists [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:3+71
procedure {:inline 1} $1_error_already_exists(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t0: int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[r]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:3+1
    assume {:print "$at(10,3585,3586)"} true;
    assume {:print "$track_local(4,1,0):", $t0} $t0 == $t0;

    // $t1 := 8 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:54+14
    $t1 := 8;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(10,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume {:print "$at(10,3626,3654)"} true;

    // assume WellFormed($t3) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28

    // trace_return[0]($t3) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume {:print "$track_return(4,1,0):", $t3} $t3 == $t3;

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:73+1
L1:

    // return $t3 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:73+1
    assume {:print "$at(10,3655,3656)"} true;
    $ret0 := $t3;
    return;

}

// struct guid::GUID at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:7:5+50
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

// struct guid::ID at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:12:5+209
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

// fun guid::create [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:23:5+286
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
    // trace_local[addr]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:23:5+1
    assume {:print "$at(155,836,837)"} true;
    assume {:print "$track_local(13,0,0):", $t0} $t0 == $t0;

    // trace_local[creation_num_ref]($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:23:5+1
    $temp_0'u64' := $Dereference($t1);
    assume {:print "$track_local(13,0,1):", $temp_0'u64'} $temp_0'u64' == $temp_0'u64';

    // $t3 := read_ref($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:24:28+17
    assume {:print "$at(155,940,957)"} true;
    $t3 := $Dereference($t1);

    // trace_local[creation_num]($t3) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:24:13+12
    assume {:print "$track_local(13,0,2):", $t3} $t3 == $t3;

    // $t4 := 1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:25:44+1
    assume {:print "$at(155,1002,1003)"} true;
    $t4 := 1;
    assume $IsValid'u64'($t4);

    // $t5 := +($t3, $t4) on_abort goto L2 with $t6 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:25:42+1
    call $t5 := $AddU64($t3, $t4);
    if ($abort_flag) {
        assume {:print "$at(155,1000,1001)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(13,0):", $t6} $t6 == $t6;
        goto L2;
    }

    // write_ref($t1, $t5) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:25:9+36
    $t1 := $UpdateMutation($t1, $t5);

    // $t7 := pack guid::ID($t3, $t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:27:17+70
    assume {:print "$at(155,1036,1106)"} true;
    $t7 := $1_guid_ID($t3, $t0);

    // $t8 := pack guid::GUID($t7) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    assume {:print "$at(155,1013,1116)"} true;
    $t8 := $1_guid_GUID($t7);

    // trace_return[0]($t8) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    assume {:print "$track_return(13,0,0):", $t8} $t8 == $t8;

    // trace_local[creation_num_ref]($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    $temp_0'u64' := $Dereference($t1);
    assume {:print "$track_local(13,0,1):", $temp_0'u64'} $temp_0'u64' == $temp_0'u64';

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(155,1121,1122)"} true;
L1:

    // return $t8 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(155,1121,1122)"} true;
    $ret0 := $t8;
    $ret1 := $t1;
    return;

    // label L2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
L2:

    // abort($t6) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(155,1121,1122)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u64'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u64'(bytes);
$IsValid'u64'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u256'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u256'(bytes);
$IsValid'u256'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'address'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'address'(bytes);
$IsValid'address'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'signer'(bytes: Vec (int)): $signer;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'signer'(bytes);
$IsValid'signer'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'u8''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'u8''(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_GUID'(bytes: Vec (int)): $1_guid_GUID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_GUID'(bytes);
$IsValid'$1_guid_GUID'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_ID'(bytes: Vec (int)): $1_guid_ID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_ID'(bytes);
$IsValid'$1_guid_ID'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_object_TransferEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_object_TransferEvent''($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ConstructorRef'(bytes: Vec (int)): $1_object_ConstructorRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ConstructorRef'(bytes);
$IsValid'$1_object_ConstructorRef'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ExtendRef'(bytes: Vec (int)): $1_object_ExtendRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ExtendRef'(bytes);
$IsValid'$1_object_ExtendRef'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ObjectCore'(bytes: Vec (int)): $1_object_ObjectCore;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ObjectCore'(bytes);
$IsValid'$1_object_ObjectCore'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$cafe_state_State'(bytes: Vec (int)): $cafe_state_State;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$cafe_state_State'(bytes);
$IsValid'$cafe_state_State'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$cafe_state_Management'(bytes: Vec (int)): $cafe_state_Management;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$cafe_state_Management'(bytes);
$IsValid'$cafe_state_Management'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'#0'(bytes: Vec (int)): #0;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'#0'(bytes);
$IsValid'#0'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u64'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u64'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u256'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u256'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'address'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'address'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'signer'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'signer'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'u8''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'u8''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_GUID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_GUID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_ID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_ID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ConstructorRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ConstructorRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ExtendRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ExtendRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ObjectCore'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ObjectCore'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$cafe_state_State'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$cafe_state_State'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$cafe_state_Management'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$cafe_state_Management'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'#0'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'#0'(bytes);
$IsValid'bool'($$res)));

// struct event::EventHandle<object::TransferEvent> at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:34:5+224
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

// fun event::new_event_handle<object::TransferEvent> [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:43:5+165
procedure {:inline 1} $1_event_new_event_handle'$1_object_TransferEvent'(_$t0: $1_guid_GUID) returns ($ret0: $1_event_EventHandle'$1_object_TransferEvent')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_event_EventHandle'$1_object_TransferEvent';
    var $t0: $1_guid_GUID;
    var $temp_0'$1_event_EventHandle'$1_object_TransferEvent'': $1_event_EventHandle'$1_object_TransferEvent';
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[guid]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:43:5+1
    assume {:print "$at(145,1543,1544)"} true;
    assume {:print "$track_local(15,5,0):", $t0} $t0 == $t0;

    // $t1 := 0 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:45:22+1
    assume {:print "$at(145,1672,1673)"} true;
    $t1 := 0;
    assume $IsValid'u64'($t1);

    // $t2 := pack event::EventHandle<#0>($t1, $t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$at(145,1634,1702)"} true;
    $t2 := $1_event_EventHandle'$1_object_TransferEvent'($t1, $t0);

    // trace_return[0]($t2) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:44:9+68
    assume {:print "$track_return(15,5,0):", $t2} $t2 == $t2;

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(145,1707,1708)"} true;
L1:

    // return $t2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/event.move:48:5+1
    assume {:print "$at(145,1707,1708)"} true;
    $ret0 := $t2;
    return;

}

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.spec.move:548:10+75
function  $1_object_spec_create_object_address(source: int, seed: Vec (int)): int;
axiom (forall source: int, seed: Vec (int) ::
(var $$res := $1_object_spec_create_object_address(source, seed);
$IsValid'address'($$res)));

// struct object::ConstructorRef at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:133:5+167
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

// struct object::ExtendRef at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:145:5+63
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

// struct object::ObjectCore at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:97:5+551
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

// struct object::TransferEvent at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:167:5+113
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

// fun object::create_named_object [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:252:5+284
procedure {:inline 1} $1_object_create_named_object(_$t0: $signer, _$t1: Vec (int)) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: bool;
    var $t10: $1_object_ConstructorRef;
    var $t0: $signer;
    var $t1: Vec (int);
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t4, signer::$address_of($t0)) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+50
    assume {:print "$at(166,6577,6627)"} true;
    assume ($t4 == $1_signer_$address_of($t0));

    // assume Identical($t5, object::spec_create_object_address($t4, $t1)) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+65
    assume {:print "$at(166,6636,6701)"} true;
    assume ($t5 == $1_object_spec_create_object_address($t4, $t1));

    // trace_local[creator]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:252:5+1
    assume {:print "$at(165,10888,10889)"} true;
    assume {:print "$track_local(24,9,0):", $t0} $t0 == $t0;

    // trace_local[seed]($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:252:5+1
    assume {:print "$track_local(24,9,1):", $t1} $t1 == $t1;

    // $t6 := signer::address_of($t0) on_abort goto L2 with $t7 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:253:31+27
    assume {:print "$at(165,11003,11030)"} true;
    call $t6 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(165,11003,11030)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(24,9):", $t7} $t7 == $t7;
        goto L2;
    }

    // trace_local[creator_address]($t6) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:253:13+15
    assume {:print "$track_local(24,9,2):", $t6} $t6 == $t6;

    // $t8 := opaque begin: object::create_object_address($t6, $t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:254:24+45
    assume {:print "$at(165,11055,11100)"} true;

    // assume WellFormed($t8) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:254:24+45
    assume $IsValid'address'($t8);

    // assume Eq<address>($t8, object::spec_create_object_address($t6, $t1)) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:254:24+45
    assume $IsEqual'address'($t8, $1_object_spec_create_object_address($t6, $t1));

    // $t8 := opaque end: object::create_object_address($t6, $t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:254:24+45

    // trace_local[obj_addr]($t8) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:254:13+8
    assume {:print "$track_local(24,9,3):", $t8} $t8 == $t8;

    // $t9 := false at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:255:59+5
    assume {:print "$at(165,11160,11165)"} true;
    $t9 := false;
    assume $IsValid'bool'($t9);

    // $t10 := object::create_object_internal($t6, $t8, $t9) on_abort goto L2 with $t7 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:255:9+56
    call $t10 := $1_object_create_object_internal($t6, $t8, $t9);
    if ($abort_flag) {
        assume {:print "$at(165,11110,11166)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(24,9):", $t7} $t7 == $t7;
        goto L2;
    }

    // trace_return[0]($t10) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:255:9+56
    assume {:print "$track_return(24,9,0):", $t10} $t10 == $t10;

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(165,11171,11172)"} true;
L1:

    // return $t10 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(165,11171,11172)"} true;
    $ret0 := $t10;
    return;

    // label L2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
L2:

    // abort($t7) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(165,11171,11172)"} true;
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun object::create_object_internal [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:319:5+766
procedure {:inline 1} $1_object_create_object_internal(_$t0: int, _$t1: int, _$t2: bool) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t3: int;
    var $t4: $signer;
    var $t5: $1_guid_GUID;
    var $t6: bool;
    var $t7: bool;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $signer;
    var $t12: int;
    var $t13: $Mutation (int);
    var $t14: $1_guid_GUID;
    var $t15: int;
    var $t16: bool;
    var $t17: $1_event_EventHandle'$1_object_TransferEvent';
    var $t18: $1_object_ObjectCore;
    var $t19: $1_object_ConstructorRef;
    var $t0: int;
    var $t1: int;
    var $t2: bool;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // trace_local[creator_address]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:319:5+1
    assume {:print "$at(165,14426,14427)"} true;
    assume {:print "$track_local(24,15,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:319:5+1
    assume {:print "$track_local(24,15,1):", $t1} $t1 == $t1;

    // trace_local[can_delete]($t2) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:319:5+1
    assume {:print "$track_local(24,15,2):", $t2} $t2 == $t2;

    // $t6 := exists<object::ObjectCore>($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:18+6
    assume {:print "$at(165,14580,14586)"} true;
    $t6 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // $t7 := !($t6) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:17+1
    call $t7 := $Not($t6);

    // if ($t7) goto L1 else goto L0 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:9+75
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:9+75
L1:

    // goto L2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:9+75
    assume {:print "$at(165,14571,14646)"} true;
    goto L2;

    // label L0 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:68+14
L0:

    // $t8 := 1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:68+14
    assume {:print "$at(165,14630,14644)"} true;
    $t8 := 1;
    assume $IsValid'u64'($t8);

    // $t9 := error::already_exists($t8) on_abort goto L4 with $t10 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:46+37
    call $t9 := $1_error_already_exists($t8);
    if ($abort_flag) {
        assume {:print "$at(165,14608,14645)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(24,15):", $t10} $t10 == $t10;
        goto L4;
    }

    // trace_abort($t9) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:9+75
    assume {:print "$at(165,14571,14646)"} true;
    assume {:print "$track_abort(24,15):", $t9} $t9 == $t9;

    // $t10 := move($t9) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:9+75
    $t10 := $t9;

    // goto L4 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:324:9+75
    goto L4;

    // label L2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:326:43+6
    assume {:print "$at(165,14691,14697)"} true;
L2:

    // $t11 := opaque begin: create_signer::create_signer($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:326:29+21
    assume {:print "$at(165,14677,14698)"} true;

    // assume WellFormed($t11) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:326:29+21
    assume $IsValid'signer'($t11) && $1_signer_is_txn_signer($t11) && $1_signer_is_txn_signer_addr($t11->$addr);

    // assume Eq<address>(signer::$address_of($t11), $t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:326:29+21
    assume $IsEqual'address'($1_signer_$address_of($t11), $t1);

    // $t11 := opaque end: create_signer::create_signer($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:326:29+21

    // trace_local[object_signer]($t11) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:326:13+13
    assume {:print "$track_local(24,15,4):", $t11} $t11 == $t11;

    // $t12 := 1125899906842624 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:327:33+22
    assume {:print "$at(165,14732,14754)"} true;
    $t12 := 1125899906842624;
    assume $IsValid'u64'($t12);

    // $t3 := $t12 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:327:13+17
    $t3 := $t12;

    // trace_local[guid_creation_num]($t3) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:327:13+17
    assume {:print "$track_local(24,15,3):", $t3} $t3 == $t3;

    // $t13 := borrow_local($t3) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:328:57+22
    assume {:print "$at(165,14812,14834)"} true;
    $t13 := $Mutation($Local(3), EmptyVec(), $t3);

    // $t14 := guid::create($t1, $t13) on_abort goto L4 with $t10 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:328:36+44
    call $t14,$t13 := $1_guid_create($t1, $t13);
    if ($abort_flag) {
        assume {:print "$at(165,14791,14835)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(24,15):", $t10} $t10 == $t10;
        goto L4;
    }

    // write_back[LocalRoot($t3)@]($t13) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:328:36+44
    $t3 := $Dereference($t13);

    // trace_local[guid_creation_num]($t3) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:328:36+44
    assume {:print "$track_local(24,15,3):", $t3} $t3 == $t3;

    // trace_local[transfer_events_guid]($t14) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:328:13+20
    assume {:print "$track_local(24,15,5):", $t14} $t14 == $t14;

    // $t15 := move($t3) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:333:17+17
    assume {:print "$at(165,14924,14941)"} true;
    $t15 := $t3;

    // $t16 := true at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:335:41+4
    assume {:print "$at(165,15023,15027)"} true;
    $t16 := true;
    assume $IsValid'bool'($t16);

    // $t17 := event::new_event_handle<object::TransferEvent>($t14) on_abort goto L4 with $t10 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:336:34+45
    assume {:print "$at(165,15062,15107)"} true;
    call $t17 := $1_event_new_event_handle'$1_object_TransferEvent'($t14);
    if ($abort_flag) {
        assume {:print "$at(165,15062,15107)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(24,15):", $t10} $t10 == $t10;
        goto L4;
    }

    // $t18 := pack object::ObjectCore($t15, $t0, $t16, $t17) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:332:13+227
    assume {:print "$at(165,14895,15122)"} true;
    $t18 := $1_object_ObjectCore($t15, $t0, $t16, $t17);

    // move_to<object::ObjectCore>($t18, $t11) on_abort goto L4 with $t10 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:330:9+7
    assume {:print "$at(165,14846,14853)"} true;
    if ($ResourceExists($1_object_ObjectCore_$memory, $t11->$addr)) {
        call $ExecFailureAbort();
    } else {
        $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $t11->$addr, $t18);
    }
    if ($abort_flag) {
        assume {:print "$at(165,14846,14853)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(24,15):", $t10} $t10 == $t10;
        goto L4;
    }

    // $t19 := pack object::ConstructorRef($t1, $t2) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:339:9+43
    assume {:print "$at(165,15143,15186)"} true;
    $t19 := $1_object_ConstructorRef($t1, $t2);

    // trace_return[0]($t19) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:339:9+43
    assume {:print "$track_return(24,15,0):", $t19} $t19 == $t19;

    // label L3 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:340:5+1
    assume {:print "$at(165,15191,15192)"} true;
L3:

    // return $t19 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:340:5+1
    assume {:print "$at(165,15191,15192)"} true;
    $ret0 := $t19;
    return;

    // label L4 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:340:5+1
L4:

    // abort($t10) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:340:5+1
    assume {:print "$at(165,15191,15192)"} true;
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun object::generate_extend_ref [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:351:5+108
procedure {:inline 1} $1_object_generate_extend_ref(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_ExtendRef)
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_ExtendRef;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[ref]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:351:5+1
    assume {:print "$at(165,15603,15604)"} true;
    assume {:print "$track_local(24,27,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:352:27+8
    assume {:print "$at(165,15695,15703)"} true;
    $t1 := $t0->$self;

    // $t2 := pack object::ExtendRef($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:352:9+28
    $t2 := $1_object_ExtendRef($t1);

    // trace_return[0]($t2) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:352:9+28
    assume {:print "$track_return(24,27,0):", $t2} $t2 == $t2;

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:353:5+1
    assume {:print "$at(165,15710,15711)"} true;
L1:

    // return $t2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:353:5+1
    assume {:print "$at(165,15710,15711)"} true;
    $ret0 := $t2;
    return;

}

// fun object::generate_signer [baseline] at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:367:5+96
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
    // trace_local[ref]($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:367:5+1
    assume {:print "$at(165,16299,16300)"} true;
    assume {:print "$track_local(24,29,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:368:23+8
    assume {:print "$at(165,16380,16388)"} true;
    $t1 := $t0->$self;

    // $t2 := opaque begin: create_signer::create_signer($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:368:9+23

    // assume WellFormed($t2) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:368:9+23
    assume $IsValid'signer'($t2) && $1_signer_is_txn_signer($t2) && $1_signer_is_txn_signer_addr($t2->$addr);

    // assume Eq<address>(signer::$address_of($t2), $t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:368:9+23
    assume $IsEqual'address'($1_signer_$address_of($t2), $t1);

    // $t2 := opaque end: create_signer::create_signer($t1) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:368:9+23

    // trace_return[0]($t2) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:368:9+23
    assume {:print "$track_return(24,29,0):", $t2} $t2 == $t2;

    // label L1 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:369:5+1
    assume {:print "$at(165,16394,16395)"} true;
L1:

    // return $t2 at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.move:369:5+1
    assume {:print "$at(165,16394,16395)"} true;
    $ret0 := $t2;
    return;

}

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:7:9+50
function  $1_aptos_hash_spec_keccak256(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_keccak256(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:12:9+58
function  $1_aptos_hash_spec_sha2_512_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_sha2_512_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:17:9+58
function  $1_aptos_hash_spec_sha3_512_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_sha3_512_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:22:9+59
function  $1_aptos_hash_spec_ripemd160_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_ripemd160_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:27:9+61
function  $1_aptos_hash_spec_blake2b_256_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_blake2b_256_internal(bytes);
$IsValid'vec'u8''($$res)));

// struct state::State at aptos-move/move-examples/circle-experiment/sources/state.move:18:5+85
datatype $cafe_state_State {
    $cafe_state_State($paused: bool, $next_available_nonce: int)
}
function {:inline} $Update'$cafe_state_State'_paused(s: $cafe_state_State, x: bool): $cafe_state_State {
    $cafe_state_State(x, s->$next_available_nonce)
}
function {:inline} $Update'$cafe_state_State'_next_available_nonce(s: $cafe_state_State, x: int): $cafe_state_State {
    $cafe_state_State(s->$paused, x)
}
function $IsValid'$cafe_state_State'(s: $cafe_state_State): bool {
    $IsValid'bool'(s->$paused)
      && $IsValid'u64'(s->$next_available_nonce)
}
function {:inline} $IsEqual'$cafe_state_State'(s1: $cafe_state_State, s2: $cafe_state_State): bool {
    s1 == s2
}
var $cafe_state_State_$memory: $Memory $cafe_state_State;

// struct state::Management at aptos-move/move-examples/circle-experiment/sources/state.move:13:5+63
datatype $cafe_state_Management {
    $cafe_state_Management($extend_ref: $1_object_ExtendRef)
}
function {:inline} $Update'$cafe_state_Management'_extend_ref(s: $cafe_state_Management, x: $1_object_ExtendRef): $cafe_state_Management {
    $cafe_state_Management(x)
}
function $IsValid'$cafe_state_Management'(s: $cafe_state_Management): bool {
    $IsValid'$1_object_ExtendRef'(s->$extend_ref)
}
function {:inline} $IsEqual'$cafe_state_Management'(s1: $cafe_state_Management, s2: $cafe_state_Management): bool {
    s1 == s2
}
var $cafe_state_Management_$memory: $Memory $cafe_state_Management;

// fun state::get_next_available_nonce [baseline] at aptos-move/move-examples/circle-experiment/sources/state.move:38:5+172
procedure {:inline 1} $cafe_state_get_next_available_nonce() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: $cafe_state_State;
    var $t3: int;
    var $temp_0'u64': int;

    // bytecode translation starts here
    // $t0 := state::get_signer_address() on_abort goto L2 with $t1 at aptos-move/move-examples/circle-experiment/sources/state.move:39:42+20
    assume {:print "$at(3,1210,1230)"} true;
    call $t0 := $cafe_state_get_signer_address();
    if ($abort_flag) {
        assume {:print "$at(3,1210,1230)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(84,0):", $t1} $t1 == $t1;
        goto L2;
    }

    // $t2 := get_global<state::State>($t0) on_abort goto L2 with $t1 at aptos-move/move-examples/circle-experiment/sources/state.move:39:21+13
    if (!$ResourceExists($cafe_state_State_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t2 := $ResourceValue($cafe_state_State_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(3,1189,1202)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(84,0):", $t1} $t1 == $t1;
        goto L2;
    }

    // $t3 := get_field<state::State>.next_available_nonce($t2) at aptos-move/move-examples/circle-experiment/sources/state.move:40:9+26
    assume {:print "$at(3,1241,1267)"} true;
    $t3 := $t2->$next_available_nonce;

    // trace_return[0]($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:40:9+26
    assume {:print "$track_return(84,0,0):", $t3} $t3 == $t3;

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
    assume {:print "$at(3,1272,1273)"} true;
L1:

    // return $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
    assume {:print "$at(3,1272,1273)"} true;
    $ret0 := $t3;
    return;

    // label L2 at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
L2:

    // abort($t1) at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
    assume {:print "$at(3,1272,1273)"} true;
    $abort_code := $t1;
    $abort_flag := true;
    return;

}

// fun state::get_next_available_nonce [verification] at aptos-move/move-examples/circle-experiment/sources/state.move:38:5+172
procedure {:timeLimit 40} $cafe_state_get_next_available_nonce$verify() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: $cafe_state_State;
    var $t3: int;
    var $temp_0'u64': int;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume forall $rsc: state::State: ResourceDomain<state::State>(): WellFormed($rsc) at aptos-move/move-examples/circle-experiment/sources/state.move:38:5+1
    assume {:print "$at(3,1101,1102)"} true;
    assume (forall $a_0: int :: {$ResourceValue($cafe_state_State_$memory, $a_0)}(var $rsc := $ResourceValue($cafe_state_State_$memory, $a_0);
    ($IsValid'$cafe_state_State'($rsc))));

    // $t0 := state::get_signer_address() on_abort goto L2 with $t1 at aptos-move/move-examples/circle-experiment/sources/state.move:39:42+20
    assume {:print "$at(3,1210,1230)"} true;
    call $t0 := $cafe_state_get_signer_address();
    if ($abort_flag) {
        assume {:print "$at(3,1210,1230)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(84,0):", $t1} $t1 == $t1;
        goto L2;
    }

    // $t2 := get_global<state::State>($t0) on_abort goto L2 with $t1 at aptos-move/move-examples/circle-experiment/sources/state.move:39:21+13
    if (!$ResourceExists($cafe_state_State_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t2 := $ResourceValue($cafe_state_State_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(3,1189,1202)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(84,0):", $t1} $t1 == $t1;
        goto L2;
    }

    // $t3 := get_field<state::State>.next_available_nonce($t2) at aptos-move/move-examples/circle-experiment/sources/state.move:40:9+26
    assume {:print "$at(3,1241,1267)"} true;
    $t3 := $t2->$next_available_nonce;

    // trace_return[0]($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:40:9+26
    assume {:print "$track_return(84,0,0):", $t3} $t3 == $t3;

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
    assume {:print "$at(3,1272,1273)"} true;
L1:

    // return $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
    assume {:print "$at(3,1272,1273)"} true;
    $ret0 := $t3;
    return;

    // label L2 at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
L2:

    // abort($t1) at aptos-move/move-examples/circle-experiment/sources/state.move:41:5+1
    assume {:print "$at(3,1272,1273)"} true;
    $abort_code := $t1;
    $abort_flag := true;
    return;

}

// fun state::get_signer_address [baseline] at aptos-move/move-examples/circle-experiment/sources/state.move:48:5+123
procedure {:inline 1} $cafe_state_get_signer_address() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: Vec (int);
    var $t3: int;
    var $temp_0'address': int;

    // bytecode translation starts here
    // $t1 := 0xcafe at aptos-move/move-examples/circle-experiment/sources/state.move:49:40+20
    assume {:print "$at(3,1565,1585)"} true;
    $t1 := 51966;
    assume $IsValid'address'($t1);

    // $t2 := [83, 84, 65, 84, 69] at aptos-move/move-examples/circle-experiment/sources/state.move:49:62+4
    $t2 := ConcatVec(MakeVec4(83, 84, 65, 84), MakeVec1(69));
    assume $IsValid'vec'u8''($t2);

    // $t3 := opaque begin: object::create_object_address($t1, $t2) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58

    // assume WellFormed($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58
    assume $IsValid'address'($t3);

    // assume Eq<address>($t3, object::spec_create_object_address($t1, $t2)) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58
    assume $IsEqual'address'($t3, $1_object_spec_create_object_address($t1, $t2));

    // $t3 := opaque end: object::create_object_address($t1, $t2) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58

    // trace_return[0]($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58
    assume {:print "$track_return(84,1,0):", $t3} $t3 == $t3;

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:50:5+1
    assume {:print "$at(3,1597,1598)"} true;
L1:

    // return $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:50:5+1
    assume {:print "$at(3,1597,1598)"} true;
    $ret0 := $t3;
    return;

}

// fun state::get_signer_address [verification] at aptos-move/move-examples/circle-experiment/sources/state.move:48:5+123
procedure {:timeLimit 40} $cafe_state_get_signer_address$verify() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: Vec (int);
    var $t3: int;
    var $temp_0'address': int;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // $t1 := 0xcafe at aptos-move/move-examples/circle-experiment/sources/state.move:49:40+20
    assume {:print "$at(3,1565,1585)"} true;
    $t1 := 51966;
    assume $IsValid'address'($t1);

    // $t2 := [83, 84, 65, 84, 69] at aptos-move/move-examples/circle-experiment/sources/state.move:49:62+4
    $t2 := ConcatVec(MakeVec4(83, 84, 65, 84), MakeVec1(69));
    assume $IsValid'vec'u8''($t2);

    // $t3 := opaque begin: object::create_object_address($t1, $t2) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58

    // assume WellFormed($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58
    assume $IsValid'address'($t3);

    // assume Eq<address>($t3, object::spec_create_object_address($t1, $t2)) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58
    assume $IsEqual'address'($t3, $1_object_spec_create_object_address($t1, $t2));

    // $t3 := opaque end: object::create_object_address($t1, $t2) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58

    // trace_return[0]($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:49:9+58
    assume {:print "$track_return(84,1,0):", $t3} $t3 == $t3;

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:50:5+1
    assume {:print "$at(3,1597,1598)"} true;
L1:

    // return $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:50:5+1
    assume {:print "$at(3,1597,1598)"} true;
    $ret0 := $t3;
    return;

}

// fun state::init_module [verification] at aptos-move/move-examples/circle-experiment/sources/state.move:23:5+469
procedure {:timeLimit 40} $cafe_state_init_module$verify(_$t0: $signer) returns ()
{
    // declare local variables
    var $t1: $1_object_ConstructorRef;
    var $t2: $signer;
    var $t3: $1_object_ConstructorRef;
    var $t4: $1_object_ExtendRef;
    var $t5: $signer;
    var $t6: Vec (int);
    var $t7: int;
    var $t8: int;
    var $t9: $1_object_ConstructorRef;
    var $t10: int;
    var $t11: $signer;
    var $t12: $1_object_ExtendRef;
    var $t13: $cafe_state_Management;
    var $t14: bool;
    var $t15: int;
    var $t16: $cafe_state_State;
    var $t0: $signer;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at aptos-move/move-examples/circle-experiment/sources/state.move:23:5+1
    assume {:print "$at(3,626,627)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($t0->$addr);

    // assume forall $rsc: object::ObjectCore: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at aptos-move/move-examples/circle-experiment/sources/state.move:23:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // assume forall $rsc: state::State: ResourceDomain<state::State>(): WellFormed($rsc) at aptos-move/move-examples/circle-experiment/sources/state.move:23:5+1
    assume (forall $a_0: int :: {$ResourceValue($cafe_state_State_$memory, $a_0)}(var $rsc := $ResourceValue($cafe_state_State_$memory, $a_0);
    ($IsValid'$cafe_state_State'($rsc))));

    // assume forall $rsc: state::Management: ResourceDomain<state::Management>(): WellFormed($rsc) at aptos-move/move-examples/circle-experiment/sources/state.move:23:5+1
    assume (forall $a_0: int :: {$ResourceValue($cafe_state_Management_$memory, $a_0)}(var $rsc := $ResourceValue($cafe_state_Management_$memory, $a_0);
    ($IsValid'$cafe_state_Management'($rsc))));

    // trace_local[deployer]($t0) at aptos-move/move-examples/circle-experiment/sources/state.move:23:5+1
    assume {:print "$track_local(84,2,0):", $t0} $t0 == $t0;

    // $t6 := [83, 84, 65, 84, 69] at aptos-move/move-examples/circle-experiment/sources/state.move:24:70+4
    assume {:print "$at(3,732,736)"} true;
    $t6 := ConcatVec(MakeVec4(83, 84, 65, 84), MakeVec1(69));
    assume $IsValid'vec'u8''($t6);

    // assume Identical($t7, signer::$address_of($t0)) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+50
    assume {:print "$at(166,6577,6627)"} true;
    assume ($t7 == $1_signer_$address_of($t0));

    // assume Identical($t8, object::spec_create_object_address($t7, $t6)) at /Users/kshitijchakvarty/.move/https___github_com_aptos-labs_aptos-core_git_main/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+65
    assume {:print "$at(166,6636,6701)"} true;
    assume ($t8 == $1_object_spec_create_object_address($t7, $t6));

    // $t9 := object::create_named_object($t0, $t6) on_abort goto L2 with $t10 at aptos-move/move-examples/circle-experiment/sources/state.move:24:32+43
    assume {:print "$at(3,694,737)"} true;
    call $t9 := $1_object_create_named_object($t0, $t6);
    if ($abort_flag) {
        assume {:print "$at(3,694,737)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(84,2):", $t10} $t10 == $t10;
        goto L2;
    }

    // trace_local[constructor_ref]($t9) at aptos-move/move-examples/circle-experiment/sources/state.move:24:13+15
    assume {:print "$track_local(84,2,3):", $t9} $t9 == $t9;

    // $t11 := object::generate_signer($t9) on_abort goto L2 with $t10 at aptos-move/move-examples/circle-experiment/sources/state.move:25:30+40
    assume {:print "$at(3,768,808)"} true;
    call $t11 := $1_object_generate_signer($t9);
    if ($abort_flag) {
        assume {:print "$at(3,768,808)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(84,2):", $t10} $t10 == $t10;
        goto L2;
    }

    // trace_local[object_signer]($t11) at aptos-move/move-examples/circle-experiment/sources/state.move:25:13+13
    assume {:print "$track_local(84,2,5):", $t11} $t11 == $t11;

    // $t12 := object::generate_extend_ref($t9) on_abort goto L2 with $t10 at aptos-move/move-examples/circle-experiment/sources/state.move:26:26+44
    assume {:print "$at(3,835,879)"} true;
    call $t12 := $1_object_generate_extend_ref($t9);
    if ($abort_flag) {
        assume {:print "$at(3,835,879)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(84,2):", $t10} $t10 == $t10;
        goto L2;
    }

    // trace_local[extend_ref]($t12) at aptos-move/move-examples/circle-experiment/sources/state.move:26:13+10
    assume {:print "$track_local(84,2,4):", $t12} $t12 == $t12;

    // $t13 := pack state::Management($t12) at aptos-move/move-examples/circle-experiment/sources/state.move:28:32+58
    assume {:print "$at(3,913,971)"} true;
    $t13 := $cafe_state_Management($t12);

    // move_to<state::Management>($t13, $t11) on_abort goto L2 with $t10 at aptos-move/move-examples/circle-experiment/sources/state.move:28:9+7
    if ($ResourceExists($cafe_state_Management_$memory, $t11->$addr)) {
        call $ExecFailureAbort();
    } else {
        $cafe_state_Management_$memory := $ResourceUpdate($cafe_state_Management_$memory, $t11->$addr, $t13);
    }
    if ($abort_flag) {
        assume {:print "$at(3,890,897)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(84,2):", $t10} $t10 == $t10;
        goto L2;
    }

    // $t14 := false at aptos-move/move-examples/circle-experiment/sources/state.move:33:21+5
    assume {:print "$at(3,1034,1039)"} true;
    $t14 := false;
    assume $IsValid'bool'($t14);

    // $t15 := 0 at aptos-move/move-examples/circle-experiment/sources/state.move:34:35+1
    assume {:print "$at(3,1075,1076)"} true;
    $t15 := 0;
    assume $IsValid'u64'($t15);

    // $t16 := pack state::State($t14, $t15) at aptos-move/move-examples/circle-experiment/sources/state.move:32:32+81
    assume {:print "$at(3,1006,1087)"} true;
    $t16 := $cafe_state_State($t14, $t15);

    // move_to<state::State>($t16, $t11) on_abort goto L2 with $t10 at aptos-move/move-examples/circle-experiment/sources/state.move:32:9+7
    if ($ResourceExists($cafe_state_State_$memory, $t11->$addr)) {
        call $ExecFailureAbort();
    } else {
        $cafe_state_State_$memory := $ResourceUpdate($cafe_state_State_$memory, $t11->$addr, $t16);
    }
    if ($abort_flag) {
        assume {:print "$at(3,983,990)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(84,2):", $t10} $t10 == $t10;
        goto L2;
    }

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:36:5+1
    assume {:print "$at(3,1094,1095)"} true;
L1:

    // return () at aptos-move/move-examples/circle-experiment/sources/state.move:36:5+1
    assume {:print "$at(3,1094,1095)"} true;
    return;

    // label L2 at aptos-move/move-examples/circle-experiment/sources/state.move:36:5+1
L2:

    // abort($t10) at aptos-move/move-examples/circle-experiment/sources/state.move:36:5+1
    assume {:print "$at(3,1094,1095)"} true;
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun state::reserve_and_increment_nonce [verification] at aptos-move/move-examples/circle-experiment/sources/state.move:52:5+175
procedure {:timeLimit 40} $cafe_state_reserve_and_increment_nonce$verify() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $temp_0'u64': int;
    var $cafe_state_State_$memory#23: $Memory $cafe_state_State;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume forall $rsc: state::State: ResourceDomain<state::State>(): WellFormed($rsc) at aptos-move/move-examples/circle-experiment/sources/state.move:52:5+1
    assume {:print "$at(3,1604,1605)"} true;
    assume (forall $a_0: int :: {$ResourceValue($cafe_state_State_$memory, $a_0)}(var $rsc := $ResourceValue($cafe_state_State_$memory, $a_0);
    ($IsValid'$cafe_state_State'($rsc))));

    // assume Identical($t1, object::spec_create_object_address(0xcafe, [83, 84, 65, 84, 69])) at aptos-move/move-examples/circle-experiment/sources/state.move:59:9+70
    assume {:print "$at(3,1828,1898)"} true;
    assume ($t1 == $1_object_spec_create_object_address(51966, ConcatVec(MakeVec4(83, 84, 65, 84), MakeVec1(69))));

    // @23 := save_mem(state::State) at aptos-move/move-examples/circle-experiment/sources/state.move:52:5+1
    assume {:print "$at(3,1604,1605)"} true;
    $cafe_state_State_$memory#23 := $cafe_state_State_$memory;

    // $t2 := state::get_next_available_nonce() on_abort goto L2 with $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:53:21+26
    assume {:print "$at(3,1687,1713)"} true;
    call $t2 := $cafe_state_get_next_available_nonce();
    if ($abort_flag) {
        assume {:print "$at(3,1687,1713)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(84,3):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_local[nonce]($t2) at aptos-move/move-examples/circle-experiment/sources/state.move:53:13+5
    assume {:print "$track_local(84,3,0):", $t2} $t2 == $t2;

    // $t4 := 1 at aptos-move/move-examples/circle-experiment/sources/state.move:54:42+1
    assume {:print "$at(3,1756,1757)"} true;
    $t4 := 1;
    assume $IsValid'u64'($t4);

    // $t5 := +($t2, $t4) on_abort goto L2 with $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:54:40+1
    call $t5 := $AddU64($t2, $t4);
    if ($abort_flag) {
        assume {:print "$at(3,1754,1755)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(84,3):", $t3} $t3 == $t3;
        goto L2;
    }

    // state::set_next_available_nonce($t5) on_abort goto L2 with $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:54:9+35
    call $cafe_state_set_next_available_nonce($t5);
    if ($abort_flag) {
        assume {:print "$at(3,1723,1758)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(84,3):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at aptos-move/move-examples/circle-experiment/sources/state.move:55:9+5
    assume {:print "$at(3,1768,1773)"} true;
    assume {:print "$track_return(84,3,0):", $t2} $t2 == $t2;

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:56:5+1
    assume {:print "$at(3,1778,1779)"} true;
L1:

    // assert Not(Not(exists[@23]<state::State>($t1))) at aptos-move/move-examples/circle-experiment/sources/state.move:60:9+35
    assume {:print "$at(3,1907,1942)"} true;
    assert {:msg "assert_failed(3,1907,1942): function does not abort under this condition"}
      !!$ResourceExists($cafe_state_State_$memory#23, $t1);

    // assert Not(Eq<u64>(select state::State.next_available_nonce<state::State>(global[@23]<state::State>($t1)), 18446744073709551615)) at aptos-move/move-examples/circle-experiment/sources/state.move:61:9+66
    assume {:print "$at(3,1951,2017)"} true;
    assert {:msg "assert_failed(3,1951,2017): function does not abort under this condition"}
      !$IsEqual'u64'($ResourceValue($cafe_state_State_$memory#23, $t1)->$next_available_nonce, 18446744073709551615);

    // assert Eq<u64>(select state::State.next_available_nonce<state::State>(global<state::State>($t1)), Add(select state::State.next_available_nonce<state::State>(global[@23]<state::State>($t1)), 1)) at aptos-move/move-examples/circle-experiment/sources/state.move:62:9+110
    assume {:print "$at(3,2026,2136)"} true;
    assert {:msg "assert_failed(3,2026,2136): post-condition does not hold"}
      $IsEqual'u64'($ResourceValue($cafe_state_State_$memory, $t1)->$next_available_nonce, ($ResourceValue($cafe_state_State_$memory#23, $t1)->$next_available_nonce + 1));

    // return $t2 at aptos-move/move-examples/circle-experiment/sources/state.move:62:9+110
    $ret0 := $t2;
    return;

    // label L2 at aptos-move/move-examples/circle-experiment/sources/state.move:56:5+1
    assume {:print "$at(3,1778,1779)"} true;
L2:

    // assert Or(Not(exists[@23]<state::State>($t1)), Eq<u64>(select state::State.next_available_nonce<state::State>(global[@23]<state::State>($t1)), 18446744073709551615)) at aptos-move/move-examples/circle-experiment/sources/state.move:58:5+357
    assume {:print "$at(3,1785,2142)"} true;
    assert {:msg "assert_failed(3,1785,2142): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($cafe_state_State_$memory#23, $t1) || $IsEqual'u64'($ResourceValue($cafe_state_State_$memory#23, $t1)->$next_available_nonce, 18446744073709551615));

    // abort($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:58:5+357
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun state::set_next_available_nonce [baseline] at aptos-move/move-examples/circle-experiment/sources/state.move:43:5+190
procedure {:inline 1} $cafe_state_set_next_available_nonce(_$t0: int) returns ()
{
    // declare local variables
    var $t1: $Mutation ($cafe_state_State);
    var $t2: int;
    var $t3: int;
    var $t4: $Mutation ($cafe_state_State);
    var $t5: $Mutation (int);
    var $t0: int;
    var $temp_0'$cafe_state_State': $cafe_state_State;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[nonce]($t0) at aptos-move/move-examples/circle-experiment/sources/state.move:43:5+1
    assume {:print "$at(3,1279,1280)"} true;
    assume {:print "$track_local(84,4,0):", $t0} $t0 == $t0;

    // $t2 := state::get_signer_address() on_abort goto L2 with $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:44:46+20
    assume {:print "$at(3,1397,1417)"} true;
    call $t2 := $cafe_state_get_signer_address();
    if ($abort_flag) {
        assume {:print "$at(3,1397,1417)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(84,4):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t4 := borrow_global<state::State>($t2) on_abort goto L2 with $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:44:21+17
    if (!$ResourceExists($cafe_state_State_$memory, $t2)) {
        call $ExecFailureAbort();
    } else {
        $t4 := $Mutation($Global($t2), EmptyVec(), $ResourceValue($cafe_state_State_$memory, $t2));
    }
    if ($abort_flag) {
        assume {:print "$at(3,1372,1389)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(84,4):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_local[state]($t4) at aptos-move/move-examples/circle-experiment/sources/state.move:44:13+5
    $temp_0'$cafe_state_State' := $Dereference($t4);
    assume {:print "$track_local(84,4,1):", $temp_0'$cafe_state_State'} $temp_0'$cafe_state_State' == $temp_0'$cafe_state_State';

    // $t5 := borrow_field<state::State>.next_available_nonce($t4) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+26
    assume {:print "$at(3,1428,1454)"} true;
    $t5 := $ChildMutation($t4, 1, $Dereference($t4)->$next_available_nonce);

    // write_ref($t5, $t0) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+34
    $t5 := $UpdateMutation($t5, $t0);

    // write_back[Reference($t4).next_available_nonce (u64)]($t5) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+34
    $t4 := $UpdateMutation($t4, $Update'$cafe_state_State'_next_available_nonce($Dereference($t4), $Dereference($t5)));

    // write_back[state::State@]($t4) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+34
    $cafe_state_State_$memory := $ResourceUpdate($cafe_state_State_$memory, $GlobalLocationAddress($t4),
        $Dereference($t4));

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
    assume {:print "$at(3,1468,1469)"} true;
L1:

    // return () at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
    assume {:print "$at(3,1468,1469)"} true;
    return;

    // label L2 at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
L2:

    // abort($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
    assume {:print "$at(3,1468,1469)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun state::set_next_available_nonce [verification] at aptos-move/move-examples/circle-experiment/sources/state.move:43:5+190
procedure {:timeLimit 40} $cafe_state_set_next_available_nonce$verify(_$t0: int) returns ()
{
    // declare local variables
    var $t1: $Mutation ($cafe_state_State);
    var $t2: int;
    var $t3: int;
    var $t4: $Mutation ($cafe_state_State);
    var $t5: $Mutation (int);
    var $t0: int;
    var $temp_0'$cafe_state_State': $cafe_state_State;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at aptos-move/move-examples/circle-experiment/sources/state.move:43:5+1
    assume {:print "$at(3,1279,1280)"} true;
    assume $IsValid'u64'($t0);

    // assume forall $rsc: state::State: ResourceDomain<state::State>(): WellFormed($rsc) at aptos-move/move-examples/circle-experiment/sources/state.move:43:5+1
    assume (forall $a_0: int :: {$ResourceValue($cafe_state_State_$memory, $a_0)}(var $rsc := $ResourceValue($cafe_state_State_$memory, $a_0);
    ($IsValid'$cafe_state_State'($rsc))));

    // trace_local[nonce]($t0) at aptos-move/move-examples/circle-experiment/sources/state.move:43:5+1
    assume {:print "$track_local(84,4,0):", $t0} $t0 == $t0;

    // $t2 := state::get_signer_address() on_abort goto L2 with $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:44:46+20
    assume {:print "$at(3,1397,1417)"} true;
    call $t2 := $cafe_state_get_signer_address();
    if ($abort_flag) {
        assume {:print "$at(3,1397,1417)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(84,4):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t4 := borrow_global<state::State>($t2) on_abort goto L2 with $t3 at aptos-move/move-examples/circle-experiment/sources/state.move:44:21+17
    if (!$ResourceExists($cafe_state_State_$memory, $t2)) {
        call $ExecFailureAbort();
    } else {
        $t4 := $Mutation($Global($t2), EmptyVec(), $ResourceValue($cafe_state_State_$memory, $t2));
    }
    if ($abort_flag) {
        assume {:print "$at(3,1372,1389)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(84,4):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_local[state]($t4) at aptos-move/move-examples/circle-experiment/sources/state.move:44:13+5
    $temp_0'$cafe_state_State' := $Dereference($t4);
    assume {:print "$track_local(84,4,1):", $temp_0'$cafe_state_State'} $temp_0'$cafe_state_State' == $temp_0'$cafe_state_State';

    // $t5 := borrow_field<state::State>.next_available_nonce($t4) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+26
    assume {:print "$at(3,1428,1454)"} true;
    $t5 := $ChildMutation($t4, 1, $Dereference($t4)->$next_available_nonce);

    // write_ref($t5, $t0) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+34
    $t5 := $UpdateMutation($t5, $t0);

    // write_back[Reference($t4).next_available_nonce (u64)]($t5) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+34
    $t4 := $UpdateMutation($t4, $Update'$cafe_state_State'_next_available_nonce($Dereference($t4), $Dereference($t5)));

    // write_back[state::State@]($t4) at aptos-move/move-examples/circle-experiment/sources/state.move:45:9+34
    $cafe_state_State_$memory := $ResourceUpdate($cafe_state_State_$memory, $GlobalLocationAddress($t4),
        $Dereference($t4));

    // label L1 at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
    assume {:print "$at(3,1468,1469)"} true;
L1:

    // return () at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
    assume {:print "$at(3,1468,1469)"} true;
    return;

    // label L2 at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
L2:

    // abort($t3) at aptos-move/move-examples/circle-experiment/sources/state.move:46:5+1
    assume {:print "$at(3,1468,1469)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun message_transmitter::reserve_and_increment_nonce [verification] at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:6:5+165
procedure {:timeLimit 40} $cafe_message_transmitter_reserve_and_increment_nonce$verify() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $temp_0'u64': int;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume forall $rsc: state::State: ResourceDomain<state::State>(): WellFormed($rsc) at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:6:5+1
    assume {:print "$at(2,130,131)"} true;
    assume (forall $a_0: int :: {$ResourceValue($cafe_state_State_$memory, $a_0)}(var $rsc := $ResourceValue($cafe_state_State_$memory, $a_0);
    ($IsValid'$cafe_state_State'($rsc))));

    // $t1 := state::get_next_available_nonce() on_abort goto L2 with $t2 at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:7:21+33
    assume {:print "$at(2,191,224)"} true;
    call $t1 := $cafe_state_get_next_available_nonce();
    if ($abort_flag) {
        assume {:print "$at(2,191,224)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(85,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_local[nonce]($t1) at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:7:13+5
    assume {:print "$track_local(85,0,0):", $t1} $t1 == $t1;

    // $t3 := 1 at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:8:47+1
    assume {:print "$at(2,272,273)"} true;
    $t3 := 1;
    assume $IsValid'u64'($t3);

    // $t4 := +($t1, $t3) on_abort goto L2 with $t2 at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:8:46+1
    call $t4 := $AddU64($t1, $t3);
    if ($abort_flag) {
        assume {:print "$at(2,271,272)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(85,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // state::set_next_available_nonce($t4) on_abort goto L2 with $t2 at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:8:9+40
    call $cafe_state_set_next_available_nonce($t4);
    if ($abort_flag) {
        assume {:print "$at(2,234,274)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(85,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_return[0]($t1) at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:9:9+5
    assume {:print "$at(2,284,289)"} true;
    assume {:print "$track_return(85,0,0):", $t1} $t1 == $t1;

    // label L1 at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:10:5+1
    assume {:print "$at(2,294,295)"} true;
L1:

    // return $t1 at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:10:5+1
    assume {:print "$at(2,294,295)"} true;
    $ret0 := $t1;
    return;

    // label L2 at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:10:5+1
L2:

    // abort($t2) at aptos-move/move-examples/circle-experiment/sources/message_transmitter.move:10:5+1
    assume {:print "$at(2,294,295)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}
