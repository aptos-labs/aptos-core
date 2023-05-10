
// ** Expanded prelude

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Basic theory for vectors using arrays. This version of vectors is not extensional.

type {:datatype} Vec _;

function {:constructor} Vec<T>(v: [int]T, l: int): Vec T;

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
    (var l := l#Vec(v);
    Vec(v#Vec(v)[l := elem], l + 1))
}

function {:inline} ReadVec<T>(v: Vec T, i: int): T {
    v#Vec(v)[i]
}

function {:inline} LenVec<T>(v: Vec T): int {
    l#Vec(v)
}

function {:inline} IsEmptyVec<T>(v: Vec T): bool {
    l#Vec(v) == 0
}

function {:inline} RemoveVec<T>(v: Vec T): Vec T {
    (var l := l#Vec(v) - 1;
    Vec(v#Vec(v)[l := DefaultVecElem()], l))
}

function {:inline} RemoveAtVec<T>(v: Vec T, i: int): Vec T {
    (var l := l#Vec(v) - 1;
    Vec(
        (lambda j: int ::
           if j >= 0 && j < l then
               if j < i then v#Vec(v)[j] else v#Vec(v)[j+1]
           else DefaultVecElem()),
        l))
}

function {:inline} ConcatVec<T>(v1: Vec T, v2: Vec T): Vec T {
    (var l1, m1, l2, m2 := l#Vec(v1), v#Vec(v1), l#Vec(v2), v#Vec(v2);
    Vec(
        (lambda i: int ::
          if i >= 0 && i < l1 + l2 then
            if i < l1 then m1[i] else m2[i - l1]
          else DefaultVecElem()),
        l1 + l2))
}

function {:inline} ReverseVec<T>(v: Vec T): Vec T {
    (var l := l#Vec(v);
    Vec(
        (lambda i: int :: if 0 <= i && i < l then v#Vec(v)[l - i - 1] else DefaultVecElem()),
        l))
}

function {:inline} SliceVec<T>(v: Vec T, i: int, j: int): Vec T {
    (var m := v#Vec(v);
    Vec(
        (lambda k:int ::
          if 0 <= k && k < j - i then
            m[i + k]
          else
            DefaultVecElem()),
        (if j - i < 0 then 0 else j - i)))
}


function {:inline} UpdateVec<T>(v: Vec T, i: int, elem: T): Vec T {
    Vec(v#Vec(v)[i := elem], l#Vec(v))
}

function {:inline} SwapVec<T>(v: Vec T, i: int, j: int): Vec T {
    (var m := v#Vec(v);
    Vec(m[i := m[j]][j := m[i]], l#Vec(v)))
}

function {:inline} ContainsVec<T>(v: Vec T, e: T): bool {
    (var l := l#Vec(v);
    (exists i: int :: InRangeVec(v, i) && v#Vec(v)[i] == e))
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

type {:datatype} Multiset _;
function {:constructor} Multiset<T>(v: [T]int, l: int): Multiset T;

function {:builtin "MapConst"} MapConstMultiset<T>(l: int): [T]int;

function {:inline} EmptyMultiset<T>(): Multiset T {
    Multiset(MapConstMultiset(0), 0)
}

function {:inline} LenMultiset<T>(s: Multiset T): int {
    l#Multiset(s)
}

function {:inline} ExtendMultiset<T>(s: Multiset T, v: T): Multiset T {
    (var len := l#Multiset(s);
    (var cnt := v#Multiset(s)[v];
    Multiset(v#Multiset(s)[v := (cnt + 1)], len + 1)))
}

// This function returns (s1 - s2). This function assumes that s2 is a subset of s1.
function {:inline} SubtractMultiset<T>(s1: Multiset T, s2: Multiset T): Multiset T {
    (var len1 := l#Multiset(s1);
    (var len2 := l#Multiset(s2);
    Multiset((lambda v:T :: v#Multiset(s1)[v]-v#Multiset(s2)[v]), len1-len2)))
}

function {:inline} IsEmptyMultiset<T>(s: Multiset T): bool {
    (l#Multiset(s) == 0) &&
    (forall v: T :: v#Multiset(s)[v] == 0)
}

function {:inline} IsSubsetMultiset<T>(s1: Multiset T, s2: Multiset T): bool {
    (l#Multiset(s1) <= l#Multiset(s2)) &&
    (forall v: T :: v#Multiset(s1)[v] <= v#Multiset(s2)[v])
}

function {:inline} ContainsMultiset<T>(s: Multiset T, v: T): bool {
    v#Multiset(s)[v] > 0
}

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Theory for tables.

type {:datatype} Table _ _;

// v is the SMT array holding the key-value assignment. e is an array which
// independently determines whether a key is valid or not. l is the length.
//
// Note that even though the program cannot reflect over existence of a key,
// we want the specification to be able to do this, so it can express
// verification conditions like "key has been inserted".
function {:constructor} Table<K, V>(v: [K]V, e: [K]bool, l: int): Table K V;

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
    v#Table(t)[k]
}

function {:inline} LenTable<K,V>(t: Table K V): int {
    l#Table(t)
}


function {:inline} ContainsTable<K,V>(t: Table K V, k: K): bool {
    e#Table(t)[k]
}

function {:inline} UpdateTable<K,V>(t: Table K V, k: K, v: V): Table K V {
    Table(v#Table(t)[k := v], e#Table(t), l#Table(t))
}

function {:inline} AddTable<K,V>(t: Table K V, k: K, v: V): Table K V {
    // This function has an undetermined result if the key is already in the table
    // (all specification functions have this "partial definiteness" behavior). Thus we can
    // just increment the length.
    Table(v#Table(t)[k := v], e#Table(t)[k := true], l#Table(t) + 1)
}

function {:inline} RemoveTable<K,V>(t: Table K V, k: K): Table K V {
    // Similar as above, we only need to consider the case where the key is in the table.
    Table(v#Table(t), e#Table(t)[k := false], l#Table(t) - 1)
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

type {:datatype} $1_aggregator_Aggregator;
function {:constructor} $1_aggregator_Aggregator($handle: int, $key: int, $limit: int, $val: int): $1_aggregator_Aggregator;
function {:inline} $Update'$1_aggregator_Aggregator'_handle(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator(x, $key#$1_aggregator_Aggregator(s), $limit#$1_aggregator_Aggregator(s), $val#$1_aggregator_Aggregator(s))
}
function {:inline} $Update'$1_aggregator_Aggregator'_key(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator($handle#$1_aggregator_Aggregator(s), x, $limit#$1_aggregator_Aggregator(s), $val#$1_aggregator_Aggregator(s))
}
function {:inline} $Update'$1_aggregator_Aggregator'_limit(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator($handle#$1_aggregator_Aggregator(s), $key#$1_aggregator_Aggregator(s), x, $val#$1_aggregator_Aggregator(s))
}
function {:inline} $Update'$1_aggregator_Aggregator'_val(s: $1_aggregator_Aggregator, x: int): $1_aggregator_Aggregator {
    $1_aggregator_Aggregator($handle#$1_aggregator_Aggregator(s), $key#$1_aggregator_Aggregator(s), $limit#$1_aggregator_Aggregator(s), x)
}
function $IsValid'$1_aggregator_Aggregator'(s: $1_aggregator_Aggregator): bool {
    $IsValid'address'($handle#$1_aggregator_Aggregator(s))
      && $IsValid'address'($key#$1_aggregator_Aggregator(s))
      && $IsValid'u128'($limit#$1_aggregator_Aggregator(s))
      && $IsValid'u128'($val#$1_aggregator_Aggregator(s))
}
function {:inline} $IsEqual'$1_aggregator_Aggregator'(s1: $1_aggregator_Aggregator, s2: $1_aggregator_Aggregator): bool {
    s1 == s2
}
function {:inline} $1_aggregator_spec_get_limit(s1: $1_aggregator_Aggregator): int {
    $limit#$1_aggregator_Aggregator(s1)
}
function {:inline} $1_aggregator_spec_get_handle(s1: $1_aggregator_Aggregator): int {
    $handle#$1_aggregator_Aggregator(s1)
}
function {:inline} $1_aggregator_spec_get_key(s1: $1_aggregator_Aggregator): int {
    $key#$1_aggregator_Aggregator(s1)
}
function {:inline} $1_aggregator_spec_get_val(s1: $1_aggregator_Aggregator): int {
    $val#$1_aggregator_Aggregator(s1)
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

type {:datatype} $Range;
function {:constructor} $Range(lb: int, ub: int): $Range;

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
   $IsValid'u64'(lb#$Range(r)) &&  $IsValid'u64'(ub#$Range(r))
}

// Intentionally not inlined so it serves as a trigger in quantifiers.
function $InRange(r: $Range, i: int): bool {
   lb#$Range(r) <= i && i < ub#$Range(r)
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

type {:datatype} $Location;

// A global resource location within the statically known resource type's memory,
// where `a` is an address.
function {:constructor} $Global(a: int): $Location;

// A local location. `i` is the unique index of the local.
function {:constructor} $Local(i: int): $Location;

// The location of a reference outside of the verification scope, for example, a `&mut` parameter
// of the function being verified. References with these locations don't need to be written back
// when mutation ends.
function {:constructor} $Param(i: int): $Location;

// The location of an uninitialized mutation. Using this to make sure that the location
// will not be equal to any valid mutation locations, i.e., $Local, $Global, or $Param.
function {:constructor} $Uninitialized(): $Location;

// A mutable reference which also carries its current value. Since mutable references
// are single threaded in Move, we can keep them together and treat them as a value
// during mutation until the point they are stored back to their original location.
type {:datatype} $Mutation _;
function {:constructor} $Mutation<T>(l: $Location, p: Vec int, v: T): $Mutation T;

// Representation of memory for a given type.
type {:datatype} $Memory _;
function {:constructor} $Memory<T>(domain: [int]bool, contents: [int]T): $Memory T;

function {:builtin "MapConst"} $ConstMemoryDomain(v: bool): [int]bool;
function {:builtin "MapConst"} $ConstMemoryContent<T>(v: T): [int]T;
axiom $ConstMemoryDomain(false) == (lambda i: int :: false);
axiom $ConstMemoryDomain(true) == (lambda i: int :: true);


// Dereferences a mutation.
function {:inline} $Dereference<T>(ref: $Mutation T): T {
    v#$Mutation(ref)
}

// Update the value of a mutation.
function {:inline} $UpdateMutation<T>(m: $Mutation T, v: T): $Mutation T {
    $Mutation(l#$Mutation(m), p#$Mutation(m), v)
}

function {:inline} $ChildMutation<T1, T2>(m: $Mutation T1, offset: int, v: T2): $Mutation T2 {
    $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), offset), v)
}

// Return true if two mutations share the location and path
function {:inline} $IsSameMutation<T1, T2>(parent: $Mutation T1, child: $Mutation T2 ): bool {
    l#$Mutation(parent) == l#$Mutation(child) && p#$Mutation(parent) == p#$Mutation(child)
}

// Return true if the mutation is a parent of a child which was derived with the given edge offset. This
// is used to implement write-back choices.
function {:inline} $IsParentMutation<T1, T2>(parent: $Mutation T1, edge: int, child: $Mutation T2 ): bool {
    l#$Mutation(parent) == l#$Mutation(child) &&
    (var pp := p#$Mutation(parent);
    (var cp := p#$Mutation(child);
    (var pl := LenVec(pp);
    (var cl := LenVec(cp);
     cl == pl + 1 &&
     (forall i: int:: i >= 0 && i < pl ==> ReadVec(pp, i) ==  ReadVec(cp, i)) &&
     $EdgeMatches(ReadVec(cp, pl), edge)
    ))))
}

// Return true if the mutation is a parent of a child, for hyper edge.
function {:inline} $IsParentMutationHyper<T1, T2>(parent: $Mutation T1, hyper_edge: Vec int, child: $Mutation T2 ): bool {
    l#$Mutation(parent) == l#$Mutation(child) &&
    (var pp := p#$Mutation(parent);
    (var cp := p#$Mutation(child);
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
    l#$Mutation(m1) == l#$Mutation(m2)
}

function {:inline} $HasGlobalLocation<T>(m: $Mutation T): bool {
    is#$Global(l#$Mutation(m))
}

function {:inline} $HasLocalLocation<T>(m: $Mutation T, idx: int): bool {
    l#$Mutation(m) == $Local(idx)
}

function {:inline} $GlobalLocationAddress<T>(m: $Mutation T): int {
    a#$Global(l#$Mutation(m))
}



// Tests whether resource exists.
function {:inline} $ResourceExists<T>(m: $Memory T, addr: int): bool {
    domain#$Memory(m)[addr]
}

// Obtains Value of given resource.
function {:inline} $ResourceValue<T>(m: $Memory T, addr: int): T {
    contents#$Memory(m)[addr]
}

// Update resource.
function {:inline} $ResourceUpdate<T>(m: $Memory T, a: int, v: T): $Memory T {
    $Memory(domain#$Memory(m)[a := true], contents#$Memory(m)[a := v])
}

// Remove resource.
function {:inline} $ResourceRemove<T>(m: $Memory T, a: int): $Memory T {
    $Memory(domain#$Memory(m)[a := false], contents#$Memory(m))
}

// Copies resource from memory s to m.
function {:inline} $ResourceCopy<T>(m: $Memory T, s: $Memory T, a: int): $Memory T {
    $Memory(domain#$Memory(m)[a := domain#$Memory(s)[a]],
            contents#$Memory(m)[a := contents#$Memory(s)[a]])
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
    SliceVec(v, lb#$Range(r), ub#$Range(r))
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
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
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
// Native Vector implementation for element type `$1_aggregator_Aggregator`

// Not inlined. It appears faster this way.
function $IsEqual'vec'$1_aggregator_Aggregator''(v1: Vec ($1_aggregator_Aggregator), v2: Vec ($1_aggregator_Aggregator)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'$1_aggregator_Aggregator'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'$1_aggregator_Aggregator''(v: Vec ($1_aggregator_Aggregator), prefix: Vec ($1_aggregator_Aggregator)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'$1_aggregator_Aggregator'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'$1_aggregator_Aggregator''(v: Vec ($1_aggregator_Aggregator), suffix: Vec ($1_aggregator_Aggregator)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'$1_aggregator_Aggregator'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'$1_aggregator_Aggregator''(v: Vec ($1_aggregator_Aggregator)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'$1_aggregator_Aggregator'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), e: $1_aggregator_Aggregator): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'$1_aggregator_Aggregator'(ReadVec(v, i), e))
}

function $IndexOfVec'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), e: $1_aggregator_Aggregator): int;
axiom (forall v: Vec ($1_aggregator_Aggregator), e: $1_aggregator_Aggregator:: {$IndexOfVec'$1_aggregator_Aggregator'(v, e)}
    (var i := $IndexOfVec'$1_aggregator_Aggregator'(v, e);
     if (!$ContainsVec'$1_aggregator_Aggregator'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'$1_aggregator_Aggregator'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'$1_aggregator_Aggregator'(ReadVec(v, j), e))));


function {:inline} $RangeVec'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'$1_aggregator_Aggregator'(): Vec ($1_aggregator_Aggregator) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'$1_aggregator_Aggregator'() returns (v: Vec ($1_aggregator_Aggregator)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'$1_aggregator_Aggregator'(): Vec ($1_aggregator_Aggregator) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), val: $1_aggregator_Aggregator) returns (m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), val: $1_aggregator_Aggregator): Vec ($1_aggregator_Aggregator) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator))) returns (e: $1_aggregator_Aggregator, m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    var v: Vec ($1_aggregator_Aggregator);
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

procedure {:inline 1} $1_vector_append'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), other: Vec ($1_aggregator_Aggregator)) returns (m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator))) returns (m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), other: Vec ($1_aggregator_Aggregator)) returns (m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), new_len: int) returns (v: (Vec ($1_aggregator_Aggregator)), m': $Mutation (Vec ($1_aggregator_Aggregator))) {
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

procedure {:inline 1} $1_vector_trim'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), new_len: int) returns (v: (Vec ($1_aggregator_Aggregator)), m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), left: int, right: int) returns (m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    var left_vec: Vec ($1_aggregator_Aggregator);
    var mid_vec: Vec ($1_aggregator_Aggregator);
    var right_vec: Vec ($1_aggregator_Aggregator);
    var v: Vec ($1_aggregator_Aggregator);
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

procedure {:inline 1} $1_vector_rotate'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), rot: int) returns (n: int, m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    var v: Vec ($1_aggregator_Aggregator);
    var len: int;
    var left_vec: Vec ($1_aggregator_Aggregator);
    var right_vec: Vec ($1_aggregator_Aggregator);
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

procedure {:inline 1} $1_vector_rotate_slice'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    var left_vec: Vec ($1_aggregator_Aggregator);
    var mid_vec: Vec ($1_aggregator_Aggregator);
    var right_vec: Vec ($1_aggregator_Aggregator);
    var mid_left_vec: Vec ($1_aggregator_Aggregator);
    var mid_right_vec: Vec ($1_aggregator_Aggregator);
    var v: Vec ($1_aggregator_Aggregator);
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

procedure {:inline 1} $1_vector_insert'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), i: int, e: $1_aggregator_Aggregator) returns (m': $Mutation (Vec ($1_aggregator_Aggregator))) {
    var left_vec: Vec ($1_aggregator_Aggregator);
    var right_vec: Vec ($1_aggregator_Aggregator);
    var v: Vec ($1_aggregator_Aggregator);
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

procedure {:inline 1} $1_vector_length'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), i: int) returns (dst: $1_aggregator_Aggregator) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), i: int): $1_aggregator_Aggregator {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), index: int)
returns (dst: $Mutation ($1_aggregator_Aggregator), m': $Mutation (Vec ($1_aggregator_Aggregator)))
{
    var v: Vec ($1_aggregator_Aggregator);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), i: int): $1_aggregator_Aggregator {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), i: int, j: int) returns (m': $Mutation (Vec ($1_aggregator_Aggregator)))
{
    var v: Vec ($1_aggregator_Aggregator);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), i: int, j: int): Vec ($1_aggregator_Aggregator) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), i: int) returns (e: $1_aggregator_Aggregator, m': $Mutation (Vec ($1_aggregator_Aggregator)))
{
    var v: Vec ($1_aggregator_Aggregator);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'$1_aggregator_Aggregator'(m: $Mutation (Vec ($1_aggregator_Aggregator)), i: int) returns (e: $1_aggregator_Aggregator, m': $Mutation (Vec ($1_aggregator_Aggregator)))
{
    var len: int;
    var v: Vec ($1_aggregator_Aggregator);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), e: $1_aggregator_Aggregator) returns (res: bool)  {
    res := $ContainsVec'$1_aggregator_Aggregator'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator), e: $1_aggregator_Aggregator) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'$1_aggregator_Aggregator'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `$1_optional_aggregator_Integer`

// Not inlined. It appears faster this way.
function $IsEqual'vec'$1_optional_aggregator_Integer''(v1: Vec ($1_optional_aggregator_Integer), v2: Vec ($1_optional_aggregator_Integer)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'$1_optional_aggregator_Integer'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'$1_optional_aggregator_Integer''(v: Vec ($1_optional_aggregator_Integer), prefix: Vec ($1_optional_aggregator_Integer)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'$1_optional_aggregator_Integer'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'$1_optional_aggregator_Integer''(v: Vec ($1_optional_aggregator_Integer), suffix: Vec ($1_optional_aggregator_Integer)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'$1_optional_aggregator_Integer'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'$1_optional_aggregator_Integer''(v: Vec ($1_optional_aggregator_Integer)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'$1_optional_aggregator_Integer'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), e: $1_optional_aggregator_Integer): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'$1_optional_aggregator_Integer'(ReadVec(v, i), e))
}

function $IndexOfVec'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), e: $1_optional_aggregator_Integer): int;
axiom (forall v: Vec ($1_optional_aggregator_Integer), e: $1_optional_aggregator_Integer:: {$IndexOfVec'$1_optional_aggregator_Integer'(v, e)}
    (var i := $IndexOfVec'$1_optional_aggregator_Integer'(v, e);
     if (!$ContainsVec'$1_optional_aggregator_Integer'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'$1_optional_aggregator_Integer'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'$1_optional_aggregator_Integer'(ReadVec(v, j), e))));


function {:inline} $RangeVec'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'$1_optional_aggregator_Integer'(): Vec ($1_optional_aggregator_Integer) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'$1_optional_aggregator_Integer'() returns (v: Vec ($1_optional_aggregator_Integer)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'$1_optional_aggregator_Integer'(): Vec ($1_optional_aggregator_Integer) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), val: $1_optional_aggregator_Integer) returns (m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), val: $1_optional_aggregator_Integer): Vec ($1_optional_aggregator_Integer) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer))) returns (e: $1_optional_aggregator_Integer, m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    var v: Vec ($1_optional_aggregator_Integer);
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

procedure {:inline 1} $1_vector_append'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), other: Vec ($1_optional_aggregator_Integer)) returns (m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer))) returns (m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), other: Vec ($1_optional_aggregator_Integer)) returns (m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), new_len: int) returns (v: (Vec ($1_optional_aggregator_Integer)), m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
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

procedure {:inline 1} $1_vector_trim'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), new_len: int) returns (v: (Vec ($1_optional_aggregator_Integer)), m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), left: int, right: int) returns (m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    var left_vec: Vec ($1_optional_aggregator_Integer);
    var mid_vec: Vec ($1_optional_aggregator_Integer);
    var right_vec: Vec ($1_optional_aggregator_Integer);
    var v: Vec ($1_optional_aggregator_Integer);
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

procedure {:inline 1} $1_vector_rotate'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), rot: int) returns (n: int, m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    var v: Vec ($1_optional_aggregator_Integer);
    var len: int;
    var left_vec: Vec ($1_optional_aggregator_Integer);
    var right_vec: Vec ($1_optional_aggregator_Integer);
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

procedure {:inline 1} $1_vector_rotate_slice'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    var left_vec: Vec ($1_optional_aggregator_Integer);
    var mid_vec: Vec ($1_optional_aggregator_Integer);
    var right_vec: Vec ($1_optional_aggregator_Integer);
    var mid_left_vec: Vec ($1_optional_aggregator_Integer);
    var mid_right_vec: Vec ($1_optional_aggregator_Integer);
    var v: Vec ($1_optional_aggregator_Integer);
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

procedure {:inline 1} $1_vector_insert'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), i: int, e: $1_optional_aggregator_Integer) returns (m': $Mutation (Vec ($1_optional_aggregator_Integer))) {
    var left_vec: Vec ($1_optional_aggregator_Integer);
    var right_vec: Vec ($1_optional_aggregator_Integer);
    var v: Vec ($1_optional_aggregator_Integer);
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

procedure {:inline 1} $1_vector_length'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), i: int) returns (dst: $1_optional_aggregator_Integer) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), i: int): $1_optional_aggregator_Integer {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), index: int)
returns (dst: $Mutation ($1_optional_aggregator_Integer), m': $Mutation (Vec ($1_optional_aggregator_Integer)))
{
    var v: Vec ($1_optional_aggregator_Integer);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), i: int): $1_optional_aggregator_Integer {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), i: int, j: int) returns (m': $Mutation (Vec ($1_optional_aggregator_Integer)))
{
    var v: Vec ($1_optional_aggregator_Integer);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), i: int, j: int): Vec ($1_optional_aggregator_Integer) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), i: int) returns (e: $1_optional_aggregator_Integer, m': $Mutation (Vec ($1_optional_aggregator_Integer)))
{
    var v: Vec ($1_optional_aggregator_Integer);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'$1_optional_aggregator_Integer'(m: $Mutation (Vec ($1_optional_aggregator_Integer)), i: int) returns (e: $1_optional_aggregator_Integer, m': $Mutation (Vec ($1_optional_aggregator_Integer)))
{
    var len: int;
    var v: Vec ($1_optional_aggregator_Integer);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), e: $1_optional_aggregator_Integer) returns (res: bool)  {
    res := $ContainsVec'$1_optional_aggregator_Integer'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer), e: $1_optional_aggregator_Integer) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'$1_optional_aggregator_Integer'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `$1_optional_aggregator_OptionalAggregator`

// Not inlined. It appears faster this way.
function $IsEqual'vec'$1_optional_aggregator_OptionalAggregator''(v1: Vec ($1_optional_aggregator_OptionalAggregator), v2: Vec ($1_optional_aggregator_OptionalAggregator)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'$1_optional_aggregator_OptionalAggregator'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'$1_optional_aggregator_OptionalAggregator''(v: Vec ($1_optional_aggregator_OptionalAggregator), prefix: Vec ($1_optional_aggregator_OptionalAggregator)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'$1_optional_aggregator_OptionalAggregator'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'$1_optional_aggregator_OptionalAggregator''(v: Vec ($1_optional_aggregator_OptionalAggregator), suffix: Vec ($1_optional_aggregator_OptionalAggregator)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'$1_optional_aggregator_OptionalAggregator'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'$1_optional_aggregator_OptionalAggregator''(v: Vec ($1_optional_aggregator_OptionalAggregator)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'$1_optional_aggregator_OptionalAggregator'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), e: $1_optional_aggregator_OptionalAggregator): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'$1_optional_aggregator_OptionalAggregator'(ReadVec(v, i), e))
}

function $IndexOfVec'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), e: $1_optional_aggregator_OptionalAggregator): int;
axiom (forall v: Vec ($1_optional_aggregator_OptionalAggregator), e: $1_optional_aggregator_OptionalAggregator:: {$IndexOfVec'$1_optional_aggregator_OptionalAggregator'(v, e)}
    (var i := $IndexOfVec'$1_optional_aggregator_OptionalAggregator'(v, e);
     if (!$ContainsVec'$1_optional_aggregator_OptionalAggregator'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'$1_optional_aggregator_OptionalAggregator'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'$1_optional_aggregator_OptionalAggregator'(ReadVec(v, j), e))));


function {:inline} $RangeVec'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'$1_optional_aggregator_OptionalAggregator'(): Vec ($1_optional_aggregator_OptionalAggregator) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'$1_optional_aggregator_OptionalAggregator'() returns (v: Vec ($1_optional_aggregator_OptionalAggregator)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'$1_optional_aggregator_OptionalAggregator'(): Vec ($1_optional_aggregator_OptionalAggregator) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), val: $1_optional_aggregator_OptionalAggregator) returns (m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), val: $1_optional_aggregator_OptionalAggregator): Vec ($1_optional_aggregator_OptionalAggregator) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) returns (e: $1_optional_aggregator_OptionalAggregator, m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    var v: Vec ($1_optional_aggregator_OptionalAggregator);
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

procedure {:inline 1} $1_vector_append'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), other: Vec ($1_optional_aggregator_OptionalAggregator)) returns (m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) returns (m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), other: Vec ($1_optional_aggregator_OptionalAggregator)) returns (m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), new_len: int) returns (v: (Vec ($1_optional_aggregator_OptionalAggregator)), m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
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

procedure {:inline 1} $1_vector_trim'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), new_len: int) returns (v: (Vec ($1_optional_aggregator_OptionalAggregator)), m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), left: int, right: int) returns (m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    var left_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var mid_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var right_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var v: Vec ($1_optional_aggregator_OptionalAggregator);
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

procedure {:inline 1} $1_vector_rotate'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), rot: int) returns (n: int, m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    var v: Vec ($1_optional_aggregator_OptionalAggregator);
    var len: int;
    var left_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var right_vec: Vec ($1_optional_aggregator_OptionalAggregator);
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

procedure {:inline 1} $1_vector_rotate_slice'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    var left_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var mid_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var right_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var mid_left_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var mid_right_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var v: Vec ($1_optional_aggregator_OptionalAggregator);
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

procedure {:inline 1} $1_vector_insert'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), i: int, e: $1_optional_aggregator_OptionalAggregator) returns (m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator))) {
    var left_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var right_vec: Vec ($1_optional_aggregator_OptionalAggregator);
    var v: Vec ($1_optional_aggregator_OptionalAggregator);
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

procedure {:inline 1} $1_vector_length'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), i: int) returns (dst: $1_optional_aggregator_OptionalAggregator) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), i: int): $1_optional_aggregator_OptionalAggregator {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), index: int)
returns (dst: $Mutation ($1_optional_aggregator_OptionalAggregator), m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)))
{
    var v: Vec ($1_optional_aggregator_OptionalAggregator);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), i: int): $1_optional_aggregator_OptionalAggregator {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), i: int, j: int) returns (m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)))
{
    var v: Vec ($1_optional_aggregator_OptionalAggregator);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), i: int, j: int): Vec ($1_optional_aggregator_OptionalAggregator) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), i: int) returns (e: $1_optional_aggregator_OptionalAggregator, m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)))
{
    var v: Vec ($1_optional_aggregator_OptionalAggregator);

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'$1_optional_aggregator_OptionalAggregator'(m: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)), i: int) returns (e: $1_optional_aggregator_OptionalAggregator, m': $Mutation (Vec ($1_optional_aggregator_OptionalAggregator)))
{
    var len: int;
    var v: Vec ($1_optional_aggregator_OptionalAggregator);

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), e: $1_optional_aggregator_OptionalAggregator) returns (res: bool)  {
    res := $ContainsVec'$1_optional_aggregator_OptionalAggregator'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator), e: $1_optional_aggregator_OptionalAggregator) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'$1_optional_aggregator_OptionalAggregator'(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `vec'u8'`

// Not inlined. It appears faster this way.
function $IsEqual'vec'vec'u8'''(v1: Vec (Vec (int)), v2: Vec (Vec (int))): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'vec'u8''(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'vec'u8'''(v: Vec (Vec (int)), prefix: Vec (Vec (int))): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'vec'u8''(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'vec'u8'''(v: Vec (Vec (int)), suffix: Vec (Vec (int))): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'vec'u8''(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'vec'u8'''(v: Vec (Vec (int))): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'vec'u8''(ReadVec(v, i)))
}


function {:inline} $ContainsVec'vec'u8''(v: Vec (Vec (int)), e: Vec (int)): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'vec'u8''(ReadVec(v, i), e))
}

function $IndexOfVec'vec'u8''(v: Vec (Vec (int)), e: Vec (int)): int;
axiom (forall v: Vec (Vec (int)), e: Vec (int):: {$IndexOfVec'vec'u8''(v, e)}
    (var i := $IndexOfVec'vec'u8''(v, e);
     if (!$ContainsVec'vec'u8''(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'vec'u8''(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'vec'u8''(ReadVec(v, j), e))));


function {:inline} $RangeVec'vec'u8''(v: Vec (Vec (int))): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'vec'u8''(): Vec (Vec (int)) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'vec'u8''() returns (v: Vec (Vec (int))) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'vec'u8''(): Vec (Vec (int)) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'vec'u8''(v: Vec (Vec (int))) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'vec'u8''(m: $Mutation (Vec (Vec (int))), val: Vec (int)) returns (m': $Mutation (Vec (Vec (int)))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'vec'u8''(v: Vec (Vec (int)), val: Vec (int)): Vec (Vec (int)) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'vec'u8''(m: $Mutation (Vec (Vec (int)))) returns (e: Vec (int), m': $Mutation (Vec (Vec (int)))) {
    var v: Vec (Vec (int));
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

procedure {:inline 1} $1_vector_append'vec'u8''(m: $Mutation (Vec (Vec (int))), other: Vec (Vec (int))) returns (m': $Mutation (Vec (Vec (int)))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'vec'u8''(m: $Mutation (Vec (Vec (int)))) returns (m': $Mutation (Vec (Vec (int)))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'vec'u8''(m: $Mutation (Vec (Vec (int))), other: Vec (Vec (int))) returns (m': $Mutation (Vec (Vec (int)))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'vec'u8''(m: $Mutation (Vec (Vec (int))), new_len: int) returns (v: (Vec (Vec (int))), m': $Mutation (Vec (Vec (int)))) {
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

procedure {:inline 1} $1_vector_trim'vec'u8''(m: $Mutation (Vec (Vec (int))), new_len: int) returns (v: (Vec (Vec (int))), m': $Mutation (Vec (Vec (int)))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'vec'u8''(m: $Mutation (Vec (Vec (int))), left: int, right: int) returns (m': $Mutation (Vec (Vec (int)))) {
    var left_vec: Vec (Vec (int));
    var mid_vec: Vec (Vec (int));
    var right_vec: Vec (Vec (int));
    var v: Vec (Vec (int));
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

procedure {:inline 1} $1_vector_rotate'vec'u8''(m: $Mutation (Vec (Vec (int))), rot: int) returns (n: int, m': $Mutation (Vec (Vec (int)))) {
    var v: Vec (Vec (int));
    var len: int;
    var left_vec: Vec (Vec (int));
    var right_vec: Vec (Vec (int));
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

procedure {:inline 1} $1_vector_rotate_slice'vec'u8''(m: $Mutation (Vec (Vec (int))), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (Vec (int)))) {
    var left_vec: Vec (Vec (int));
    var mid_vec: Vec (Vec (int));
    var right_vec: Vec (Vec (int));
    var mid_left_vec: Vec (Vec (int));
    var mid_right_vec: Vec (Vec (int));
    var v: Vec (Vec (int));
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

procedure {:inline 1} $1_vector_insert'vec'u8''(m: $Mutation (Vec (Vec (int))), i: int, e: Vec (int)) returns (m': $Mutation (Vec (Vec (int)))) {
    var left_vec: Vec (Vec (int));
    var right_vec: Vec (Vec (int));
    var v: Vec (Vec (int));
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

procedure {:inline 1} $1_vector_length'vec'u8''(v: Vec (Vec (int))) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'vec'u8''(v: Vec (Vec (int))): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'vec'u8''(v: Vec (Vec (int)), i: int) returns (dst: Vec (int)) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'vec'u8''(v: Vec (Vec (int)), i: int): Vec (int) {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'vec'u8''(m: $Mutation (Vec (Vec (int))), index: int)
returns (dst: $Mutation (Vec (int)), m': $Mutation (Vec (Vec (int))))
{
    var v: Vec (Vec (int));
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'vec'u8''(v: Vec (Vec (int)), i: int): Vec (int) {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'vec'u8''(v: Vec (Vec (int))) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'vec'u8''(m: $Mutation (Vec (Vec (int))), i: int, j: int) returns (m': $Mutation (Vec (Vec (int))))
{
    var v: Vec (Vec (int));
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'vec'u8''(v: Vec (Vec (int)), i: int, j: int): Vec (Vec (int)) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'vec'u8''(m: $Mutation (Vec (Vec (int))), i: int) returns (e: Vec (int), m': $Mutation (Vec (Vec (int))))
{
    var v: Vec (Vec (int));

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'vec'u8''(m: $Mutation (Vec (Vec (int))), i: int) returns (e: Vec (int), m': $Mutation (Vec (Vec (int))))
{
    var len: int;
    var v: Vec (Vec (int));

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'vec'u8''(v: Vec (Vec (int)), e: Vec (int)) returns (res: bool)  {
    res := $ContainsVec'vec'u8''(v, e);
}

procedure {:inline 1}
$1_vector_index_of'vec'u8''(v: Vec (Vec (int)), e: Vec (int)) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'vec'u8''(v, e);
    if (res2 >= 0) {
        res1 := true;
    } else {
        res1 := false;
        res2 := 0;
    }
}


// ----------------------------------------------------------------------------------
// Native Vector implementation for element type `address`

// Not inlined. It appears faster this way.
function $IsEqual'vec'address''(v1: Vec (int), v2: Vec (int)): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'address'(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'address''(v: Vec (int), prefix: Vec (int)): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'address'(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'address''(v: Vec (int), suffix: Vec (int)): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'address'(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'address''(v: Vec (int)): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'address'(ReadVec(v, i)))
}


function {:inline} $ContainsVec'address'(v: Vec (int), e: int): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'address'(ReadVec(v, i), e))
}

function $IndexOfVec'address'(v: Vec (int), e: int): int;
axiom (forall v: Vec (int), e: int:: {$IndexOfVec'address'(v, e)}
    (var i := $IndexOfVec'address'(v, e);
     if (!$ContainsVec'address'(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'address'(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'address'(ReadVec(v, j), e))));


function {:inline} $RangeVec'address'(v: Vec (int)): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'address'(): Vec (int) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'address'() returns (v: Vec (int)) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'address'(): Vec (int) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'address'(v: Vec (int)) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'address'(m: $Mutation (Vec (int)), val: int) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'address'(v: Vec (int), val: int): Vec (int) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'address'(m: $Mutation (Vec (int))) returns (e: int, m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_append'address'(m: $Mutation (Vec (int)), other: Vec (int)) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'address'(m: $Mutation (Vec (int))) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'address'(m: $Mutation (Vec (int)), other: Vec (int)) returns (m': $Mutation (Vec (int))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'address'(m: $Mutation (Vec (int)), new_len: int) returns (v: (Vec (int)), m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_trim'address'(m: $Mutation (Vec (int)), new_len: int) returns (v: (Vec (int)), m': $Mutation (Vec (int))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'address'(m: $Mutation (Vec (int)), left: int, right: int) returns (m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_rotate'address'(m: $Mutation (Vec (int)), rot: int) returns (n: int, m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_rotate_slice'address'(m: $Mutation (Vec (int)), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_insert'address'(m: $Mutation (Vec (int)), i: int, e: int) returns (m': $Mutation (Vec (int))) {
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

procedure {:inline 1} $1_vector_length'address'(v: Vec (int)) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'address'(v: Vec (int)): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'address'(v: Vec (int), i: int) returns (dst: int) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'address'(v: Vec (int), i: int): int {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'address'(m: $Mutation (Vec (int)), index: int)
returns (dst: $Mutation (int), m': $Mutation (Vec (int)))
{
    var v: Vec (int);
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'address'(v: Vec (int), i: int): int {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'address'(v: Vec (int)) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'address'(m: $Mutation (Vec (int)), i: int, j: int) returns (m': $Mutation (Vec (int)))
{
    var v: Vec (int);
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'address'(v: Vec (int), i: int, j: int): Vec (int) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'address'(m: $Mutation (Vec (int)), i: int) returns (e: int, m': $Mutation (Vec (int)))
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

procedure {:inline 1} $1_vector_swap_remove'address'(m: $Mutation (Vec (int)), i: int) returns (e: int, m': $Mutation (Vec (int)))
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

procedure {:inline 1} $1_vector_contains'address'(v: Vec (int), e: int) returns (res: bool)  {
    res := $ContainsVec'address'(v, e);
}

procedure {:inline 1}
$1_vector_index_of'address'(v: Vec (int), e: int) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'address'(v, e);
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
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
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
// Native Vector implementation for element type `vec'bv8'`

// Not inlined. It appears faster this way.
function $IsEqual'vec'vec'bv8'''(v1: Vec (Vec (bv8)), v2: Vec (Vec (bv8))): bool {
    LenVec(v1) == LenVec(v2) &&
    (forall i: int:: InRangeVec(v1, i) ==> $IsEqual'vec'bv8''(ReadVec(v1, i), ReadVec(v2, i)))
}

// Not inlined.
function $IsPrefix'vec'vec'bv8'''(v: Vec (Vec (bv8)), prefix: Vec (Vec (bv8))): bool {
    LenVec(v) >= LenVec(prefix) &&
    (forall i: int:: InRangeVec(prefix, i) ==> $IsEqual'vec'bv8''(ReadVec(v, i), ReadVec(prefix, i)))
}

// Not inlined.
function $IsSuffix'vec'vec'bv8'''(v: Vec (Vec (bv8)), suffix: Vec (Vec (bv8))): bool {
    LenVec(v) >= LenVec(suffix) &&
    (forall i: int:: InRangeVec(suffix, i) ==> $IsEqual'vec'bv8''(ReadVec(v, LenVec(v) - LenVec(suffix) + i), ReadVec(suffix, i)))
}

// Not inlined.
function $IsValid'vec'vec'bv8'''(v: Vec (Vec (bv8))): bool {
    $IsValid'u64'(LenVec(v)) &&
    (forall i: int:: InRangeVec(v, i) ==> $IsValid'vec'bv8''(ReadVec(v, i)))
}


function {:inline} $ContainsVec'vec'bv8''(v: Vec (Vec (bv8)), e: Vec (bv8)): bool {
    (exists i: int :: $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'vec'bv8''(ReadVec(v, i), e))
}

function $IndexOfVec'vec'bv8''(v: Vec (Vec (bv8)), e: Vec (bv8)): int;
axiom (forall v: Vec (Vec (bv8)), e: Vec (bv8):: {$IndexOfVec'vec'bv8''(v, e)}
    (var i := $IndexOfVec'vec'bv8''(v, e);
     if (!$ContainsVec'vec'bv8''(v, e)) then i == -1
     else $IsValid'u64'(i) && InRangeVec(v, i) && $IsEqual'vec'bv8''(ReadVec(v, i), e) &&
        (forall j: int :: $IsValid'u64'(j) && j >= 0 && j < i ==> !$IsEqual'vec'bv8''(ReadVec(v, j), e))));


function {:inline} $RangeVec'vec'bv8''(v: Vec (Vec (bv8))): $Range {
    $Range(0, LenVec(v))
}


function {:inline} $EmptyVec'vec'bv8''(): Vec (Vec (bv8)) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_empty'vec'bv8''() returns (v: Vec (Vec (bv8))) {
    v := EmptyVec();
}

function {:inline} $1_vector_$empty'vec'bv8''(): Vec (Vec (bv8)) {
    EmptyVec()
}

procedure {:inline 1} $1_vector_is_empty'vec'bv8''(v: Vec (Vec (bv8))) returns (b: bool) {
    b := IsEmptyVec(v);
}

procedure {:inline 1} $1_vector_push_back'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), val: Vec (bv8)) returns (m': $Mutation (Vec (Vec (bv8)))) {
    m' := $UpdateMutation(m, ExtendVec($Dereference(m), val));
}

function {:inline} $1_vector_$push_back'vec'bv8''(v: Vec (Vec (bv8)), val: Vec (bv8)): Vec (Vec (bv8)) {
    ExtendVec(v, val)
}

procedure {:inline 1} $1_vector_pop_back'vec'bv8''(m: $Mutation (Vec (Vec (bv8)))) returns (e: Vec (bv8), m': $Mutation (Vec (Vec (bv8)))) {
    var v: Vec (Vec (bv8));
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

procedure {:inline 1} $1_vector_append'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), other: Vec (Vec (bv8))) returns (m': $Mutation (Vec (Vec (bv8)))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), other));
}

procedure {:inline 1} $1_vector_reverse'vec'bv8''(m: $Mutation (Vec (Vec (bv8)))) returns (m': $Mutation (Vec (Vec (bv8)))) {
    m' := $UpdateMutation(m, ReverseVec($Dereference(m)));
}

procedure {:inline 1} $1_vector_reverse_append'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), other: Vec (Vec (bv8))) returns (m': $Mutation (Vec (Vec (bv8)))) {
    m' := $UpdateMutation(m, ConcatVec($Dereference(m), ReverseVec(other)));
}

procedure {:inline 1} $1_vector_trim_reverse'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), new_len: int) returns (v: (Vec (Vec (bv8))), m': $Mutation (Vec (Vec (bv8)))) {
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

procedure {:inline 1} $1_vector_trim'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), new_len: int) returns (v: (Vec (Vec (bv8))), m': $Mutation (Vec (Vec (bv8)))) {
    var len: int;
    v := $Dereference(m);
    if (LenVec(v) < new_len) {
        call $ExecFailureAbort();
        return;
    }
    v := SliceVec(v, new_len, LenVec(v));
    m' := $UpdateMutation(m, SliceVec($Dereference(m), 0, new_len));
}

procedure {:inline 1} $1_vector_reverse_slice'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), left: int, right: int) returns (m': $Mutation (Vec (Vec (bv8)))) {
    var left_vec: Vec (Vec (bv8));
    var mid_vec: Vec (Vec (bv8));
    var right_vec: Vec (Vec (bv8));
    var v: Vec (Vec (bv8));
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

procedure {:inline 1} $1_vector_rotate'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), rot: int) returns (n: int, m': $Mutation (Vec (Vec (bv8)))) {
    var v: Vec (Vec (bv8));
    var len: int;
    var left_vec: Vec (Vec (bv8));
    var right_vec: Vec (Vec (bv8));
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

procedure {:inline 1} $1_vector_rotate_slice'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), left: int, rot: int, right: int) returns (n: int, m': $Mutation (Vec (Vec (bv8)))) {
    var left_vec: Vec (Vec (bv8));
    var mid_vec: Vec (Vec (bv8));
    var right_vec: Vec (Vec (bv8));
    var mid_left_vec: Vec (Vec (bv8));
    var mid_right_vec: Vec (Vec (bv8));
    var v: Vec (Vec (bv8));
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

procedure {:inline 1} $1_vector_insert'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), i: int, e: Vec (bv8)) returns (m': $Mutation (Vec (Vec (bv8)))) {
    var left_vec: Vec (Vec (bv8));
    var right_vec: Vec (Vec (bv8));
    var v: Vec (Vec (bv8));
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

procedure {:inline 1} $1_vector_length'vec'bv8''(v: Vec (Vec (bv8))) returns (l: int) {
    l := LenVec(v);
}

function {:inline} $1_vector_$length'vec'bv8''(v: Vec (Vec (bv8))): int {
    LenVec(v)
}

procedure {:inline 1} $1_vector_borrow'vec'bv8''(v: Vec (Vec (bv8)), i: int) returns (dst: Vec (bv8)) {
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    dst := ReadVec(v, i);
}

function {:inline} $1_vector_$borrow'vec'bv8''(v: Vec (Vec (bv8)), i: int): Vec (bv8) {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_borrow_mut'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), index: int)
returns (dst: $Mutation (Vec (bv8)), m': $Mutation (Vec (Vec (bv8))))
{
    var v: Vec (Vec (bv8));
    v := $Dereference(m);
    if (!InRangeVec(v, index)) {
        call $ExecFailureAbort();
        return;
    }
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
    m' := m;
}

function {:inline} $1_vector_$borrow_mut'vec'bv8''(v: Vec (Vec (bv8)), i: int): Vec (bv8) {
    ReadVec(v, i)
}

procedure {:inline 1} $1_vector_destroy_empty'vec'bv8''(v: Vec (Vec (bv8))) {
    if (!IsEmptyVec(v)) {
      call $ExecFailureAbort();
    }
}

procedure {:inline 1} $1_vector_swap'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), i: int, j: int) returns (m': $Mutation (Vec (Vec (bv8))))
{
    var v: Vec (Vec (bv8));
    v := $Dereference(m);
    if (!InRangeVec(v, i) || !InRangeVec(v, j)) {
        call $ExecFailureAbort();
        return;
    }
    m' := $UpdateMutation(m, SwapVec(v, i, j));
}

function {:inline} $1_vector_$swap'vec'bv8''(v: Vec (Vec (bv8)), i: int, j: int): Vec (Vec (bv8)) {
    SwapVec(v, i, j)
}

procedure {:inline 1} $1_vector_remove'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), i: int) returns (e: Vec (bv8), m': $Mutation (Vec (Vec (bv8))))
{
    var v: Vec (Vec (bv8));

    v := $Dereference(m);

    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveAtVec(v, i));
}

procedure {:inline 1} $1_vector_swap_remove'vec'bv8''(m: $Mutation (Vec (Vec (bv8))), i: int) returns (e: Vec (bv8), m': $Mutation (Vec (Vec (bv8))))
{
    var len: int;
    var v: Vec (Vec (bv8));

    v := $Dereference(m);
    len := LenVec(v);
    if (!InRangeVec(v, i)) {
        call $ExecFailureAbort();
        return;
    }
    e := ReadVec(v, i);
    m' := $UpdateMutation(m, RemoveVec(SwapVec(v, i, len-1)));
}

procedure {:inline 1} $1_vector_contains'vec'bv8''(v: Vec (Vec (bv8)), e: Vec (bv8)) returns (res: bool)  {
    res := $ContainsVec'vec'bv8''(v, e);
}

procedure {:inline 1}
$1_vector_index_of'vec'bv8''(v: Vec (Vec (bv8)), e: Vec (bv8)) returns (res1: bool, res2: int) {
    res2 := $IndexOfVec'vec'bv8''(v, e);
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
    dst := $Mutation(l#$Mutation(m), ExtendVec(p#$Mutation(m), index), ReadVec(v, index));
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

type {:datatype} $signer;
function {:constructor} $signer($addr: int): $signer;
function {:inline} $IsValid'signer'(s: $signer): bool {
    $IsValid'address'($addr#$signer(s))
}
function {:inline} $IsEqual'signer'(s1: $signer, s2: $signer): bool {
    s1 == s2
}

procedure {:inline 1} $1_signer_borrow_address(signer: $signer) returns (res: int) {
    res := $addr#$signer(signer);
}

function {:inline} $1_signer_$borrow_address(signer: $signer): int
{
    $addr#$signer(signer)
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

type {:datatype} $TypeParamInfo;

function {:constructor} $TypeParamBool(): $TypeParamInfo;
function {:constructor} $TypeParamU8(): $TypeParamInfo;
function {:constructor} $TypeParamU16(): $TypeParamInfo;
function {:constructor} $TypeParamU32(): $TypeParamInfo;
function {:constructor} $TypeParamU64(): $TypeParamInfo;
function {:constructor} $TypeParamU128(): $TypeParamInfo;
function {:constructor} $TypeParamU256(): $TypeParamInfo;
function {:constructor} $TypeParamAddress(): $TypeParamInfo;
function {:constructor} $TypeParamSigner(): $TypeParamInfo;
function {:constructor} $TypeParamVector(e: $TypeParamInfo): $TypeParamInfo;
function {:constructor} $TypeParamStruct(a: int, m: Vec int, s: Vec int): $TypeParamInfo;



//==================================
// Begin Translation

function $TypeName(t: $TypeParamInfo): Vec int;
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamBool(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 98][1 := 111][2 := 111][3 := 108], 4)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 98][1 := 111][2 := 111][3 := 108], 4)) ==> is#$TypeParamBool(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamU8(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 56], 2)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 56], 2)) ==> is#$TypeParamU8(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamU16(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 54], 3)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 54], 3)) ==> is#$TypeParamU16(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamU32(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 51][2 := 50], 3)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 51][2 := 50], 3)) ==> is#$TypeParamU32(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamU64(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 54][2 := 52], 3)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 54][2 := 52], 3)) ==> is#$TypeParamU64(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamU128(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 50][3 := 56], 4)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 49][2 := 50][3 := 56], 4)) ==> is#$TypeParamU128(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamU256(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 50][2 := 53][3 := 54], 4)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 117][1 := 50][2 := 53][3 := 54], 4)) ==> is#$TypeParamU256(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamAddress(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 97][1 := 100][2 := 100][3 := 114][4 := 101][5 := 115][6 := 115], 7)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 97][1 := 100][2 := 100][3 := 114][4 := 101][5 := 115][6 := 115], 7)) ==> is#$TypeParamAddress(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamSigner(t) ==> $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 115][1 := 105][2 := 103][3 := 110][4 := 101][5 := 114], 6)));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsEqual'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 115][1 := 105][2 := 103][3 := 110][4 := 101][5 := 114], 6)) ==> is#$TypeParamSigner(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamVector(t) ==> $IsEqual'vec'u8''($TypeName(t), ConcatVec(ConcatVec(Vec(DefaultVecMap()[0 := 118][1 := 101][2 := 99][3 := 116][4 := 111][5 := 114][6 := 60], 7), $TypeName(e#$TypeParamVector(t))), Vec(DefaultVecMap()[0 := 62], 1))));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} ($IsPrefix'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 118][1 := 101][2 := 99][3 := 116][4 := 111][5 := 114][6 := 60], 7)) && $IsSuffix'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 62], 1))) ==> is#$TypeParamVector(t));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} is#$TypeParamStruct(t) ==> $IsEqual'vec'u8''($TypeName(t), ConcatVec(ConcatVec(ConcatVec(ConcatVec(ConcatVec(Vec(DefaultVecMap()[0 := 48][1 := 120], 2), MakeVec1(a#$TypeParamStruct(t))), Vec(DefaultVecMap()[0 := 58][1 := 58], 2)), m#$TypeParamStruct(t)), Vec(DefaultVecMap()[0 := 58][1 := 58], 2)), s#$TypeParamStruct(t))));
axiom (forall t: $TypeParamInfo :: {$TypeName(t)} $IsPrefix'vec'u8''($TypeName(t), Vec(DefaultVecMap()[0 := 48][1 := 120], 2)) ==> is#$TypeParamVector(t));


// Given Types for Type Parameters

type #0;
function {:inline} $IsEqual'#0'(x1: #0, x2: #0): bool { x1 == x2 }
function {:inline} $IsValid'#0'(x: #0): bool { true }
var #0_info: $TypeParamInfo;
var #0_$memory: $Memory #0;

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'bool'($1_from_bcs_deserialize'bool'(b1), $1_from_bcs_deserialize'bool'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'u8'($1_from_bcs_deserialize'u8'(b1), $1_from_bcs_deserialize'u8'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'u64'($1_from_bcs_deserialize'u64'(b1), $1_from_bcs_deserialize'u64'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <u128>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'u128'($1_from_bcs_deserialize'u128'(b1), $1_from_bcs_deserialize'u128'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'u256'($1_from_bcs_deserialize'u256'(b1), $1_from_bcs_deserialize'u256'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'address'($1_from_bcs_deserialize'address'(b1), $1_from_bcs_deserialize'address'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'signer'($1_from_bcs_deserialize'signer'(b1), $1_from_bcs_deserialize'signer'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_from_bcs_deserialize'vec'u8''(b1), $1_from_bcs_deserialize'vec'u8''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'address''($1_from_bcs_deserialize'vec'address''(b1), $1_from_bcs_deserialize'vec'address''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<vector<u8>>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'vec'u8'''($1_from_bcs_deserialize'vec'vec'u8'''(b1), $1_from_bcs_deserialize'vec'vec'u8'''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<aggregator::Aggregator>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'$1_aggregator_Aggregator''($1_from_bcs_deserialize'vec'$1_aggregator_Aggregator''(b1), $1_from_bcs_deserialize'vec'$1_aggregator_Aggregator''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<optional_aggregator::Integer>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'$1_optional_aggregator_Integer''($1_from_bcs_deserialize'vec'$1_optional_aggregator_Integer''(b1), $1_from_bcs_deserialize'vec'$1_optional_aggregator_Integer''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<optional_aggregator::OptionalAggregator>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'$1_optional_aggregator_OptionalAggregator''($1_from_bcs_deserialize'vec'$1_optional_aggregator_OptionalAggregator''(b1), $1_from_bcs_deserialize'vec'$1_optional_aggregator_OptionalAggregator''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'#0''($1_from_bcs_deserialize'vec'#0''(b1), $1_from_bcs_deserialize'vec'#0''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <option::Option<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_option_Option'address''($1_from_bcs_deserialize'$1_option_Option'address''(b1), $1_from_bcs_deserialize'$1_option_Option'address''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <option::Option<aggregator::Aggregator>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_option_Option'$1_aggregator_Aggregator''($1_from_bcs_deserialize'$1_option_Option'$1_aggregator_Aggregator''(b1), $1_from_bcs_deserialize'$1_option_Option'$1_aggregator_Aggregator''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <option::Option<optional_aggregator::Integer>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_option_Option'$1_optional_aggregator_Integer''($1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_Integer''(b1), $1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_Integer''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <option::Option<optional_aggregator::OptionalAggregator>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_option_Option'$1_optional_aggregator_OptionalAggregator''($1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_OptionalAggregator''(b1), $1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_OptionalAggregator''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <string::String>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_string_String'($1_from_bcs_deserialize'$1_string_String'(b1), $1_from_bcs_deserialize'$1_string_String'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <features::Features>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_features_Features'($1_from_bcs_deserialize'$1_features_Features'(b1), $1_from_bcs_deserialize'$1_features_Features'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <type_info::TypeInfo>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_type_info_TypeInfo'($1_from_bcs_deserialize'$1_type_info_TypeInfo'(b1), $1_from_bcs_deserialize'$1_type_info_TypeInfo'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_guid_GUID'($1_from_bcs_deserialize'$1_guid_GUID'(b1), $1_from_bcs_deserialize'$1_guid_GUID'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_guid_ID'($1_from_bcs_deserialize'$1_guid_ID'(b1), $1_from_bcs_deserialize'$1_guid_ID'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<account::CoinRegisterEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<account::KeyRotationEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<coin::DepositEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_coin_DepositEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_DepositEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_DepositEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<coin::WithdrawEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_coin_WithdrawEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_WithdrawEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_WithdrawEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<reconfiguration::NewEpochEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <chain_id::ChainId>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_chain_id_ChainId'($1_from_bcs_deserialize'$1_chain_id_ChainId'(b1), $1_from_bcs_deserialize'$1_chain_id_ChainId'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <account::Account>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_account_Account'($1_from_bcs_deserialize'$1_account_Account'(b1), $1_from_bcs_deserialize'$1_account_Account'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <account::CapabilityOffer<account::RotationCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <account::CapabilityOffer<account::SignerCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <aggregator::Aggregator>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_aggregator_Aggregator'($1_from_bcs_deserialize'$1_aggregator_Aggregator'(b1), $1_from_bcs_deserialize'$1_aggregator_Aggregator'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <optional_aggregator::Integer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_optional_aggregator_Integer'($1_from_bcs_deserialize'$1_optional_aggregator_Integer'(b1), $1_from_bcs_deserialize'$1_optional_aggregator_Integer'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <optional_aggregator::OptionalAggregator>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_optional_aggregator_OptionalAggregator'($1_from_bcs_deserialize'$1_optional_aggregator_OptionalAggregator'(b1), $1_from_bcs_deserialize'$1_optional_aggregator_OptionalAggregator'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <coin::AggregatableCoin<aptos_coin::AptosCoin>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''($1_from_bcs_deserialize'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''(b1), $1_from_bcs_deserialize'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <coin::BurnCapability<aptos_coin::AptosCoin>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''($1_from_bcs_deserialize'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''(b1), $1_from_bcs_deserialize'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <coin::Coin<aptos_coin::AptosCoin>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_coin_Coin'$1_aptos_coin_AptosCoin''($1_from_bcs_deserialize'$1_coin_Coin'$1_aptos_coin_AptosCoin''(b1), $1_from_bcs_deserialize'$1_coin_Coin'$1_aptos_coin_AptosCoin''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <coin::CoinInfo<aptos_coin::AptosCoin>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''($1_from_bcs_deserialize'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''(b1), $1_from_bcs_deserialize'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <coin::CoinStore<aptos_coin::AptosCoin>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''($1_from_bcs_deserialize'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''(b1), $1_from_bcs_deserialize'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <aptos_coin::AptosCoin>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_aptos_coin_AptosCoin'($1_from_bcs_deserialize'$1_aptos_coin_AptosCoin'(b1), $1_from_bcs_deserialize'$1_aptos_coin_AptosCoin'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <chain_status::GenesisEndMarker>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_chain_status_GenesisEndMarker'($1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(b1), $1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <timestamp::CurrentTimeMicroseconds>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_timestamp_CurrentTimeMicroseconds'($1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(b1), $1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <transaction_fee::AptosCoinCapabilities>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_transaction_fee_AptosCoinCapabilities'($1_from_bcs_deserialize'$1_transaction_fee_AptosCoinCapabilities'(b1), $1_from_bcs_deserialize'$1_transaction_fee_AptosCoinCapabilities'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <transaction_fee::CollectedFeesPerBlock>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_transaction_fee_CollectedFeesPerBlock'($1_from_bcs_deserialize'$1_transaction_fee_CollectedFeesPerBlock'(b1), $1_from_bcs_deserialize'$1_transaction_fee_CollectedFeesPerBlock'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <reconfiguration::Configuration>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_reconfiguration_Configuration'($1_from_bcs_deserialize'$1_reconfiguration_Configuration'(b1), $1_from_bcs_deserialize'$1_reconfiguration_Configuration'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <transaction_validation::TransactionValidation>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_transaction_validation_TransactionValidation'($1_from_bcs_deserialize'$1_transaction_validation_TransactionValidation'(b1), $1_from_bcs_deserialize'$1_transaction_validation_TransactionValidation'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'#0'($1_from_bcs_deserialize'#0'(b1), $1_from_bcs_deserialize'#0'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/vector.move:143:5+86
function {:inline} $1_vector_$is_empty'$1_aggregator_Aggregator'(v: Vec ($1_aggregator_Aggregator)): bool {
    $IsEqual'u64'($1_vector_$length'$1_aggregator_Aggregator'(v), 0)
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/vector.move:143:5+86
function {:inline} $1_vector_$is_empty'$1_optional_aggregator_Integer'(v: Vec ($1_optional_aggregator_Integer)): bool {
    $IsEqual'u64'($1_vector_$length'$1_optional_aggregator_Integer'(v), 0)
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/vector.move:143:5+86
function {:inline} $1_vector_$is_empty'$1_optional_aggregator_OptionalAggregator'(v: Vec ($1_optional_aggregator_OptionalAggregator)): bool {
    $IsEqual'u64'($1_vector_$length'$1_optional_aggregator_OptionalAggregator'(v), 0)
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:102:5+145
function {:inline} $1_option_$borrow'$1_aggregator_Aggregator'(t: $1_option_Option'$1_aggregator_Aggregator'): $1_aggregator_Aggregator {
    $1_vector_$borrow'$1_aggregator_Aggregator'($vec#$1_option_Option'$1_aggregator_Aggregator'(t), 0)
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:102:5+145
function {:inline} $1_option_$borrow'$1_optional_aggregator_Integer'(t: $1_option_Option'$1_optional_aggregator_Integer'): $1_optional_aggregator_Integer {
    $1_vector_$borrow'$1_optional_aggregator_Integer'($vec#$1_option_Option'$1_optional_aggregator_Integer'(t), 0)
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:61:5+95
function {:inline} $1_option_$is_none'$1_aggregator_Aggregator'(t: $1_option_Option'$1_aggregator_Aggregator'): bool {
    $1_vector_$is_empty'$1_aggregator_Aggregator'($vec#$1_option_Option'$1_aggregator_Aggregator'(t))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:61:5+95
function {:inline} $1_option_$is_none'$1_optional_aggregator_Integer'(t: $1_option_Option'$1_optional_aggregator_Integer'): bool {
    $1_vector_$is_empty'$1_optional_aggregator_Integer'($vec#$1_option_Option'$1_optional_aggregator_Integer'(t))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:74:5+96
function {:inline} $1_option_$is_some'$1_aggregator_Aggregator'(t: $1_option_Option'$1_aggregator_Aggregator'): bool {
    !$1_vector_$is_empty'$1_aggregator_Aggregator'($vec#$1_option_Option'$1_aggregator_Aggregator'(t))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:74:5+96
function {:inline} $1_option_$is_some'$1_optional_aggregator_Integer'(t: $1_option_Option'$1_optional_aggregator_Integer'): bool {
    !$1_vector_$is_empty'$1_optional_aggregator_Integer'($vec#$1_option_Option'$1_optional_aggregator_Integer'(t))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:82:10+92
function {:inline} $1_option_spec_is_some'$1_aggregator_Aggregator'(t: $1_option_Option'$1_aggregator_Aggregator'): bool {
    !$1_vector_$is_empty'$1_aggregator_Aggregator'($vec#$1_option_Option'$1_aggregator_Aggregator'(t))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:82:10+92
function {:inline} $1_option_spec_is_some'$1_optional_aggregator_Integer'(t: $1_option_Option'$1_optional_aggregator_Integer'): bool {
    !$1_vector_$is_empty'$1_optional_aggregator_Integer'($vec#$1_option_Option'$1_optional_aggregator_Integer'(t))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:82:10+92
function {:inline} $1_option_spec_is_some'$1_optional_aggregator_OptionalAggregator'(t: $1_option_Option'$1_optional_aggregator_OptionalAggregator'): bool {
    !$1_vector_$is_empty'$1_optional_aggregator_OptionalAggregator'($vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'(t))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:111:10+78
function {:inline} $1_option_spec_borrow'$1_optional_aggregator_OptionalAggregator'(t: $1_option_Option'$1_optional_aggregator_OptionalAggregator'): $1_optional_aggregator_OptionalAggregator {
    ReadVec($vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'(t), 0)
}

// struct option::Option<address> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:7:5+81
type {:datatype} $1_option_Option'address';
function {:constructor} $1_option_Option'address'($vec: Vec (int)): $1_option_Option'address';
function {:inline} $Update'$1_option_Option'address''_vec(s: $1_option_Option'address', x: Vec (int)): $1_option_Option'address' {
    $1_option_Option'address'(x)
}
function $IsValid'$1_option_Option'address''(s: $1_option_Option'address'): bool {
    $IsValid'vec'address''($vec#$1_option_Option'address'(s))
}
function {:inline} $IsEqual'$1_option_Option'address''(s1: $1_option_Option'address', s2: $1_option_Option'address'): bool {
    $IsEqual'vec'address''($vec#$1_option_Option'address'(s1), $vec#$1_option_Option'address'(s2))}

// struct option::Option<aggregator::Aggregator> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:7:5+81
type {:datatype} $1_option_Option'$1_aggregator_Aggregator';
function {:constructor} $1_option_Option'$1_aggregator_Aggregator'($vec: Vec ($1_aggregator_Aggregator)): $1_option_Option'$1_aggregator_Aggregator';
function {:inline} $Update'$1_option_Option'$1_aggregator_Aggregator''_vec(s: $1_option_Option'$1_aggregator_Aggregator', x: Vec ($1_aggregator_Aggregator)): $1_option_Option'$1_aggregator_Aggregator' {
    $1_option_Option'$1_aggregator_Aggregator'(x)
}
function $IsValid'$1_option_Option'$1_aggregator_Aggregator''(s: $1_option_Option'$1_aggregator_Aggregator'): bool {
    $IsValid'vec'$1_aggregator_Aggregator''($vec#$1_option_Option'$1_aggregator_Aggregator'(s))
}
function {:inline} $IsEqual'$1_option_Option'$1_aggregator_Aggregator''(s1: $1_option_Option'$1_aggregator_Aggregator', s2: $1_option_Option'$1_aggregator_Aggregator'): bool {
    $IsEqual'vec'$1_aggregator_Aggregator''($vec#$1_option_Option'$1_aggregator_Aggregator'(s1), $vec#$1_option_Option'$1_aggregator_Aggregator'(s2))}

// struct option::Option<optional_aggregator::Integer> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:7:5+81
type {:datatype} $1_option_Option'$1_optional_aggregator_Integer';
function {:constructor} $1_option_Option'$1_optional_aggregator_Integer'($vec: Vec ($1_optional_aggregator_Integer)): $1_option_Option'$1_optional_aggregator_Integer';
function {:inline} $Update'$1_option_Option'$1_optional_aggregator_Integer''_vec(s: $1_option_Option'$1_optional_aggregator_Integer', x: Vec ($1_optional_aggregator_Integer)): $1_option_Option'$1_optional_aggregator_Integer' {
    $1_option_Option'$1_optional_aggregator_Integer'(x)
}
function $IsValid'$1_option_Option'$1_optional_aggregator_Integer''(s: $1_option_Option'$1_optional_aggregator_Integer'): bool {
    $IsValid'vec'$1_optional_aggregator_Integer''($vec#$1_option_Option'$1_optional_aggregator_Integer'(s))
}
function {:inline} $IsEqual'$1_option_Option'$1_optional_aggregator_Integer''(s1: $1_option_Option'$1_optional_aggregator_Integer', s2: $1_option_Option'$1_optional_aggregator_Integer'): bool {
    $IsEqual'vec'$1_optional_aggregator_Integer''($vec#$1_option_Option'$1_optional_aggregator_Integer'(s1), $vec#$1_option_Option'$1_optional_aggregator_Integer'(s2))}

// struct option::Option<optional_aggregator::OptionalAggregator> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:7:5+81
type {:datatype} $1_option_Option'$1_optional_aggregator_OptionalAggregator';
function {:constructor} $1_option_Option'$1_optional_aggregator_OptionalAggregator'($vec: Vec ($1_optional_aggregator_OptionalAggregator)): $1_option_Option'$1_optional_aggregator_OptionalAggregator';
function {:inline} $Update'$1_option_Option'$1_optional_aggregator_OptionalAggregator''_vec(s: $1_option_Option'$1_optional_aggregator_OptionalAggregator', x: Vec ($1_optional_aggregator_OptionalAggregator)): $1_option_Option'$1_optional_aggregator_OptionalAggregator' {
    $1_option_Option'$1_optional_aggregator_OptionalAggregator'(x)
}
function $IsValid'$1_option_Option'$1_optional_aggregator_OptionalAggregator''(s: $1_option_Option'$1_optional_aggregator_OptionalAggregator'): bool {
    $IsValid'vec'$1_optional_aggregator_OptionalAggregator''($vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'(s))
}
function {:inline} $IsEqual'$1_option_Option'$1_optional_aggregator_OptionalAggregator''(s1: $1_option_Option'$1_optional_aggregator_OptionalAggregator', s2: $1_option_Option'$1_optional_aggregator_OptionalAggregator'): bool {
    $IsEqual'vec'$1_optional_aggregator_OptionalAggregator''($vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'(s1), $vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'(s2))}

// fun option::borrow_mut<aggregator::Aggregator> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+165
procedure {:inline 1} $1_option_borrow_mut'$1_aggregator_Aggregator'(_$t0: $Mutation ($1_option_Option'$1_aggregator_Aggregator')) returns ($ret0: $Mutation ($1_aggregator_Aggregator), $ret1: $Mutation ($1_option_Option'$1_aggregator_Aggregator'))
{
    // declare local variables
    var $t1: $1_option_Option'$1_aggregator_Aggregator';
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: $Mutation (Vec ($1_aggregator_Aggregator));
    var $t6: int;
    var $t7: $Mutation ($1_aggregator_Aggregator);
    var $t0: $Mutation ($1_option_Option'$1_aggregator_Aggregator');
    var $temp_0'$1_aggregator_Aggregator': $1_aggregator_Aggregator;
    var $temp_0'$1_option_Option'$1_aggregator_Aggregator'': $1_option_Option'$1_aggregator_Aggregator';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+1
    assume {:print "$at(12,5765,5766)"} true;
    $temp_0'$1_option_Option'$1_aggregator_Aggregator'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_aggregator_Aggregator''} $temp_0'$1_option_Option'$1_aggregator_Aggregator'' == $temp_0'$1_option_Option'$1_aggregator_Aggregator'';

    // $t1 := read_ref($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:25+1
    assume {:print "$at(12,5861,5862)"} true;
    $t1 := $Dereference($t0);

    // $t2 := opaque begin: option::is_some<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsValid'bool'($t2);

    // assume Eq<bool>($t2, option::spec_is_some<#0>($t1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsEqual'bool'($t2, $1_option_spec_is_some'$1_aggregator_Aggregator'($t1));

    // $t2 := opaque end: option::is_some<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // if ($t2) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L0:

    // pack_ref_deep($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;

    // destroy($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36

    // $t3 := 262145 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:29+15
    $t3 := 262145;
    assume $IsValid'u64'($t3);

    // trace_abort($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;
    assume {:print "$track_abort(1,1):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    $t4 := $t3;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    goto L4;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:33+1
    assume {:print "$at(12,5915,5916)"} true;
L2:

    // $t5 := borrow_field<option::Option<#0>>.vec($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:28+10
    assume {:print "$at(12,5910,5920)"} true;
    $t5 := $ChildMutation($t0, 0, $vec#$1_option_Option'$1_aggregator_Aggregator'($Dereference($t0)));

    // $t6 := 0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:40+1
    $t6 := 0;
    assume $IsValid'u64'($t6);

    // $t7 := vector::borrow_mut<#0>($t5, $t6) on_abort goto L4 with $t4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    call $t7,$t5 := $1_vector_borrow_mut'$1_aggregator_Aggregator'($t5, $t6);
    if ($abort_flag) {
        assume {:print "$at(12,5891,5924)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(1,1):", $t4} $t4 == $t4;
        goto L4;
    }

    // trace_return[0]($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_aggregator_Aggregator' := $Dereference($t7);
    assume {:print "$track_return(1,1,0):", $temp_0'$1_aggregator_Aggregator'} $temp_0'$1_aggregator_Aggregator' == $temp_0'$1_aggregator_Aggregator';

    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'$1_aggregator_Aggregator'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_aggregator_Aggregator''} $temp_0'$1_option_Option'$1_aggregator_Aggregator'' == $temp_0'$1_option_Option'$1_aggregator_Aggregator'';

    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'$1_aggregator_Aggregator'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_aggregator_Aggregator''} $temp_0'$1_option_Option'$1_aggregator_Aggregator'' == $temp_0'$1_option_Option'$1_aggregator_Aggregator'';

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
L3:

    // return $t7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
    $ret0 := $t7;
    $ret1 := $t0;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
L4:

    // abort($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun option::borrow_mut<optional_aggregator::Integer> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+165
procedure {:inline 1} $1_option_borrow_mut'$1_optional_aggregator_Integer'(_$t0: $Mutation ($1_option_Option'$1_optional_aggregator_Integer')) returns ($ret0: $Mutation ($1_optional_aggregator_Integer), $ret1: $Mutation ($1_option_Option'$1_optional_aggregator_Integer'))
{
    // declare local variables
    var $t1: $1_option_Option'$1_optional_aggregator_Integer';
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: $Mutation (Vec ($1_optional_aggregator_Integer));
    var $t6: int;
    var $t7: $Mutation ($1_optional_aggregator_Integer);
    var $t0: $Mutation ($1_option_Option'$1_optional_aggregator_Integer');
    var $temp_0'$1_option_Option'$1_optional_aggregator_Integer'': $1_option_Option'$1_optional_aggregator_Integer';
    var $temp_0'$1_optional_aggregator_Integer': $1_optional_aggregator_Integer;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+1
    assume {:print "$at(12,5765,5766)"} true;
    $temp_0'$1_option_Option'$1_optional_aggregator_Integer'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_optional_aggregator_Integer''} $temp_0'$1_option_Option'$1_optional_aggregator_Integer'' == $temp_0'$1_option_Option'$1_optional_aggregator_Integer'';

    // $t1 := read_ref($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:25+1
    assume {:print "$at(12,5861,5862)"} true;
    $t1 := $Dereference($t0);

    // $t2 := opaque begin: option::is_some<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsValid'bool'($t2);

    // assume Eq<bool>($t2, option::spec_is_some<#0>($t1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsEqual'bool'($t2, $1_option_spec_is_some'$1_optional_aggregator_Integer'($t1));

    // $t2 := opaque end: option::is_some<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // if ($t2) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L0:

    // pack_ref_deep($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;

    // destroy($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36

    // $t3 := 262145 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:29+15
    $t3 := 262145;
    assume $IsValid'u64'($t3);

    // trace_abort($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;
    assume {:print "$track_abort(1,1):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    $t4 := $t3;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    goto L4;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:33+1
    assume {:print "$at(12,5915,5916)"} true;
L2:

    // $t5 := borrow_field<option::Option<#0>>.vec($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:28+10
    assume {:print "$at(12,5910,5920)"} true;
    $t5 := $ChildMutation($t0, 0, $vec#$1_option_Option'$1_optional_aggregator_Integer'($Dereference($t0)));

    // $t6 := 0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:40+1
    $t6 := 0;
    assume $IsValid'u64'($t6);

    // $t7 := vector::borrow_mut<#0>($t5, $t6) on_abort goto L4 with $t4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    call $t7,$t5 := $1_vector_borrow_mut'$1_optional_aggregator_Integer'($t5, $t6);
    if ($abort_flag) {
        assume {:print "$at(12,5891,5924)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(1,1):", $t4} $t4 == $t4;
        goto L4;
    }

    // trace_return[0]($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_optional_aggregator_Integer' := $Dereference($t7);
    assume {:print "$track_return(1,1,0):", $temp_0'$1_optional_aggregator_Integer'} $temp_0'$1_optional_aggregator_Integer' == $temp_0'$1_optional_aggregator_Integer';

    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'$1_optional_aggregator_Integer'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_optional_aggregator_Integer''} $temp_0'$1_option_Option'$1_optional_aggregator_Integer'' == $temp_0'$1_option_Option'$1_optional_aggregator_Integer'';

    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'$1_optional_aggregator_Integer'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_optional_aggregator_Integer''} $temp_0'$1_option_Option'$1_optional_aggregator_Integer'' == $temp_0'$1_option_Option'$1_optional_aggregator_Integer'';

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
L3:

    // return $t7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
    $ret0 := $t7;
    $ret1 := $t0;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
L4:

    // abort($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun option::borrow_mut<optional_aggregator::OptionalAggregator> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+165
procedure {:inline 1} $1_option_borrow_mut'$1_optional_aggregator_OptionalAggregator'(_$t0: $Mutation ($1_option_Option'$1_optional_aggregator_OptionalAggregator')) returns ($ret0: $Mutation ($1_optional_aggregator_OptionalAggregator), $ret1: $Mutation ($1_option_Option'$1_optional_aggregator_OptionalAggregator'))
{
    // declare local variables
    var $t1: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: $Mutation (Vec ($1_optional_aggregator_OptionalAggregator));
    var $t6: int;
    var $t7: $Mutation ($1_optional_aggregator_OptionalAggregator);
    var $t0: $Mutation ($1_option_Option'$1_optional_aggregator_OptionalAggregator');
    var $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'': $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $temp_0'$1_optional_aggregator_OptionalAggregator': $1_optional_aggregator_OptionalAggregator;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:173:5+1
    assume {:print "$at(12,5765,5766)"} true;
    $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator''} $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' == $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'';

    // $t1 := read_ref($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:25+1
    assume {:print "$at(12,5861,5862)"} true;
    $t1 := $Dereference($t0);

    // $t2 := opaque begin: option::is_some<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsValid'bool'($t2);

    // assume Eq<bool>($t2, option::spec_is_some<#0>($t1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10
    assume $IsEqual'bool'($t2, $1_option_spec_is_some'$1_optional_aggregator_OptionalAggregator'($t1));

    // $t2 := opaque end: option::is_some<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:17+10

    // if ($t2) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
L0:

    // pack_ref_deep($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;

    // destroy($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36

    // $t3 := 262145 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:29+15
    $t3 := 262145;
    assume $IsValid'u64'($t3);

    // trace_abort($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    assume {:print "$at(12,5845,5881)"} true;
    assume {:print "$track_abort(1,1):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    $t4 := $t3;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:174:9+36
    goto L4;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:33+1
    assume {:print "$at(12,5915,5916)"} true;
L2:

    // $t5 := borrow_field<option::Option<#0>>.vec($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:28+10
    assume {:print "$at(12,5910,5920)"} true;
    $t5 := $ChildMutation($t0, 0, $vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'($Dereference($t0)));

    // $t6 := 0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:40+1
    $t6 := 0;
    assume $IsValid'u64'($t6);

    // $t7 := vector::borrow_mut<#0>($t5, $t6) on_abort goto L4 with $t4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    call $t7,$t5 := $1_vector_borrow_mut'$1_optional_aggregator_OptionalAggregator'($t5, $t6);
    if ($abort_flag) {
        assume {:print "$at(12,5891,5924)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(1,1):", $t4} $t4 == $t4;
        goto L4;
    }

    // trace_return[0]($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_optional_aggregator_OptionalAggregator' := $Dereference($t7);
    assume {:print "$track_return(1,1,0):", $temp_0'$1_optional_aggregator_OptionalAggregator'} $temp_0'$1_optional_aggregator_OptionalAggregator' == $temp_0'$1_optional_aggregator_OptionalAggregator';

    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator''} $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' == $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'';

    // trace_local[t]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:175:9+33
    $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' := $Dereference($t0);
    assume {:print "$track_local(1,1,0):", $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator''} $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' == $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'';

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
L3:

    // return $t7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
    $ret0 := $t7;
    $ret1 := $t0;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
L4:

    // abort($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:176:5+1
    assume {:print "$at(12,5929,5930)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// struct string::String at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/string.move:13:5+70
type {:datatype} $1_string_String;
function {:constructor} $1_string_String($bytes: Vec (int)): $1_string_String;
function {:inline} $Update'$1_string_String'_bytes(s: $1_string_String, x: Vec (int)): $1_string_String {
    $1_string_String(x)
}
function $IsValid'$1_string_String'(s: $1_string_String): bool {
    $IsValid'vec'u8''($bytes#$1_string_String(s))
}
function {:inline} $IsEqual'$1_string_String'(s1: $1_string_String, s2: $1_string_String): bool {
    $IsEqual'vec'u8''($bytes#$1_string_String(s1), $bytes#$1_string_String(s2))}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
function {:inline} $1_signer_$address_of(s: $signer): int {
    $1_signer_$borrow_address(s)
}

// fun signer::address_of [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
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
    // trace_local[s]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+1
    assume {:print "$at(13,396,397)"} true;
    assume {:print "$track_local(3,0,0):", $t0} $t0 == $t0;

    // $t1 := signer::borrow_address($t0) on_abort goto L2 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:10+17
    assume {:print "$at(13,450,467)"} true;
    call $t1 := $1_signer_borrow_address($t0);
    if ($abort_flag) {
        assume {:print "$at(13,450,467)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(3,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_return[0]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:9+18
    assume {:print "$track_return(3,0,0):", $t1} $t1 == $t1;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(13,472,473)"} true;
L1:

    // return $t1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(13,472,473)"} true;
    $ret0 := $t1;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
L2:

    // abort($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(13,472,473)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun error::invalid_argument [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:3+76
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
    // trace_local[r]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:3+1
    assume {:print "$at(9,3082,3083)"} true;
    assume {:print "$track_local(4,4,0):", $t0} $t0 == $t0;

    // $t1 := 1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:57+16
    $t1 := 1;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(9,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume {:print "$at(9,3126,3156)"} true;

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume {:print "$track_return(4,4,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:78+1
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:78+1
    assume {:print "$at(9,3157,3158)"} true;
    $ret0 := $t3;
    return;

}

// fun error::not_found [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+61
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
    // trace_local[r]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+1
    assume {:print "$at(9,3461,3462)"} true;
    assume {:print "$track_local(4,6,0):", $t0} $t0 == $t0;

    // $t1 := 6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:49+9
    $t1 := 6;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(9,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$at(9,3497,3520)"} true;

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$track_return(4,6,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
    assume {:print "$at(9,3521,3522)"} true;
    $ret0 := $t3;
    return;

}

// fun error::out_of_range [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:3+68
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
    // trace_local[r]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:3+1
    assume {:print "$at(9,3161,3162)"} true;
    assume {:print "$track_local(4,8,0):", $t0} $t0 == $t0;

    // $t1 := 2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:53+12
    $t1 := 2;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(9,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume {:print "$at(9,3201,3227)"} true;

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume {:print "$track_return(4,8,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:70+1
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:70+1
    assume {:print "$at(9,3228,3229)"} true;
    $ret0 := $t3;
    return;

}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.spec.move:38:10+40
function  $1_features_spec_is_enabled(feature: int): bool;
axiom (forall feature: int ::
(var $$res := $1_features_spec_is_enabled(feature);
$IsValid'bool'($$res)));

// struct features::Features at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:192:5+61
type {:datatype} $1_features_Features;
function {:constructor} $1_features_Features($features: Vec (bv8)): $1_features_Features;
function {:inline} $Update'$1_features_Features'_features(s: $1_features_Features, x: Vec (bv8)): $1_features_Features {
    $1_features_Features(x)
}
function $IsValid'$1_features_Features'(s: $1_features_Features): bool {
    $IsValid'vec'bv8''($features#$1_features_Features(s))
}
function {:inline} $IsEqual'$1_features_Features'(s1: $1_features_Features, s2: $1_features_Features): bool {
    $IsEqual'vec'bv8''($features#$1_features_Features(s1), $features#$1_features_Features(s2))}
var $1_features_Features_$memory: $Memory $1_features_Features;

// fun features::collect_and_distribute_gas_fees [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:87:5+128
procedure {:inline 1} $1_features_collect_and_distribute_gas_fees() returns ($ret0: bool)
{
    // declare local variables
    var $t0: int;
    var $t1: bool;
    var $temp_0'bool': bool;

    // bytecode translation starts here
    // $t0 := 6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:88:20+31
    assume {:print "$at(7,3999,4030)"} true;
    $t0 := 6;
    assume $IsValid'u64'($t0);

    // $t1 := opaque begin: features::is_enabled($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:88:9+43

    // assume WellFormed($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:88:9+43
    assume $IsValid'bool'($t1);

    // assume Eq<bool>($t1, features::spec_is_enabled($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:88:9+43
    assume $IsEqual'bool'($t1, $1_features_spec_is_enabled($t0));

    // $t1 := opaque end: features::is_enabled($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:88:9+43

    // trace_return[0]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:88:9+43
    assume {:print "$track_return(5,6,0):", $t1} $t1 == $t1;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:89:5+1
    assume {:print "$at(7,4036,4037)"} true;
L1:

    // return $t1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:89:5+1
    assume {:print "$at(7,4036,4037)"} true;
    $ret0 := $t1;
    return;

}

// struct type_info::TypeInfo at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/type_info.move:17:5+145
type {:datatype} $1_type_info_TypeInfo;
function {:constructor} $1_type_info_TypeInfo($account_address: int, $module_name: Vec (int), $struct_name: Vec (int)): $1_type_info_TypeInfo;
function {:inline} $Update'$1_type_info_TypeInfo'_account_address(s: $1_type_info_TypeInfo, x: int): $1_type_info_TypeInfo {
    $1_type_info_TypeInfo(x, $module_name#$1_type_info_TypeInfo(s), $struct_name#$1_type_info_TypeInfo(s))
}
function {:inline} $Update'$1_type_info_TypeInfo'_module_name(s: $1_type_info_TypeInfo, x: Vec (int)): $1_type_info_TypeInfo {
    $1_type_info_TypeInfo($account_address#$1_type_info_TypeInfo(s), x, $struct_name#$1_type_info_TypeInfo(s))
}
function {:inline} $Update'$1_type_info_TypeInfo'_struct_name(s: $1_type_info_TypeInfo, x: Vec (int)): $1_type_info_TypeInfo {
    $1_type_info_TypeInfo($account_address#$1_type_info_TypeInfo(s), $module_name#$1_type_info_TypeInfo(s), x)
}
function $IsValid'$1_type_info_TypeInfo'(s: $1_type_info_TypeInfo): bool {
    $IsValid'address'($account_address#$1_type_info_TypeInfo(s))
      && $IsValid'vec'u8''($module_name#$1_type_info_TypeInfo(s))
      && $IsValid'vec'u8''($struct_name#$1_type_info_TypeInfo(s))
}
function {:inline} $IsEqual'$1_type_info_TypeInfo'(s1: $1_type_info_TypeInfo, s2: $1_type_info_TypeInfo): bool {
    $IsEqual'address'($account_address#$1_type_info_TypeInfo(s1), $account_address#$1_type_info_TypeInfo(s2))
    && $IsEqual'vec'u8''($module_name#$1_type_info_TypeInfo(s1), $module_name#$1_type_info_TypeInfo(s2))
    && $IsEqual'vec'u8''($struct_name#$1_type_info_TypeInfo(s1), $struct_name#$1_type_info_TypeInfo(s2))}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/system_addresses.move:59:5+99
function {:inline} $1_system_addresses_$is_aptos_framework_address(addr: int): bool {
    $IsEqual'address'(addr, 1)
}

// struct guid::GUID at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:7:5+50
type {:datatype} $1_guid_GUID;
function {:constructor} $1_guid_GUID($id: $1_guid_ID): $1_guid_GUID;
function {:inline} $Update'$1_guid_GUID'_id(s: $1_guid_GUID, x: $1_guid_ID): $1_guid_GUID {
    $1_guid_GUID(x)
}
function $IsValid'$1_guid_GUID'(s: $1_guid_GUID): bool {
    $IsValid'$1_guid_ID'($id#$1_guid_GUID(s))
}
function {:inline} $IsEqual'$1_guid_GUID'(s1: $1_guid_GUID, s2: $1_guid_GUID): bool {
    s1 == s2
}

// struct guid::ID at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:12:5+209
type {:datatype} $1_guid_ID;
function {:constructor} $1_guid_ID($creation_num: int, $addr: int): $1_guid_ID;
function {:inline} $Update'$1_guid_ID'_creation_num(s: $1_guid_ID, x: int): $1_guid_ID {
    $1_guid_ID(x, $addr#$1_guid_ID(s))
}
function {:inline} $Update'$1_guid_ID'_addr(s: $1_guid_ID, x: int): $1_guid_ID {
    $1_guid_ID($creation_num#$1_guid_ID(s), x)
}
function $IsValid'$1_guid_ID'(s: $1_guid_ID): bool {
    $IsValid'u64'($creation_num#$1_guid_ID(s))
      && $IsValid'address'($addr#$1_guid_ID(s))
}
function {:inline} $IsEqual'$1_guid_ID'(s1: $1_guid_ID, s2: $1_guid_ID): bool {
    s1 == s2
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u8'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u8'(bytes);
$IsValid'u8'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u64'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u64'(bytes);
$IsValid'u64'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u128'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u128'(bytes);
$IsValid'u128'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u256'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u256'(bytes);
$IsValid'u256'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'address'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'address'(bytes);
$IsValid'address'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'signer'(bytes: Vec (int)): $signer;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'signer'(bytes);
$IsValid'signer'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'u8''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'u8''(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'address''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'address''(bytes);
$IsValid'vec'address''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'vec'u8'''(bytes: Vec (int)): Vec (Vec (int));
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'vec'u8'''(bytes);
$IsValid'vec'vec'u8'''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'$1_aggregator_Aggregator''(bytes: Vec (int)): Vec ($1_aggregator_Aggregator);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'$1_aggregator_Aggregator''(bytes);
$IsValid'vec'$1_aggregator_Aggregator''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'$1_optional_aggregator_Integer''(bytes: Vec (int)): Vec ($1_optional_aggregator_Integer);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'$1_optional_aggregator_Integer''(bytes);
$IsValid'vec'$1_optional_aggregator_Integer''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'$1_optional_aggregator_OptionalAggregator''(bytes: Vec (int)): Vec ($1_optional_aggregator_OptionalAggregator);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'$1_optional_aggregator_OptionalAggregator''(bytes);
$IsValid'vec'$1_optional_aggregator_OptionalAggregator''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'#0''(bytes: Vec (int)): Vec (#0);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'#0''(bytes);
$IsValid'vec'#0''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_option_Option'address''(bytes: Vec (int)): $1_option_Option'address';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_option_Option'address''(bytes);
$IsValid'$1_option_Option'address''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_option_Option'$1_aggregator_Aggregator''(bytes: Vec (int)): $1_option_Option'$1_aggregator_Aggregator';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_option_Option'$1_aggregator_Aggregator''(bytes);
$IsValid'$1_option_Option'$1_aggregator_Aggregator''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_Integer''(bytes: Vec (int)): $1_option_Option'$1_optional_aggregator_Integer';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_Integer''(bytes);
$IsValid'$1_option_Option'$1_optional_aggregator_Integer''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_OptionalAggregator''(bytes: Vec (int)): $1_option_Option'$1_optional_aggregator_OptionalAggregator';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_option_Option'$1_optional_aggregator_OptionalAggregator''(bytes);
$IsValid'$1_option_Option'$1_optional_aggregator_OptionalAggregator''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_string_String'(bytes: Vec (int)): $1_string_String;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_string_String'(bytes);
$IsValid'$1_string_String'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_features_Features'(bytes: Vec (int)): $1_features_Features;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_features_Features'(bytes);
$IsValid'$1_features_Features'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_type_info_TypeInfo'(bytes: Vec (int)): $1_type_info_TypeInfo;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_type_info_TypeInfo'(bytes);
$IsValid'$1_type_info_TypeInfo'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_GUID'(bytes: Vec (int)): $1_guid_GUID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_GUID'(bytes);
$IsValid'$1_guid_GUID'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_ID'(bytes: Vec (int)): $1_guid_ID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_ID'(bytes);
$IsValid'$1_guid_ID'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_account_CoinRegisterEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_account_CoinRegisterEvent''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_account_KeyRotationEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_account_KeyRotationEvent''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_DepositEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_coin_DepositEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_DepositEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_coin_DepositEvent''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_WithdrawEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_coin_WithdrawEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_coin_WithdrawEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_coin_WithdrawEvent''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_chain_id_ChainId'(bytes: Vec (int)): $1_chain_id_ChainId;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_chain_id_ChainId'(bytes);
$IsValid'$1_chain_id_ChainId'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_Account'(bytes: Vec (int)): $1_account_Account;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_Account'(bytes);
$IsValid'$1_account_Account'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes: Vec (int)): $1_account_CapabilityOffer'$1_account_RotationCapability';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes);
$IsValid'$1_account_CapabilityOffer'$1_account_RotationCapability''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes: Vec (int)): $1_account_CapabilityOffer'$1_account_SignerCapability';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes);
$IsValid'$1_account_CapabilityOffer'$1_account_SignerCapability''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_aggregator_Aggregator'(bytes: Vec (int)): $1_aggregator_Aggregator;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_aggregator_Aggregator'(bytes);
$IsValid'$1_aggregator_Aggregator'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_optional_aggregator_Integer'(bytes: Vec (int)): $1_optional_aggregator_Integer;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_optional_aggregator_Integer'(bytes);
$IsValid'$1_optional_aggregator_Integer'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_optional_aggregator_OptionalAggregator'(bytes: Vec (int)): $1_optional_aggregator_OptionalAggregator;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_optional_aggregator_OptionalAggregator'(bytes);
$IsValid'$1_optional_aggregator_OptionalAggregator'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''(bytes: Vec (int)): $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''(bytes);
$IsValid'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''(bytes: Vec (int)): $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''(bytes);
$IsValid'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_coin_Coin'$1_aptos_coin_AptosCoin''(bytes: Vec (int)): $1_coin_Coin'$1_aptos_coin_AptosCoin';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_coin_Coin'$1_aptos_coin_AptosCoin''(bytes);
$IsValid'$1_coin_Coin'$1_aptos_coin_AptosCoin''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''(bytes: Vec (int)): $1_coin_CoinInfo'$1_aptos_coin_AptosCoin';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''(bytes);
$IsValid'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''(bytes: Vec (int)): $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''(bytes);
$IsValid'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_aptos_coin_AptosCoin'(bytes: Vec (int)): $1_aptos_coin_AptosCoin;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_aptos_coin_AptosCoin'(bytes);
$IsValid'$1_aptos_coin_AptosCoin'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(bytes: Vec (int)): $1_chain_status_GenesisEndMarker;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(bytes);
$IsValid'$1_chain_status_GenesisEndMarker'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(bytes: Vec (int)): $1_timestamp_CurrentTimeMicroseconds;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(bytes);
$IsValid'$1_timestamp_CurrentTimeMicroseconds'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_transaction_fee_AptosCoinCapabilities'(bytes: Vec (int)): $1_transaction_fee_AptosCoinCapabilities;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_transaction_fee_AptosCoinCapabilities'(bytes);
$IsValid'$1_transaction_fee_AptosCoinCapabilities'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_transaction_fee_CollectedFeesPerBlock'(bytes: Vec (int)): $1_transaction_fee_CollectedFeesPerBlock;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_transaction_fee_CollectedFeesPerBlock'(bytes);
$IsValid'$1_transaction_fee_CollectedFeesPerBlock'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_reconfiguration_Configuration'(bytes: Vec (int)): $1_reconfiguration_Configuration;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_reconfiguration_Configuration'(bytes);
$IsValid'$1_reconfiguration_Configuration'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_transaction_validation_TransactionValidation'(bytes: Vec (int)): $1_transaction_validation_TransactionValidation;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_transaction_validation_TransactionValidation'(bytes);
$IsValid'$1_transaction_validation_TransactionValidation'($$res)));

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'#0'(bytes: Vec (int)): #0;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'#0'(bytes);
$IsValid'#0'($$res)));

// struct event::EventHandle<account::CoinRegisterEvent> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
type {:datatype} $1_event_EventHandle'$1_account_CoinRegisterEvent';
function {:constructor} $1_event_EventHandle'$1_account_CoinRegisterEvent'($counter: int, $guid: $1_guid_GUID): $1_event_EventHandle'$1_account_CoinRegisterEvent';
function {:inline} $Update'$1_event_EventHandle'$1_account_CoinRegisterEvent''_counter(s: $1_event_EventHandle'$1_account_CoinRegisterEvent', x: int): $1_event_EventHandle'$1_account_CoinRegisterEvent' {
    $1_event_EventHandle'$1_account_CoinRegisterEvent'(x, $guid#$1_event_EventHandle'$1_account_CoinRegisterEvent'(s))
}
function {:inline} $Update'$1_event_EventHandle'$1_account_CoinRegisterEvent''_guid(s: $1_event_EventHandle'$1_account_CoinRegisterEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_account_CoinRegisterEvent' {
    $1_event_EventHandle'$1_account_CoinRegisterEvent'($counter#$1_event_EventHandle'$1_account_CoinRegisterEvent'(s), x)
}
function $IsValid'$1_event_EventHandle'$1_account_CoinRegisterEvent''(s: $1_event_EventHandle'$1_account_CoinRegisterEvent'): bool {
    $IsValid'u64'($counter#$1_event_EventHandle'$1_account_CoinRegisterEvent'(s))
      && $IsValid'$1_guid_GUID'($guid#$1_event_EventHandle'$1_account_CoinRegisterEvent'(s))
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''(s1: $1_event_EventHandle'$1_account_CoinRegisterEvent', s2: $1_event_EventHandle'$1_account_CoinRegisterEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<account::KeyRotationEvent> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
type {:datatype} $1_event_EventHandle'$1_account_KeyRotationEvent';
function {:constructor} $1_event_EventHandle'$1_account_KeyRotationEvent'($counter: int, $guid: $1_guid_GUID): $1_event_EventHandle'$1_account_KeyRotationEvent';
function {:inline} $Update'$1_event_EventHandle'$1_account_KeyRotationEvent''_counter(s: $1_event_EventHandle'$1_account_KeyRotationEvent', x: int): $1_event_EventHandle'$1_account_KeyRotationEvent' {
    $1_event_EventHandle'$1_account_KeyRotationEvent'(x, $guid#$1_event_EventHandle'$1_account_KeyRotationEvent'(s))
}
function {:inline} $Update'$1_event_EventHandle'$1_account_KeyRotationEvent''_guid(s: $1_event_EventHandle'$1_account_KeyRotationEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_account_KeyRotationEvent' {
    $1_event_EventHandle'$1_account_KeyRotationEvent'($counter#$1_event_EventHandle'$1_account_KeyRotationEvent'(s), x)
}
function $IsValid'$1_event_EventHandle'$1_account_KeyRotationEvent''(s: $1_event_EventHandle'$1_account_KeyRotationEvent'): bool {
    $IsValid'u64'($counter#$1_event_EventHandle'$1_account_KeyRotationEvent'(s))
      && $IsValid'$1_guid_GUID'($guid#$1_event_EventHandle'$1_account_KeyRotationEvent'(s))
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''(s1: $1_event_EventHandle'$1_account_KeyRotationEvent', s2: $1_event_EventHandle'$1_account_KeyRotationEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<coin::DepositEvent> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
type {:datatype} $1_event_EventHandle'$1_coin_DepositEvent';
function {:constructor} $1_event_EventHandle'$1_coin_DepositEvent'($counter: int, $guid: $1_guid_GUID): $1_event_EventHandle'$1_coin_DepositEvent';
function {:inline} $Update'$1_event_EventHandle'$1_coin_DepositEvent''_counter(s: $1_event_EventHandle'$1_coin_DepositEvent', x: int): $1_event_EventHandle'$1_coin_DepositEvent' {
    $1_event_EventHandle'$1_coin_DepositEvent'(x, $guid#$1_event_EventHandle'$1_coin_DepositEvent'(s))
}
function {:inline} $Update'$1_event_EventHandle'$1_coin_DepositEvent''_guid(s: $1_event_EventHandle'$1_coin_DepositEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_coin_DepositEvent' {
    $1_event_EventHandle'$1_coin_DepositEvent'($counter#$1_event_EventHandle'$1_coin_DepositEvent'(s), x)
}
function $IsValid'$1_event_EventHandle'$1_coin_DepositEvent''(s: $1_event_EventHandle'$1_coin_DepositEvent'): bool {
    $IsValid'u64'($counter#$1_event_EventHandle'$1_coin_DepositEvent'(s))
      && $IsValid'$1_guid_GUID'($guid#$1_event_EventHandle'$1_coin_DepositEvent'(s))
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_coin_DepositEvent''(s1: $1_event_EventHandle'$1_coin_DepositEvent', s2: $1_event_EventHandle'$1_coin_DepositEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<coin::WithdrawEvent> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
type {:datatype} $1_event_EventHandle'$1_coin_WithdrawEvent';
function {:constructor} $1_event_EventHandle'$1_coin_WithdrawEvent'($counter: int, $guid: $1_guid_GUID): $1_event_EventHandle'$1_coin_WithdrawEvent';
function {:inline} $Update'$1_event_EventHandle'$1_coin_WithdrawEvent''_counter(s: $1_event_EventHandle'$1_coin_WithdrawEvent', x: int): $1_event_EventHandle'$1_coin_WithdrawEvent' {
    $1_event_EventHandle'$1_coin_WithdrawEvent'(x, $guid#$1_event_EventHandle'$1_coin_WithdrawEvent'(s))
}
function {:inline} $Update'$1_event_EventHandle'$1_coin_WithdrawEvent''_guid(s: $1_event_EventHandle'$1_coin_WithdrawEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_coin_WithdrawEvent' {
    $1_event_EventHandle'$1_coin_WithdrawEvent'($counter#$1_event_EventHandle'$1_coin_WithdrawEvent'(s), x)
}
function $IsValid'$1_event_EventHandle'$1_coin_WithdrawEvent''(s: $1_event_EventHandle'$1_coin_WithdrawEvent'): bool {
    $IsValid'u64'($counter#$1_event_EventHandle'$1_coin_WithdrawEvent'(s))
      && $IsValid'$1_guid_GUID'($guid#$1_event_EventHandle'$1_coin_WithdrawEvent'(s))
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_coin_WithdrawEvent''(s1: $1_event_EventHandle'$1_coin_WithdrawEvent', s2: $1_event_EventHandle'$1_coin_WithdrawEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<reconfiguration::NewEpochEvent> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
type {:datatype} $1_event_EventHandle'$1_reconfiguration_NewEpochEvent';
function {:constructor} $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'($counter: int, $guid: $1_guid_GUID): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent';
function {:inline} $Update'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''_counter(s: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent', x: int): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent' {
    $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'(x, $guid#$1_event_EventHandle'$1_reconfiguration_NewEpochEvent'(s))
}
function {:inline} $Update'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''_guid(s: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent' {
    $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'($counter#$1_event_EventHandle'$1_reconfiguration_NewEpochEvent'(s), x)
}
function $IsValid'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(s: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'): bool {
    $IsValid'u64'($counter#$1_event_EventHandle'$1_reconfiguration_NewEpochEvent'(s))
      && $IsValid'$1_guid_GUID'($guid#$1_event_EventHandle'$1_reconfiguration_NewEpochEvent'(s))
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(s1: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent', s2: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'): bool {
    s1 == s2
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:22:5+97
function {:inline} $1_chain_id_$get($1_chain_id_ChainId_$memory: $Memory $1_chain_id_ChainId): int {
    $id#$1_chain_id_ChainId($ResourceValue($1_chain_id_ChainId_$memory, 1))
}

// struct chain_id::ChainId at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:9:5+45
type {:datatype} $1_chain_id_ChainId;
function {:constructor} $1_chain_id_ChainId($id: int): $1_chain_id_ChainId;
function {:inline} $Update'$1_chain_id_ChainId'_id(s: $1_chain_id_ChainId, x: int): $1_chain_id_ChainId {
    $1_chain_id_ChainId(x)
}
function $IsValid'$1_chain_id_ChainId'(s: $1_chain_id_ChainId): bool {
    $IsValid'u8'($id#$1_chain_id_ChainId(s))
}
function {:inline} $IsEqual'$1_chain_id_ChainId'(s1: $1_chain_id_ChainId, s2: $1_chain_id_ChainId): bool {
    s1 == s2
}
var $1_chain_id_ChainId_$memory: $Memory $1_chain_id_ChainId;

// fun chain_id::get [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:22:5+97
procedure {:inline 1} $1_chain_id_get() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: $1_chain_id_ChainId;
    var $t2: int;
    var $t3: int;
    var $temp_0'u8': int;

    // bytecode translation starts here
    // $t0 := 0x1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:23:32+16
    assume {:print "$at(87,912,928)"} true;
    $t0 := 1;
    assume $IsValid'address'($t0);

    // $t1 := get_global<chain_id::ChainId>($t0) on_abort goto L2 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:23:9+13
    if (!$ResourceExists($1_chain_id_ChainId_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t1 := $ResourceValue($1_chain_id_ChainId_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(87,889,902)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(17,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_field<chain_id::ChainId>.id($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:23:9+43
    $t3 := $id#$1_chain_id_ChainId($t1);

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:23:9+43
    assume {:print "$track_return(17,0,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:24:5+1
    assume {:print "$at(87,937,938)"} true;
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:24:5+1
    assume {:print "$at(87,937,938)"} true;
    $ret0 := $t3;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:24:5+1
L2:

    // abort($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_id.move:24:5+1
    assume {:print "$at(87,937,938)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:224:5+79
function {:inline} $1_account_$exists_at($1_account_Account_$memory: $Memory $1_account_Account, addr: int): bool {
    $ResourceExists($1_account_Account_$memory, addr)
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:250:5+141
function {:inline} $1_account_$get_authentication_key($1_account_Account_$memory: $Memory $1_account_Account, addr: int): Vec (int) {
    $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory, addr))
}

// struct account::Account at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:26:5+401
type {:datatype} $1_account_Account;
function {:constructor} $1_account_Account($authentication_key: Vec (int), $sequence_number: int, $guid_creation_num: int, $coin_register_events: $1_event_EventHandle'$1_account_CoinRegisterEvent', $key_rotation_events: $1_event_EventHandle'$1_account_KeyRotationEvent', $rotation_capability_offer: $1_account_CapabilityOffer'$1_account_RotationCapability', $signer_capability_offer: $1_account_CapabilityOffer'$1_account_SignerCapability'): $1_account_Account;
function {:inline} $Update'$1_account_Account'_authentication_key(s: $1_account_Account, x: Vec (int)): $1_account_Account {
    $1_account_Account(x, $sequence_number#$1_account_Account(s), $guid_creation_num#$1_account_Account(s), $coin_register_events#$1_account_Account(s), $key_rotation_events#$1_account_Account(s), $rotation_capability_offer#$1_account_Account(s), $signer_capability_offer#$1_account_Account(s))
}
function {:inline} $Update'$1_account_Account'_sequence_number(s: $1_account_Account, x: int): $1_account_Account {
    $1_account_Account($authentication_key#$1_account_Account(s), x, $guid_creation_num#$1_account_Account(s), $coin_register_events#$1_account_Account(s), $key_rotation_events#$1_account_Account(s), $rotation_capability_offer#$1_account_Account(s), $signer_capability_offer#$1_account_Account(s))
}
function {:inline} $Update'$1_account_Account'_guid_creation_num(s: $1_account_Account, x: int): $1_account_Account {
    $1_account_Account($authentication_key#$1_account_Account(s), $sequence_number#$1_account_Account(s), x, $coin_register_events#$1_account_Account(s), $key_rotation_events#$1_account_Account(s), $rotation_capability_offer#$1_account_Account(s), $signer_capability_offer#$1_account_Account(s))
}
function {:inline} $Update'$1_account_Account'_coin_register_events(s: $1_account_Account, x: $1_event_EventHandle'$1_account_CoinRegisterEvent'): $1_account_Account {
    $1_account_Account($authentication_key#$1_account_Account(s), $sequence_number#$1_account_Account(s), $guid_creation_num#$1_account_Account(s), x, $key_rotation_events#$1_account_Account(s), $rotation_capability_offer#$1_account_Account(s), $signer_capability_offer#$1_account_Account(s))
}
function {:inline} $Update'$1_account_Account'_key_rotation_events(s: $1_account_Account, x: $1_event_EventHandle'$1_account_KeyRotationEvent'): $1_account_Account {
    $1_account_Account($authentication_key#$1_account_Account(s), $sequence_number#$1_account_Account(s), $guid_creation_num#$1_account_Account(s), $coin_register_events#$1_account_Account(s), x, $rotation_capability_offer#$1_account_Account(s), $signer_capability_offer#$1_account_Account(s))
}
function {:inline} $Update'$1_account_Account'_rotation_capability_offer(s: $1_account_Account, x: $1_account_CapabilityOffer'$1_account_RotationCapability'): $1_account_Account {
    $1_account_Account($authentication_key#$1_account_Account(s), $sequence_number#$1_account_Account(s), $guid_creation_num#$1_account_Account(s), $coin_register_events#$1_account_Account(s), $key_rotation_events#$1_account_Account(s), x, $signer_capability_offer#$1_account_Account(s))
}
function {:inline} $Update'$1_account_Account'_signer_capability_offer(s: $1_account_Account, x: $1_account_CapabilityOffer'$1_account_SignerCapability'): $1_account_Account {
    $1_account_Account($authentication_key#$1_account_Account(s), $sequence_number#$1_account_Account(s), $guid_creation_num#$1_account_Account(s), $coin_register_events#$1_account_Account(s), $key_rotation_events#$1_account_Account(s), $rotation_capability_offer#$1_account_Account(s), x)
}
function $IsValid'$1_account_Account'(s: $1_account_Account): bool {
    $IsValid'vec'u8''($authentication_key#$1_account_Account(s))
      && $IsValid'u64'($sequence_number#$1_account_Account(s))
      && $IsValid'u64'($guid_creation_num#$1_account_Account(s))
      && $IsValid'$1_event_EventHandle'$1_account_CoinRegisterEvent''($coin_register_events#$1_account_Account(s))
      && $IsValid'$1_event_EventHandle'$1_account_KeyRotationEvent''($key_rotation_events#$1_account_Account(s))
      && $IsValid'$1_account_CapabilityOffer'$1_account_RotationCapability''($rotation_capability_offer#$1_account_Account(s))
      && $IsValid'$1_account_CapabilityOffer'$1_account_SignerCapability''($signer_capability_offer#$1_account_Account(s))
}
function {:inline} $IsEqual'$1_account_Account'(s1: $1_account_Account, s2: $1_account_Account): bool {
    $IsEqual'vec'u8''($authentication_key#$1_account_Account(s1), $authentication_key#$1_account_Account(s2))
    && $IsEqual'u64'($sequence_number#$1_account_Account(s1), $sequence_number#$1_account_Account(s2))
    && $IsEqual'u64'($guid_creation_num#$1_account_Account(s1), $guid_creation_num#$1_account_Account(s2))
    && $IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''($coin_register_events#$1_account_Account(s1), $coin_register_events#$1_account_Account(s2))
    && $IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''($key_rotation_events#$1_account_Account(s1), $key_rotation_events#$1_account_Account(s2))
    && $IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''($rotation_capability_offer#$1_account_Account(s1), $rotation_capability_offer#$1_account_Account(s2))
    && $IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''($signer_capability_offer#$1_account_Account(s1), $signer_capability_offer#$1_account_Account(s2))}
var $1_account_Account_$memory: $Memory $1_account_Account;

// struct account::CapabilityOffer<account::RotationCapability> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:45:5+68
type {:datatype} $1_account_CapabilityOffer'$1_account_RotationCapability';
function {:constructor} $1_account_CapabilityOffer'$1_account_RotationCapability'($for: $1_option_Option'address'): $1_account_CapabilityOffer'$1_account_RotationCapability';
function {:inline} $Update'$1_account_CapabilityOffer'$1_account_RotationCapability''_for(s: $1_account_CapabilityOffer'$1_account_RotationCapability', x: $1_option_Option'address'): $1_account_CapabilityOffer'$1_account_RotationCapability' {
    $1_account_CapabilityOffer'$1_account_RotationCapability'(x)
}
function $IsValid'$1_account_CapabilityOffer'$1_account_RotationCapability''(s: $1_account_CapabilityOffer'$1_account_RotationCapability'): bool {
    $IsValid'$1_option_Option'address''($for#$1_account_CapabilityOffer'$1_account_RotationCapability'(s))
}
function {:inline} $IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''(s1: $1_account_CapabilityOffer'$1_account_RotationCapability', s2: $1_account_CapabilityOffer'$1_account_RotationCapability'): bool {
    $IsEqual'$1_option_Option'address''($for#$1_account_CapabilityOffer'$1_account_RotationCapability'(s1), $for#$1_account_CapabilityOffer'$1_account_RotationCapability'(s2))}

// struct account::CapabilityOffer<account::SignerCapability> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:45:5+68
type {:datatype} $1_account_CapabilityOffer'$1_account_SignerCapability';
function {:constructor} $1_account_CapabilityOffer'$1_account_SignerCapability'($for: $1_option_Option'address'): $1_account_CapabilityOffer'$1_account_SignerCapability';
function {:inline} $Update'$1_account_CapabilityOffer'$1_account_SignerCapability''_for(s: $1_account_CapabilityOffer'$1_account_SignerCapability', x: $1_option_Option'address'): $1_account_CapabilityOffer'$1_account_SignerCapability' {
    $1_account_CapabilityOffer'$1_account_SignerCapability'(x)
}
function $IsValid'$1_account_CapabilityOffer'$1_account_SignerCapability''(s: $1_account_CapabilityOffer'$1_account_SignerCapability'): bool {
    $IsValid'$1_option_Option'address''($for#$1_account_CapabilityOffer'$1_account_SignerCapability'(s))
}
function {:inline} $IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''(s1: $1_account_CapabilityOffer'$1_account_SignerCapability', s2: $1_account_CapabilityOffer'$1_account_SignerCapability'): bool {
    $IsEqual'$1_option_Option'address''($for#$1_account_CapabilityOffer'$1_account_SignerCapability'(s1), $for#$1_account_CapabilityOffer'$1_account_SignerCapability'(s2))}

// struct account::CoinRegisterEvent at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:41:5+77
type {:datatype} $1_account_CoinRegisterEvent;
function {:constructor} $1_account_CoinRegisterEvent($type_info: $1_type_info_TypeInfo): $1_account_CoinRegisterEvent;
function {:inline} $Update'$1_account_CoinRegisterEvent'_type_info(s: $1_account_CoinRegisterEvent, x: $1_type_info_TypeInfo): $1_account_CoinRegisterEvent {
    $1_account_CoinRegisterEvent(x)
}
function $IsValid'$1_account_CoinRegisterEvent'(s: $1_account_CoinRegisterEvent): bool {
    $IsValid'$1_type_info_TypeInfo'($type_info#$1_account_CoinRegisterEvent(s))
}
function {:inline} $IsEqual'$1_account_CoinRegisterEvent'(s1: $1_account_CoinRegisterEvent, s2: $1_account_CoinRegisterEvent): bool {
    $IsEqual'$1_type_info_TypeInfo'($type_info#$1_account_CoinRegisterEvent(s1), $type_info#$1_account_CoinRegisterEvent(s2))}

// struct account::KeyRotationEvent at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:36:5+135
type {:datatype} $1_account_KeyRotationEvent;
function {:constructor} $1_account_KeyRotationEvent($old_authentication_key: Vec (int), $new_authentication_key: Vec (int)): $1_account_KeyRotationEvent;
function {:inline} $Update'$1_account_KeyRotationEvent'_old_authentication_key(s: $1_account_KeyRotationEvent, x: Vec (int)): $1_account_KeyRotationEvent {
    $1_account_KeyRotationEvent(x, $new_authentication_key#$1_account_KeyRotationEvent(s))
}
function {:inline} $Update'$1_account_KeyRotationEvent'_new_authentication_key(s: $1_account_KeyRotationEvent, x: Vec (int)): $1_account_KeyRotationEvent {
    $1_account_KeyRotationEvent($old_authentication_key#$1_account_KeyRotationEvent(s), x)
}
function $IsValid'$1_account_KeyRotationEvent'(s: $1_account_KeyRotationEvent): bool {
    $IsValid'vec'u8''($old_authentication_key#$1_account_KeyRotationEvent(s))
      && $IsValid'vec'u8''($new_authentication_key#$1_account_KeyRotationEvent(s))
}
function {:inline} $IsEqual'$1_account_KeyRotationEvent'(s1: $1_account_KeyRotationEvent, s2: $1_account_KeyRotationEvent): bool {
    $IsEqual'vec'u8''($old_authentication_key#$1_account_KeyRotationEvent(s1), $old_authentication_key#$1_account_KeyRotationEvent(s2))
    && $IsEqual'vec'u8''($new_authentication_key#$1_account_KeyRotationEvent(s1), $new_authentication_key#$1_account_KeyRotationEvent(s2))}

// struct account::RotationCapability at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:47:5+62
type {:datatype} $1_account_RotationCapability;
function {:constructor} $1_account_RotationCapability($account: int): $1_account_RotationCapability;
function {:inline} $Update'$1_account_RotationCapability'_account(s: $1_account_RotationCapability, x: int): $1_account_RotationCapability {
    $1_account_RotationCapability(x)
}
function $IsValid'$1_account_RotationCapability'(s: $1_account_RotationCapability): bool {
    $IsValid'address'($account#$1_account_RotationCapability(s))
}
function {:inline} $IsEqual'$1_account_RotationCapability'(s1: $1_account_RotationCapability, s2: $1_account_RotationCapability): bool {
    s1 == s2
}

// struct account::SignerCapability at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:49:5+60
type {:datatype} $1_account_SignerCapability;
function {:constructor} $1_account_SignerCapability($account: int): $1_account_SignerCapability;
function {:inline} $Update'$1_account_SignerCapability'_account(s: $1_account_SignerCapability, x: int): $1_account_SignerCapability {
    $1_account_SignerCapability(x)
}
function $IsValid'$1_account_SignerCapability'(s: $1_account_SignerCapability): bool {
    $IsValid'address'($account#$1_account_SignerCapability(s))
}
function {:inline} $IsEqual'$1_account_SignerCapability'(s1: $1_account_SignerCapability, s2: $1_account_SignerCapability): bool {
    s1 == s2
}

// fun account::exists_at [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:224:5+79
procedure {:inline 1} $1_account_exists_at(_$t0: int) returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[addr]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:224:5+1
    assume {:print "$at(71,11367,11368)"} true;
    assume {:print "$track_local(18,9,0):", $t0} $t0 == $t0;

    // $t1 := exists<account::Account>($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:225:9+6
    assume {:print "$at(71,11419,11425)"} true;
    $t1 := $ResourceExists($1_account_Account_$memory, $t0);

    // trace_return[0]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:225:9+21
    assume {:print "$track_return(18,9,0):", $t1} $t1 == $t1;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:226:5+1
    assume {:print "$at(71,11445,11446)"} true;
L1:

    // return $t1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:226:5+1
    assume {:print "$at(71,11445,11446)"} true;
    $ret0 := $t1;
    return;

}

// fun account::get_authentication_key [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:250:5+141
procedure {:inline 1} $1_account_get_authentication_key(_$t0: int) returns ($ret0: Vec (int))
{
    // declare local variables
    var $t1: $1_account_Account;
    var $t2: int;
    var $t3: Vec (int);
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[addr]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:250:5+1
    assume {:print "$at(71,12127,12128)"} true;
    assume {:print "$track_local(18,10,0):", $t0} $t0 == $t0;

    // $t1 := get_global<account::Account>($t0) on_abort goto L2 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:251:9+13
    assume {:print "$at(71,12215,12228)"} true;
    if (!$ResourceExists($1_account_Account_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t1 := $ResourceValue($1_account_Account_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(71,12215,12228)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(18,10):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_field<account::Account>.authentication_key($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:251:9+47
    $t3 := $authentication_key#$1_account_Account($t1);

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:251:9+47
    assume {:print "$track_return(18,10,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:252:5+1
    assume {:print "$at(71,12267,12268)"} true;
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:252:5+1
    assume {:print "$at(71,12267,12268)"} true;
    $ret0 := $t3;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:252:5+1
L2:

    // abort($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:252:5+1
    assume {:print "$at(71,12267,12268)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun account::get_sequence_number [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:234:5+128
procedure {:inline 1} $1_account_get_sequence_number(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: $1_account_Account;
    var $t2: int;
    var $t3: int;
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[addr]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:234:5+1
    assume {:print "$at(71,11619,11620)"} true;
    assume {:print "$track_local(18,12,0):", $t0} $t0 == $t0;

    // $t1 := get_global<account::Account>($t0) on_abort goto L2 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:235:9+13
    assume {:print "$at(71,11697,11710)"} true;
    if (!$ResourceExists($1_account_Account_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t1 := $ResourceValue($1_account_Account_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(71,11697,11710)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(18,12):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_field<account::Account>.sequence_number($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:235:9+44
    $t3 := $sequence_number#$1_account_Account($t1);

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:235:9+44
    assume {:print "$track_return(18,12,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:236:5+1
    assume {:print "$at(71,11746,11747)"} true;
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:236:5+1
    assume {:print "$at(71,11746,11747)"} true;
    $ret0 := $t3;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:236:5+1
L2:

    // abort($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:236:5+1
    assume {:print "$at(71,11746,11747)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun account::increment_sequence_number [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:238:5+356
procedure {:inline 1} $1_account_increment_sequence_number(_$t0: int) returns ()
{
    // declare local variables
    var $t1: $Mutation (int);
    var $t2: int;
    var $t3: $Mutation ($1_account_Account);
    var $t4: int;
    var $t5: $Mutation (int);
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: bool;
    var $t10: int;
    var $t11: int;
    var $t12: int;
    var $t13: int;
    var $t14: int;
    var $t0: int;
    var $1_account_Account_$modifies: [int]bool;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // assume Identical($t2, select account::Account.sequence_number(global<account::Account>($t0))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:52:9+60
    assume {:print "$at(72,2145,2205)"} true;
    assume ($t2 == $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory, $t0)));

    // trace_local[addr]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:238:5+1
    assume {:print "$at(71,11753,11754)"} true;
    assume {:print "$track_local(18,15,0):", $t0} $t0 == $t0;

    // $t3 := borrow_global<account::Account>($t0) on_abort goto L4 with $t4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:239:36+17
    assume {:print "$at(71,11867,11884)"} true;
    if (!$ResourceExists($1_account_Account_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $Mutation($Global($t0), EmptyVec(), $ResourceValue($1_account_Account_$memory, $t0));
    }
    if ($abort_flag) {
        assume {:print "$at(71,11867,11884)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(18,15):", $t4} $t4 == $t4;
        goto L4;
    }

    // $t5 := borrow_field<account::Account>.sequence_number($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:239:31+53
    $t5 := $ChildMutation($t3, 1, $sequence_number#$1_account_Account($Dereference($t3)));

    // trace_local[sequence_number]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:239:13+15
    $temp_0'u64' := $Dereference($t5);
    assume {:print "$track_local(18,15,1):", $temp_0'u64'} $temp_0'u64' == $temp_0'u64';

    // $t6 := read_ref($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:242:14+16
    assume {:print "$at(71,11948,11964)"} true;
    $t6 := $Dereference($t5);

    // $t7 := (u128)($t6) on_abort goto L4 with $t4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:242:13+26
    call $t7 := $CastU128($t6);
    if ($abort_flag) {
        assume {:print "$at(71,11947,11973)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(18,15):", $t4} $t4 == $t4;
        goto L4;
    }

    // $t8 := 18446744073709551615 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:242:42+7
    $t8 := 18446744073709551615;
    assume $IsValid'u128'($t8);

    // $t9 := <($t7, $t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:242:40+1
    call $t9 := $Lt($t7, $t8);

    // if ($t9) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
    assume {:print "$at(71,11926,12052)"} true;
    if ($t9) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
    assume {:print "$at(71,11926,12052)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
L0:

    // pack_ref_deep($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
    assume {:print "$at(71,11926,12052)"} true;

    // destroy($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126

    // $t10 := 3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:243:33+24
    assume {:print "$at(71,12017,12041)"} true;
    $t10 := 3;
    assume $IsValid'u64'($t10);

    // $t11 := error::out_of_range($t10) on_abort goto L4 with $t4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:243:13+45
    call $t11 := $1_error_out_of_range($t10);
    if ($abort_flag) {
        assume {:print "$at(71,11997,12042)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(18,15):", $t4} $t4 == $t4;
        goto L4;
    }

    // trace_abort($t11) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
    assume {:print "$at(71,11926,12052)"} true;
    assume {:print "$track_abort(18,15):", $t11} $t11 == $t11;

    // $t4 := move($t11) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
    $t4 := $t11;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:241:9+126
    goto L4;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:29+15
    assume {:print "$at(71,12083,12098)"} true;
L2:

    // $t12 := read_ref($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:28+16
    assume {:print "$at(71,12082,12098)"} true;
    $t12 := $Dereference($t5);

    // $t13 := 1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:47+1
    $t13 := 1;
    assume $IsValid'u64'($t13);

    // $t14 := +($t12, $t13) on_abort goto L4 with $t4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:45+1
    call $t14 := $AddU64($t12, $t13);
    if ($abort_flag) {
        assume {:print "$at(71,12099,12100)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(18,15):", $t4} $t4 == $t4;
        goto L4;
    }

    // write_ref($t5, $t14) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:9+39
    $t5 := $UpdateMutation($t5, $t14);

    // write_back[Reference($t3).sequence_number (u64)]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:9+39
    $t3 := $UpdateMutation($t3, $Update'$1_account_Account'_sequence_number($Dereference($t3), $Dereference($t5)));

    // pack_ref_deep($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:9+39

    // write_back[account::Account@]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:246:9+39
    $1_account_Account_$memory := $ResourceUpdate($1_account_Account_$memory, $GlobalLocationAddress($t3),
        $Dereference($t3));

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:247:5+1
    assume {:print "$at(71,12108,12109)"} true;
L3:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:247:5+1
    assume {:print "$at(71,12108,12109)"} true;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:247:5+1
L4:

    // abort($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.move:247:5+1
    assume {:print "$at(71,12108,12109)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:190:5+141
function {:inline} $1_optional_aggregator_$is_parallelizable(optional_aggregator: $1_optional_aggregator_OptionalAggregator): bool {
    $1_option_$is_some'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator(optional_aggregator))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.spec.move:134:10+323
function {:inline} $1_optional_aggregator_optional_aggregator_value(optional_aggregator: $1_optional_aggregator_OptionalAggregator): int {
    (if ($1_optional_aggregator_$is_parallelizable(optional_aggregator)) then ($1_aggregator_spec_aggregator_get_val($1_option_$borrow'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator(optional_aggregator)))) else ($value#$1_optional_aggregator_Integer($1_option_$borrow'$1_optional_aggregator_Integer'($integer#$1_optional_aggregator_OptionalAggregator(optional_aggregator)))))
}

// struct optional_aggregator::Integer at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:20:5+74
type {:datatype} $1_optional_aggregator_Integer;
function {:constructor} $1_optional_aggregator_Integer($value: int, $limit: int): $1_optional_aggregator_Integer;
function {:inline} $Update'$1_optional_aggregator_Integer'_value(s: $1_optional_aggregator_Integer, x: int): $1_optional_aggregator_Integer {
    $1_optional_aggregator_Integer(x, $limit#$1_optional_aggregator_Integer(s))
}
function {:inline} $Update'$1_optional_aggregator_Integer'_limit(s: $1_optional_aggregator_Integer, x: int): $1_optional_aggregator_Integer {
    $1_optional_aggregator_Integer($value#$1_optional_aggregator_Integer(s), x)
}
function $IsValid'$1_optional_aggregator_Integer'(s: $1_optional_aggregator_Integer): bool {
    $IsValid'u128'($value#$1_optional_aggregator_Integer(s))
      && $IsValid'u128'($limit#$1_optional_aggregator_Integer(s))
}
function {:inline} $IsEqual'$1_optional_aggregator_Integer'(s1: $1_optional_aggregator_Integer, s2: $1_optional_aggregator_Integer): bool {
    s1 == s2
}

// struct optional_aggregator::OptionalAggregator at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:64:5+175
type {:datatype} $1_optional_aggregator_OptionalAggregator;
function {:constructor} $1_optional_aggregator_OptionalAggregator($aggregator: $1_option_Option'$1_aggregator_Aggregator', $integer: $1_option_Option'$1_optional_aggregator_Integer'): $1_optional_aggregator_OptionalAggregator;
function {:inline} $Update'$1_optional_aggregator_OptionalAggregator'_aggregator(s: $1_optional_aggregator_OptionalAggregator, x: $1_option_Option'$1_aggregator_Aggregator'): $1_optional_aggregator_OptionalAggregator {
    $1_optional_aggregator_OptionalAggregator(x, $integer#$1_optional_aggregator_OptionalAggregator(s))
}
function {:inline} $Update'$1_optional_aggregator_OptionalAggregator'_integer(s: $1_optional_aggregator_OptionalAggregator, x: $1_option_Option'$1_optional_aggregator_Integer'): $1_optional_aggregator_OptionalAggregator {
    $1_optional_aggregator_OptionalAggregator($aggregator#$1_optional_aggregator_OptionalAggregator(s), x)
}
function $IsValid'$1_optional_aggregator_OptionalAggregator'(s: $1_optional_aggregator_OptionalAggregator): bool {
    $IsValid'$1_option_Option'$1_aggregator_Aggregator''($aggregator#$1_optional_aggregator_OptionalAggregator(s))
      && $IsValid'$1_option_Option'$1_optional_aggregator_Integer''($integer#$1_optional_aggregator_OptionalAggregator(s))
}
function {:inline} $IsEqual'$1_optional_aggregator_OptionalAggregator'(s1: $1_optional_aggregator_OptionalAggregator, s2: $1_optional_aggregator_OptionalAggregator): bool {
    $IsEqual'$1_option_Option'$1_aggregator_Aggregator''($aggregator#$1_optional_aggregator_OptionalAggregator(s1), $aggregator#$1_optional_aggregator_OptionalAggregator(s2))
    && $IsEqual'$1_option_Option'$1_optional_aggregator_Integer''($integer#$1_optional_aggregator_OptionalAggregator(s1), $integer#$1_optional_aggregator_OptionalAggregator(s2))}

// fun optional_aggregator::sub [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:168:5+427
procedure {:inline 1} $1_optional_aggregator_sub(_$t0: $Mutation ($1_optional_aggregator_OptionalAggregator), _$t1: int) returns ($ret0: $Mutation ($1_optional_aggregator_OptionalAggregator))
{
    // declare local variables
    var $t2: $1_option_Option'$1_aggregator_Aggregator';
    var $t3: bool;
    var $t4: $Mutation ($1_option_Option'$1_aggregator_Aggregator');
    var $t5: $Mutation ($1_aggregator_Aggregator);
    var $t6: int;
    var $t7: $1_aggregator_Aggregator;
    var $t8: bool;
    var $t9: $Mutation ($1_option_Option'$1_optional_aggregator_Integer');
    var $t10: $Mutation ($1_optional_aggregator_Integer);
    var $t0: $Mutation ($1_optional_aggregator_OptionalAggregator);
    var $t1: int;
    var $temp_0'$1_aggregator_Aggregator': $1_aggregator_Aggregator;
    var $temp_0'$1_optional_aggregator_OptionalAggregator': $1_optional_aggregator_OptionalAggregator;
    var $temp_0'u128': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[optional_aggregator]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:168:5+1
    assume {:print "$at(77,6333,6334)"} true;
    $temp_0'$1_optional_aggregator_OptionalAggregator' := $Dereference($t0);
    assume {:print "$track_local(22,12,0):", $temp_0'$1_optional_aggregator_OptionalAggregator'} $temp_0'$1_optional_aggregator_OptionalAggregator' == $temp_0'$1_optional_aggregator_OptionalAggregator';

    // trace_local[value]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:168:5+1
    assume {:print "$track_local(22,12,1):", $t1} $t1 == $t1;

    // $t2 := get_field<optional_aggregator::OptionalAggregator>.aggregator($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:29+31
    assume {:print "$at(77,6437,6468)"} true;
    $t2 := $aggregator#$1_optional_aggregator_OptionalAggregator($Dereference($t0));

    // $t3 := opaque begin: option::is_some<aggregator::Aggregator>($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:13+48

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:13+48
    assume $IsValid'bool'($t3);

    // assume Eq<bool>($t3, option::spec_is_some<aggregator::Aggregator>($t2)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:13+48
    assume $IsEqual'bool'($t3, $1_option_spec_is_some'$1_aggregator_Aggregator'($t2));

    // $t3 := opaque end: option::is_some<aggregator::Aggregator>($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:13+48

    // if ($t3) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:9+337
    if ($t3) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:170:54+19
    assume {:print "$at(77,6526,6545)"} true;
L1:

    // $t4 := borrow_field<optional_aggregator::OptionalAggregator>.aggregator($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:170:49+35
    assume {:print "$at(77,6521,6556)"} true;
    $t4 := $ChildMutation($t0, 0, $aggregator#$1_optional_aggregator_OptionalAggregator($Dereference($t0)));

    // $t5 := option::borrow_mut<aggregator::Aggregator>($t4) on_abort goto L4 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:170:30+55
    call $t5,$t4 := $1_option_borrow_mut'$1_aggregator_Aggregator'($t4);
    if ($abort_flag) {
        assume {:print "$at(77,6502,6557)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(22,12):", $t6} $t6 == $t6;
        goto L4;
    }

    // opaque begin: aggregator::sub($t5, $t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    assume {:print "$at(77,6571,6605)"} true;

    // $t7 := read_ref($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    $t7 := $Dereference($t5);

    // assume Identical($t8, Lt(aggregator::spec_aggregator_get_val($t5), $t1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    assume ($t8 == ($1_aggregator_spec_aggregator_get_val($Dereference($t5)) < $t1));

    // if ($t8) goto L7 else goto L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    if ($t8) { goto L7; } else { goto L5; }

    // label L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
L6:

    // trace_abort($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    assume {:print "$at(77,6571,6605)"} true;
    assume {:print "$track_abort(22,12):", $t6} $t6 == $t6;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    goto L4;

    // label L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
L5:

    // $t5 := havoc[mut]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    assume {:print "$at(77,6571,6605)"} true;
    havoc $temp_0'$1_aggregator_Aggregator';
    $t5 := $UpdateMutation($t5, $temp_0'$1_aggregator_Aggregator');

    // assume WellFormed($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    assume $IsValid'$1_aggregator_Aggregator'($Dereference($t5));

    // assume Eq<u128>(aggregator::spec_get_limit($t5), aggregator::spec_get_limit($t7)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    assume $IsEqual'u128'($1_aggregator_spec_get_limit($Dereference($t5)), $1_aggregator_spec_get_limit($t7));

    // assume Eq<aggregator::Aggregator>($t5, aggregator::spec_aggregator_set_val($t7, Sub(aggregator::spec_aggregator_get_val($t7), $t1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    assume $IsEqual'$1_aggregator_Aggregator'($Dereference($t5), $1_aggregator_spec_aggregator_set_val($t7, ($1_aggregator_spec_aggregator_get_val($t7) - $t1)));

    // opaque end: aggregator::sub($t5, $t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34

    // write_back[Reference($t4).vec (vector<aggregator::Aggregator>)/[]]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    $t4 := $UpdateMutation($t4, (var $$sel0 := $vec#$1_option_Option'$1_aggregator_Aggregator'($Dereference($t4)); $Update'$1_option_Option'$1_aggregator_Aggregator''_vec($Dereference($t4), UpdateVec($$sel0, ReadVec(p#$Mutation($t5), LenVec(p#$Mutation($t4)) + 1), $Dereference($t5)))));

    // write_back[Reference($t0).aggregator (option::Option<aggregator::Aggregator>)]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    $t0 := $UpdateMutation($t0, $Update'$1_optional_aggregator_OptionalAggregator'_aggregator($Dereference($t0), $Dereference($t4)));

    // trace_local[optional_aggregator]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:171:13+34
    $temp_0'$1_optional_aggregator_OptionalAggregator' := $Dereference($t0);
    assume {:print "$track_local(22,12,0):", $temp_0'$1_optional_aggregator_OptionalAggregator'} $temp_0'$1_optional_aggregator_OptionalAggregator' == $temp_0'$1_optional_aggregator_OptionalAggregator';

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:9+337
    assume {:print "$at(77,6417,6754)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:173:51+19
    assume {:print "$at(77,6674,6693)"} true;
L0:

    // $t9 := borrow_field<optional_aggregator::OptionalAggregator>.integer($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:173:46+32
    assume {:print "$at(77,6669,6701)"} true;
    $t9 := $ChildMutation($t0, 1, $integer#$1_optional_aggregator_OptionalAggregator($Dereference($t0)));

    // $t10 := option::borrow_mut<optional_aggregator::Integer>($t9) on_abort goto L4 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:173:27+52
    call $t10,$t9 := $1_option_borrow_mut'$1_optional_aggregator_Integer'($t9);
    if ($abort_flag) {
        assume {:print "$at(77,6650,6702)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(22,12):", $t6} $t6 == $t6;
        goto L4;
    }

    // optional_aggregator::sub_integer($t10, $t1) on_abort goto L4 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:174:13+27
    assume {:print "$at(77,6716,6743)"} true;
    call $t10 := $1_optional_aggregator_sub_integer($t10, $t1);
    if ($abort_flag) {
        assume {:print "$at(77,6716,6743)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(22,12):", $t6} $t6 == $t6;
        goto L4;
    }

    // write_back[Reference($t9).vec (vector<optional_aggregator::Integer>)/[]]($t10) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:174:13+27
    $t9 := $UpdateMutation($t9, (var $$sel0 := $vec#$1_option_Option'$1_optional_aggregator_Integer'($Dereference($t9)); $Update'$1_option_Option'$1_optional_aggregator_Integer''_vec($Dereference($t9), UpdateVec($$sel0, ReadVec(p#$Mutation($t10), LenVec(p#$Mutation($t9)) + 1), $Dereference($t10)))));

    // write_back[Reference($t0).integer (option::Option<optional_aggregator::Integer>)]($t9) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:174:13+27
    $t0 := $UpdateMutation($t0, $Update'$1_optional_aggregator_OptionalAggregator'_integer($Dereference($t0), $Dereference($t9)));

    // trace_local[optional_aggregator]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:174:13+27
    $temp_0'$1_optional_aggregator_OptionalAggregator' := $Dereference($t0);
    assume {:print "$track_local(22,12,0):", $temp_0'$1_optional_aggregator_OptionalAggregator'} $temp_0'$1_optional_aggregator_OptionalAggregator' == $temp_0'$1_optional_aggregator_OptionalAggregator';

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:9+337
    assume {:print "$at(77,6417,6754)"} true;
L2:

    // trace_local[optional_aggregator]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:9+337
    assume {:print "$at(77,6417,6754)"} true;
    $temp_0'$1_optional_aggregator_OptionalAggregator' := $Dereference($t0);
    assume {:print "$track_local(22,12,0):", $temp_0'$1_optional_aggregator_OptionalAggregator'} $temp_0'$1_optional_aggregator_OptionalAggregator' == $temp_0'$1_optional_aggregator_OptionalAggregator';

    // pack_ref_deep($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:169:9+337

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:176:5+1
    assume {:print "$at(77,6759,6760)"} true;
L3:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:176:5+1
    assume {:print "$at(77,6759,6760)"} true;
    $ret0 := $t0;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:176:5+1
L4:

    // abort($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:176:5+1
    assume {:print "$at(77,6759,6760)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

    // label L7 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L7:

    // destroy($t0) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // destroy($t4) at <internal>:1:1+10

    // goto L6 at <internal>:1:1+10
    goto L6;

}

// fun optional_aggregator::sub_integer [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:43:5+191
procedure {:inline 1} $1_optional_aggregator_sub_integer(_$t0: $Mutation ($1_optional_aggregator_Integer), _$t1: int) returns ($ret0: $Mutation ($1_optional_aggregator_Integer))
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
    var $t0: $Mutation ($1_optional_aggregator_Integer);
    var $t1: int;
    var $temp_0'$1_optional_aggregator_Integer': $1_optional_aggregator_Integer;
    var $temp_0'u128': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[integer]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:43:5+1
    assume {:print "$at(77,1424,1425)"} true;
    $temp_0'$1_optional_aggregator_Integer' := $Dereference($t0);
    assume {:print "$track_local(22,13,0):", $temp_0'$1_optional_aggregator_Integer'} $temp_0'$1_optional_aggregator_Integer' == $temp_0'$1_optional_aggregator_Integer';

    // trace_local[value]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:43:5+1
    assume {:print "$track_local(22,13,1):", $t1} $t1 == $t1;

    // $t2 := get_field<optional_aggregator::Integer>.value($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:26+13
    assume {:print "$at(77,1503,1516)"} true;
    $t2 := $value#$1_optional_aggregator_Integer($Dereference($t0));

    // $t3 := <=($t1, $t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:23+2
    call $t3 := $Le($t1, $t2);

    // if ($t3) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
    if ($t3) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
    assume {:print "$at(77,1486,1561)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
L0:

    // destroy($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
    assume {:print "$at(77,1486,1561)"} true;

    // $t4 := 2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:61+21
    $t4 := 2;
    assume $IsValid'u64'($t4);

    // $t5 := error::out_of_range($t4) on_abort goto L4 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:41+42
    call $t5 := $1_error_out_of_range($t4);
    if ($abort_flag) {
        assume {:print "$at(77,1518,1560)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(22,13):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_abort($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
    assume {:print "$at(77,1486,1561)"} true;
    assume {:print "$track_abort(22,13):", $t5} $t5 == $t5;

    // $t6 := move($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
    $t6 := $t5;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:44:9+75
    goto L4;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:25+7
    assume {:print "$at(77,1587,1594)"} true;
L2:

    // $t7 := get_field<optional_aggregator::Integer>.value($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:25+13
    assume {:print "$at(77,1587,1600)"} true;
    $t7 := $value#$1_optional_aggregator_Integer($Dereference($t0));

    // $t8 := -($t7, $t1) on_abort goto L4 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:39+1
    call $t8 := $Sub($t7, $t1);
    if ($abort_flag) {
        assume {:print "$at(77,1601,1602)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(22,13):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t9 := borrow_field<optional_aggregator::Integer>.value($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:9+13
    $t9 := $ChildMutation($t0, 0, $value#$1_optional_aggregator_Integer($Dereference($t0)));

    // write_ref($t9, $t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:9+37
    $t9 := $UpdateMutation($t9, $t8);

    // write_back[Reference($t0).value (u128)]($t9) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:9+37
    $t0 := $UpdateMutation($t0, $Update'$1_optional_aggregator_Integer'_value($Dereference($t0), $Dereference($t9)));

    // trace_local[integer]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:9+37
    $temp_0'$1_optional_aggregator_Integer' := $Dereference($t0);
    assume {:print "$track_local(22,13,0):", $temp_0'$1_optional_aggregator_Integer'} $temp_0'$1_optional_aggregator_Integer' == $temp_0'$1_optional_aggregator_Integer';

    // trace_local[integer]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:45:46+1
    $temp_0'$1_optional_aggregator_Integer' := $Dereference($t0);
    assume {:print "$track_local(22,13,0):", $temp_0'$1_optional_aggregator_Integer'} $temp_0'$1_optional_aggregator_Integer' == $temp_0'$1_optional_aggregator_Integer';

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:46:5+1
    assume {:print "$at(77,1614,1615)"} true;
L3:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:46:5+1
    assume {:print "$at(77,1614,1615)"} true;
    $ret0 := $t0;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:46:5+1
L4:

    // abort($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aggregator/optional_aggregator.move:46:5+1
    assume {:print "$at(77,1614,1615)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// struct coin::AggregatableCoin<aptos_coin::AptosCoin> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:82:5+144
type {:datatype} $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
function {:constructor} $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($value: $1_aggregator_Aggregator): $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
function {:inline} $Update'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''_value(s: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin', x: $1_aggregator_Aggregator): $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin' {
    $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'(x)
}
function $IsValid'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''(s: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'): bool {
    $IsValid'$1_aggregator_Aggregator'($value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $IsEqual'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''(s1: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin', s2: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'): bool {
    s1 == s2
}

// struct coin::BurnCapability<aptos_coin::AptosCoin> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:139:5+58
type {:datatype} $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
function {:constructor} $1_coin_BurnCapability'$1_aptos_coin_AptosCoin'($dummy_field: bool): $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
function {:inline} $Update'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''_dummy_field(s: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin', x: bool): $1_coin_BurnCapability'$1_aptos_coin_AptosCoin' {
    $1_coin_BurnCapability'$1_aptos_coin_AptosCoin'(x)
}
function $IsValid'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''(s: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin'): bool {
    $IsValid'bool'($dummy_field#$1_coin_BurnCapability'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $IsEqual'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''(s1: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin', s2: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin'): bool {
    s1 == s2
}

// struct coin::Coin<aptos_coin::AptosCoin> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:74:5+112
type {:datatype} $1_coin_Coin'$1_aptos_coin_AptosCoin';
function {:constructor} $1_coin_Coin'$1_aptos_coin_AptosCoin'($value: int): $1_coin_Coin'$1_aptos_coin_AptosCoin';
function {:inline} $Update'$1_coin_Coin'$1_aptos_coin_AptosCoin''_value(s: $1_coin_Coin'$1_aptos_coin_AptosCoin', x: int): $1_coin_Coin'$1_aptos_coin_AptosCoin' {
    $1_coin_Coin'$1_aptos_coin_AptosCoin'(x)
}
function $IsValid'$1_coin_Coin'$1_aptos_coin_AptosCoin''(s: $1_coin_Coin'$1_aptos_coin_AptosCoin'): bool {
    $IsValid'u64'($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $IsEqual'$1_coin_Coin'$1_aptos_coin_AptosCoin''(s1: $1_coin_Coin'$1_aptos_coin_AptosCoin', s2: $1_coin_Coin'$1_aptos_coin_AptosCoin'): bool {
    s1 == s2
}

// struct coin::CoinInfo<aptos_coin::AptosCoin> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:109:5+564
type {:datatype} $1_coin_CoinInfo'$1_aptos_coin_AptosCoin';
function {:constructor} $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($name: $1_string_String, $symbol: $1_string_String, $decimals: int, $supply: $1_option_Option'$1_optional_aggregator_OptionalAggregator'): $1_coin_CoinInfo'$1_aptos_coin_AptosCoin';
function {:inline} $Update'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''_name(s: $1_coin_CoinInfo'$1_aptos_coin_AptosCoin', x: $1_string_String): $1_coin_CoinInfo'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(x, $symbol#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), $decimals#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $Update'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''_symbol(s: $1_coin_CoinInfo'$1_aptos_coin_AptosCoin', x: $1_string_String): $1_coin_CoinInfo'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($name#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), x, $decimals#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $Update'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''_decimals(s: $1_coin_CoinInfo'$1_aptos_coin_AptosCoin', x: int): $1_coin_CoinInfo'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($name#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), $symbol#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), x, $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $Update'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''_supply(s: $1_coin_CoinInfo'$1_aptos_coin_AptosCoin', x: $1_option_Option'$1_optional_aggregator_OptionalAggregator'): $1_coin_CoinInfo'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($name#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), $symbol#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), $decimals#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s), x)
}
function $IsValid'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''(s: $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'): bool {
    $IsValid'$1_string_String'($name#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s))
      && $IsValid'$1_string_String'($symbol#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s))
      && $IsValid'u8'($decimals#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s))
      && $IsValid'$1_option_Option'$1_optional_aggregator_OptionalAggregator''($supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $IsEqual'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''(s1: $1_coin_CoinInfo'$1_aptos_coin_AptosCoin', s2: $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'): bool {
    $IsEqual'$1_string_String'($name#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s1), $name#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s2))
    && $IsEqual'$1_string_String'($symbol#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s1), $symbol#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s2))
    && $IsEqual'u8'($decimals#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s1), $decimals#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s2))
    && $IsEqual'$1_option_Option'$1_optional_aggregator_OptionalAggregator''($supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s1), $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'(s2))}
var $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory: $Memory $1_coin_CoinInfo'$1_aptos_coin_AptosCoin';

// struct coin::CoinStore<aptos_coin::AptosCoin> at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:92:5+206
type {:datatype} $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
function {:constructor} $1_coin_CoinStore'$1_aptos_coin_AptosCoin'($coin: $1_coin_Coin'$1_aptos_coin_AptosCoin', $frozen: bool, $deposit_events: $1_event_EventHandle'$1_coin_DepositEvent', $withdraw_events: $1_event_EventHandle'$1_coin_WithdrawEvent'): $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
function {:inline} $Update'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''_coin(s: $1_coin_CoinStore'$1_aptos_coin_AptosCoin', x: $1_coin_Coin'$1_aptos_coin_AptosCoin'): $1_coin_CoinStore'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'(x, $frozen#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), $deposit_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), $withdraw_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $Update'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''_frozen(s: $1_coin_CoinStore'$1_aptos_coin_AptosCoin', x: bool): $1_coin_CoinStore'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), x, $deposit_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), $withdraw_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $Update'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''_deposit_events(s: $1_coin_CoinStore'$1_aptos_coin_AptosCoin', x: $1_event_EventHandle'$1_coin_DepositEvent'): $1_coin_CoinStore'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), $frozen#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), x, $withdraw_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $Update'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''_withdraw_events(s: $1_coin_CoinStore'$1_aptos_coin_AptosCoin', x: $1_event_EventHandle'$1_coin_WithdrawEvent'): $1_coin_CoinStore'$1_aptos_coin_AptosCoin' {
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), $frozen#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), $deposit_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s), x)
}
function $IsValid'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''(s: $1_coin_CoinStore'$1_aptos_coin_AptosCoin'): bool {
    $IsValid'$1_coin_Coin'$1_aptos_coin_AptosCoin''($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s))
      && $IsValid'bool'($frozen#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s))
      && $IsValid'$1_event_EventHandle'$1_coin_DepositEvent''($deposit_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s))
      && $IsValid'$1_event_EventHandle'$1_coin_WithdrawEvent''($withdraw_events#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'(s))
}
function {:inline} $IsEqual'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''(s1: $1_coin_CoinStore'$1_aptos_coin_AptosCoin', s2: $1_coin_CoinStore'$1_aptos_coin_AptosCoin'): bool {
    s1 == s2
}
var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory: $Memory $1_coin_CoinStore'$1_aptos_coin_AptosCoin';

// struct coin::DepositEvent at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:123:5+64
type {:datatype} $1_coin_DepositEvent;
function {:constructor} $1_coin_DepositEvent($amount: int): $1_coin_DepositEvent;
function {:inline} $Update'$1_coin_DepositEvent'_amount(s: $1_coin_DepositEvent, x: int): $1_coin_DepositEvent {
    $1_coin_DepositEvent(x)
}
function $IsValid'$1_coin_DepositEvent'(s: $1_coin_DepositEvent): bool {
    $IsValid'u64'($amount#$1_coin_DepositEvent(s))
}
function {:inline} $IsEqual'$1_coin_DepositEvent'(s1: $1_coin_DepositEvent, s2: $1_coin_DepositEvent): bool {
    s1 == s2
}

// struct coin::WithdrawEvent at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:128:5+65
type {:datatype} $1_coin_WithdrawEvent;
function {:constructor} $1_coin_WithdrawEvent($amount: int): $1_coin_WithdrawEvent;
function {:inline} $Update'$1_coin_WithdrawEvent'_amount(s: $1_coin_WithdrawEvent, x: int): $1_coin_WithdrawEvent {
    $1_coin_WithdrawEvent(x)
}
function $IsValid'$1_coin_WithdrawEvent'(s: $1_coin_WithdrawEvent): bool {
    $IsValid'u64'($amount#$1_coin_WithdrawEvent(s))
}
function {:inline} $IsEqual'$1_coin_WithdrawEvent'(s1: $1_coin_WithdrawEvent, s2: $1_coin_WithdrawEvent): bool {
    s1 == s2
}

// fun coin::extract<aptos_coin::AptosCoin> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:349:5+252
procedure {:inline 1} $1_coin_extract'$1_aptos_coin_AptosCoin'(_$t0: $Mutation ($1_coin_Coin'$1_aptos_coin_AptosCoin'), _$t1: int) returns ($ret0: $1_coin_Coin'$1_aptos_coin_AptosCoin', $ret1: $Mutation ($1_coin_Coin'$1_aptos_coin_AptosCoin'))
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
    var $t10: $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $t0: $Mutation ($1_coin_Coin'$1_aptos_coin_AptosCoin');
    var $t1: int;
    var $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'': $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[coin]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:349:5+1
    assume {:print "$at(93,13278,13279)"} true;
    $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'' := $Dereference($t0);
    assume {:print "$track_local(23,13,0):", $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'';

    // trace_local[amount]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:349:5+1
    assume {:print "$track_local(23,13,1):", $t1} $t1 == $t1;

    // $t2 := get_field<coin::Coin<#0>>.value($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:17+10
    assume {:print "$at(93,13381,13391)"} true;
    $t2 := $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($Dereference($t0));

    // $t3 := >=($t2, $t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:28+2
    call $t3 := $Ge($t2, $t1);

    // if ($t3) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
    if ($t3) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
    assume {:print "$at(93,13373,13450)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
L0:

    // destroy($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
    assume {:print "$at(93,13373,13450)"} true;

    // $t4 := 6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:63+21
    $t4 := 6;
    assume $IsValid'u64'($t4);

    // $t5 := error::invalid_argument($t4) on_abort goto L4 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:39+46
    call $t5 := $1_error_invalid_argument($t4);
    if ($abort_flag) {
        assume {:print "$at(93,13403,13449)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(23,13):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_abort($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
    assume {:print "$at(93,13373,13450)"} true;
    assume {:print "$track_abort(23,13):", $t5} $t5 == $t5;

    // $t6 := move($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
    $t6 := $t5;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:350:9+77
    goto L4;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:351:22+4
    assume {:print "$at(93,13473,13477)"} true;
L2:

    // $t7 := get_field<coin::Coin<#0>>.value($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:351:22+10
    assume {:print "$at(93,13473,13483)"} true;
    $t7 := $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($Dereference($t0));

    // $t8 := -($t7, $t1) on_abort goto L4 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:351:33+1
    call $t8 := $Sub($t7, $t1);
    if ($abort_flag) {
        assume {:print "$at(93,13484,13485)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(23,13):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t9 := borrow_field<coin::Coin<#0>>.value($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:351:9+10
    $t9 := $ChildMutation($t0, 0, $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($Dereference($t0)));

    // write_ref($t9, $t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:351:9+32
    $t9 := $UpdateMutation($t9, $t8);

    // write_back[Reference($t0).value (u64)]($t9) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:351:9+32
    $t0 := $UpdateMutation($t0, $Update'$1_coin_Coin'$1_aptos_coin_AptosCoin''_value($Dereference($t0), $Dereference($t9)));

    // trace_local[coin]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:351:9+32
    $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'' := $Dereference($t0);
    assume {:print "$track_local(23,13,0):", $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'';

    // $t10 := pack coin::Coin<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:352:9+22
    assume {:print "$at(93,13502,13524)"} true;
    $t10 := $1_coin_Coin'$1_aptos_coin_AptosCoin'($t1);

    // trace_return[0]($t10) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:352:9+22
    assume {:print "$track_return(23,13,0):", $t10} $t10 == $t10;

    // trace_local[coin]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:352:9+22
    $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'' := $Dereference($t0);
    assume {:print "$track_local(23,13,0):", $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'';

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:353:5+1
    assume {:print "$at(93,13529,13530)"} true;
L3:

    // return $t10 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:353:5+1
    assume {:print "$at(93,13529,13530)"} true;
    $ret0 := $t10;
    $ret1 := $t0;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:353:5+1
L4:

    // abort($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:353:5+1
    assume {:print "$at(93,13529,13530)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun coin::balance<aptos_coin::AptosCoin> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:228:5+274
procedure {:inline 1} $1_coin_balance'$1_aptos_coin_AptosCoin'(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $t6: $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $t7: int;
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[owner]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:228:5+1
    assume {:print "$at(93,8371,8372)"} true;
    assume {:print "$track_local(23,1,0):", $t0} $t0 == $t0;

    // $t1 := coin::is_account_registered<#0>($t0) on_abort goto L4 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:230:13+38
    assume {:print "$at(93,8471,8509)"} true;
    call $t1 := $1_coin_is_account_registered'$1_aptos_coin_AptosCoin'($t0);
    if ($abort_flag) {
        assume {:print "$at(93,8471,8509)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(23,1):", $t2} $t2 == $t2;
        goto L4;
    }

    // if ($t1) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:229:9+127
    assume {:print "$at(93,8450,8577)"} true;
    if ($t1) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:229:9+127
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:229:9+127
    assume {:print "$at(93,8450,8577)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:231:30+25
    assume {:print "$at(93,8540,8565)"} true;
L0:

    // $t3 := 5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:231:30+25
    assume {:print "$at(93,8540,8565)"} true;
    $t3 := 5;
    assume $IsValid'u64'($t3);

    // $t4 := error::not_found($t3) on_abort goto L4 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:231:13+43
    call $t4 := $1_error_not_found($t3);
    if ($abort_flag) {
        assume {:print "$at(93,8523,8566)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(23,1):", $t2} $t2 == $t2;
        goto L4;
    }

    // trace_abort($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:229:9+127
    assume {:print "$at(93,8450,8577)"} true;
    assume {:print "$track_abort(23,1):", $t4} $t4 == $t4;

    // $t2 := move($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:229:9+127
    $t2 := $t4;

    // goto L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:229:9+127
    goto L4;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:233:44+5
    assume {:print "$at(93,8622,8627)"} true;
L2:

    // $t5 := get_global<coin::CoinStore<#0>>($t0) on_abort goto L4 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:233:9+13
    assume {:print "$at(93,8587,8600)"} true;
    if (!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t5 := $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(93,8587,8600)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(23,1):", $t2} $t2 == $t2;
        goto L4;
    }

    // $t6 := get_field<coin::CoinStore<#0>>.coin($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:233:9+46
    $t6 := $coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($t5);

    // $t7 := get_field<coin::Coin<#0>>.value($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:233:9+52
    $t7 := $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($t6);

    // trace_return[0]($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:233:9+52
    assume {:print "$track_return(23,1,0):", $t7} $t7 == $t7;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:234:5+1
    assume {:print "$at(93,8644,8645)"} true;
L3:

    // return $t7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:234:5+1
    assume {:print "$at(93,8644,8645)"} true;
    $ret0 := $t7;
    return;

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:234:5+1
L4:

    // abort($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:234:5+1
    assume {:print "$at(93,8644,8645)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun coin::burn<aptos_coin::AptosCoin> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:285:5+528
procedure {:inline 1} $1_coin_burn'$1_aptos_coin_AptosCoin'(_$t0: $1_coin_Coin'$1_aptos_coin_AptosCoin', _$t1: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin') returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: $Mutation ($1_option_Option'$1_optional_aggregator_OptionalAggregator');
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t8: int;
    var $t9: int;
    var $t10: bool;
    var $t11: int;
    var $t12: int;
    var $t13: int;
    var $t14: int;
    var $t15: $Mutation ($1_coin_CoinInfo'$1_aptos_coin_AptosCoin');
    var $t16: $Mutation ($1_option_Option'$1_optional_aggregator_OptionalAggregator');
    var $t17: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t18: bool;
    var $t19: $Mutation ($1_optional_aggregator_OptionalAggregator);
    var $t20: int;
    var $t0: $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $t1: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
    var $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$modifies: [int]bool;
    var $temp_0'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin'': $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
    var $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'': $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'': $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t4, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:108:9+59
    assume {:print "$at(94,4082,4141)"} true;
    assume ($t4 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t5, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:74:9+58
    assume {:print "$at(94,2987,3045)"} true;
    assume ($t5 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t6, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:63:9+59
    assume {:print "$at(94,2316,2375)"} true;
    assume ($t6 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t7, select coin::CoinInfo.supply(global<coin::CoinInfo<#0>>($t6))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:64:9+59
    assume {:print "$at(94,2384,2443)"} true;
    assume ($t7 == $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t6)));

    // trace_local[coin]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:285:5+1
    assume {:print "$at(93,10603,10604)"} true;
    assume {:print "$track_local(23,2,0):", $t0} $t0 == $t0;

    // trace_local[_cap]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:285:5+1
    assume {:print "$track_local(23,2,1):", $t1} $t1 == $t1;

    // $t8 := unpack coin::Coin<#0>($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:289:13+22
    assume {:print "$at(93,10739,10761)"} true;
    $t8 := $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($t0);

    // trace_local[amount]($t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:289:27+6
    assume {:print "$track_local(23,2,2):", $t8} $t8 == $t8;

    // $t9 := 0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:26+1
    assume {:print "$at(93,10795,10796)"} true;
    $t9 := 0;
    assume $IsValid'u64'($t9);

    // $t10 := >($t8, $t9) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:24+1
    call $t10 := $Gt($t8, $t9);

    // if ($t10) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:9+63
    if ($t10) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:9+63
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:9+63
    assume {:print "$at(93,10778,10841)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:53+17
L0:

    // $t11 := 9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:53+17
    assume {:print "$at(93,10822,10839)"} true;
    $t11 := 9;
    assume $IsValid'u64'($t11);

    // $t12 := error::invalid_argument($t11) on_abort goto L7 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:29+42
    call $t12 := $1_error_invalid_argument($t11);
    if ($abort_flag) {
        assume {:print "$at(93,10798,10840)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(23,2):", $t13} $t13 == $t13;
        goto L7;
    }

    // trace_abort($t12) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:9+63
    assume {:print "$at(93,10778,10841)"} true;
    assume {:print "$track_abort(23,2):", $t12} $t12 == $t12;

    // $t13 := move($t12) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:9+63
    $t13 := $t12;

    // goto L7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:290:9+63
    goto L7;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:71+24
    assume {:print "$at(93,10914,10938)"} true;
L2:

    // $t14 := opaque begin: coin::coin_address<#0>() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:71+24
    assume {:print "$at(93,10914,10938)"} true;

    // assume WellFormed($t14) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:71+24
    assume $IsValid'address'($t14);

    // assume Eq<address>($t14, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:71+24
    assume $IsEqual'address'($t14, $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // $t14 := opaque end: coin::coin_address<#0>() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:71+24

    // $t15 := borrow_global<coin::CoinInfo<#0>>($t14) on_abort goto L7 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:33+17
    if (!$ResourceExists($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t14)) {
        call $ExecFailureAbort();
    } else {
        $t15 := $Mutation($Global($t14), EmptyVec(), $ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t14));
    }
    if ($abort_flag) {
        assume {:print "$at(93,10876,10893)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(23,2):", $t13} $t13 == $t13;
        goto L7;
    }

    // $t16 := borrow_field<coin::CoinInfo<#0>>.supply($t15) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:28+75
    $t16 := $ChildMutation($t15, 3, $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($Dereference($t15)));

    // trace_local[maybe_supply]($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:292:13+12
    $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' := $Dereference($t16);
    assume {:print "$track_local(23,2,3):", $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator''} $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'' == $temp_0'$1_option_Option'$1_optional_aggregator_OptionalAggregator'';

    // $t17 := read_ref($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:29+12
    assume {:print "$at(93,10976,10988)"} true;
    $t17 := $Dereference($t16);

    // $t18 := opaque begin: option::is_some<optional_aggregator::OptionalAggregator>($t17) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:13+29

    // assume WellFormed($t18) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:13+29
    assume $IsValid'bool'($t18);

    // assume Eq<bool>($t18, option::spec_is_some<optional_aggregator::OptionalAggregator>($t17)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:13+29
    assume $IsEqual'bool'($t18, $1_option_spec_is_some'$1_optional_aggregator_OptionalAggregator'($t17));

    // $t18 := opaque end: option::is_some<optional_aggregator::OptionalAggregator>($t17) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:13+29

    // if ($t18) goto L4 else goto L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:9+169
    if ($t18) { goto L4; } else { goto L3; }

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:294:45+12
    assume {:print "$at(93,11037,11049)"} true;
L4:

    // $t19 := option::borrow_mut<optional_aggregator::OptionalAggregator>($t16) on_abort goto L7 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:294:26+32
    assume {:print "$at(93,11018,11050)"} true;
    call $t19,$t16 := $1_option_borrow_mut'$1_optional_aggregator_OptionalAggregator'($t16);
    if ($abort_flag) {
        assume {:print "$at(93,11018,11050)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(23,2):", $t13} $t13 == $t13;
        goto L7;
    }

    // $t20 := (u128)($t8) on_abort goto L7 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:295:46+16
    assume {:print "$at(93,11097,11113)"} true;
    call $t20 := $CastU128($t8);
    if ($abort_flag) {
        assume {:print "$at(93,11097,11113)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(23,2):", $t13} $t13 == $t13;
        goto L7;
    }

    // optional_aggregator::sub($t19, $t20) on_abort goto L7 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:295:13+50
    call $t19 := $1_optional_aggregator_sub($t19, $t20);
    if ($abort_flag) {
        assume {:print "$at(93,11064,11114)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(23,2):", $t13} $t13 == $t13;
        goto L7;
    }

    // write_back[Reference($t16).vec (vector<optional_aggregator::OptionalAggregator>)/[]]($t19) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:295:13+50
    $t16 := $UpdateMutation($t16, (var $$sel0 := $vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'($Dereference($t16)); $Update'$1_option_Option'$1_optional_aggregator_OptionalAggregator''_vec($Dereference($t16), UpdateVec($$sel0, ReadVec(p#$Mutation($t19), LenVec(p#$Mutation($t16)) + 1), $Dereference($t19)))));

    // write_back[Reference($t15).supply (option::Option<optional_aggregator::OptionalAggregator>)]($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:295:13+50
    $t15 := $UpdateMutation($t15, $Update'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''_supply($Dereference($t15), $Dereference($t16)));

    // pack_ref_deep($t15) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:295:13+50

    // write_back[coin::CoinInfo<#0>@]($t15) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:295:13+50
    $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory := $ResourceUpdate($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $GlobalLocationAddress($t15),
        $Dereference($t15));

    // goto L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:9+169
    assume {:print "$at(93,10956,11125)"} true;
    goto L5;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:9+169
L3:

    // pack_ref_deep($t15) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:9+169
    assume {:print "$at(93,10956,11125)"} true;

    // destroy($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:9+169

    // label L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:293:9+169
L5:

    // label L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:297:5+1
    assume {:print "$at(93,11130,11131)"} true;
L6:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:297:5+1
    assume {:print "$at(93,11130,11131)"} true;
    return;

    // label L7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:297:5+1
L7:

    // abort($t13) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:297:5+1
    assume {:print "$at(93,11130,11131)"} true;
    $abort_code := $t13;
    $abort_flag := true;
    return;

}

// fun coin::burn_from<aptos_coin::AptosCoin> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:304:5+531
procedure {:inline 1} $1_coin_burn_from'$1_aptos_coin_AptosCoin'(_$t0: int, _$t1: int, _$t2: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin') returns ()
{
    // declare local variables
    var $t3: int;
    var $t4: $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $t5: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t6: $1_optional_aggregator_OptionalAggregator;
    var $t7: int;
    var $t8: int;
    var $t9: bool;
    var $t10: $Mutation ($1_coin_CoinStore'$1_aptos_coin_AptosCoin');
    var $t11: int;
    var $t12: $Mutation ($1_coin_Coin'$1_aptos_coin_AptosCoin');
    var $t13: $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $t14: int;
    var $t15: int;
    var $t16: int;
    var $t17: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t0: int;
    var $t1: int;
    var $t2: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
    var $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$modifies: [int]bool;
    var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$modifies: [int]bool;
    var $temp_0'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin'': $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // assume Identical($t3, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:121:9+58
    assume {:print "$at(94,4529,4587)"} true;
    assume ($t3 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t4, global<coin::CoinStore<#0>>($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:122:9+59
    assume {:print "$at(94,4596,4655)"} true;
    assume ($t4 == $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0));

    // assume Identical($t5, select coin::CoinInfo.supply(global<coin::CoinInfo<#0>>($t3))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:132:9+59
    assume {:print "$at(94,5051,5110)"} true;
    assume ($t5 == $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t3)));

    // assume Identical($t6, option::spec_borrow<optional_aggregator::OptionalAggregator>($t5)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:133:9+47
    assume {:print "$at(94,5119,5166)"} true;
    assume ($t6 == $1_option_spec_borrow'$1_optional_aggregator_OptionalAggregator'($t5));

    // assume Identical($t7, optional_aggregator::optional_aggregator_value($t6)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:134:9+67
    assume {:print "$at(94,5175,5242)"} true;
    assume ($t7 == $1_optional_aggregator_optional_aggregator_value($t6));

    // trace_local[account_addr]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:304:5+1
    assume {:print "$at(93,11492,11493)"} true;
    assume {:print "$track_local(23,3,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:304:5+1
    assume {:print "$track_local(23,3,1):", $t1} $t1 == $t1;

    // trace_local[burn_cap]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:304:5+1
    assume {:print "$track_local(23,3,2):", $t2} $t2 == $t2;

    // $t8 := 0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:310:23+1
    assume {:print "$at(93,11799,11800)"} true;
    $t8 := 0;
    assume $IsValid'u64'($t8);

    // $t9 := ==($t1, $t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:310:20+2
    $t9 := $IsEqual'u64'($t1, $t8);

    // if ($t9) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:310:9+47
    if ($t9) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:311:13+6
    assume {:print "$at(93,11816,11822)"} true;
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:311:13+6
    assume {:print "$at(93,11816,11822)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:314:65+12
    assume {:print "$at(93,11899,11911)"} true;
L0:

    // $t10 := borrow_global<coin::CoinStore<#0>>($t0) on_abort goto L3 with $t11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:314:26+17
    assume {:print "$at(93,11860,11877)"} true;
    if (!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t10 := $Mutation($Global($t0), EmptyVec(), $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0));
    }
    if ($abort_flag) {
        assume {:print "$at(93,11860,11877)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(23,3):", $t11} $t11 == $t11;
        goto L3;
    }

    // $t12 := borrow_field<coin::CoinStore<#0>>.coin($t10) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:315:36+20
    assume {:print "$at(93,11949,11969)"} true;
    $t12 := $ChildMutation($t10, 0, $coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($Dereference($t10)));

    // $t13 := coin::extract<#0>($t12, $t1) on_abort goto L3 with $t11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:315:28+37
    call $t13,$t12 := $1_coin_extract'$1_aptos_coin_AptosCoin'($t12, $t1);
    if ($abort_flag) {
        assume {:print "$at(93,11941,11978)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(23,3):", $t11} $t11 == $t11;
        goto L3;
    }

    // write_back[Reference($t10).coin (coin::Coin<#0>)]($t12) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:315:28+37
    $t10 := $UpdateMutation($t10, $Update'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''_coin($Dereference($t10), $Dereference($t12)));

    // write_back[coin::CoinStore<#0>@]($t10) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:315:28+37
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory := $ResourceUpdate($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $GlobalLocationAddress($t10),
        $Dereference($t10));

    // assume Identical($t14, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:108:9+59
    assume {:print "$at(94,4082,4141)"} true;
    assume ($t14 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t15, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:74:9+58
    assume {:print "$at(94,2987,3045)"} true;
    assume ($t15 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t16, select type_info::TypeInfo.account_address(type_info::$type_of<#0>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:63:9+59
    assume {:print "$at(94,2316,2375)"} true;
    assume ($t16 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t17, select coin::CoinInfo.supply(global<coin::CoinInfo<#0>>($t16))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:64:9+59
    assume {:print "$at(94,2384,2443)"} true;
    assume ($t17 == $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t16)));

    // coin::burn<#0>($t13, $t2) on_abort goto L3 with $t11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:316:9+28
    assume {:print "$at(93,11988,12016)"} true;
    call $1_coin_burn'$1_aptos_coin_AptosCoin'($t13, $t2);
    if ($abort_flag) {
        assume {:print "$at(93,11988,12016)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(23,3):", $t11} $t11 == $t11;
        goto L3;
    }

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:317:5+1
    assume {:print "$at(93,12022,12023)"} true;
L2:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:317:5+1
    assume {:print "$at(93,12022,12023)"} true;
    return;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:317:5+1
L3:

    // abort($t11) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:317:5+1
    assume {:print "$at(93,12022,12023)"} true;
    $abort_code := $t11;
    $abort_flag := true;
    return;

}

// fun coin::collect_into_aggregatable_coin<aptos_coin::AptosCoin> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:201:5+486
procedure {:inline 1} $1_coin_collect_into_aggregatable_coin'$1_aptos_coin_AptosCoin'(_$t0: int, _$t1: int, _$t2: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin')) returns ($ret0: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'))
{
    // declare local variables
    var $t3: $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $t4: $1_aggregator_Aggregator;
    var $t5: $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $t6: int;
    var $t7: bool;
    var $t8: $Mutation ($1_coin_CoinStore'$1_aptos_coin_AptosCoin');
    var $t9: int;
    var $t10: $Mutation ($1_coin_Coin'$1_aptos_coin_AptosCoin');
    var $t11: $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $t12: $1_aggregator_Aggregator;
    var $t0: int;
    var $t1: int;
    var $t2: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin');
    var $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'': $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
    var $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'': $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // assume Identical($t4, select coin::AggregatableCoin.value($t2)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:395:9+26
    assume {:print "$at(94,16731,16757)"} true;
    assume ($t4 == $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($Dereference($t2)));

    // assume Identical($t5, global<coin::CoinStore<#0>>($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:396:9+59
    assume {:print "$at(94,16766,16825)"} true;
    assume ($t5 == $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0));

    // trace_local[account_addr]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:201:5+1
    assume {:print "$at(93,7546,7547)"} true;
    assume {:print "$track_local(23,5,0):", $t0} $t0 == $t0;

    // trace_local[amount]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:201:5+1
    assume {:print "$track_local(23,5,1):", $t1} $t1 == $t1;

    // trace_local[dst_coin]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:201:5+1
    $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' := $Dereference($t2);
    assume {:print "$track_local(23,5,2):", $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'';

    // $t6 := 0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:207:23+1
    assume {:print "$at(93,7805,7806)"} true;
    $t6 := 0;
    assume $IsValid'u64'($t6);

    // $t7 := ==($t1, $t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:207:20+2
    $t7 := $IsEqual'u64'($t1, $t6);

    // if ($t7) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:207:9+47
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:208:13+6
    assume {:print "$at(93,7822,7828)"} true;
L1:

    // destroy($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:208:13+6
    assume {:print "$at(93,7822,7828)"} true;

    // trace_local[dst_coin]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:208:13+6
    $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' := $Dereference($t2);
    assume {:print "$track_local(23,5,2):", $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'';

    // pack_ref_deep($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:208:13+6

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:208:13+6
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:211:65+12
    assume {:print "$at(93,7905,7917)"} true;
L0:

    // $t8 := borrow_global<coin::CoinStore<#0>>($t0) on_abort goto L3 with $t9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:211:26+17
    assume {:print "$at(93,7866,7883)"} true;
    if (!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t8 := $Mutation($Global($t0), EmptyVec(), $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0));
    }
    if ($abort_flag) {
        assume {:print "$at(93,7866,7883)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(23,5):", $t9} $t9 == $t9;
        goto L3;
    }

    // $t10 := borrow_field<coin::CoinStore<#0>>.coin($t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:212:28+20
    assume {:print "$at(93,7947,7967)"} true;
    $t10 := $ChildMutation($t8, 0, $coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($Dereference($t8)));

    // $t11 := coin::extract<#0>($t10, $t1) on_abort goto L3 with $t9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:212:20+37
    call $t11,$t10 := $1_coin_extract'$1_aptos_coin_AptosCoin'($t10, $t1);
    if ($abort_flag) {
        assume {:print "$at(93,7939,7976)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(23,5):", $t9} $t9 == $t9;
        goto L3;
    }

    // write_back[Reference($t8).coin (coin::Coin<#0>)]($t10) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:212:20+37
    $t8 := $UpdateMutation($t8, $Update'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''_coin($Dereference($t8), $Dereference($t10)));

    // write_back[coin::CoinStore<#0>@]($t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:212:20+37
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory := $ResourceUpdate($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $GlobalLocationAddress($t8),
        $Dereference($t8));

    // trace_local[coin]($t11) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:212:13+4
    assume {:print "$track_local(23,5,3):", $t11} $t11 == $t11;

    // assume Identical($t12, select coin::AggregatableCoin.value($t2)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:387:9+26
    assume {:print "$at(94,16340,16366)"} true;
    assume ($t12 == $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($Dereference($t2)));

    // coin::merge_aggregatable_coin<#0>($t2, $t11) on_abort goto L3 with $t9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:213:9+39
    assume {:print "$at(93,7986,8025)"} true;
    call $t2 := $1_coin_merge_aggregatable_coin'$1_aptos_coin_AptosCoin'($t2, $t11);
    if ($abort_flag) {
        assume {:print "$at(93,7986,8025)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(23,5):", $t9} $t9 == $t9;
        goto L3;
    }

    // trace_local[dst_coin]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:213:48+1
    $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' := $Dereference($t2);
    assume {:print "$track_local(23,5,2):", $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'';

    // pack_ref_deep($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:213:48+1

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:214:5+1
    assume {:print "$at(93,8031,8032)"} true;
L2:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:214:5+1
    assume {:print "$at(93,8031,8032)"} true;
    $ret0 := $t2;
    return;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:214:5+1
L3:

    // abort($t9) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:214:5+1
    assume {:print "$at(93,8031,8032)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

}

// fun coin::is_account_registered<aptos_coin::AptosCoin> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:244:5+129
procedure {:inline 1} $1_coin_is_account_registered'$1_aptos_coin_AptosCoin'(_$t0: int) returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[account_addr]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:244:5+1
    assume {:print "$at(93,8946,8947)"} true;
    assume {:print "$track_local(23,21,0):", $t0} $t0 == $t0;

    // $t1 := exists<coin::CoinStore<#0>>($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:245:9+6
    assume {:print "$at(93,9028,9034)"} true;
    $t1 := $ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0);

    // trace_return[0]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:245:9+41
    assume {:print "$track_return(23,21,0):", $t1} $t1 == $t1;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:246:5+1
    assume {:print "$at(93,9074,9075)"} true;
L1:

    // return $t1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:246:5+1
    assume {:print "$at(93,9074,9075)"} true;
    $ret0 := $t1;
    return;

}

// fun coin::merge_aggregatable_coin<aptos_coin::AptosCoin> [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:194:5+252
procedure {:inline 1} $1_coin_merge_aggregatable_coin'$1_aptos_coin_AptosCoin'(_$t0: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'), _$t1: $1_coin_Coin'$1_aptos_coin_AptosCoin') returns ($ret0: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'))
{
    // declare local variables
    var $t2: int;
    var $t3: $1_aggregator_Aggregator;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $Mutation ($1_aggregator_Aggregator);
    var $t8: $1_aggregator_Aggregator;
    var $t9: bool;
    var $t0: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin');
    var $t1: $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $temp_0'$1_aggregator_Aggregator': $1_aggregator_Aggregator;
    var $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'': $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
    var $temp_0'$1_coin_Coin'$1_aptos_coin_AptosCoin'': $1_coin_Coin'$1_aptos_coin_AptosCoin';
    var $temp_0'u128': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t3, select coin::AggregatableCoin.value($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:387:9+26
    assume {:print "$at(94,16340,16366)"} true;
    assume ($t3 == $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($Dereference($t0)));

    // trace_local[dst_coin]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:194:5+1
    assume {:print "$at(93,7204,7205)"} true;
    $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' := $Dereference($t0);
    assume {:print "$track_local(23,25,0):", $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'';

    // trace_local[coin]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:194:5+1
    assume {:print "$track_local(23,25,1):", $t1} $t1 == $t1;

    // $t4 := unpack coin::Coin<#0>($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:195:13+14
    assume {:print "$at(93,7336,7350)"} true;
    $t4 := $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($t1);

    // $t5 := (u128)($t4) on_abort goto L2 with $t6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:196:22+15
    assume {:print "$at(93,7380,7395)"} true;
    call $t5 := $CastU128($t4);
    if ($abort_flag) {
        assume {:print "$at(93,7380,7395)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(23,25):", $t6} $t6 == $t6;
        goto L2;
    }

    // trace_local[amount]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:196:13+6
    assume {:print "$track_local(23,25,2):", $t5} $t5 == $t5;

    // $t7 := borrow_field<coin::AggregatableCoin<#0>>.value($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:25+19
    assume {:print "$at(93,7421,7440)"} true;
    $t7 := $ChildMutation($t0, 0, $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($Dereference($t0)));

    // opaque begin: aggregator::add($t7, $t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44

    // $t8 := read_ref($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    $t8 := $Dereference($t7);

    // assume Identical($t9, Or(Gt(Add(aggregator::spec_aggregator_get_val($t7), $t5), aggregator::spec_get_limit($t7)), Gt(Add(aggregator::spec_aggregator_get_val($t7), $t5), 340282366920938463463374607431768211455))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    assume ($t9 == ((($1_aggregator_spec_aggregator_get_val($Dereference($t7)) + $t5) > $1_aggregator_spec_get_limit($Dereference($t7))) || (($1_aggregator_spec_aggregator_get_val($Dereference($t7)) + $t5) > 340282366920938463463374607431768211455)));

    // if ($t9) goto L5 else goto L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    if ($t9) { goto L5; } else { goto L3; }

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
L4:

    // trace_abort($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    assume {:print "$at(93,7405,7449)"} true;
    assume {:print "$track_abort(23,25):", $t6} $t6 == $t6;

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    goto L2;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
L3:

    // $t7 := havoc[mut]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    assume {:print "$at(93,7405,7449)"} true;
    havoc $temp_0'$1_aggregator_Aggregator';
    $t7 := $UpdateMutation($t7, $temp_0'$1_aggregator_Aggregator');

    // assume WellFormed($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    assume $IsValid'$1_aggregator_Aggregator'($Dereference($t7));

    // assume Eq<u128>(aggregator::spec_get_limit($t7), aggregator::spec_get_limit($t8)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    assume $IsEqual'u128'($1_aggregator_spec_get_limit($Dereference($t7)), $1_aggregator_spec_get_limit($t8));

    // assume Eq<aggregator::Aggregator>($t7, aggregator::spec_aggregator_set_val($t8, Add(aggregator::spec_aggregator_get_val($t8), $t5))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    assume $IsEqual'$1_aggregator_Aggregator'($Dereference($t7), $1_aggregator_spec_aggregator_set_val($t8, ($1_aggregator_spec_aggregator_get_val($t8) + $t5)));

    // opaque end: aggregator::add($t7, $t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44

    // write_back[Reference($t0).value (aggregator::Aggregator)]($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    $t0 := $UpdateMutation($t0, $Update'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''_value($Dereference($t0), $Dereference($t7)));

    // trace_local[dst_coin]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:9+44
    $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' := $Dereference($t0);
    assume {:print "$track_local(23,25,0):", $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'';

    // trace_local[dst_coin]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:53+1
    $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' := $Dereference($t0);
    assume {:print "$track_local(23,25,0):", $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'';

    // pack_ref_deep($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:197:53+1

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:198:5+1
    assume {:print "$at(93,7455,7456)"} true;
L1:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:198:5+1
    assume {:print "$at(93,7455,7456)"} true;
    $ret0 := $t0;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:198:5+1
L2:

    // abort($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.move:198:5+1
    assume {:print "$at(93,7455,7456)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

    // label L5 at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;
L5:

    // destroy($t0) at <internal>:1:1+10
    assume {:print "$at(1,0,10)"} true;

    // goto L4 at <internal>:1:1+10
    goto L4;

}

// struct aptos_coin::AptosCoin at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/aptos_coin.move:22:5+27
type {:datatype} $1_aptos_coin_AptosCoin;
function {:constructor} $1_aptos_coin_AptosCoin($dummy_field: bool): $1_aptos_coin_AptosCoin;
function {:inline} $Update'$1_aptos_coin_AptosCoin'_dummy_field(s: $1_aptos_coin_AptosCoin, x: bool): $1_aptos_coin_AptosCoin {
    $1_aptos_coin_AptosCoin(x)
}
function $IsValid'$1_aptos_coin_AptosCoin'(s: $1_aptos_coin_AptosCoin): bool {
    $IsValid'bool'($dummy_field#$1_aptos_coin_AptosCoin(s))
}
function {:inline} $IsEqual'$1_aptos_coin_AptosCoin'(s1: $1_aptos_coin_AptosCoin, s2: $1_aptos_coin_AptosCoin): bool {
    s1 == s2
}
var $1_aptos_coin_AptosCoin_$memory: $Memory $1_aptos_coin_AptosCoin;

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_status.move:35:5+90
function {:inline} $1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory: $Memory $1_chain_status_GenesisEndMarker): bool {
    $ResourceExists($1_chain_status_GenesisEndMarker_$memory, 1)
}

// struct chain_status::GenesisEndMarker at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/chain_status.move:12:5+34
type {:datatype} $1_chain_status_GenesisEndMarker;
function {:constructor} $1_chain_status_GenesisEndMarker($dummy_field: bool): $1_chain_status_GenesisEndMarker;
function {:inline} $Update'$1_chain_status_GenesisEndMarker'_dummy_field(s: $1_chain_status_GenesisEndMarker, x: bool): $1_chain_status_GenesisEndMarker {
    $1_chain_status_GenesisEndMarker(x)
}
function $IsValid'$1_chain_status_GenesisEndMarker'(s: $1_chain_status_GenesisEndMarker): bool {
    $IsValid'bool'($dummy_field#$1_chain_status_GenesisEndMarker(s))
}
function {:inline} $IsEqual'$1_chain_status_GenesisEndMarker'(s1: $1_chain_status_GenesisEndMarker, s2: $1_chain_status_GenesisEndMarker): bool {
    s1 == s2
}
var $1_chain_status_GenesisEndMarker_$memory: $Memory $1_chain_status_GenesisEndMarker;

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:61:5+153
function {:inline} $1_timestamp_$now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds): int {
    $microseconds#$1_timestamp_CurrentTimeMicroseconds($ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, 1))
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:67:5+123
function {:inline} $1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds): int {
    ($1_timestamp_$now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory) div 1000000)
}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.spec.move:22:10+111
function {:inline} $1_timestamp_spec_now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds): int {
    $microseconds#$1_timestamp_CurrentTimeMicroseconds($ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, 1))
}

// struct timestamp::CurrentTimeMicroseconds at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:12:5+73
type {:datatype} $1_timestamp_CurrentTimeMicroseconds;
function {:constructor} $1_timestamp_CurrentTimeMicroseconds($microseconds: int): $1_timestamp_CurrentTimeMicroseconds;
function {:inline} $Update'$1_timestamp_CurrentTimeMicroseconds'_microseconds(s: $1_timestamp_CurrentTimeMicroseconds, x: int): $1_timestamp_CurrentTimeMicroseconds {
    $1_timestamp_CurrentTimeMicroseconds(x)
}
function $IsValid'$1_timestamp_CurrentTimeMicroseconds'(s: $1_timestamp_CurrentTimeMicroseconds): bool {
    $IsValid'u64'($microseconds#$1_timestamp_CurrentTimeMicroseconds(s))
}
function {:inline} $IsEqual'$1_timestamp_CurrentTimeMicroseconds'(s1: $1_timestamp_CurrentTimeMicroseconds, s2: $1_timestamp_CurrentTimeMicroseconds): bool {
    s1 == s2
}
var $1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds;

// fun timestamp::now_microseconds [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:61:5+153
procedure {:inline 1} $1_timestamp_now_microseconds() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: $1_timestamp_CurrentTimeMicroseconds;
    var $t2: int;
    var $t3: int;
    var $temp_0'u64': int;

    // bytecode translation starts here
    // $t0 := 0x1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:48+16
    assume {:print "$at(142,2511,2527)"} true;
    $t0 := 1;
    assume $IsValid'address'($t0);

    // $t1 := get_global<timestamp::CurrentTimeMicroseconds>($t0) on_abort goto L2 with $t2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:9+13
    if (!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t1 := $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(142,2472,2485)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(28,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_field<timestamp::CurrentTimeMicroseconds>.microseconds($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:9+69
    $t3 := $microseconds#$1_timestamp_CurrentTimeMicroseconds($t1);

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:9+69
    assume {:print "$track_return(28,0,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
    assume {:print "$at(142,2546,2547)"} true;
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
    assume {:print "$at(142,2546,2547)"} true;
    $ret0 := $t3;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
L2:

    // abort($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
    assume {:print "$at(142,2546,2547)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun timestamp::now_seconds [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:67:5+123
procedure {:inline 1} $1_timestamp_now_seconds() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $temp_0'u64': int;

    // bytecode translation starts here
    // $t0 := timestamp::now_microseconds() on_abort goto L2 with $t1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:9+18
    assume {:print "$at(142,2680,2698)"} true;
    call $t0 := $1_timestamp_now_microseconds();
    if ($abort_flag) {
        assume {:print "$at(142,2680,2698)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(28,1):", $t1} $t1 == $t1;
        goto L2;
    }

    // $t2 := 1000000 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:30+23
    $t2 := 1000000;
    assume $IsValid'u64'($t2);

    // $t3 := /($t0, $t2) on_abort goto L2 with $t1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:28+1
    call $t3 := $Div($t0, $t2);
    if ($abort_flag) {
        assume {:print "$at(142,2699,2700)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(28,1):", $t1} $t1 == $t1;
        goto L2;
    }

    // trace_return[0]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:9+44
    assume {:print "$track_return(28,1,0):", $t3} $t3 == $t3;

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
    assume {:print "$at(142,2729,2730)"} true;
L1:

    // return $t3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
    assume {:print "$at(142,2729,2730)"} true;
    $ret0 := $t3;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
L2:

    // abort($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
    assume {:print "$at(142,2729,2730)"} true;
    $abort_code := $t1;
    $abort_flag := true;
    return;

}

// struct transaction_fee::AptosCoinCapabilities at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:23:5+89
type {:datatype} $1_transaction_fee_AptosCoinCapabilities;
function {:constructor} $1_transaction_fee_AptosCoinCapabilities($burn_cap: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin'): $1_transaction_fee_AptosCoinCapabilities;
function {:inline} $Update'$1_transaction_fee_AptosCoinCapabilities'_burn_cap(s: $1_transaction_fee_AptosCoinCapabilities, x: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin'): $1_transaction_fee_AptosCoinCapabilities {
    $1_transaction_fee_AptosCoinCapabilities(x)
}
function $IsValid'$1_transaction_fee_AptosCoinCapabilities'(s: $1_transaction_fee_AptosCoinCapabilities): bool {
    $IsValid'$1_coin_BurnCapability'$1_aptos_coin_AptosCoin''($burn_cap#$1_transaction_fee_AptosCoinCapabilities(s))
}
function {:inline} $IsEqual'$1_transaction_fee_AptosCoinCapabilities'(s1: $1_transaction_fee_AptosCoinCapabilities, s2: $1_transaction_fee_AptosCoinCapabilities): bool {
    s1 == s2
}
var $1_transaction_fee_AptosCoinCapabilities_$memory: $Memory $1_transaction_fee_AptosCoinCapabilities;

// struct transaction_fee::CollectedFeesPerBlock at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:29:5+153
type {:datatype} $1_transaction_fee_CollectedFeesPerBlock;
function {:constructor} $1_transaction_fee_CollectedFeesPerBlock($amount: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin', $proposer: $1_option_Option'address', $burn_percentage: int): $1_transaction_fee_CollectedFeesPerBlock;
function {:inline} $Update'$1_transaction_fee_CollectedFeesPerBlock'_amount(s: $1_transaction_fee_CollectedFeesPerBlock, x: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'): $1_transaction_fee_CollectedFeesPerBlock {
    $1_transaction_fee_CollectedFeesPerBlock(x, $proposer#$1_transaction_fee_CollectedFeesPerBlock(s), $burn_percentage#$1_transaction_fee_CollectedFeesPerBlock(s))
}
function {:inline} $Update'$1_transaction_fee_CollectedFeesPerBlock'_proposer(s: $1_transaction_fee_CollectedFeesPerBlock, x: $1_option_Option'address'): $1_transaction_fee_CollectedFeesPerBlock {
    $1_transaction_fee_CollectedFeesPerBlock($amount#$1_transaction_fee_CollectedFeesPerBlock(s), x, $burn_percentage#$1_transaction_fee_CollectedFeesPerBlock(s))
}
function {:inline} $Update'$1_transaction_fee_CollectedFeesPerBlock'_burn_percentage(s: $1_transaction_fee_CollectedFeesPerBlock, x: int): $1_transaction_fee_CollectedFeesPerBlock {
    $1_transaction_fee_CollectedFeesPerBlock($amount#$1_transaction_fee_CollectedFeesPerBlock(s), $proposer#$1_transaction_fee_CollectedFeesPerBlock(s), x)
}
function $IsValid'$1_transaction_fee_CollectedFeesPerBlock'(s: $1_transaction_fee_CollectedFeesPerBlock): bool {
    $IsValid'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''($amount#$1_transaction_fee_CollectedFeesPerBlock(s))
      && $IsValid'$1_option_Option'address''($proposer#$1_transaction_fee_CollectedFeesPerBlock(s))
      && $IsValid'u8'($burn_percentage#$1_transaction_fee_CollectedFeesPerBlock(s))
}
function {:inline} $IsEqual'$1_transaction_fee_CollectedFeesPerBlock'(s1: $1_transaction_fee_CollectedFeesPerBlock, s2: $1_transaction_fee_CollectedFeesPerBlock): bool {
    $IsEqual'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''($amount#$1_transaction_fee_CollectedFeesPerBlock(s1), $amount#$1_transaction_fee_CollectedFeesPerBlock(s2))
    && $IsEqual'$1_option_Option'address''($proposer#$1_transaction_fee_CollectedFeesPerBlock(s1), $proposer#$1_transaction_fee_CollectedFeesPerBlock(s2))
    && $IsEqual'u8'($burn_percentage#$1_transaction_fee_CollectedFeesPerBlock(s1), $burn_percentage#$1_transaction_fee_CollectedFeesPerBlock(s2))}
var $1_transaction_fee_CollectedFeesPerBlock_$memory: $Memory $1_transaction_fee_CollectedFeesPerBlock;

// fun transaction_fee::burn_fee [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:158:5+257
procedure {:inline 1} $1_transaction_fee_burn_fee(_$t0: int, _$t1: int) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $t6: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t7: $1_optional_aggregator_OptionalAggregator;
    var $t8: int;
    var $t9: int;
    var $t10: $1_transaction_fee_AptosCoinCapabilities;
    var $t11: int;
    var $t12: $1_coin_BurnCapability'$1_aptos_coin_AptosCoin';
    var $t13: int;
    var $t14: $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $t15: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t16: $1_optional_aggregator_OptionalAggregator;
    var $t17: int;
    var $t0: int;
    var $t1: int;
    var $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$modifies: [int]bool;
    var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$modifies: [int]bool;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t2, $t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:78:9+27
    assume {:print "$at(147,3636,3663)"} true;
    assume ($t2 == $t0);

    // assume Identical($t3, $t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:79:9+17
    assume {:print "$at(147,3672,3689)"} true;
    assume ($t3 == $t1);

    // assume Identical($t4, select type_info::TypeInfo.account_address(type_info::$type_of<aptos_coin::AptosCoin>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:81:9+65
    assume {:print "$at(147,3699,3764)"} true;
    assume ($t4 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t5, global<coin::CoinStore<aptos_coin::AptosCoin>>($t2)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:82:9+60
    assume {:print "$at(147,3773,3833)"} true;
    assume ($t5 == $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t2));

    // assume Identical($t6, select coin::CoinInfo.supply(global<coin::CoinInfo<aptos_coin::AptosCoin>>($t4))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:92:9+66
    assume {:print "$at(147,4229,4295)"} true;
    assume ($t6 == $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t4)));

    // assume Identical($t7, option::spec_borrow<optional_aggregator::OptionalAggregator>($t6)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:93:9+47
    assume {:print "$at(147,4304,4351)"} true;
    assume ($t7 == $1_option_spec_borrow'$1_optional_aggregator_OptionalAggregator'($t6));

    // assume Identical($t8, optional_aggregator::optional_aggregator_value($t7)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:94:9+67
    assume {:print "$at(147,4360,4427)"} true;
    assume ($t8 == $1_optional_aggregator_optional_aggregator_value($t7));

    // trace_local[account]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:158:5+1
    assume {:print "$at(146,7211,7212)"} true;
    assume {:print "$track_local(39,1,0):", $t0} $t0 == $t0;

    // trace_local[fee]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:158:5+1
    assume {:print "$track_local(39,1,1):", $t1} $t1 == $t1;

    // $t9 := 0x1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:162:51+16
    assume {:print "$at(146,7424,7440)"} true;
    $t9 := 1;
    assume $IsValid'address'($t9);

    // $t10 := get_global<transaction_fee::AptosCoinCapabilities>($t9) on_abort goto L2 with $t11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:162:14+13
    if (!$ResourceExists($1_transaction_fee_AptosCoinCapabilities_$memory, $t9)) {
        call $ExecFailureAbort();
    } else {
        $t10 := $ResourceValue($1_transaction_fee_AptosCoinCapabilities_$memory, $t9);
    }
    if ($abort_flag) {
        assume {:print "$at(146,7387,7400)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(39,1):", $t11} $t11 == $t11;
        goto L2;
    }

    // $t12 := get_field<transaction_fee::AptosCoinCapabilities>.burn_cap($t10) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:162:13+64
    $t12 := $burn_cap#$1_transaction_fee_AptosCoinCapabilities($t10);

    // assume Identical($t13, select type_info::TypeInfo.account_address(type_info::$type_of<aptos_coin::AptosCoin>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:121:9+58
    assume {:print "$at(94,4529,4587)"} true;
    assume ($t13 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t14, global<coin::CoinStore<aptos_coin::AptosCoin>>($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:122:9+59
    assume {:print "$at(94,4596,4655)"} true;
    assume ($t14 == $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0));

    // assume Identical($t15, select coin::CoinInfo.supply(global<coin::CoinInfo<aptos_coin::AptosCoin>>($t13))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:132:9+59
    assume {:print "$at(94,5051,5110)"} true;
    assume ($t15 == $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t13)));

    // assume Identical($t16, option::spec_borrow<optional_aggregator::OptionalAggregator>($t15)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:133:9+47
    assume {:print "$at(94,5119,5166)"} true;
    assume ($t16 == $1_option_spec_borrow'$1_optional_aggregator_OptionalAggregator'($t15));

    // assume Identical($t17, optional_aggregator::optional_aggregator_value($t16)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:134:9+67
    assume {:print "$at(94,5175,5242)"} true;
    assume ($t17 == $1_optional_aggregator_optional_aggregator_value($t16));

    // coin::burn_from<aptos_coin::AptosCoin>($t0, $t1, $t12) on_abort goto L2 with $t11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:159:9+153
    assume {:print "$at(146,7308,7461)"} true;
    call $1_coin_burn_from'$1_aptos_coin_AptosCoin'($t0, $t1, $t12);
    if ($abort_flag) {
        assume {:print "$at(146,7308,7461)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(39,1):", $t11} $t11 == $t11;
        goto L2;
    }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:164:5+1
    assume {:print "$at(146,7467,7468)"} true;
L1:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:164:5+1
    assume {:print "$at(146,7467,7468)"} true;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:164:5+1
L2:

    // abort($t11) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:164:5+1
    assume {:print "$at(146,7467,7468)"} true;
    $abort_code := $t11;
    $abort_flag := true;
    return;

}

// fun transaction_fee::collect_fee [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:167:5+635
procedure {:inline 1} $1_transaction_fee_collect_fee(_$t0: int, _$t1: int) returns ()
{
    // declare local variables
    var $t2: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin');
    var $t3: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
    var $t4: $1_aggregator_Aggregator;
    var $t5: int;
    var $t6: $Mutation ($1_transaction_fee_CollectedFeesPerBlock);
    var $t7: int;
    var $t8: $Mutation ($1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin');
    var $t9: $1_aggregator_Aggregator;
    var $t10: $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'': $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t3, select transaction_fee::CollectedFeesPerBlock.amount(global<transaction_fee::CollectedFeesPerBlock>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:112:9+76
    assume {:print "$at(147,5114,5190)"} true;
    assume ($t3 == $amount#$1_transaction_fee_CollectedFeesPerBlock($ResourceValue($1_transaction_fee_CollectedFeesPerBlock_$memory, 1)));

    // assume Identical($t4, select coin::AggregatableCoin.value($t3)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:113:9+32
    assume {:print "$at(147,5199,5231)"} true;
    assume ($t4 == $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($t3));

    // trace_local[account]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:167:5+1
    assume {:print "$at(146,7520,7521)"} true;
    assume {:print "$track_local(39,2,0):", $t0} $t0 == $t0;

    // trace_local[fee]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:167:5+1
    assume {:print "$track_local(39,2,1):", $t1} $t1 == $t1;

    // $t5 := 0x1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:168:71+16
    assume {:print "$at(146,7682,7698)"} true;
    $t5 := 1;
    assume $IsValid'address'($t5);

    // $t6 := borrow_global<transaction_fee::CollectedFeesPerBlock>($t5) on_abort goto L2 with $t7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:168:30+17
    if (!$ResourceExists($1_transaction_fee_CollectedFeesPerBlock_$memory, $t5)) {
        call $ExecFailureAbort();
    } else {
        $t6 := $Mutation($Global($t5), EmptyVec(), $ResourceValue($1_transaction_fee_CollectedFeesPerBlock_$memory, $t5));
    }
    if ($abort_flag) {
        assume {:print "$at(146,7641,7658)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(39,2):", $t7} $t7 == $t7;
        goto L2;
    }

    // $t8 := borrow_field<transaction_fee::CollectedFeesPerBlock>.amount($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:173:32+26
    assume {:print "$at(146,8033,8059)"} true;
    $t8 := $ChildMutation($t6, 0, $amount#$1_transaction_fee_CollectedFeesPerBlock($Dereference($t6)));

    // trace_local[collected_amount]($t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:173:13+16
    $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' := $Dereference($t8);
    assume {:print "$track_local(39,2,2):", $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin''} $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'' == $temp_0'$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'';

    // assume Identical($t9, select coin::AggregatableCoin.value($t8)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:395:9+26
    assume {:print "$at(94,16731,16757)"} true;
    assume ($t9 == $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($Dereference($t8)));

    // assume Identical($t10, global<coin::CoinStore<aptos_coin::AptosCoin>>($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/coin.spec.move:396:9+59
    assume {:print "$at(94,16766,16825)"} true;
    assume ($t10 == $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t0));

    // coin::collect_into_aggregatable_coin<aptos_coin::AptosCoin>($t0, $t1, $t8) on_abort goto L2 with $t7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:174:9+79
    assume {:print "$at(146,8069,8148)"} true;
    call $t8 := $1_coin_collect_into_aggregatable_coin'$1_aptos_coin_AptosCoin'($t0, $t1, $t8);
    if ($abort_flag) {
        assume {:print "$at(146,8069,8148)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(39,2):", $t7} $t7 == $t7;
        goto L2;
    }

    // write_back[Reference($t6).amount (coin::AggregatableCoin<aptos_coin::AptosCoin>)]($t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:174:9+79
    $t6 := $UpdateMutation($t6, $Update'$1_transaction_fee_CollectedFeesPerBlock'_amount($Dereference($t6), $Dereference($t8)));

    // pack_ref_deep($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:174:9+79

    // write_back[transaction_fee::CollectedFeesPerBlock@]($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:174:9+79
    $1_transaction_fee_CollectedFeesPerBlock_$memory := $ResourceUpdate($1_transaction_fee_CollectedFeesPerBlock_$memory, $GlobalLocationAddress($t6),
        $Dereference($t6));

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:175:5+1
    assume {:print "$at(146,8154,8155)"} true;
L1:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:175:5+1
    assume {:print "$at(146,8154,8155)"} true;
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:175:5+1
L2:

    // abort($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.move:175:5+1
    assume {:print "$at(146,8154,8155)"} true;
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// spec fun at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.move:154:5+155
function {:inline} $1_reconfiguration_$last_reconfiguration_time($1_reconfiguration_Configuration_$memory: $Memory $1_reconfiguration_Configuration): int {
    $last_reconfiguration_time#$1_reconfiguration_Configuration($ResourceValue($1_reconfiguration_Configuration_$memory, 1))
}

// struct reconfiguration::Configuration at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.move:33:5+306
type {:datatype} $1_reconfiguration_Configuration;
function {:constructor} $1_reconfiguration_Configuration($epoch: int, $last_reconfiguration_time: int, $events: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'): $1_reconfiguration_Configuration;
function {:inline} $Update'$1_reconfiguration_Configuration'_epoch(s: $1_reconfiguration_Configuration, x: int): $1_reconfiguration_Configuration {
    $1_reconfiguration_Configuration(x, $last_reconfiguration_time#$1_reconfiguration_Configuration(s), $events#$1_reconfiguration_Configuration(s))
}
function {:inline} $Update'$1_reconfiguration_Configuration'_last_reconfiguration_time(s: $1_reconfiguration_Configuration, x: int): $1_reconfiguration_Configuration {
    $1_reconfiguration_Configuration($epoch#$1_reconfiguration_Configuration(s), x, $events#$1_reconfiguration_Configuration(s))
}
function {:inline} $Update'$1_reconfiguration_Configuration'_events(s: $1_reconfiguration_Configuration, x: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'): $1_reconfiguration_Configuration {
    $1_reconfiguration_Configuration($epoch#$1_reconfiguration_Configuration(s), $last_reconfiguration_time#$1_reconfiguration_Configuration(s), x)
}
function $IsValid'$1_reconfiguration_Configuration'(s: $1_reconfiguration_Configuration): bool {
    $IsValid'u64'($epoch#$1_reconfiguration_Configuration(s))
      && $IsValid'u64'($last_reconfiguration_time#$1_reconfiguration_Configuration(s))
      && $IsValid'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''($events#$1_reconfiguration_Configuration(s))
}
function {:inline} $IsEqual'$1_reconfiguration_Configuration'(s1: $1_reconfiguration_Configuration, s2: $1_reconfiguration_Configuration): bool {
    s1 == s2
}
var $1_reconfiguration_Configuration_$memory: $Memory $1_reconfiguration_Configuration;

// struct reconfiguration::NewEpochEvent at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.move:28:5+64
type {:datatype} $1_reconfiguration_NewEpochEvent;
function {:constructor} $1_reconfiguration_NewEpochEvent($epoch: int): $1_reconfiguration_NewEpochEvent;
function {:inline} $Update'$1_reconfiguration_NewEpochEvent'_epoch(s: $1_reconfiguration_NewEpochEvent, x: int): $1_reconfiguration_NewEpochEvent {
    $1_reconfiguration_NewEpochEvent(x)
}
function $IsValid'$1_reconfiguration_NewEpochEvent'(s: $1_reconfiguration_NewEpochEvent): bool {
    $IsValid'u64'($epoch#$1_reconfiguration_NewEpochEvent(s))
}
function {:inline} $IsEqual'$1_reconfiguration_NewEpochEvent'(s1: $1_reconfiguration_NewEpochEvent, s2: $1_reconfiguration_NewEpochEvent): bool {
    s1 == s2
}

// struct transaction_validation::TransactionValidation at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:19:5+278
type {:datatype} $1_transaction_validation_TransactionValidation;
function {:constructor} $1_transaction_validation_TransactionValidation($module_addr: int, $module_name: Vec (int), $script_prologue_name: Vec (int), $module_prologue_name: Vec (int), $multi_agent_prologue_name: Vec (int), $user_epilogue_name: Vec (int)): $1_transaction_validation_TransactionValidation;
function {:inline} $Update'$1_transaction_validation_TransactionValidation'_module_addr(s: $1_transaction_validation_TransactionValidation, x: int): $1_transaction_validation_TransactionValidation {
    $1_transaction_validation_TransactionValidation(x, $module_name#$1_transaction_validation_TransactionValidation(s), $script_prologue_name#$1_transaction_validation_TransactionValidation(s), $module_prologue_name#$1_transaction_validation_TransactionValidation(s), $multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s), $user_epilogue_name#$1_transaction_validation_TransactionValidation(s))
}
function {:inline} $Update'$1_transaction_validation_TransactionValidation'_module_name(s: $1_transaction_validation_TransactionValidation, x: Vec (int)): $1_transaction_validation_TransactionValidation {
    $1_transaction_validation_TransactionValidation($module_addr#$1_transaction_validation_TransactionValidation(s), x, $script_prologue_name#$1_transaction_validation_TransactionValidation(s), $module_prologue_name#$1_transaction_validation_TransactionValidation(s), $multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s), $user_epilogue_name#$1_transaction_validation_TransactionValidation(s))
}
function {:inline} $Update'$1_transaction_validation_TransactionValidation'_script_prologue_name(s: $1_transaction_validation_TransactionValidation, x: Vec (int)): $1_transaction_validation_TransactionValidation {
    $1_transaction_validation_TransactionValidation($module_addr#$1_transaction_validation_TransactionValidation(s), $module_name#$1_transaction_validation_TransactionValidation(s), x, $module_prologue_name#$1_transaction_validation_TransactionValidation(s), $multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s), $user_epilogue_name#$1_transaction_validation_TransactionValidation(s))
}
function {:inline} $Update'$1_transaction_validation_TransactionValidation'_module_prologue_name(s: $1_transaction_validation_TransactionValidation, x: Vec (int)): $1_transaction_validation_TransactionValidation {
    $1_transaction_validation_TransactionValidation($module_addr#$1_transaction_validation_TransactionValidation(s), $module_name#$1_transaction_validation_TransactionValidation(s), $script_prologue_name#$1_transaction_validation_TransactionValidation(s), x, $multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s), $user_epilogue_name#$1_transaction_validation_TransactionValidation(s))
}
function {:inline} $Update'$1_transaction_validation_TransactionValidation'_multi_agent_prologue_name(s: $1_transaction_validation_TransactionValidation, x: Vec (int)): $1_transaction_validation_TransactionValidation {
    $1_transaction_validation_TransactionValidation($module_addr#$1_transaction_validation_TransactionValidation(s), $module_name#$1_transaction_validation_TransactionValidation(s), $script_prologue_name#$1_transaction_validation_TransactionValidation(s), $module_prologue_name#$1_transaction_validation_TransactionValidation(s), x, $user_epilogue_name#$1_transaction_validation_TransactionValidation(s))
}
function {:inline} $Update'$1_transaction_validation_TransactionValidation'_user_epilogue_name(s: $1_transaction_validation_TransactionValidation, x: Vec (int)): $1_transaction_validation_TransactionValidation {
    $1_transaction_validation_TransactionValidation($module_addr#$1_transaction_validation_TransactionValidation(s), $module_name#$1_transaction_validation_TransactionValidation(s), $script_prologue_name#$1_transaction_validation_TransactionValidation(s), $module_prologue_name#$1_transaction_validation_TransactionValidation(s), $multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s), x)
}
function $IsValid'$1_transaction_validation_TransactionValidation'(s: $1_transaction_validation_TransactionValidation): bool {
    $IsValid'address'($module_addr#$1_transaction_validation_TransactionValidation(s))
      && $IsValid'vec'u8''($module_name#$1_transaction_validation_TransactionValidation(s))
      && $IsValid'vec'u8''($script_prologue_name#$1_transaction_validation_TransactionValidation(s))
      && $IsValid'vec'u8''($module_prologue_name#$1_transaction_validation_TransactionValidation(s))
      && $IsValid'vec'u8''($multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s))
      && $IsValid'vec'u8''($user_epilogue_name#$1_transaction_validation_TransactionValidation(s))
}
function {:inline} $IsEqual'$1_transaction_validation_TransactionValidation'(s1: $1_transaction_validation_TransactionValidation, s2: $1_transaction_validation_TransactionValidation): bool {
    $IsEqual'address'($module_addr#$1_transaction_validation_TransactionValidation(s1), $module_addr#$1_transaction_validation_TransactionValidation(s2))
    && $IsEqual'vec'u8''($module_name#$1_transaction_validation_TransactionValidation(s1), $module_name#$1_transaction_validation_TransactionValidation(s2))
    && $IsEqual'vec'u8''($script_prologue_name#$1_transaction_validation_TransactionValidation(s1), $script_prologue_name#$1_transaction_validation_TransactionValidation(s2))
    && $IsEqual'vec'u8''($module_prologue_name#$1_transaction_validation_TransactionValidation(s1), $module_prologue_name#$1_transaction_validation_TransactionValidation(s2))
    && $IsEqual'vec'u8''($multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s1), $multi_agent_prologue_name#$1_transaction_validation_TransactionValidation(s2))
    && $IsEqual'vec'u8''($user_epilogue_name#$1_transaction_validation_TransactionValidation(s1), $user_epilogue_name#$1_transaction_validation_TransactionValidation(s2))}
var $1_transaction_validation_TransactionValidation_$memory: $Memory $1_transaction_validation_TransactionValidation;

// fun transaction_validation::initialize [verification] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+620
procedure {:timeLimit 40} $1_transaction_validation_initialize$verify(_$t0: $signer, _$t1: Vec (int), _$t2: Vec (int), _$t3: Vec (int), _$t4: Vec (int)) returns ()
{
    // declare local variables
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: Vec (int);
    var $t10: $1_transaction_validation_TransactionValidation;
    var $t0: $signer;
    var $t1: Vec (int);
    var $t2: Vec (int);
    var $t3: Vec (int);
    var $t4: Vec (int);
    var $temp_0'signer': $signer;
    var $temp_0'vec'u8'': Vec (int);
    var $1_transaction_validation_TransactionValidation_$memory#28: $Memory $1_transaction_validation_TransactionValidation;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;
    $t4 := _$t4;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume {:print "$at(2,1805,1806)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume $IsValid'vec'u8''($t1);

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume $IsValid'vec'u8''($t2);

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume $IsValid'vec'u8''($t3);

    // assume WellFormed($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume $IsValid'vec'u8''($t4);

    // assume forall $rsc: ResourceDomain<transaction_validation::TransactionValidation>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_transaction_validation_TransactionValidation_$memory, $a_0)}(var $rsc := $ResourceValue($1_transaction_validation_TransactionValidation_$memory, $a_0);
    ($IsValid'$1_transaction_validation_TransactionValidation'($rsc))));

    // assume Identical($t5, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:17:9+47
    assume {:print "$at(148,504,551)"} true;
    assume ($t5 == $1_signer_$address_of($t0));

    // @28 := save_mem(transaction_validation::TransactionValidation) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume {:print "$at(2,1805,1806)"} true;
    $1_transaction_validation_TransactionValidation_$memory#28 := $1_transaction_validation_TransactionValidation_$memory;

    // trace_local[aptos_framework]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume {:print "$track_local(59,1,0):", $t0} $t0 == $t0;

    // trace_local[script_prologue_name]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume {:print "$track_local(59,1,1):", $t1} $t1 == $t1;

    // trace_local[module_prologue_name]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume {:print "$track_local(59,1,2):", $t2} $t2 == $t2;

    // trace_local[multi_agent_prologue_name]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume {:print "$track_local(59,1,3):", $t3} $t3 == $t3;

    // trace_local[user_epilogue_name]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:47:5+1
    assume {:print "$track_local(59,1,4):", $t4} $t4 == $t4;

    // opaque begin: system_addresses::assert_aptos_framework($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
    assume {:print "$at(2,2057,2114)"} true;

    // assume Identical($t6, Neq<address>(signer::$address_of($t0), 1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
    assume ($t6 == !$IsEqual'address'($1_signer_$address_of($t0), 1));

    // if ($t6) goto L4 else goto L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
    if ($t6) { goto L4; } else { goto L3; }

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
L4:

    // assume And(Neq<address>(signer::$address_of($t0), 1), Eq(5, $t7)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
    assume {:print "$at(2,2057,2114)"} true;
    assume (!$IsEqual'address'($1_signer_$address_of($t0), 1) && $IsEqual'num'(5, $t7));

    // trace_abort($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
    assume {:print "$at(2,2057,2114)"} true;
    assume {:print "$track_abort(59,1):", $t7} $t7 == $t7;

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
    goto L2;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
L3:

    // opaque end: system_addresses::assert_aptos_framework($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:54:9+57
    assume {:print "$at(2,2057,2114)"} true;

    // $t8 := 0x1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:57:26+16
    assume {:print "$at(2,2199,2215)"} true;
    $t8 := 1;
    assume $IsValid'address'($t8);

    // $t9 := [116, 114, 97, 110, 115, 97, 99, 116, 105, 111, 110, 95, 118, 97, 108, 105, 100, 97, 116, 105, 111, 110] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:58:26+25
    assume {:print "$at(2,2242,2267)"} true;
    $t9 := ConcatVec(ConcatVec(ConcatVec(ConcatVec(ConcatVec(MakeVec4(116, 114, 97, 110), MakeVec4(115, 97, 99, 116)), MakeVec4(105, 111, 110, 95)), MakeVec4(118, 97, 108, 105)), MakeVec4(100, 97, 116, 105)), MakeVec2(111, 110));
    assume $IsValid'vec'u8''($t9);

    // $t10 := pack transaction_validation::TransactionValidation($t8, $t9, $t1, $t2, $t3, $t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:56:34+267
    assume {:print "$at(2,2150,2417)"} true;
    $t10 := $1_transaction_validation_TransactionValidation($t8, $t9, $t1, $t2, $t3, $t4);

    // move_to<transaction_validation::TransactionValidation>($t10, $t0) on_abort goto L2 with $t7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:56:9+7
    if ($ResourceExists($1_transaction_validation_TransactionValidation_$memory, $addr#$signer($t0))) {
        call $ExecFailureAbort();
    } else {
        $1_transaction_validation_TransactionValidation_$memory := $ResourceUpdate($1_transaction_validation_TransactionValidation_$memory, $addr#$signer($t0), $t10);
    }
    if ($abort_flag) {
        assume {:print "$at(2,2125,2132)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(59,1):", $t7} $t7 == $t7;
        goto L2;
    }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:64:5+1
    assume {:print "$at(2,2424,2425)"} true;
L1:

    // assert Not(Not(system_addresses::$is_aptos_framework_address[]($t5))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:18:9+62
    assume {:print "$at(148,560,622)"} true;
    assert {:msg "assert_failed(148,560,622): function does not abort under this condition"}
      !!$1_system_addresses_$is_aptos_framework_address($t5);

    // assert Not(exists[@28]<transaction_validation::TransactionValidation>($t5)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:19:9+46
    assume {:print "$at(148,631,677)"} true;
    assert {:msg "assert_failed(148,631,677): function does not abort under this condition"}
      !$ResourceExists($1_transaction_validation_TransactionValidation_$memory#28, $t5);

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:19:9+46
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:64:5+1
    assume {:print "$at(2,2424,2425)"} true;
L2:

    // assert Or(Not(system_addresses::$is_aptos_framework_address[]($t5)), exists[@28]<transaction_validation::TransactionValidation>($t5)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:9:5+440
    assume {:print "$at(148,242,682)"} true;
    assert {:msg "assert_failed(148,242,682): abort not covered by any of the `aborts_if` clauses"}
      (!$1_system_addresses_$is_aptos_framework_address($t5) || $ResourceExists($1_transaction_validation_TransactionValidation_$memory#28, $t5));

    // abort($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:9:5+440
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun transaction_validation::epilogue [verification] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1518
procedure {:timeLimit 40} $1_transaction_validation_epilogue$verify(_$t0: $signer, _$t1: int, _$t2: int, _$t3: int, _$t4: int) returns ()
{
    // declare local variables
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: $1_account_Account;
    var $t13: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
    var $t14: $1_aggregator_Aggregator;
    var $t15: int;
    var $t16: int;
    var $t17: int;
    var $t18: int;
    var $t19: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t20: $1_optional_aggregator_OptionalAggregator;
    var $t21: int;
    var $t22: bool;
    var $t23: int;
    var $t24: int;
    var $t25: int;
    var $t26: int;
    var $t27: int;
    var $t28: int;
    var $t29: int;
    var $t30: int;
    var $t31: bool;
    var $t32: int;
    var $t33: int;
    var $t34: int;
    var $t35: int;
    var $t36: int;
    var $t37: bool;
    var $t38: int;
    var $t39: int;
    var $t40: bool;
    var $t41: $1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin';
    var $t42: $1_aggregator_Aggregator;
    var $t43: int;
    var $t44: int;
    var $t45: int;
    var $t46: $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $t47: $1_option_Option'$1_optional_aggregator_OptionalAggregator';
    var $t48: $1_optional_aggregator_OptionalAggregator;
    var $t49: int;
    var $t50: int;
    var $t51: int;
    var $t52: $1_account_Account;
    var $t0: $signer;
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#48: $Memory $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    var $1_account_Account_$memory#49: $Memory $1_account_Account;
    var $1_transaction_fee_CollectedFeesPerBlock_$memory#50: $Memory $1_transaction_fee_CollectedFeesPerBlock;
    var $1_transaction_fee_AptosCoinCapabilities_$memory#51: $Memory $1_transaction_fee_AptosCoinCapabilities;
    var $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory#52: $Memory $1_coin_CoinInfo'$1_aptos_coin_AptosCoin';
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;
    $t4 := _$t4;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume {:print "$at(2,7114,7115)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume $IsValid'u64'($t1);

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume $IsValid'u64'($t2);

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume $IsValid'u64'($t3);

    // assume WellFormed($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume $IsValid'u64'($t4);

    // assume forall $rsc: ResourceDomain<features::Features>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_features_Features_$memory, $a_0)}(var $rsc := $ResourceValue($1_features_Features_$memory, $a_0);
    ($IsValid'$1_features_Features'($rsc))));

    // assume forall $rsc: ResourceDomain<account::Account>(): And(WellFormed($rsc), And(Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.rotation_capability_offer($rsc)))), 1), Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.signer_capability_offer($rsc)))), 1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_account_Account_$memory, $a_0)}(var $rsc := $ResourceValue($1_account_Account_$memory, $a_0);
    (($IsValid'$1_account_Account'($rsc) && ((LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_RotationCapability'($rotation_capability_offer#$1_account_Account($rsc)))) <= 1) && (LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_SignerCapability'($signer_capability_offer#$1_account_Account($rsc)))) <= 1))))));

    // assume forall $rsc: ResourceDomain<coin::CoinInfo<aptos_coin::AptosCoin>>(): And(WellFormed($rsc), And(Le(Len<optional_aggregator::OptionalAggregator>(select option::Option.vec(select coin::CoinInfo.supply($rsc))), 1), forall $elem: select option::Option.vec(select coin::CoinInfo.supply($rsc)): And(And(And(And(And(Iff(option::$is_some<aggregator::Aggregator>(select optional_aggregator::OptionalAggregator.aggregator($elem)), option::$is_none<optional_aggregator::Integer>(select optional_aggregator::OptionalAggregator.integer($elem))), Iff(option::$is_some<optional_aggregator::Integer>(select optional_aggregator::OptionalAggregator.integer($elem)), option::$is_none<aggregator::Aggregator>(select optional_aggregator::OptionalAggregator.aggregator($elem)))), Implies(option::$is_some<optional_aggregator::Integer>(select optional_aggregator::OptionalAggregator.integer($elem)), Le(select optional_aggregator::Integer.value(option::$borrow<optional_aggregator::Integer>(select optional_aggregator::OptionalAggregator.integer($elem))), select optional_aggregator::Integer.limit(option::$borrow<optional_aggregator::Integer>(select optional_aggregator::OptionalAggregator.integer($elem)))))), Implies(option::$is_some<aggregator::Aggregator>(select optional_aggregator::OptionalAggregator.aggregator($elem)), Le(aggregator::spec_aggregator_get_val(option::$borrow<aggregator::Aggregator>(select optional_aggregator::OptionalAggregator.aggregator($elem))), aggregator::spec_get_limit(option::$borrow<aggregator::Aggregator>(select optional_aggregator::OptionalAggregator.aggregator($elem)))))), Le(Len<aggregator::Aggregator>(select option::Option.vec(select optional_aggregator::OptionalAggregator.aggregator($elem))), 1)), Le(Len<optional_aggregator::Integer>(select option::Option.vec(select optional_aggregator::OptionalAggregator.integer($elem))), 1)))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $a_0)}(var $rsc := $ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $a_0);
    (($IsValid'$1_coin_CoinInfo'$1_aptos_coin_AptosCoin''($rsc) && ((LenVec($vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'($supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($rsc))) <= 1) && (var $range_1 := $vec#$1_option_Option'$1_optional_aggregator_OptionalAggregator'($supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($rsc)); (forall $i_2: int :: InRangeVec($range_1, $i_2) ==> (var $elem := ReadVec($range_1, $i_2);
    ((((((($1_option_$is_some'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator($elem)) <==> $1_option_$is_none'$1_optional_aggregator_Integer'($integer#$1_optional_aggregator_OptionalAggregator($elem))) && ($1_option_$is_some'$1_optional_aggregator_Integer'($integer#$1_optional_aggregator_OptionalAggregator($elem)) <==> $1_option_$is_none'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator($elem)))) && ($1_option_$is_some'$1_optional_aggregator_Integer'($integer#$1_optional_aggregator_OptionalAggregator($elem)) ==> ($value#$1_optional_aggregator_Integer($1_option_$borrow'$1_optional_aggregator_Integer'($integer#$1_optional_aggregator_OptionalAggregator($elem))) <= $limit#$1_optional_aggregator_Integer($1_option_$borrow'$1_optional_aggregator_Integer'($integer#$1_optional_aggregator_OptionalAggregator($elem)))))) && ($1_option_$is_some'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator($elem)) ==> ($1_aggregator_spec_aggregator_get_val($1_option_$borrow'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator($elem))) <= $1_aggregator_spec_get_limit($1_option_$borrow'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator($elem)))))) && (LenVec($vec#$1_option_Option'$1_aggregator_Aggregator'($aggregator#$1_optional_aggregator_OptionalAggregator($elem))) <= 1)) && (LenVec($vec#$1_option_Option'$1_optional_aggregator_Integer'($integer#$1_optional_aggregator_OptionalAggregator($elem))) <= 1)))))))))));

    // assume forall $rsc: ResourceDomain<coin::CoinStore<aptos_coin::AptosCoin>>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0)}(var $rsc := $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0);
    ($IsValid'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''($rsc))));

    // assume forall $rsc: ResourceDomain<chain_status::GenesisEndMarker>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0);
    ($IsValid'$1_chain_status_GenesisEndMarker'($rsc))));

    // assume forall $rsc: ResourceDomain<transaction_fee::AptosCoinCapabilities>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_transaction_fee_AptosCoinCapabilities_$memory, $a_0)}(var $rsc := $ResourceValue($1_transaction_fee_AptosCoinCapabilities_$memory, $a_0);
    ($IsValid'$1_transaction_fee_AptosCoinCapabilities'($rsc))));

    // assume forall $rsc: ResourceDomain<transaction_fee::CollectedFeesPerBlock>(): And(WellFormed($rsc), And(And(Le(select transaction_fee::CollectedFeesPerBlock.burn_percentage($rsc), 100), Eq<u128>(aggregator::spec_get_limit(select coin::AggregatableCoin.value(select transaction_fee::CollectedFeesPerBlock.amount($rsc))), 18446744073709551615)), Le(Len<address>(select option::Option.vec(select transaction_fee::CollectedFeesPerBlock.proposer($rsc))), 1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_transaction_fee_CollectedFeesPerBlock_$memory, $a_0)}(var $rsc := $ResourceValue($1_transaction_fee_CollectedFeesPerBlock_$memory, $a_0);
    (($IsValid'$1_transaction_fee_CollectedFeesPerBlock'($rsc) && ((($burn_percentage#$1_transaction_fee_CollectedFeesPerBlock($rsc) <= 100) && $IsEqual'u128'($1_aggregator_spec_get_limit($value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($amount#$1_transaction_fee_CollectedFeesPerBlock($rsc))), 18446744073709551615)) && (LenVec($vec#$1_option_Option'address'($proposer#$1_transaction_fee_CollectedFeesPerBlock($rsc))) <= 1))))));

    // assume Implies(chain_status::$is_operating(), exists<transaction_fee::AptosCoinCapabilities>(1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1518
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:7:9+105
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> $ResourceExists($1_transaction_fee_AptosCoinCapabilities_$memory, 1));

    // assume Identical($t8, Sub($t3, $t4)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:150:9+55
    assume {:print "$at(148,5666,5721)"} true;
    assume ($t8 == ($t3 - $t4));

    // assume Identical($t9, Mul($t2, $t8)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:153:9+54
    assume {:print "$at(148,5789,5843)"} true;
    assume ($t9 == ($t2 * $t8));

    // assume Identical($t10, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:155:9+39
    assume {:print "$at(148,5853,5892)"} true;
    assume ($t10 == $1_signer_$address_of($t0));

    // assume Identical($t11, select coin::Coin.value(select coin::CoinStore.coin(global<coin::CoinStore<aptos_coin::AptosCoin>>($t10)))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:163:9+70
    assume {:print "$at(148,6196,6266)"} true;
    assume ($t11 == $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t10))));

    // assume Identical($t12, global<account::Account>($t10)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:165:9+49
    assume {:print "$at(148,6355,6404)"} true;
    assume ($t12 == $ResourceValue($1_account_Account_$memory, $t10));

    // assume Identical($t13, select transaction_fee::CollectedFeesPerBlock.amount(global<transaction_fee::CollectedFeesPerBlock>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:172:9+76
    assume {:print "$at(148,6667,6743)"} true;
    assume ($t13 == $amount#$1_transaction_fee_CollectedFeesPerBlock($ResourceValue($1_transaction_fee_CollectedFeesPerBlock_$memory, 1)));

    // assume Identical($t14, select coin::AggregatableCoin.value($t13)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:173:9+32
    assume {:print "$at(148,6752,6784)"} true;
    assume ($t14 == $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($t13));

    // assume Identical($t15, aggregator::spec_aggregator_get_val($t14)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:174:9+57
    assume {:print "$at(148,6793,6850)"} true;
    assume ($t15 == $1_aggregator_spec_aggregator_get_val($t14));

    // assume Identical($t16, aggregator::spec_get_limit($t14)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:175:9+48
    assume {:print "$at(148,6859,6907)"} true;
    assume ($t16 == $1_aggregator_spec_get_limit($t14));

    // assume Identical($t17, select type_info::TypeInfo.account_address(type_info::$type_of<aptos_coin::AptosCoin>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:176:9+65
    assume {:print "$at(148,6916,6981)"} true;
    assume ($t17 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t18, select type_info::TypeInfo.account_address(type_info::$type_of<aptos_coin::AptosCoin>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:178:9+63
    assume {:print "$at(148,7039,7102)"} true;
    assume ($t18 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t19, select coin::CoinInfo.supply(global<coin::CoinInfo<aptos_coin::AptosCoin>>($t18))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:179:9+68
    assume {:print "$at(148,7111,7179)"} true;
    assume ($t19 == $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t18)));

    // assume Identical($t20, option::spec_borrow<optional_aggregator::OptionalAggregator>($t19)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:180:9+55
    assume {:print "$at(148,7188,7243)"} true;
    assume ($t20 == $1_option_spec_borrow'$1_optional_aggregator_OptionalAggregator'($t19));

    // assume Identical($t21, optional_aggregator::optional_aggregator_value($t20)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:181:9+82
    assume {:print "$at(148,7252,7334)"} true;
    assume ($t21 == $1_optional_aggregator_optional_aggregator_value($t20));

    // @49 := save_mem(account::Account) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume {:print "$at(2,7114,7115)"} true;
    $1_account_Account_$memory#49 := $1_account_Account_$memory;

    // @52 := save_mem(coin::CoinInfo<aptos_coin::AptosCoin>) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory#52 := $1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory;

    // @48 := save_mem(coin::CoinStore<aptos_coin::AptosCoin>) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#48 := $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory;

    // @51 := save_mem(transaction_fee::AptosCoinCapabilities) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    $1_transaction_fee_AptosCoinCapabilities_$memory#51 := $1_transaction_fee_AptosCoinCapabilities_$memory;

    // @50 := save_mem(transaction_fee::CollectedFeesPerBlock) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    $1_transaction_fee_CollectedFeesPerBlock_$memory#50 := $1_transaction_fee_CollectedFeesPerBlock_$memory;

    // trace_local[account]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume {:print "$track_local(59,0,0):", $t0} $t0 == $t0;

    // trace_local[_txn_sequence_number]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume {:print "$track_local(59,0,1):", $t1} $t1 == $t1;

    // trace_local[txn_gas_price]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume {:print "$track_local(59,0,2):", $t2} $t2 == $t2;

    // trace_local[txn_max_gas_units]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume {:print "$track_local(59,0,3):", $t3} $t3 == $t3;

    // trace_local[gas_units_remaining]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:183:5+1
    assume {:print "$track_local(59,0,4):", $t4} $t4 == $t4;

    // $t22 := >=($t3, $t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:35+2
    assume {:print "$at(2,7323,7325)"} true;
    call $t22 := $Ge($t3, $t4);

    // if ($t22) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:9+87
    if ($t22) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:9+87
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:9+87
    assume {:print "$at(2,7297,7384)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:83+11
L0:

    // $t23 := 6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:83+11
    assume {:print "$at(2,7371,7382)"} true;
    $t23 := 6;
    assume $IsValid'u64'($t23);

    // $t24 := error::invalid_argument($t23) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:59+36
    call $t24 := $1_error_invalid_argument($t23);
    if ($abort_flag) {
        assume {:print "$at(2,7347,7383)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // trace_abort($t24) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:9+87
    assume {:print "$at(2,7297,7384)"} true;
    assume {:print "$track_abort(59,0):", $t24} $t24 == $t24;

    // $t25 := move($t24) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:9+87
    $t25 := $t24;

    // goto L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:190:9+87
    goto L13;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:191:24+17
    assume {:print "$at(2,7409,7426)"} true;
L2:

    // $t26 := -($t3, $t4) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:191:42+1
    assume {:print "$at(2,7427,7428)"} true;
    call $t26 := $Sub($t3, $t4);
    if ($abort_flag) {
        assume {:print "$at(2,7427,7428)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // trace_local[gas_used]($t26) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:191:13+8
    assume {:print "$track_local(59,0,6):", $t26} $t26 == $t26;

    // $t27 := (u128)($t2) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:194:13+23
    assume {:print "$at(2,7480,7503)"} true;
    call $t27 := $CastU128($t2);
    if ($abort_flag) {
        assume {:print "$at(2,7480,7503)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // $t28 := (u128)($t26) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:194:39+18
    call $t28 := $CastU128($t26);
    if ($abort_flag) {
        assume {:print "$at(2,7506,7524)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // $t29 := *($t27, $t28) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:194:37+1
    call $t29 := $MulU128($t27, $t28);
    if ($abort_flag) {
        assume {:print "$at(2,7504,7505)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // $t30 := 18446744073709551615 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:194:61+7
    $t30 := 18446744073709551615;
    assume $IsValid'u128'($t30);

    // $t31 := <=($t29, $t30) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:194:58+2
    call $t31 := $Le($t29, $t30);

    // if ($t31) goto L4 else goto L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:193:9+132
    assume {:print "$at(2,7459,7591)"} true;
    if ($t31) { goto L4; } else { goto L3; }

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:193:9+132
L4:

    // goto L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:193:9+132
    assume {:print "$at(2,7459,7591)"} true;
    goto L5;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:195:33+11
    assume {:print "$at(2,7569,7580)"} true;
L3:

    // $t32 := 6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:195:33+11
    assume {:print "$at(2,7569,7580)"} true;
    $t32 := 6;
    assume $IsValid'u64'($t32);

    // $t33 := error::out_of_range($t32) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:195:13+32
    call $t33 := $1_error_out_of_range($t32);
    if ($abort_flag) {
        assume {:print "$at(2,7549,7581)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // trace_abort($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:193:9+132
    assume {:print "$at(2,7459,7591)"} true;
    assume {:print "$track_abort(59,0):", $t33} $t33 == $t33;

    // $t25 := move($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:193:9+132
    $t25 := $t33;

    // goto L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:193:9+132
    goto L13;

    // label L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:197:38+13
    assume {:print "$at(2,7630,7643)"} true;
L5:

    // $t34 := *($t2, $t26) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:197:52+1
    assume {:print "$at(2,7644,7645)"} true;
    call $t34 := $MulU64($t2, $t26);
    if ($abort_flag) {
        assume {:print "$at(2,7644,7645)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // trace_local[transaction_fee_amount]($t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:197:13+22
    assume {:print "$track_local(59,0,7):", $t34} $t34 == $t34;

    // $t35 := signer::address_of($t0) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:198:20+28
    assume {:print "$at(2,7675,7703)"} true;
    call $t35 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,7675,7703)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // trace_local[addr]($t35) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:198:13+4
    assume {:print "$track_local(59,0,5):", $t35} $t35 == $t35;

    // $t36 := coin::balance<aptos_coin::AptosCoin>($t35) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:202:13+30
    assume {:print "$at(2,7851,7881)"} true;
    call $t36 := $1_coin_balance'$1_aptos_coin_AptosCoin'($t35);
    if ($abort_flag) {
        assume {:print "$at(2,7851,7881)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // $t37 := >=($t36, $t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:202:44+2
    call $t37 := $Ge($t36, $t34);

    // if ($t37) goto L7 else goto L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:201:9+153
    assume {:print "$at(2,7830,7983)"} true;
    if ($t37) { goto L7; } else { goto L6; }

    // label L7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:201:9+153
L7:

    // goto L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:201:9+153
    assume {:print "$at(2,7830,7983)"} true;
    goto L8;

    // label L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:203:33+30
    assume {:print "$at(2,7941,7971)"} true;
L6:

    // $t38 := 1005 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:203:33+30
    assume {:print "$at(2,7941,7971)"} true;
    $t38 := 1005;
    assume $IsValid'u64'($t38);

    // $t39 := error::out_of_range($t38) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:203:13+51
    call $t39 := $1_error_out_of_range($t38);
    if ($abort_flag) {
        assume {:print "$at(2,7921,7972)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // trace_abort($t39) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:201:9+153
    assume {:print "$at(2,7830,7983)"} true;
    assume {:print "$track_abort(59,0):", $t39} $t39 == $t39;

    // $t25 := move($t39) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:201:9+153
    $t25 := $t39;

    // goto L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:201:9+153
    goto L13;

    // label L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:206:13+43
    assume {:print "$at(2,7998,8041)"} true;
L8:

    // $t40 := features::collect_and_distribute_gas_fees() on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:206:13+43
    assume {:print "$at(2,7998,8041)"} true;
    call $t40 := $1_features_collect_and_distribute_gas_fees();
    if ($abort_flag) {
        assume {:print "$at(2,7998,8041)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // if ($t40) goto L10 else goto L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:206:9+543
    if ($t40) { goto L10; } else { goto L9; }

    // label L10 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:209:42+4
    assume {:print "$at(2,8213,8217)"} true;
L10:

    // assume Identical($t41, select transaction_fee::CollectedFeesPerBlock.amount(global<transaction_fee::CollectedFeesPerBlock>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:112:9+76
    assume {:print "$at(147,5114,5190)"} true;
    assume ($t41 == $amount#$1_transaction_fee_CollectedFeesPerBlock($ResourceValue($1_transaction_fee_CollectedFeesPerBlock_$memory, 1)));

    // assume Identical($t42, select coin::AggregatableCoin.value($t41)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:113:9+32
    assume {:print "$at(147,5199,5231)"} true;
    assume ($t42 == $value#$1_coin_AggregatableCoin'$1_aptos_coin_AptosCoin'($t41));

    // transaction_fee::collect_fee($t35, $t34) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:209:13+58
    assume {:print "$at(2,8184,8242)"} true;
    call $1_transaction_fee_collect_fee($t35, $t34);
    if ($abort_flag) {
        assume {:print "$at(2,8184,8242)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // goto L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:209:71+1
    goto L11;

    // label L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:214:39+4
    assume {:print "$at(2,8497,8501)"} true;
L9:

    // assume Identical($t43, $t35) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:78:9+27
    assume {:print "$at(147,3636,3663)"} true;
    assume ($t43 == $t35);

    // assume Identical($t44, $t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:79:9+17
    assume {:print "$at(147,3672,3689)"} true;
    assume ($t44 == $t34);

    // assume Identical($t45, select type_info::TypeInfo.account_address(type_info::$type_of<aptos_coin::AptosCoin>())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:81:9+65
    assume {:print "$at(147,3699,3764)"} true;
    assume ($t45 == $account_address#$1_type_info_TypeInfo($1_type_info_TypeInfo(1, Vec(DefaultVecMap()[0 := 97][1 := 112][2 := 116][3 := 111][4 := 115][5 := 95][6 := 99][7 := 111][8 := 105][9 := 110], 10), Vec(DefaultVecMap()[0 := 65][1 := 112][2 := 116][3 := 111][4 := 115][5 := 67][6 := 111][7 := 105][8 := 110], 9))));

    // assume Identical($t46, global<coin::CoinStore<aptos_coin::AptosCoin>>($t43)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:82:9+60
    assume {:print "$at(147,3773,3833)"} true;
    assume ($t46 == $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t43));

    // assume Identical($t47, select coin::CoinInfo.supply(global<coin::CoinInfo<aptos_coin::AptosCoin>>($t45))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:92:9+66
    assume {:print "$at(147,4229,4295)"} true;
    assume ($t47 == $supply#$1_coin_CoinInfo'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory, $t45)));

    // assume Identical($t48, option::spec_borrow<optional_aggregator::OptionalAggregator>($t47)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:93:9+47
    assume {:print "$at(147,4304,4351)"} true;
    assume ($t48 == $1_option_spec_borrow'$1_optional_aggregator_OptionalAggregator'($t47));

    // assume Identical($t49, optional_aggregator::optional_aggregator_value($t48)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_fee.spec.move:94:9+67
    assume {:print "$at(147,4360,4427)"} true;
    assume ($t49 == $1_optional_aggregator_optional_aggregator_value($t48));

    // transaction_fee::burn_fee($t35, $t34) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:214:13+55
    assume {:print "$at(2,8471,8526)"} true;
    call $1_transaction_fee_burn_fee($t35, $t34);
    if ($abort_flag) {
        assume {:print "$at(2,8471,8526)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // label L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:218:44+4
    assume {:print "$at(2,8620,8624)"} true;
L11:

    // assume Identical($t50, select account::Account.sequence_number(global<account::Account>($t35))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:52:9+60
    assume {:print "$at(72,2145,2205)"} true;
    assume ($t50 == $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory, $t35)));

    // account::increment_sequence_number($t35) on_abort goto L13 with $t25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:218:9+40
    assume {:print "$at(2,8585,8625)"} true;
    call $1_account_increment_sequence_number($t35);
    if ($abort_flag) {
        assume {:print "$at(2,8585,8625)"} true;
        $t25 := $abort_code;
        assume {:print "$track_abort(59,0):", $t25} $t25 == $t25;
        goto L13;
    }

    // label L12 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:219:5+1
    assume {:print "$at(2,8631,8632)"} true;
L12:

    // assume Identical($t51, select coin::Coin.value(select coin::CoinStore.coin(global<coin::CoinStore<aptos_coin::AptosCoin>>($t10)))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:164:9+71
    assume {:print "$at(148,6275,6346)"} true;
    assume ($t51 == $value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $t10))));

    // assume Identical($t52, global<account::Account>($t10)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:166:9+50
    assume {:print "$at(148,6413,6463)"} true;
    assume ($t52 == $ResourceValue($1_account_Account_$memory, $t10));

    // assert Not(Not(Ge($t3, $t4))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:149:9+54
    assume {:print "$at(148,5603,5657)"} true;
    assert {:msg "assert_failed(148,5603,5657): function does not abort under this condition"}
      !!($t3 >= $t4);

    // assert Not(Not(Le(Mul($t2, $t8), 18446744073709551615))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:152:9+49
    assume {:print "$at(148,5731,5780)"} true;
    assert {:msg "assert_failed(148,5731,5780): function does not abort under this condition"}
      !!(($t2 * $t8) <= 18446744073709551615);

    // assert Not(Not(exists[@48]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:156:9+46
    assume {:print "$at(148,5901,5947)"} true;
    assert {:msg "assert_failed(148,5901,5947): function does not abort under this condition"}
      !!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#48, $t10);

    // assert Not(Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@48]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))), $t9))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:158:9+85
    assume {:print "$at(148,5988,6073)"} true;
    assert {:msg "assert_failed(148,5988,6073): function does not abort under this condition"}
      !!($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#48, $t10))) >= $t9);

    // assert Not(Not(exists[@49]<account::Account>($t10))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:160:9+33
    assume {:print "$at(148,6083,6116)"} true;
    assert {:msg "assert_failed(148,6083,6116): function does not abort under this condition"}
      !!$ResourceExists($1_account_Account_$memory#49, $t10);

    // assert Not(Not(Lt(select account::Account.sequence_number(global[@49]<account::Account>($t10)), 18446744073709551615))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:161:9+61
    assume {:print "$at(148,6125,6186)"} true;
    assert {:msg "assert_failed(148,6125,6186): function does not abort under this condition"}
      !!($sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#49, $t10)) < 18446744073709551615);

    // assert Not((if features::spec_is_enabled[](6) {Or(Not(exists[@50]<transaction_fee::CollectedFeesPerBlock>(1)), And(Gt($t9, 0), Or(Gt(Add($t15, $t9), $t16), Gt(Add($t15, $t9), 340282366920938463463374607431768211455))))} else {Or(Or(Not(exists[@51]<transaction_fee::AptosCoinCapabilities>(1)), And(Gt($t9, 0), Not(exists[@52]<coin::CoinInfo<aptos_coin::AptosCoin>>($t17)))), And(option::spec_is_some[]<optional_aggregator::OptionalAggregator>($t19), Lt($t21, $t9)))})) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:183:9+956
    assume {:print "$at(148,7393,8349)"} true;
    assert {:msg "assert_failed(148,7393,8349): function does not abort under this condition"}
      !(if ($1_features_spec_is_enabled(6)) then ((!$ResourceExists($1_transaction_fee_CollectedFeesPerBlock_$memory#50, 1) || (($t9 > 0) && ((($t15 + $t9) > $t16) || (($t15 + $t9) > 340282366920938463463374607431768211455))))) else (((!$ResourceExists($1_transaction_fee_AptosCoinCapabilities_$memory#51, 1) || (($t9 > 0) && !$ResourceExists($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory#52, $t17))) || ($1_option_spec_is_some'$1_optional_aggregator_OptionalAggregator'($t19) && ($t21 < $t9)))));

    // assert Eq<u64>($t51, Sub($t11, $t9)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:167:9+56
    assume {:print "$at(148,6472,6528)"} true;
    assert {:msg "assert_failed(148,6472,6528): post-condition does not hold"}
      $IsEqual'u64'($t51, ($t11 - $t9));

    // assert Eq<u64>(select account::Account.sequence_number($t52), Add(select account::Account.sequence_number($t12), 1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:168:9+67
    assume {:print "$at(148,6537,6604)"} true;
    assert {:msg "assert_failed(148,6537,6604): post-condition does not hold"}
      $IsEqual'u64'($sequence_number#$1_account_Account($t52), ($sequence_number#$1_account_Account($t12) + 1));

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:168:9+67
    return;

    // label L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:219:5+1
    assume {:print "$at(2,8631,8632)"} true;
L13:

    // assert Or(Or(Or(Or(Or(Or(Not(Ge($t3, $t4)), Not(Le(Mul($t2, $t8), 18446744073709551615))), Not(exists[@48]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))), Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@48]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))), $t9))), Not(exists[@49]<account::Account>($t10))), Not(Lt(select account::Account.sequence_number(global[@49]<account::Account>($t10)), 18446744073709551615))), (if features::spec_is_enabled[](6) {Or(Not(exists[@50]<transaction_fee::CollectedFeesPerBlock>(1)), And(Gt($t9, 0), Or(Gt(Add($t15, $t9), $t16), Gt(Add($t15, $t9), 340282366920938463463374607431768211455))))} else {Or(Or(Not(exists[@51]<transaction_fee::AptosCoinCapabilities>(1)), And(Gt($t9, 0), Not(exists[@52]<coin::CoinInfo<aptos_coin::AptosCoin>>($t17)))), And(option::spec_is_some[]<optional_aggregator::OptionalAggregator>($t19), Lt($t21, $t9)))})) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:133:5+3342
    assume {:print "$at(148,5013,8355)"} true;
    assert {:msg "assert_failed(148,5013,8355): abort not covered by any of the `aborts_if` clauses"}
      ((((((!($t3 >= $t4) || !(($t2 * $t8) <= 18446744073709551615)) || !$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#48, $t10)) || !($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#48, $t10))) >= $t9)) || !$ResourceExists($1_account_Account_$memory#49, $t10)) || !($sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#49, $t10)) < 18446744073709551615)) || (if ($1_features_spec_is_enabled(6)) then ((!$ResourceExists($1_transaction_fee_CollectedFeesPerBlock_$memory#50, 1) || (($t9 > 0) && ((($t15 + $t9) > $t16) || (($t15 + $t9) > 340282366920938463463374607431768211455))))) else (((!$ResourceExists($1_transaction_fee_AptosCoinCapabilities_$memory#51, 1) || (($t9 > 0) && !$ResourceExists($1_coin_CoinInfo'$1_aptos_coin_AptosCoin'_$memory#52, $t17))) || ($1_option_spec_is_some'$1_optional_aggregator_OptionalAggregator'($t19) && ($t21 < $t9))))));

    // abort($t25) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:133:5+3342
    $abort_code := $t25;
    $abort_flag := true;
    return;

}

// fun transaction_validation::module_prologue [verification] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+378
procedure {:timeLimit 40} $1_transaction_validation_module_prologue$verify(_$t0: $signer, _$t1: int, _$t2: Vec (int), _$t3: int, _$t4: int, _$t5: int, _$t6: int) returns ()
{
    // declare local variables
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t0: $signer;
    var $t1: int;
    var $t2: Vec (int);
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $temp_0'u8': int;
    var $temp_0'vec'u8'': Vec (int);
    var $1_timestamp_CurrentTimeMicroseconds_$memory#33: $Memory $1_timestamp_CurrentTimeMicroseconds;
    var $1_chain_id_ChainId_$memory#34: $Memory $1_chain_id_ChainId;
    var $1_account_Account_$memory#35: $Memory $1_account_Account;
    var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#36: $Memory $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
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
    // assume WellFormed($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$at(2,4443,4444)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume $IsValid'u64'($t1);

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume $IsValid'vec'u8''($t2);

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume $IsValid'u64'($t3);

    // assume WellFormed($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume $IsValid'u64'($t4);

    // assume WellFormed($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume $IsValid'u64'($t5);

    // assume WellFormed($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume $IsValid'u8'($t6);

    // assume forall $rsc: ResourceDomain<chain_id::ChainId>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_id_ChainId_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_id_ChainId_$memory, $a_0);
    ($IsValid'$1_chain_id_ChainId'($rsc))));

    // assume forall $rsc: ResourceDomain<account::Account>(): And(WellFormed($rsc), And(Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.rotation_capability_offer($rsc)))), 1), Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.signer_capability_offer($rsc)))), 1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_account_Account_$memory, $a_0)}(var $rsc := $ResourceValue($1_account_Account_$memory, $a_0);
    (($IsValid'$1_account_Account'($rsc) && ((LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_RotationCapability'($rotation_capability_offer#$1_account_Account($rsc)))) <= 1) && (LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_SignerCapability'($signer_capability_offer#$1_account_Account($rsc)))) <= 1))))));

    // assume forall $rsc: ResourceDomain<coin::CoinStore<aptos_coin::AptosCoin>>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0)}(var $rsc := $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0);
    ($IsValid'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''($rsc))));

    // assume forall $rsc: ResourceDomain<chain_status::GenesisEndMarker>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0);
    ($IsValid'$1_chain_status_GenesisEndMarker'($rsc))));

    // assume forall $rsc: ResourceDomain<timestamp::CurrentTimeMicroseconds>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0)}(var $rsc := $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0);
    ($IsValid'$1_timestamp_CurrentTimeMicroseconds'($rsc))));

    // assume forall $rsc: ResourceDomain<reconfiguration::Configuration>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_reconfiguration_Configuration_$memory, $a_0)}(var $rsc := $ResourceValue($1_reconfiguration_Configuration_$memory, $a_0);
    ($IsValid'$1_reconfiguration_Configuration'($rsc))));

    // assume Implies(chain_status::$is_operating(), exists<timestamp::CurrentTimeMicroseconds>(1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+378
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.spec.move:4:9+93
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> $ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory, 1));

    // assume Implies(chain_status::$is_operating(), Ge(timestamp::spec_now_microseconds(), reconfiguration::$last_reconfiguration_time())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+378
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.spec.move:8:9+137
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> ($1_timestamp_spec_now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory) >= $1_reconfiguration_$last_reconfiguration_time($1_reconfiguration_Configuration_$memory)));

    // assume Identical($t7, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t7 == $1_signer_$address_of($t0));

    // assume Identical($t8, Mul($t3, $t4)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t8 == ($t3 * $t4));

    // @34 := save_mem(chain_id::ChainId) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$at(2,4443,4444)"} true;
    $1_chain_id_ChainId_$memory#34 := $1_chain_id_ChainId_$memory;

    // @35 := save_mem(account::Account) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    $1_account_Account_$memory#35 := $1_account_Account_$memory;

    // @36 := save_mem(coin::CoinStore<aptos_coin::AptosCoin>) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#36 := $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory;

    // @33 := save_mem(timestamp::CurrentTimeMicroseconds) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    $1_timestamp_CurrentTimeMicroseconds_$memory#33 := $1_timestamp_CurrentTimeMicroseconds_$memory;

    // trace_local[sender]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$track_local(59,2,0):", $t0} $t0 == $t0;

    // trace_local[txn_sequence_number]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$track_local(59,2,1):", $t1} $t1 == $t1;

    // trace_local[txn_public_key]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$track_local(59,2,2):", $t2} $t2 == $t2;

    // trace_local[txn_gas_price]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$track_local(59,2,3):", $t3} $t3 == $t3;

    // trace_local[txn_max_gas_units]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$track_local(59,2,4):", $t4} $t4 == $t4;

    // trace_local[txn_expiration_time]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$track_local(59,2,5):", $t5} $t5 == $t5;

    // trace_local[chain_id]($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:115:5+1
    assume {:print "$track_local(59,2,6):", $t6} $t6 == $t6;

    // assume Identical($t9, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t9 == $1_signer_$address_of($t0));

    // assume Identical($t10, Mul($t3, $t4)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t10 == ($t3 * $t4));

    // transaction_validation::prologue_common($t0, $t1, $t2, $t3, $t4, $t5, $t6) on_abort goto L2 with $t11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:124:9+125
    assume {:print "$at(2,4690,4815)"} true;
    call $1_transaction_validation_prologue_common($t0, $t1, $t2, $t3, $t4, $t5, $t6);
    if ($abort_flag) {
        assume {:print "$at(2,4690,4815)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(59,2):", $t11} $t11 == $t11;
        goto L2;
    }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:125:5+1
    assume {:print "$at(2,4820,4821)"} true;
L1:

    // assert Not(Not(exists[@33]<timestamp::CurrentTimeMicroseconds>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:37:9+61
    assume {:print "$at(148,1284,1345)"} true;
    assert {:msg "assert_failed(148,1284,1345): function does not abort under this condition"}
      !!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#33, 1);

    // assert Not(Not(Lt(timestamp::$now_seconds[@33](), $t5))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:38:9+60
    assume {:print "$at(148,1354,1414)"} true;
    assert {:msg "assert_failed(148,1354,1414): function does not abort under this condition"}
      !!($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#33) < $t5);

    // assert Not(Not(exists[@34]<chain_id::ChainId>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:40:9+45
    assume {:print "$at(148,1424,1469)"} true;
    assert {:msg "assert_failed(148,1424,1469): function does not abort under this condition"}
      !!$ResourceExists($1_chain_id_ChainId_$memory#34, 1);

    // assert Not(Not(Eq<u8>(chain_id::$get[@34](), $t6))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:41:9+41
    assume {:print "$at(148,1478,1519)"} true;
    assert {:msg "assert_failed(148,1478,1519): function does not abort under this condition"}
      !!$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#34), $t6);

    // assert Not(Not(account::$exists_at[@35]($t7))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:43:9+50
    assume {:print "$at(148,1589,1639)"} true;
    assert {:msg "assert_failed(148,1589,1639): function does not abort under this condition"}
      !!$1_account_$exists_at($1_account_Account_$memory#35, $t7);

    // assert Not(Not(Ge($t1, select account::Account.sequence_number(global[@35]<account::Account>($t7))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:44:9+88
    assume {:print "$at(148,1648,1736)"} true;
    assert {:msg "assert_failed(148,1648,1736): function does not abort under this condition"}
      !!($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#35, $t7)));

    // assert Not(Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@35]<account::Account>($t7))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:45:9+94
    assume {:print "$at(148,1745,1839)"} true;
    assert {:msg "assert_failed(148,1745,1839): function does not abort under this condition"}
      !!$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#35, $t7)));

    // assert Not(Not(Lt($t1, 18446744073709551615))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:46:9+43
    assume {:print "$at(148,1848,1891)"} true;
    assert {:msg "assert_failed(148,1848,1891): function does not abort under this condition"}
      !!($t1 < 18446744073709551615);

    // assert Not(Gt($t8, 18446744073709551615)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:49:9+40
    assume {:print "$at(148,1970,2010)"} true;
    assert {:msg "assert_failed(148,1970,2010): function does not abort under this condition"}
      !($t8 > 18446744073709551615);

    // assert Not(Not(Eq<u64>($t1, select account::Account.sequence_number(global[@35]<account::Account>($t7))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:50:9+88
    assume {:print "$at(148,2019,2107)"} true;
    assert {:msg "assert_failed(148,2019,2107): function does not abort under this condition"}
      !!$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#35, $t7)));

    // assert Not(Not(exists[@36]<coin::CoinStore<aptos_coin::AptosCoin>>($t7))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:51:9+60
    assume {:print "$at(148,2116,2176)"} true;
    assert {:msg "assert_failed(148,2116,2176): function does not abort under this condition"}
      !!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#36, $t7);

    // assert Not(Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@36]<coin::CoinStore<aptos_coin::AptosCoin>>($t7))), $t8))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:52:9+96
    assume {:print "$at(148,2185,2281)"} true;
    assert {:msg "assert_failed(148,2185,2281): function does not abort under this condition"}
      !!($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#36, $t7))) >= $t8);

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:52:9+96
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:125:5+1
    assume {:print "$at(2,4820,4821)"} true;
L2:

    // assert Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Not(exists[@33]<timestamp::CurrentTimeMicroseconds>(1)), Not(Lt(timestamp::$now_seconds[@33](), $t5))), Not(exists[@34]<chain_id::ChainId>(1))), Not(Eq<u8>(chain_id::$get[@34](), $t6))), Not(account::$exists_at[@35]($t7))), Not(Ge($t1, select account::Account.sequence_number(global[@35]<account::Account>($t7))))), Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@35]<account::Account>($t7))))), Not(Lt($t1, 18446744073709551615))), Gt($t8, 18446744073709551615)), Not(Eq<u64>($t1, select account::Account.sequence_number(global[@35]<account::Account>($t7))))), Not(exists[@36]<coin::CoinStore<aptos_coin::AptosCoin>>($t7))), Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@36]<coin::CoinStore<aptos_coin::AptosCoin>>($t7))), $t8))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:67:5+348
    assume {:print "$at(148,2592,2940)"} true;
    assert {:msg "assert_failed(148,2592,2940): abort not covered by any of the `aborts_if` clauses"}
      (((((((((((!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#33, 1) || !($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#33) < $t5)) || !$ResourceExists($1_chain_id_ChainId_$memory#34, 1)) || !$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#34), $t6)) || !$1_account_$exists_at($1_account_Account_$memory#35, $t7)) || !($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#35, $t7)))) || !$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#35, $t7)))) || !($t1 < 18446744073709551615)) || ($t8 > 18446744073709551615)) || !$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#35, $t7)))) || !$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#36, $t7)) || !($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#36, $t7))) >= $t8));

    // abort($t11) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:67:5+348
    $abort_code := $t11;
    $abort_flag := true;
    return;

}

// fun transaction_validation::multi_agent_script_prologue [verification] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1754
procedure {:timeLimit 40} $1_transaction_validation_multi_agent_script_prologue$verify(_$t0: $signer, _$t1: int, _$t2: Vec (int), _$t3: Vec (int), _$t4: Vec (Vec (int)), _$t5: int, _$t6: int, _$t7: int, _$t8: int) returns ()
{
    // declare local variables
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: int;
    var $t13: int;
    var $t14: int;
    var $t15: int;
    var $t16: int;
    var $t17: int;
    var $t18: int;
    var $t19: int;
    var $t20: bool;
    var $t21: int;
    var $t22: int;
    var $t23: int;
    var $t24: bool;
    var $t25: int;
    var $t26: bool;
    var $t27: Vec (int);
    var $t28: Vec (int);
    var $t29: bool;
    var $t30: int;
    var $t31: int;
    var $t32: int;
    var $t33: int;
    var $t34: int;
    var $t35: int;
    var $t0: $signer;
    var $t1: int;
    var $t2: Vec (int);
    var $t3: Vec (int);
    var $t4: Vec (Vec (int));
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $temp_0'u8': int;
    var $temp_0'vec'address'': Vec (int);
    var $temp_0'vec'u8'': Vec (int);
    var $temp_0'vec'vec'u8''': Vec (Vec (int));
    var $1_timestamp_CurrentTimeMicroseconds_$memory#37: $Memory $1_timestamp_CurrentTimeMicroseconds;
    var $1_chain_id_ChainId_$memory#38: $Memory $1_chain_id_ChainId;
    var $1_account_Account_$memory#39: $Memory $1_account_Account;
    var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#40: $Memory $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;
    $t4 := _$t4;
    $t5 := _$t5;
    $t6 := _$t6;
    $t7 := _$t7;
    $t8 := _$t8;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$at(2,5245,5246)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'u64'($t1);

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'vec'u8''($t2);

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'vec'address''($t3);

    // assume WellFormed($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'vec'vec'u8'''($t4);

    // assume WellFormed($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'u64'($t5);

    // assume WellFormed($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'u64'($t6);

    // assume WellFormed($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'u64'($t7);

    // assume WellFormed($t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume $IsValid'u8'($t8);

    // assume forall $rsc: ResourceDomain<chain_id::ChainId>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_id_ChainId_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_id_ChainId_$memory, $a_0);
    ($IsValid'$1_chain_id_ChainId'($rsc))));

    // assume forall $rsc: ResourceDomain<account::Account>(): And(WellFormed($rsc), And(Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.rotation_capability_offer($rsc)))), 1), Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.signer_capability_offer($rsc)))), 1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_account_Account_$memory, $a_0)}(var $rsc := $ResourceValue($1_account_Account_$memory, $a_0);
    (($IsValid'$1_account_Account'($rsc) && ((LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_RotationCapability'($rotation_capability_offer#$1_account_Account($rsc)))) <= 1) && (LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_SignerCapability'($signer_capability_offer#$1_account_Account($rsc)))) <= 1))))));

    // assume forall $rsc: ResourceDomain<coin::CoinStore<aptos_coin::AptosCoin>>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0)}(var $rsc := $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0);
    ($IsValid'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''($rsc))));

    // assume forall $rsc: ResourceDomain<chain_status::GenesisEndMarker>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0);
    ($IsValid'$1_chain_status_GenesisEndMarker'($rsc))));

    // assume forall $rsc: ResourceDomain<timestamp::CurrentTimeMicroseconds>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0)}(var $rsc := $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0);
    ($IsValid'$1_timestamp_CurrentTimeMicroseconds'($rsc))));

    // assume forall $rsc: ResourceDomain<reconfiguration::Configuration>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_reconfiguration_Configuration_$memory, $a_0)}(var $rsc := $ResourceValue($1_reconfiguration_Configuration_$memory, $a_0);
    ($IsValid'$1_reconfiguration_Configuration'($rsc))));

    // assume Implies(chain_status::$is_operating(), exists<timestamp::CurrentTimeMicroseconds>(1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1754
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.spec.move:4:9+93
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> $ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory, 1));

    // assume Implies(chain_status::$is_operating(), Ge(timestamp::spec_now_microseconds(), reconfiguration::$last_reconfiguration_time())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1754
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.spec.move:8:9+137
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> ($1_timestamp_spec_now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory) >= $1_reconfiguration_$last_reconfiguration_time($1_reconfiguration_Configuration_$memory)));

    // assume Identical($t12, Len<address>($t3)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:114:9+60
    assume {:print "$at(148,3990,4050)"} true;
    assume ($t12 == LenVec($t3));

    // assume Identical($t13, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t13 == $1_signer_$address_of($t0));

    // assume Identical($t14, Mul($t5, $t6)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t14 == ($t5 * $t6));

    // @38 := save_mem(chain_id::ChainId) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$at(2,5245,5246)"} true;
    $1_chain_id_ChainId_$memory#38 := $1_chain_id_ChainId_$memory;

    // @39 := save_mem(account::Account) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    $1_account_Account_$memory#39 := $1_account_Account_$memory;

    // @40 := save_mem(coin::CoinStore<aptos_coin::AptosCoin>) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#40 := $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory;

    // @37 := save_mem(timestamp::CurrentTimeMicroseconds) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    $1_timestamp_CurrentTimeMicroseconds_$memory#37 := $1_timestamp_CurrentTimeMicroseconds_$memory;

    // trace_local[sender]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,0):", $t0} $t0 == $t0;

    // trace_local[txn_sequence_number]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,1):", $t1} $t1 == $t1;

    // trace_local[txn_sender_public_key]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,2):", $t2} $t2 == $t2;

    // trace_local[secondary_signer_addresses]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,3):", $t3} $t3 == $t3;

    // trace_local[secondary_signer_public_key_hashes]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,4):", $t4} $t4 == $t4;

    // trace_local[txn_gas_price]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,5):", $t5} $t5 == $t5;

    // trace_local[txn_max_gas_units]($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,6):", $t6} $t6 == $t6;

    // trace_local[txn_expiration_time]($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,7):", $t7} $t7 == $t7;

    // trace_local[chain_id]($t8) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:140:5+1
    assume {:print "$track_local(59,3,8):", $t8} $t8 == $t8;

    // assume Identical($t15, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t15 == $1_signer_$address_of($t0));

    // assume Identical($t16, Mul($t5, $t6)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t16 == ($t5 * $t6));

    // transaction_validation::prologue_common($t0, $t1, $t2, $t5, $t6, $t7, $t8) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:151:9+132
    assume {:print "$at(2,5628,5760)"} true;
    call $1_transaction_validation_prologue_common($t0, $t1, $t2, $t5, $t6, $t7, $t8);
    if ($abort_flag) {
        assume {:print "$at(2,5628,5760)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // $t18 := vector::length<address>($t3) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:153:37+43
    assume {:print "$at(2,5799,5842)"} true;
    call $t18 := $1_vector_length'address'($t3);
    if ($abort_flag) {
        assume {:print "$at(2,5799,5842)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // trace_local[num_secondary_signers]($t18) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:153:13+21
    assume {:print "$track_local(59,3,10):", $t18} $t18 == $t18;

    // $t19 := vector::length<vector<u8>>($t4) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:156:13+51
    assume {:print "$at(2,5874,5925)"} true;
    call $t19 := $1_vector_length'vec'u8''($t4);
    if ($abort_flag) {
        assume {:print "$at(2,5874,5925)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // $t20 := ==($t19, $t18) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:156:65+2
    $t20 := $IsEqual'u64'($t19, $t18);

    // if ($t20) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:155:9+196
    assume {:print "$at(2,5853,6049)"} true;
    if ($t20) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:155:9+196
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:155:9+196
    assume {:print "$at(2,5853,6049)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:157:37+49
    assume {:print "$at(2,5988,6037)"} true;
L0:

    // $t21 := 1009 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:157:37+49
    assume {:print "$at(2,5988,6037)"} true;
    $t21 := 1009;
    assume $IsValid'u64'($t21);

    // $t22 := error::invalid_argument($t21) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:157:13+74
    call $t22 := $1_error_invalid_argument($t21);
    if ($abort_flag) {
        assume {:print "$at(2,5964,6038)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // trace_abort($t22) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:155:9+196
    assume {:print "$at(2,5853,6049)"} true;
    assume {:print "$track_abort(59,3):", $t22} $t22 == $t22;

    // $t17 := move($t22) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:155:9+196
    $t17 := $t22;

    // goto L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:155:9+196
    goto L15;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:160:17+1
    assume {:print "$at(2,6068,6069)"} true;
L2:

    // $t23 := 0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:160:17+1
    assume {:print "$at(2,6068,6069)"} true;
    $t23 := 0;
    assume $IsValid'u64'($t23);

    // trace_local[i]($t23) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:160:13+1
    assume {:print "$track_local(59,3,9):", $t23} $t23 == $t23;

    // label L12 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:162:13+296
    assume {:print "$at(2,6100,6396)"} true;
L12:

    // assert Le($t23, $t18) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:163:17+37
    assume {:print "$at(2,6123,6160)"} true;
    assert {:msg "assert_failed(2,6123,6160): base case of the loop invariant does not hold"}
      ($t23 <= $t18);

    // assert forall j: Range(0, $t23): And(account::$exists_at(Index($t3, j)), Eq<vector<u8>>(Index($t4, j), account::$get_authentication_key(Index($t3, j)))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume {:print "$at(2,6177,6382)"} true;
    assert {:msg "assert_failed(2,6177,6382): base case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t23); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var j := $i_1;
    (($1_account_$exists_at($1_account_Account_$memory, ReadVec($t3, j)) && $IsEqual'vec'u8''(ReadVec($t4, j), $1_account_$get_authentication_key($1_account_Account_$memory, ReadVec($t3, j))))))));

    // $t9 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t9;

    // assume WellFormed($t9) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'u64'($t9);

    // $t24 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t24;

    // assume WellFormed($t24) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'bool'($t24);

    // $t25 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t25;

    // assume WellFormed($t25) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'address'($t25);

    // $t26 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t26;

    // assume WellFormed($t26) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'bool'($t26);

    // $t27 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t27;

    // assume WellFormed($t27) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'vec'u8''($t27);

    // $t28 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t28;

    // assume WellFormed($t28) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'vec'u8''($t28);

    // $t29 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t29;

    // assume WellFormed($t29) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'bool'($t29);

    // $t30 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t30;

    // assume WellFormed($t30) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'u64'($t30);

    // $t31 := havoc[val]() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    havoc $t31;

    // assume WellFormed($t31) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume $IsValid'u64'($t31);

    // trace_local[i]($t9) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume {:print "$info(): enter loop, variable(s) i havocked and reassigned"} true;
    assume {:print "$track_local(59,3,9):", $t9} $t9 == $t9;

    // assume Not(AbortFlag()) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume {:print "$info(): loop invariant holds at current state"} true;
    assume !$abort_flag;

    // assume Le($t9, $t18) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:163:17+37
    assume {:print "$at(2,6123,6160)"} true;
    assume ($t9 <= $t18);

    // assume forall j: Range(0, $t9): And(account::$exists_at(Index($t3, j)), Eq<vector<u8>>(Index($t4, j), account::$get_authentication_key(Index($t3, j)))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume {:print "$at(2,6177,6382)"} true;
    assume (var $range_0 := $Range(0, $t9); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var j := $i_1;
    (($1_account_$exists_at($1_account_Account_$memory, ReadVec($t3, j)) && $IsEqual'vec'u8''(ReadVec($t4, j), $1_account_$get_authentication_key($1_account_Account_$memory, ReadVec($t3, j))))))));

    // $t24 := <($t9, $t18) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:167:16+1
    assume {:print "$at(2,6413,6414)"} true;
    call $t24 := $Lt($t9, $t18);

    // if ($t24) goto L4 else goto L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:161:9+914
    assume {:print "$at(2,6079,6993)"} true;
    if ($t24) { goto L4; } else { goto L3; }

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:161:9+914
L4:

    // label L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:169:53+27
    assume {:print "$at(2,6503,6530)"} true;
L5:

    // $t25 := vector::borrow<address>($t3, $t9) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:169:38+46
    assume {:print "$at(2,6488,6534)"} true;
    call $t25 := $1_vector_borrow'address'($t3, $t9);
    if ($abort_flag) {
        assume {:print "$at(2,6488,6534)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // trace_local[secondary_address]($t25) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:169:17+17
    assume {:print "$track_local(59,3,11):", $t25} $t25 == $t25;

    // $t26 := account::exists_at($t25) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:21+37
    assume {:print "$at(2,6556,6593)"} true;
    call $t26 := $1_account_exists_at($t25);
    if ($abort_flag) {
        assume {:print "$at(2,6556,6593)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // if ($t26) goto L7 else goto L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:13+105
    if ($t26) { goto L7; } else { goto L6; }

    // label L7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:13+105
L7:

    // goto L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:13+105
    assume {:print "$at(2,6548,6653)"} true;
    goto L8;

    // label L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:84+32
L6:

    // $t32 := 1004 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:84+32
    assume {:print "$at(2,6619,6651)"} true;
    $t32 := 1004;
    assume $IsValid'u64'($t32);

    // $t33 := error::invalid_argument($t32) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:60+57
    call $t33 := $1_error_invalid_argument($t32);
    if ($abort_flag) {
        assume {:print "$at(2,6595,6652)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // trace_abort($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:13+105
    assume {:print "$at(2,6548,6653)"} true;
    assume {:print "$track_abort(59,3):", $t33} $t33 == $t33;

    // $t17 := move($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:13+105
    $t17 := $t33;

    // goto L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:170:13+105
    goto L15;

    // label L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:172:58+35
    assume {:print "$at(2,6713,6748)"} true;
L8:

    // $t27 := vector::borrow<vector<u8>>($t4, $t9) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:172:43+54
    assume {:print "$at(2,6698,6752)"} true;
    call $t27 := $1_vector_borrow'vec'u8''($t4, $t9);
    if ($abort_flag) {
        assume {:print "$at(2,6698,6752)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // $t28 := account::get_authentication_key($t25) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:174:43+50
    assume {:print "$at(2,6817,6867)"} true;
    call $t28 := $1_account_get_authentication_key($t25);
    if ($abort_flag) {
        assume {:print "$at(2,6817,6867)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // $t29 := ==($t27, $t28) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:174:40+2
    $t29 := $IsEqual'vec'u8''($t27, $t28);

    // if ($t29) goto L10 else goto L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:173:13+193
    assume {:print "$at(2,6766,6959)"} true;
    if ($t29) { goto L10; } else { goto L9; }

    // label L10 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:173:13+193
L10:

    // goto L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:173:13+193
    assume {:print "$at(2,6766,6959)"} true;
    goto L11;

    // label L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:175:41+34
    assume {:print "$at(2,6909,6943)"} true;
L9:

    // $t34 := 1001 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:175:41+34
    assume {:print "$at(2,6909,6943)"} true;
    $t34 := 1001;
    assume $IsValid'u64'($t34);

    // $t35 := error::invalid_argument($t34) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:175:17+59
    call $t35 := $1_error_invalid_argument($t34);
    if ($abort_flag) {
        assume {:print "$at(2,6885,6944)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // trace_abort($t35) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:173:13+193
    assume {:print "$at(2,6766,6959)"} true;
    assume {:print "$track_abort(59,3):", $t35} $t35 == $t35;

    // $t17 := move($t35) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:173:13+193
    $t17 := $t35;

    // goto L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:173:13+193
    goto L15;

    // label L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:177:17+1
    assume {:print "$at(2,6977,6978)"} true;
L11:

    // $t30 := 1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:177:21+1
    assume {:print "$at(2,6981,6982)"} true;
    $t30 := 1;
    assume $IsValid'u64'($t30);

    // $t31 := +($t9, $t30) on_abort goto L15 with $t17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:177:19+1
    call $t31 := $AddU64($t9, $t30);
    if ($abort_flag) {
        assume {:print "$at(2,6979,6980)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(59,3):", $t17} $t17 == $t17;
        goto L15;
    }

    // trace_local[i]($t31) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:177:13+1
    assume {:print "$track_local(59,3,9):", $t31} $t31 == $t31;

    // goto L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:177:22+1
    goto L13;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:161:9+914
    assume {:print "$at(2,6079,6993)"} true;
L3:

    // goto L14 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:161:9+914
    assume {:print "$at(2,6079,6993)"} true;
    goto L14;

    // label L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:161:9+914
    // Loop invariant checking block for the loop started with header: L12
L13:

    // assert Le($t31, $t18) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:163:17+37
    assume {:print "$at(2,6123,6160)"} true;
    assert {:msg "assert_failed(2,6123,6160): induction case of the loop invariant does not hold"}
      ($t31 <= $t18);

    // assert forall j: Range(0, $t31): And(account::$exists_at(Index($t3, j)), Eq<vector<u8>>(Index($t4, j), account::$get_authentication_key(Index($t3, j)))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume {:print "$at(2,6177,6382)"} true;
    assert {:msg "assert_failed(2,6177,6382): induction case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t31); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var j := $i_1;
    (($1_account_$exists_at($1_account_Account_$memory, ReadVec($t3, j)) && $IsEqual'vec'u8''(ReadVec($t4, j), $1_account_$get_authentication_key($1_account_Account_$memory, ReadVec($t3, j))))))));

    // stop() at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:164:17+205
    assume false;
    return;

    // label L14 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:179:5+1
    assume {:print "$at(2,6998,6999)"} true;
L14:

    // assert Not(Not(exists[@37]<timestamp::CurrentTimeMicroseconds>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:37:9+61
    assume {:print "$at(148,1284,1345)"} true;
    assert {:msg "assert_failed(148,1284,1345): function does not abort under this condition"}
      !!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#37, 1);

    // assert Not(Not(Lt(timestamp::$now_seconds[@37](), $t7))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:38:9+60
    assume {:print "$at(148,1354,1414)"} true;
    assert {:msg "assert_failed(148,1354,1414): function does not abort under this condition"}
      !!($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#37) < $t7);

    // assert Not(Not(exists[@38]<chain_id::ChainId>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:40:9+45
    assume {:print "$at(148,1424,1469)"} true;
    assert {:msg "assert_failed(148,1424,1469): function does not abort under this condition"}
      !!$ResourceExists($1_chain_id_ChainId_$memory#38, 1);

    // assert Not(Not(Eq<u8>(chain_id::$get[@38](), $t8))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:41:9+41
    assume {:print "$at(148,1478,1519)"} true;
    assert {:msg "assert_failed(148,1478,1519): function does not abort under this condition"}
      !!$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#38), $t8);

    // assert Not(Not(account::$exists_at[@39]($t13))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:43:9+50
    assume {:print "$at(148,1589,1639)"} true;
    assert {:msg "assert_failed(148,1589,1639): function does not abort under this condition"}
      !!$1_account_$exists_at($1_account_Account_$memory#39, $t13);

    // assert Not(Not(Ge($t1, select account::Account.sequence_number(global[@39]<account::Account>($t13))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:44:9+88
    assume {:print "$at(148,1648,1736)"} true;
    assert {:msg "assert_failed(148,1648,1736): function does not abort under this condition"}
      !!($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#39, $t13)));

    // assert Not(Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@39]<account::Account>($t13))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:45:9+94
    assume {:print "$at(148,1745,1839)"} true;
    assert {:msg "assert_failed(148,1745,1839): function does not abort under this condition"}
      !!$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#39, $t13)));

    // assert Not(Not(Lt($t1, 18446744073709551615))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:46:9+43
    assume {:print "$at(148,1848,1891)"} true;
    assert {:msg "assert_failed(148,1848,1891): function does not abort under this condition"}
      !!($t1 < 18446744073709551615);

    // assert Not(Gt($t14, 18446744073709551615)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:49:9+40
    assume {:print "$at(148,1970,2010)"} true;
    assert {:msg "assert_failed(148,1970,2010): function does not abort under this condition"}
      !($t14 > 18446744073709551615);

    // assert Not(Not(Eq<u64>($t1, select account::Account.sequence_number(global[@39]<account::Account>($t13))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:50:9+88
    assume {:print "$at(148,2019,2107)"} true;
    assert {:msg "assert_failed(148,2019,2107): function does not abort under this condition"}
      !!$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#39, $t13)));

    // assert Not(Not(exists[@40]<coin::CoinStore<aptos_coin::AptosCoin>>($t13))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:51:9+60
    assume {:print "$at(148,2116,2176)"} true;
    assert {:msg "assert_failed(148,2116,2176): function does not abort under this condition"}
      !!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#40, $t13);

    // assert Not(Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@40]<coin::CoinStore<aptos_coin::AptosCoin>>($t13))), $t14))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:52:9+96
    assume {:print "$at(148,2185,2281)"} true;
    assert {:msg "assert_failed(148,2185,2281): function does not abort under this condition"}
      !!($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#40, $t13))) >= $t14);

    // assert Not(Neq<num>(Len<vector<u8>>($t4), $t12)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:115:9+75
    assume {:print "$at(148,4059,4134)"} true;
    assert {:msg "assert_failed(148,4059,4134): function does not abort under this condition"}
      !!$IsEqual'num'(LenVec($t4), $t12);

    // assert Not(exists i: Range(0, $t12): Or(Not(account::$exists_at[@39](Index($t3, i))), Neq<vector<u8>>(Index($t4, i), account::$get_authentication_key[@39](Index($t3, i))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:118:9+254
    assume {:print "$at(148,4228,4482)"} true;
    assert {:msg "assert_failed(148,4228,4482): function does not abort under this condition"}
      !(var $range_0 := $Range(0, $t12); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$1_account_$exists_at($1_account_Account_$memory#39, ReadVec($t3, i)) || !$IsEqual'vec'u8''(ReadVec($t4, i), $1_account_$get_authentication_key($1_account_Account_$memory#39, ReadVec($t3, i))))))));

    // assert forall i: Range(0, $t12): And(account::$exists_at(Index($t3, i)), Eq<vector<u8>>(Index($t4, i), account::$get_authentication_key(Index($t3, i)))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:124:9+251
    assume {:print "$at(148,4592,4843)"} true;
    assert {:msg "assert_failed(148,4592,4843): post-condition does not hold"}
      (var $range_0 := $Range(0, $t12); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var i := $i_1;
    (($1_account_$exists_at($1_account_Account_$memory, ReadVec($t3, i)) && $IsEqual'vec'u8''(ReadVec($t4, i), $1_account_$get_authentication_key($1_account_Account_$memory, ReadVec($t3, i))))))));

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:124:9+251
    return;

    // label L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:179:5+1
    assume {:print "$at(2,6998,6999)"} true;
L15:

    // assert Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Not(exists[@37]<timestamp::CurrentTimeMicroseconds>(1)), Not(Lt(timestamp::$now_seconds[@37](), $t7))), Not(exists[@38]<chain_id::ChainId>(1))), Not(Eq<u8>(chain_id::$get[@38](), $t8))), Not(account::$exists_at[@39]($t13))), Not(Ge($t1, select account::Account.sequence_number(global[@39]<account::Account>($t13))))), Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@39]<account::Account>($t13))))), Not(Lt($t1, 18446744073709551615))), Gt($t14, 18446744073709551615)), Not(Eq<u64>($t1, select account::Account.sequence_number(global[@39]<account::Account>($t13))))), Not(exists[@40]<coin::CoinStore<aptos_coin::AptosCoin>>($t13))), Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@40]<coin::CoinStore<aptos_coin::AptosCoin>>($t13))), $t14))), Neq<num>(Len<vector<u8>>($t4), $t12)), exists i: Range(0, $t12): Or(Not(account::$exists_at[@39](Index($t3, i))), Neq<vector<u8>>(Index($t4, i), account::$get_authentication_key[@39](Index($t3, i))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:98:5+1421
    assume {:print "$at(148,3428,4849)"} true;
    assert {:msg "assert_failed(148,3428,4849): abort not covered by any of the `aborts_if` clauses"}
      (((((((((((((!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#37, 1) || !($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#37) < $t7)) || !$ResourceExists($1_chain_id_ChainId_$memory#38, 1)) || !$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#38), $t8)) || !$1_account_$exists_at($1_account_Account_$memory#39, $t13)) || !($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#39, $t13)))) || !$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#39, $t13)))) || !($t1 < 18446744073709551615)) || ($t14 > 18446744073709551615)) || !$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#39, $t13)))) || !$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#40, $t13)) || !($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#40, $t13))) >= $t14)) || !$IsEqual'num'(LenVec($t4), $t12)) || (var $range_0 := $Range(0, $t12); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$1_account_$exists_at($1_account_Account_$memory#39, ReadVec($t3, i)) || !$IsEqual'vec'u8''(ReadVec($t4, i), $1_account_$get_authentication_key($1_account_Account_$memory#39, ReadVec($t3, i)))))))));

    // abort($t17) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:98:5+1421
    $abort_code := $t17;
    $abort_flag := true;
    return;

}

// fun transaction_validation::prologue_common [baseline] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+2006
procedure {:inline 1} $1_transaction_validation_prologue_common(_$t0: $signer, _$t1: int, _$t2: Vec (int), _$t3: int, _$t4: int, _$t5: int, _$t6: int) returns ()
{
    // declare local variables
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: int;
    var $t13: int;
    var $t14: bool;
    var $t15: int;
    var $t16: int;
    var $t17: int;
    var $t18: bool;
    var $t19: int;
    var $t20: int;
    var $t21: int;
    var $t22: bool;
    var $t23: int;
    var $t24: int;
    var $t25: Vec (int);
    var $t26: bool;
    var $t27: int;
    var $t28: int;
    var $t29: int;
    var $t30: int;
    var $t31: bool;
    var $t32: int;
    var $t33: int;
    var $t34: int;
    var $t35: bool;
    var $t36: int;
    var $t37: int;
    var $t38: bool;
    var $t39: int;
    var $t40: int;
    var $t41: int;
    var $t42: bool;
    var $t43: int;
    var $t44: int;
    var $t45: int;
    var $t46: bool;
    var $t47: int;
    var $t48: int;
    var $t0: $signer;
    var $t1: int;
    var $t2: Vec (int);
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $temp_0'u8': int;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;
    $t4 := _$t4;
    $t5 := _$t5;
    $t6 := _$t6;

    // bytecode translation starts here
    // assume Identical($t10, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t10 == $1_signer_$address_of($t0));

    // assume Identical($t11, Mul($t3, $t4)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t11 == ($t3 * $t4));

    // trace_local[sender]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$at(2,2431,2432)"} true;
    assume {:print "$track_local(59,4,0):", $t0} $t0 == $t0;

    // trace_local[txn_sequence_number]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,1):", $t1} $t1 == $t1;

    // trace_local[txn_authentication_key]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,2):", $t2} $t2 == $t2;

    // trace_local[txn_gas_price]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,3):", $t3} $t3 == $t3;

    // trace_local[txn_max_gas_units]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,4):", $t4} $t4 == $t4;

    // trace_local[txn_expiration_time]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,5):", $t5} $t5 == $t5;

    // trace_local[chain_id]($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,6):", $t6} $t6 == $t6;

    // $t12 := timestamp::now_seconds() on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:76:13+24
    assume {:print "$at(2,2707,2731)"} true;
    call $t12 := $1_timestamp_now_seconds();
    if ($abort_flag) {
        assume {:print "$at(2,2707,2731)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t14 := <($t12, $t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:76:38+1
    call $t14 := $Lt($t12, $t5);

    // if ($t14) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    assume {:print "$at(2,2686,2832)"} true;
    if ($t14) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    assume {:print "$at(2,2686,2832)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:77:37+29
    assume {:print "$at(2,2791,2820)"} true;
L0:

    // $t15 := 1006 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:77:37+29
    assume {:print "$at(2,2791,2820)"} true;
    $t15 := 1006;
    assume $IsValid'u64'($t15);

    // $t16 := error::invalid_argument($t15) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:77:13+54
    call $t16 := $1_error_invalid_argument($t15);
    if ($abort_flag) {
        assume {:print "$at(2,2767,2821)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    assume {:print "$at(2,2686,2832)"} true;
    assume {:print "$track_abort(59,4):", $t16} $t16 == $t16;

    // $t13 := move($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    $t13 := $t16;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    goto L28;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:17+15
    assume {:print "$at(2,2850,2865)"} true;
L2:

    // $t17 := chain_id::get() on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:17+15
    assume {:print "$at(2,2850,2865)"} true;
    call $t17 := $1_chain_id_get();
    if ($abort_flag) {
        assume {:print "$at(2,2850,2865)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t18 := ==($t17, $t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:33+2
    $t18 := $IsEqual'u8'($t17, $t6);

    // if ($t18) goto L4 else goto L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    if ($t18) { goto L4; } else { goto L3; }

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
L4:

    // goto L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    assume {:print "$at(2,2842,2927)"} true;
    goto L5;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:70+22
L3:

    // $t19 := 1007 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:70+22
    assume {:print "$at(2,2903,2925)"} true;
    $t19 := 1007;
    assume $IsValid'u64'($t19);

    // $t20 := error::invalid_argument($t19) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:46+47
    call $t20 := $1_error_invalid_argument($t19);
    if ($abort_flag) {
        assume {:print "$at(2,2879,2926)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t20) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    assume {:print "$at(2,2842,2927)"} true;
    assume {:print "$track_abort(59,4):", $t20} $t20 == $t20;

    // $t13 := move($t20) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    $t13 := $t20;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    goto L28;

    // label L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:81:53+7
    assume {:print "$at(2,2982,2989)"} true;
L5:

    // $t21 := signer::address_of($t0) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:81:34+27
    assume {:print "$at(2,2963,2990)"} true;
    call $t21 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,2963,2990)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_local[transaction_sender]($t21) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:81:13+18
    assume {:print "$track_local(59,4,9):", $t21} $t21 == $t21;

    // $t22 := account::exists_at($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:17+38
    assume {:print "$at(2,3008,3046)"} true;
    call $t22 := $1_account_exists_at($t21);
    if ($abort_flag) {
        assume {:print "$at(2,3008,3046)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // if ($t22) goto L7 else goto L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    if ($t22) { goto L7; } else { goto L6; }

    // label L7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
L7:

    // goto L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    assume {:print "$at(2,3000,3106)"} true;
    goto L8;

    // label L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:81+32
L6:

    // $t23 := 1004 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:81+32
    assume {:print "$at(2,3072,3104)"} true;
    $t23 := 1004;
    assume $IsValid'u64'($t23);

    // $t24 := error::invalid_argument($t23) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:57+57
    call $t24 := $1_error_invalid_argument($t23);
    if ($abort_flag) {
        assume {:print "$at(2,3048,3105)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t24) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    assume {:print "$at(2,3000,3106)"} true;
    assume {:print "$track_abort(59,4):", $t24} $t24 == $t24;

    // $t13 := move($t24) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    $t13 := $t24;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    goto L28;

    // label L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:84:13+22
    assume {:print "$at(2,3137,3159)"} true;
L8:

    // $t25 := account::get_authentication_key($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:84:39+51
    assume {:print "$at(2,3163,3214)"} true;
    call $t25 := $1_account_get_authentication_key($t21);
    if ($abort_flag) {
        assume {:print "$at(2,3163,3214)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t26 := ==($t2, $t25) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:84:36+2
    $t26 := $IsEqual'vec'u8''($t2, $t25);

    // if ($t26) goto L10 else goto L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    assume {:print "$at(2,3116,3298)"} true;
    if ($t26) { goto L10; } else { goto L9; }

    // label L10 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
L10:

    // goto L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    assume {:print "$at(2,3116,3298)"} true;
    goto L11;

    // label L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:85:37+34
    assume {:print "$at(2,3252,3286)"} true;
L9:

    // $t27 := 1001 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:85:37+34
    assume {:print "$at(2,3252,3286)"} true;
    $t27 := 1001;
    assume $IsValid'u64'($t27);

    // $t28 := error::invalid_argument($t27) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:85:13+59
    call $t28 := $1_error_invalid_argument($t27);
    if ($abort_flag) {
        assume {:print "$at(2,3228,3287)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t28) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    assume {:print "$at(2,3116,3298)"} true;
    assume {:print "$track_abort(59,4):", $t28} $t28 == $t28;

    // $t13 := move($t28) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    $t13 := $t28;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    goto L28;

    // label L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:14+19
    assume {:print "$at(2,3331,3350)"} true;
L11:

    // $t29 := (u128)($t1) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:13+29
    assume {:print "$at(2,3330,3359)"} true;
    call $t29 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,3330,3359)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t30 := 18446744073709551615 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:45+7
    $t30 := 18446744073709551615;
    assume $IsValid'u128'($t30);

    // $t31 := <($t29, $t30) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:43+1
    call $t31 := $Lt($t29, $t30);

    // if ($t31) goto L13 else goto L12 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    assume {:print "$at(2,3309,3447)"} true;
    if ($t31) { goto L13; } else { goto L12; }

    // label L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
L13:

    // goto L14 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    assume {:print "$at(2,3309,3447)"} true;
    goto L14;

    // label L12 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:90:33+33
    assume {:print "$at(2,3403,3436)"} true;
L12:

    // $t32 := 1008 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:90:33+33
    assume {:print "$at(2,3403,3436)"} true;
    $t32 := 1008;
    assume $IsValid'u64'($t32);

    // $t33 := error::out_of_range($t32) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:90:13+54
    call $t33 := $1_error_out_of_range($t32);
    if ($abort_flag) {
        assume {:print "$at(2,3383,3437)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    assume {:print "$at(2,3309,3447)"} true;
    assume {:print "$track_abort(59,4):", $t33} $t33 == $t33;

    // $t13 := move($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    $t13 := $t33;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    goto L28;

    // label L14 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:93:68+18
    assume {:print "$at(2,3517,3535)"} true;
L14:

    // $t34 := account::get_sequence_number($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:93:39+48
    assume {:print "$at(2,3488,3536)"} true;
    call $t34 := $1_account_get_sequence_number($t21);
    if ($abort_flag) {
        assume {:print "$at(2,3488,3536)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_local[account_sequence_number]($t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:93:13+23
    assume {:print "$track_local(59,4,7):", $t34} $t34 == $t34;

    // $t35 := >=($t1, $t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:95:33+2
    assume {:print "$at(2,3587,3589)"} true;
    call $t35 := $Ge($t1, $t34);

    // if ($t35) goto L16 else goto L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    assume {:print "$at(2,3546,3695)"} true;
    if ($t35) { goto L16; } else { goto L15; }

    // label L16 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
L16:

    // goto L17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    assume {:print "$at(2,3546,3695)"} true;
    goto L17;

    // label L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:96:37+33
    assume {:print "$at(2,3651,3684)"} true;
L15:

    // $t36 := 1002 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:96:37+33
    assume {:print "$at(2,3651,3684)"} true;
    $t36 := 1002;
    assume $IsValid'u64'($t36);

    // $t37 := error::invalid_argument($t36) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:96:13+58
    call $t37 := $1_error_invalid_argument($t36);
    if ($abort_flag) {
        assume {:print "$at(2,3627,3685)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t37) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    assume {:print "$at(2,3546,3695)"} true;
    assume {:print "$track_abort(59,4):", $t37} $t37 == $t37;

    // $t13 := move($t37) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    $t13 := $t37;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    goto L28;

    // label L17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:102:13+19
    assume {:print "$at(2,3889,3908)"} true;
L17:

    // $t38 := ==($t1, $t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:102:33+2
    assume {:print "$at(2,3909,3911)"} true;
    $t38 := $IsEqual'u64'($t1, $t34);

    // if ($t38) goto L19 else goto L18 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    assume {:print "$at(2,3868,4017)"} true;
    if ($t38) { goto L19; } else { goto L18; }

    // label L19 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
L19:

    // goto L20 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    assume {:print "$at(2,3868,4017)"} true;
    goto L20;

    // label L18 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:103:37+33
    assume {:print "$at(2,3973,4006)"} true;
L18:

    // $t39 := 1003 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:103:37+33
    assume {:print "$at(2,3973,4006)"} true;
    $t39 := 1003;
    assume $IsValid'u64'($t39);

    // $t40 := error::invalid_argument($t39) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:103:13+58
    call $t40 := $1_error_invalid_argument($t39);
    if ($abort_flag) {
        assume {:print "$at(2,3949,4007)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t40) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    assume {:print "$at(2,3868,4017)"} true;
    assume {:print "$track_abort(59,4):", $t40} $t40 == $t40;

    // $t13 := move($t40) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    $t13 := $t40;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    goto L28;

    // label L20 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:106:35+13
    assume {:print "$at(2,4054,4067)"} true;
L20:

    // $t41 := *($t3, $t4) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:106:49+1
    assume {:print "$at(2,4068,4069)"} true;
    call $t41 := $MulU64($t3, $t4);
    if ($abort_flag) {
        assume {:print "$at(2,4068,4069)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_local[max_transaction_fee]($t41) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:106:13+19
    assume {:print "$track_local(59,4,8):", $t41} $t41 == $t41;

    // $t42 := coin::is_account_registered<aptos_coin::AptosCoin>($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:108:13+58
    assume {:print "$at(2,4118,4176)"} true;
    call $t42 := $1_coin_is_account_registered'$1_aptos_coin_AptosCoin'($t21);
    if ($abort_flag) {
        assume {:print "$at(2,4118,4176)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // if ($t42) goto L22 else goto L21 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    assume {:print "$at(2,4097,4256)"} true;
    if ($t42) { goto L22; } else { goto L21; }

    // label L22 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
L22:

    // goto L23 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    assume {:print "$at(2,4097,4256)"} true;
    goto L23;

    // label L21 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:109:37+30
    assume {:print "$at(2,4214,4244)"} true;
L21:

    // $t43 := 1005 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:109:37+30
    assume {:print "$at(2,4214,4244)"} true;
    $t43 := 1005;
    assume $IsValid'u64'($t43);

    // $t44 := error::invalid_argument($t43) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:109:13+55
    call $t44 := $1_error_invalid_argument($t43);
    if ($abort_flag) {
        assume {:print "$at(2,4190,4245)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t44) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    assume {:print "$at(2,4097,4256)"} true;
    assume {:print "$track_abort(59,4):", $t44} $t44 == $t44;

    // $t13 := move($t44) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    $t13 := $t44;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    goto L28;

    // label L23 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:111:48+18
    assume {:print "$at(2,4305,4323)"} true;
L23:

    // $t45 := coin::balance<aptos_coin::AptosCoin>($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:111:23+44
    assume {:print "$at(2,4280,4324)"} true;
    call $t45 := $1_coin_balance'$1_aptos_coin_AptosCoin'($t21);
    if ($abort_flag) {
        assume {:print "$at(2,4280,4324)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t46 := >=($t45, $t41) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:25+2
    assume {:print "$at(2,4350,4352)"} true;
    call $t46 := $Ge($t45, $t41);

    // if ($t46) goto L25 else goto L24 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    if ($t46) { goto L25; } else { goto L24; }

    // label L25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
L25:

    // goto L26 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    assume {:print "$at(2,4334,4430)"} true;
    goto L26;

    // label L24 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:73+30
L24:

    // $t47 := 1005 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:73+30
    assume {:print "$at(2,4398,4428)"} true;
    $t47 := 1005;
    assume $IsValid'u64'($t47);

    // $t48 := error::invalid_argument($t47) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:49+55
    call $t48 := $1_error_invalid_argument($t47);
    if ($abort_flag) {
        assume {:print "$at(2,4374,4429)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t48) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    assume {:print "$at(2,4334,4430)"} true;
    assume {:print "$track_abort(59,4):", $t48} $t48 == $t48;

    // $t13 := move($t48) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    $t13 := $t48;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    goto L28;

    // label L26 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:105+1
L26:

    // label L27 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:113:5+1
    assume {:print "$at(2,4436,4437)"} true;
L27:

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:113:5+1
    assume {:print "$at(2,4436,4437)"} true;
    return;

    // label L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:113:5+1
L28:

    // abort($t13) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:113:5+1
    assume {:print "$at(2,4436,4437)"} true;
    $abort_code := $t13;
    $abort_flag := true;
    return;

}

// fun transaction_validation::prologue_common [verification] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+2006
procedure {:timeLimit 40} $1_transaction_validation_prologue_common$verify(_$t0: $signer, _$t1: int, _$t2: Vec (int), _$t3: int, _$t4: int, _$t5: int, _$t6: int) returns ()
{
    // declare local variables
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: int;
    var $t13: int;
    var $t14: bool;
    var $t15: int;
    var $t16: int;
    var $t17: int;
    var $t18: bool;
    var $t19: int;
    var $t20: int;
    var $t21: int;
    var $t22: bool;
    var $t23: int;
    var $t24: int;
    var $t25: Vec (int);
    var $t26: bool;
    var $t27: int;
    var $t28: int;
    var $t29: int;
    var $t30: int;
    var $t31: bool;
    var $t32: int;
    var $t33: int;
    var $t34: int;
    var $t35: bool;
    var $t36: int;
    var $t37: int;
    var $t38: bool;
    var $t39: int;
    var $t40: int;
    var $t41: int;
    var $t42: bool;
    var $t43: int;
    var $t44: int;
    var $t45: int;
    var $t46: bool;
    var $t47: int;
    var $t48: int;
    var $t0: $signer;
    var $t1: int;
    var $t2: Vec (int);
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $temp_0'u8': int;
    var $temp_0'vec'u8'': Vec (int);
    var $1_timestamp_CurrentTimeMicroseconds_$memory#19: $Memory $1_timestamp_CurrentTimeMicroseconds;
    var $1_chain_id_ChainId_$memory#20: $Memory $1_chain_id_ChainId;
    var $1_account_Account_$memory#21: $Memory $1_account_Account;
    var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#22: $Memory $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
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
    // assume WellFormed($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$at(2,2431,2432)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume $IsValid'u64'($t1);

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume $IsValid'vec'u8''($t2);

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume $IsValid'u64'($t3);

    // assume WellFormed($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume $IsValid'u64'($t4);

    // assume WellFormed($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume $IsValid'u64'($t5);

    // assume WellFormed($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume $IsValid'u8'($t6);

    // assume forall $rsc: ResourceDomain<chain_id::ChainId>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_id_ChainId_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_id_ChainId_$memory, $a_0);
    ($IsValid'$1_chain_id_ChainId'($rsc))));

    // assume forall $rsc: ResourceDomain<account::Account>(): And(WellFormed($rsc), And(Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.rotation_capability_offer($rsc)))), 1), Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.signer_capability_offer($rsc)))), 1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_account_Account_$memory, $a_0)}(var $rsc := $ResourceValue($1_account_Account_$memory, $a_0);
    (($IsValid'$1_account_Account'($rsc) && ((LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_RotationCapability'($rotation_capability_offer#$1_account_Account($rsc)))) <= 1) && (LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_SignerCapability'($signer_capability_offer#$1_account_Account($rsc)))) <= 1))))));

    // assume forall $rsc: ResourceDomain<coin::CoinStore<aptos_coin::AptosCoin>>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0)}(var $rsc := $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0);
    ($IsValid'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''($rsc))));

    // assume forall $rsc: ResourceDomain<chain_status::GenesisEndMarker>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0);
    ($IsValid'$1_chain_status_GenesisEndMarker'($rsc))));

    // assume forall $rsc: ResourceDomain<timestamp::CurrentTimeMicroseconds>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0)}(var $rsc := $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0);
    ($IsValid'$1_timestamp_CurrentTimeMicroseconds'($rsc))));

    // assume forall $rsc: ResourceDomain<reconfiguration::Configuration>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_reconfiguration_Configuration_$memory, $a_0)}(var $rsc := $ResourceValue($1_reconfiguration_Configuration_$memory, $a_0);
    ($IsValid'$1_reconfiguration_Configuration'($rsc))));

    // assume Implies(chain_status::$is_operating(), exists<timestamp::CurrentTimeMicroseconds>(1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+2006
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.spec.move:4:9+93
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> $ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory, 1));

    // assume Implies(chain_status::$is_operating(), Ge(timestamp::spec_now_microseconds(), reconfiguration::$last_reconfiguration_time())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+2006
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.spec.move:8:9+137
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> ($1_timestamp_spec_now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory) >= $1_reconfiguration_$last_reconfiguration_time($1_reconfiguration_Configuration_$memory)));

    // assume Identical($t10, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t10 == $1_signer_$address_of($t0));

    // assume Identical($t11, Mul($t3, $t4)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t11 == ($t3 * $t4));

    // @20 := save_mem(chain_id::ChainId) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$at(2,2431,2432)"} true;
    $1_chain_id_ChainId_$memory#20 := $1_chain_id_ChainId_$memory;

    // @21 := save_mem(account::Account) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    $1_account_Account_$memory#21 := $1_account_Account_$memory;

    // @22 := save_mem(coin::CoinStore<aptos_coin::AptosCoin>) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#22 := $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory;

    // @19 := save_mem(timestamp::CurrentTimeMicroseconds) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    $1_timestamp_CurrentTimeMicroseconds_$memory#19 := $1_timestamp_CurrentTimeMicroseconds_$memory;

    // trace_local[sender]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,0):", $t0} $t0 == $t0;

    // trace_local[txn_sequence_number]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,1):", $t1} $t1 == $t1;

    // trace_local[txn_authentication_key]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,2):", $t2} $t2 == $t2;

    // trace_local[txn_gas_price]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,3):", $t3} $t3 == $t3;

    // trace_local[txn_max_gas_units]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,4):", $t4} $t4 == $t4;

    // trace_local[txn_expiration_time]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,5):", $t5} $t5 == $t5;

    // trace_local[chain_id]($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:66:5+1
    assume {:print "$track_local(59,4,6):", $t6} $t6 == $t6;

    // $t12 := timestamp::now_seconds() on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:76:13+24
    assume {:print "$at(2,2707,2731)"} true;
    call $t12 := $1_timestamp_now_seconds();
    if ($abort_flag) {
        assume {:print "$at(2,2707,2731)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t14 := <($t12, $t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:76:38+1
    call $t14 := $Lt($t12, $t5);

    // if ($t14) goto L1 else goto L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    assume {:print "$at(2,2686,2832)"} true;
    if ($t14) { goto L1; } else { goto L0; }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
L1:

    // goto L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    assume {:print "$at(2,2686,2832)"} true;
    goto L2;

    // label L0 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:77:37+29
    assume {:print "$at(2,2791,2820)"} true;
L0:

    // $t15 := 1006 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:77:37+29
    assume {:print "$at(2,2791,2820)"} true;
    $t15 := 1006;
    assume $IsValid'u64'($t15);

    // $t16 := error::invalid_argument($t15) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:77:13+54
    call $t16 := $1_error_invalid_argument($t15);
    if ($abort_flag) {
        assume {:print "$at(2,2767,2821)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    assume {:print "$at(2,2686,2832)"} true;
    assume {:print "$track_abort(59,4):", $t16} $t16 == $t16;

    // $t13 := move($t16) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    $t13 := $t16;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:75:9+146
    goto L28;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:17+15
    assume {:print "$at(2,2850,2865)"} true;
L2:

    // $t17 := chain_id::get() on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:17+15
    assume {:print "$at(2,2850,2865)"} true;
    call $t17 := $1_chain_id_get();
    if ($abort_flag) {
        assume {:print "$at(2,2850,2865)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t18 := ==($t17, $t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:33+2
    $t18 := $IsEqual'u8'($t17, $t6);

    // if ($t18) goto L4 else goto L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    if ($t18) { goto L4; } else { goto L3; }

    // label L4 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
L4:

    // goto L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    assume {:print "$at(2,2842,2927)"} true;
    goto L5;

    // label L3 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:70+22
L3:

    // $t19 := 1007 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:70+22
    assume {:print "$at(2,2903,2925)"} true;
    $t19 := 1007;
    assume $IsValid'u64'($t19);

    // $t20 := error::invalid_argument($t19) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:46+47
    call $t20 := $1_error_invalid_argument($t19);
    if ($abort_flag) {
        assume {:print "$at(2,2879,2926)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t20) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    assume {:print "$at(2,2842,2927)"} true;
    assume {:print "$track_abort(59,4):", $t20} $t20 == $t20;

    // $t13 := move($t20) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    $t13 := $t20;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:79:9+85
    goto L28;

    // label L5 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:81:53+7
    assume {:print "$at(2,2982,2989)"} true;
L5:

    // $t21 := signer::address_of($t0) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:81:34+27
    assume {:print "$at(2,2963,2990)"} true;
    call $t21 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,2963,2990)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_local[transaction_sender]($t21) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:81:13+18
    assume {:print "$track_local(59,4,9):", $t21} $t21 == $t21;

    // $t22 := account::exists_at($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:17+38
    assume {:print "$at(2,3008,3046)"} true;
    call $t22 := $1_account_exists_at($t21);
    if ($abort_flag) {
        assume {:print "$at(2,3008,3046)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // if ($t22) goto L7 else goto L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    if ($t22) { goto L7; } else { goto L6; }

    // label L7 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
L7:

    // goto L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    assume {:print "$at(2,3000,3106)"} true;
    goto L8;

    // label L6 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:81+32
L6:

    // $t23 := 1004 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:81+32
    assume {:print "$at(2,3072,3104)"} true;
    $t23 := 1004;
    assume $IsValid'u64'($t23);

    // $t24 := error::invalid_argument($t23) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:57+57
    call $t24 := $1_error_invalid_argument($t23);
    if ($abort_flag) {
        assume {:print "$at(2,3048,3105)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t24) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    assume {:print "$at(2,3000,3106)"} true;
    assume {:print "$track_abort(59,4):", $t24} $t24 == $t24;

    // $t13 := move($t24) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    $t13 := $t24;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:82:9+106
    goto L28;

    // label L8 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:84:13+22
    assume {:print "$at(2,3137,3159)"} true;
L8:

    // $t25 := account::get_authentication_key($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:84:39+51
    assume {:print "$at(2,3163,3214)"} true;
    call $t25 := $1_account_get_authentication_key($t21);
    if ($abort_flag) {
        assume {:print "$at(2,3163,3214)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t26 := ==($t2, $t25) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:84:36+2
    $t26 := $IsEqual'vec'u8''($t2, $t25);

    // if ($t26) goto L10 else goto L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    assume {:print "$at(2,3116,3298)"} true;
    if ($t26) { goto L10; } else { goto L9; }

    // label L10 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
L10:

    // goto L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    assume {:print "$at(2,3116,3298)"} true;
    goto L11;

    // label L9 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:85:37+34
    assume {:print "$at(2,3252,3286)"} true;
L9:

    // $t27 := 1001 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:85:37+34
    assume {:print "$at(2,3252,3286)"} true;
    $t27 := 1001;
    assume $IsValid'u64'($t27);

    // $t28 := error::invalid_argument($t27) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:85:13+59
    call $t28 := $1_error_invalid_argument($t27);
    if ($abort_flag) {
        assume {:print "$at(2,3228,3287)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t28) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    assume {:print "$at(2,3116,3298)"} true;
    assume {:print "$track_abort(59,4):", $t28} $t28 == $t28;

    // $t13 := move($t28) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    $t13 := $t28;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:83:9+182
    goto L28;

    // label L11 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:14+19
    assume {:print "$at(2,3331,3350)"} true;
L11:

    // $t29 := (u128)($t1) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:13+29
    assume {:print "$at(2,3330,3359)"} true;
    call $t29 := $CastU128($t1);
    if ($abort_flag) {
        assume {:print "$at(2,3330,3359)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t30 := 18446744073709551615 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:45+7
    $t30 := 18446744073709551615;
    assume $IsValid'u128'($t30);

    // $t31 := <($t29, $t30) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:89:43+1
    call $t31 := $Lt($t29, $t30);

    // if ($t31) goto L13 else goto L12 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    assume {:print "$at(2,3309,3447)"} true;
    if ($t31) { goto L13; } else { goto L12; }

    // label L13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
L13:

    // goto L14 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    assume {:print "$at(2,3309,3447)"} true;
    goto L14;

    // label L12 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:90:33+33
    assume {:print "$at(2,3403,3436)"} true;
L12:

    // $t32 := 1008 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:90:33+33
    assume {:print "$at(2,3403,3436)"} true;
    $t32 := 1008;
    assume $IsValid'u64'($t32);

    // $t33 := error::out_of_range($t32) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:90:13+54
    call $t33 := $1_error_out_of_range($t32);
    if ($abort_flag) {
        assume {:print "$at(2,3383,3437)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    assume {:print "$at(2,3309,3447)"} true;
    assume {:print "$track_abort(59,4):", $t33} $t33 == $t33;

    // $t13 := move($t33) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    $t13 := $t33;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:88:9+138
    goto L28;

    // label L14 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:93:68+18
    assume {:print "$at(2,3517,3535)"} true;
L14:

    // $t34 := account::get_sequence_number($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:93:39+48
    assume {:print "$at(2,3488,3536)"} true;
    call $t34 := $1_account_get_sequence_number($t21);
    if ($abort_flag) {
        assume {:print "$at(2,3488,3536)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_local[account_sequence_number]($t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:93:13+23
    assume {:print "$track_local(59,4,7):", $t34} $t34 == $t34;

    // $t35 := >=($t1, $t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:95:33+2
    assume {:print "$at(2,3587,3589)"} true;
    call $t35 := $Ge($t1, $t34);

    // if ($t35) goto L16 else goto L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    assume {:print "$at(2,3546,3695)"} true;
    if ($t35) { goto L16; } else { goto L15; }

    // label L16 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
L16:

    // goto L17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    assume {:print "$at(2,3546,3695)"} true;
    goto L17;

    // label L15 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:96:37+33
    assume {:print "$at(2,3651,3684)"} true;
L15:

    // $t36 := 1002 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:96:37+33
    assume {:print "$at(2,3651,3684)"} true;
    $t36 := 1002;
    assume $IsValid'u64'($t36);

    // $t37 := error::invalid_argument($t36) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:96:13+58
    call $t37 := $1_error_invalid_argument($t36);
    if ($abort_flag) {
        assume {:print "$at(2,3627,3685)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t37) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    assume {:print "$at(2,3546,3695)"} true;
    assume {:print "$track_abort(59,4):", $t37} $t37 == $t37;

    // $t13 := move($t37) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    $t13 := $t37;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:94:9+149
    goto L28;

    // label L17 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:102:13+19
    assume {:print "$at(2,3889,3908)"} true;
L17:

    // $t38 := ==($t1, $t34) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:102:33+2
    assume {:print "$at(2,3909,3911)"} true;
    $t38 := $IsEqual'u64'($t1, $t34);

    // if ($t38) goto L19 else goto L18 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    assume {:print "$at(2,3868,4017)"} true;
    if ($t38) { goto L19; } else { goto L18; }

    // label L19 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
L19:

    // goto L20 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    assume {:print "$at(2,3868,4017)"} true;
    goto L20;

    // label L18 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:103:37+33
    assume {:print "$at(2,3973,4006)"} true;
L18:

    // $t39 := 1003 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:103:37+33
    assume {:print "$at(2,3973,4006)"} true;
    $t39 := 1003;
    assume $IsValid'u64'($t39);

    // $t40 := error::invalid_argument($t39) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:103:13+58
    call $t40 := $1_error_invalid_argument($t39);
    if ($abort_flag) {
        assume {:print "$at(2,3949,4007)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t40) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    assume {:print "$at(2,3868,4017)"} true;
    assume {:print "$track_abort(59,4):", $t40} $t40 == $t40;

    // $t13 := move($t40) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    $t13 := $t40;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:101:9+149
    goto L28;

    // label L20 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:106:35+13
    assume {:print "$at(2,4054,4067)"} true;
L20:

    // $t41 := *($t3, $t4) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:106:49+1
    assume {:print "$at(2,4068,4069)"} true;
    call $t41 := $MulU64($t3, $t4);
    if ($abort_flag) {
        assume {:print "$at(2,4068,4069)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_local[max_transaction_fee]($t41) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:106:13+19
    assume {:print "$track_local(59,4,8):", $t41} $t41 == $t41;

    // $t42 := coin::is_account_registered<aptos_coin::AptosCoin>($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:108:13+58
    assume {:print "$at(2,4118,4176)"} true;
    call $t42 := $1_coin_is_account_registered'$1_aptos_coin_AptosCoin'($t21);
    if ($abort_flag) {
        assume {:print "$at(2,4118,4176)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // if ($t42) goto L22 else goto L21 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    assume {:print "$at(2,4097,4256)"} true;
    if ($t42) { goto L22; } else { goto L21; }

    // label L22 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
L22:

    // goto L23 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    assume {:print "$at(2,4097,4256)"} true;
    goto L23;

    // label L21 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:109:37+30
    assume {:print "$at(2,4214,4244)"} true;
L21:

    // $t43 := 1005 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:109:37+30
    assume {:print "$at(2,4214,4244)"} true;
    $t43 := 1005;
    assume $IsValid'u64'($t43);

    // $t44 := error::invalid_argument($t43) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:109:13+55
    call $t44 := $1_error_invalid_argument($t43);
    if ($abort_flag) {
        assume {:print "$at(2,4190,4245)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t44) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    assume {:print "$at(2,4097,4256)"} true;
    assume {:print "$track_abort(59,4):", $t44} $t44 == $t44;

    // $t13 := move($t44) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    $t13 := $t44;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:107:9+159
    goto L28;

    // label L23 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:111:48+18
    assume {:print "$at(2,4305,4323)"} true;
L23:

    // $t45 := coin::balance<aptos_coin::AptosCoin>($t21) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:111:23+44
    assume {:print "$at(2,4280,4324)"} true;
    call $t45 := $1_coin_balance'$1_aptos_coin_AptosCoin'($t21);
    if ($abort_flag) {
        assume {:print "$at(2,4280,4324)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // $t46 := >=($t45, $t41) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:25+2
    assume {:print "$at(2,4350,4352)"} true;
    call $t46 := $Ge($t45, $t41);

    // if ($t46) goto L25 else goto L24 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    if ($t46) { goto L25; } else { goto L24; }

    // label L25 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
L25:

    // goto L26 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    assume {:print "$at(2,4334,4430)"} true;
    goto L26;

    // label L24 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:73+30
L24:

    // $t47 := 1005 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:73+30
    assume {:print "$at(2,4398,4428)"} true;
    $t47 := 1005;
    assume $IsValid'u64'($t47);

    // $t48 := error::invalid_argument($t47) on_abort goto L28 with $t13 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:49+55
    call $t48 := $1_error_invalid_argument($t47);
    if ($abort_flag) {
        assume {:print "$at(2,4374,4429)"} true;
        $t13 := $abort_code;
        assume {:print "$track_abort(59,4):", $t13} $t13 == $t13;
        goto L28;
    }

    // trace_abort($t48) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    assume {:print "$at(2,4334,4430)"} true;
    assume {:print "$track_abort(59,4):", $t48} $t48 == $t48;

    // $t13 := move($t48) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    $t13 := $t48;

    // goto L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:9+96
    goto L28;

    // label L26 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:112:105+1
L26:

    // label L27 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:113:5+1
    assume {:print "$at(2,4436,4437)"} true;
L27:

    // assert Not(Not(exists[@19]<timestamp::CurrentTimeMicroseconds>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:37:9+61
    assume {:print "$at(148,1284,1345)"} true;
    assert {:msg "assert_failed(148,1284,1345): function does not abort under this condition"}
      !!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#19, 1);

    // assert Not(Not(Lt(timestamp::$now_seconds[@19](), $t5))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:38:9+60
    assume {:print "$at(148,1354,1414)"} true;
    assert {:msg "assert_failed(148,1354,1414): function does not abort under this condition"}
      !!($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#19) < $t5);

    // assert Not(Not(exists[@20]<chain_id::ChainId>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:40:9+45
    assume {:print "$at(148,1424,1469)"} true;
    assert {:msg "assert_failed(148,1424,1469): function does not abort under this condition"}
      !!$ResourceExists($1_chain_id_ChainId_$memory#20, 1);

    // assert Not(Not(Eq<u8>(chain_id::$get[@20](), $t6))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:41:9+41
    assume {:print "$at(148,1478,1519)"} true;
    assert {:msg "assert_failed(148,1478,1519): function does not abort under this condition"}
      !!$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#20), $t6);

    // assert Not(Not(account::$exists_at[@21]($t10))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:43:9+50
    assume {:print "$at(148,1589,1639)"} true;
    assert {:msg "assert_failed(148,1589,1639): function does not abort under this condition"}
      !!$1_account_$exists_at($1_account_Account_$memory#21, $t10);

    // assert Not(Not(Ge($t1, select account::Account.sequence_number(global[@21]<account::Account>($t10))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:44:9+88
    assume {:print "$at(148,1648,1736)"} true;
    assert {:msg "assert_failed(148,1648,1736): function does not abort under this condition"}
      !!($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#21, $t10)));

    // assert Not(Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@21]<account::Account>($t10))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:45:9+94
    assume {:print "$at(148,1745,1839)"} true;
    assert {:msg "assert_failed(148,1745,1839): function does not abort under this condition"}
      !!$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#21, $t10)));

    // assert Not(Not(Lt($t1, 18446744073709551615))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:46:9+43
    assume {:print "$at(148,1848,1891)"} true;
    assert {:msg "assert_failed(148,1848,1891): function does not abort under this condition"}
      !!($t1 < 18446744073709551615);

    // assert Not(Gt($t11, 18446744073709551615)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:49:9+40
    assume {:print "$at(148,1970,2010)"} true;
    assert {:msg "assert_failed(148,1970,2010): function does not abort under this condition"}
      !($t11 > 18446744073709551615);

    // assert Not(Not(Eq<u64>($t1, select account::Account.sequence_number(global[@21]<account::Account>($t10))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:50:9+88
    assume {:print "$at(148,2019,2107)"} true;
    assert {:msg "assert_failed(148,2019,2107): function does not abort under this condition"}
      !!$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#21, $t10)));

    // assert Not(Not(exists[@22]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:51:9+60
    assume {:print "$at(148,2116,2176)"} true;
    assert {:msg "assert_failed(148,2116,2176): function does not abort under this condition"}
      !!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#22, $t10);

    // assert Not(Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@22]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))), $t11))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:52:9+96
    assume {:print "$at(148,2185,2281)"} true;
    assert {:msg "assert_failed(148,2185,2281): function does not abort under this condition"}
      !!($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#22, $t10))) >= $t11);

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:52:9+96
    return;

    // label L28 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:113:5+1
    assume {:print "$at(2,4436,4437)"} true;
L28:

    // assert Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Not(exists[@19]<timestamp::CurrentTimeMicroseconds>(1)), Not(Lt(timestamp::$now_seconds[@19](), $t5))), Not(exists[@20]<chain_id::ChainId>(1))), Not(Eq<u8>(chain_id::$get[@20](), $t6))), Not(account::$exists_at[@21]($t10))), Not(Ge($t1, select account::Account.sequence_number(global[@21]<account::Account>($t10))))), Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@21]<account::Account>($t10))))), Not(Lt($t1, 18446744073709551615))), Gt($t11, 18446744073709551615)), Not(Eq<u64>($t1, select account::Account.sequence_number(global[@21]<account::Account>($t10))))), Not(exists[@22]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))), Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@22]<coin::CoinStore<aptos_coin::AptosCoin>>($t10))), $t11))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:55:5+293
    assume {:print "$at(148,2293,2586)"} true;
    assert {:msg "assert_failed(148,2293,2586): abort not covered by any of the `aborts_if` clauses"}
      (((((((((((!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#19, 1) || !($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#19) < $t5)) || !$ResourceExists($1_chain_id_ChainId_$memory#20, 1)) || !$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#20), $t6)) || !$1_account_$exists_at($1_account_Account_$memory#21, $t10)) || !($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#21, $t10)))) || !$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#21, $t10)))) || !($t1 < 18446744073709551615)) || ($t11 > 18446744073709551615)) || !$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#21, $t10)))) || !$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#22, $t10)) || !($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#22, $t10))) >= $t11));

    // abort($t13) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:55:5+293
    $abort_code := $t13;
    $abort_flag := true;
    return;

}

// fun transaction_validation::script_prologue [verification] at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+412
procedure {:timeLimit 40} $1_transaction_validation_script_prologue$verify(_$t0: $signer, _$t1: int, _$t2: Vec (int), _$t3: int, _$t4: int, _$t5: int, _$t6: int, _$t7: Vec (int)) returns ()
{
    // declare local variables
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: int;
    var $t0: $signer;
    var $t1: int;
    var $t2: Vec (int);
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: Vec (int);
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $temp_0'u8': int;
    var $temp_0'vec'u8'': Vec (int);
    var $1_timestamp_CurrentTimeMicroseconds_$memory#29: $Memory $1_timestamp_CurrentTimeMicroseconds;
    var $1_chain_id_ChainId_$memory#30: $Memory $1_chain_id_ChainId;
    var $1_account_Account_$memory#31: $Memory $1_account_Account;
    var $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#32: $Memory $1_coin_CoinStore'$1_aptos_coin_AptosCoin';
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;
    $t3 := _$t3;
    $t4 := _$t4;
    $t5 := _$t5;
    $t6 := _$t6;
    $t7 := _$t7;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$at(2,4827,4828)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume $IsValid'u64'($t1);

    // assume WellFormed($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume $IsValid'vec'u8''($t2);

    // assume WellFormed($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume $IsValid'u64'($t3);

    // assume WellFormed($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume $IsValid'u64'($t4);

    // assume WellFormed($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume $IsValid'u64'($t5);

    // assume WellFormed($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume $IsValid'u8'($t6);

    // assume WellFormed($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume $IsValid'vec'u8''($t7);

    // assume forall $rsc: ResourceDomain<chain_id::ChainId>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_id_ChainId_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_id_ChainId_$memory, $a_0);
    ($IsValid'$1_chain_id_ChainId'($rsc))));

    // assume forall $rsc: ResourceDomain<account::Account>(): And(WellFormed($rsc), And(Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.rotation_capability_offer($rsc)))), 1), Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.signer_capability_offer($rsc)))), 1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_account_Account_$memory, $a_0)}(var $rsc := $ResourceValue($1_account_Account_$memory, $a_0);
    (($IsValid'$1_account_Account'($rsc) && ((LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_RotationCapability'($rotation_capability_offer#$1_account_Account($rsc)))) <= 1) && (LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_SignerCapability'($signer_capability_offer#$1_account_Account($rsc)))) <= 1))))));

    // assume forall $rsc: ResourceDomain<coin::CoinStore<aptos_coin::AptosCoin>>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0)}(var $rsc := $ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory, $a_0);
    ($IsValid'$1_coin_CoinStore'$1_aptos_coin_AptosCoin''($rsc))));

    // assume forall $rsc: ResourceDomain<chain_status::GenesisEndMarker>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0)}(var $rsc := $ResourceValue($1_chain_status_GenesisEndMarker_$memory, $a_0);
    ($IsValid'$1_chain_status_GenesisEndMarker'($rsc))));

    // assume forall $rsc: ResourceDomain<timestamp::CurrentTimeMicroseconds>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0)}(var $rsc := $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $a_0);
    ($IsValid'$1_timestamp_CurrentTimeMicroseconds'($rsc))));

    // assume forall $rsc: ResourceDomain<reconfiguration::Configuration>(): WellFormed($rsc) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_reconfiguration_Configuration_$memory, $a_0)}(var $rsc := $ResourceValue($1_reconfiguration_Configuration_$memory, $a_0);
    ($IsValid'$1_reconfiguration_Configuration'($rsc))));

    // assume Implies(chain_status::$is_operating(), exists<timestamp::CurrentTimeMicroseconds>(1)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+412
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.spec.move:4:9+93
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> $ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory, 1));

    // assume Implies(chain_status::$is_operating(), Ge(timestamp::spec_now_microseconds(), reconfiguration::$last_reconfiguration_time())) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+412
    // global invariant at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.spec.move:8:9+137
    assume ($1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory) ==> ($1_timestamp_spec_now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory) >= $1_reconfiguration_$last_reconfiguration_time($1_reconfiguration_Configuration_$memory)));

    // assume Identical($t8, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t8 == $1_signer_$address_of($t0));

    // assume Identical($t9, Mul($t3, $t4)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t9 == ($t3 * $t4));

    // @30 := save_mem(chain_id::ChainId) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$at(2,4827,4828)"} true;
    $1_chain_id_ChainId_$memory#30 := $1_chain_id_ChainId_$memory;

    // @31 := save_mem(account::Account) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    $1_account_Account_$memory#31 := $1_account_Account_$memory;

    // @32 := save_mem(coin::CoinStore<aptos_coin::AptosCoin>) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#32 := $1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory;

    // @29 := save_mem(timestamp::CurrentTimeMicroseconds) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    $1_timestamp_CurrentTimeMicroseconds_$memory#29 := $1_timestamp_CurrentTimeMicroseconds_$memory;

    // trace_local[sender]($t0) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,0):", $t0} $t0 == $t0;

    // trace_local[txn_sequence_number]($t1) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,1):", $t1} $t1 == $t1;

    // trace_local[txn_public_key]($t2) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,2):", $t2} $t2 == $t2;

    // trace_local[txn_gas_price]($t3) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,3):", $t3} $t3 == $t3;

    // trace_local[txn_max_gas_units]($t4) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,4):", $t4} $t4 == $t4;

    // trace_local[txn_expiration_time]($t5) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,5):", $t5} $t5 == $t5;

    // trace_local[chain_id]($t6) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,6):", $t6} $t6 == $t6;

    // trace_local[_script_hash]($t7) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:127:5+1
    assume {:print "$track_local(59,5,7):", $t7} $t7 == $t7;

    // assume Identical($t10, signer::$address_of($t0)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:42:9+52
    assume {:print "$at(148,1528,1580)"} true;
    assume ($t10 == $1_signer_$address_of($t0));

    // assume Identical($t11, Mul($t3, $t4)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:48:9+60
    assume {:print "$at(148,1901,1961)"} true;
    assume ($t11 == ($t3 * $t4));

    // transaction_validation::prologue_common($t0, $t1, $t2, $t3, $t4, $t5, $t6) on_abort goto L2 with $t12 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:137:9+125
    assume {:print "$at(2,5108,5233)"} true;
    call $1_transaction_validation_prologue_common($t0, $t1, $t2, $t3, $t4, $t5, $t6);
    if ($abort_flag) {
        assume {:print "$at(2,5108,5233)"} true;
        $t12 := $abort_code;
        assume {:print "$track_abort(59,5):", $t12} $t12 == $t12;
        goto L2;
    }

    // label L1 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:138:5+1
    assume {:print "$at(2,5238,5239)"} true;
L1:

    // assert Not(Not(exists[@29]<timestamp::CurrentTimeMicroseconds>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:37:9+61
    assume {:print "$at(148,1284,1345)"} true;
    assert {:msg "assert_failed(148,1284,1345): function does not abort under this condition"}
      !!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#29, 1);

    // assert Not(Not(Lt(timestamp::$now_seconds[@29](), $t5))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:38:9+60
    assume {:print "$at(148,1354,1414)"} true;
    assert {:msg "assert_failed(148,1354,1414): function does not abort under this condition"}
      !!($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#29) < $t5);

    // assert Not(Not(exists[@30]<chain_id::ChainId>(1))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:40:9+45
    assume {:print "$at(148,1424,1469)"} true;
    assert {:msg "assert_failed(148,1424,1469): function does not abort under this condition"}
      !!$ResourceExists($1_chain_id_ChainId_$memory#30, 1);

    // assert Not(Not(Eq<u8>(chain_id::$get[@30](), $t6))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:41:9+41
    assume {:print "$at(148,1478,1519)"} true;
    assert {:msg "assert_failed(148,1478,1519): function does not abort under this condition"}
      !!$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#30), $t6);

    // assert Not(Not(account::$exists_at[@31]($t8))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:43:9+50
    assume {:print "$at(148,1589,1639)"} true;
    assert {:msg "assert_failed(148,1589,1639): function does not abort under this condition"}
      !!$1_account_$exists_at($1_account_Account_$memory#31, $t8);

    // assert Not(Not(Ge($t1, select account::Account.sequence_number(global[@31]<account::Account>($t8))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:44:9+88
    assume {:print "$at(148,1648,1736)"} true;
    assert {:msg "assert_failed(148,1648,1736): function does not abort under this condition"}
      !!($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#31, $t8)));

    // assert Not(Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@31]<account::Account>($t8))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:45:9+94
    assume {:print "$at(148,1745,1839)"} true;
    assert {:msg "assert_failed(148,1745,1839): function does not abort under this condition"}
      !!$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#31, $t8)));

    // assert Not(Not(Lt($t1, 18446744073709551615))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:46:9+43
    assume {:print "$at(148,1848,1891)"} true;
    assert {:msg "assert_failed(148,1848,1891): function does not abort under this condition"}
      !!($t1 < 18446744073709551615);

    // assert Not(Gt($t9, 18446744073709551615)) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:49:9+40
    assume {:print "$at(148,1970,2010)"} true;
    assert {:msg "assert_failed(148,1970,2010): function does not abort under this condition"}
      !($t9 > 18446744073709551615);

    // assert Not(Not(Eq<u64>($t1, select account::Account.sequence_number(global[@31]<account::Account>($t8))))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:50:9+88
    assume {:print "$at(148,2019,2107)"} true;
    assert {:msg "assert_failed(148,2019,2107): function does not abort under this condition"}
      !!$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#31, $t8)));

    // assert Not(Not(exists[@32]<coin::CoinStore<aptos_coin::AptosCoin>>($t8))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:51:9+60
    assume {:print "$at(148,2116,2176)"} true;
    assert {:msg "assert_failed(148,2116,2176): function does not abort under this condition"}
      !!$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#32, $t8);

    // assert Not(Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@32]<coin::CoinStore<aptos_coin::AptosCoin>>($t8))), $t9))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:52:9+96
    assume {:print "$at(148,2185,2281)"} true;
    assert {:msg "assert_failed(148,2185,2281): function does not abort under this condition"}
      !!($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#32, $t8))) >= $t9);

    // return () at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:52:9+96
    return;

    // label L2 at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.move:138:5+1
    assume {:print "$at(2,5238,5239)"} true;
L2:

    // assert Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Or(Not(exists[@29]<timestamp::CurrentTimeMicroseconds>(1)), Not(Lt(timestamp::$now_seconds[@29](), $t5))), Not(exists[@30]<chain_id::ChainId>(1))), Not(Eq<u8>(chain_id::$get[@30](), $t6))), Not(account::$exists_at[@31]($t8))), Not(Ge($t1, select account::Account.sequence_number(global[@31]<account::Account>($t8))))), Not(Eq<vector<u8>>($t2, select account::Account.authentication_key(global[@31]<account::Account>($t8))))), Not(Lt($t1, 18446744073709551615))), Gt($t9, 18446744073709551615)), Not(Eq<u64>($t1, select account::Account.sequence_number(global[@31]<account::Account>($t8))))), Not(exists[@32]<coin::CoinStore<aptos_coin::AptosCoin>>($t8))), Not(Ge(select coin::Coin.value(select coin::CoinStore.coin(global[@32]<coin::CoinStore<aptos_coin::AptosCoin>>($t8))), $t9))) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:81:5+382
    assume {:print "$at(148,2946,3328)"} true;
    assert {:msg "assert_failed(148,2946,3328): abort not covered by any of the `aborts_if` clauses"}
      (((((((((((!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory#29, 1) || !($1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory#29) < $t5)) || !$ResourceExists($1_chain_id_ChainId_$memory#30, 1)) || !$IsEqual'u8'($1_chain_id_$get($1_chain_id_ChainId_$memory#30), $t6)) || !$1_account_$exists_at($1_account_Account_$memory#31, $t8)) || !($t1 >= $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#31, $t8)))) || !$IsEqual'vec'u8''($t2, $authentication_key#$1_account_Account($ResourceValue($1_account_Account_$memory#31, $t8)))) || !($t1 < 18446744073709551615)) || ($t9 > 18446744073709551615)) || !$IsEqual'u64'($t1, $sequence_number#$1_account_Account($ResourceValue($1_account_Account_$memory#31, $t8)))) || !$ResourceExists($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#32, $t8)) || !($value#$1_coin_Coin'$1_aptos_coin_AptosCoin'($coin#$1_coin_CoinStore'$1_aptos_coin_AptosCoin'($ResourceValue($1_coin_CoinStore'$1_aptos_coin_AptosCoin'_$memory#32, $t8))) >= $t9));

    // abort($t12) at /home/xudong/move-projects/mb-aptos-core/aptos-move/framework/aptos-framework/sources/transaction_validation.spec.move:81:5+382
    $abort_code := $t12;
    $abort_flag := true;
    return;

}
