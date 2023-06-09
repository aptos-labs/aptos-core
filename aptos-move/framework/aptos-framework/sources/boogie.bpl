
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

// ----------------------------------------------------------------------------------
// Native BCS implementation for element type `$1_guid_GUID`

// Serialize is modeled as an uninterpreted function, with an additional
// axiom to say it's an injection.

function $1_bcs_serialize'$1_guid_GUID'(v: $1_guid_GUID): Vec int;

axiom (forall v1, v2: $1_guid_GUID :: {$1_bcs_serialize'$1_guid_GUID'(v1), $1_bcs_serialize'$1_guid_GUID'(v2)}
   $IsEqual'$1_guid_GUID'(v1, v2) <==> $IsEqual'vec'u8''($1_bcs_serialize'$1_guid_GUID'(v1), $1_bcs_serialize'$1_guid_GUID'(v2)));

// This says that serialize returns a non-empty vec<u8>

axiom (forall v: $1_guid_GUID :: {$1_bcs_serialize'$1_guid_GUID'(v)}
     ( var r := $1_bcs_serialize'$1_guid_GUID'(v); $IsValid'vec'u8''(r) && LenVec(r) > 0 ));


procedure $1_bcs_to_bytes'$1_guid_GUID'(v: $1_guid_GUID) returns (res: Vec int);
ensures res == $1_bcs_serialize'$1_guid_GUID'(v);

function {:inline} $1_bcs_$to_bytes'$1_guid_GUID'(v: $1_guid_GUID): Vec int {
    $1_bcs_serialize'$1_guid_GUID'(v)
}




// ----------------------------------------------------------------------------------
// Native BCS implementation for element type `$1_guid_ID`

// Serialize is modeled as an uninterpreted function, with an additional
// axiom to say it's an injection.

function $1_bcs_serialize'$1_guid_ID'(v: $1_guid_ID): Vec int;

axiom (forall v1, v2: $1_guid_ID :: {$1_bcs_serialize'$1_guid_ID'(v1), $1_bcs_serialize'$1_guid_ID'(v2)}
   $IsEqual'$1_guid_ID'(v1, v2) <==> $IsEqual'vec'u8''($1_bcs_serialize'$1_guid_ID'(v1), $1_bcs_serialize'$1_guid_ID'(v2)));

// This says that serialize returns a non-empty vec<u8>

axiom (forall v: $1_guid_ID :: {$1_bcs_serialize'$1_guid_ID'(v)}
     ( var r := $1_bcs_serialize'$1_guid_ID'(v); $IsValid'vec'u8''(r) && LenVec(r) > 0 ));


procedure $1_bcs_to_bytes'$1_guid_ID'(v: $1_guid_ID) returns (res: Vec int);
ensures res == $1_bcs_serialize'$1_guid_ID'(v);

function {:inline} $1_bcs_$to_bytes'$1_guid_ID'(v: $1_guid_ID): Vec int {
    $1_bcs_serialize'$1_guid_ID'(v)
}




// ----------------------------------------------------------------------------------
// Native BCS implementation for element type `address`

// Serialize is modeled as an uninterpreted function, with an additional
// axiom to say it's an injection.

function $1_bcs_serialize'address'(v: int): Vec int;

axiom (forall v1, v2: int :: {$1_bcs_serialize'address'(v1), $1_bcs_serialize'address'(v2)}
   $IsEqual'address'(v1, v2) <==> $IsEqual'vec'u8''($1_bcs_serialize'address'(v1), $1_bcs_serialize'address'(v2)));

// This says that serialize returns a non-empty vec<u8>

axiom (forall v: int :: {$1_bcs_serialize'address'(v)}
     ( var r := $1_bcs_serialize'address'(v); $IsValid'vec'u8''(r) && LenVec(r) > 0 ));


procedure $1_bcs_to_bytes'address'(v: int) returns (res: Vec int);
ensures res == $1_bcs_serialize'address'(v);

function {:inline} $1_bcs_$to_bytes'address'(v: int): Vec int {
    $1_bcs_serialize'address'(v)
}

// Serialized addresses should have the same length.
const $serialized_address_len: int;
// Serialized addresses should have the same length
axiom (forall v: int :: {$1_bcs_serialize'address'(v)}
     ( var r := $1_bcs_serialize'address'(v); LenVec(r) == $serialized_address_len));




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
type #1;
function {:inline} $IsEqual'#1'(x1: #1, x2: #1): bool { x1 == x2 }
function {:inline} $IsValid'#1'(x: #1): bool { true }
var #1_info: $TypeParamInfo;
var #1_$memory: $Memory #1;

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'bool'($1_from_bcs_deserialize'bool'(b1), $1_from_bcs_deserialize'bool'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'u8'($1_from_bcs_deserialize'u8'(b1), $1_from_bcs_deserialize'u8'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'u64'($1_from_bcs_deserialize'u64'(b1), $1_from_bcs_deserialize'u64'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'u256'($1_from_bcs_deserialize'u256'(b1), $1_from_bcs_deserialize'u256'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'address'($1_from_bcs_deserialize'address'(b1), $1_from_bcs_deserialize'address'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'signer'($1_from_bcs_deserialize'signer'(b1), $1_from_bcs_deserialize'signer'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_from_bcs_deserialize'vec'u8''(b1), $1_from_bcs_deserialize'vec'u8''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'address''($1_from_bcs_deserialize'vec'address''(b1), $1_from_bcs_deserialize'vec'address''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'#0''($1_from_bcs_deserialize'vec'#0''(b1), $1_from_bcs_deserialize'vec'#0''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <option::Option<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_option_Option'address''($1_from_bcs_deserialize'$1_option_Option'address''(b1), $1_from_bcs_deserialize'$1_option_Option'address''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <type_info::TypeInfo>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_type_info_TypeInfo'($1_from_bcs_deserialize'$1_type_info_TypeInfo'(b1), $1_from_bcs_deserialize'$1_type_info_TypeInfo'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_guid_GUID'($1_from_bcs_deserialize'$1_guid_GUID'(b1), $1_from_bcs_deserialize'$1_guid_GUID'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_guid_ID'($1_from_bcs_deserialize'$1_guid_ID'(b1), $1_from_bcs_deserialize'$1_guid_ID'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<account::CoinRegisterEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<account::KeyRotationEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<object::TransferEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'$1_object_TransferEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <event::EventHandle<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_event_EventHandle'#0''($1_from_bcs_deserialize'$1_event_EventHandle'#0''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'#0''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <account::Account>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_account_Account'($1_from_bcs_deserialize'$1_account_Account'(b1), $1_from_bcs_deserialize'$1_account_Account'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <account::CapabilityOffer<account::RotationCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <account::CapabilityOffer<account::SignerCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::ConstructorRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_ConstructorRef'($1_from_bcs_deserialize'$1_object_ConstructorRef'(b1), $1_from_bcs_deserialize'$1_object_ConstructorRef'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::DeleteRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_DeleteRef'($1_from_bcs_deserialize'$1_object_DeleteRef'(b1), $1_from_bcs_deserialize'$1_object_DeleteRef'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::DeriveRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_DeriveRef'($1_from_bcs_deserialize'$1_object_DeriveRef'(b1), $1_from_bcs_deserialize'$1_object_DeriveRef'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::ExtendRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_ExtendRef'($1_from_bcs_deserialize'$1_object_ExtendRef'(b1), $1_from_bcs_deserialize'$1_object_ExtendRef'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::LinearTransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_LinearTransferRef'($1_from_bcs_deserialize'$1_object_LinearTransferRef'(b1), $1_from_bcs_deserialize'$1_object_LinearTransferRef'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::Object<object::ObjectCore>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_Object'$1_object_ObjectCore''($1_from_bcs_deserialize'$1_object_Object'$1_object_ObjectCore''(b1), $1_from_bcs_deserialize'$1_object_Object'$1_object_ObjectCore''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::Object<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_Object'#0''($1_from_bcs_deserialize'$1_object_Object'#0''(b1), $1_from_bcs_deserialize'$1_object_Object'#0''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::Object<#1>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_Object'#1''($1_from_bcs_deserialize'$1_object_Object'#1''(b1), $1_from_bcs_deserialize'$1_object_Object'#1''(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::ObjectCore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_ObjectCore'($1_from_bcs_deserialize'$1_object_ObjectCore'(b1), $1_from_bcs_deserialize'$1_object_ObjectCore'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::TransferEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_TransferEvent'($1_from_bcs_deserialize'$1_object_TransferEvent'(b1), $1_from_bcs_deserialize'$1_object_TransferEvent'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <object::TransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'$1_object_TransferRef'($1_from_bcs_deserialize'$1_object_TransferRef'(b1), $1_from_bcs_deserialize'$1_object_TransferRef'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'#0'($1_from_bcs_deserialize'#0'(b1), $1_from_bcs_deserialize'#0'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:14:9+116, instance <#1>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'#1'($1_from_bcs_deserialize'#1'(b1), $1_from_bcs_deserialize'#1'(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'bool'(b1), $1_from_bcs_deserializable'bool'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u8'(b1), $1_from_bcs_deserializable'u8'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u64'(b1), $1_from_bcs_deserializable'u64'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u256'(b1), $1_from_bcs_deserializable'u256'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'address'(b1), $1_from_bcs_deserializable'address'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'signer'(b1), $1_from_bcs_deserializable'signer'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'u8''(b1), $1_from_bcs_deserializable'vec'u8''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'address''(b1), $1_from_bcs_deserializable'vec'address''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'#0''(b1), $1_from_bcs_deserializable'vec'#0''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <option::Option<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_option_Option'address''(b1), $1_from_bcs_deserializable'$1_option_Option'address''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <type_info::TypeInfo>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_type_info_TypeInfo'(b1), $1_from_bcs_deserializable'$1_type_info_TypeInfo'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_GUID'(b1), $1_from_bcs_deserializable'$1_guid_GUID'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_ID'(b1), $1_from_bcs_deserializable'$1_guid_ID'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<account::CoinRegisterEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<account::KeyRotationEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<object::TransferEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <event::EventHandle<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'#0''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'#0''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <account::Account>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_account_Account'(b1), $1_from_bcs_deserializable'$1_account_Account'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <account::CapabilityOffer<account::RotationCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(b1), $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <account::CapabilityOffer<account::SignerCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(b1), $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ConstructorRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ConstructorRef'(b1), $1_from_bcs_deserializable'$1_object_ConstructorRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::DeleteRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_DeleteRef'(b1), $1_from_bcs_deserializable'$1_object_DeleteRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::DeriveRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_DeriveRef'(b1), $1_from_bcs_deserializable'$1_object_DeriveRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ExtendRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ExtendRef'(b1), $1_from_bcs_deserializable'$1_object_ExtendRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::LinearTransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_LinearTransferRef'(b1), $1_from_bcs_deserializable'$1_object_LinearTransferRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::Object<object::ObjectCore>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_Object'$1_object_ObjectCore''(b1), $1_from_bcs_deserializable'$1_object_Object'$1_object_ObjectCore''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::Object<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_Object'#0''(b1), $1_from_bcs_deserializable'$1_object_Object'#0''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::Object<#1>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_Object'#1''(b1), $1_from_bcs_deserializable'$1_object_Object'#1''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::ObjectCore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_ObjectCore'(b1), $1_from_bcs_deserializable'$1_object_ObjectCore'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::TransferEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_TransferEvent'(b1), $1_from_bcs_deserializable'$1_object_TransferEvent'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <object::TransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_object_TransferRef'(b1), $1_from_bcs_deserializable'$1_object_TransferRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'#0'(b1), $1_from_bcs_deserializable'#0'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <#1>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'#1'(b1), $1_from_bcs_deserializable'#1'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserialize'bool'(b1), $1_from_bcs_deserialize'bool'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u8'($1_from_bcs_deserialize'u8'(b1), $1_from_bcs_deserialize'u8'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u64'($1_from_bcs_deserialize'u64'(b1), $1_from_bcs_deserialize'u64'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u256'($1_from_bcs_deserialize'u256'(b1), $1_from_bcs_deserialize'u256'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'address'($1_from_bcs_deserialize'address'(b1), $1_from_bcs_deserialize'address'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'signer'($1_from_bcs_deserialize'signer'(b1), $1_from_bcs_deserialize'signer'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'u8''($1_from_bcs_deserialize'vec'u8''(b1), $1_from_bcs_deserialize'vec'u8''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'address''($1_from_bcs_deserialize'vec'address''(b1), $1_from_bcs_deserialize'vec'address''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'#0''($1_from_bcs_deserialize'vec'#0''(b1), $1_from_bcs_deserialize'vec'#0''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <option::Option<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_option_Option'address''($1_from_bcs_deserialize'$1_option_Option'address''(b1), $1_from_bcs_deserialize'$1_option_Option'address''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <type_info::TypeInfo>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_type_info_TypeInfo'($1_from_bcs_deserialize'$1_type_info_TypeInfo'(b1), $1_from_bcs_deserialize'$1_type_info_TypeInfo'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_GUID'($1_from_bcs_deserialize'$1_guid_GUID'(b1), $1_from_bcs_deserialize'$1_guid_GUID'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_ID'($1_from_bcs_deserialize'$1_guid_ID'(b1), $1_from_bcs_deserialize'$1_guid_ID'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<account::CoinRegisterEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<account::KeyRotationEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<object::TransferEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_object_TransferEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <event::EventHandle<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'#0''($1_from_bcs_deserialize'$1_event_EventHandle'#0''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'#0''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <account::Account>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_account_Account'($1_from_bcs_deserialize'$1_account_Account'(b1), $1_from_bcs_deserialize'$1_account_Account'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <account::CapabilityOffer<account::RotationCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <account::CapabilityOffer<account::SignerCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ConstructorRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ConstructorRef'($1_from_bcs_deserialize'$1_object_ConstructorRef'(b1), $1_from_bcs_deserialize'$1_object_ConstructorRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::DeleteRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_DeleteRef'($1_from_bcs_deserialize'$1_object_DeleteRef'(b1), $1_from_bcs_deserialize'$1_object_DeleteRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::DeriveRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_DeriveRef'($1_from_bcs_deserialize'$1_object_DeriveRef'(b1), $1_from_bcs_deserialize'$1_object_DeriveRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ExtendRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ExtendRef'($1_from_bcs_deserialize'$1_object_ExtendRef'(b1), $1_from_bcs_deserialize'$1_object_ExtendRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::LinearTransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_LinearTransferRef'($1_from_bcs_deserialize'$1_object_LinearTransferRef'(b1), $1_from_bcs_deserialize'$1_object_LinearTransferRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::Object<object::ObjectCore>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_Object'$1_object_ObjectCore''($1_from_bcs_deserialize'$1_object_Object'$1_object_ObjectCore''(b1), $1_from_bcs_deserialize'$1_object_Object'$1_object_ObjectCore''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::Object<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_Object'#0''($1_from_bcs_deserialize'$1_object_Object'#0''(b1), $1_from_bcs_deserialize'$1_object_Object'#0''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::Object<#1>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_Object'#1''($1_from_bcs_deserialize'$1_object_Object'#1''(b1), $1_from_bcs_deserialize'$1_object_Object'#1''(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::ObjectCore>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_ObjectCore'($1_from_bcs_deserialize'$1_object_ObjectCore'(b1), $1_from_bcs_deserialize'$1_object_ObjectCore'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::TransferEvent>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_TransferEvent'($1_from_bcs_deserialize'$1_object_TransferEvent'(b1), $1_from_bcs_deserialize'$1_object_TransferEvent'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <object::TransferRef>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_object_TransferRef'($1_from_bcs_deserialize'$1_object_TransferRef'(b1), $1_from_bcs_deserialize'$1_object_TransferRef'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'#0'($1_from_bcs_deserialize'#0'(b1), $1_from_bcs_deserialize'#0'(b2)))));

// axiom at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <#1>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'#1'($1_from_bcs_deserialize'#1'(b1), $1_from_bcs_deserialize'#1'(b2)))));

// struct option::Option<address> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:7:5+81
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

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
function {:inline} $1_signer_$address_of(s: $signer): int {
    $1_signer_$borrow_address(s)
}

// fun signer::address_of [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+77
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
    // trace_local[s]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:12:5+1
    assume {:print "$at(14,396,397)"} true;
    assume {:print "$track_local(3,0,0):", $t0} $t0 == $t0;

    // $t1 := signer::borrow_address($t0) on_abort goto L2 with $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:10+17
    assume {:print "$at(14,450,467)"} true;
    call $t1 := $1_signer_borrow_address($t0);
    if ($abort_flag) {
        assume {:print "$at(14,450,467)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(3,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:13:9+18
    assume {:print "$track_return(3,0,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(14,472,473)"} true;
L1:

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(14,472,473)"} true;
    $ret0 := $t1;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
L2:

    // abort($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:14:5+1
    assume {:print "$at(14,472,473)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun error::already_exists [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:3+71
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
    // trace_local[r]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:3+1
    assume {:print "$at(10,3585,3586)"} true;
    assume {:print "$track_local(4,1,0):", $t0} $t0 == $t0;

    // $t1 := 8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:54+14
    $t1 := 8;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(10,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume {:print "$at(10,3626,3654)"} true;

    // assume WellFormed($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28

    // trace_return[0]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume {:print "$track_return(4,1,0):", $t3} $t3 == $t3;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:73+1
L1:

    // return $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:73+1
    assume {:print "$at(10,3655,3656)"} true;
    $ret0 := $t3;
    return;

}

// fun error::not_found [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+61
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
    // trace_local[r]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+1
    assume {:print "$at(10,3461,3462)"} true;
    assume {:print "$track_local(4,6,0):", $t0} $t0 == $t0;

    // $t1 := 6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:49+9
    $t1 := 6;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(10,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$at(10,3497,3520)"} true;

    // assume WellFormed($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23

    // trace_return[0]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$track_return(4,6,0):", $t3} $t3 == $t3;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
L1:

    // return $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
    assume {:print "$at(10,3521,3522)"} true;
    $ret0 := $t3;
    return;

}

// fun error::out_of_range [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:3+68
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
    // trace_local[r]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:3+1
    assume {:print "$at(10,3161,3162)"} true;
    assume {:print "$track_local(4,8,0):", $t0} $t0 == $t0;

    // $t1 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:53+12
    $t1 := 2;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(10,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume {:print "$at(10,3201,3227)"} true;

    // assume WellFormed($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26

    // trace_return[0]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:43+26
    assume {:print "$track_return(4,8,0):", $t3} $t3 == $t3;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:70+1
L1:

    // return $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:77:70+1
    assume {:print "$at(10,3228,3229)"} true;
    $ret0 := $t3;
    return;

}

// fun error::permission_denied [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:3+77
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
    // trace_local[r]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:3+1
    assume {:print "$at(10,3381,3382)"} true;
    assume {:print "$track_local(4,9,0):", $t0} $t0 == $t0;

    // $t1 := 5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:57+17
    $t1 := 5;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(10,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume {:print "$at(10,3425,3456)"} true;

    // assume WellFormed($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31

    // trace_return[0]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume {:print "$track_return(4,9,0):", $t3} $t3 == $t3;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:79+1
L1:

    // return $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:79+1
    assume {:print "$at(10,3457,3458)"} true;
    $ret0 := $t3;
    return;

}

// struct type_info::TypeInfo at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/type_info.move:17:5+145
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

// struct guid::GUID at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:7:5+50
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

// struct guid::ID at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:12:5+209
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

// fun guid::create [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:23:5+286
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
    // trace_local[addr]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:23:5+1
    assume {:print "$at(118,836,837)"} true;
    assume {:print "$track_local(13,0,0):", $t0} $t0 == $t0;

    // trace_local[creation_num_ref]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:23:5+1
    $temp_0'u64' := $Dereference($t1);
    assume {:print "$track_local(13,0,1):", $temp_0'u64'} $temp_0'u64' == $temp_0'u64';

    // $t3 := read_ref($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:24:28+17
    assume {:print "$at(118,940,957)"} true;
    $t3 := $Dereference($t1);

    // trace_local[creation_num]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:24:13+12
    assume {:print "$track_local(13,0,2):", $t3} $t3 == $t3;

    // $t4 := 1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:25:44+1
    assume {:print "$at(118,1002,1003)"} true;
    $t4 := 1;
    assume $IsValid'u64'($t4);

    // $t5 := +($t3, $t4) on_abort goto L2 with $t6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:25:42+1
    call $t5 := $AddU64($t3, $t4);
    if ($abort_flag) {
        assume {:print "$at(118,1000,1001)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(13,0):", $t6} $t6 == $t6;
        goto L2;
    }

    // write_ref($t1, $t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:25:9+36
    $t1 := $UpdateMutation($t1, $t5);

    // $t7 := pack guid::ID($t3, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:27:17+70
    assume {:print "$at(118,1036,1106)"} true;
    $t7 := $1_guid_ID($t3, $t0);

    // $t8 := pack guid::GUID($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    assume {:print "$at(118,1013,1116)"} true;
    $t8 := $1_guid_GUID($t7);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    assume {:print "$track_return(13,0,0):", $t8} $t8 == $t8;

    // trace_local[creation_num_ref]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:26:9+103
    $temp_0'u64' := $Dereference($t1);
    assume {:print "$track_local(13,0,1):", $temp_0'u64'} $temp_0'u64' == $temp_0'u64';

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(118,1121,1122)"} true;
L1:

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(118,1121,1122)"} true;
    $ret0 := $t8;
    $ret1 := $t1;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
L2:

    // abort($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:32:5+1
    assume {:print "$at(118,1121,1122)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun guid::create_id [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:35:5+100
procedure {:inline 1} $1_guid_create_id(_$t0: int, _$t1: int) returns ($ret0: $1_guid_ID)
{
    // declare local variables
    var $t2: $1_guid_ID;
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_guid_ID': $1_guid_ID;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[addr]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:35:5+1
    assume {:print "$at(118,1194,1195)"} true;
    assume {:print "$track_local(13,1,0):", $t0} $t0 == $t0;

    // trace_local[creation_num]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:35:5+1
    assume {:print "$track_local(13,1,1):", $t1} $t1 == $t1;

    // $t2 := pack guid::ID($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:36:9+25
    assume {:print "$at(118,1263,1288)"} true;
    $t2 := $1_guid_ID($t1, $t0);

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:36:9+25
    assume {:print "$track_return(13,1,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:37:5+1
    assume {:print "$at(118,1293,1294)"} true;
L1:

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:37:5+1
    assume {:print "$at(118,1293,1294)"} true;
    $ret0 := $t2;
    return;

}

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u8'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u8'(bytes);
$IsValid'u8'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u64'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u64'(bytes);
$IsValid'u64'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u256'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u256'(bytes);
$IsValid'u256'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'address'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'address'(bytes);
$IsValid'address'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'signer'(bytes: Vec (int)): $signer;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'signer'(bytes);
$IsValid'signer'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'u8''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'u8''(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'address''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'address''(bytes);
$IsValid'vec'address''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'#0''(bytes: Vec (int)): Vec (#0);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'#0''(bytes);
$IsValid'vec'#0''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_option_Option'address''(bytes: Vec (int)): $1_option_Option'address';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_option_Option'address''(bytes);
$IsValid'$1_option_Option'address''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_type_info_TypeInfo'(bytes: Vec (int)): $1_type_info_TypeInfo;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_type_info_TypeInfo'(bytes);
$IsValid'$1_type_info_TypeInfo'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_GUID'(bytes: Vec (int)): $1_guid_GUID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_GUID'(bytes);
$IsValid'$1_guid_GUID'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_ID'(bytes: Vec (int)): $1_guid_ID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_ID'(bytes);
$IsValid'$1_guid_ID'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_account_CoinRegisterEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_account_CoinRegisterEvent''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_account_KeyRotationEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_account_KeyRotationEvent''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_object_TransferEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_object_TransferEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_object_TransferEvent''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'#0''(bytes: Vec (int)): $1_event_EventHandle'#0';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'#0''(bytes);
$IsValid'$1_event_EventHandle'#0''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_Account'(bytes: Vec (int)): $1_account_Account;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_Account'(bytes);
$IsValid'$1_account_Account'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes: Vec (int)): $1_account_CapabilityOffer'$1_account_RotationCapability';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes);
$IsValid'$1_account_CapabilityOffer'$1_account_RotationCapability''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes: Vec (int)): $1_account_CapabilityOffer'$1_account_SignerCapability';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes);
$IsValid'$1_account_CapabilityOffer'$1_account_SignerCapability''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ConstructorRef'(bytes: Vec (int)): $1_object_ConstructorRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ConstructorRef'(bytes);
$IsValid'$1_object_ConstructorRef'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_DeleteRef'(bytes: Vec (int)): $1_object_DeleteRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_DeleteRef'(bytes);
$IsValid'$1_object_DeleteRef'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_DeriveRef'(bytes: Vec (int)): $1_object_DeriveRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_DeriveRef'(bytes);
$IsValid'$1_object_DeriveRef'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ExtendRef'(bytes: Vec (int)): $1_object_ExtendRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ExtendRef'(bytes);
$IsValid'$1_object_ExtendRef'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_LinearTransferRef'(bytes: Vec (int)): $1_object_LinearTransferRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_LinearTransferRef'(bytes);
$IsValid'$1_object_LinearTransferRef'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_Object'$1_object_ObjectCore''(bytes: Vec (int)): $1_object_Object'$1_object_ObjectCore';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_Object'$1_object_ObjectCore''(bytes);
$IsValid'$1_object_Object'$1_object_ObjectCore''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_Object'#0''(bytes: Vec (int)): $1_object_Object'#0';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_Object'#0''(bytes);
$IsValid'$1_object_Object'#0''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_Object'#1''(bytes: Vec (int)): $1_object_Object'#1';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_Object'#1''(bytes);
$IsValid'$1_object_Object'#1''($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_ObjectCore'(bytes: Vec (int)): $1_object_ObjectCore;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_ObjectCore'(bytes);
$IsValid'$1_object_ObjectCore'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_TransferEvent'(bytes: Vec (int)): $1_object_TransferEvent;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_TransferEvent'(bytes);
$IsValid'$1_object_TransferEvent'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_object_TransferRef'(bytes: Vec (int)): $1_object_TransferRef;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_object_TransferRef'(bytes);
$IsValid'$1_object_TransferRef'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'#0'(bytes: Vec (int)): #0;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'#0'(bytes);
$IsValid'#0'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'#1'(bytes: Vec (int)): #1;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'#1'(bytes);
$IsValid'#1'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u8'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u8'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u64'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u64'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u256'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u256'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'address'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'address'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'signer'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'signer'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'u8''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'u8''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'address''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'address''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'#0''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'#0''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_option_Option'address''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_option_Option'address''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_type_info_TypeInfo'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_type_info_TypeInfo'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_GUID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_GUID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_ID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_ID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_object_TransferEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'#0''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'#0''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_account_Account'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_account_Account'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ConstructorRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ConstructorRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_DeleteRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_DeleteRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_DeriveRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_DeriveRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ExtendRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ExtendRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_LinearTransferRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_LinearTransferRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_Object'$1_object_ObjectCore''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_Object'$1_object_ObjectCore''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_Object'#0''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_Object'#0''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_Object'#1''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_Object'#1''(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_ObjectCore'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_ObjectCore'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_TransferEvent'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_TransferEvent'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_object_TransferRef'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_object_TransferRef'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'#0'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'#0'(bytes);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'#1'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'#1'(bytes);
$IsValid'bool'($$res)));

// fun from_bcs::to_address [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:47:5+84
procedure {:inline 1} $1_from_bcs_to_address(_$t0: Vec (int)) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: bool;
    var $t3: int;
    var $t0: Vec (int);
    var $temp_0'address': int;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[v]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:47:5+1
    assume {:print "$at(48,1348,1349)"} true;
    assume {:print "$track_local(14,1,0):", $t0} $t0 == $t0;

    // $t1 := opaque begin: from_bcs::from_bytes<address>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    assume {:print "$at(48,1404,1426)"} true;

    // assume Identical($t2, Not(from_bcs::deserializable<address>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    assume ($t2 == !$1_from_bcs_deserializable'address'($t0));

    // if ($t2) goto L4 else goto L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    if ($t2) { goto L4; } else { goto L3; }

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
L4:

    // trace_abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    assume {:print "$at(48,1404,1426)"} true;
    assume {:print "$track_abort(14,1):", $t3} $t3 == $t3;

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    goto L2;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
L3:

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    assume {:print "$at(48,1404,1426)"} true;
    assume $IsValid'address'($t1);

    // assume Eq<address>($t1, from_bcs::deserialize<address>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    assume $IsEqual'address'($t1, $1_from_bcs_deserialize'address'($t0));

    // $t1 := opaque end: from_bcs::from_bytes<address>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:48:9+22
    assume {:print "$track_return(14,1,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:49:5+1
    assume {:print "$at(48,1431,1432)"} true;
L1:

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:49:5+1
    assume {:print "$at(48,1431,1432)"} true;
    $ret0 := $t1;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:49:5+1
L2:

    // abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.move:49:5+1
    assume {:print "$at(48,1431,1432)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// struct event::EventHandle<account::CoinRegisterEvent> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
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

// struct event::EventHandle<account::KeyRotationEvent> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
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

// struct event::EventHandle<object::TransferEvent> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
type {:datatype} $1_event_EventHandle'$1_object_TransferEvent';
function {:constructor} $1_event_EventHandle'$1_object_TransferEvent'($counter: int, $guid: $1_guid_GUID): $1_event_EventHandle'$1_object_TransferEvent';
function {:inline} $Update'$1_event_EventHandle'$1_object_TransferEvent''_counter(s: $1_event_EventHandle'$1_object_TransferEvent', x: int): $1_event_EventHandle'$1_object_TransferEvent' {
    $1_event_EventHandle'$1_object_TransferEvent'(x, $guid#$1_event_EventHandle'$1_object_TransferEvent'(s))
}
function {:inline} $Update'$1_event_EventHandle'$1_object_TransferEvent''_guid(s: $1_event_EventHandle'$1_object_TransferEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_object_TransferEvent' {
    $1_event_EventHandle'$1_object_TransferEvent'($counter#$1_event_EventHandle'$1_object_TransferEvent'(s), x)
}
function $IsValid'$1_event_EventHandle'$1_object_TransferEvent''(s: $1_event_EventHandle'$1_object_TransferEvent'): bool {
    $IsValid'u64'($counter#$1_event_EventHandle'$1_object_TransferEvent'(s))
      && $IsValid'$1_guid_GUID'($guid#$1_event_EventHandle'$1_object_TransferEvent'(s))
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_object_TransferEvent''(s1: $1_event_EventHandle'$1_object_TransferEvent', s2: $1_event_EventHandle'$1_object_TransferEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<#0> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:16:5+224
type {:datatype} $1_event_EventHandle'#0';
function {:constructor} $1_event_EventHandle'#0'($counter: int, $guid: $1_guid_GUID): $1_event_EventHandle'#0';
function {:inline} $Update'$1_event_EventHandle'#0''_counter(s: $1_event_EventHandle'#0', x: int): $1_event_EventHandle'#0' {
    $1_event_EventHandle'#0'(x, $guid#$1_event_EventHandle'#0'(s))
}
function {:inline} $Update'$1_event_EventHandle'#0''_guid(s: $1_event_EventHandle'#0', x: $1_guid_GUID): $1_event_EventHandle'#0' {
    $1_event_EventHandle'#0'($counter#$1_event_EventHandle'#0'(s), x)
}
function $IsValid'$1_event_EventHandle'#0''(s: $1_event_EventHandle'#0'): bool {
    $IsValid'u64'($counter#$1_event_EventHandle'#0'(s))
      && $IsValid'$1_guid_GUID'($guid#$1_event_EventHandle'#0'(s))
}
function {:inline} $IsEqual'$1_event_EventHandle'#0''(s1: $1_event_EventHandle'#0', s2: $1_event_EventHandle'#0'): bool {
    s1 == s2
}

// fun event::destroy_handle<object::TransferEvent> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:54:5+131
procedure {:inline 1} $1_event_destroy_handle'$1_object_TransferEvent'(_$t0: $1_event_EventHandle'$1_object_TransferEvent') returns ()
{
    // declare local variables
    var $t1: int;
    var $t2: $1_guid_GUID;
    var $t0: $1_event_EventHandle'$1_object_TransferEvent';
    var $temp_0'$1_event_EventHandle'$1_object_TransferEvent'': $1_event_EventHandle'$1_object_TransferEvent';
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[handle]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:54:5+1
    assume {:print "$at(110,2111,2112)"} true;
    assume {:print "$track_local(15,1,0):", $t0} $t0 == $t0;

    // ($t1, $t2) := unpack event::EventHandle<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:55:9+38
    assume {:print "$at(110,2188,2226)"} true;
    $t1 := $counter#$1_event_EventHandle'$1_object_TransferEvent'($t0);
    $t2 := $guid#$1_event_EventHandle'$1_object_TransferEvent'($t0);

    // destroy($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:55:44+1

    // destroy($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:55:35+1

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:56:5+1
    assume {:print "$at(110,2241,2242)"} true;
L1:

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:56:5+1
    assume {:print "$at(110,2241,2242)"} true;
    return;

}

// fun event::new_event_handle<object::TransferEvent> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:24:5+165
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
    // trace_local[guid]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:24:5+1
    assume {:print "$at(110,978,979)"} true;
    assume {:print "$track_local(15,4,0):", $t0} $t0 == $t0;

    // $t1 := 0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:26:22+1
    assume {:print "$at(110,1107,1108)"} true;
    $t1 := 0;
    assume $IsValid'u64'($t1);

    // $t2 := pack event::EventHandle<#0>($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:25:9+68
    assume {:print "$at(110,1069,1137)"} true;
    $t2 := $1_event_EventHandle'$1_object_TransferEvent'($t1, $t0);

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:25:9+68
    assume {:print "$track_return(15,4,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:29:5+1
    assume {:print "$at(110,1142,1143)"} true;
L1:

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:29:5+1
    assume {:print "$at(110,1142,1143)"} true;
    $ret0 := $t2;
    return;

}

// fun event::new_event_handle<#0> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:24:5+165
procedure {:inline 1} $1_event_new_event_handle'#0'(_$t0: $1_guid_GUID) returns ($ret0: $1_event_EventHandle'#0')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_event_EventHandle'#0';
    var $t0: $1_guid_GUID;
    var $temp_0'$1_event_EventHandle'#0'': $1_event_EventHandle'#0';
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[guid]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:24:5+1
    assume {:print "$at(110,978,979)"} true;
    assume {:print "$track_local(15,4,0):", $t0} $t0 == $t0;

    // $t1 := 0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:26:22+1
    assume {:print "$at(110,1107,1108)"} true;
    $t1 := 0;
    assume $IsValid'u64'($t1);

    // $t2 := pack event::EventHandle<#0>($t1, $t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:25:9+68
    assume {:print "$at(110,1069,1137)"} true;
    $t2 := $1_event_EventHandle'#0'($t1, $t0);

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:25:9+68
    assume {:print "$track_return(15,4,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:29:5+1
    assume {:print "$at(110,1142,1143)"} true;
L1:

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:29:5+1
    assume {:print "$at(110,1142,1143)"} true;
    $ret0 := $t2;
    return;

}

// struct account::Account at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:26:5+401
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

// struct account::CapabilityOffer<account::RotationCapability> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:45:5+68
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

// struct account::CapabilityOffer<account::SignerCapability> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:45:5+68
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

// struct account::CoinRegisterEvent at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:41:5+77
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

// struct account::KeyRotationEvent at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:36:5+135
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

// struct account::RotationCapability at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:47:5+62
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

// struct account::SignerCapability at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:49:5+60
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

// fun account::create_guid [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:654:5+436
procedure {:inline 1} $1_account_create_guid(_$t0: $signer) returns ($ret0: $1_guid_GUID)
{
    // declare local variables
    var $t1: $Mutation ($1_account_Account);
    var $t2: int;
    var $t3: $1_guid_GUID;
    var $t4: int;
    var $t5: int;
    var $t6: $1_account_Account;
    var $t7: int;
    var $t8: int;
    var $t9: $Mutation ($1_account_Account);
    var $t10: $Mutation (int);
    var $t11: $1_guid_GUID;
    var $t12: int;
    var $t13: int;
    var $t14: bool;
    var $t15: int;
    var $t16: int;
    var $t0: $signer;
    var $1_account_Account_$modifies: [int]bool;
    var $temp_0'$1_account_Account': $1_account_Account;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // bytecode translation starts here
    // assume Identical($t4, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:416:9+46
    assume {:print "$at(73,20442,20488)"} true;
    assume ($t4 == $1_signer_$address_of($t0));

    // assume Identical($t5, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:430:9+39
    assume {:print "$at(73,20934,20973)"} true;
    assume ($t5 == $1_signer_$address_of($t0));

    // assume Identical($t6, global<account::Account>($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:431:9+36
    assume {:print "$at(73,20982,21018)"} true;
    assume ($t6 == $ResourceValue($1_account_Account_$memory, $t5));

    // trace_local[account_signer]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:654:5+1
    assume {:print "$at(72,37618,37619)"} true;
    assume {:print "$track_local(18,5,0):", $t0} $t0 == $t0;

    // $t7 := signer::address_of($t0) on_abort goto L4 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:655:20+34
    assume {:print "$at(72,37716,37750)"} true;
    call $t7 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(72,37716,37750)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(18,5):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_local[addr]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:655:13+4
    assume {:print "$track_local(18,5,2):", $t7} $t7 == $t7;

    // $t9 := borrow_global<account::Account>($t7) on_abort goto L4 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:656:23+17
    assume {:print "$at(72,37774,37791)"} true;
    if (!$ResourceExists($1_account_Account_$memory, $t7)) {
        call $ExecFailureAbort();
    } else {
        $t9 := $Mutation($Global($t7), EmptyVec(), $ResourceValue($1_account_Account_$memory, $t7));
    }
    if ($abort_flag) {
        assume {:print "$at(72,37774,37791)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(18,5):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_local[account]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:656:13+7
    $temp_0'$1_account_Account' := $Dereference($t9);
    assume {:print "$track_local(18,5,1):", $temp_0'$1_account_Account'} $temp_0'$1_account_Account' == $temp_0'$1_account_Account';

    // $t10 := borrow_field<account::Account>.guid_creation_num($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:657:39+30
    assume {:print "$at(72,37846,37876)"} true;
    $t10 := $ChildMutation($t9, 2, $guid_creation_num#$1_account_Account($Dereference($t9)));

    // $t11 := guid::create($t7, $t10) on_abort goto L4 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:657:20+50
    call $t11,$t10 := $1_guid_create($t7, $t10);
    if ($abort_flag) {
        assume {:print "$at(72,37827,37877)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(18,5):", $t8} $t8 == $t8;
        goto L4;
    }

    // write_back[Reference($t9).guid_creation_num (u64)]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:657:20+50
    $t9 := $UpdateMutation($t9, $Update'$1_account_Account'_guid_creation_num($Dereference($t9), $Dereference($t10)));

    // trace_local[guid]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:657:13+4
    assume {:print "$track_local(18,5,3):", $t11} $t11 == $t11;

    // $t12 := get_field<account::Account>.guid_creation_num($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:659:13+25
    assume {:print "$at(72,37908,37933)"} true;
    $t12 := $guid_creation_num#$1_account_Account($Dereference($t9));

    // pack_ref_deep($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:659:13+25

    // write_back[account::Account@]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:659:13+25
    $1_account_Account_$memory := $ResourceUpdate($1_account_Account_$memory, $GlobalLocationAddress($t9),
        $Dereference($t9));

    // $t13 := 1125899906842624 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:659:41+21
    $t13 := 1125899906842624;
    assume $IsValid'u64'($t13);

    // $t14 := <($t12, $t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:659:39+1
    call $t14 := $Lt($t12, $t13);

    // if ($t14) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:658:9+147
    assume {:print "$at(72,37887,38034)"} true;
    if ($t14) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:658:9+147
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:658:9+147
    assume {:print "$at(72,37887,38034)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:660:33+31
    assume {:print "$at(72,37991,38022)"} true;
L0:

    // $t15 := 20 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:660:33+31
    assume {:print "$at(72,37991,38022)"} true;
    $t15 := 20;
    assume $IsValid'u64'($t15);

    // $t16 := error::out_of_range($t15) on_abort goto L4 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:660:13+52
    call $t16 := $1_error_out_of_range($t15);
    if ($abort_flag) {
        assume {:print "$at(72,37971,38023)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(18,5):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_abort($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:658:9+147
    assume {:print "$at(72,37887,38034)"} true;
    assume {:print "$track_abort(18,5):", $t16} $t16 == $t16;

    // $t8 := move($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:658:9+147
    $t8 := $t16;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:658:9+147
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:662:9+4
    assume {:print "$at(72,38044,38048)"} true;
L2:

    // trace_return[0]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:662:9+4
    assume {:print "$at(72,38044,38048)"} true;
    assume {:print "$track_return(18,5,0):", $t11} $t11 == $t11;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:663:5+1
    assume {:print "$at(72,38053,38054)"} true;
L3:

    // return $t11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:663:5+1
    assume {:print "$at(72,38053,38054)"} true;
    $ret0 := $t11;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:663:5+1
L4:

    // abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.move:663:5+1
    assume {:print "$at(72,38053,38054)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:7:10+50
function  $1_object_spec_exists_at'#0'(object: int): bool;
axiom (forall object: int ::
(var $$res := $1_object_spec_exists_at'#0'(object);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:7:10+50
function  $1_object_spec_exists_at'#1'(object: int): bool;
axiom (forall object: int ::
(var $$res := $1_object_spec_exists_at'#1'(object);
$IsValid'bool'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:242:10+75
function  $1_object_spec_create_object_address(source: int, seed: Vec (int)): int;
axiom (forall source: int, seed: Vec (int) ::
(var $$res := $1_object_spec_create_object_address(source, seed);
$IsValid'address'($$res)));

// spec fun at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:244:10+92
function  $1_object_spec_create_user_derived_object_address(source: int, derive_from: int): int;
axiom (forall source: int, derive_from: int ::
(var $$res := $1_object_spec_create_user_derived_object_address(source, derive_from);
$IsValid'address'($$res)));

// struct object::ConstructorRef at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:109:5+232
type {:datatype} $1_object_ConstructorRef;
function {:constructor} $1_object_ConstructorRef($self: int, $can_delete: bool): $1_object_ConstructorRef;
function {:inline} $Update'$1_object_ConstructorRef'_self(s: $1_object_ConstructorRef, x: int): $1_object_ConstructorRef {
    $1_object_ConstructorRef(x, $can_delete#$1_object_ConstructorRef(s))
}
function {:inline} $Update'$1_object_ConstructorRef'_can_delete(s: $1_object_ConstructorRef, x: bool): $1_object_ConstructorRef {
    $1_object_ConstructorRef($self#$1_object_ConstructorRef(s), x)
}
function $IsValid'$1_object_ConstructorRef'(s: $1_object_ConstructorRef): bool {
    $IsValid'address'($self#$1_object_ConstructorRef(s))
      && $IsValid'bool'($can_delete#$1_object_ConstructorRef(s))
}
function {:inline} $IsEqual'$1_object_ConstructorRef'(s1: $1_object_ConstructorRef, s2: $1_object_ConstructorRef): bool {
    s1 == s2
}

// struct object::DeleteRef at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:117:5+63
type {:datatype} $1_object_DeleteRef;
function {:constructor} $1_object_DeleteRef($self: int): $1_object_DeleteRef;
function {:inline} $Update'$1_object_DeleteRef'_self(s: $1_object_DeleteRef, x: int): $1_object_DeleteRef {
    $1_object_DeleteRef(x)
}
function $IsValid'$1_object_DeleteRef'(s: $1_object_DeleteRef): bool {
    $IsValid'address'($self#$1_object_DeleteRef(s))
}
function {:inline} $IsEqual'$1_object_DeleteRef'(s1: $1_object_DeleteRef, s2: $1_object_DeleteRef): bool {
    s1 == s2
}

// struct object::DeriveRef at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:139:5+63
type {:datatype} $1_object_DeriveRef;
function {:constructor} $1_object_DeriveRef($self: int): $1_object_DeriveRef;
function {:inline} $Update'$1_object_DeriveRef'_self(s: $1_object_DeriveRef, x: int): $1_object_DeriveRef {
    $1_object_DeriveRef(x)
}
function $IsValid'$1_object_DeriveRef'(s: $1_object_DeriveRef): bool {
    $IsValid'address'($self#$1_object_DeriveRef(s))
}
function {:inline} $IsEqual'$1_object_DeriveRef'(s1: $1_object_DeriveRef, s2: $1_object_DeriveRef): bool {
    s1 == s2
}

// struct object::ExtendRef at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:122:5+63
type {:datatype} $1_object_ExtendRef;
function {:constructor} $1_object_ExtendRef($self: int): $1_object_ExtendRef;
function {:inline} $Update'$1_object_ExtendRef'_self(s: $1_object_ExtendRef, x: int): $1_object_ExtendRef {
    $1_object_ExtendRef(x)
}
function $IsValid'$1_object_ExtendRef'(s: $1_object_ExtendRef): bool {
    $IsValid'address'($self#$1_object_ExtendRef(s))
}
function {:inline} $IsEqual'$1_object_ExtendRef'(s1: $1_object_ExtendRef, s2: $1_object_ExtendRef): bool {
    s1 == s2
}

// struct object::LinearTransferRef at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:133:5+88
type {:datatype} $1_object_LinearTransferRef;
function {:constructor} $1_object_LinearTransferRef($self: int, $owner: int): $1_object_LinearTransferRef;
function {:inline} $Update'$1_object_LinearTransferRef'_self(s: $1_object_LinearTransferRef, x: int): $1_object_LinearTransferRef {
    $1_object_LinearTransferRef(x, $owner#$1_object_LinearTransferRef(s))
}
function {:inline} $Update'$1_object_LinearTransferRef'_owner(s: $1_object_LinearTransferRef, x: int): $1_object_LinearTransferRef {
    $1_object_LinearTransferRef($self#$1_object_LinearTransferRef(s), x)
}
function $IsValid'$1_object_LinearTransferRef'(s: $1_object_LinearTransferRef): bool {
    $IsValid'address'($self#$1_object_LinearTransferRef(s))
      && $IsValid'address'($owner#$1_object_LinearTransferRef(s))
}
function {:inline} $IsEqual'$1_object_LinearTransferRef'(s1: $1_object_LinearTransferRef, s2: $1_object_LinearTransferRef): bool {
    s1 == s2
}

// struct object::Object<object::ObjectCore> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:104:5+78
type {:datatype} $1_object_Object'$1_object_ObjectCore';
function {:constructor} $1_object_Object'$1_object_ObjectCore'($inner: int): $1_object_Object'$1_object_ObjectCore';
function {:inline} $Update'$1_object_Object'$1_object_ObjectCore''_inner(s: $1_object_Object'$1_object_ObjectCore', x: int): $1_object_Object'$1_object_ObjectCore' {
    $1_object_Object'$1_object_ObjectCore'(x)
}
function $IsValid'$1_object_Object'$1_object_ObjectCore''(s: $1_object_Object'$1_object_ObjectCore'): bool {
    $IsValid'address'($inner#$1_object_Object'$1_object_ObjectCore'(s))
}
function {:inline} $IsEqual'$1_object_Object'$1_object_ObjectCore''(s1: $1_object_Object'$1_object_ObjectCore', s2: $1_object_Object'$1_object_ObjectCore'): bool {
    s1 == s2
}

// struct object::Object<#0> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:104:5+78
type {:datatype} $1_object_Object'#0';
function {:constructor} $1_object_Object'#0'($inner: int): $1_object_Object'#0';
function {:inline} $Update'$1_object_Object'#0''_inner(s: $1_object_Object'#0', x: int): $1_object_Object'#0' {
    $1_object_Object'#0'(x)
}
function $IsValid'$1_object_Object'#0''(s: $1_object_Object'#0'): bool {
    $IsValid'address'($inner#$1_object_Object'#0'(s))
}
function {:inline} $IsEqual'$1_object_Object'#0''(s1: $1_object_Object'#0', s2: $1_object_Object'#0'): bool {
    s1 == s2
}

// struct object::Object<#1> at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:104:5+78
type {:datatype} $1_object_Object'#1';
function {:constructor} $1_object_Object'#1'($inner: int): $1_object_Object'#1';
function {:inline} $Update'$1_object_Object'#1''_inner(s: $1_object_Object'#1', x: int): $1_object_Object'#1' {
    $1_object_Object'#1'(x)
}
function $IsValid'$1_object_Object'#1''(s: $1_object_Object'#1'): bool {
    $IsValid'address'($inner#$1_object_Object'#1'(s))
}
function {:inline} $IsEqual'$1_object_Object'#1''(s1: $1_object_Object'#1', s2: $1_object_Object'#1'): bool {
    s1 == s2
}

// struct object::ObjectCore at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:84:5+551
type {:datatype} $1_object_ObjectCore;
function {:constructor} $1_object_ObjectCore($guid_creation_num: int, $owner: int, $allow_ungated_transfer: bool, $transfer_events: $1_event_EventHandle'$1_object_TransferEvent'): $1_object_ObjectCore;
function {:inline} $Update'$1_object_ObjectCore'_guid_creation_num(s: $1_object_ObjectCore, x: int): $1_object_ObjectCore {
    $1_object_ObjectCore(x, $owner#$1_object_ObjectCore(s), $allow_ungated_transfer#$1_object_ObjectCore(s), $transfer_events#$1_object_ObjectCore(s))
}
function {:inline} $Update'$1_object_ObjectCore'_owner(s: $1_object_ObjectCore, x: int): $1_object_ObjectCore {
    $1_object_ObjectCore($guid_creation_num#$1_object_ObjectCore(s), x, $allow_ungated_transfer#$1_object_ObjectCore(s), $transfer_events#$1_object_ObjectCore(s))
}
function {:inline} $Update'$1_object_ObjectCore'_allow_ungated_transfer(s: $1_object_ObjectCore, x: bool): $1_object_ObjectCore {
    $1_object_ObjectCore($guid_creation_num#$1_object_ObjectCore(s), $owner#$1_object_ObjectCore(s), x, $transfer_events#$1_object_ObjectCore(s))
}
function {:inline} $Update'$1_object_ObjectCore'_transfer_events(s: $1_object_ObjectCore, x: $1_event_EventHandle'$1_object_TransferEvent'): $1_object_ObjectCore {
    $1_object_ObjectCore($guid_creation_num#$1_object_ObjectCore(s), $owner#$1_object_ObjectCore(s), $allow_ungated_transfer#$1_object_ObjectCore(s), x)
}
function $IsValid'$1_object_ObjectCore'(s: $1_object_ObjectCore): bool {
    $IsValid'u64'($guid_creation_num#$1_object_ObjectCore(s))
      && $IsValid'address'($owner#$1_object_ObjectCore(s))
      && $IsValid'bool'($allow_ungated_transfer#$1_object_ObjectCore(s))
      && $IsValid'$1_event_EventHandle'$1_object_TransferEvent''($transfer_events#$1_object_ObjectCore(s))
}
function {:inline} $IsEqual'$1_object_ObjectCore'(s1: $1_object_ObjectCore, s2: $1_object_ObjectCore): bool {
    s1 == s2
}
var $1_object_ObjectCore_$memory: $Memory $1_object_ObjectCore;

// struct object::TransferEvent at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:144:5+113
type {:datatype} $1_object_TransferEvent;
function {:constructor} $1_object_TransferEvent($object: int, $from: int, $to: int): $1_object_TransferEvent;
function {:inline} $Update'$1_object_TransferEvent'_object(s: $1_object_TransferEvent, x: int): $1_object_TransferEvent {
    $1_object_TransferEvent(x, $from#$1_object_TransferEvent(s), $to#$1_object_TransferEvent(s))
}
function {:inline} $Update'$1_object_TransferEvent'_from(s: $1_object_TransferEvent, x: int): $1_object_TransferEvent {
    $1_object_TransferEvent($object#$1_object_TransferEvent(s), x, $to#$1_object_TransferEvent(s))
}
function {:inline} $Update'$1_object_TransferEvent'_to(s: $1_object_TransferEvent, x: int): $1_object_TransferEvent {
    $1_object_TransferEvent($object#$1_object_TransferEvent(s), $from#$1_object_TransferEvent(s), x)
}
function $IsValid'$1_object_TransferEvent'(s: $1_object_TransferEvent): bool {
    $IsValid'address'($object#$1_object_TransferEvent(s))
      && $IsValid'address'($from#$1_object_TransferEvent(s))
      && $IsValid'address'($to#$1_object_TransferEvent(s))
}
function {:inline} $IsEqual'$1_object_TransferEvent'(s1: $1_object_TransferEvent, s2: $1_object_TransferEvent): bool {
    s1 == s2
}

// struct object::TransferRef at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:127:5+65
type {:datatype} $1_object_TransferRef;
function {:constructor} $1_object_TransferRef($self: int): $1_object_TransferRef;
function {:inline} $Update'$1_object_TransferRef'_self(s: $1_object_TransferRef, x: int): $1_object_TransferRef {
    $1_object_TransferRef(x)
}
function $IsValid'$1_object_TransferRef'(s: $1_object_TransferRef): bool {
    $IsValid'address'($self#$1_object_TransferRef(s))
}
function {:inline} $IsEqual'$1_object_TransferRef'(s1: $1_object_TransferRef, s2: $1_object_TransferRef): bool {
    s1 == s2
}

// fun object::new_event_handle [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:311:5+180
procedure {:timeLimit 40} $1_object_new_event_handle$verify(_$t0: $signer) returns ($ret0: $1_event_EventHandle'#0')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_ObjectCore;
    var $t3: $1_object_ObjectCore;
    var $t4: $1_object_ObjectCore;
    var $t5: $1_object_ObjectCore;
    var $t6: $1_guid_GUID;
    var $t7: int;
    var $t8: $1_event_EventHandle'#0';
    var $t9: int;
    var $t10: bool;
    var $t11: bool;
    var $t12: bool;
    var $t13: int;
    var $t14: bool;
    var $t15: bool;
    var $t16: bool;
    var $t0: $signer;
    var $temp_0'$1_event_EventHandle'#0'': $1_event_EventHandle'#0';
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $1_object_ObjectCore_$memory#40: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:311:5+1
    assume {:print "$at(2,13396,13397)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:311:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:65+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:65+6
    assume {:print "$at(3,4965,4971)"} true;
    assume {:print "$track_exp_sub(25935):", $t0} true;

    // assume Identical($t1, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:46+26
    assume ($t1 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:46+26]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:46+26
    assume {:print "$track_exp_sub(25936):", $t1} true;

    // assume Identical($t2, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:27+46
    assume ($t2 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:27+46]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:27+46
    assume {:print "$track_exp_sub(25937):", $t2} true;

    // assume Identical($t3, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:9+65
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:9+65]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:9+65
    assume {:print "$track_exp(25938):", $t3} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:9+65
    assume {:print "$track_global_mem(27218):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t4, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:131:9+65
    assume ($t4 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // @40 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:311:5+1
    assume {:print "$at(2,13396,13397)"} true;
    $1_object_ObjectCore_$memory#40 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:311:5+1
    assume {:print "$track_local(52,29,0):", $t0} $t0 == $t0;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:314:33+19
    assume {:print "$at(2,13550,13569)"} true;
    assume {:print "$track_global_mem(27219):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65
    assume {:print "$at(3,4573,4638)"} true;
    assume ($t5 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // $t6 := object::create_guid($t0) on_abort goto L2 with $t7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:314:33+19
    assume {:print "$at(2,13550,13569)"} true;
    call $t6 := $1_object_create_guid($t0);
    if ($abort_flag) {
        assume {:print "$at(2,13550,13569)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(52,29):", $t7} $t7 == $t7;
        goto L2;
    }

    // $t8 := event::new_event_handle<#0>($t6) on_abort goto L2 with $t7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:314:9+44
    call $t8 := $1_event_new_event_handle'#0'($t6);
    if ($abort_flag) {
        assume {:print "$at(2,13526,13570)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(52,29):", $t7} $t7 == $t7;
        goto L2;
    }

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:314:9+44
    assume {:print "$track_return(52,29,0):", $t8} $t8 == $t8;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:315:5+1
    assume {:print "$at(2,13575,13576)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:58+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:58+6
    assume {:print "$at(3,4865,4871)"} true;
    assume {:print "$track_exp_sub(25944):", $t0} true;

    // assume Identical($t9, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:39+26
    assume ($t9 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:39+26]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:39+26
    assume {:print "$track_exp_sub(25945):", $t9} true;

    // assume Identical($t10, exists[@40]<object::ObjectCore>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:20+46
    assume ($t10 == $ResourceExists($1_object_ObjectCore_$memory#40, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:20+46]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:20+46
    assume {:print "$track_exp_sub(25946):", $t10} true;

    // assume Identical($t11, Not(exists[@40]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58
    assume ($t11 == !$ResourceExists($1_object_ObjectCore_$memory#40, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58
    assume {:print "$track_exp(25947):", $t11} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58
    assume {:print "$track_global_mem(27220):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@40]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58
    assert {:msg "assert_failed(3,4816,4874): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#40, $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:19+11]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:19+11
    assume {:print "$at(3,4993,5004)"} true;
    assume {:print "$track_exp_sub(25951):", $t4} true;

    // assume Identical($t12, Gt(Add(select object::ObjectCore.guid_creation_num($t4), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    assume ($t12 == (($guid_creation_num#$1_object_ObjectCore($t4) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    assume {:print "$track_exp(25952):", $t12} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    assume {:print "$track_global_mem(27221):", $1_object_ObjectCore_$memory} true;

    // assert Not(Gt(Add(select object::ObjectCore.guid_creation_num($t4), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    assert {:msg "assert_failed(3,4983,5037): function does not abort under this condition"}
      !(($guid_creation_num#$1_object_ObjectCore($t4) + 1) > 18446744073709551615);

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    $ret0 := $t8;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:315:5+1
    assume {:print "$at(2,13575,13576)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:58+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:58+6
    assume {:print "$at(3,4865,4871)"} true;
    assume {:print "$track_exp_sub(25944):", $t0} true;

    // assume Identical($t13, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:39+26
    assume ($t13 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:39+26]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:39+26
    assume {:print "$track_exp_sub(25945):", $t13} true;

    // assume Identical($t14, exists[@40]<object::ObjectCore>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:20+46
    assume ($t14 == $ResourceExists($1_object_ObjectCore_$memory#40, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:20+46]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:20+46
    assume {:print "$track_exp_sub(25946):", $t14} true;

    // assume Identical($t15, Not(exists[@40]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58
    assume ($t15 == !$ResourceExists($1_object_ObjectCore_$memory#40, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:129:9+58
    assume {:print "$track_exp(25947):", $t15} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:19+11]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:19+11
    assume {:print "$at(3,4993,5004)"} true;
    assume {:print "$track_exp_sub(25951):", $t4} true;

    // assume Identical($t16, Gt(Add(select object::ObjectCore.guid_creation_num($t4), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    assume ($t16 == (($guid_creation_num#$1_object_ObjectCore($t4) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    assume {:print "$track_exp(25952):", $t16} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:132:9+54
    assume {:print "$track_global_mem(27222):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@40]<object::ObjectCore>(signer::$address_of[]($t0))), Gt(Add(select object::ObjectCore.guid_creation_num($t4), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:126:5+330
    assume {:print "$at(3,4713,5043)"} true;
    assert {:msg "assert_failed(3,4713,5043): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#40, $1_signer_$address_of($t0)) || (($guid_creation_num#$1_object_ObjectCore($t4) + 1) > 18446744073709551615));

    // abort($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:126:5+330
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun object::create_guid [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:304:5+252
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
    // assume Identical($t3, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65
    assume {:print "$at(3,4573,4638)"} true;
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:304:5+1
    assume {:print "$at(2,13101,13102)"} true;
    assume {:print "$track_local(52,6,0):", $t0} $t0 == $t0;

    // $t4 := signer::address_of($t0) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:305:20+26
    assume {:print "$at(2,13194,13220)"} true;
    call $t4 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,13194,13220)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,6):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_local[addr]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:305:13+4
    assume {:print "$track_local(52,6,1):", $t4} $t4 == $t4;

    // $t6 := borrow_global<object::ObjectCore>($t4) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:306:27+17
    assume {:print "$at(2,13248,13265)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t4)) {
        call $ExecFailureAbort();
    } else {
        $t6 := $Mutation($Global($t4), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t4));
    }
    if ($abort_flag) {
        assume {:print "$at(2,13248,13265)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,6):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_local[object_data]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:306:13+11
    $temp_0'$1_object_ObjectCore' := $Dereference($t6);
    assume {:print "$track_local(52,6,2):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t7 := borrow_field<object::ObjectCore>.guid_creation_num($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:28+34
    assume {:print "$at(2,13312,13346)"} true;
    $t7 := $ChildMutation($t6, 0, $guid_creation_num#$1_object_ObjectCore($Dereference($t6)));

    // $t8 := guid::create($t4, $t7) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    call $t8,$t7 := $1_guid_create($t4, $t7);
    if ($abort_flag) {
        assume {:print "$at(2,13293,13347)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,6):", $t5} $t5 == $t5;
        goto L2;
    }

    // write_back[Reference($t6).guid_creation_num (u64)]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    $t6 := $UpdateMutation($t6, $Update'$1_object_ObjectCore'_guid_creation_num($Dereference($t6), $Dereference($t7)));

    // write_back[object::ObjectCore@]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t6),
        $Dereference($t6));

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    assume {:print "$track_return(52,6,0):", $t8} $t8 == $t8;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:308:5+1
    assume {:print "$at(2,13352,13353)"} true;
L1:

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:308:5+1
    assume {:print "$at(2,13352,13353)"} true;
    $ret0 := $t8;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:308:5+1
L2:

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:308:5+1
    assume {:print "$at(2,13352,13353)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::create_guid [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:304:5+252
procedure {:timeLimit 40} $1_object_create_guid$verify(_$t0: $signer) returns ($ret0: $1_guid_GUID)
{
    // declare local variables
    var $t1: int;
    var $t2: $Mutation ($1_object_ObjectCore);
    var $t3: int;
    var $t4: $1_object_ObjectCore;
    var $t5: $1_object_ObjectCore;
    var $t6: $1_object_ObjectCore;
    var $t7: int;
    var $t8: int;
    var $t9: $Mutation ($1_object_ObjectCore);
    var $t10: $Mutation (int);
    var $t11: $1_guid_GUID;
    var $t12: int;
    var $t13: bool;
    var $t14: bool;
    var $t15: bool;
    var $t16: int;
    var $t17: bool;
    var $t18: bool;
    var $t19: bool;
    var $t0: $signer;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $1_object_ObjectCore_$memory#29: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:304:5+1
    assume {:print "$at(2,13101,13102)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:304:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:65+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:65+6
    assume {:print "$at(3,4629,4635)"} true;
    assume {:print "$track_exp_sub(25536):", $t0} true;

    // assume Identical($t3, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:46+26
    assume ($t3 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:46+26]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:46+26
    assume {:print "$track_exp_sub(25537):", $t3} true;

    // assume Identical($t4, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:27+46
    assume ($t4 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:27+46]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:27+46
    assume {:print "$track_exp_sub(25538):", $t4} true;

    // assume Identical($t5, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65
    assume ($t5 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65
    assume {:print "$track_exp(25539):", $t5} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65
    assume {:print "$track_global_mem(27223):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t6, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65
    assume ($t6 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // @29 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:304:5+1
    assume {:print "$at(2,13101,13102)"} true;
    $1_object_ObjectCore_$memory#29 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:304:5+1
    assume {:print "$track_local(52,6,0):", $t0} $t0 == $t0;

    // $t7 := signer::address_of($t0) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:305:20+26
    assume {:print "$at(2,13194,13220)"} true;
    call $t7 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,13194,13220)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,6):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_local[addr]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:305:13+4
    assume {:print "$track_local(52,6,1):", $t7} $t7 == $t7;

    // $t9 := borrow_global<object::ObjectCore>($t7) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:306:27+17
    assume {:print "$at(2,13248,13265)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t7)) {
        call $ExecFailureAbort();
    } else {
        $t9 := $Mutation($Global($t7), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t7));
    }
    if ($abort_flag) {
        assume {:print "$at(2,13248,13265)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,6):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_local[object_data]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:306:13+11
    $temp_0'$1_object_ObjectCore' := $Dereference($t9);
    assume {:print "$track_local(52,6,2):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t10 := borrow_field<object::ObjectCore>.guid_creation_num($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:28+34
    assume {:print "$at(2,13312,13346)"} true;
    $t10 := $ChildMutation($t9, 0, $guid_creation_num#$1_object_ObjectCore($Dereference($t9)));

    // $t11 := guid::create($t7, $t10) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    call $t11,$t10 := $1_guid_create($t7, $t10);
    if ($abort_flag) {
        assume {:print "$at(2,13293,13347)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,6):", $t8} $t8 == $t8;
        goto L2;
    }

    // write_back[Reference($t9).guid_creation_num (u64)]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    $t9 := $UpdateMutation($t9, $Update'$1_object_ObjectCore'_guid_creation_num($Dereference($t9), $Dereference($t10)));

    // write_back[object::ObjectCore@]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t9),
        $Dereference($t9));

    // trace_return[0]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:307:9+54
    assume {:print "$track_return(52,6,0):", $t11} $t11 == $t11;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:308:5+1
    assume {:print "$at(2,13352,13353)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:58+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:58+6
    assume {:print "$at(3,4529,4535)"} true;
    assume {:print "$track_exp_sub(25545):", $t0} true;

    // assume Identical($t12, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:39+26
    assume ($t12 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:39+26]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:39+26
    assume {:print "$track_exp_sub(25546):", $t12} true;

    // assume Identical($t13, exists[@29]<object::ObjectCore>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:20+46
    assume ($t13 == $ResourceExists($1_object_ObjectCore_$memory#29, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:20+46]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:20+46
    assume {:print "$track_exp_sub(25547):", $t13} true;

    // assume Identical($t14, Not(exists[@29]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58
    assume ($t14 == !$ResourceExists($1_object_ObjectCore_$memory#29, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58
    assume {:print "$track_exp(25548):", $t14} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58
    assume {:print "$track_global_mem(27224):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@29]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58
    assert {:msg "assert_failed(3,4480,4538): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#29, $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:19+11]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:19+11
    assume {:print "$at(3,4657,4668)"} true;
    assume {:print "$track_exp_sub(25552):", $t6} true;

    // assume Identical($t15, Gt(Add(select object::ObjectCore.guid_creation_num($t6), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    assume ($t15 == (($guid_creation_num#$1_object_ObjectCore($t6) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    assume {:print "$track_exp(25553):", $t15} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    assume {:print "$track_global_mem(27225):", $1_object_ObjectCore_$memory} true;

    // assert Not(Gt(Add(select object::ObjectCore.guid_creation_num($t6), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    assert {:msg "assert_failed(3,4647,4701): function does not abort under this condition"}
      !(($guid_creation_num#$1_object_ObjectCore($t6) + 1) > 18446744073709551615);

    // return $t11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    $ret0 := $t11;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:308:5+1
    assume {:print "$at(2,13352,13353)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:58+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:58+6
    assume {:print "$at(3,4529,4535)"} true;
    assume {:print "$track_exp_sub(25545):", $t0} true;

    // assume Identical($t16, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:39+26
    assume ($t16 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:39+26]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:39+26
    assume {:print "$track_exp_sub(25546):", $t16} true;

    // assume Identical($t17, exists[@29]<object::ObjectCore>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:20+46
    assume ($t17 == $ResourceExists($1_object_ObjectCore_$memory#29, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:20+46]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:20+46
    assume {:print "$track_exp_sub(25547):", $t17} true;

    // assume Identical($t18, Not(exists[@29]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58
    assume ($t18 == !$ResourceExists($1_object_ObjectCore_$memory#29, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:120:9+58
    assume {:print "$track_exp(25548):", $t18} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:19+11]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:19+11
    assume {:print "$at(3,4657,4668)"} true;
    assume {:print "$track_exp_sub(25552):", $t6} true;

    // assume Identical($t19, Gt(Add(select object::ObjectCore.guid_creation_num($t6), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    assume ($t19 == (($guid_creation_num#$1_object_ObjectCore($t6) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    assume {:print "$track_exp(25553):", $t19} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:123:9+54
    assume {:print "$track_global_mem(27226):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@29]<object::ObjectCore>(signer::$address_of[]($t0))), Gt(Add(select object::ObjectCore.guid_creation_num($t6), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:119:5+282
    assume {:print "$at(3,4425,4707)"} true;
    assert {:msg "assert_failed(3,4425,4707): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#29, $1_signer_$address_of($t0)) || (($guid_creation_num#$1_object_ObjectCore($t6) + 1) > 18446744073709551615));

    // abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:119:5+282
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun object::owner<object::ObjectCore> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+262
procedure {:inline 1} $1_object_owner'$1_object_ObjectCore'(_$t0: $1_object_Object'$1_object_ObjectCore') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $1_object_ObjectCore;
    var $t8: int;
    var $t0: $1_object_Object'$1_object_ObjectCore';
    var $temp_0'$1_object_Object'$1_object_ObjectCore'': $1_object_Object'$1_object_ObjectCore';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+1
    assume {:print "$at(2,20129,20130)"} true;
    assume {:print "$track_local(52,33,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:509:32+12
    assume {:print "$at(2,20252,20264)"} true;
    $t1 := $inner#$1_object_Object'$1_object_ObjectCore'($t0);

    // $t2 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:509:13+6
    $t2 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // if ($t2) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:30+22
    assume {:print "$at(2,20296,20318)"} true;
L0:

    // $t3 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:30+22
    assume {:print "$at(2,20296,20318)"} true;
    $t3 := 2;
    assume $IsValid'u64'($t3);

    // $t4 := error::not_found($t3) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:13+40
    call $t4 := $1_error_not_found($t3);
    if ($abort_flag) {
        assume {:print "$at(2,20279,20319)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,33):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    assume {:print "$track_abort(52,33):", $t4} $t4 == $t4;

    // $t5 := move($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    $t5 := $t4;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:35+6
    assume {:print "$at(2,20366,20372)"} true;
L2:

    // $t6 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:35+12
    assume {:print "$at(2,20366,20378)"} true;
    $t6 := $inner#$1_object_Object'$1_object_ObjectCore'($t0);

    // $t7 := get_global<object::ObjectCore>($t6) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+13
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t6)) {
        call $ExecFailureAbort();
    } else {
        $t7 := $ResourceValue($1_object_ObjectCore_$memory, $t6);
    }
    if ($abort_flag) {
        assume {:print "$at(2,20340,20353)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,33):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t8 := get_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+45
    $t8 := $owner#$1_object_ObjectCore($t7);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+45
    assume {:print "$track_return(52,33,0):", $t8} $t8 == $t8;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
L3:

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
    $ret0 := $t8;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
L4:

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::owner<#0> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+262
procedure {:inline 1} $1_object_owner'#0'(_$t0: $1_object_Object'#0') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $1_object_ObjectCore;
    var $t8: int;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+1
    assume {:print "$at(2,20129,20130)"} true;
    assume {:print "$track_local(52,33,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:509:32+12
    assume {:print "$at(2,20252,20264)"} true;
    $t1 := $inner#$1_object_Object'#0'($t0);

    // $t2 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:509:13+6
    $t2 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // if ($t2) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:30+22
    assume {:print "$at(2,20296,20318)"} true;
L0:

    // $t3 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:30+22
    assume {:print "$at(2,20296,20318)"} true;
    $t3 := 2;
    assume $IsValid'u64'($t3);

    // $t4 := error::not_found($t3) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:13+40
    call $t4 := $1_error_not_found($t3);
    if ($abort_flag) {
        assume {:print "$at(2,20279,20319)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,33):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    assume {:print "$track_abort(52,33):", $t4} $t4 == $t4;

    // $t5 := move($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    $t5 := $t4;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:35+6
    assume {:print "$at(2,20366,20372)"} true;
L2:

    // $t6 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:35+12
    assume {:print "$at(2,20366,20378)"} true;
    $t6 := $inner#$1_object_Object'#0'($t0);

    // $t7 := get_global<object::ObjectCore>($t6) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+13
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t6)) {
        call $ExecFailureAbort();
    } else {
        $t7 := $ResourceValue($1_object_ObjectCore_$memory, $t6);
    }
    if ($abort_flag) {
        assume {:print "$at(2,20340,20353)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,33):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t8 := get_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+45
    $t8 := $owner#$1_object_ObjectCore($t7);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+45
    assume {:print "$track_return(52,33,0):", $t8} $t8 == $t8;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
L3:

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
    $ret0 := $t8;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
L4:

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::owner [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+262
procedure {:timeLimit 40} $1_object_owner$verify(_$t0: $1_object_Object'#0') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $1_object_ObjectCore;
    var $t8: int;
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t12: bool;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#21: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+1
    assume {:print "$at(2,20129,20130)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @21 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+1
    $1_object_ObjectCore_$memory#21 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:507:5+1
    assume {:print "$track_local(52,33,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:509:32+12
    assume {:print "$at(2,20252,20264)"} true;
    $t1 := $inner#$1_object_Object'#0'($t0);

    // $t2 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:509:13+6
    $t2 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // if ($t2) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:30+22
    assume {:print "$at(2,20296,20318)"} true;
L0:

    // $t3 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:30+22
    assume {:print "$at(2,20296,20318)"} true;
    $t3 := 2;
    assume $IsValid'u64'($t3);

    // $t4 := error::not_found($t3) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:510:13+40
    call $t4 := $1_error_not_found($t3);
    if ($abort_flag) {
        assume {:print "$at(2,20279,20319)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,33):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    assume {:print "$at(2,20212,20330)"} true;
    assume {:print "$track_abort(52,33):", $t4} $t4 == $t4;

    // $t5 := move($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    $t5 := $t4;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:508:9+118
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:35+6
    assume {:print "$at(2,20366,20372)"} true;
L2:

    // $t6 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:35+12
    assume {:print "$at(2,20366,20378)"} true;
    $t6 := $inner#$1_object_Object'#0'($t0);

    // $t7 := get_global<object::ObjectCore>($t6) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+13
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t6)) {
        call $ExecFailureAbort();
    } else {
        $t7 := $ResourceValue($1_object_ObjectCore_$memory, $t6);
    }
    if ($abort_flag) {
        assume {:print "$at(2,20340,20353)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,33):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t8 := get_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+45
    $t8 := $owner#$1_object_ObjectCore($t7);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:512:9+45
    assume {:print "$track_return(52,33,0):", $t8} $t8 == $t8;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
L3:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:39+6
    assume {:print "$at(3,9073,9079)"} true;
    assume {:print "$track_exp_sub(25236):", $t0} true;

    // assume Identical($t9, exists[@21]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:20+32
    assume ($t9 == $ResourceExists($1_object_ObjectCore_$memory#21, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:20+32]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:20+32
    assume {:print "$track_exp_sub(25237):", $t9} true;

    // assume Identical($t10, Not(exists[@21]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    assume ($t10 == !$ResourceExists($1_object_ObjectCore_$memory#21, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    assume {:print "$track_exp(25238):", $t10} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    assume {:print "$track_global_mem(27227):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@21]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    assert {:msg "assert_failed(3,9043,9087): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#21, $inner#$1_object_Object'#0'($t0));

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    $ret0 := $t8;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:513:5+1
    assume {:print "$at(2,20390,20391)"} true;
L4:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:39+6
    assume {:print "$at(3,9073,9079)"} true;
    assume {:print "$track_exp_sub(25236):", $t0} true;

    // assume Identical($t11, exists[@21]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:20+32
    assume ($t11 == $ResourceExists($1_object_ObjectCore_$memory#21, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:20+32]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:20+32
    assume {:print "$track_exp_sub(25237):", $t11} true;

    // assume Identical($t12, Not(exists[@21]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    assume ($t12 == !$ResourceExists($1_object_ObjectCore_$memory#21, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    assume {:print "$track_exp(25238):", $t12} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:234:9+44
    assume {:print "$track_global_mem(27228):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@21]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:233:5+106
    assume {:print "$at(3,8987,9093)"} true;
    assert {:msg "assert_failed(3,8987,9093): abort not covered by any of the `aborts_if` clauses"}
      !$ResourceExists($1_object_ObjectCore_$memory#21, $inner#$1_object_Object'#0'($t0));

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:233:5+106
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::transfer<#0> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+186
procedure {:inline 1} $1_object_transfer'#0'(_$t0: $signer, _$t1: $1_object_Object'#0', _$t2: int) returns ()
{
    // declare local variables
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t0: $signer;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // assume Identical($t3, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46
    assume {:print "$at(3,6446,6492)"} true;
    assume ($t3 == $1_signer_$address_of($t0));

    // assume Identical($t4, select object::Object.inner($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:9+34
    assume {:print "$at(3,6501,6535)"} true;
    assume ($t4 == $inner#$1_object_Object'#0'($t1));

    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$at(2,16642,16643)"} true;
    assume {:print "$track_local(52,35,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$track_local(52,35,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$track_local(52,35,2):", $t2} $t2 == $t2;

    // $t5 := get_field<object::Object<#0>>.inner($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:411:29+12
    assume {:print "$at(2,16805,16817)"} true;
    $t5 := $inner#$1_object_Object'#0'($t1);

    // assume Identical($t6, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume {:print "$at(3,7098,7144)"} true;
    assume ($t6 == $1_signer_$address_of($t0));

    // object::transfer_raw($t0, $t5, $t2) on_abort goto L2 with $t7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:411:9+37
    assume {:print "$at(2,16785,16822)"} true;
    call $1_object_transfer_raw($t0, $t5, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,16785,16822)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(52,35):", $t7} $t7 == $t7;
        goto L2;
    }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:412:5+1
    assume {:print "$at(2,16827,16828)"} true;
L1:

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:412:5+1
    assume {:print "$at(2,16827,16828)"} true;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:412:5+1
L2:

    // abort($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:412:5+1
    assume {:print "$at(2,16827,16828)"} true;
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun object::transfer [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+186
procedure {:timeLimit 40} $1_object_transfer$verify(_$t0: $signer, _$t1: $1_object_Object'#0', _$t2: int) returns ()
{
    // declare local variables
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: bool;
    var $t12: bool;
    var $t13: $1_object_ObjectCore;
    var $t14: bool;
    var $t15: bool;
    var $t16: bool;
    var $t17: $1_object_ObjectCore;
    var $t18: bool;
    var $t19: bool;
    var $t20: bool;
    var $t21: $1_object_ObjectCore;
    var $t22: bool;
    var $t23: bool;
    var $t24: bool;
    var $t25: $1_object_ObjectCore;
    var $t26: bool;
    var $t0: $signer;
    var $t1: $1_object_Object'#0';
    var $t2: int;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $1_object_ObjectCore_$memory#41: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$at(2,16642,16643)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume $IsValid'address'($t2);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:48+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:48+5
    assume {:print "$at(3,6485,6490)"} true;
    assume {:print "$track_exp_sub(25987):", $t0} true;

    // assume Identical($t3, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:29+25
    assume ($t3 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:29+25]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:29+25
    assume {:print "$track_exp_sub(25988):", $t3} true;

    // assume Identical($t4, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46
    assume ($t4 == $1_signer_$address_of($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46
    assume {:print "$track_exp(25989):", $t4} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46
    assume {:print "$track_global_mem(27229):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46
    assume ($t5 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:30+6
    assume {:print "$at(3,6522,6528)"} true;
    assume {:print "$track_exp_sub(25992):", $t1} true;

    // assume Identical($t6, select object::Object.inner($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:9+34
    assume ($t6 == $inner#$1_object_Object'#0'($t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:9+34]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:9+34
    assume {:print "$track_exp(25993):", $t6} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:9+34
    assume {:print "$track_global_mem(27230):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t7, select object::Object.inner($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:9+34
    assume ($t7 == $inner#$1_object_Object'#0'($t1));

    // @41 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$at(2,16642,16643)"} true;
    $1_object_ObjectCore_$memory#41 := $1_object_ObjectCore_$memory;

    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$track_local(52,35,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$track_local(52,35,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:406:5+1
    assume {:print "$track_local(52,35,2):", $t2} $t2 == $t2;

    // $t8 := get_field<object::Object<#0>>.inner($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:411:29+12
    assume {:print "$at(2,16805,16817)"} true;
    $t8 := $inner#$1_object_Object'#0'($t1);

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:411:9+37
    assume {:print "$track_global_mem(27231):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t9, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume {:print "$at(3,7098,7144)"} true;
    assume ($t9 == $1_signer_$address_of($t0));

    // object::transfer_raw($t0, $t8, $t2) on_abort goto L2 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:411:9+37
    assume {:print "$at(2,16785,16822)"} true;
    call $1_object_transfer_raw($t0, $t8, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,16785,16822)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,35):", $t10} $t10 == $t10;
        goto L2;
    }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:412:5+1
    assume {:print "$at(2,16827,16828)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:39+14
    assume {:print "$at(3,6574,6588)"} true;
    assume {:print "$track_exp_sub(25998):", $t7} true;

    // assume Identical($t11, exists[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:20+34
    assume ($t11 == $ResourceExists($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:20+34]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:20+34
    assume {:print "$track_exp_sub(25999):", $t11} true;

    // assume Identical($t12, Not(exists[@41]<object::ObjectCore>($t7))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46
    assume ($t12 == !$ResourceExists($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46
    assume {:print "$track_exp(26000):", $t12} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46
    assume {:print "$track_global_mem(27232):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@41]<object::ObjectCore>($t7))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46
    assert {:msg "assert_failed(3,6544,6590): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#41, $t7);

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:39+14
    assume {:print "$at(3,6629,6643)"} true;
    assume {:print "$track_exp_sub(26005):", $t7} true;

    // assume Identical($t13, global[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:20+34
    assume ($t13 == $ResourceValue($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:20+34]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:20+34
    assume {:print "$track_exp_sub(26006):", $t13} true;

    // assume Identical($t14, Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69
    assume ($t14 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69
    assume {:print "$track_exp(26007):", $t14} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69
    assume {:print "$track_global_mem(27233):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69
    assert {:msg "assert_failed(3,6599,6668): function does not abort under this condition"}
      !!$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:13+13
    assume {:print "$at(3,6742,6755)"} true;
    assume {:print "$track_exp_sub(26014):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:30+14
    assume {:print "$track_exp_sub(26016):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:68+14
    assume {:print "$track_exp_sub(26018):", $t7} true;

    // assume Identical($t15, exists[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:49+34
    assume ($t15 == $ResourceExists($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:49+34]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:49+34
    assume {:print "$track_exp_sub(26019):", $t15} true;

    // assume Identical($t16, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@41]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136
    assume {:print "$at(3,6677,6813)"} true;
    assume ($t16 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#41, $t7)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136
    assume {:print "$track_exp(26020):", $t16} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136
    assume {:print "$track_global_mem(27234):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@41]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136
    assert {:msg "assert_failed(3,6677,6813): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#41, $t7))))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:13+13
    assume {:print "$at(3,6887,6900)"} true;
    assume {:print "$track_exp_sub(26027):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:30+14
    assume {:print "$track_exp_sub(26029):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:68+14
    assume {:print "$track_exp_sub(26031):", $t7} true;

    // assume Identical($t17, global[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:49+34
    assume ($t17 == $ResourceValue($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:49+34]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:49+34
    assume {:print "$track_exp_sub(26032):", $t17} true;

    // assume Identical($t18, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    assume {:print "$at(3,6822,6981)"} true;
    assume ($t18 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    assume {:print "$track_exp(26033):", $t18} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    assume {:print "$track_global_mem(27235):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    assert {:msg "assert_failed(3,6822,6981): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7)))))));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:412:5+1
    assume {:print "$at(2,16827,16828)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:39+14
    assume {:print "$at(3,6574,6588)"} true;
    assume {:print "$track_exp_sub(25998):", $t7} true;

    // assume Identical($t19, exists[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:20+34
    assume ($t19 == $ResourceExists($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:20+34]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:20+34
    assume {:print "$track_exp_sub(25999):", $t19} true;

    // assume Identical($t20, Not(exists[@41]<object::ObjectCore>($t7))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46
    assume ($t20 == !$ResourceExists($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:179:9+46
    assume {:print "$track_exp(26000):", $t20} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:39+14
    assume {:print "$at(3,6629,6643)"} true;
    assume {:print "$track_exp_sub(26005):", $t7} true;

    // assume Identical($t21, global[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:20+34
    assume ($t21 == $ResourceValue($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:20+34]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:20+34
    assume {:print "$track_exp_sub(26006):", $t21} true;

    // assume Identical($t22, Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69
    assume ($t22 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:180:9+69
    assume {:print "$track_exp(26007):", $t22} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:13+13
    assume {:print "$at(3,6742,6755)"} true;
    assume {:print "$track_exp_sub(26014):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:30+14
    assume {:print "$track_exp_sub(26016):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:68+14
    assume {:print "$track_exp_sub(26018):", $t7} true;

    // assume Identical($t23, exists[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:49+34
    assume ($t23 == $ResourceExists($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:49+34]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:182:49+34
    assume {:print "$track_exp_sub(26019):", $t23} true;

    // assume Identical($t24, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@41]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136
    assume {:print "$at(3,6677,6813)"} true;
    assume ($t24 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#41, $t7)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136]($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:181:9+136
    assume {:print "$track_exp(26020):", $t24} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:13+13
    assume {:print "$at(3,6887,6900)"} true;
    assume {:print "$track_exp_sub(26027):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:30+14
    assume {:print "$track_exp_sub(26029):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:68+14
    assume {:print "$track_exp_sub(26031):", $t7} true;

    // assume Identical($t25, global[@41]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:49+34
    assume ($t25 == $ResourceValue($1_object_ObjectCore_$memory#41, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:49+34]($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:184:49+34
    assume {:print "$track_exp_sub(26032):", $t25} true;

    // assume Identical($t26, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    assume {:print "$at(3,6822,6981)"} true;
    assume ($t26 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    assume {:print "$track_exp(26033):", $t26} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:183:9+159
    assume {:print "$track_global_mem(27236):", $1_object_ObjectCore_$memory} true;

    // assert Or(Or(Or(Not(exists[@41]<object::ObjectCore>($t7)), Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@41]<object::ObjectCore>($t7)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@41]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:172:5+652
    assume {:print "$at(3,6335,6987)"} true;
    assert {:msg "assert_failed(3,6335,6987): abort not covered by any of the `aborts_if` clauses"}
      (((!$ResourceExists($1_object_ObjectCore_$memory#41, $t7) || !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7))) || (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#41, $t7))))))) || (var $range_2 := $Range(0, (8 - 1)); (exists $i_3: int :: $InRange($range_2, $i_3) && (var i := $i_3;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#41, $t7))))))));

    // abort($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:172:5+652
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun object::address_from_constructor_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:287:5+95
procedure {:timeLimit 40} $1_object_address_from_constructor_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:287:5+1
    assume {:print "$at(2,12531,12532)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:287:5+1
    assume {:print "$track_local(52,0,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:288:9+8
    assume {:print "$at(2,12612,12620)"} true;
    $t1 := $self#$1_object_ConstructorRef($t0);

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:288:9+8
    assume {:print "$track_return(52,0,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:289:5+1
    assume {:print "$at(2,12625,12626)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:289:5+1
    assume {:print "$at(2,12625,12626)"} true;
    assert {:msg "assert_failed(2,12625,12626): function does not abort under this condition"}
      !false;

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:289:5+1
    $ret0 := $t1;
    return;

}

// fun object::address_from_delete_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:320:5+85
procedure {:timeLimit 40} $1_object_address_from_delete_ref$verify(_$t0: $1_object_DeleteRef) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_DeleteRef;
    var $temp_0'$1_object_DeleteRef': $1_object_DeleteRef;
    var $temp_0'address': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:320:5+1
    assume {:print "$at(2,13667,13668)"} true;
    assume $IsValid'$1_object_DeleteRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:320:5+1
    assume {:print "$track_local(52,1,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::DeleteRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:321:9+8
    assume {:print "$at(2,13738,13746)"} true;
    $t1 := $self#$1_object_DeleteRef($t0);

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:321:9+8
    assume {:print "$track_return(52,1,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:322:5+1
    assume {:print "$at(2,13751,13752)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:322:5+1
    assume {:print "$at(2,13751,13752)"} true;
    assert {:msg "assert_failed(2,13751,13752): function does not abort under this condition"}
      !false;

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:322:5+1
    $ret0 := $t1;
    return;

}

// fun object::address_from_extend_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:349:5+85
procedure {:timeLimit 40} $1_object_address_from_extend_ref$verify(_$t0: $1_object_ExtendRef) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_ExtendRef;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    var $temp_0'address': int;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:349:5+1
    assume {:print "$at(2,14576,14577)"} true;
    assume $IsValid'$1_object_ExtendRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:349:5+1
    assume {:print "$track_local(52,2,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ExtendRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:350:9+8
    assume {:print "$at(2,14647,14655)"} true;
    $t1 := $self#$1_object_ExtendRef($t0);

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:350:9+8
    assume {:print "$track_return(52,2,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:351:5+1
    assume {:print "$at(2,14660,14661)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:351:5+1
    assume {:print "$at(2,14660,14661)"} true;
    assert {:msg "assert_failed(2,14660,14661): function does not abort under this condition"}
      !false;

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:351:5+1
    $ret0 := $t1;
    return;

}

// fun object::address_to_object<#0> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+278
procedure {:inline 1} $1_object_address_to_object'#0'(_$t0: int) returns ($ret0: $1_object_Object'#0')
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: $1_object_Object'#0';
    var $t0: int;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+1
    assume {:print "$at(2,6628,6629)"} true;
    assume {:print "$track_local(52,3,0):", $t0} $t0 == $t0;

    // $t1 := exists<object::ObjectCore>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:17+6
    assume {:print "$at(2,6711,6717)"} true;
    $t1 := $ResourceExists($1_object_ObjectCore_$memory, $t0);

    // if ($t1) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    if ($t1) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    assume {:print "$at(2,6703,6780)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:62+22
L0:

    // $t2 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:62+22
    assume {:print "$at(2,6756,6778)"} true;
    $t2 := 2;
    assume $IsValid'u64'($t2);

    // $t3 := error::not_found($t2) on_abort goto L7 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:45+40
    call $t3 := $1_error_not_found($t2);
    if ($abort_flag) {
        assume {:print "$at(2,6739,6779)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    assume {:print "$at(2,6703,6780)"} true;
    assume {:print "$track_abort(52,3):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    $t4 := $t3;

    // goto L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    goto L7;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:30+6
    assume {:print "$at(2,6811,6817)"} true;
L2:

    // $t5 := opaque begin: object::exists_at<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume {:print "$at(2,6798,6818)"} true;

    // assume WellFormed($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume $IsValid'bool'($t5);

    // assume Eq<bool>($t5, object::spec_exists_at<#0>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume $IsEqual'bool'($t5, $1_object_spec_exists_at'#0'($t0));

    // $t5 := opaque end: object::exists_at<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20

    // if ($t5) goto L4 else goto L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    if ($t5) { goto L4; } else { goto L3; }

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
L4:

    // goto L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    assume {:print "$at(2,6790,6863)"} true;
    goto L5;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:56+24
L3:

    // $t6 := 7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:56+24
    assume {:print "$at(2,6837,6861)"} true;
    $t6 := 7;
    assume $IsValid'u64'($t6);

    // $t7 := error::not_found($t6) on_abort goto L7 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:39+42
    call $t7 := $1_error_not_found($t6);
    if ($abort_flag) {
        assume {:print "$at(2,6820,6862)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    assume {:print "$at(2,6790,6863)"} true;
    assume {:print "$track_abort(52,3):", $t7} $t7 == $t7;

    // $t4 := move($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    $t4 := $t7;

    // goto L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    goto L7;

    // label L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:28+6
    assume {:print "$at(2,6892,6898)"} true;
L5:

    // $t8 := pack object::Object<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:9+27
    assume {:print "$at(2,6873,6900)"} true;
    $t8 := $1_object_Object'#0'($t0);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:9+27
    assume {:print "$track_return(52,3,0):", $t8} $t8 == $t8;

    // label L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
L6:

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
    $ret0 := $t8;
    return;

    // label L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
L7:

    // abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::address_to_object<#1> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+278
procedure {:inline 1} $1_object_address_to_object'#1'(_$t0: int) returns ($ret0: $1_object_Object'#1')
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: $1_object_Object'#1';
    var $t0: int;
    var $temp_0'$1_object_Object'#1'': $1_object_Object'#1';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+1
    assume {:print "$at(2,6628,6629)"} true;
    assume {:print "$track_local(52,3,0):", $t0} $t0 == $t0;

    // $t1 := exists<object::ObjectCore>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:17+6
    assume {:print "$at(2,6711,6717)"} true;
    $t1 := $ResourceExists($1_object_ObjectCore_$memory, $t0);

    // if ($t1) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    if ($t1) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    assume {:print "$at(2,6703,6780)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:62+22
L0:

    // $t2 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:62+22
    assume {:print "$at(2,6756,6778)"} true;
    $t2 := 2;
    assume $IsValid'u64'($t2);

    // $t3 := error::not_found($t2) on_abort goto L7 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:45+40
    call $t3 := $1_error_not_found($t2);
    if ($abort_flag) {
        assume {:print "$at(2,6739,6779)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    assume {:print "$at(2,6703,6780)"} true;
    assume {:print "$track_abort(52,3):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    $t4 := $t3;

    // goto L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    goto L7;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:30+6
    assume {:print "$at(2,6811,6817)"} true;
L2:

    // $t5 := opaque begin: object::exists_at<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume {:print "$at(2,6798,6818)"} true;

    // assume WellFormed($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume $IsValid'bool'($t5);

    // assume Eq<bool>($t5, object::spec_exists_at<#0>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume $IsEqual'bool'($t5, $1_object_spec_exists_at'#1'($t0));

    // $t5 := opaque end: object::exists_at<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20

    // if ($t5) goto L4 else goto L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    if ($t5) { goto L4; } else { goto L3; }

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
L4:

    // goto L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    assume {:print "$at(2,6790,6863)"} true;
    goto L5;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:56+24
L3:

    // $t6 := 7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:56+24
    assume {:print "$at(2,6837,6861)"} true;
    $t6 := 7;
    assume $IsValid'u64'($t6);

    // $t7 := error::not_found($t6) on_abort goto L7 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:39+42
    call $t7 := $1_error_not_found($t6);
    if ($abort_flag) {
        assume {:print "$at(2,6820,6862)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    assume {:print "$at(2,6790,6863)"} true;
    assume {:print "$track_abort(52,3):", $t7} $t7 == $t7;

    // $t4 := move($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    $t4 := $t7;

    // goto L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    goto L7;

    // label L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:28+6
    assume {:print "$at(2,6892,6898)"} true;
L5:

    // $t8 := pack object::Object<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:9+27
    assume {:print "$at(2,6873,6900)"} true;
    $t8 := $1_object_Object'#1'($t0);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:9+27
    assume {:print "$track_return(52,3,0):", $t8} $t8 == $t8;

    // label L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
L6:

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
    $ret0 := $t8;
    return;

    // label L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
L7:

    // abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::address_to_object [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+278
procedure {:timeLimit 40} $1_object_address_to_object$verify(_$t0: int) returns ($ret0: $1_object_Object'#0')
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: bool;
    var $t6: int;
    var $t7: int;
    var $t8: $1_object_Object'#0';
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t12: bool;
    var $t13: bool;
    var $t14: bool;
    var $t15: bool;
    var $t16: bool;
    var $t0: int;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#18: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+1
    assume {:print "$at(2,6628,6629)"} true;
    assume $IsValid'address'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @18 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+1
    $1_object_ObjectCore_$memory#18 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:151:5+1
    assume {:print "$track_local(52,3,0):", $t0} $t0 == $t0;

    // $t1 := exists<object::ObjectCore>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:17+6
    assume {:print "$at(2,6711,6717)"} true;
    $t1 := $ResourceExists($1_object_ObjectCore_$memory, $t0);

    // if ($t1) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    if ($t1) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    assume {:print "$at(2,6703,6780)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:62+22
L0:

    // $t2 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:62+22
    assume {:print "$at(2,6756,6778)"} true;
    $t2 := 2;
    assume $IsValid'u64'($t2);

    // $t3 := error::not_found($t2) on_abort goto L7 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:45+40
    call $t3 := $1_error_not_found($t2);
    if ($abort_flag) {
        assume {:print "$at(2,6739,6779)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    assume {:print "$at(2,6703,6780)"} true;
    assume {:print "$track_abort(52,3):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    $t4 := $t3;

    // goto L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:152:9+77
    goto L7;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:30+6
    assume {:print "$at(2,6811,6817)"} true;
L2:

    // $t5 := opaque begin: object::exists_at<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume {:print "$at(2,6798,6818)"} true;

    // assume WellFormed($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume $IsValid'bool'($t5);

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume {:print "$track_global_mem(27237):", $1_object_ObjectCore_$memory} true;

    // assume Eq<bool>($t5, object::spec_exists_at<#0>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20
    assume $IsEqual'bool'($t5, $1_object_spec_exists_at'#0'($t0));

    // $t5 := opaque end: object::exists_at<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:17+20

    // if ($t5) goto L4 else goto L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    if ($t5) { goto L4; } else { goto L3; }

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
L4:

    // goto L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    assume {:print "$at(2,6790,6863)"} true;
    goto L5;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:56+24
L3:

    // $t6 := 7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:56+24
    assume {:print "$at(2,6837,6861)"} true;
    $t6 := 7;
    assume $IsValid'u64'($t6);

    // $t7 := error::not_found($t6) on_abort goto L7 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:39+42
    call $t7 := $1_error_not_found($t6);
    if ($abort_flag) {
        assume {:print "$at(2,6820,6862)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,3):", $t4} $t4 == $t4;
        goto L7;
    }

    // trace_abort($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    assume {:print "$at(2,6790,6863)"} true;
    assume {:print "$track_abort(52,3):", $t7} $t7 == $t7;

    // $t4 := move($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    $t4 := $t7;

    // goto L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:153:9+73
    goto L7;

    // label L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:28+6
    assume {:print "$at(2,6892,6898)"} true;
L5:

    // $t8 := pack object::Object<#0>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:9+27
    assume {:print "$at(2,6873,6900)"} true;
    $t8 := $1_object_Object'#0'($t0);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:154:9+27
    assume {:print "$track_return(52,3,0):", $t8} $t8 == $t8;

    // label L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
L6:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:39+6
    assume {:print "$at(3,403,409)"} true;
    assume {:print "$track_exp_sub(25175):", $t0} true;

    // assume Identical($t9, exists[@18]<object::ObjectCore>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:20+26
    assume ($t9 == $ResourceExists($1_object_ObjectCore_$memory#18, $t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:20+26]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:20+26
    assume {:print "$track_exp_sub(25176):", $t9} true;

    // assume Identical($t10, Not(exists[@18]<object::ObjectCore>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38
    assume ($t10 == !$ResourceExists($1_object_ObjectCore_$memory#18, $t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38
    assume {:print "$track_exp(25177):", $t10} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38
    assume {:print "$track_global_mem(27238):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@18]<object::ObjectCore>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38
    assert {:msg "assert_failed(3,373,411): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#18, $t0);

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:38+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:38+6
    assume {:print "$at(3,449,455)"} true;
    assume {:print "$track_exp_sub(25182):", $t0} true;

    // assume Identical($t11, object::spec_exists_at[]<#0>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:20+25
    assume ($t11 == $1_object_spec_exists_at'#0'($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:20+25]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:20+25
    assume {:print "$track_exp_sub(25183):", $t11} true;

    // assume Identical($t12, Not(object::spec_exists_at[]<#0>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    assume ($t12 == !$1_object_spec_exists_at'#0'($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    assume {:print "$track_exp(25184):", $t12} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    assume {:print "$track_global_mem(27239):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(object::spec_exists_at[]<#0>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    assert {:msg "assert_failed(3,420,457): function does not abort under this condition"}
      !!$1_object_spec_exists_at'#0'($t0);

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    $ret0 := $t8;
    return;

    // label L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:155:5+1
    assume {:print "$at(2,6905,6906)"} true;
L7:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:39+6
    assume {:print "$at(3,403,409)"} true;
    assume {:print "$track_exp_sub(25175):", $t0} true;

    // assume Identical($t13, exists[@18]<object::ObjectCore>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:20+26
    assume ($t13 == $ResourceExists($1_object_ObjectCore_$memory#18, $t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:20+26]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:20+26
    assume {:print "$track_exp_sub(25176):", $t13} true;

    // assume Identical($t14, Not(exists[@18]<object::ObjectCore>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38
    assume ($t14 == !$ResourceExists($1_object_ObjectCore_$memory#18, $t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:15:9+38
    assume {:print "$track_exp(25177):", $t14} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:38+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:38+6
    assume {:print "$at(3,449,455)"} true;
    assume {:print "$track_exp_sub(25182):", $t0} true;

    // assume Identical($t15, object::spec_exists_at[]<#0>($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:20+25
    assume ($t15 == $1_object_spec_exists_at'#0'($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:20+25]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:20+25
    assume {:print "$track_exp_sub(25183):", $t15} true;

    // assume Identical($t16, Not(object::spec_exists_at[]<#0>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    assume ($t16 == !$1_object_spec_exists_at'#0'($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    assume {:print "$track_exp(25184):", $t16} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:16:9+37
    assume {:print "$track_global_mem(27240):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@18]<object::ObjectCore>($t0)), Not(object::spec_exists_at[]<#0>($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:14:5+159
    assume {:print "$at(3,304,463)"} true;
    assert {:msg "assert_failed(3,304,463): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#18, $t0) || !$1_object_spec_exists_at'#0'($t0));

    // abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:14:5+159
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::can_generate_delete_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:297:5+93
procedure {:timeLimit 40} $1_object_can_generate_delete_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: bool)
{
    // declare local variables
    var $t1: bool;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:297:5+1
    assume {:print "$at(2,12904,12905)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:297:5+1
    assume {:print "$track_local(52,4,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.can_delete($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:298:9+14
    assume {:print "$at(2,12977,12991)"} true;
    $t1 := $can_delete#$1_object_ConstructorRef($t0);

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:298:9+14
    assume {:print "$track_return(52,4,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:299:5+1
    assume {:print "$at(2,12996,12997)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:299:5+1
    assume {:print "$at(2,12996,12997)"} true;
    assert {:msg "assert_failed(2,12996,12997): function does not abort under this condition"}
      !false;

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:299:5+1
    $ret0 := $t1;
    return;

}

// fun object::convert [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:194:5+115
procedure {:timeLimit 40} $1_object_convert$verify(_$t0: $1_object_Object'#0') returns ($ret0: $1_object_Object'#1')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'#1';
    var $t3: int;
    var $t4: bool;
    var $t5: bool;
    var $t6: bool;
    var $t7: bool;
    var $t8: bool;
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'#1'': $1_object_Object'#1';
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#31: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:194:5+1
    assume {:print "$at(2,8534,8535)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:194:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @31 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:194:5+1
    $1_object_ObjectCore_$memory#31 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:194:5+1
    assume {:print "$track_local(52,5,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:195:30+12
    assume {:print "$at(2,8630,8642)"} true;
    $t1 := $inner#$1_object_Object'#0'($t0);

    // $t2 := object::address_to_object<#1>($t1) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:195:9+34
    call $t2 := $1_object_address_to_object'#1'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,8609,8643)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,5):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:195:9+34
    assume {:print "$track_return(52,5,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:196:5+1
    assume {:print "$at(2,8648,8649)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:39+6
    assume {:print "$at(3,1518,1524)"} true;
    assume {:print "$track_exp_sub(25597):", $t0} true;

    // assume Identical($t4, exists[@31]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:20+32
    assume ($t4 == $ResourceExists($1_object_ObjectCore_$memory#31, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:20+32]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:20+32
    assume {:print "$track_exp_sub(25598):", $t4} true;

    // assume Identical($t5, Not(exists[@31]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44
    assume ($t5 == !$ResourceExists($1_object_ObjectCore_$memory#31, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44
    assume {:print "$track_exp(25599):", $t5} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44
    assume {:print "$track_global_mem(27241):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@31]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44
    assert {:msg "assert_failed(3,1488,1532): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#31, $inner#$1_object_Object'#0'($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:38+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:38+6
    assume {:print "$at(3,1570,1576)"} true;
    assume {:print "$track_exp_sub(25604):", $t0} true;

    // assume Identical($t6, object::spec_exists_at[]<#1>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:20+31
    assume ($t6 == $1_object_spec_exists_at'#1'($inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:20+31]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:20+31
    assume {:print "$track_exp_sub(25605):", $t6} true;

    // assume Identical($t7, Not(object::spec_exists_at[]<#1>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    assume ($t7 == !$1_object_spec_exists_at'#1'($inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    assume {:print "$track_exp(25606):", $t7} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    assume {:print "$track_global_mem(27242):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(object::spec_exists_at[]<#1>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    assert {:msg "assert_failed(3,1541,1584): function does not abort under this condition"}
      !!$1_object_spec_exists_at'#1'($inner#$1_object_Object'#0'($t0));

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    $ret0 := $t2;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:196:5+1
    assume {:print "$at(2,8648,8649)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:39+6
    assume {:print "$at(3,1518,1524)"} true;
    assume {:print "$track_exp_sub(25597):", $t0} true;

    // assume Identical($t8, exists[@31]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:20+32
    assume ($t8 == $ResourceExists($1_object_ObjectCore_$memory#31, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:20+32]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:20+32
    assume {:print "$track_exp_sub(25598):", $t8} true;

    // assume Identical($t9, Not(exists[@31]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44
    assume ($t9 == !$ResourceExists($1_object_ObjectCore_$memory#31, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:45:9+44
    assume {:print "$track_exp(25599):", $t9} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:38+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:38+6
    assume {:print "$at(3,1570,1576)"} true;
    assume {:print "$track_exp_sub(25604):", $t0} true;

    // assume Identical($t10, object::spec_exists_at[]<#1>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:20+31
    assume ($t10 == $1_object_spec_exists_at'#1'($inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:20+31]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:20+31
    assume {:print "$track_exp_sub(25605):", $t10} true;

    // assume Identical($t11, Not(object::spec_exists_at[]<#1>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    assume ($t11 == !$1_object_spec_exists_at'#1'($inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    assume {:print "$track_exp(25606):", $t11} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:46:9+43
    assume {:print "$track_global_mem(27243):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@31]<object::ObjectCore>(select object::Object.inner($t0))), Not(object::spec_exists_at[]<#1>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:44:5+171
    assume {:print "$at(3,1419,1590)"} true;
    assert {:msg "assert_failed(3,1419,1590): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#31, $inner#$1_object_Object'#0'($t0)) || !$1_object_spec_exists_at'#1'($inner#$1_object_Object'#0'($t0)));

    // abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:44:5+171
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::create_guid_object_address [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:179:5+310
procedure {:timeLimit 40} $1_object_create_guid_object_address$verify(_$t0: int, _$t1: int) returns ($ret0: int)
{
    // declare local variables
    var $t2: Vec (int);
    var $t3: $1_guid_ID;
    var $t4: $1_guid_ID;
    var $t5: int;
    var $t6: $Mutation (Vec (int));
    var $t7: int;
    var $t8: Vec (int);
    var $t9: Vec (int);
    var $t10: int;
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_guid_ID': $1_guid_ID;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:179:5+1
    assume {:print "$at(2,7972,7973)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:179:5+1
    assume $IsValid'u64'($t1);

    // trace_local[source]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:179:5+1
    assume {:print "$track_local(52,7,0):", $t0} $t0 == $t0;

    // trace_local[creation_num]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:179:5+1
    assume {:print "$track_local(52,7,1):", $t1} $t1 == $t1;

    // $t4 := guid::create_id($t0, $t1) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:180:18+37
    assume {:print "$at(2,8074,8111)"} true;
    call $t4 := $1_guid_create_id($t0, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,8074,8111)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_local[id]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:180:13+2
    assume {:print "$track_local(52,7,3):", $t4} $t4 == $t4;

    // $t2 := bcs::to_bytes<guid::ID>($t4) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:181:21+18
    assume {:print "$at(2,8133,8151)"} true;
    call $t2 := $1_bcs_to_bytes'$1_guid_ID'($t4);
    if ($abort_flag) {
        assume {:print "$at(2,8133,8151)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:181:13+5
    assume {:print "$track_local(52,7,2):", $t2} $t2 == $t2;

    // $t6 := borrow_local($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:182:27+10
    assume {:print "$at(2,8179,8189)"} true;
    $t6 := $Mutation($Local(2), EmptyVec(), $t2);

    // $t7 := 253 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:182:39+31
    $t7 := 253;
    assume $IsValid'u8'($t7);

    // vector::push_back<u8>($t6, $t7) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:182:9+62
    call $t6 := $1_vector_push_back'u8'($t6, $t7);
    if ($abort_flag) {
        assume {:print "$at(2,8161,8223)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:182:9+62
    $t2 := $Dereference($t6);

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:182:9+62
    assume {:print "$track_local(52,7,2):", $t2} $t2 == $t2;

    // $t8 := move($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:183:45+5
    assume {:print "$at(2,8269,8274)"} true;
    $t8 := $t2;

    // $t9 := hash::sha3_256($t8) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:183:30+21
    call $t9 := $1_hash_sha3_256($t8);
    if ($abort_flag) {
        assume {:print "$at(2,8254,8275)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // $t10 := from_bcs::to_address($t9) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:183:9+43
    call $t10 := $1_from_bcs_to_address($t9);
    if ($abort_flag) {
        assume {:print "$at(2,8233,8276)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,7):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_return[0]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:183:9+43
    assume {:print "$track_return(52,7,0):", $t10} $t10 == $t10;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:184:5+1
    assume {:print "$at(2,8281,8282)"} true;
L1:

    // return $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:184:5+1
    assume {:print "$at(2,8281,8282)"} true;
    $ret0 := $t10;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:184:5+1
L2:

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:184:5+1
    assume {:print "$at(2,8281,8282)"} true;
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::create_named_object [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:200:5+284
procedure {:timeLimit 40} $1_object_create_named_object$verify(_$t0: $signer, _$t1: Vec (int)) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: int;
    var $t13: bool;
    var $t14: $1_object_ConstructorRef;
    var $t15: bool;
    var $t16: bool;
    var $t17: bool;
    var $t18: bool;
    var $t0: $signer;
    var $t1: Vec (int);
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $temp_0'vec'u8'': Vec (int);
    var $1_object_ObjectCore_$memory#46: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:200:5+1
    assume {:print "$at(2,8855,8856)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:200:5+1
    assume $IsValid'vec'u8''($t1);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:200:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:50+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:50+7
    assume {:print "$at(3,1724,1731)"} true;
    assume {:print "$track_exp_sub(26395):", $t0} true;

    // assume Identical($t4, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:31+27
    assume ($t4 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:31+27]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:31+27
    assume {:print "$track_exp_sub(26396):", $t4} true;

    // assume Identical($t5, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:9+50
    assume ($t5 == $1_signer_$address_of($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:9+50]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:9+50
    assume {:print "$track_exp(26397):", $t5} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:9+50
    assume {:print "$track_global_mem(27244):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t6, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:50:9+50
    assume ($t6 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:51+15]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:51+15
    assume {:print "$at(3,1784,1799)"} true;
    assume {:print "$track_exp_sub(26403):", $t6} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:68+4]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:68+4
    assume {:print "$track_exp_sub(26404):", $t1} true;

    // assume Identical($t7, object::spec_create_object_address($t6, $t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:24+49
    assume ($t7 == $1_object_spec_create_object_address($t6, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:24+49]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:24+49
    assume {:print "$track_exp_sub(26405):", $t7} true;

    // assume Identical($t8, object::spec_create_object_address($t6, $t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:9+65
    assume ($t8 == $1_object_spec_create_object_address($t6, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:9+65]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:9+65
    assume {:print "$track_exp(26406):", $t8} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:9+65
    assume {:print "$track_global_mem(27245):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t9, object::spec_create_object_address($t6, $t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:51:9+65
    assume ($t9 == $1_object_spec_create_object_address($t6, $t1));

    // @46 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:200:5+1
    assume {:print "$at(2,8855,8856)"} true;
    $1_object_ObjectCore_$memory#46 := $1_object_ObjectCore_$memory;

    // trace_local[creator]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:200:5+1
    assume {:print "$track_local(52,8,0):", $t0} $t0 == $t0;

    // trace_local[seed]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:200:5+1
    assume {:print "$track_local(52,8,1):", $t1} $t1 == $t1;

    // $t10 := signer::address_of($t0) on_abort goto L2 with $t11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:201:31+27
    assume {:print "$at(2,8970,8997)"} true;
    call $t10 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,8970,8997)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(52,8):", $t11} $t11 == $t11;
        goto L2;
    }

    // trace_local[creator_address]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:201:13+15
    assume {:print "$track_local(52,8,2):", $t10} $t10 == $t10;

    // $t12 := opaque begin: object::create_object_address($t10, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:202:24+45
    assume {:print "$at(2,9022,9067)"} true;

    // assume WellFormed($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:202:24+45
    assume $IsValid'address'($t12);

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:202:24+45
    assume {:print "$track_global_mem(27246):", $1_object_ObjectCore_$memory} true;

    // assume Eq<address>($t12, object::spec_create_object_address($t10, $t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:202:24+45
    assume $IsEqual'address'($t12, $1_object_spec_create_object_address($t10, $t1));

    // $t12 := opaque end: object::create_object_address($t10, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:202:24+45

    // trace_local[obj_addr]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:202:13+8
    assume {:print "$track_local(52,8,3):", $t12} $t12 == $t12;

    // $t13 := false at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:203:59+5
    assume {:print "$at(2,9127,9132)"} true;
    $t13 := false;
    assume $IsValid'bool'($t13);

    // $t14 := object::create_object_internal($t10, $t12, $t13) on_abort goto L2 with $t11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:203:9+56
    call $t14 := $1_object_create_object_internal($t10, $t12, $t13);
    if ($abort_flag) {
        assume {:print "$at(2,9077,9133)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(52,8):", $t11} $t11 == $t11;
        goto L2;
    }

    // trace_return[0]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:203:9+56
    assume {:print "$track_return(52,8,0):", $t14} $t14 == $t14;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:204:5+1
    assume {:print "$at(2,9138,9139)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:38+8]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:38+8
    assume {:print "$at(3,1845,1853)"} true;
    assume {:print "$track_exp_sub(26411):", $t9} true;

    // assume Identical($t15, exists[@46]<object::ObjectCore>($t9)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:19+28
    assume ($t15 == $ResourceExists($1_object_ObjectCore_$memory#46, $t9));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:19+28]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:19+28
    assume {:print "$track_exp_sub(26412):", $t15} true;

    // assume Identical($t16, exists[@46]<object::ObjectCore>($t9)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    assume ($t16 == $ResourceExists($1_object_ObjectCore_$memory#46, $t9));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    assume {:print "$track_exp(26413):", $t16} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    assume {:print "$track_global_mem(27247):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@46]<object::ObjectCore>($t9)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    assert {:msg "assert_failed(3,1816,1855): function does not abort under this condition"}
      !$ResourceExists($1_object_ObjectCore_$memory#46, $t9);

    // return $t14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    $ret0 := $t14;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:204:5+1
    assume {:print "$at(2,9138,9139)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:38+8]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:38+8
    assume {:print "$at(3,1845,1853)"} true;
    assume {:print "$track_exp_sub(26411):", $t9} true;

    // assume Identical($t17, exists[@46]<object::ObjectCore>($t9)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:19+28
    assume ($t17 == $ResourceExists($1_object_ObjectCore_$memory#46, $t9));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:19+28]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:19+28
    assume {:print "$track_exp_sub(26412):", $t17} true;

    // assume Identical($t18, exists[@46]<object::ObjectCore>($t9)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    assume ($t18 == $ResourceExists($1_object_ObjectCore_$memory#46, $t9));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    assume {:print "$track_exp(26413):", $t18} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:52:9+39
    assume {:print "$track_global_mem(27248):", $1_object_ObjectCore_$memory} true;

    // assert exists[@46]<object::ObjectCore>($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:49:5+265
    assume {:print "$at(3,1596,1861)"} true;
    assert {:msg "assert_failed(3,1596,1861): abort not covered by any of the `aborts_if` clauses"}
      $ResourceExists($1_object_ObjectCore_$memory#46, $t9);

    // abort($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:49:5+265
    $abort_code := $t11;
    $abort_flag := true;
    return;

}

// fun object::create_object_address [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:163:5+294
procedure {:timeLimit 40} $1_object_create_object_address$verify(_$t0: int, _$t1: Vec (int)) returns ($ret0: int)
{
    // declare local variables
    var $t2: Vec (int);
    var $t3: int;
    var $t4: $Mutation (Vec (int));
    var $t5: $Mutation (Vec (int));
    var $t6: int;
    var $t7: Vec (int);
    var $t8: Vec (int);
    var $t9: int;
    var $t0: int;
    var $t1: Vec (int);
    var $temp_0'address': int;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:163:5+1
    assume {:print "$at(2,7180,7181)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:163:5+1
    assume $IsValid'vec'u8''($t1);

    // trace_local[source]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:163:5+1
    assume {:print "$track_local(52,9,0):", $t0} $t0 == $t0;

    // trace_local[seed]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:163:5+1
    assume {:print "$track_local(52,9,1):", $t1} $t1 == $t1;

    // $t2 := bcs::to_bytes<address>($t0) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:164:21+21
    assume {:print "$at(2,7280,7301)"} true;
    call $t2 := $1_bcs_to_bytes'address'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,7280,7301)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,9):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:164:13+5
    assume {:print "$track_local(52,9,2):", $t2} $t2 == $t2;

    // $t4 := borrow_local($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:165:24+10
    assume {:print "$at(2,7326,7336)"} true;
    $t4 := $Mutation($Local(2), EmptyVec(), $t2);

    // vector::append<u8>($t4, $t1) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:165:9+32
    call $t4 := $1_vector_append'u8'($t4, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,7311,7343)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,9):", $t3} $t3 == $t3;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:165:9+32
    $t2 := $Dereference($t4);

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:165:9+32
    assume {:print "$track_local(52,9,2):", $t2} $t2 == $t2;

    // $t5 := borrow_local($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:166:27+10
    assume {:print "$at(2,7371,7381)"} true;
    $t5 := $Mutation($Local(2), EmptyVec(), $t2);

    // $t6 := 254 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:166:39+31
    $t6 := 254;
    assume $IsValid'u8'($t6);

    // vector::push_back<u8>($t5, $t6) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:166:9+62
    call $t5 := $1_vector_push_back'u8'($t5, $t6);
    if ($abort_flag) {
        assume {:print "$at(2,7353,7415)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,9):", $t3} $t3 == $t3;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:166:9+62
    $t2 := $Dereference($t5);

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:166:9+62
    assume {:print "$track_local(52,9,2):", $t2} $t2 == $t2;

    // $t7 := move($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:167:45+5
    assume {:print "$at(2,7461,7466)"} true;
    $t7 := $t2;

    // $t8 := hash::sha3_256($t7) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:167:30+21
    call $t8 := $1_hash_sha3_256($t7);
    if ($abort_flag) {
        assume {:print "$at(2,7446,7467)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,9):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t9 := from_bcs::to_address($t8) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:167:9+43
    call $t9 := $1_from_bcs_to_address($t8);
    if ($abort_flag) {
        assume {:print "$at(2,7425,7468)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,9):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:167:9+43
    assume {:print "$track_return(52,9,0):", $t9} $t9 == $t9;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:168:5+1
    assume {:print "$at(2,7473,7474)"} true;
L1:

    // return $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:168:5+1
    assume {:print "$at(2,7473,7474)"} true;
    $ret0 := $t9;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:168:5+1
L2:

    // abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:168:5+1
    assume {:print "$at(2,7473,7474)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::create_object_from_account [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:214:5+196
procedure {:timeLimit 40} $1_object_create_object_from_account$verify(_$t0: $signer) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t1: $1_guid_GUID;
    var $t2: int;
    var $t3: $1_account_Account;
    var $t4: $1_account_Account;
    var $t5: $1_account_Account;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: $1_guid_GUID;
    var $t12: $1_guid_GUID;
    var $t13: int;
    var $t14: int;
    var $t15: $1_account_Account;
    var $t16: $1_guid_GUID;
    var $t17: int;
    var $t18: int;
    var $t19: Vec (int);
    var $t20: Vec (int);
    var $t21: Vec (int);
    var $t22: int;
    var $t23: $1_object_ConstructorRef;
    var $t24: int;
    var $t25: bool;
    var $t26: bool;
    var $t27: bool;
    var $t28: bool;
    var $t29: int;
    var $t30: bool;
    var $t31: bool;
    var $t32: bool;
    var $t33: bool;
    var $t0: $signer;
    var $temp_0'$1_account_Account': $1_account_Account;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $1_account_Account_$memory#45: $Memory $1_account_Account;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:214:5+1
    assume {:print "$at(2,9673,9674)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume forall $rsc: ResourceDomain<account::Account>(): And(WellFormed($rsc), And(Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.rotation_capability_offer($rsc)))), 1), Le(Len<address>(select option::Option.vec(select account::CapabilityOffer.for(select account::Account.signer_capability_offer($rsc)))), 1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:214:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_account_Account_$memory, $a_0)}(var $rsc := $ResourceValue($1_account_Account_$memory, $a_0);
    (($IsValid'$1_account_Account'($rsc) && ((LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_RotationCapability'($rotation_capability_offer#$1_account_Account($rsc)))) <= 1) && (LenVec($vec#$1_option_Option'address'($for#$1_account_CapabilityOffer'$1_account_SignerCapability'($signer_capability_offer#$1_account_Account($rsc)))) <= 1))))));

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:214:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:71+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:71+7
    assume {:print "$at(3,2362,2369)"} true;
    assume {:print "$track_exp_sub(26279):", $t0} true;

    // assume Identical($t2, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:52+27
    assume ($t2 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:52+27]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:52+27
    assume {:print "$track_exp_sub(26280):", $t2} true;

    // assume Identical($t3, global<account::Account>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:27+53
    assume ($t3 == $ResourceValue($1_account_Account_$memory, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:27+53]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:27+53
    assume {:print "$track_exp_sub(26281):", $t3} true;

    // assume Identical($t4, global<account::Account>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:9+72
    assume ($t4 == $ResourceValue($1_account_Account_$memory, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:9+72]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:9+72
    assume {:print "$track_exp(26282):", $t4} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:9+72
    assume {:print "$track_global_mem(27249):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:9+72
    assume {:print "$track_global_mem(27250):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, global<account::Account>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:63:9+72
    assume ($t5 == $ResourceValue($1_account_Account_$memory, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:28+11]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:28+11
    assume {:print "$at(3,2550,2561)"} true;
    assume {:print "$track_exp_sub(26286):", $t5} true;

    // assume Identical($t6, select account::Account.guid_creation_num($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:9+49
    assume ($t6 == $guid_creation_num#$1_account_Account($t5));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:9+49]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:9+49
    assume {:print "$track_exp(26287):", $t6} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:9+49
    assume {:print "$track_global_mem(27251):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:9+49
    assume {:print "$track_global_mem(27252):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t7, select account::Account.guid_creation_num($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:66:9+49
    assume ($t7 == $guid_creation_num#$1_account_Account($t5));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:39+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:39+7
    assume {:print "$at(3,2619,2626)"} true;
    assume {:print "$track_exp_sub(26291):", $t0} true;

    // assume Identical($t8, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:20+27
    assume ($t8 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:20+27]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:20+27
    assume {:print "$track_exp_sub(26292):", $t8} true;

    // assume Identical($t9, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:9+39
    assume ($t9 == $1_signer_$address_of($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:9+39]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:9+39
    assume {:print "$track_exp(26293):", $t9} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:9+39
    assume {:print "$track_global_mem(27253):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:9+39
    assume {:print "$track_global_mem(27254):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t10, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:67:9+39
    assume ($t10 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:70:17+12]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:70:17+12
    assume {:print "$at(3,2701,2713)"} true;
    assume {:print "$track_exp_sub(26298):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:71:17+4]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:71:17+4
    assume {:print "$at(3,2731,2735)"} true;
    assume {:print "$track_exp_sub(26300):", $t10} true;

    // assume Identical($t11, pack guid::GUID(pack guid::ID($t7, $t10))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:68:9+124
    assume {:print "$at(3,2637,2761)"} true;
    assume ($t11 == $1_guid_GUID($1_guid_ID($t7, $t10)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:68:9+124]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:68:9+124
    assume {:print "$track_exp(26301):", $t11} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:68:9+124
    assume {:print "$track_global_mem(27255):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:68:9+124
    assume {:print "$track_global_mem(27256):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t12, pack guid::GUID(pack guid::ID($t7, $t10))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:68:9+124
    assume ($t12 == $1_guid_GUID($1_guid_ID($t7, $t10)));

    // @45 := save_mem(account::Account) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:214:5+1
    assume {:print "$at(2,9673,9674)"} true;
    $1_account_Account_$memory#45 := $1_account_Account_$memory;

    // trace_local[creator]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:214:5+1
    assume {:print "$track_local(52,10,0):", $t0} $t0 == $t0;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:20+29
    assume {:print "$at(2,9766,9795)"} true;
    assume {:print "$track_global_mem(27257):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:20+29
    assume {:print "$track_global_mem(27258):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t13, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:416:9+46
    assume {:print "$at(73,20442,20488)"} true;
    assume ($t13 == $1_signer_$address_of($t0));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:416:9+46
    assume {:print "$track_global_mem(27259):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:416:9+46
    assume {:print "$track_global_mem(27260):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t14, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:430:9+39
    assume {:print "$at(73,20934,20973)"} true;
    assume ($t14 == $1_signer_$address_of($t0));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:430:9+39
    assume {:print "$track_global_mem(27261):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:430:9+39
    assume {:print "$track_global_mem(27262):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t15, global<account::Account>($t14)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/account.spec.move:431:9+36
    assume {:print "$at(73,20982,21018)"} true;
    assume ($t15 == $ResourceValue($1_account_Account_$memory, $t14));

    // $t16 := account::create_guid($t0) on_abort goto L2 with $t17 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:20+29
    assume {:print "$at(2,9766,9795)"} true;
    call $t16 := $1_account_create_guid($t0);
    if ($abort_flag) {
        assume {:print "$at(2,9766,9795)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(52,10):", $t17} $t17 == $t17;
        goto L2;
    }

    // trace_local[guid]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:215:13+4
    assume {:print "$track_local(52,10,1):", $t16} $t16 == $t16;

    // $t18 := signer::address_of($t0) on_abort goto L2 with $t17 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:33+27
    assume {:print "$at(2,9829,9856)"} true;
    call $t18 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,9829,9856)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(52,10):", $t17} $t17 == $t17;
        goto L2;
    }

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+58
    assume {:print "$track_global_mem(27263):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+58
    assume {:print "$track_global_mem(27264):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t19, bcs::$to_bytes<guid::GUID>($t16)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$at(3,3536,3573)"} true;
    assume ($t19 == $1_bcs_$to_bytes'$1_guid_GUID'($t16));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$track_global_mem(27265):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$track_global_mem(27266):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t20, ConcatVec<u8>($t19, SingleVec<u8>(253))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$at(3,3582,3654)"} true;
    assume ($t20 == ConcatVec($t19, MakeVec1(253)));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$track_global_mem(27267):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$track_global_mem(27268):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t21, hash::$sha3_256($t20)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$at(3,3663,3702)"} true;
    assume ($t21 == $1_hash_$sha3_256($t20));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$track_global_mem(27269):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$track_global_mem(27270):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t22, from_bcs::deserialize<address>($t21)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58
    assume {:print "$at(3,3711,3769)"} true;
    assume ($t22 == $1_from_bcs_deserialize'address'($t21));

    // $t23 := object::create_object_from_guid($t18, $t16) on_abort goto L2 with $t17 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+58
    assume {:print "$at(2,9805,9863)"} true;
    call $t23 := $1_object_create_object_from_guid($t18, $t16);
    if ($abort_flag) {
        assume {:print "$at(2,9805,9863)"} true;
        $t17 := $abort_code;
        assume {:print "$track_abort(52,10):", $t17} $t17 == $t17;
        goto L2;
    }

    // trace_return[0]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:216:9+58
    assume {:print "$track_return(52,10,0):", $t23} $t23 == $t23;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(2,9868,9869)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:64+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:64+7
    assume {:print "$at(3,2255,2262)"} true;
    assume {:print "$track_exp_sub(26307):", $t0} true;

    // assume Identical($t24, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:45+27
    assume ($t24 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:45+27]($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:45+27
    assume {:print "$track_exp_sub(26308):", $t24} true;

    // assume Identical($t25, exists[@45]<account::Account>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:20+53
    assume ($t25 == $ResourceExists($1_account_Account_$memory#45, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:20+53]($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:20+53
    assume {:print "$track_exp_sub(26309):", $t25} true;

    // assume Identical($t26, Not(exists[@45]<account::Account>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65
    assume ($t26 == !$ResourceExists($1_account_Account_$memory#45, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65
    assume {:print "$track_exp(26310):", $t26} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65
    assume {:print "$track_global_mem(27271):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65
    assume {:print "$track_global_mem(27272):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@45]<account::Account>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65
    assert {:msg "assert_failed(3,2200,2265): function does not abort under this condition"}
      !!$ResourceExists($1_account_Account_$memory#45, $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:19+11]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:19+11
    assume {:print "$at(3,2391,2402)"} true;
    assume {:print "$track_exp_sub(26314):", $t5} true;

    // assume Identical($t27, Gt(Add(select account::Account.guid_creation_num($t5), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54
    assume ($t27 == (($guid_creation_num#$1_account_Account($t5) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54]($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54
    assume {:print "$track_exp(26315):", $t27} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54
    assume {:print "$track_global_mem(27273):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54
    assume {:print "$track_global_mem(27274):", $1_object_ObjectCore_$memory} true;

    // assert Not(Gt(Add(select account::Account.guid_creation_num($t5), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54
    assert {:msg "assert_failed(3,2381,2435): function does not abort under this condition"}
      !(($guid_creation_num#$1_account_Account($t5) + 1) > 18446744073709551615);

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:19+11]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:19+11
    assume {:print "$at(3,2454,2465)"} true;
    assume {:print "$track_exp_sub(26319):", $t5} true;

    // assume Identical($t28, Ge(Add(select account::Account.guid_creation_num($t5), 1), 1125899906842624)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume ($t28 == (($guid_creation_num#$1_account_Account($t5) + 1) >= 1125899906842624));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78]($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume {:print "$track_exp(26320):", $t28} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume {:print "$track_global_mem(27275):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume {:print "$track_global_mem(27276):", $1_object_ObjectCore_$memory} true;

    // assert Not(Ge(Add(select account::Account.guid_creation_num($t5), 1), 1125899906842624)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assert {:msg "assert_failed(3,2444,2522): function does not abort under this condition"}
      !(($guid_creation_num#$1_account_Account($t5) + 1) >= 1125899906842624);

    // return $t23 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    $ret0 := $t23;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:217:5+1
    assume {:print "$at(2,9868,9869)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:64+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:64+7
    assume {:print "$at(3,2255,2262)"} true;
    assume {:print "$track_exp_sub(26307):", $t0} true;

    // assume Identical($t29, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:45+27
    assume ($t29 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:45+27]($t29) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:45+27
    assume {:print "$track_exp_sub(26308):", $t29} true;

    // assume Identical($t30, exists[@45]<account::Account>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:20+53
    assume ($t30 == $ResourceExists($1_account_Account_$memory#45, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:20+53]($t30) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:20+53
    assume {:print "$track_exp_sub(26309):", $t30} true;

    // assume Identical($t31, Not(exists[@45]<account::Account>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65
    assume ($t31 == !$ResourceExists($1_account_Account_$memory#45, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65]($t31) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:61:9+65
    assume {:print "$track_exp(26310):", $t31} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:19+11]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:19+11
    assume {:print "$at(3,2391,2402)"} true;
    assume {:print "$track_exp_sub(26314):", $t5} true;

    // assume Identical($t32, Gt(Add(select account::Account.guid_creation_num($t5), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54
    assume ($t32 == (($guid_creation_num#$1_account_Account($t5) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54]($t32) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:64:9+54
    assume {:print "$track_exp(26315):", $t32} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:19+11]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:19+11
    assume {:print "$at(3,2454,2465)"} true;
    assume {:print "$track_exp_sub(26319):", $t5} true;

    // assume Identical($t33, Ge(Add(select account::Account.guid_creation_num($t5), 1), 1125899906842624)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume ($t33 == (($guid_creation_num#$1_account_Account($t5) + 1) >= 1125899906842624));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78]($t33) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume {:print "$track_exp(26320):", $t33} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume {:print "$track_global_mem(27277):", $1_account_Account_$memory} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:65:9+78
    assume {:print "$track_global_mem(27278):", $1_object_ObjectCore_$memory} true;

    // assert Or(Or(Not(exists[@45]<account::Account>(signer::$address_of[]($t0))), Gt(Add(select account::Account.guid_creation_num($t5), 1), 18446744073709551615)), Ge(Add(select account::Account.guid_creation_num($t5), 1), 1125899906842624)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:60:5+643
    assume {:print "$at(3,2124,2767)"} true;
    assert {:msg "assert_failed(3,2124,2767): abort not covered by any of the `aborts_if` clauses"}
      ((!$ResourceExists($1_account_Account_$memory#45, $1_signer_$address_of($t0)) || (($guid_creation_num#$1_account_Account($t5) + 1) > 18446744073709551615)) || (($guid_creation_num#$1_account_Account($t5) + 1) >= 1125899906842624));

    // abort($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:60:5+643
    $abort_code := $t17;
    $abort_flag := true;
    return;

}

// fun object::create_object_from_guid [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+444
procedure {:inline 1} $1_object_create_object_from_guid(_$t0: int, _$t1: $1_guid_GUID) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t2: Vec (int);
    var $t3: int;
    var $t4: Vec (int);
    var $t5: Vec (int);
    var $t6: Vec (int);
    var $t7: int;
    var $t8: int;
    var $t9: $Mutation (Vec (int));
    var $t10: int;
    var $t11: Vec (int);
    var $t12: Vec (int);
    var $t13: int;
    var $t14: bool;
    var $t15: $1_object_ConstructorRef;
    var $t0: int;
    var $t1: $1_guid_GUID;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t4, bcs::$to_bytes<guid::GUID>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$at(3,3536,3573)"} true;
    assume ($t4 == $1_bcs_$to_bytes'$1_guid_GUID'($t1));

    // assume Identical($t5, ConcatVec<u8>($t4, SingleVec<u8>(253))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$at(3,3582,3654)"} true;
    assume ($t5 == ConcatVec($t4, MakeVec1(253)));

    // assume Identical($t6, hash::$sha3_256($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$at(3,3663,3702)"} true;
    assume ($t6 == $1_hash_$sha3_256($t5));

    // assume Identical($t7, from_bcs::deserialize<address>($t6)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58
    assume {:print "$at(3,3711,3769)"} true;
    assume ($t7 == $1_from_bcs_deserialize'address'($t6));

    // trace_local[creator_address]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume {:print "$at(2,10151,10152)"} true;
    assume {:print "$track_local(52,11,0):", $t0} $t0 == $t0;

    // trace_local[guid]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume {:print "$track_local(52,11,1):", $t1} $t1 == $t1;

    // $t2 := bcs::to_bytes<guid::GUID>($t1) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:226:21+20
    assume {:print "$at(2,10261,10281)"} true;
    call $t2 := $1_bcs_to_bytes'$1_guid_GUID'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,10261,10281)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,11):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:226:13+5
    assume {:print "$track_local(52,11,2):", $t2} $t2 == $t2;

    // $t9 := borrow_local($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:27+10
    assume {:print "$at(2,10309,10319)"} true;
    $t9 := $Mutation($Local(2), EmptyVec(), $t2);

    // $t10 := 253 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:39+31
    $t10 := 253;
    assume $IsValid'u8'($t10);

    // vector::push_back<u8>($t9, $t10) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:9+62
    call $t9 := $1_vector_push_back'u8'($t9, $t10);
    if ($abort_flag) {
        assume {:print "$at(2,10291,10353)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,11):", $t8} $t8 == $t8;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:9+62
    $t2 := $Dereference($t9);

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:9+62
    assume {:print "$track_local(52,11,2):", $t2} $t2 == $t2;

    // assume from_bcs::deserializable<address>(hash::$sha3_256($t2)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:13+64
    assume {:print "$at(2,10382,10446)"} true;
    assume $1_from_bcs_deserializable'address'($1_hash_$sha3_256($t2));

    // $t11 := move($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:60+5
    assume {:print "$at(2,10517,10522)"} true;
    $t11 := $t2;

    // $t12 := hash::sha3_256($t11) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:45+21
    call $t12 := $1_hash_sha3_256($t11);
    if ($abort_flag) {
        assume {:print "$at(2,10502,10523)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,11):", $t8} $t8 == $t8;
        goto L2;
    }

    // $t13 := from_bcs::to_address($t12) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:24+43
    call $t13 := $1_from_bcs_to_address($t12);
    if ($abort_flag) {
        assume {:print "$at(2,10481,10524)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,11):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_local[obj_addr]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:13+8
    assume {:print "$track_local(52,11,3):", $t13} $t13 == $t13;

    // $t14 := true at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:232:59+4
    assume {:print "$at(2,10584,10588)"} true;
    $t14 := true;
    assume $IsValid'bool'($t14);

    // $t15 := object::create_object_internal($t0, $t13, $t14) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:232:9+55
    call $t15 := $1_object_create_object_internal($t0, $t13, $t14);
    if ($abort_flag) {
        assume {:print "$at(2,10534,10589)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,11):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_return[0]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:232:9+55
    assume {:print "$track_return(52,11,0):", $t15} $t15 == $t15;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:233:5+1
    assume {:print "$at(2,10594,10595)"} true;
L1:

    // return $t15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:233:5+1
    assume {:print "$at(2,10594,10595)"} true;
    $ret0 := $t15;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:233:5+1
L2:

    // abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:233:5+1
    assume {:print "$at(2,10594,10595)"} true;
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun object::create_object_from_guid [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+444
procedure {:timeLimit 40} $1_object_create_object_from_guid$verify(_$t0: int, _$t1: $1_guid_GUID) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t2: Vec (int);
    var $t3: int;
    var $t4: Vec (int);
    var $t5: Vec (int);
    var $t6: Vec (int);
    var $t7: Vec (int);
    var $t8: Vec (int);
    var $t9: Vec (int);
    var $t10: Vec (int);
    var $t11: Vec (int);
    var $t12: int;
    var $t13: int;
    var $t14: int;
    var $t15: int;
    var $t16: $Mutation (Vec (int));
    var $t17: int;
    var $t18: Vec (int);
    var $t19: bool;
    var $t20: bool;
    var $t21: Vec (int);
    var $t22: Vec (int);
    var $t23: int;
    var $t24: bool;
    var $t25: $1_object_ConstructorRef;
    var $t26: bool;
    var $t27: bool;
    var $t28: bool;
    var $t29: bool;
    var $t0: int;
    var $t1: $1_guid_GUID;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'vec'u8'': Vec (int);
    var $1_object_ObjectCore_$memory#38: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume {:print "$at(2,10151,10152)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume $IsValid'$1_guid_GUID'($t1);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:40+4]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:40+4
    assume {:print "$at(3,3567,3571)"} true;
    assume {:print "$track_exp_sub(25843):", $t1} true;

    // assume Identical($t4, bcs::$to_bytes<guid::GUID>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:26+19
    assume ($t4 == $1_bcs_$to_bytes'$1_guid_GUID'($t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:26+19]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:26+19
    assume {:print "$track_exp_sub(25844):", $t4} true;

    // assume Identical($t5, bcs::$to_bytes<guid::GUID>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume ($t5 == $1_bcs_$to_bytes'$1_guid_GUID'($t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$track_exp(25845):", $t5} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$track_global_mem(27279):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t6, bcs::$to_bytes<guid::GUID>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume ($t6 == $1_bcs_$to_bytes'$1_guid_GUID'($t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:28+10]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:28+10
    assume {:print "$at(3,3601,3611)"} true;
    assume {:print "$track_exp_sub(25849):", $t6} true;

    // assume Identical($t7, ConcatVec<u8>($t6, SingleVec<u8>(253))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume ($t7 == ConcatVec($t6, MakeVec1(253)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$track_exp(25850):", $t7} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$track_global_mem(27280):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t8, ConcatVec<u8>($t6, SingleVec<u8>(253))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume ($t8 == ConcatVec($t6, MakeVec1(253)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:41+5]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:41+5
    assume {:print "$at(3,3695,3700)"} true;
    assume {:print "$track_exp_sub(25855):", $t8} true;

    // assume Identical($t9, hash::$sha3_256($t8)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:26+21
    assume ($t9 == $1_hash_$sha3_256($t8));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:26+21]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:26+21
    assume {:print "$track_exp_sub(25856):", $t9} true;

    // assume Identical($t10, hash::$sha3_256($t8)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume ($t10 == $1_hash_$sha3_256($t8));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$track_exp(25857):", $t10} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$track_global_mem(27281):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t11, hash::$sha3_256($t8)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume ($t11 == $1_hash_$sha3_256($t8));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:55+10]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:55+10
    assume {:print "$at(3,3757,3767)"} true;
    assume {:print "$track_exp_sub(25862):", $t11} true;

    // assume Identical($t12, from_bcs::deserialize<address>($t11)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:24+42
    assume ($t12 == $1_from_bcs_deserialize'address'($t11));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:24+42]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:24+42
    assume {:print "$track_exp_sub(25863):", $t12} true;

    // assume Identical($t13, from_bcs::deserialize<address>($t11)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58
    assume ($t13 == $1_from_bcs_deserialize'address'($t11));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58
    assume {:print "$track_exp(25864):", $t13} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58
    assume {:print "$track_global_mem(27282):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t14, from_bcs::deserialize<address>($t11)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58
    assume ($t14 == $1_from_bcs_deserialize'address'($t11));

    // @38 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume {:print "$at(2,10151,10152)"} true;
    $1_object_ObjectCore_$memory#38 := $1_object_ObjectCore_$memory;

    // trace_local[creator_address]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume {:print "$track_local(52,11,0):", $t0} $t0 == $t0;

    // trace_local[guid]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:225:5+1
    assume {:print "$track_local(52,11,1):", $t1} $t1 == $t1;

    // $t2 := bcs::to_bytes<guid::GUID>($t1) on_abort goto L2 with $t15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:226:21+20
    assume {:print "$at(2,10261,10281)"} true;
    call $t2 := $1_bcs_to_bytes'$1_guid_GUID'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,10261,10281)"} true;
        $t15 := $abort_code;
        assume {:print "$track_abort(52,11):", $t15} $t15 == $t15;
        goto L2;
    }

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:226:13+5
    assume {:print "$track_local(52,11,2):", $t2} $t2 == $t2;

    // $t16 := borrow_local($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:27+10
    assume {:print "$at(2,10309,10319)"} true;
    $t16 := $Mutation($Local(2), EmptyVec(), $t2);

    // $t17 := 253 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:39+31
    $t17 := 253;
    assume $IsValid'u8'($t17);

    // vector::push_back<u8>($t16, $t17) on_abort goto L2 with $t15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:9+62
    call $t16 := $1_vector_push_back'u8'($t16, $t17);
    if ($abort_flag) {
        assume {:print "$at(2,10291,10353)"} true;
        $t15 := $abort_code;
        assume {:print "$track_abort(52,11):", $t15} $t15 == $t15;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:9+62
    $t2 := $Dereference($t16);

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:227:9+62
    assume {:print "$track_local(52,11,2):", $t2} $t2 == $t2;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:69+5]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:69+5
    assume {:print "$at(2,10438,10443)"} true;
    assume {:print "$track_exp_sub(25876):", $t2} true;

    // assume Identical($t18, hash::$sha3_256($t2)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:54+21
    assume ($t18 == $1_hash_$sha3_256($t2));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:54+21]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:54+21
    assume {:print "$track_exp_sub(25877):", $t18} true;

    // assume Identical($t19, from_bcs::deserializable<address>(hash::$sha3_256($t2))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:20+56
    assume ($t19 == $1_from_bcs_deserializable'address'($1_hash_$sha3_256($t2)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:20+56]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:20+56
    assume {:print "$track_exp_sub(25878):", $t19} true;

    // assume Identical($t20, from_bcs::deserializable<address>(hash::$sha3_256($t2))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:13+64
    assume ($t20 == $1_from_bcs_deserializable'address'($1_hash_$sha3_256($t2)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:13+64]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:13+64
    assume {:print "$track_exp(25879):", $t20} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:13+64
    assume {:print "$track_global_mem(27283):", $1_object_ObjectCore_$memory} true;

    // assume from_bcs::deserializable<address>(hash::$sha3_256($t2)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:229:13+64
    assume $1_from_bcs_deserializable'address'($1_hash_$sha3_256($t2));

    // $t21 := move($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:60+5
    assume {:print "$at(2,10517,10522)"} true;
    $t21 := $t2;

    // $t22 := hash::sha3_256($t21) on_abort goto L2 with $t15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:45+21
    call $t22 := $1_hash_sha3_256($t21);
    if ($abort_flag) {
        assume {:print "$at(2,10502,10523)"} true;
        $t15 := $abort_code;
        assume {:print "$track_abort(52,11):", $t15} $t15 == $t15;
        goto L2;
    }

    // $t23 := from_bcs::to_address($t22) on_abort goto L2 with $t15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:24+43
    call $t23 := $1_from_bcs_to_address($t22);
    if ($abort_flag) {
        assume {:print "$at(2,10481,10524)"} true;
        $t15 := $abort_code;
        assume {:print "$track_abort(52,11):", $t15} $t15 == $t15;
        goto L2;
    }

    // trace_local[obj_addr]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:231:13+8
    assume {:print "$track_local(52,11,3):", $t23} $t23 == $t23;

    // $t24 := true at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:232:59+4
    assume {:print "$at(2,10584,10588)"} true;
    $t24 := true;
    assume $IsValid'bool'($t24);

    // $t25 := object::create_object_internal($t0, $t23, $t24) on_abort goto L2 with $t15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:232:9+55
    call $t25 := $1_object_create_object_internal($t0, $t23, $t24);
    if ($abort_flag) {
        assume {:print "$at(2,10534,10589)"} true;
        $t15 := $abort_code;
        assume {:print "$track_abort(52,11):", $t15} $t15 == $t15;
        goto L2;
    }

    // trace_return[0]($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:232:9+55
    assume {:print "$track_return(52,11,0):", $t25} $t25 == $t25;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:233:5+1
    assume {:print "$at(2,10594,10595)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:38+8]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:38+8
    assume {:print "$at(3,3807,3815)"} true;
    assume {:print "$track_exp_sub(25869):", $t14} true;

    // assume Identical($t26, exists[@38]<object::ObjectCore>($t14)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:19+28
    assume ($t26 == $ResourceExists($1_object_ObjectCore_$memory#38, $t14));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:19+28]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:19+28
    assume {:print "$track_exp_sub(25870):", $t26} true;

    // assume Identical($t27, exists[@38]<object::ObjectCore>($t14)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    assume ($t27 == $ResourceExists($1_object_ObjectCore_$memory#38, $t14));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39]($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    assume {:print "$track_exp(25871):", $t27} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    assume {:print "$track_global_mem(27284):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@38]<object::ObjectCore>($t14)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    assert {:msg "assert_failed(3,3778,3817): function does not abort under this condition"}
      !$ResourceExists($1_object_ObjectCore_$memory#38, $t14);

    // return $t25 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    $ret0 := $t25;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:233:5+1
    assume {:print "$at(2,10594,10595)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:38+8]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:38+8
    assume {:print "$at(3,3807,3815)"} true;
    assume {:print "$track_exp_sub(25869):", $t14} true;

    // assume Identical($t28, exists[@38]<object::ObjectCore>($t14)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:19+28
    assume ($t28 == $ResourceExists($1_object_ObjectCore_$memory#38, $t14));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:19+28]($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:19+28
    assume {:print "$track_exp_sub(25870):", $t28} true;

    // assume Identical($t29, exists[@38]<object::ObjectCore>($t14)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    assume ($t29 == $ResourceExists($1_object_ObjectCore_$memory#38, $t14));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39]($t29) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    assume {:print "$track_exp(25871):", $t29} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:95:9+39
    assume {:print "$track_global_mem(27285):", $1_object_ObjectCore_$memory} true;

    // assert exists[@38]<object::ObjectCore>($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:90:5+386
    assume {:print "$at(3,3437,3823)"} true;
    assert {:msg "assert_failed(3,3437,3823): abort not covered by any of the `aborts_if` clauses"}
      $ResourceExists($1_object_ObjectCore_$memory#38, $t14);

    // abort($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:90:5+386
    $abort_code := $t15;
    $abort_flag := true;
    return;

}

// fun object::create_object_from_object [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:220:5+206
procedure {:timeLimit 40} $1_object_create_object_from_object$verify(_$t0: $signer) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t1: $1_guid_GUID;
    var $t2: int;
    var $t3: $1_object_ObjectCore;
    var $t4: $1_object_ObjectCore;
    var $t5: $1_object_ObjectCore;
    var $t6: $1_object_ObjectCore;
    var $t7: $1_guid_GUID;
    var $t8: int;
    var $t9: int;
    var $t10: Vec (int);
    var $t11: Vec (int);
    var $t12: Vec (int);
    var $t13: int;
    var $t14: $1_object_ConstructorRef;
    var $t15: int;
    var $t16: bool;
    var $t17: bool;
    var $t18: bool;
    var $t19: int;
    var $t20: bool;
    var $t21: bool;
    var $t22: bool;
    var $t0: $signer;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $1_object_ObjectCore_$memory#44: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:220:5+1
    assume {:print "$at(2,9939,9940)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:220:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:65+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:65+7
    assume {:print "$at(3,2997,3004)"} true;
    assume {:print "$track_exp_sub(26214):", $t0} true;

    // assume Identical($t2, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:46+27
    assume ($t2 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:46+27]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:46+27
    assume {:print "$track_exp_sub(26215):", $t2} true;

    // assume Identical($t3, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:27+47
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:27+47]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:27+47
    assume {:print "$track_exp_sub(26216):", $t3} true;

    // assume Identical($t4, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:9+66
    assume ($t4 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:9+66]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:9+66
    assume {:print "$track_exp(26217):", $t4} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:9+66
    assume {:print "$track_global_mem(27286):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:79:9+66
    assume ($t5 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // @44 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:220:5+1
    assume {:print "$at(2,9939,9940)"} true;
    $1_object_ObjectCore_$memory#44 := $1_object_ObjectCore_$memory;

    // trace_local[creator]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:220:5+1
    assume {:print "$track_local(52,12,0):", $t0} $t0 == $t0;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:221:20+20
    assume {:print "$at(2,10051,10071)"} true;
    assume {:print "$track_global_mem(27287):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t6, global<object::ObjectCore>(signer::$address_of($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:122:9+65
    assume {:print "$at(3,4573,4638)"} true;
    assume ($t6 == $ResourceValue($1_object_ObjectCore_$memory, $1_signer_$address_of($t0)));

    // $t7 := object::create_guid($t0) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:221:20+20
    assume {:print "$at(2,10051,10071)"} true;
    call $t7 := $1_object_create_guid($t0);
    if ($abort_flag) {
        assume {:print "$at(2,10051,10071)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,12):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_local[guid]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:221:13+4
    assume {:print "$track_local(52,12,1):", $t7} $t7 == $t7;

    // $t9 := signer::address_of($t0) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:33+27
    assume {:print "$at(2,10105,10132)"} true;
    call $t9 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,10105,10132)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,12):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:9+58
    assume {:print "$track_global_mem(27288):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t10, bcs::$to_bytes<guid::GUID>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$at(3,3536,3573)"} true;
    assume ($t10 == $1_bcs_$to_bytes'$1_guid_GUID'($t7));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:91:9+37
    assume {:print "$track_global_mem(27289):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t11, ConcatVec<u8>($t10, SingleVec<u8>(253))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$at(3,3582,3654)"} true;
    assume ($t11 == ConcatVec($t10, MakeVec1(253)));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:92:9+72
    assume {:print "$track_global_mem(27290):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t12, hash::$sha3_256($t11)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$at(3,3663,3702)"} true;
    assume ($t12 == $1_hash_$sha3_256($t11));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:93:9+39
    assume {:print "$track_global_mem(27291):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t13, from_bcs::deserialize<address>($t12)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:94:9+58
    assume {:print "$at(3,3711,3769)"} true;
    assume ($t13 == $1_from_bcs_deserialize'address'($t12));

    // $t14 := object::create_object_from_guid($t9, $t7) on_abort goto L2 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:9+58
    assume {:print "$at(2,10081,10139)"} true;
    call $t14 := $1_object_create_object_from_guid($t9, $t7);
    if ($abort_flag) {
        assume {:print "$at(2,10081,10139)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,12):", $t8} $t8 == $t8;
        goto L2;
    }

    // trace_return[0]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:222:9+58
    assume {:print "$track_return(52,12,0):", $t14} $t14 == $t14;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:223:5+1
    assume {:print "$at(2,10144,10145)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:58+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:58+7
    assume {:print "$at(3,2896,2903)"} true;
    assume {:print "$track_exp_sub(26223):", $t0} true;

    // assume Identical($t15, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:39+27
    assume ($t15 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:39+27]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:39+27
    assume {:print "$track_exp_sub(26224):", $t15} true;

    // assume Identical($t16, exists[@44]<object::ObjectCore>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:20+47
    assume ($t16 == $ResourceExists($1_object_ObjectCore_$memory#44, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:20+47]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:20+47
    assume {:print "$track_exp_sub(26225):", $t16} true;

    // assume Identical($t17, Not(exists[@44]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59
    assume ($t17 == !$ResourceExists($1_object_ObjectCore_$memory#44, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59
    assume {:print "$track_exp(26226):", $t17} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59
    assume {:print "$track_global_mem(27292):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@44]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59
    assert {:msg "assert_failed(3,2847,2906): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#44, $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:19+11]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:19+11
    assume {:print "$at(3,3026,3037)"} true;
    assume {:print "$track_exp_sub(26230):", $t5} true;

    // assume Identical($t18, Gt(Add(select object::ObjectCore.guid_creation_num($t5), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    assume ($t18 == (($guid_creation_num#$1_object_ObjectCore($t5) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    assume {:print "$track_exp(26231):", $t18} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    assume {:print "$track_global_mem(27293):", $1_object_ObjectCore_$memory} true;

    // assert Not(Gt(Add(select object::ObjectCore.guid_creation_num($t5), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    assert {:msg "assert_failed(3,3016,3070): function does not abort under this condition"}
      !(($guid_creation_num#$1_object_ObjectCore($t5) + 1) > 18446744073709551615);

    // return $t14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    $ret0 := $t14;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:223:5+1
    assume {:print "$at(2,10144,10145)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:58+7]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:58+7
    assume {:print "$at(3,2896,2903)"} true;
    assume {:print "$track_exp_sub(26223):", $t0} true;

    // assume Identical($t19, signer::$address_of[]($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:39+27
    assume ($t19 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:39+27]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:39+27
    assume {:print "$track_exp_sub(26224):", $t19} true;

    // assume Identical($t20, exists[@44]<object::ObjectCore>(signer::$address_of[]($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:20+47
    assume ($t20 == $ResourceExists($1_object_ObjectCore_$memory#44, $1_signer_$address_of($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:20+47]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:20+47
    assume {:print "$track_exp_sub(26225):", $t20} true;

    // assume Identical($t21, Not(exists[@44]<object::ObjectCore>(signer::$address_of[]($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59
    assume ($t21 == !$ResourceExists($1_object_ObjectCore_$memory#44, $1_signer_$address_of($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:77:9+59
    assume {:print "$track_exp(26226):", $t21} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:19+11]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:19+11
    assume {:print "$at(3,3026,3037)"} true;
    assume {:print "$track_exp_sub(26230):", $t5} true;

    // assume Identical($t22, Gt(Add(select object::ObjectCore.guid_creation_num($t5), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    assume ($t22 == (($guid_creation_num#$1_object_ObjectCore($t5) + 1) > 18446744073709551615));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    assume {:print "$track_exp(26231):", $t22} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:80:9+54
    assume {:print "$track_global_mem(27294):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@44]<object::ObjectCore>(signer::$address_of[]($t0))), Gt(Add(select object::ObjectCore.guid_creation_num($t5), 1), 18446744073709551615)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:76:5+658
    assume {:print "$at(3,2773,3431)"} true;
    assert {:msg "assert_failed(3,2773,3431): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#44, $1_signer_$address_of($t0)) || (($guid_creation_num#$1_object_ObjectCore($t5) + 1) > 18446744073709551615));

    // abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:76:5+658
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun object::create_object_internal [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+766
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
    // trace_local[creator_address]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume {:print "$at(2,10601,10602)"} true;
    assume {:print "$track_local(52,13,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume {:print "$track_local(52,13,1):", $t1} $t1 == $t1;

    // trace_local[can_delete]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume {:print "$track_local(52,13,2):", $t2} $t2 == $t2;

    // $t6 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:18+6
    assume {:print "$at(2,10755,10761)"} true;
    $t6 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // $t7 := !($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:17+1
    call $t7 := $Not($t6);

    // if ($t7) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    assume {:print "$at(2,10746,10821)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:68+14
L0:

    // $t8 := 1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:68+14
    assume {:print "$at(2,10805,10819)"} true;
    $t8 := 1;
    assume $IsValid'u64'($t8);

    // $t9 := error::already_exists($t8) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:46+37
    call $t9 := $1_error_already_exists($t8);
    if ($abort_flag) {
        assume {:print "$at(2,10783,10820)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // trace_abort($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    assume {:print "$at(2,10746,10821)"} true;
    assume {:print "$track_abort(52,13):", $t9} $t9 == $t9;

    // $t10 := move($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    $t10 := $t9;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:43+6
    assume {:print "$at(2,10866,10872)"} true;
L2:

    // $t11 := opaque begin: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21
    assume {:print "$at(2,10852,10873)"} true;

    // assume WellFormed($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21
    assume $IsValid'signer'($t11) && $1_signer_is_txn_signer($t11) && $1_signer_is_txn_signer_addr($addr#$signer($t11));

    // assume Eq<address>(signer::$address_of($t11), $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21
    assume $IsEqual'address'($1_signer_$address_of($t11), $t1);

    // $t11 := opaque end: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21

    // trace_local[object_signer]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:13+13
    assume {:print "$track_local(52,13,4):", $t11} $t11 == $t11;

    // $t12 := 1125899906842624 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:243:33+22
    assume {:print "$at(2,10907,10929)"} true;
    $t12 := 1125899906842624;
    assume $IsValid'u64'($t12);

    // $t3 := $t12 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:243:13+17
    $t3 := $t12;

    // trace_local[guid_creation_num]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:243:13+17
    assume {:print "$track_local(52,13,3):", $t3} $t3 == $t3;

    // $t13 := borrow_local($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:57+22
    assume {:print "$at(2,10987,11009)"} true;
    $t13 := $Mutation($Local(3), EmptyVec(), $t3);

    // $t14 := guid::create($t1, $t13) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:36+44
    call $t14,$t13 := $1_guid_create($t1, $t13);
    if ($abort_flag) {
        assume {:print "$at(2,10966,11010)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // write_back[LocalRoot($t3)@]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:36+44
    $t3 := $Dereference($t13);

    // trace_local[guid_creation_num]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:36+44
    assume {:print "$track_local(52,13,3):", $t3} $t3 == $t3;

    // trace_local[transfer_events_guid]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:13+20
    assume {:print "$track_local(52,13,5):", $t14} $t14 == $t14;

    // $t15 := move($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:249:17+17
    assume {:print "$at(2,11099,11116)"} true;
    $t15 := $t3;

    // $t16 := true at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:251:41+4
    assume {:print "$at(2,11198,11202)"} true;
    $t16 := true;
    assume $IsValid'bool'($t16);

    // $t17 := event::new_event_handle<object::TransferEvent>($t14) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:252:34+45
    assume {:print "$at(2,11237,11282)"} true;
    call $t17 := $1_event_new_event_handle'$1_object_TransferEvent'($t14);
    if ($abort_flag) {
        assume {:print "$at(2,11237,11282)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // $t18 := pack object::ObjectCore($t15, $t0, $t16, $t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:248:13+227
    assume {:print "$at(2,11070,11297)"} true;
    $t18 := $1_object_ObjectCore($t15, $t0, $t16, $t17);

    // move_to<object::ObjectCore>($t18, $t11) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:246:9+7
    assume {:print "$at(2,11021,11028)"} true;
    if ($ResourceExists($1_object_ObjectCore_$memory, $addr#$signer($t11))) {
        call $ExecFailureAbort();
    } else {
        $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $addr#$signer($t11), $t18);
    }
    if ($abort_flag) {
        assume {:print "$at(2,11021,11028)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // $t19 := pack object::ConstructorRef($t1, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:255:9+43
    assume {:print "$at(2,11318,11361)"} true;
    $t19 := $1_object_ConstructorRef($t1, $t2);

    // trace_return[0]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:255:9+43
    assume {:print "$track_return(52,13,0):", $t19} $t19 == $t19;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(2,11366,11367)"} true;
L3:

    // return $t19 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(2,11366,11367)"} true;
    $ret0 := $t19;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
L4:

    // abort($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(2,11366,11367)"} true;
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun object::create_object_internal [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+766
procedure {:timeLimit 40} $1_object_create_object_internal$verify(_$t0: int, _$t1: int, _$t2: bool) returns ($ret0: $1_object_ConstructorRef)
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
    var $t20: bool;
    var $t21: bool;
    var $t22: bool;
    var $t23: bool;
    var $t0: int;
    var $t1: int;
    var $t2: bool;
    var $temp_0'$1_guid_GUID': $1_guid_GUID;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $temp_0'u64': int;
    var $1_object_ObjectCore_$memory#26: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume {:print "$at(2,10601,10602)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume $IsValid'address'($t1);

    // assume WellFormed($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume $IsValid'bool'($t2);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @26 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    $1_object_ObjectCore_$memory#26 := $1_object_ObjectCore_$memory;

    // trace_local[creator_address]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume {:print "$track_local(52,13,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume {:print "$track_local(52,13,1):", $t1} $t1 == $t1;

    // trace_local[can_delete]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:235:5+1
    assume {:print "$track_local(52,13,2):", $t2} $t2 == $t2;

    // $t6 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:18+6
    assume {:print "$at(2,10755,10761)"} true;
    $t6 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // $t7 := !($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:17+1
    call $t7 := $Not($t6);

    // if ($t7) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    if ($t7) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    assume {:print "$at(2,10746,10821)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:68+14
L0:

    // $t8 := 1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:68+14
    assume {:print "$at(2,10805,10819)"} true;
    $t8 := 1;
    assume $IsValid'u64'($t8);

    // $t9 := error::already_exists($t8) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:46+37
    call $t9 := $1_error_already_exists($t8);
    if ($abort_flag) {
        assume {:print "$at(2,10783,10820)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // trace_abort($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    assume {:print "$at(2,10746,10821)"} true;
    assume {:print "$track_abort(52,13):", $t9} $t9 == $t9;

    // $t10 := move($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    $t10 := $t9;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:240:9+75
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:43+6
    assume {:print "$at(2,10866,10872)"} true;
L2:

    // $t11 := opaque begin: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21
    assume {:print "$at(2,10852,10873)"} true;

    // assume WellFormed($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21
    assume $IsValid'signer'($t11) && $1_signer_is_txn_signer($t11) && $1_signer_is_txn_signer_addr($addr#$signer($t11));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21
    assume {:print "$track_global_mem(27295):", $1_object_ObjectCore_$memory} true;

    // assume Eq<address>(signer::$address_of($t11), $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21
    assume $IsEqual'address'($1_signer_$address_of($t11), $t1);

    // $t11 := opaque end: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:29+21

    // trace_local[object_signer]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:242:13+13
    assume {:print "$track_local(52,13,4):", $t11} $t11 == $t11;

    // $t12 := 1125899906842624 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:243:33+22
    assume {:print "$at(2,10907,10929)"} true;
    $t12 := 1125899906842624;
    assume $IsValid'u64'($t12);

    // $t3 := $t12 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:243:13+17
    $t3 := $t12;

    // trace_local[guid_creation_num]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:243:13+17
    assume {:print "$track_local(52,13,3):", $t3} $t3 == $t3;

    // $t13 := borrow_local($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:57+22
    assume {:print "$at(2,10987,11009)"} true;
    $t13 := $Mutation($Local(3), EmptyVec(), $t3);

    // $t14 := guid::create($t1, $t13) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:36+44
    call $t14,$t13 := $1_guid_create($t1, $t13);
    if ($abort_flag) {
        assume {:print "$at(2,10966,11010)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // write_back[LocalRoot($t3)@]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:36+44
    $t3 := $Dereference($t13);

    // trace_local[guid_creation_num]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:36+44
    assume {:print "$track_local(52,13,3):", $t3} $t3 == $t3;

    // trace_local[transfer_events_guid]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:244:13+20
    assume {:print "$track_local(52,13,5):", $t14} $t14 == $t14;

    // $t15 := move($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:249:17+17
    assume {:print "$at(2,11099,11116)"} true;
    $t15 := $t3;

    // $t16 := true at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:251:41+4
    assume {:print "$at(2,11198,11202)"} true;
    $t16 := true;
    assume $IsValid'bool'($t16);

    // $t17 := event::new_event_handle<object::TransferEvent>($t14) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:252:34+45
    assume {:print "$at(2,11237,11282)"} true;
    call $t17 := $1_event_new_event_handle'$1_object_TransferEvent'($t14);
    if ($abort_flag) {
        assume {:print "$at(2,11237,11282)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // $t18 := pack object::ObjectCore($t15, $t0, $t16, $t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:248:13+227
    assume {:print "$at(2,11070,11297)"} true;
    $t18 := $1_object_ObjectCore($t15, $t0, $t16, $t17);

    // move_to<object::ObjectCore>($t18, $t11) on_abort goto L4 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:246:9+7
    assume {:print "$at(2,11021,11028)"} true;
    if ($ResourceExists($1_object_ObjectCore_$memory, $addr#$signer($t11))) {
        call $ExecFailureAbort();
    } else {
        $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $addr#$signer($t11), $t18);
    }
    if ($abort_flag) {
        assume {:print "$at(2,11021,11028)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,13):", $t10} $t10 == $t10;
        goto L4;
    }

    // $t19 := pack object::ConstructorRef($t1, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:255:9+43
    assume {:print "$at(2,11318,11361)"} true;
    $t19 := $1_object_ConstructorRef($t1, $t2);

    // trace_return[0]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:255:9+43
    assume {:print "$track_return(52,13,0):", $t19} $t19 == $t19;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(2,11366,11367)"} true;
L3:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:38+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:38+6
    assume {:print "$at(3,4004,4010)"} true;
    assume {:print "$track_exp_sub(25486):", $t1} true;

    // assume Identical($t20, exists[@26]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:19+26
    assume ($t20 == $ResourceExists($1_object_ObjectCore_$memory#26, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:19+26]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:19+26
    assume {:print "$track_exp_sub(25487):", $t20} true;

    // assume Identical($t21, exists[@26]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    assume ($t21 == $ResourceExists($1_object_ObjectCore_$memory#26, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    assume {:print "$track_exp(25488):", $t21} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    assume {:print "$track_global_mem(27296):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@26]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    assert {:msg "assert_failed(3,3975,4012): function does not abort under this condition"}
      !$ResourceExists($1_object_ObjectCore_$memory#26, $t1);

    // return $t19 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    $ret0 := $t19;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:256:5+1
    assume {:print "$at(2,11366,11367)"} true;
L4:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:38+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:38+6
    assume {:print "$at(3,4004,4010)"} true;
    assume {:print "$track_exp_sub(25486):", $t1} true;

    // assume Identical($t22, exists[@26]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:19+26
    assume ($t22 == $ResourceExists($1_object_ObjectCore_$memory#26, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:19+26]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:19+26
    assume {:print "$track_exp_sub(25487):", $t22} true;

    // assume Identical($t23, exists[@26]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    assume ($t23 == $ResourceExists($1_object_ObjectCore_$memory#26, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    assume {:print "$track_exp(25488):", $t23} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:103:9+37
    assume {:print "$track_global_mem(27297):", $1_object_ObjectCore_$memory} true;

    // assert exists[@26]<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:98:5+189
    assume {:print "$at(3,3829,4018)"} true;
    assert {:msg "assert_failed(3,3829,4018): abort not covered by any of the `aborts_if` clauses"}
      $ResourceExists($1_object_ObjectCore_$memory#26, $t1);

    // abort($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:98:5+189
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun object::create_user_derived_object [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:208:5+277
procedure {:timeLimit 40} $1_object_create_user_derived_object$verify(_$t0: int, _$t1: $1_object_DeriveRef) returns ($ret0: $1_object_ConstructorRef)
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: bool;
    var $t9: $1_object_ConstructorRef;
    var $t10: int;
    var $t11: bool;
    var $t12: bool;
    var $t13: bool;
    var $t14: bool;
    var $t0: int;
    var $t1: $1_object_DeriveRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_DeriveRef': $1_object_DeriveRef;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#47: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:208:5+1
    assume {:print "$at(2,9325,9326)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:208:5+1
    assume $IsValid'$1_object_DeriveRef'($t1);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:208:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:64+15]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:64+15
    assume {:print "$at(3,2030,2045)"} true;
    assume {:print "$track_exp_sub(26445):", $t0} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:81+10]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:81+10
    assume {:print "$track_exp_sub(26446):", $t1} true;

    // assume Identical($t3, object::spec_create_user_derived_object_address($t0, select object::DeriveRef.self($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:24+73
    assume ($t3 == $1_object_spec_create_user_derived_object_address($t0, $self#$1_object_DeriveRef($t1)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:24+73]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:24+73
    assume {:print "$track_exp_sub(26447):", $t3} true;

    // assume Identical($t4, object::spec_create_user_derived_object_address($t0, select object::DeriveRef.self($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:9+89
    assume ($t4 == $1_object_spec_create_user_derived_object_address($t0, $self#$1_object_DeriveRef($t1)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:9+89]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:9+89
    assume {:print "$track_exp(26448):", $t4} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:9+89
    assume {:print "$track_global_mem(27298):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, object::spec_create_user_derived_object_address($t0, select object::DeriveRef.self($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:56:9+89
    assume ($t5 == $1_object_spec_create_user_derived_object_address($t0, $self#$1_object_DeriveRef($t1)));

    // @47 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:208:5+1
    assume {:print "$at(2,9325,9326)"} true;
    $1_object_ObjectCore_$memory#47 := $1_object_ObjectCore_$memory;

    // trace_local[creator_address]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:208:5+1
    assume {:print "$track_local(52,14,0):", $t0} $t0 == $t0;

    // trace_local[derive_ref]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:208:5+1
    assume {:print "$track_local(52,14,1):", $t1} $t1 == $t1;

    // $t6 := get_field<object::DeriveRef>.self($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:209:76+15
    assume {:print "$at(2,9514,9529)"} true;
    $t6 := $self#$1_object_DeriveRef($t1);

    // $t7 := opaque begin: object::create_user_derived_object_address($t0, $t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:209:24+68

    // assume WellFormed($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:209:24+68
    assume $IsValid'address'($t7);

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:209:24+68
    assume {:print "$track_global_mem(27299):", $1_object_ObjectCore_$memory} true;

    // assume Eq<address>($t7, object::spec_create_user_derived_object_address($t0, $t6)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:209:24+68
    assume $IsEqual'address'($t7, $1_object_spec_create_user_derived_object_address($t0, $t6));

    // $t7 := opaque end: object::create_user_derived_object_address($t0, $t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:209:24+68

    // trace_local[obj_addr]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:209:13+8
    assume {:print "$track_local(52,14,2):", $t7} $t7 == $t7;

    // $t8 := false at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:210:59+5
    assume {:print "$at(2,9590,9595)"} true;
    $t8 := false;
    assume $IsValid'bool'($t8);

    // $t9 := object::create_object_internal($t0, $t7, $t8) on_abort goto L2 with $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:210:9+56
    call $t9 := $1_object_create_object_internal($t0, $t7, $t8);
    if ($abort_flag) {
        assume {:print "$at(2,9540,9596)"} true;
        $t10 := $abort_code;
        assume {:print "$track_abort(52,14):", $t10} $t10 == $t10;
        goto L2;
    }

    // trace_return[0]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:210:9+56
    assume {:print "$track_return(52,14,0):", $t9} $t9 == $t9;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:211:5+1
    assume {:print "$at(2,9601,9602)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:38+8]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:38+8
    assume {:print "$at(3,2102,2110)"} true;
    assume {:print "$track_exp_sub(26453):", $t5} true;

    // assume Identical($t11, exists[@47]<object::ObjectCore>($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:19+28
    assume ($t11 == $ResourceExists($1_object_ObjectCore_$memory#47, $t5));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:19+28]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:19+28
    assume {:print "$track_exp_sub(26454):", $t11} true;

    // assume Identical($t12, exists[@47]<object::ObjectCore>($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    assume ($t12 == $ResourceExists($1_object_ObjectCore_$memory#47, $t5));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    assume {:print "$track_exp(26455):", $t12} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    assume {:print "$track_global_mem(27300):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@47]<object::ObjectCore>($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    assert {:msg "assert_failed(3,2073,2112): function does not abort under this condition"}
      !$ResourceExists($1_object_ObjectCore_$memory#47, $t5);

    // return $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    $ret0 := $t9;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:211:5+1
    assume {:print "$at(2,9601,9602)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:38+8]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:38+8
    assume {:print "$at(3,2102,2110)"} true;
    assume {:print "$track_exp_sub(26453):", $t5} true;

    // assume Identical($t13, exists[@47]<object::ObjectCore>($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:19+28
    assume ($t13 == $ResourceExists($1_object_ObjectCore_$memory#47, $t5));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:19+28]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:19+28
    assume {:print "$track_exp_sub(26454):", $t13} true;

    // assume Identical($t14, exists[@47]<object::ObjectCore>($t5)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    assume ($t14 == $ResourceExists($1_object_ObjectCore_$memory#47, $t5));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    assume {:print "$track_exp(26455):", $t14} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:57:9+39
    assume {:print "$track_global_mem(27301):", $1_object_ObjectCore_$memory} true;

    // assert exists[@47]<object::ObjectCore>($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:55:5+251
    assume {:print "$at(3,1867,2118)"} true;
    assert {:msg "assert_failed(3,1867,2118): abort not covered by any of the `aborts_if` clauses"}
      $ResourceExists($1_object_ObjectCore_$memory#47, $t5);

    // abort($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:55:5+251
    $abort_code := $t10;
    $abort_flag := true;
    return;

}

// fun object::create_user_derived_object_address [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:171:5+324
procedure {:timeLimit 40} $1_object_create_user_derived_object_address$verify(_$t0: int, _$t1: int) returns ($ret0: int)
{
    // declare local variables
    var $t2: Vec (int);
    var $t3: int;
    var $t4: $Mutation (Vec (int));
    var $t5: Vec (int);
    var $t6: $Mutation (Vec (int));
    var $t7: int;
    var $t8: Vec (int);
    var $t9: Vec (int);
    var $t10: int;
    var $t0: int;
    var $t1: int;
    var $temp_0'address': int;
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:171:5+1
    assume {:print "$at(2,7594,7595)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:171:5+1
    assume $IsValid'address'($t1);

    // trace_local[source]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:171:5+1
    assume {:print "$track_local(52,15,0):", $t0} $t0 == $t0;

    // trace_local[derive_from]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:171:5+1
    assume {:print "$track_local(52,15,1):", $t1} $t1 == $t1;

    // $t2 := bcs::to_bytes<address>($t0) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:172:21+22
    assume {:print "$at(2,7710,7732)"} true;
    call $t2 := $1_bcs_to_bytes'address'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,7710,7732)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,15):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:172:13+5
    assume {:print "$track_local(52,15,2):", $t2} $t2 == $t2;

    // $t4 := borrow_local($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:24+10
    assume {:print "$at(2,7757,7767)"} true;
    $t4 := $Mutation($Local(2), EmptyVec(), $t2);

    // $t5 := bcs::to_bytes<address>($t1) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:36+27
    call $t5 := $1_bcs_to_bytes'address'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,7769,7796)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,15):", $t3} $t3 == $t3;
        goto L2;
    }

    // vector::append<u8>($t4, $t5) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+55
    call $t4 := $1_vector_append'u8'($t4, $t5);
    if ($abort_flag) {
        assume {:print "$at(2,7742,7797)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,15):", $t3} $t3 == $t3;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+55
    $t2 := $Dereference($t4);

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:173:9+55
    assume {:print "$track_local(52,15,2):", $t2} $t2 == $t2;

    // $t6 := borrow_local($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:27+10
    assume {:print "$at(2,7825,7835)"} true;
    $t6 := $Mutation($Local(2), EmptyVec(), $t2);

    // $t7 := 252 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:39+21
    $t7 := 252;
    assume $IsValid'u8'($t7);

    // vector::push_back<u8>($t6, $t7) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+52
    call $t6 := $1_vector_push_back'u8'($t6, $t7);
    if ($abort_flag) {
        assume {:print "$at(2,7807,7859)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,15):", $t3} $t3 == $t3;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+52
    $t2 := $Dereference($t6);

    // trace_local[bytes]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:174:9+52
    assume {:print "$track_local(52,15,2):", $t2} $t2 == $t2;

    // $t8 := move($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:45+5
    assume {:print "$at(2,7905,7910)"} true;
    $t8 := $t2;

    // $t9 := hash::sha3_256($t8) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:30+21
    call $t9 := $1_hash_sha3_256($t8);
    if ($abort_flag) {
        assume {:print "$at(2,7890,7911)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,15):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t10 := from_bcs::to_address($t9) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:9+43
    call $t10 := $1_from_bcs_to_address($t9);
    if ($abort_flag) {
        assume {:print "$at(2,7869,7912)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,15):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:175:9+43
    assume {:print "$track_return(52,15,0):", $t10} $t10 == $t10;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(2,7917,7918)"} true;
L1:

    // return $t10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(2,7917,7918)"} true;
    $ret0 := $t10;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
L2:

    // abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:176:5+1
    assume {:print "$at(2,7917,7918)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::delete [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:330:5+342
procedure {:timeLimit 40} $1_object_delete$verify(_$t0: $1_object_DeleteRef) returns ()
{
    // declare local variables
    var $t1: $1_event_EventHandle'$1_object_TransferEvent';
    var $t2: int;
    var $t3: $1_object_ObjectCore;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: bool;
    var $t8: $1_event_EventHandle'$1_object_TransferEvent';
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t12: bool;
    var $t0: $1_object_DeleteRef;
    var $temp_0'$1_event_EventHandle'$1_object_TransferEvent'': $1_event_EventHandle'$1_object_TransferEvent';
    var $temp_0'$1_object_DeleteRef': $1_object_DeleteRef;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#16: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:330:5+1
    assume {:print "$at(2,13997,13998)"} true;
    assume $IsValid'$1_object_DeleteRef'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:330:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @16 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:330:5+1
    $1_object_ObjectCore_$memory#16 := $1_object_ObjectCore_$memory;

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:330:5+1
    assume {:print "$track_local(52,16,0):", $t0} $t0 == $t0;

    // $t2 := get_field<object::DeleteRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:331:49+8
    assume {:print "$at(2,14101,14109)"} true;
    $t2 := $self#$1_object_DeleteRef($t0);

    // $t3 := move_from<object::ObjectCore>($t2) on_abort goto L2 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:331:27+9
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t2)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $ResourceValue($1_object_ObjectCore_$memory, $t2);
        $1_object_ObjectCore_$memory := $ResourceRemove($1_object_ObjectCore_$memory, $t2);
    }
    if ($abort_flag) {
        assume {:print "$at(2,14079,14088)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,16):", $t4} $t4 == $t4;
        goto L2;
    }

    // ($t5, $t6, $t7, $t8) := unpack object::ObjectCore($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:332:13+146
    assume {:print "$at(2,14124,14270)"} true;
    $t5 := $guid_creation_num#$1_object_ObjectCore($t3);
    $t6 := $owner#$1_object_ObjectCore($t3);
    $t7 := $allow_ungated_transfer#$1_object_ObjectCore($t3);
    $t8 := $transfer_events#$1_object_ObjectCore($t3);

    // trace_local[transfer_events]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:336:13+15
    assume {:print "$at(2,14244,14259)"} true;
    assume {:print "$track_local(52,16,1):", $t8} $t8 == $t8;

    // destroy($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:335:37+1
    assume {:print "$at(2,14229,14230)"} true;

    // destroy($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:334:20+1
    assume {:print "$at(2,14190,14191)"} true;

    // destroy($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:333:32+1
    assume {:print "$at(2,14168,14169)"} true;

    // event::destroy_handle<object::TransferEvent>($t8) on_abort goto L2 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:338:9+38
    assume {:print "$at(2,14294,14332)"} true;
    call $1_event_destroy_handle'$1_object_TransferEvent'($t8);
    if ($abort_flag) {
        assume {:print "$at(2,14294,14332)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,16):", $t4} $t4 == $t4;
        goto L2;
    }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:339:5+1
    assume {:print "$at(2,14338,14339)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:39+3
    assume {:print "$at(3,5291,5294)"} true;
    assume {:print "$track_exp_sub(25065):", $t0} true;

    // assume Identical($t9, exists[@16]<object::ObjectCore>(select object::DeleteRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:20+28
    assume ($t9 == $ResourceExists($1_object_ObjectCore_$memory#16, $self#$1_object_DeleteRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:20+28]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:20+28
    assume {:print "$track_exp_sub(25066):", $t9} true;

    // assume Identical($t10, Not(exists[@16]<object::ObjectCore>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    assume ($t10 == !$ResourceExists($1_object_ObjectCore_$memory#16, $self#$1_object_DeleteRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    assume {:print "$track_exp(25067):", $t10} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    assume {:print "$track_global_mem(27302):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@16]<object::ObjectCore>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    assert {:msg "assert_failed(3,5261,5301): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#16, $self#$1_object_DeleteRef($t0));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:339:5+1
    assume {:print "$at(2,14338,14339)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:39+3
    assume {:print "$at(3,5291,5294)"} true;
    assume {:print "$track_exp_sub(25065):", $t0} true;

    // assume Identical($t11, exists[@16]<object::ObjectCore>(select object::DeleteRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:20+28
    assume ($t11 == $ResourceExists($1_object_ObjectCore_$memory#16, $self#$1_object_DeleteRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:20+28]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:20+28
    assume {:print "$track_exp_sub(25066):", $t11} true;

    // assume Identical($t12, Not(exists[@16]<object::ObjectCore>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    assume ($t12 == !$ResourceExists($1_object_ObjectCore_$memory#16, $self#$1_object_DeleteRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    assume {:print "$track_exp(25067):", $t12} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:141:9+40
    assume {:print "$track_global_mem(27303):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@16]<object::ObjectCore>(select object::DeleteRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:140:5+84
    assume {:print "$at(3,5223,5307)"} true;
    assert {:msg "assert_failed(3,5223,5307): abort not covered by any of the `aborts_if` clauses"}
      !$ResourceExists($1_object_ObjectCore_$memory#16, $self#$1_object_DeleteRef($t0));

    // abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:140:5+84
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::disable_ungated_transfer [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:356:5+191
procedure {:timeLimit 40} $1_object_disable_ungated_transfer$verify(_$t0: $1_object_TransferRef) returns ()
{
    // declare local variables
    var $t1: $Mutation ($1_object_ObjectCore);
    var $t2: int;
    var $t3: $Mutation ($1_object_ObjectCore);
    var $t4: int;
    var $t5: bool;
    var $t6: $Mutation (bool);
    var $t7: bool;
    var $t8: bool;
    var $t9: bool;
    var $t10: bool;
    var $t0: $1_object_TransferRef;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'$1_object_TransferRef': $1_object_TransferRef;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#15: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:356:5+1
    assume {:print "$at(2,14781,14782)"} true;
    assume $IsValid'$1_object_TransferRef'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:356:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @15 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:356:5+1
    $1_object_ObjectCore_$memory#15 := $1_object_ObjectCore_$memory;

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:356:5+1
    assume {:print "$track_local(52,17,0):", $t0} $t0 == $t0;

    // $t2 := get_field<object::TransferRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:52+8
    assume {:print "$at(2,14909,14917)"} true;
    $t2 := $self#$1_object_TransferRef($t0);

    // $t3 := borrow_global<object::ObjectCore>($t2) on_abort goto L2 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:22+17
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t2)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $Mutation($Global($t2), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t2));
    }
    if ($abort_flag) {
        assume {:print "$at(2,14879,14896)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,17):", $t4} $t4 == $t4;
        goto L2;
    }

    // trace_local[object]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:357:13+6
    $temp_0'$1_object_ObjectCore' := $Dereference($t3);
    assume {:print "$track_local(52,17,1):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t5 := false at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:41+5
    assume {:print "$at(2,14960,14965)"} true;
    $t5 := false;
    assume $IsValid'bool'($t5);

    // $t6 := borrow_field<object::ObjectCore>.allow_ungated_transfer($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:9+29
    $t6 := $ChildMutation($t3, 2, $allow_ungated_transfer#$1_object_ObjectCore($Dereference($t3)));

    // write_ref($t6, $t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:9+37
    $t6 := $UpdateMutation($t6, $t5);

    // write_back[Reference($t3).allow_ungated_transfer (bool)]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:9+37
    $t3 := $UpdateMutation($t3, $Update'$1_object_ObjectCore'_allow_ungated_transfer($Dereference($t3), $Dereference($t6)));

    // write_back[object::ObjectCore@]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:358:9+37
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t3),
        $Dereference($t3));

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:359:5+1
    assume {:print "$at(2,14971,14972)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:39+3
    assume {:print "$at(3,4219,4222)"} true;
    assume {:print "$track_exp_sub(25022):", $t0} true;

    // assume Identical($t7, exists[@15]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:20+28
    assume ($t7 == $ResourceExists($1_object_ObjectCore_$memory#15, $self#$1_object_TransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:20+28]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:20+28
    assume {:print "$track_exp_sub(25023):", $t7} true;

    // assume Identical($t8, Not(exists[@15]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    assume ($t8 == !$ResourceExists($1_object_ObjectCore_$memory#15, $self#$1_object_TransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    assume {:print "$track_exp(25024):", $t8} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    assume {:print "$track_global_mem(27304):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@15]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    assert {:msg "assert_failed(3,4189,4229): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#15, $self#$1_object_TransferRef($t0));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:359:5+1
    assume {:print "$at(2,14971,14972)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:39+3
    assume {:print "$at(3,4219,4222)"} true;
    assume {:print "$track_exp_sub(25022):", $t0} true;

    // assume Identical($t9, exists[@15]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:20+28
    assume ($t9 == $ResourceExists($1_object_ObjectCore_$memory#15, $self#$1_object_TransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:20+28]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:20+28
    assume {:print "$track_exp_sub(25023):", $t9} true;

    // assume Identical($t10, Not(exists[@15]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    assume ($t10 == !$ResourceExists($1_object_ObjectCore_$memory#15, $self#$1_object_TransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    assume {:print "$track_exp(25024):", $t10} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:111:9+40
    assume {:print "$track_global_mem(27305):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@15]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:110:5+105
    assume {:print "$at(3,4130,4235)"} true;
    assert {:msg "assert_failed(3,4130,4235): abort not covered by any of the `aborts_if` clauses"}
      !$ResourceExists($1_object_ObjectCore_$memory#15, $self#$1_object_TransferRef($t0));

    // abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:110:5+105
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::enable_ungated_transfer [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:362:5+189
procedure {:timeLimit 40} $1_object_enable_ungated_transfer$verify(_$t0: $1_object_TransferRef) returns ()
{
    // declare local variables
    var $t1: $Mutation ($1_object_ObjectCore);
    var $t2: int;
    var $t3: $Mutation ($1_object_ObjectCore);
    var $t4: int;
    var $t5: bool;
    var $t6: $Mutation (bool);
    var $t7: bool;
    var $t8: bool;
    var $t9: bool;
    var $t10: bool;
    var $t0: $1_object_TransferRef;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'$1_object_TransferRef': $1_object_TransferRef;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#14: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:362:5+1
    assume {:print "$at(2,15010,15011)"} true;
    assume $IsValid'$1_object_TransferRef'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:362:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @14 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:362:5+1
    $1_object_ObjectCore_$memory#14 := $1_object_ObjectCore_$memory;

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:362:5+1
    assume {:print "$track_local(52,18,0):", $t0} $t0 == $t0;

    // $t2 := get_field<object::TransferRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:363:52+8
    assume {:print "$at(2,15137,15145)"} true;
    $t2 := $self#$1_object_TransferRef($t0);

    // $t3 := borrow_global<object::ObjectCore>($t2) on_abort goto L2 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:363:22+17
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t2)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $Mutation($Global($t2), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t2));
    }
    if ($abort_flag) {
        assume {:print "$at(2,15107,15124)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,18):", $t4} $t4 == $t4;
        goto L2;
    }

    // trace_local[object]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:363:13+6
    $temp_0'$1_object_ObjectCore' := $Dereference($t3);
    assume {:print "$track_local(52,18,1):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t5 := true at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:41+4
    assume {:print "$at(2,15188,15192)"} true;
    $t5 := true;
    assume $IsValid'bool'($t5);

    // $t6 := borrow_field<object::ObjectCore>.allow_ungated_transfer($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+29
    $t6 := $ChildMutation($t3, 2, $allow_ungated_transfer#$1_object_ObjectCore($Dereference($t3)));

    // write_ref($t6, $t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+36
    $t6 := $UpdateMutation($t6, $t5);

    // write_back[Reference($t3).allow_ungated_transfer (bool)]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+36
    $t3 := $UpdateMutation($t3, $Update'$1_object_ObjectCore'_allow_ungated_transfer($Dereference($t3), $Dereference($t6)));

    // write_back[object::ObjectCore@]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:364:9+36
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t3),
        $Dereference($t3));

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(2,15198,15199)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:39+3
    assume {:print "$at(3,5401,5404)"} true;
    assume {:print "$track_exp_sub(25005):", $t0} true;

    // assume Identical($t7, exists[@14]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:20+28
    assume ($t7 == $ResourceExists($1_object_ObjectCore_$memory#14, $self#$1_object_TransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:20+28]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:20+28
    assume {:print "$track_exp_sub(25006):", $t7} true;

    // assume Identical($t8, Not(exists[@14]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    assume ($t8 == !$ResourceExists($1_object_ObjectCore_$memory#14, $self#$1_object_TransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    assume {:print "$track_exp(25007):", $t8} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    assume {:print "$track_global_mem(27306):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@14]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    assert {:msg "assert_failed(3,5371,5411): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#14, $self#$1_object_TransferRef($t0));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:365:5+1
    assume {:print "$at(2,15198,15199)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:39+3
    assume {:print "$at(3,5401,5404)"} true;
    assume {:print "$track_exp_sub(25005):", $t0} true;

    // assume Identical($t9, exists[@14]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:20+28
    assume ($t9 == $ResourceExists($1_object_ObjectCore_$memory#14, $self#$1_object_TransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:20+28]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:20+28
    assume {:print "$track_exp_sub(25006):", $t9} true;

    // assume Identical($t10, Not(exists[@14]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    assume ($t10 == !$ResourceExists($1_object_ObjectCore_$memory#14, $self#$1_object_TransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    assume {:print "$track_exp(25007):", $t10} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:145:9+40
    assume {:print "$track_global_mem(27307):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@14]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:144:5+104
    assume {:print "$at(3,5313,5417)"} true;
    assert {:msg "assert_failed(3,5313,5417): abort not covered by any of the `aborts_if` clauses"}
      !$ResourceExists($1_object_ObjectCore_$memory#14, $self#$1_object_TransferRef($t0));

    // abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:144:5+104
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::generate_delete_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:261:5+183
procedure {:timeLimit 40} $1_object_generate_delete_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_DeleteRef)
{
    // declare local variables
    var $t1: bool;
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: $1_object_DeleteRef;
    var $t7: bool;
    var $t8: bool;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_DeleteRef': $1_object_DeleteRef;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:261:5+1
    assume {:print "$at(2,11491,11492)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:261:5+1
    assume {:print "$track_local(52,20,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.can_delete($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:17+14
    assume {:print "$at(2,11573,11587)"} true;
    $t1 := $can_delete#$1_object_ConstructorRef($t0);

    // if ($t1) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:9+65
    if ($t1) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:9+65
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:9+65
    assume {:print "$at(2,11565,11630)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:9+65
L0:

    // $t2 := 5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:58+14
    assume {:print "$at(2,11614,11628)"} true;
    $t2 := 5;
    assume $IsValid'u64'($t2);

    // $t3 := error::permission_denied($t2) on_abort goto L4 with $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:33+40
    call $t3 := $1_error_permission_denied($t2);
    if ($abort_flag) {
        assume {:print "$at(2,11589,11629)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(52,20):", $t4} $t4 == $t4;
        goto L4;
    }

    // trace_abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:9+65
    assume {:print "$at(2,11565,11630)"} true;
    assume {:print "$track_abort(52,20):", $t3} $t3 == $t3;

    // $t4 := move($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:9+65
    $t4 := $t3;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:262:9+65
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:263:27+3
    assume {:print "$at(2,11658,11661)"} true;
L2:

    // $t5 := get_field<object::ConstructorRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:263:27+8
    assume {:print "$at(2,11658,11666)"} true;
    $t5 := $self#$1_object_ConstructorRef($t0);

    // $t6 := pack object::DeleteRef($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:263:9+28
    $t6 := $1_object_DeleteRef($t5);

    // trace_return[0]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:263:9+28
    assume {:print "$track_return(52,20,0):", $t6} $t6 == $t6;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:264:5+1
    assume {:print "$at(2,11673,11674)"} true;
L3:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:20+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:20+3
    assume {:print "$at(3,4103,4106)"} true;
    assume {:print "$track_exp_sub(25121):", $t0} true;

    // assume Identical($t7, Not(select object::ConstructorRef.can_delete($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26
    assume ($t7 == !$can_delete#$1_object_ConstructorRef($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26
    assume {:print "$track_exp(25122):", $t7} true;

    // assert Not(Not(select object::ConstructorRef.can_delete($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26
    assert {:msg "assert_failed(3,4092,4118): function does not abort under this condition"}
      !!$can_delete#$1_object_ConstructorRef($t0);

    // return $t6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26
    $ret0 := $t6;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:264:5+1
    assume {:print "$at(2,11673,11674)"} true;
L4:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:20+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:20+3
    assume {:print "$at(3,4103,4106)"} true;
    assume {:print "$track_exp_sub(25121):", $t0} true;

    // assume Identical($t8, Not(select object::ConstructorRef.can_delete($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26
    assume ($t8 == !$can_delete#$1_object_ConstructorRef($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:107:9+26
    assume {:print "$track_exp(25122):", $t8} true;

    // assert Not(select object::ConstructorRef.can_delete($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:106:5+100
    assume {:print "$at(3,4024,4124)"} true;
    assert {:msg "assert_failed(3,4024,4124): abort not covered by any of the `aborts_if` clauses"}
      !$can_delete#$1_object_ConstructorRef($t0);

    // abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:106:5+100
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun object::generate_derive_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:277:5+108
procedure {:timeLimit 40} $1_object_generate_derive_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_DeriveRef)
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_DeriveRef;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_DeriveRef': $1_object_DeriveRef;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:277:5+1
    assume {:print "$at(2,12208,12209)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:277:5+1
    assume {:print "$track_local(52,21,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:278:27+8
    assume {:print "$at(2,12300,12308)"} true;
    $t1 := $self#$1_object_ConstructorRef($t0);

    // $t2 := pack object::DeriveRef($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:278:9+28
    $t2 := $1_object_DeriveRef($t1);

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:278:9+28
    assume {:print "$track_return(52,21,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:279:5+1
    assume {:print "$at(2,12315,12316)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:279:5+1
    assume {:print "$at(2,12315,12316)"} true;
    assert {:msg "assert_failed(2,12315,12316): function does not abort under this condition"}
      !false;

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:279:5+1
    $ret0 := $t2;
    return;

}

// fun object::generate_extend_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:267:5+108
procedure {:timeLimit 40} $1_object_generate_extend_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_ExtendRef)
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_ExtendRef;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:267:5+1
    assume {:print "$at(2,11778,11779)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:267:5+1
    assume {:print "$track_local(52,22,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:268:27+8
    assume {:print "$at(2,11870,11878)"} true;
    $t1 := $self#$1_object_ConstructorRef($t0);

    // $t2 := pack object::ExtendRef($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:268:9+28
    $t2 := $1_object_ExtendRef($t1);

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:268:9+28
    assume {:print "$track_return(52,22,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:269:5+1
    assume {:print "$at(2,11885,11886)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:269:5+1
    assume {:print "$at(2,11885,11886)"} true;
    assert {:msg "assert_failed(2,11885,11886): function does not abort under this condition"}
      !false;

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:269:5+1
    $ret0 := $t2;
    return;

}

// fun object::generate_linear_transfer_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:369:5+257
procedure {:timeLimit 40} $1_object_generate_linear_transfer_ref$verify(_$t0: $1_object_TransferRef) returns ($ret0: $1_object_LinearTransferRef)
{
    // declare local variables
    var $t1: int;
    var $t2: int;
    var $t3: $1_object_Object'$1_object_ObjectCore';
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $1_object_LinearTransferRef;
    var $t8: bool;
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t0: $1_object_TransferRef;
    var $temp_0'$1_object_LinearTransferRef': $1_object_LinearTransferRef;
    var $temp_0'$1_object_TransferRef': $1_object_TransferRef;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#34: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:369:5+1
    assume {:print "$at(2,15370,15371)"} true;
    assume $IsValid'$1_object_TransferRef'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:369:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @34 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:369:5+1
    $1_object_ObjectCore_$memory#34 := $1_object_ObjectCore_$memory;

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:369:5+1
    assume {:print "$track_local(52,23,0):", $t0} $t0 == $t0;

    // $t2 := get_field<object::TransferRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:370:55+8
    assume {:print "$at(2,15524,15532)"} true;
    $t2 := $self#$1_object_TransferRef($t0);

    // $t3 := pack object::Object<object::ObjectCore>($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:370:27+38
    $t3 := $1_object_Object'$1_object_ObjectCore'($t2);

    // $t4 := object::owner<object::ObjectCore>($t3) on_abort goto L2 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:370:21+45
    call $t4 := $1_object_owner'$1_object_ObjectCore'($t3);
    if ($abort_flag) {
        assume {:print "$at(2,15490,15535)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,23):", $t5} $t5 == $t5;
        goto L2;
    }

    // trace_local[owner]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:370:13+5
    assume {:print "$track_local(52,23,1):", $t4} $t4 == $t4;

    // $t6 := get_field<object::TransferRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:372:19+8
    assume {:print "$at(2,15583,15591)"} true;
    $t6 := $self#$1_object_TransferRef($t0);

    // $t7 := pack object::LinearTransferRef($t6, $t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:371:9+76
    assume {:print "$at(2,15545,15621)"} true;
    $t7 := $1_object_LinearTransferRef($t6, $t4);

    // trace_return[0]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:371:9+76
    assume {:print "$track_return(52,23,0):", $t7} $t7 == $t7;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:375:5+1
    assume {:print "$at(2,15626,15627)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:39+3
    assume {:print "$at(3,5534,5537)"} true;
    assume {:print "$track_exp_sub(25703):", $t0} true;

    // assume Identical($t8, exists[@34]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:20+28
    assume ($t8 == $ResourceExists($1_object_ObjectCore_$memory#34, $self#$1_object_TransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:20+28]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:20+28
    assume {:print "$track_exp_sub(25704):", $t8} true;

    // assume Identical($t9, Not(exists[@34]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    assume ($t9 == !$ResourceExists($1_object_ObjectCore_$memory#34, $self#$1_object_TransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    assume {:print "$track_exp(25705):", $t9} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    assume {:print "$track_global_mem(27308):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@34]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    assert {:msg "assert_failed(3,5504,5544): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#34, $self#$1_object_TransferRef($t0));

    // return $t7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    $ret0 := $t7;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:375:5+1
    assume {:print "$at(2,15626,15627)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:39+3
    assume {:print "$at(3,5534,5537)"} true;
    assume {:print "$track_exp_sub(25703):", $t0} true;

    // assume Identical($t10, exists[@34]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:20+28
    assume ($t10 == $ResourceExists($1_object_ObjectCore_$memory#34, $self#$1_object_TransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:20+28]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:20+28
    assume {:print "$track_exp_sub(25704):", $t10} true;

    // assume Identical($t11, Not(exists[@34]<object::ObjectCore>(select object::TransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    assume ($t11 == !$ResourceExists($1_object_ObjectCore_$memory#34, $self#$1_object_TransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    assume {:print "$track_exp(25705):", $t11} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:149:9+40
    assume {:print "$track_global_mem(27309):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@34]<object::ObjectCore>(select object::TransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:148:5+127
    assume {:print "$at(3,5423,5550)"} true;
    assert {:msg "assert_failed(3,5423,5550): abort not covered by any of the `aborts_if` clauses"}
      !$ResourceExists($1_object_ObjectCore_$memory#34, $self#$1_object_TransferRef($t0));

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:148:5+127
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::generate_signer [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:282:5+96
procedure {:timeLimit 40} $1_object_generate_signer$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $signer)
{
    // declare local variables
    var $t1: int;
    var $t2: $signer;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:282:5+1
    assume {:print "$at(2,12369,12370)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:282:5+1
    assume {:print "$track_local(52,24,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:283:23+8
    assume {:print "$at(2,12450,12458)"} true;
    $t1 := $self#$1_object_ConstructorRef($t0);

    // $t2 := opaque begin: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:283:9+23

    // assume WellFormed($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:283:9+23
    assume $IsValid'signer'($t2) && $1_signer_is_txn_signer($t2) && $1_signer_is_txn_signer_addr($addr#$signer($t2));

    // assume Eq<address>(signer::$address_of($t2), $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:283:9+23
    assume $IsEqual'address'($1_signer_$address_of($t2), $t1);

    // $t2 := opaque end: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:283:9+23

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:283:9+23
    assume {:print "$track_return(52,24,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:284:5+1
    assume {:print "$at(2,12464,12465)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:284:5+1
    assume {:print "$at(2,12464,12465)"} true;
    assert {:msg "assert_failed(2,12464,12465): function does not abort under this condition"}
      !false;

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:284:5+1
    $ret0 := $t2;
    return;

}

// fun object::generate_signer_for_extending [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+105
procedure {:timeLimit 40} $1_object_generate_signer_for_extending$verify(_$t0: $1_object_ExtendRef) returns ($ret0: $signer)
{
    // declare local variables
    var $t1: int;
    var $t2: $signer;
    var $t0: $1_object_ExtendRef;
    var $temp_0'$1_object_ExtendRef': $1_object_ExtendRef;
    var $temp_0'signer': $signer;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$at(2,14413,14414)"} true;
    assume $IsValid'$1_object_ExtendRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:344:5+1
    assume {:print "$track_local(52,25,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ExtendRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:345:23+8
    assume {:print "$at(2,14503,14511)"} true;
    $t1 := $self#$1_object_ExtendRef($t0);

    // $t2 := opaque begin: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:345:9+23

    // assume WellFormed($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:345:9+23
    assume $IsValid'signer'($t2) && $1_signer_is_txn_signer($t2) && $1_signer_is_txn_signer_addr($addr#$signer($t2));

    // assume Eq<address>(signer::$address_of($t2), $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:345:9+23
    assume $IsEqual'address'($1_signer_$address_of($t2), $t1);

    // $t2 := opaque end: create_signer::create_signer($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:345:9+23

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:345:9+23
    assume {:print "$track_return(52,25,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:346:5+1
    assume {:print "$at(2,14517,14518)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:346:5+1
    assume {:print "$at(2,14517,14518)"} true;
    assert {:msg "assert_failed(2,14517,14518): function does not abort under this condition"}
      !false;

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:346:5+1
    $ret0 := $t2;
    return;

}

// fun object::generate_transfer_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:272:5+114
procedure {:timeLimit 40} $1_object_generate_transfer_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_TransferRef)
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_TransferRef;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_TransferRef': $1_object_TransferRef;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:272:5+1
    assume {:print "$at(2,11973,11974)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:272:5+1
    assume {:print "$track_local(52,26,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:273:29+8
    assume {:print "$at(2,12071,12079)"} true;
    $t1 := $self#$1_object_ConstructorRef($t0);

    // $t2 := pack object::TransferRef($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:273:9+30
    $t2 := $1_object_TransferRef($t1);

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:273:9+30
    assume {:print "$track_return(52,26,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:274:5+1
    assume {:print "$at(2,12086,12087)"} true;
L1:

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:274:5+1
    assume {:print "$at(2,12086,12087)"} true;
    assert {:msg "assert_failed(2,12086,12087): function does not abort under this condition"}
      !false;

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:274:5+1
    $ret0 := $t2;
    return;

}

// fun object::is_object [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:158:5+86
procedure {:timeLimit 40} $1_object_is_object$verify(_$t0: int) returns ($ret0: bool)
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
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:158:5+1
    assume {:print "$at(2,6989,6990)"} true;
    assume $IsValid'address'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:158:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:158:5+1
    assume {:print "$track_local(52,27,0):", $t0} $t0 == $t0;

    // $t1 := exists<object::ObjectCore>($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:159:9+6
    assume {:print "$at(2,7043,7049)"} true;
    $t1 := $ResourceExists($1_object_ObjectCore_$memory, $t0);

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:159:9+26
    assume {:print "$track_return(52,27,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:160:5+1
    assume {:print "$at(2,7074,7075)"} true;
L1:

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:160:5+1
    assume {:print "$at(2,7074,7075)"} true;
    assume {:print "$track_global_mem(27310):", $1_object_ObjectCore_$memory} true;

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:160:5+1
    assert {:msg "assert_failed(2,7074,7075): function does not abort under this condition"}
      !false;

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:160:5+1
    $ret0 := $t1;
    return;

}

// fun object::is_owner [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:516:5+127
procedure {:timeLimit 40} $1_object_is_owner$verify(_$t0: $1_object_Object'#0', _$t1: int) returns ($ret0: bool)
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: bool;
    var $t5: bool;
    var $t6: bool;
    var $t7: bool;
    var $t8: bool;
    var $t0: $1_object_Object'#0';
    var $t1: int;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#35: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:516:5+1
    assume {:print "$at(2,20463,20464)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:516:5+1
    assume $IsValid'address'($t1);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:516:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @35 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:516:5+1
    $1_object_ObjectCore_$memory#35 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:516:5+1
    assume {:print "$track_local(52,28,0):", $t0} $t0 == $t0;

    // trace_local[owner]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:516:5+1
    assume {:print "$track_local(52,28,1):", $t1} $t1 == $t1;

    // $t2 := object::owner<#0>($t0) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:517:9+13
    assume {:print "$at(2,20562,20575)"} true;
    call $t2 := $1_object_owner'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,20562,20575)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,28):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t4 := ==($t2, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:517:23+2
    $t4 := $IsEqual'address'($t2, $t1);

    // trace_return[0]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:517:9+22
    assume {:print "$track_return(52,28,0):", $t4} $t4 == $t4;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:518:5+1
    assume {:print "$at(2,20589,20590)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:39+6
    assume {:print "$at(3,8961,8967)"} true;
    assume {:print "$track_exp_sub(25721):", $t0} true;

    // assume Identical($t5, exists[@35]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:20+32
    assume ($t5 == $ResourceExists($1_object_ObjectCore_$memory#35, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:20+32]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:20+32
    assume {:print "$track_exp_sub(25722):", $t5} true;

    // assume Identical($t6, Not(exists[@35]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    assume ($t6 == !$ResourceExists($1_object_ObjectCore_$memory#35, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    assume {:print "$track_exp(25723):", $t6} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    assume {:print "$track_global_mem(27311):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@35]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    assert {:msg "assert_failed(3,8931,8975): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#35, $inner#$1_object_Object'#0'($t0));

    // return $t4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    $ret0 := $t4;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:518:5+1
    assume {:print "$at(2,20589,20590)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:39+6
    assume {:print "$at(3,8961,8967)"} true;
    assume {:print "$track_exp_sub(25721):", $t0} true;

    // assume Identical($t7, exists[@35]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:20+32
    assume ($t7 == $ResourceExists($1_object_ObjectCore_$memory#35, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:20+32]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:20+32
    assume {:print "$track_exp_sub(25722):", $t7} true;

    // assume Identical($t8, Not(exists[@35]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    assume ($t8 == !$ResourceExists($1_object_ObjectCore_$memory#35, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    assume {:print "$track_exp(25723):", $t8} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:230:9+44
    assume {:print "$track_global_mem(27312):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@35]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:229:5+122
    assume {:print "$at(3,8859,8981)"} true;
    assert {:msg "assert_failed(3,8859,8981): abort not covered by any of the `aborts_if` clauses"}
      !$ResourceExists($1_object_ObjectCore_$memory#35, $inner#$1_object_Object'#0'($t0));

    // abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:229:5+122
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::object_address<#0> [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:189:5+91
procedure {:inline 1} $1_object_object_address'#0'(_$t0: $1_object_Object'#0') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:189:5+1
    assume {:print "$at(2,8397,8398)"} true;
    assume {:print "$track_local(52,30,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:190:9+12
    assume {:print "$at(2,8470,8482)"} true;
    $t1 := $inner#$1_object_Object'#0'($t0);

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:190:9+12
    assume {:print "$track_return(52,30,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:191:5+1
    assume {:print "$at(2,8487,8488)"} true;
L1:

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:191:5+1
    assume {:print "$at(2,8487,8488)"} true;
    $ret0 := $t1;
    return;

}

// fun object::object_address [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:189:5+91
procedure {:timeLimit 40} $1_object_object_address$verify(_$t0: $1_object_Object'#0') returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: bool;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:189:5+1
    assume {:print "$at(2,8397,8398)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:189:5+1
    assume {:print "$track_local(52,30,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:190:9+12
    assume {:print "$at(2,8470,8482)"} true;
    $t1 := $inner#$1_object_Object'#0'($t0);

    // trace_return[0]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:190:9+12
    assume {:print "$track_return(52,30,0):", $t1} $t1 == $t1;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:191:5+1
    assume {:print "$at(2,8487,8488)"} true;
L1:

    // assume Identical($t2, false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:41:9+16
    assume {:print "$at(3,1391,1407)"} true;
    assume ($t2 == false);

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:41:9+16]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:41:9+16
    assume {:print "$track_exp(24989):", $t2} true;

    // assert Not(false) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:41:9+16
    assert {:msg "assert_failed(3,1391,1407): function does not abort under this condition"}
      !false;

    // return $t1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:41:9+16
    $ret0 := $t1;
    return;

}

// fun object::object_from_constructor_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:292:5+126
procedure {:timeLimit 40} $1_object_object_from_constructor_ref$verify(_$t0: $1_object_ConstructorRef) returns ($ret0: $1_object_Object'#0')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'#0';
    var $t3: int;
    var $t4: bool;
    var $t5: bool;
    var $t6: bool;
    var $t7: bool;
    var $t8: bool;
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t0: $1_object_ConstructorRef;
    var $temp_0'$1_object_ConstructorRef': $1_object_ConstructorRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#33: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:292:5+1
    assume {:print "$at(2,12690,12691)"} true;
    assume $IsValid'$1_object_ConstructorRef'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:292:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @33 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:292:5+1
    $1_object_ObjectCore_$memory#33 := $1_object_ObjectCore_$memory;

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:292:5+1
    assume {:print "$track_local(52,31,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::ConstructorRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:293:30+8
    assume {:print "$at(2,12801,12809)"} true;
    $t1 := $self#$1_object_ConstructorRef($t0);

    // $t2 := object::address_to_object<#0>($t1) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:293:9+30
    call $t2 := $1_object_address_to_object'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,12780,12810)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,31):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:293:9+30
    assume {:print "$track_return(52,31,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:294:5+1
    assume {:print "$at(2,12815,12816)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:39+3
    assume {:print "$at(3,4355,4358)"} true;
    assume {:print "$track_exp_sub(25668):", $t0} true;

    // assume Identical($t4, exists[@33]<object::ObjectCore>(select object::ConstructorRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:20+28
    assume ($t4 == $ResourceExists($1_object_ObjectCore_$memory#33, $self#$1_object_ConstructorRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:20+28]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:20+28
    assume {:print "$track_exp_sub(25669):", $t4} true;

    // assume Identical($t5, Not(exists[@33]<object::ObjectCore>(select object::ConstructorRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40
    assume ($t5 == !$ResourceExists($1_object_ObjectCore_$memory#33, $self#$1_object_ConstructorRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40
    assume {:print "$track_exp(25670):", $t5} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40
    assume {:print "$track_global_mem(27313):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@33]<object::ObjectCore>(select object::ConstructorRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40
    assert {:msg "assert_failed(3,4325,4365): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#33, $self#$1_object_ConstructorRef($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:38+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:38+3
    assume {:print "$at(3,4403,4406)"} true;
    assume {:print "$track_exp_sub(25675):", $t0} true;

    // assume Identical($t6, object::spec_exists_at[]<#0>(select object::ConstructorRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:20+27
    assume ($t6 == $1_object_spec_exists_at'#0'($self#$1_object_ConstructorRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:20+27]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:20+27
    assume {:print "$track_exp_sub(25676):", $t6} true;

    // assume Identical($t7, Not(object::spec_exists_at[]<#0>(select object::ConstructorRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    assume ($t7 == !$1_object_spec_exists_at'#0'($self#$1_object_ConstructorRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    assume {:print "$track_exp(25677):", $t7} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    assume {:print "$track_global_mem(27314):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(object::spec_exists_at[]<#0>(select object::ConstructorRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    assert {:msg "assert_failed(3,4374,4413): function does not abort under this condition"}
      !!$1_object_spec_exists_at'#0'($self#$1_object_ConstructorRef($t0));

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    $ret0 := $t2;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:294:5+1
    assume {:print "$at(2,12815,12816)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:39+3
    assume {:print "$at(3,4355,4358)"} true;
    assume {:print "$track_exp_sub(25668):", $t0} true;

    // assume Identical($t8, exists[@33]<object::ObjectCore>(select object::ConstructorRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:20+28
    assume ($t8 == $ResourceExists($1_object_ObjectCore_$memory#33, $self#$1_object_ConstructorRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:20+28]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:20+28
    assume {:print "$track_exp_sub(25669):", $t8} true;

    // assume Identical($t9, Not(exists[@33]<object::ObjectCore>(select object::ConstructorRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40
    assume ($t9 == !$ResourceExists($1_object_ObjectCore_$memory#33, $self#$1_object_ConstructorRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:115:9+40
    assume {:print "$track_exp(25670):", $t9} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:38+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:38+3
    assume {:print "$at(3,4403,4406)"} true;
    assume {:print "$track_exp_sub(25675):", $t0} true;

    // assume Identical($t10, object::spec_exists_at[]<#0>(select object::ConstructorRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:20+27
    assume ($t10 == $1_object_spec_exists_at'#0'($self#$1_object_ConstructorRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:20+27]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:20+27
    assume {:print "$track_exp_sub(25676):", $t10} true;

    // assume Identical($t11, Not(object::spec_exists_at[]<#0>(select object::ConstructorRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    assume ($t11 == !$1_object_spec_exists_at'#0'($self#$1_object_ConstructorRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    assume {:print "$track_exp(25677):", $t11} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:116:9+39
    assume {:print "$track_global_mem(27315):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@33]<object::ObjectCore>(select object::ConstructorRef.self($t0))), Not(object::spec_exists_at[]<#0>(select object::ConstructorRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:114:5+178
    assume {:print "$at(3,4241,4419)"} true;
    assert {:msg "assert_failed(3,4241,4419): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#33, $self#$1_object_ConstructorRef($t0)) || !$1_object_spec_exists_at'#0'($self#$1_object_ConstructorRef($t0)));

    // abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:114:5+178
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::object_from_delete_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:325:5+116
procedure {:timeLimit 40} $1_object_object_from_delete_ref$verify(_$t0: $1_object_DeleteRef) returns ($ret0: $1_object_Object'#0')
{
    // declare local variables
    var $t1: int;
    var $t2: $1_object_Object'#0';
    var $t3: int;
    var $t4: bool;
    var $t5: bool;
    var $t6: bool;
    var $t7: bool;
    var $t8: bool;
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t0: $1_object_DeleteRef;
    var $temp_0'$1_object_DeleteRef': $1_object_DeleteRef;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#32: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:325:5+1
    assume {:print "$at(2,13812,13813)"} true;
    assume $IsValid'$1_object_DeleteRef'($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:325:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @32 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:325:5+1
    $1_object_ObjectCore_$memory#32 := $1_object_ObjectCore_$memory;

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:325:5+1
    assume {:print "$track_local(52,32,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::DeleteRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:326:30+8
    assume {:print "$at(2,13913,13921)"} true;
    $t1 := $self#$1_object_DeleteRef($t0);

    // $t2 := object::address_to_object<#0>($t1) on_abort goto L2 with $t3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:326:9+30
    call $t2 := $1_object_address_to_object'#0'($t1);
    if ($abort_flag) {
        assume {:print "$at(2,13892,13922)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(52,32):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:326:9+30
    assume {:print "$track_return(52,32,0):", $t2} $t2 == $t2;

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:327:5+1
    assume {:print "$at(2,13927,13928)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:39+3
    assume {:print "$at(3,5153,5156)"} true;
    assume {:print "$track_exp_sub(25633):", $t0} true;

    // assume Identical($t4, exists[@32]<object::ObjectCore>(select object::DeleteRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:20+28
    assume ($t4 == $ResourceExists($1_object_ObjectCore_$memory#32, $self#$1_object_DeleteRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:20+28]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:20+28
    assume {:print "$track_exp_sub(25634):", $t4} true;

    // assume Identical($t5, Not(exists[@32]<object::ObjectCore>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40
    assume ($t5 == !$ResourceExists($1_object_ObjectCore_$memory#32, $self#$1_object_DeleteRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40
    assume {:print "$track_exp(25635):", $t5} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40
    assume {:print "$track_global_mem(27316):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@32]<object::ObjectCore>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40
    assert {:msg "assert_failed(3,5123,5163): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#32, $self#$1_object_DeleteRef($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:38+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:38+3
    assume {:print "$at(3,5201,5204)"} true;
    assume {:print "$track_exp_sub(25640):", $t0} true;

    // assume Identical($t6, object::spec_exists_at[]<#0>(select object::DeleteRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:20+27
    assume ($t6 == $1_object_spec_exists_at'#0'($self#$1_object_DeleteRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:20+27]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:20+27
    assume {:print "$track_exp_sub(25641):", $t6} true;

    // assume Identical($t7, Not(object::spec_exists_at[]<#0>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    assume ($t7 == !$1_object_spec_exists_at'#0'($self#$1_object_DeleteRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    assume {:print "$track_exp(25642):", $t7} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    assume {:print "$track_global_mem(27317):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(object::spec_exists_at[]<#0>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    assert {:msg "assert_failed(3,5172,5211): function does not abort under this condition"}
      !!$1_object_spec_exists_at'#0'($self#$1_object_DeleteRef($t0));

    // return $t2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    $ret0 := $t2;
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:327:5+1
    assume {:print "$at(2,13927,13928)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:39+3
    assume {:print "$at(3,5153,5156)"} true;
    assume {:print "$track_exp_sub(25633):", $t0} true;

    // assume Identical($t8, exists[@32]<object::ObjectCore>(select object::DeleteRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:20+28
    assume ($t8 == $ResourceExists($1_object_ObjectCore_$memory#32, $self#$1_object_DeleteRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:20+28]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:20+28
    assume {:print "$track_exp_sub(25634):", $t8} true;

    // assume Identical($t9, Not(exists[@32]<object::ObjectCore>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40
    assume ($t9 == !$ResourceExists($1_object_ObjectCore_$memory#32, $self#$1_object_DeleteRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:136:9+40
    assume {:print "$track_exp(25635):", $t9} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:38+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:38+3
    assume {:print "$at(3,5201,5204)"} true;
    assume {:print "$track_exp_sub(25640):", $t0} true;

    // assume Identical($t10, object::spec_exists_at[]<#0>(select object::DeleteRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:20+27
    assume ($t10 == $1_object_spec_exists_at'#0'($self#$1_object_DeleteRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:20+27]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:20+27
    assume {:print "$track_exp_sub(25641):", $t10} true;

    // assume Identical($t11, Not(object::spec_exists_at[]<#0>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    assume ($t11 == !$1_object_spec_exists_at'#0'($self#$1_object_DeleteRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    assume {:print "$track_exp(25642):", $t11} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:137:9+39
    assume {:print "$track_global_mem(27318):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@32]<object::ObjectCore>(select object::DeleteRef.self($t0))), Not(object::spec_exists_at[]<#0>(select object::DeleteRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:135:5+168
    assume {:print "$at(3,5049,5217)"} true;
    assert {:msg "assert_failed(3,5049,5217): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#32, $self#$1_object_DeleteRef($t0)) || !$1_object_spec_exists_at'#0'($self#$1_object_DeleteRef($t0)));

    // abort($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:135:5+168
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun object::owns [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:521:5+896
procedure {:timeLimit 40} $1_object_owns$verify(_$t0: $1_object_Object'#0', _$t1: int) returns ($ret0: bool)
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: bool;
    var $t7: bool;
    var $t8: bool;
    var $t9: bool;
    var $t10: int;
    var $t11: int;
    var $t12: $1_object_ObjectCore;
    var $t13: int;
    var $t14: bool;
    var $t15: bool;
    var $t16: bool;
    var $t17: int;
    var $t18: bool;
    var $t19: int;
    var $t20: bool;
    var $t21: bool;
    var $t22: bool;
    var $t23: bool;
    var $t24: bool;
    var $t25: bool;
    var $t26: bool;
    var $t27: $1_object_ObjectCore;
    var $t28: bool;
    var $t29: int;
    var $t30: bool;
    var $t31: bool;
    var $t32: int;
    var $t33: int;
    var $t34: bool;
    var $t35: bool;
    var $t36: bool;
    var $t37: bool;
    var $t38: bool;
    var $t39: bool;
    var $t0: $1_object_Object'#0';
    var $t1: int;
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'u8': int;
    var $1_object_ObjectCore_$memory#23: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:521:5+1
    assume {:print "$at(2,20697,20698)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:521:5+1
    assume $IsValid'address'($t1);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:521:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @23 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:521:5+1
    $1_object_ObjectCore_$memory#23 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:521:5+1
    assume {:print "$track_local(52,34,0):", $t0} $t0 == $t0;

    // trace_local[owner]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:521:5+1
    assume {:print "$track_local(52,34,1):", $t1} $t1 == $t1;

    // $t4 := object::object_address<#0>($t0) on_abort goto L16 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:522:31+23
    assume {:print "$at(2,20814,20837)"} true;
    call $t4 := $1_object_object_address'#0'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,20814,20837)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,34):", $t5} $t5 == $t5;
        goto L16;
    }

    // trace_local[current_address]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:522:13+15
    assume {:print "$track_local(52,34,2):", $t4} $t4 == $t4;

    // $t6 := ==($t4, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:523:29+2
    assume {:print "$at(2,20867,20869)"} true;
    $t6 := $IsEqual'address'($t4, $t1);

    // if ($t6) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:523:9+65
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:524:20+4
    assume {:print "$at(2,20898,20902)"} true;
L1:

    // $t7 := true at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:524:20+4
    assume {:print "$at(2,20898,20902)"} true;
    $t7 := true;
    assume $IsValid'bool'($t7);

    // trace_return[0]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:524:13+11
    assume {:print "$track_return(52,34,0):", $t7} $t7 == $t7;

    // $t8 := move($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:524:13+11
    $t8 := $t7;

    // goto L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:524:13+11
    goto L15;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:528:32+15
    assume {:print "$at(2,20963,20978)"} true;
L0:

    // $t9 := exists<object::ObjectCore>($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:528:13+6
    assume {:print "$at(2,20944,20950)"} true;
    $t9 := $ResourceExists($1_object_ObjectCore_$memory, $t4);

    // if ($t9) goto L3 else goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:527:9+121
    assume {:print "$at(2,20923,21044)"} true;
    if ($t9) { goto L3; } else { goto L2; }

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:527:9+121
L3:

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:527:9+121
    assume {:print "$at(2,20923,21044)"} true;
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:529:30+22
    assume {:print "$at(2,21010,21032)"} true;
L2:

    // $t10 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:529:30+22
    assume {:print "$at(2,21010,21032)"} true;
    $t10 := 2;
    assume $IsValid'u64'($t10);

    // $t11 := error::not_found($t10) on_abort goto L16 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:529:13+40
    call $t11 := $1_error_not_found($t10);
    if ($abort_flag) {
        assume {:print "$at(2,20993,21033)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,34):", $t5} $t5 == $t5;
        goto L16;
    }

    // trace_abort($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:527:9+121
    assume {:print "$at(2,20923,21044)"} true;
    assume {:print "$track_abort(52,34):", $t11} $t11 == $t11;

    // $t5 := move($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:527:9+121
    $t5 := $t11;

    // goto L16 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:527:9+121
    goto L16;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:532:48+15
    assume {:print "$at(2,21094,21109)"} true;
L4:

    // $t12 := get_global<object::ObjectCore>($t4) on_abort goto L16 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:532:22+13
    assume {:print "$at(2,21068,21081)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t4)) {
        call $ExecFailureAbort();
    } else {
        $t12 := $ResourceValue($1_object_ObjectCore_$memory, $t4);
    }
    if ($abort_flag) {
        assume {:print "$at(2,21068,21081)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,34):", $t5} $t5 == $t5;
        goto L16;
    }

    // $t13 := get_field<object::ObjectCore>.owner($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:533:31+12
    assume {:print "$at(2,21142,21154)"} true;
    $t13 := $owner#$1_object_ObjectCore($t12);

    // trace_local[current_address#2]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:533:13+15
    assume {:print "$track_local(52,34,3):", $t13} $t13 == $t13;

    // label L13 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$at(2,21195,21200)"} true;
L13:

    // $t3 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$at(2,21195,21200)"} true;
    havoc $t3;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25264):", $t3} true;

    // assume Identical($t14, WellFormed($t3)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t14 == $IsValid'address'($t3));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25265):", $t14} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27319):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'address'($t3);

    // $t15 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t15;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25268):", $t15} true;

    // assume Identical($t16, WellFormed($t15)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t16 == $IsValid'bool'($t15));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25269):", $t16} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27320):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'bool'($t15);

    // $t17 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t17;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25272):", $t17} true;

    // assume Identical($t18, WellFormed($t17)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t18 == $IsValid'u8'($t17));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25273):", $t18} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27321):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'u8'($t17);

    // $t19 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t19;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25276):", $t19} true;

    // assume Identical($t20, WellFormed($t19)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t20 == $IsValid'u8'($t19));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25277):", $t20} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27322):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'u8'($t19);

    // $t21 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t21;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25280):", $t21} true;

    // assume Identical($t22, WellFormed($t21)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t22 == $IsValid'bool'($t21));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25281):", $t22} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27323):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'bool'($t21);

    // $t23 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t23;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25284):", $t23} true;

    // assume Identical($t24, WellFormed($t23)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t24 == $IsValid'bool'($t23));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25285):", $t24} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27324):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'bool'($t23);

    // $t25 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t25;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25288):", $t25} true;

    // assume Identical($t26, WellFormed($t25)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t26 == $IsValid'bool'($t25));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25289):", $t26} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27325):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'bool'($t25);

    // $t27 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t27;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25292):", $t27} true;

    // assume Identical($t28, WellFormed($t27)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t28 == $IsValid'$1_object_ObjectCore'($t27));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25293):", $t28} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27326):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'$1_object_ObjectCore'($t27);

    // $t29 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    havoc $t29;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t29) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp_sub(25296):", $t29} true;

    // assume Identical($t30, WellFormed($t29)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t30 == $IsValid'address'($t29));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t30) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25297):", $t30} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27327):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t29) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume $IsValid'address'($t29);

    // trace_local[current_address#2]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$info(): enter loop, variable(s) current_address#2 havocked and reassigned"} true;
    assume {:print "$track_local(52,34,3):", $t3} $t3 == $t3;

    // assume Identical($t31, Not(AbortFlag())) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume ($t31 == !$abort_flag);

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5]($t31) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_exp(25299):", $t31} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume {:print "$track_global_mem(27328):", $1_object_ObjectCore_$memory} true;

    // assume Not(AbortFlag()) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:16+5
    assume !$abort_flag;

    // $t15 := !=($t1, $t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:22+2
    $t15 := !$IsEqual'address'($t1, $t3);

    // if ($t15) goto L6 else goto L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:9+385
    if ($t15) { goto L6; } else { goto L5; }

    // label L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:536:9+385
L6:

    // label L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:537:25+9
    assume {:print "$at(2,21247,21256)"} true;
L7:

    // $t17 := 1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:537:25+9
    assume {:print "$at(2,21247,21256)"} true;
    $t17 := 1;
    assume $IsValid'u8'($t17);

    // $t19 := 8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:29+22
    assume {:print "$at(2,21286,21308)"} true;
    $t19 := 8;
    assume $IsValid'u8'($t19);

    // $t21 := <($t17, $t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:27+1
    call $t21 := $Lt($t17, $t19);

    // if ($t21) goto L9 else goto L8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:13+78
    if ($t21) { goto L9; } else { goto L8; }

    // label L9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:13+78
L9:

    // goto L10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:13+78
    assume {:print "$at(2,21270,21348)"} true;
    goto L10;

    // label L8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:73+16
L8:

    // $t32 := 6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:73+16
    assume {:print "$at(2,21330,21346)"} true;
    $t32 := 6;
    assume $IsValid'u64'($t32);

    // $t33 := error::out_of_range($t32) on_abort goto L16 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:53+37
    call $t33 := $1_error_out_of_range($t32);
    if ($abort_flag) {
        assume {:print "$at(2,21310,21347)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,34):", $t5} $t5 == $t5;
        goto L16;
    }

    // trace_abort($t33) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:13+78
    assume {:print "$at(2,21270,21348)"} true;
    assume {:print "$track_abort(52,34):", $t33} $t33 == $t33;

    // $t5 := move($t33) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:13+78
    $t5 := $t33;

    // goto L16 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:538:13+78
    goto L16;

    // label L10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:539:37+15
    assume {:print "$at(2,21386,21401)"} true;
L10:

    // $t23 := exists<object::ObjectCore>($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:539:18+6
    assume {:print "$at(2,21367,21373)"} true;
    $t23 := $ResourceExists($1_object_ObjectCore_$memory, $t3);

    // $t25 := !($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:539:17+1
    call $t25 := $Not($t23);

    // if ($t25) goto L12 else goto L11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:539:13+86
    if ($t25) { goto L12; } else { goto L11; }

    // label L12 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:540:24+5
    assume {:print "$at(2,21429,21434)"} true;
L12:

    // $t34 := false at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:540:24+5
    assume {:print "$at(2,21429,21434)"} true;
    $t34 := false;
    assume $IsValid'bool'($t34);

    // trace_return[0]($t34) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:540:17+12
    assume {:print "$track_return(52,34,0):", $t34} $t34 == $t34;

    // $t8 := move($t34) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:540:17+12
    $t8 := $t34;

    // goto L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:540:17+12
    goto L15;

    // label L11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:543:52+15
    assume {:print "$at(2,21502,21517)"} true;
L11:

    // $t27 := get_global<object::ObjectCore>($t3) on_abort goto L16 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:543:26+13
    assume {:print "$at(2,21476,21489)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t3)) {
        call $ExecFailureAbort();
    } else {
        $t27 := $ResourceValue($1_object_ObjectCore_$memory, $t3);
    }
    if ($abort_flag) {
        assume {:print "$at(2,21476,21489)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,34):", $t5} $t5 == $t5;
        goto L16;
    }

    // $t29 := get_field<object::ObjectCore>.owner($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:544:31+12
    assume {:print "$at(2,21550,21562)"} true;
    $t29 := $owner#$1_object_ObjectCore($t27);

    // trace_local[current_address#2]($t29) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:544:13+15
    assume {:print "$track_local(52,34,3):", $t29} $t29 == $t29;

    // goto L14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:544:43+1
    goto L14;

    // label L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:546:9+4
    assume {:print "$at(2,21583,21587)"} true;
L5:

    // $t35 := true at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:546:9+4
    assume {:print "$at(2,21583,21587)"} true;
    $t35 := true;
    assume $IsValid'bool'($t35);

    // trace_return[0]($t35) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:546:9+4
    assume {:print "$track_return(52,34,0):", $t35} $t35 == $t35;

    // $t8 := move($t35) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:546:9+4
    $t8 := $t35;

    // goto L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:546:9+4
    goto L15;

    // label L14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:546:9+4
    // Loop invariant checking block for the loop started with header: L13
L14:

    // stop() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:546:9+4
    assume {:print "$at(2,21583,21587)"} true;
    assume false;
    return;

    // label L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:547:5+1
    assume {:print "$at(2,21592,21593)"} true;
L15:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:19+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:19+6
    assume {:print "$at(3,9178,9184)"} true;
    assume {:print "$track_exp_sub(25255):", $t0} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:35+5]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:35+5
    assume {:print "$track_exp_sub(25257):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:64+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:64+6
    assume {:print "$track_exp_sub(25259):", $t0} true;

    // assume Identical($t36, exists[@23]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:45+32
    assume ($t36 == $ResourceExists($1_object_ObjectCore_$memory#23, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:45+32]($t36) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:45+32
    assume {:print "$track_exp_sub(25260):", $t36} true;

    // assume Identical($t37, And(Neq<address>(select object::Object.inner($t0), $t1), Not(exists[@23]<object::ObjectCore>(select object::Object.inner($t0))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    assume ($t37 == (!$IsEqual'address'($inner#$1_object_Object'#0'($t0), $t1) && !$ResourceExists($1_object_ObjectCore_$memory#23, $inner#$1_object_Object'#0'($t0))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69]($t37) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    assume {:print "$track_exp(25261):", $t37} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    assume {:print "$track_global_mem(27329):", $1_object_ObjectCore_$memory} true;

    // assert Not(And(Neq<address>(select object::Object.inner($t0), $t1), Not(exists[@23]<object::ObjectCore>(select object::Object.inner($t0))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    assert {:msg "assert_failed(3,9168,9237): function does not abort under this condition"}
      !(!$IsEqual'address'($inner#$1_object_Object'#0'($t0), $t1) && !$ResourceExists($1_object_ObjectCore_$memory#23, $inner#$1_object_Object'#0'($t0)));

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    $ret0 := $t8;
    return;

    // label L16 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:547:5+1
    assume {:print "$at(2,21592,21593)"} true;
L16:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:19+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:19+6
    assume {:print "$at(3,9178,9184)"} true;
    assume {:print "$track_exp_sub(25255):", $t0} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:35+5]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:35+5
    assume {:print "$track_exp_sub(25257):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:64+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:64+6
    assume {:print "$track_exp_sub(25259):", $t0} true;

    // assume Identical($t38, exists[@23]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:45+32
    assume ($t38 == $ResourceExists($1_object_ObjectCore_$memory#23, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:45+32]($t38) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:45+32
    assume {:print "$track_exp_sub(25260):", $t38} true;

    // assume Identical($t39, And(Neq<address>(select object::Object.inner($t0), $t1), Not(exists[@23]<object::ObjectCore>(select object::Object.inner($t0))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    assume ($t39 == (!$IsEqual'address'($inner#$1_object_Object'#0'($t0), $t1) && !$ResourceExists($1_object_ObjectCore_$memory#23, $inner#$1_object_Object'#0'($t0))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69]($t39) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    assume {:print "$track_exp(25261):", $t39} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:238:9+69
    assume {:print "$track_global_mem(27330):", $1_object_ObjectCore_$memory} true;

    // assert And(Neq<address>(select object::Object.inner($t0), $t1), Not(exists[@23]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:237:5+144
    assume {:print "$at(3,9099,9243)"} true;
    assert {:msg "assert_failed(3,9099,9243): abort not covered by any of the `aborts_if` clauses"}
      (!$IsEqual'address'($inner#$1_object_Object'#0'($t0), $t1) && !$ResourceExists($1_object_ObjectCore_$memory#23, $inner#$1_object_Object'#0'($t0)));

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:237:5+144
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::transfer_call [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+175
procedure {:timeLimit 40} $1_object_transfer_call$verify(_$t0: $signer, _$t1: int, _$t2: int) returns ()
{
    // declare local variables
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: bool;
    var $t9: bool;
    var $t10: $1_object_ObjectCore;
    var $t11: bool;
    var $t12: bool;
    var $t13: bool;
    var $t14: $1_object_ObjectCore;
    var $t15: bool;
    var $t16: bool;
    var $t17: bool;
    var $t18: $1_object_ObjectCore;
    var $t19: bool;
    var $t20: bool;
    var $t21: bool;
    var $t22: $1_object_ObjectCore;
    var $t23: bool;
    var $t0: $signer;
    var $t1: int;
    var $t2: int;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $1_object_ObjectCore_$memory#43: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume {:print "$at(2,16320,16321)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume $IsValid'address'($t1);

    // assume WellFormed($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume $IsValid'address'($t2);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:48+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:48+5
    assume {:print "$at(3,5918,5923)"} true;
    assume {:print "$track_exp_sub(26118):", $t0} true;

    // assume Identical($t3, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:29+25
    assume ($t3 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:29+25]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:29+25
    assume {:print "$track_exp_sub(26119):", $t3} true;

    // assume Identical($t4, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:9+46
    assume ($t4 == $1_signer_$address_of($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:9+46]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:9+46
    assume {:print "$track_exp(26120):", $t4} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:9+46
    assume {:print "$track_global_mem(27331):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:163:9+46
    assume ($t5 == $1_signer_$address_of($t0));

    // @43 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume {:print "$at(2,16320,16321)"} true;
    $1_object_ObjectCore_$memory#43 := $1_object_ObjectCore_$memory;

    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume {:print "$track_local(52,36,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume {:print "$track_local(52,36,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:396:5+1
    assume {:print "$track_local(52,36,2):", $t2} $t2 == $t2;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:401:9+31
    assume {:print "$at(2,16458,16489)"} true;
    assume {:print "$track_global_mem(27332):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t6, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume {:print "$at(3,7098,7144)"} true;
    assume ($t6 == $1_signer_$address_of($t0));

    // object::transfer_raw($t0, $t1, $t2) on_abort goto L2 with $t7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:401:9+31
    assume {:print "$at(2,16458,16489)"} true;
    call $1_object_transfer_raw($t0, $t1, $t2);
    if ($abort_flag) {
        assume {:print "$at(2,16458,16489)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(52,36):", $t7} $t7 == $t7;
        goto L2;
    }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:402:5+1
    assume {:print "$at(2,16494,16495)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:39+6
    assume {:print "$at(3,5964,5970)"} true;
    assume {:print "$track_exp_sub(26125):", $t1} true;

    // assume Identical($t8, exists[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:20+26
    assume ($t8 == $ResourceExists($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:20+26]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:20+26
    assume {:print "$track_exp_sub(26126):", $t8} true;

    // assume Identical($t9, Not(exists[@43]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38
    assume ($t9 == !$ResourceExists($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38
    assume {:print "$track_exp(26127):", $t9} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38
    assume {:print "$track_global_mem(27333):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@43]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38
    assert {:msg "assert_failed(3,5934,5972): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#43, $t1);

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:39+6
    assume {:print "$at(3,6011,6017)"} true;
    assume {:print "$track_exp_sub(26132):", $t1} true;

    // assume Identical($t10, global[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:20+26
    assume ($t10 == $ResourceValue($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:20+26]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:20+26
    assume {:print "$track_exp_sub(26133):", $t10} true;

    // assume Identical($t11, Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61
    assume ($t11 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61
    assume {:print "$track_exp(26134):", $t11} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61
    assume {:print "$track_global_mem(27334):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61
    assert {:msg "assert_failed(3,5981,6042): function does not abort under this condition"}
      !!$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:13+13
    assume {:print "$at(3,6116,6129)"} true;
    assume {:print "$track_exp_sub(26141):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:30+6
    assume {:print "$track_exp_sub(26143):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:60+6
    assume {:print "$track_exp_sub(26145):", $t1} true;

    // assume Identical($t12, exists[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:41+26
    assume ($t12 == $ResourceExists($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:41+26]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:41+26
    assume {:print "$track_exp_sub(26146):", $t12} true;

    // assume Identical($t13, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(exists[@43]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120
    assume {:print "$at(3,6051,6171)"} true;
    assume ($t13 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#43, $t1)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120
    assume {:print "$track_exp(26147):", $t13} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120
    assume {:print "$track_global_mem(27335):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(exists[@43]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120
    assert {:msg "assert_failed(3,6051,6171): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#43, $t1))))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:13+13
    assume {:print "$at(3,6245,6258)"} true;
    assume {:print "$track_exp_sub(26154):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:30+6
    assume {:print "$track_exp_sub(26156):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:60+6
    assume {:print "$track_exp_sub(26158):", $t1} true;

    // assume Identical($t14, global[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:41+26
    assume ($t14 == $ResourceValue($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:41+26]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:41+26
    assume {:print "$track_exp_sub(26159):", $t14} true;

    // assume Identical($t15, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    assume {:print "$at(3,6180,6323)"} true;
    assume ($t15 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    assume {:print "$track_exp(26160):", $t15} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    assume {:print "$track_global_mem(27336):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    assert {:msg "assert_failed(3,6180,6323): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1)))))));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:402:5+1
    assume {:print "$at(2,16494,16495)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:39+6
    assume {:print "$at(3,5964,5970)"} true;
    assume {:print "$track_exp_sub(26125):", $t1} true;

    // assume Identical($t16, exists[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:20+26
    assume ($t16 == $ResourceExists($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:20+26]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:20+26
    assume {:print "$track_exp_sub(26126):", $t16} true;

    // assume Identical($t17, Not(exists[@43]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38
    assume ($t17 == !$ResourceExists($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:164:9+38
    assume {:print "$track_exp(26127):", $t17} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:39+6
    assume {:print "$at(3,6011,6017)"} true;
    assume {:print "$track_exp_sub(26132):", $t1} true;

    // assume Identical($t18, global[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:20+26
    assume ($t18 == $ResourceValue($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:20+26]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:20+26
    assume {:print "$track_exp_sub(26133):", $t18} true;

    // assume Identical($t19, Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61
    assume ($t19 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:165:9+61
    assume {:print "$track_exp(26134):", $t19} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:13+13
    assume {:print "$at(3,6116,6129)"} true;
    assume {:print "$track_exp_sub(26141):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:30+6
    assume {:print "$track_exp_sub(26143):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:60+6
    assume {:print "$track_exp_sub(26145):", $t1} true;

    // assume Identical($t20, exists[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:41+26
    assume ($t20 == $ResourceExists($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:41+26]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:167:41+26
    assume {:print "$track_exp_sub(26146):", $t20} true;

    // assume Identical($t21, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(exists[@43]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120
    assume {:print "$at(3,6051,6171)"} true;
    assume ($t21 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#43, $t1)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:166:9+120
    assume {:print "$track_exp(26147):", $t21} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:13+13
    assume {:print "$at(3,6245,6258)"} true;
    assume {:print "$track_exp_sub(26154):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:30+6
    assume {:print "$track_exp_sub(26156):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:60+6
    assume {:print "$track_exp_sub(26158):", $t1} true;

    // assume Identical($t22, global[@43]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:41+26
    assume ($t22 == $ResourceValue($1_object_ObjectCore_$memory#43, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:41+26]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:169:41+26
    assume {:print "$track_exp_sub(26159):", $t22} true;

    // assume Identical($t23, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    assume {:print "$at(3,6180,6323)"} true;
    assume ($t23 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    assume {:print "$track_exp(26160):", $t23} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:168:9+143
    assume {:print "$track_global_mem(27337):", $1_object_ObjectCore_$memory} true;

    // assert Or(Or(Or(Not(exists[@43]<object::ObjectCore>($t1)), Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(exists[@43]<object::ObjectCore>($t1)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@43]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:158:5+556
    assume {:print "$at(3,5773,6329)"} true;
    assert {:msg "assert_failed(3,5773,6329): abort not covered by any of the `aborts_if` clauses"}
      (((!$ResourceExists($1_object_ObjectCore_$memory#43, $t1) || !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1))) || (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#43, $t1))))))) || (var $range_2 := $Range(0, (8 - 1)); (exists $i_3: int :: $InRange($range_2, $i_3) && (var i := $i_3;
    ((!$IsEqual'address'($t5, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#43, $t1))))))));

    // abort($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:158:5+556
    $abort_code := $t7;
    $abort_flag := true;
    return;

}

// fun object::transfer_raw [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+633
procedure {:inline 1} $1_object_transfer_raw(_$t0: $signer, _$t1: int, _$t2: int) returns ()
{
    // declare local variables
    var $t3: $Mutation ($1_object_ObjectCore);
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $Mutation ($1_object_ObjectCore);
    var $t8: int;
    var $t9: bool;
    var $t10: $Mutation ($1_event_EventHandle'$1_object_TransferEvent');
    var $t11: int;
    var $t12: $1_object_TransferEvent;
    var $t13: $Mutation (int);
    var $t0: $signer;
    var $t1: int;
    var $t2: int;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'signer': $signer;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // assume Identical($t4, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume {:print "$at(3,7098,7144)"} true;
    assume ($t4 == $1_signer_$address_of($t0));

    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$at(2,17129,17130)"} true;
    assume {:print "$track_local(52,37,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$track_local(52,37,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$track_local(52,37,2):", $t2} $t2 == $t2;

    // $t5 := signer::address_of($t0) on_abort goto L3 with $t6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:423:29+25
    assume {:print "$at(2,17280,17305)"} true;
    call $t5 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,17280,17305)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(52,37):", $t6} $t6 == $t6;
        goto L3;
    }

    // object::verify_ungated_and_descendant($t5, $t1) on_abort goto L3 with $t6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:424:9+52
    assume {:print "$at(2,17315,17367)"} true;
    call $1_object_verify_ungated_and_descendant($t5, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,17315,17367)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(52,37):", $t6} $t6 == $t6;
        goto L3;
    }

    // $t7 := borrow_global<object::ObjectCore>($t1) on_abort goto L3 with $t6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:426:27+17
    assume {:print "$at(2,17396,17413)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t7 := $Mutation($Global($t1), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t1));
    }
    if ($abort_flag) {
        assume {:print "$at(2,17396,17413)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(52,37):", $t6} $t6 == $t6;
        goto L3;
    }

    // trace_local[object_core]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:426:13+11
    $temp_0'$1_object_ObjectCore' := $Dereference($t7);
    assume {:print "$track_local(52,37,3):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t8 := get_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:427:13+17
    assume {:print "$at(2,17447,17464)"} true;
    $t8 := $owner#$1_object_ObjectCore($Dereference($t7));

    // $t9 := ==($t8, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:427:31+2
    $t9 := $IsEqual'address'($t8, $t2);

    // if ($t9) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:427:9+59
    if ($t9) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:428:13+6
    assume {:print "$at(2,17486,17492)"} true;
L1:

    // destroy($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:428:13+6
    assume {:print "$at(2,17486,17492)"} true;

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:428:13+6
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:432:18+11
    assume {:print "$at(2,17549,17560)"} true;
L0:

    // $t10 := borrow_field<object::ObjectCore>.transfer_events($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:432:13+32
    assume {:print "$at(2,17544,17576)"} true;
    $t10 := $ChildMutation($t7, 3, $transfer_events#$1_object_ObjectCore($Dereference($t7)));

    // $t11 := get_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:435:23+17
    assume {:print "$at(2,17660,17677)"} true;
    $t11 := $owner#$1_object_ObjectCore($Dereference($t7));

    // $t12 := pack object::TransferEvent($t1, $t11, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:433:13+122
    assume {:print "$at(2,17590,17712)"} true;
    $t12 := $1_object_TransferEvent($t1, $t11, $t2);

    // opaque begin: event::emit_event<object::TransferEvent>($t10, $t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:431:9+210
    assume {:print "$at(2,17513,17723)"} true;

    // opaque end: event::emit_event<object::TransferEvent>($t10, $t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:431:9+210

    // write_back[Reference($t7).transfer_events (event::EventHandle<object::TransferEvent>)]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:431:9+210
    $t7 := $UpdateMutation($t7, $Update'$1_object_ObjectCore'_transfer_events($Dereference($t7), $Dereference($t10)));

    // $t13 := borrow_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+17
    assume {:print "$at(2,17733,17750)"} true;
    $t13 := $ChildMutation($t7, 1, $owner#$1_object_ObjectCore($Dereference($t7)));

    // write_ref($t13, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+22
    $t13 := $UpdateMutation($t13, $t2);

    // write_back[Reference($t7).owner (address)]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+22
    $t7 := $UpdateMutation($t7, $Update'$1_object_ObjectCore'_owner($Dereference($t7), $Dereference($t13)));

    // write_back[object::ObjectCore@]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+22
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t7),
        $Dereference($t7));

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:440:5+1
    assume {:print "$at(2,17761,17762)"} true;
L2:

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:440:5+1
    assume {:print "$at(2,17761,17762)"} true;
    return;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:440:5+1
L3:

    // abort($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:440:5+1
    assume {:print "$at(2,17761,17762)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun object::transfer_raw [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+633
procedure {:timeLimit 40} $1_object_transfer_raw$verify(_$t0: $signer, _$t1: int, _$t2: int) returns ()
{
    // declare local variables
    var $t3: $Mutation ($1_object_ObjectCore);
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: $Mutation ($1_object_ObjectCore);
    var $t10: int;
    var $t11: bool;
    var $t12: $Mutation ($1_event_EventHandle'$1_object_TransferEvent');
    var $t13: int;
    var $t14: $1_object_TransferEvent;
    var $t15: $Mutation (int);
    var $t16: bool;
    var $t17: bool;
    var $t18: $1_object_ObjectCore;
    var $t19: bool;
    var $t20: bool;
    var $t21: bool;
    var $t22: $1_object_ObjectCore;
    var $t23: bool;
    var $t24: bool;
    var $t25: bool;
    var $t26: $1_object_ObjectCore;
    var $t27: bool;
    var $t28: bool;
    var $t29: bool;
    var $t30: $1_object_ObjectCore;
    var $t31: bool;
    var $t0: $signer;
    var $t1: int;
    var $t2: int;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $1_object_ObjectCore_$memory#36: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$at(2,17129,17130)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume $IsValid'address'($t1);

    // assume WellFormed($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume $IsValid'address'($t2);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:48+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:48+5
    assume {:print "$at(3,7137,7142)"} true;
    assume {:print "$track_exp_sub(25736):", $t0} true;

    // assume Identical($t4, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:29+25
    assume ($t4 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:29+25]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:29+25
    assume {:print "$track_exp_sub(25737):", $t4} true;

    // assume Identical($t5, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume ($t5 == $1_signer_$address_of($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume {:print "$track_exp(25738):", $t5} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume {:print "$track_global_mem(27338):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t6, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:192:9+46
    assume ($t6 == $1_signer_$address_of($t0));

    // @36 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$at(2,17129,17130)"} true;
    $1_object_ObjectCore_$memory#36 := $1_object_ObjectCore_$memory;

    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$track_local(52,37,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$track_local(52,37,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:418:5+1
    assume {:print "$track_local(52,37,2):", $t2} $t2 == $t2;

    // $t7 := signer::address_of($t0) on_abort goto L3 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:423:29+25
    assume {:print "$at(2,17280,17305)"} true;
    call $t7 := $1_signer_address_of($t0);
    if ($abort_flag) {
        assume {:print "$at(2,17280,17305)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,37):", $t8} $t8 == $t8;
        goto L3;
    }

    // object::verify_ungated_and_descendant($t7, $t1) on_abort goto L3 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:424:9+52
    assume {:print "$at(2,17315,17367)"} true;
    call $1_object_verify_ungated_and_descendant($t7, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,17315,17367)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,37):", $t8} $t8 == $t8;
        goto L3;
    }

    // $t9 := borrow_global<object::ObjectCore>($t1) on_abort goto L3 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:426:27+17
    assume {:print "$at(2,17396,17413)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t9 := $Mutation($Global($t1), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t1));
    }
    if ($abort_flag) {
        assume {:print "$at(2,17396,17413)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,37):", $t8} $t8 == $t8;
        goto L3;
    }

    // trace_local[object_core]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:426:13+11
    $temp_0'$1_object_ObjectCore' := $Dereference($t9);
    assume {:print "$track_local(52,37,3):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t10 := get_field<object::ObjectCore>.owner($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:427:13+17
    assume {:print "$at(2,17447,17464)"} true;
    $t10 := $owner#$1_object_ObjectCore($Dereference($t9));

    // $t11 := ==($t10, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:427:31+2
    $t11 := $IsEqual'address'($t10, $t2);

    // if ($t11) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:427:9+59
    if ($t11) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:428:13+6
    assume {:print "$at(2,17486,17492)"} true;
L1:

    // destroy($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:428:13+6
    assume {:print "$at(2,17486,17492)"} true;

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:428:13+6
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:432:18+11
    assume {:print "$at(2,17549,17560)"} true;
L0:

    // $t12 := borrow_field<object::ObjectCore>.transfer_events($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:432:13+32
    assume {:print "$at(2,17544,17576)"} true;
    $t12 := $ChildMutation($t9, 3, $transfer_events#$1_object_ObjectCore($Dereference($t9)));

    // $t13 := get_field<object::ObjectCore>.owner($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:435:23+17
    assume {:print "$at(2,17660,17677)"} true;
    $t13 := $owner#$1_object_ObjectCore($Dereference($t9));

    // $t14 := pack object::TransferEvent($t1, $t13, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:433:13+122
    assume {:print "$at(2,17590,17712)"} true;
    $t14 := $1_object_TransferEvent($t1, $t13, $t2);

    // opaque begin: event::emit_event<object::TransferEvent>($t12, $t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:431:9+210
    assume {:print "$at(2,17513,17723)"} true;

    // opaque end: event::emit_event<object::TransferEvent>($t12, $t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:431:9+210

    // write_back[Reference($t9).transfer_events (event::EventHandle<object::TransferEvent>)]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:431:9+210
    $t9 := $UpdateMutation($t9, $Update'$1_object_ObjectCore'_transfer_events($Dereference($t9), $Dereference($t12)));

    // $t15 := borrow_field<object::ObjectCore>.owner($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+17
    assume {:print "$at(2,17733,17750)"} true;
    $t15 := $ChildMutation($t9, 1, $owner#$1_object_ObjectCore($Dereference($t9)));

    // write_ref($t15, $t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+22
    $t15 := $UpdateMutation($t15, $t2);

    // write_back[Reference($t9).owner (address)]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+22
    $t9 := $UpdateMutation($t9, $Update'$1_object_ObjectCore'_owner($Dereference($t9), $Dereference($t15)));

    // write_back[object::ObjectCore@]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:439:9+22
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t9),
        $Dereference($t9));

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:440:5+1
    assume {:print "$at(2,17761,17762)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:39+6
    assume {:print "$at(3,7183,7189)"} true;
    assume {:print "$track_exp_sub(25743):", $t1} true;

    // assume Identical($t16, exists[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:20+26
    assume ($t16 == $ResourceExists($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:20+26]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:20+26
    assume {:print "$track_exp_sub(25744):", $t16} true;

    // assume Identical($t17, Not(exists[@36]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38
    assume ($t17 == !$ResourceExists($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38
    assume {:print "$track_exp(25745):", $t17} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38
    assume {:print "$track_global_mem(27339):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@36]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38
    assert {:msg "assert_failed(3,7153,7191): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#36, $t1);

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:39+6
    assume {:print "$at(3,7230,7236)"} true;
    assume {:print "$track_exp_sub(25750):", $t1} true;

    // assume Identical($t18, global[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:20+26
    assume ($t18 == $ResourceValue($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:20+26]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:20+26
    assume {:print "$track_exp_sub(25751):", $t18} true;

    // assume Identical($t19, Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61
    assume ($t19 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61
    assume {:print "$track_exp(25752):", $t19} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61
    assume {:print "$track_global_mem(27340):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61
    assert {:msg "assert_failed(3,7200,7261): function does not abort under this condition"}
      !!$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:13+13]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:13+13
    assume {:print "$at(3,7335,7348)"} true;
    assume {:print "$track_exp_sub(25759):", $t6} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:30+6
    assume {:print "$track_exp_sub(25761):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:60+6
    assume {:print "$track_exp_sub(25763):", $t1} true;

    // assume Identical($t20, exists[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:41+26
    assume ($t20 == $ResourceExists($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:41+26]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:41+26
    assume {:print "$track_exp_sub(25764):", $t20} true;

    // assume Identical($t21, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(exists[@36]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120
    assume {:print "$at(3,7270,7390)"} true;
    assume ($t21 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t6, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#36, $t1)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120
    assume {:print "$track_exp(25765):", $t21} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120
    assume {:print "$track_global_mem(27341):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(exists[@36]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120
    assert {:msg "assert_failed(3,7270,7390): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t6, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#36, $t1))))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:13+13]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:13+13
    assume {:print "$at(3,7464,7477)"} true;
    assume {:print "$track_exp_sub(25772):", $t6} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:30+6
    assume {:print "$track_exp_sub(25774):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:60+6
    assume {:print "$track_exp_sub(25776):", $t1} true;

    // assume Identical($t22, global[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:41+26
    assume ($t22 == $ResourceValue($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:41+26]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:41+26
    assume {:print "$track_exp_sub(25777):", $t22} true;

    // assume Identical($t23, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    assume {:print "$at(3,7399,7542)"} true;
    assume ($t23 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t6, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    assume {:print "$track_exp(25778):", $t23} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    assume {:print "$track_global_mem(27342):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    assert {:msg "assert_failed(3,7399,7542): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t6, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1)))))));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    return;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:440:5+1
    assume {:print "$at(2,17761,17762)"} true;
L3:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:39+6
    assume {:print "$at(3,7183,7189)"} true;
    assume {:print "$track_exp_sub(25743):", $t1} true;

    // assume Identical($t24, exists[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:20+26
    assume ($t24 == $ResourceExists($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:20+26]($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:20+26
    assume {:print "$track_exp_sub(25744):", $t24} true;

    // assume Identical($t25, Not(exists[@36]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38
    assume ($t25 == !$ResourceExists($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38]($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:193:9+38
    assume {:print "$track_exp(25745):", $t25} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:39+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:39+6
    assume {:print "$at(3,7230,7236)"} true;
    assume {:print "$track_exp_sub(25750):", $t1} true;

    // assume Identical($t26, global[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:20+26
    assume ($t26 == $ResourceValue($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:20+26]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:20+26
    assume {:print "$track_exp_sub(25751):", $t26} true;

    // assume Identical($t27, Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61
    assume ($t27 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61]($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:194:9+61
    assume {:print "$track_exp(25752):", $t27} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:13+13]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:13+13
    assume {:print "$at(3,7335,7348)"} true;
    assume {:print "$track_exp_sub(25759):", $t6} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:30+6
    assume {:print "$track_exp_sub(25761):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:60+6
    assume {:print "$track_exp_sub(25763):", $t1} true;

    // assume Identical($t28, exists[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:41+26
    assume ($t28 == $ResourceExists($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:41+26]($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:196:41+26
    assume {:print "$track_exp_sub(25764):", $t28} true;

    // assume Identical($t29, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(exists[@36]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120
    assume {:print "$at(3,7270,7390)"} true;
    assume ($t29 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t6, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#36, $t1)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120]($t29) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:195:9+120
    assume {:print "$track_exp(25765):", $t29} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:13+13]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:13+13
    assume {:print "$at(3,7464,7477)"} true;
    assume {:print "$track_exp_sub(25772):", $t6} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:30+6
    assume {:print "$track_exp_sub(25774):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:60+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:60+6
    assume {:print "$track_exp_sub(25776):", $t1} true;

    // assume Identical($t30, global[@36]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:41+26
    assume ($t30 == $ResourceValue($1_object_ObjectCore_$memory#36, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:41+26]($t30) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:198:41+26
    assume {:print "$track_exp_sub(25777):", $t30} true;

    // assume Identical($t31, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    assume {:print "$at(3,7399,7542)"} true;
    assume ($t31 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t6, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143]($t31) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    assume {:print "$track_exp(25778):", $t31} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:197:9+143
    assume {:print "$track_global_mem(27343):", $1_object_ObjectCore_$memory} true;

    // assert Or(Or(Or(Not(exists[@36]<object::ObjectCore>($t1)), Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(exists[@36]<object::ObjectCore>($t1)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t6, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@36]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:187:5+555
    assume {:print "$at(3,6993,7548)"} true;
    assert {:msg "assert_failed(3,6993,7548): abort not covered by any of the `aborts_if` clauses"}
      (((!$ResourceExists($1_object_ObjectCore_$memory#36, $t1) || !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1))) || (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t6, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#36, $t1))))))) || (var $range_2 := $Range(0, (8 - 1)); (exists $i_3: int :: $InRange($range_2, $i_3) && (var i := $i_3;
    ((!$IsEqual'address'($t6, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#36, $t1))))))));

    // abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:187:5+555
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun object::transfer_to_object [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+202
procedure {:timeLimit 40} $1_object_transfer_to_object$verify(_$t0: $signer, _$t1: $1_object_Object'#0', _$t2: $1_object_Object'#1') returns ()
{
    // declare local variables
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: int;
    var $t12: bool;
    var $t13: bool;
    var $t14: $1_object_ObjectCore;
    var $t15: bool;
    var $t16: bool;
    var $t17: bool;
    var $t18: $1_object_ObjectCore;
    var $t19: bool;
    var $t20: bool;
    var $t21: bool;
    var $t22: $1_object_ObjectCore;
    var $t23: bool;
    var $t24: bool;
    var $t25: bool;
    var $t26: $1_object_ObjectCore;
    var $t27: bool;
    var $t0: $signer;
    var $t1: $1_object_Object'#0';
    var $t2: $1_object_Object'#1';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'$1_object_Object'#1'': $1_object_Object'#1';
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'signer': $signer;
    var $1_object_ObjectCore_$memory#48: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume {:print "$at(2,17858,17859)"} true;
    assume $IsValid'signer'($t0) && $1_signer_is_txn_signer($t0) && $1_signer_is_txn_signer_addr($addr#$signer($t0));

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume $IsValid'$1_object_Object'#0''($t1);

    // assume WellFormed($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume $IsValid'$1_object_Object'#1''($t2);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:48+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:48+5
    assume {:print "$at(3,7724,7729)"} true;
    assume {:print "$track_exp_sub(26480):", $t0} true;

    // assume Identical($t3, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:29+25
    assume ($t3 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:29+25]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:29+25
    assume {:print "$track_exp_sub(26481):", $t3} true;

    // assume Identical($t4, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:9+46
    assume ($t4 == $1_signer_$address_of($t0));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:9+46]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:9+46
    assume {:print "$track_exp(26482):", $t4} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:9+46
    assume {:print "$track_global_mem(27344):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:206:9+46
    assume ($t5 == $1_signer_$address_of($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:207:30+6]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:207:30+6
    assume {:print "$at(3,7761,7767)"} true;
    assume {:print "$track_exp_sub(26485):", $t1} true;

    // assume Identical($t6, select object::Object.inner($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:207:9+34
    assume ($t6 == $inner#$1_object_Object'#0'($t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:207:9+34]($t6) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:207:9+34
    assume {:print "$track_exp(26486):", $t6} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:207:9+34
    assume {:print "$track_global_mem(27345):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t7, select object::Object.inner($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:207:9+34
    assume ($t7 == $inner#$1_object_Object'#0'($t1));

    // @48 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume {:print "$at(2,17858,17859)"} true;
    $1_object_ObjectCore_$memory#48 := $1_object_ObjectCore_$memory;

    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume {:print "$track_local(52,38,0):", $t0} $t0 == $t0;

    // trace_local[object]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume {:print "$track_local(52,38,1):", $t1} $t1 == $t1;

    // trace_local[to]($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:443:5+1
    assume {:print "$track_local(52,38,2):", $t2} $t2 == $t2;

    // $t8 := get_field<object::Object<#1>>.inner($t2) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:448:33+8
    assume {:print "$at(2,18045,18053)"} true;
    $t8 := $inner#$1_object_Object'#1'($t2);

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:448:9+33
    assume {:print "$track_global_mem(27346):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t9, signer::$address_of($t0)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46
    assume {:print "$at(3,6446,6492)"} true;
    assume ($t9 == $1_signer_$address_of($t0));

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:177:9+46
    assume {:print "$track_global_mem(27347):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t10, select object::Object.inner($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:178:9+34
    assume {:print "$at(3,6501,6535)"} true;
    assume ($t10 == $inner#$1_object_Object'#0'($t1));

    // object::transfer<#0>($t0, $t1, $t8) on_abort goto L2 with $t11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:448:9+33
    assume {:print "$at(2,18021,18054)"} true;
    call $1_object_transfer'#0'($t0, $t1, $t8);
    if ($abort_flag) {
        assume {:print "$at(2,18021,18054)"} true;
        $t11 := $abort_code;
        assume {:print "$track_abort(52,38):", $t11} $t11 == $t11;
        goto L2;
    }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:449:5+1
    assume {:print "$at(2,18059,18060)"} true;
L1:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:39+14
    assume {:print "$at(3,7813,7827)"} true;
    assume {:print "$track_exp_sub(26491):", $t7} true;

    // assume Identical($t12, exists[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:20+34
    assume ($t12 == $ResourceExists($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:20+34]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:20+34
    assume {:print "$track_exp_sub(26492):", $t12} true;

    // assume Identical($t13, Not(exists[@48]<object::ObjectCore>($t7))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46
    assume ($t13 == !$ResourceExists($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46]($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46
    assume {:print "$track_exp(26493):", $t13} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46
    assume {:print "$track_global_mem(27348):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@48]<object::ObjectCore>($t7))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46
    assert {:msg "assert_failed(3,7783,7829): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#48, $t7);

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:39+14
    assume {:print "$at(3,7868,7882)"} true;
    assume {:print "$track_exp_sub(26498):", $t7} true;

    // assume Identical($t14, global[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:20+34
    assume ($t14 == $ResourceValue($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:20+34]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:20+34
    assume {:print "$track_exp_sub(26499):", $t14} true;

    // assume Identical($t15, Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69
    assume ($t15 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69
    assume {:print "$track_exp(26500):", $t15} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69
    assume {:print "$track_global_mem(27349):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69
    assert {:msg "assert_failed(3,7838,7907): function does not abort under this condition"}
      !!$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:13+13
    assume {:print "$at(3,7981,7994)"} true;
    assume {:print "$track_exp_sub(26507):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:30+14
    assume {:print "$track_exp_sub(26509):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:68+14
    assume {:print "$track_exp_sub(26511):", $t7} true;

    // assume Identical($t16, exists[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:49+34
    assume ($t16 == $ResourceExists($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:49+34]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:49+34
    assume {:print "$track_exp_sub(26512):", $t16} true;

    // assume Identical($t17, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@48]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136
    assume {:print "$at(3,7916,8052)"} true;
    assume ($t17 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#48, $t7)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136
    assume {:print "$track_exp(26513):", $t17} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136
    assume {:print "$track_global_mem(27350):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@48]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136
    assert {:msg "assert_failed(3,7916,8052): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#48, $t7))))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:13+13
    assume {:print "$at(3,8126,8139)"} true;
    assume {:print "$track_exp_sub(26520):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:30+14
    assume {:print "$track_exp_sub(26522):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:68+14
    assume {:print "$track_exp_sub(26524):", $t7} true;

    // assume Identical($t18, global[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:49+34
    assume ($t18 == $ResourceValue($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:49+34]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:49+34
    assume {:print "$track_exp_sub(26525):", $t18} true;

    // assume Identical($t19, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    assume {:print "$at(3,8061,8220)"} true;
    assume ($t19 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    assume {:print "$track_exp(26526):", $t19} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    assume {:print "$track_global_mem(27351):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    assert {:msg "assert_failed(3,8061,8220): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7)))))));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    return;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:449:5+1
    assume {:print "$at(2,18059,18060)"} true;
L2:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:39+14
    assume {:print "$at(3,7813,7827)"} true;
    assume {:print "$track_exp_sub(26491):", $t7} true;

    // assume Identical($t20, exists[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:20+34
    assume ($t20 == $ResourceExists($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:20+34]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:20+34
    assume {:print "$track_exp_sub(26492):", $t20} true;

    // assume Identical($t21, Not(exists[@48]<object::ObjectCore>($t7))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46
    assume ($t21 == !$ResourceExists($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:208:9+46
    assume {:print "$track_exp(26493):", $t21} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:39+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:39+14
    assume {:print "$at(3,7868,7882)"} true;
    assume {:print "$track_exp_sub(26498):", $t7} true;

    // assume Identical($t22, global[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:20+34
    assume ($t22 == $ResourceValue($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:20+34]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:20+34
    assume {:print "$track_exp_sub(26499):", $t22} true;

    // assume Identical($t23, Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69
    assume ($t23 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:209:9+69
    assume {:print "$track_exp(26500):", $t23} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:13+13
    assume {:print "$at(3,7981,7994)"} true;
    assume {:print "$track_exp_sub(26507):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:30+14
    assume {:print "$track_exp_sub(26509):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:68+14
    assume {:print "$track_exp_sub(26511):", $t7} true;

    // assume Identical($t24, exists[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:49+34
    assume ($t24 == $ResourceExists($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:49+34]($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:211:49+34
    assume {:print "$track_exp_sub(26512):", $t24} true;

    // assume Identical($t25, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@48]<object::ObjectCore>($t7)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136
    assume {:print "$at(3,7916,8052)"} true;
    assume ($t25 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#48, $t7)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136]($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:210:9+136
    assume {:print "$track_exp(26513):", $t25} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:13+13]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:13+13
    assume {:print "$at(3,8126,8139)"} true;
    assume {:print "$track_exp_sub(26520):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:30+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:30+14
    assume {:print "$track_exp_sub(26522):", $t7} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:68+14]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:68+14
    assume {:print "$track_exp_sub(26524):", $t7} true;

    // assume Identical($t26, global[@48]<object::ObjectCore>($t7)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:49+34
    assume ($t26 == $ResourceValue($1_object_ObjectCore_$memory#48, $t7));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:49+34]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:213:49+34
    assume {:print "$track_exp_sub(26525):", $t26} true;

    // assume Identical($t27, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    assume {:print "$at(3,8061,8220)"} true;
    assume ($t27 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159]($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    assume {:print "$track_exp(26526):", $t27} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:212:9+159
    assume {:print "$track_global_mem(27352):", $1_object_ObjectCore_$memory} true;

    // assert Or(Or(Or(Not(exists[@48]<object::ObjectCore>($t7)), Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(exists[@48]<object::ObjectCore>($t7)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t5, $t7), Not(select object::ObjectCore.allow_ungated_transfer(global[@48]<object::ObjectCore>($t7))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:201:5+672
    assume {:print "$at(3,7554,8226)"} true;
    assert {:msg "assert_failed(3,7554,8226): abort not covered by any of the `aborts_if` clauses"}
      (((!$ResourceExists($1_object_ObjectCore_$memory#48, $t7) || !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7))) || (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t5, $t7) && !$ResourceExists($1_object_ObjectCore_$memory#48, $t7))))))) || (var $range_2 := $Range(0, (8 - 1)); (exists $i_3: int :: $InRange($range_2, $i_3) && (var i := $i_3;
    ((!$IsEqual'address'($t5, $t7) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#48, $t7))))))));

    // abort($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:201:5+672
    $abort_code := $t11;
    $abort_flag := true;
    return;

}

// fun object::transfer_with_ref [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:378:5+518
procedure {:timeLimit 40} $1_object_transfer_with_ref$verify(_$t0: $1_object_LinearTransferRef, _$t1: int) returns ()
{
    // declare local variables
    var $t2: $Mutation ($1_object_ObjectCore);
    var $t3: $1_object_ObjectCore;
    var $t4: $1_object_ObjectCore;
    var $t5: $1_object_ObjectCore;
    var $t6: int;
    var $t7: $Mutation ($1_object_ObjectCore);
    var $t8: int;
    var $t9: int;
    var $t10: int;
    var $t11: bool;
    var $t12: int;
    var $t13: int;
    var $t14: $Mutation ($1_event_EventHandle'$1_object_TransferEvent');
    var $t15: int;
    var $t16: int;
    var $t17: $1_object_TransferEvent;
    var $t18: $Mutation (int);
    var $t19: bool;
    var $t20: bool;
    var $t21: bool;
    var $t22: bool;
    var $t23: bool;
    var $t24: bool;
    var $t0: $1_object_LinearTransferRef;
    var $t1: int;
    var $temp_0'$1_object_LinearTransferRef': $1_object_LinearTransferRef;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#17: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:378:5+1
    assume {:print "$at(2,15704,15705)"} true;
    assume $IsValid'$1_object_LinearTransferRef'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:378:5+1
    assume $IsValid'address'($t1);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:378:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:41+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:41+3
    assume {:print "$at(3,5657,5660)"} true;
    assume {:print "$track_exp_sub(25131):", $t0} true;

    // assume Identical($t3, global<object::ObjectCore>(select object::LinearTransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:22+28
    assume ($t3 == $ResourceValue($1_object_ObjectCore_$memory, $self#$1_object_LinearTransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:22+28]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:22+28
    assume {:print "$track_exp_sub(25132):", $t3} true;

    // assume Identical($t4, global<object::ObjectCore>(select object::LinearTransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:9+42
    assume ($t4 == $ResourceValue($1_object_ObjectCore_$memory, $self#$1_object_LinearTransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:9+42]($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:9+42
    assume {:print "$track_exp(25133):", $t4} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:9+42
    assume {:print "$track_global_mem(27353):", $1_object_ObjectCore_$memory} true;

    // assume Identical($t5, global<object::ObjectCore>(select object::LinearTransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:153:9+42
    assume ($t5 == $ResourceValue($1_object_ObjectCore_$memory, $self#$1_object_LinearTransferRef($t0)));

    // @17 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:378:5+1
    assume {:print "$at(2,15704,15705)"} true;
    $1_object_ObjectCore_$memory#17 := $1_object_ObjectCore_$memory;

    // trace_local[ref]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:378:5+1
    assume {:print "$track_local(52,39,0):", $t0} $t0 == $t0;

    // trace_local[to]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:378:5+1
    assume {:print "$track_local(52,39,1):", $t1} $t1 == $t1;

    // $t6 := get_field<object::LinearTransferRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:379:52+8
    assume {:print "$at(2,15843,15851)"} true;
    $t6 := $self#$1_object_LinearTransferRef($t0);

    // $t7 := borrow_global<object::ObjectCore>($t6) on_abort goto L4 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:379:22+17
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t6)) {
        call $ExecFailureAbort();
    } else {
        $t7 := $Mutation($Global($t6), EmptyVec(), $ResourceValue($1_object_ObjectCore_$memory, $t6));
    }
    if ($abort_flag) {
        assume {:print "$at(2,15813,15830)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,39):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_local[object]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:379:13+6
    $temp_0'$1_object_ObjectCore' := $Dereference($t7);
    assume {:print "$track_local(52,39,2):", $temp_0'$1_object_ObjectCore'} $temp_0'$1_object_ObjectCore' == $temp_0'$1_object_ObjectCore';

    // $t9 := get_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:381:13+12
    assume {:print "$at(2,15883,15895)"} true;
    $t9 := $owner#$1_object_ObjectCore($Dereference($t7));

    // $t10 := get_field<object::LinearTransferRef>.owner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:381:29+9
    $t10 := $owner#$1_object_LinearTransferRef($t0);

    // $t11 := ==($t9, $t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:381:26+2
    $t11 := $IsEqual'address'($t9, $t10);

    // if ($t11) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
    assume {:print "$at(2,15862,15976)"} true;
    if ($t11) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
    assume {:print "$at(2,15862,15976)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
L0:

    // destroy($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
    assume {:print "$at(2,15862,15976)"} true;

    // $t12 := 4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:382:38+17
    assume {:print "$at(2,15947,15964)"} true;
    $t12 := 4;
    assume $IsValid'u64'($t12);

    // $t13 := error::permission_denied($t12) on_abort goto L4 with $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:382:13+43
    call $t13 := $1_error_permission_denied($t12);
    if ($abort_flag) {
        assume {:print "$at(2,15922,15965)"} true;
        $t8 := $abort_code;
        assume {:print "$track_abort(52,39):", $t8} $t8 == $t8;
        goto L4;
    }

    // trace_abort($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
    assume {:print "$at(2,15862,15976)"} true;
    assume {:print "$track_abort(52,39):", $t13} $t13 == $t13;

    // $t8 := move($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
    $t8 := $t13;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:380:9+114
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:385:18+6
    assume {:print "$at(2,16022,16028)"} true;
L2:

    // $t14 := borrow_field<object::ObjectCore>.transfer_events($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:385:13+27
    assume {:print "$at(2,16017,16044)"} true;
    $t14 := $ChildMutation($t7, 3, $transfer_events#$1_object_ObjectCore($Dereference($t7)));

    // $t15 := get_field<object::LinearTransferRef>.self($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:387:25+8
    assume {:print "$at(2,16098,16106)"} true;
    $t15 := $self#$1_object_LinearTransferRef($t0);

    // $t16 := get_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:388:23+12
    assume {:print "$at(2,16130,16142)"} true;
    $t16 := $owner#$1_object_ObjectCore($Dereference($t7));

    // $t17 := pack object::TransferEvent($t15, $t16, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:386:13+119
    assume {:print "$at(2,16058,16177)"} true;
    $t17 := $1_object_TransferEvent($t15, $t16, $t1);

    // opaque begin: event::emit_event<object::TransferEvent>($t14, $t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:384:9+202
    assume {:print "$at(2,15986,16188)"} true;

    // opaque end: event::emit_event<object::TransferEvent>($t14, $t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:384:9+202

    // write_back[Reference($t7).transfer_events (event::EventHandle<object::TransferEvent>)]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:384:9+202
    $t7 := $UpdateMutation($t7, $Update'$1_object_ObjectCore'_transfer_events($Dereference($t7), $Dereference($t14)));

    // $t18 := borrow_field<object::ObjectCore>.owner($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:392:9+12
    assume {:print "$at(2,16198,16210)"} true;
    $t18 := $ChildMutation($t7, 1, $owner#$1_object_ObjectCore($Dereference($t7)));

    // write_ref($t18, $t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:392:9+17
    $t18 := $UpdateMutation($t18, $t1);

    // write_back[Reference($t7).owner (address)]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:392:9+17
    $t7 := $UpdateMutation($t7, $Update'$1_object_ObjectCore'_owner($Dereference($t7), $Dereference($t18)));

    // write_back[object::ObjectCore@]($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:392:9+17
    $1_object_ObjectCore_$memory := $ResourceUpdate($1_object_ObjectCore_$memory, $GlobalLocationAddress($t7),
        $Dereference($t7));

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:393:5+1
    assume {:print "$at(2,16221,16222)"} true;
L3:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:39+3
    assume {:print "$at(3,5706,5709)"} true;
    assume {:print "$track_exp_sub(25138):", $t0} true;

    // assume Identical($t19, exists[@17]<object::ObjectCore>(select object::LinearTransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:20+28
    assume ($t19 == $ResourceExists($1_object_ObjectCore_$memory#17, $self#$1_object_LinearTransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:20+28]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:20+28
    assume {:print "$track_exp_sub(25139):", $t19} true;

    // assume Identical($t20, Not(exists[@17]<object::ObjectCore>(select object::LinearTransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40
    assume ($t20 == !$ResourceExists($1_object_ObjectCore_$memory#17, $self#$1_object_LinearTransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40
    assume {:print "$track_exp(25140):", $t20} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40
    assume {:print "$track_global_mem(27354):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@17]<object::ObjectCore>(select object::LinearTransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40
    assert {:msg "assert_failed(3,5676,5716): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#17, $self#$1_object_LinearTransferRef($t0));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:19+6]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:19+6
    assume {:print "$at(3,5735,5741)"} true;
    assume {:print "$track_exp_sub(25145):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:35+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:35+3
    assume {:print "$track_exp_sub(25147):", $t0} true;

    // assume Identical($t21, Neq<address>(select object::ObjectCore.owner($t5), select object::LinearTransferRef.owner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    assume ($t21 == !$IsEqual'address'($owner#$1_object_ObjectCore($t5), $owner#$1_object_LinearTransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    assume {:print "$track_exp(25148):", $t21} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    assume {:print "$track_global_mem(27355):", $1_object_ObjectCore_$memory} true;

    // assert Not(Neq<address>(select object::ObjectCore.owner($t5), select object::LinearTransferRef.owner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    assert {:msg "assert_failed(3,5725,5761): function does not abort under this condition"}
      !!$IsEqual'address'($owner#$1_object_ObjectCore($t5), $owner#$1_object_LinearTransferRef($t0));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:393:5+1
    assume {:print "$at(2,16221,16222)"} true;
L4:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:39+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:39+3
    assume {:print "$at(3,5706,5709)"} true;
    assume {:print "$track_exp_sub(25138):", $t0} true;

    // assume Identical($t22, exists[@17]<object::ObjectCore>(select object::LinearTransferRef.self($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:20+28
    assume ($t22 == $ResourceExists($1_object_ObjectCore_$memory#17, $self#$1_object_LinearTransferRef($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:20+28]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:20+28
    assume {:print "$track_exp_sub(25139):", $t22} true;

    // assume Identical($t23, Not(exists[@17]<object::ObjectCore>(select object::LinearTransferRef.self($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40
    assume ($t23 == !$ResourceExists($1_object_ObjectCore_$memory#17, $self#$1_object_LinearTransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:154:9+40
    assume {:print "$track_exp(25140):", $t23} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:19+6]($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:19+6
    assume {:print "$at(3,5735,5741)"} true;
    assume {:print "$track_exp_sub(25145):", $t5} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:35+3]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:35+3
    assume {:print "$track_exp_sub(25147):", $t0} true;

    // assume Identical($t24, Neq<address>(select object::ObjectCore.owner($t5), select object::LinearTransferRef.owner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    assume ($t24 == !$IsEqual'address'($owner#$1_object_ObjectCore($t5), $owner#$1_object_LinearTransferRef($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36]($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    assume {:print "$track_exp(25148):", $t24} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:155:9+36
    assume {:print "$track_global_mem(27356):", $1_object_ObjectCore_$memory} true;

    // assert Or(Not(exists[@17]<object::ObjectCore>(select object::LinearTransferRef.self($t0))), Neq<address>(select object::ObjectCore.owner($t5), select object::LinearTransferRef.owner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:152:5+211
    assume {:print "$at(3,5556,5767)"} true;
    assert {:msg "assert_failed(3,5556,5767): abort not covered by any of the `aborts_if` clauses"}
      (!$ResourceExists($1_object_ObjectCore_$memory#17, $self#$1_object_LinearTransferRef($t0)) || !$IsEqual'address'($owner#$1_object_ObjectCore($t5), $owner#$1_object_LinearTransferRef($t0)));

    // abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:152:5+211
    $abort_code := $t8;
    $abort_flag := true;
    return;

}

// fun object::ungated_transfer_allowed [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:498:5+295
procedure {:timeLimit 40} $1_object_ungated_transfer_allowed$verify(_$t0: $1_object_Object'#0') returns ($ret0: bool)
{
    // declare local variables
    var $t1: int;
    var $t2: bool;
    var $t3: int;
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: $1_object_ObjectCore;
    var $t8: bool;
    var $t9: bool;
    var $t10: bool;
    var $t11: bool;
    var $t12: bool;
    var $t0: $1_object_Object'#0';
    var $temp_0'$1_object_Object'#0'': $1_object_Object'#0';
    var $temp_0'bool': bool;
    var $1_object_ObjectCore_$memory#20: $Memory $1_object_ObjectCore;
    $t0 := _$t0;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:498:5+1
    assume {:print "$at(2,19794,19795)"} true;
    assume $IsValid'$1_object_Object'#0''($t0);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:498:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @20 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:498:5+1
    $1_object_ObjectCore_$memory#20 := $1_object_ObjectCore_$memory;

    // trace_local[object]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:498:5+1
    assume {:print "$track_local(52,40,0):", $t0} $t0 == $t0;

    // $t1 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:500:32+12
    assume {:print "$at(2,19933,19945)"} true;
    $t1 := $inner#$1_object_Object'#0'($t0);

    // $t2 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:500:13+6
    $t2 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // if ($t2) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:499:9+118
    assume {:print "$at(2,19893,20011)"} true;
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:499:9+118
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:499:9+118
    assume {:print "$at(2,19893,20011)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:501:30+22
    assume {:print "$at(2,19977,19999)"} true;
L0:

    // $t3 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:501:30+22
    assume {:print "$at(2,19977,19999)"} true;
    $t3 := 2;
    assume $IsValid'u64'($t3);

    // $t4 := error::not_found($t3) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:501:13+40
    call $t4 := $1_error_not_found($t3);
    if ($abort_flag) {
        assume {:print "$at(2,19960,20000)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,40):", $t5} $t5 == $t5;
        goto L4;
    }

    // trace_abort($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:499:9+118
    assume {:print "$at(2,19893,20011)"} true;
    assume {:print "$track_abort(52,40):", $t4} $t4 == $t4;

    // $t5 := move($t4) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:499:9+118
    $t5 := $t4;

    // goto L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:499:9+118
    goto L4;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:503:35+6
    assume {:print "$at(2,20047,20053)"} true;
L2:

    // $t6 := get_field<object::Object<#0>>.inner($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:503:35+12
    assume {:print "$at(2,20047,20059)"} true;
    $t6 := $inner#$1_object_Object'#0'($t0);

    // $t7 := get_global<object::ObjectCore>($t6) on_abort goto L4 with $t5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:503:9+13
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t6)) {
        call $ExecFailureAbort();
    } else {
        $t7 := $ResourceValue($1_object_ObjectCore_$memory, $t6);
    }
    if ($abort_flag) {
        assume {:print "$at(2,20021,20034)"} true;
        $t5 := $abort_code;
        assume {:print "$track_abort(52,40):", $t5} $t5 == $t5;
        goto L4;
    }

    // $t8 := get_field<object::ObjectCore>.allow_ungated_transfer($t7) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:503:9+62
    $t8 := $allow_ungated_transfer#$1_object_ObjectCore($t7);

    // trace_return[0]($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:503:9+62
    assume {:print "$track_return(52,40,0):", $t8} $t8 == $t8;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:504:5+1
    assume {:print "$at(2,20088,20089)"} true;
L3:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:39+6
    assume {:print "$at(3,8833,8839)"} true;
    assume {:print "$track_exp_sub(25220):", $t0} true;

    // assume Identical($t9, exists[@20]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:20+32
    assume ($t9 == $ResourceExists($1_object_ObjectCore_$memory#20, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:20+32]($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:20+32
    assume {:print "$track_exp_sub(25221):", $t9} true;

    // assume Identical($t10, Not(exists[@20]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    assume ($t10 == !$ResourceExists($1_object_ObjectCore_$memory#20, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    assume {:print "$track_exp(25222):", $t10} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    assume {:print "$track_global_mem(27357):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@20]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    assert {:msg "assert_failed(3,8803,8847): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#20, $inner#$1_object_Object'#0'($t0));

    // return $t8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    $ret0 := $t8;
    return;

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:504:5+1
    assume {:print "$at(2,20088,20089)"} true;
L4:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:39+6]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:39+6
    assume {:print "$at(3,8833,8839)"} true;
    assume {:print "$track_exp_sub(25220):", $t0} true;

    // assume Identical($t11, exists[@20]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:20+32
    assume ($t11 == $ResourceExists($1_object_ObjectCore_$memory#20, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:20+32]($t11) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:20+32
    assume {:print "$track_exp_sub(25221):", $t11} true;

    // assume Identical($t12, Not(exists[@20]<object::ObjectCore>(select object::Object.inner($t0)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    assume ($t12 == !$ResourceExists($1_object_ObjectCore_$memory#20, $inner#$1_object_Object'#0'($t0)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44]($t12) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    assume {:print "$track_exp(25222):", $t12} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:226:9+44
    assume {:print "$track_global_mem(27358):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists[@20]<object::ObjectCore>(select object::Object.inner($t0))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:225:5+123
    assume {:print "$at(3,8730,8853)"} true;
    assert {:msg "assert_failed(3,8730,8853): abort not covered by any of the `aborts_if` clauses"}
      !$ResourceExists($1_object_ObjectCore_$memory#20, $inner#$1_object_Object'#0'($t0));

    // abort($t5) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:225:5+123
    $abort_code := $t5;
    $abort_flag := true;
    return;

}

// fun object::verify_ungated_and_descendant [baseline] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1411
procedure {:inline 1} $1_object_verify_ungated_and_descendant(_$t0: int, _$t1: int) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: $1_object_ObjectCore;
    var $t5: $1_object_ObjectCore;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: $1_object_ObjectCore;
    var $t11: bool;
    var $t12: int;
    var $t13: int;
    var $t14: int;
    var $t15: bool;
    var $t16: int;
    var $t17: int;
    var $t18: bool;
    var $t19: bool;
    var $t20: $1_object_ObjectCore;
    var $t21: bool;
    var $t22: int;
    var $t23: int;
    var $t24: int;
    var $t25: int;
    var $t26: int;
    var $t27: int;
    var $t28: int;
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    assume {:print "$at(2,18307,18308)"} true;
    assume {:print "$track_local(52,41,0):", $t0} $t0 == $t0;

    // trace_local[destination]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    assume {:print "$track_local(52,41,1):", $t1} $t1 == $t1;

    // trace_local[current_address]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:455:13+15
    assume {:print "$at(2,18413,18428)"} true;
    assume {:print "$track_local(52,41,2):", $t1} $t1 == $t1;

    // $t6 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:457:13+6
    assume {:print "$at(2,18473,18479)"} true;
    $t6 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // if ($t6) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    assume {:print "$at(2,18452,18573)"} true;
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    assume {:print "$at(2,18452,18573)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:458:30+22
    assume {:print "$at(2,18539,18561)"} true;
L0:

    // $t7 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:458:30+22
    assume {:print "$at(2,18539,18561)"} true;
    $t7 := 2;
    assume $IsValid'u64'($t7);

    // $t8 := error::not_found($t7) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:458:13+40
    call $t8 := $1_error_not_found($t7);
    if ($abort_flag) {
        assume {:print "$at(2,18522,18562)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    assume {:print "$at(2,18452,18573)"} true;
    assume {:print "$track_abort(52,41):", $t8} $t8 == $t8;

    // $t9 := move($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    $t9 := $t8;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    goto L21;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:461:48+15
    assume {:print "$at(2,18623,18638)"} true;
L2:

    // $t10 := get_global<object::ObjectCore>($t1) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:461:22+13
    assume {:print "$at(2,18597,18610)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t10 := $ResourceValue($1_object_ObjectCore_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,18597,18610)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_local[object]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:461:13+6
    assume {:print "$track_local(52,41,4):", $t10} $t10 == $t10;

    // $t11 := get_field<object::ObjectCore>.allow_ungated_transfer($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:463:13+29
    assume {:print "$at(2,18670,18699)"} true;
    $t11 := $allow_ungated_transfer#$1_object_ObjectCore($t10);

    // if ($t11) goto L4 else goto L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    assume {:print "$at(2,18649,18771)"} true;
    if ($t11) { goto L4; } else { goto L3; }

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
L4:

    // goto L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    assume {:print "$at(2,18649,18771)"} true;
    goto L5;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
L3:

    // $t12 := 3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:464:38+21
    assume {:print "$at(2,18738,18759)"} true;
    $t12 := 3;
    assume $IsValid'u64'($t12);

    // $t13 := error::permission_denied($t12) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:464:13+47
    call $t13 := $1_error_permission_denied($t12);
    if ($abort_flag) {
        assume {:print "$at(2,18713,18760)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    assume {:print "$at(2,18649,18771)"} true;
    assume {:print "$track_abort(52,41):", $t13} $t13 == $t13;

    // $t9 := move($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    $t9 := $t13;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    goto L21;

    // label L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:467:31+6
    assume {:print "$at(2,18804,18810)"} true;
L5:

    // $t14 := get_field<object::ObjectCore>.owner($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:467:31+12
    assume {:print "$at(2,18804,18816)"} true;
    $t14 := $owner#$1_object_ObjectCore($t10);

    // trace_local[current_address#1]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:467:13+15
    assume {:print "$track_local(52,41,3):", $t14} $t14 == $t14;

    // label L18 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$at(2,18857,18862)"} true;
L18:

    // $t3 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$at(2,18857,18862)"} true;
    havoc $t3;

    // assume WellFormed($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'address'($t3);

    // $t15 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t15;

    // assume WellFormed($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t15);

    // $t16 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t16;

    // assume WellFormed($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'u8'($t16);

    // $t17 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t17;

    // assume WellFormed($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'u8'($t17);

    // $t18 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t18;

    // assume WellFormed($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t18);

    // $t19 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t19;

    // assume WellFormed($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t19);

    // $t20 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t20;

    // assume WellFormed($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'$1_object_ObjectCore'($t20);

    // $t21 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t21;

    // assume WellFormed($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t21);

    // $t22 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t22;

    // assume WellFormed($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'address'($t22);

    // trace_local[current_address#1]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$info(): enter loop, variable(s) current_address#1 havocked and reassigned"} true;
    assume {:print "$track_local(52,41,3):", $t3} $t3 == $t3;

    // assume Not(AbortFlag()) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume !$abort_flag;

    // $t15 := !=($t0, $t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:22+2
    $t15 := !$IsEqual'address'($t0, $t3);

    // if ($t15) goto L7 else goto L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:9+861
    if ($t15) { goto L7; } else { goto L6; }

    // label L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:9+861
L7:

    // label L8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:472:25+9
    assume {:print "$at(2,18917,18926)"} true;
L8:

    // $t16 := 1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:472:25+9
    assume {:print "$at(2,18917,18926)"} true;
    $t16 := 1;
    assume $IsValid'u8'($t16);

    // $t17 := 8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:29+22
    assume {:print "$at(2,18956,18978)"} true;
    $t17 := 8;
    assume $IsValid'u8'($t17);

    // $t18 := <($t16, $t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:27+1
    call $t18 := $Lt($t16, $t17);

    // if ($t18) goto L10 else goto L9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    if ($t18) { goto L10; } else { goto L9; }

    // label L10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
L10:

    // goto L11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    assume {:print "$at(2,18940,19018)"} true;
    goto L11;

    // label L9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:73+16
L9:

    // $t23 := 6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:73+16
    assume {:print "$at(2,19000,19016)"} true;
    $t23 := 6;
    assume $IsValid'u64'($t23);

    // $t24 := error::out_of_range($t23) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:53+37
    call $t24 := $1_error_out_of_range($t23);
    if ($abort_flag) {
        assume {:print "$at(2,18980,19017)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    assume {:print "$at(2,18940,19018)"} true;
    assume {:print "$track_abort(52,41):", $t24} $t24 == $t24;

    // $t9 := move($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    $t9 := $t24;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    goto L21;

    // label L11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:475:13+75
    assume {:print "$at(2,19033,19108)"} true;
L11:

    // assume Eq<address>($t1, $t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:17+38
    assume {:print "$at(2,19056,19094)"} true;
    assume $IsEqual'address'($t1, $t3);

    // $t19 := exists<object::ObjectCore>($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:482:17+6
    assume {:print "$at(2,19326,19332)"} true;
    $t19 := $ResourceExists($1_object_ObjectCore_$memory, $t3);

    // if ($t19) goto L13 else goto L12 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    assume {:print "$at(2,19301,19437)"} true;
    if ($t19) { goto L13; } else { goto L12; }

    // label L13 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
L13:

    // goto L14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    assume {:print "$at(2,19301,19437)"} true;
    goto L14;

    // label L12 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:483:42+17
    assume {:print "$at(2,19404,19421)"} true;
L12:

    // $t25 := 4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:483:42+17
    assume {:print "$at(2,19404,19421)"} true;
    $t25 := 4;
    assume $IsValid'u64'($t25);

    // $t26 := error::permission_denied($t25) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:483:17+43
    call $t26 := $1_error_permission_denied($t25);
    if ($abort_flag) {
        assume {:print "$at(2,19379,19422)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    assume {:print "$at(2,19301,19437)"} true;
    assume {:print "$track_abort(52,41):", $t26} $t26 == $t26;

    // $t9 := move($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    $t9 := $t26;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    goto L21;

    // label L14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:485:52+15
    assume {:print "$at(2,19490,19505)"} true;
L14:

    // $t20 := get_global<object::ObjectCore>($t3) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:485:26+13
    assume {:print "$at(2,19464,19477)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t3)) {
        call $ExecFailureAbort();
    } else {
        $t20 := $ResourceValue($1_object_ObjectCore_$memory, $t3);
    }
    if ($abort_flag) {
        assume {:print "$at(2,19464,19477)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_local[object#3]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:485:17+6
    assume {:print "$track_local(52,41,5):", $t20} $t20 == $t20;

    // $t21 := get_field<object::ObjectCore>.allow_ungated_transfer($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:487:17+29
    assume {:print "$at(2,19545,19574)"} true;
    $t21 := $allow_ungated_transfer#$1_object_ObjectCore($t20);

    // if ($t21) goto L16 else goto L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    assume {:print "$at(2,19520,19654)"} true;
    if ($t21) { goto L16; } else { goto L15; }

    // label L16 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
L16:

    // goto L17 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    assume {:print "$at(2,19520,19654)"} true;
    goto L17;

    // label L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
L15:

    // $t27 := 3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:488:42+21
    assume {:print "$at(2,19617,19638)"} true;
    $t27 := 3;
    assume $IsValid'u64'($t27);

    // $t28 := error::permission_denied($t27) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:488:17+47
    call $t28 := $1_error_permission_denied($t27);
    if ($abort_flag) {
        assume {:print "$at(2,19592,19639)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    assume {:print "$at(2,19520,19654)"} true;
    assume {:print "$track_abort(52,41):", $t28} $t28 == $t28;

    // $t9 := move($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    $t9 := $t28;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    goto L21;

    // label L17 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:31+6
    assume {:print "$at(2,19687,19693)"} true;
L17:

    // $t22 := get_field<object::ObjectCore>.owner($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:31+12
    assume {:print "$at(2,19687,19699)"} true;
    $t22 := $owner#$1_object_ObjectCore($t20);

    // trace_local[current_address#1]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:13+15
    assume {:print "$track_local(52,41,3):", $t22} $t22 == $t22;

    // goto L19 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:43+1
    goto L19;

    // label L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    assume {:print "$at(2,19711,19712)"} true;
L6:

    // goto L20 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    assume {:print "$at(2,19711,19712)"} true;
    goto L20;

    // label L19 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    // Loop invariant checking block for the loop started with header: L18
L19:

    // stop() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    assume {:print "$at(2,19711,19712)"} true;
    assume false;
    return;

    // label L20 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:494:5+1
    assume {:print "$at(2,19717,19718)"} true;
L20:

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:494:5+1
    assume {:print "$at(2,19717,19718)"} true;
    return;

    // label L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:494:5+1
L21:

    // abort($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:494:5+1
    assume {:print "$at(2,19717,19718)"} true;
    $abort_code := $t9;
    $abort_flag := true;
    return;

}

// fun object::verify_ungated_and_descendant [verification] at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1411
procedure {:timeLimit 40} $1_object_verify_ungated_and_descendant$verify(_$t0: int, _$t1: int) returns ()
{
    // declare local variables
    var $t2: int;
    var $t3: int;
    var $t4: $1_object_ObjectCore;
    var $t5: $1_object_ObjectCore;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: $1_object_ObjectCore;
    var $t11: bool;
    var $t12: int;
    var $t13: int;
    var $t14: int;
    var $t15: bool;
    var $t16: bool;
    var $t17: bool;
    var $t18: int;
    var $t19: bool;
    var $t20: int;
    var $t21: bool;
    var $t22: bool;
    var $t23: bool;
    var $t24: bool;
    var $t25: bool;
    var $t26: $1_object_ObjectCore;
    var $t27: bool;
    var $t28: bool;
    var $t29: bool;
    var $t30: int;
    var $t31: bool;
    var $t32: bool;
    var $t33: int;
    var $t34: int;
    var $t35: bool;
    var $t36: int;
    var $t37: int;
    var $t38: int;
    var $t39: int;
    var $t40: bool;
    var $t41: bool;
    var $t42: $1_object_ObjectCore;
    var $t43: bool;
    var $t44: bool;
    var $t45: bool;
    var $t46: $1_object_ObjectCore;
    var $t47: bool;
    var $t48: bool;
    var $t49: bool;
    var $t50: $1_object_ObjectCore;
    var $t51: bool;
    var $t52: bool;
    var $t53: bool;
    var $t54: $1_object_ObjectCore;
    var $t55: bool;
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_object_ObjectCore': $1_object_ObjectCore;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    var $temp_0'u8': int;
    var $1_object_ObjectCore_$memory#24: $Memory $1_object_ObjectCore;
    $t0 := _$t0;
    $t1 := _$t1;

    // verification entrypoint assumptions
    call $InitVerification();

    // bytecode translation starts here
    // assume WellFormed($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    assume {:print "$at(2,18307,18308)"} true;
    assume $IsValid'address'($t0);

    // assume WellFormed($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    assume $IsValid'address'($t1);

    // assume forall $rsc: ResourceDomain<object::ObjectCore>(): WellFormed($rsc) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    assume (forall $a_0: int :: {$ResourceValue($1_object_ObjectCore_$memory, $a_0)}(var $rsc := $ResourceValue($1_object_ObjectCore_$memory, $a_0);
    ($IsValid'$1_object_ObjectCore'($rsc))));

    // @24 := save_mem(object::ObjectCore) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    $1_object_ObjectCore_$memory#24 := $1_object_ObjectCore_$memory;

    // trace_local[owner]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    assume {:print "$track_local(52,41,0):", $t0} $t0 == $t0;

    // trace_local[destination]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:454:5+1
    assume {:print "$track_local(52,41,1):", $t1} $t1 == $t1;

    // trace_local[current_address]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:455:13+15
    assume {:print "$at(2,18413,18428)"} true;
    assume {:print "$track_local(52,41,2):", $t1} $t1 == $t1;

    // $t6 := exists<object::ObjectCore>($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:457:13+6
    assume {:print "$at(2,18473,18479)"} true;
    $t6 := $ResourceExists($1_object_ObjectCore_$memory, $t1);

    // if ($t6) goto L1 else goto L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    assume {:print "$at(2,18452,18573)"} true;
    if ($t6) { goto L1; } else { goto L0; }

    // label L1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
L1:

    // goto L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    assume {:print "$at(2,18452,18573)"} true;
    goto L2;

    // label L0 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:458:30+22
    assume {:print "$at(2,18539,18561)"} true;
L0:

    // $t7 := 2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:458:30+22
    assume {:print "$at(2,18539,18561)"} true;
    $t7 := 2;
    assume $IsValid'u64'($t7);

    // $t8 := error::not_found($t7) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:458:13+40
    call $t8 := $1_error_not_found($t7);
    if ($abort_flag) {
        assume {:print "$at(2,18522,18562)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    assume {:print "$at(2,18452,18573)"} true;
    assume {:print "$track_abort(52,41):", $t8} $t8 == $t8;

    // $t9 := move($t8) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    $t9 := $t8;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:456:9+121
    goto L21;

    // label L2 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:461:48+15
    assume {:print "$at(2,18623,18638)"} true;
L2:

    // $t10 := get_global<object::ObjectCore>($t1) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:461:22+13
    assume {:print "$at(2,18597,18610)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t10 := $ResourceValue($1_object_ObjectCore_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,18597,18610)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_local[object]($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:461:13+6
    assume {:print "$track_local(52,41,4):", $t10} $t10 == $t10;

    // $t11 := get_field<object::ObjectCore>.allow_ungated_transfer($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:463:13+29
    assume {:print "$at(2,18670,18699)"} true;
    $t11 := $allow_ungated_transfer#$1_object_ObjectCore($t10);

    // if ($t11) goto L4 else goto L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    assume {:print "$at(2,18649,18771)"} true;
    if ($t11) { goto L4; } else { goto L3; }

    // label L4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
L4:

    // goto L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    assume {:print "$at(2,18649,18771)"} true;
    goto L5;

    // label L3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
L3:

    // $t12 := 3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:464:38+21
    assume {:print "$at(2,18738,18759)"} true;
    $t12 := 3;
    assume $IsValid'u64'($t12);

    // $t13 := error::permission_denied($t12) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:464:13+47
    call $t13 := $1_error_permission_denied($t12);
    if ($abort_flag) {
        assume {:print "$at(2,18713,18760)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    assume {:print "$at(2,18649,18771)"} true;
    assume {:print "$track_abort(52,41):", $t13} $t13 == $t13;

    // $t9 := move($t13) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    $t9 := $t13;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:462:9+122
    goto L21;

    // label L5 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:467:31+6
    assume {:print "$at(2,18804,18810)"} true;
L5:

    // $t14 := get_field<object::ObjectCore>.owner($t10) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:467:31+12
    assume {:print "$at(2,18804,18816)"} true;
    $t14 := $owner#$1_object_ObjectCore($t10);

    // trace_local[current_address#1]($t14) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:467:13+15
    assume {:print "$track_local(52,41,3):", $t14} $t14 == $t14;

    // label L18 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$at(2,18857,18862)"} true;
L18:

    // $t3 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$at(2,18857,18862)"} true;
    havoc $t3;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25371):", $t3} true;

    // assume Identical($t15, WellFormed($t3)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t15 == $IsValid'address'($t3));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t15) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25372):", $t15} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27359):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'address'($t3);

    // $t16 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t16;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25375):", $t16} true;

    // assume Identical($t17, WellFormed($t16)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t17 == $IsValid'bool'($t16));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t17) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25376):", $t17} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27360):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t16) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t16);

    // $t18 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t18;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25379):", $t18} true;

    // assume Identical($t19, WellFormed($t18)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t19 == $IsValid'u8'($t18));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t19) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25380):", $t19} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27361):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t18) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'u8'($t18);

    // $t20 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t20;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25383):", $t20} true;

    // assume Identical($t21, WellFormed($t20)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t21 == $IsValid'u8'($t20));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t21) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25384):", $t21} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27362):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'u8'($t20);

    // $t22 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t22;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25387):", $t22} true;

    // assume Identical($t23, WellFormed($t22)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t23 == $IsValid'bool'($t22));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t23) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25388):", $t23} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27363):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t22) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t22);

    // $t24 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t24;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25391):", $t24} true;

    // assume Identical($t25, WellFormed($t24)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t25 == $IsValid'bool'($t24));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t25) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25392):", $t25} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27364):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t24) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t24);

    // $t26 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t26;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25395):", $t26} true;

    // assume Identical($t27, WellFormed($t26)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t27 == $IsValid'$1_object_ObjectCore'($t26));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t27) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25396):", $t27} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27365):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'$1_object_ObjectCore'($t26);

    // $t28 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t28;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25399):", $t28} true;

    // assume Identical($t29, WellFormed($t28)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t29 == $IsValid'bool'($t28));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t29) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25400):", $t29} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27366):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t28) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'bool'($t28);

    // $t30 := havoc[val]() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    havoc $t30;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t30) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp_sub(25403):", $t30} true;

    // assume Identical($t31, WellFormed($t30)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t31 == $IsValid'address'($t30));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t31) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25404):", $t31} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27367):", $1_object_ObjectCore_$memory} true;

    // assume WellFormed($t30) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume $IsValid'address'($t30);

    // trace_local[current_address#1]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$info(): enter loop, variable(s) current_address#1 havocked and reassigned"} true;
    assume {:print "$track_local(52,41,3):", $t3} $t3 == $t3;

    // assume Identical($t32, Not(AbortFlag())) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume ($t32 == !$abort_flag);

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5]($t32) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_exp(25406):", $t32} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume {:print "$track_global_mem(27368):", $1_object_ObjectCore_$memory} true;

    // assume Not(AbortFlag()) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:16+5
    assume !$abort_flag;

    // $t16 := !=($t0, $t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:22+2
    $t16 := !$IsEqual'address'($t0, $t3);

    // if ($t16) goto L7 else goto L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:9+861
    if ($t16) { goto L7; } else { goto L6; }

    // label L7 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:470:9+861
L7:

    // label L8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:472:25+9
    assume {:print "$at(2,18917,18926)"} true;
L8:

    // $t18 := 1 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:472:25+9
    assume {:print "$at(2,18917,18926)"} true;
    $t18 := 1;
    assume $IsValid'u8'($t18);

    // $t20 := 8 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:29+22
    assume {:print "$at(2,18956,18978)"} true;
    $t20 := 8;
    assume $IsValid'u8'($t20);

    // $t22 := <($t18, $t20) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:27+1
    call $t22 := $Lt($t18, $t20);

    // if ($t22) goto L10 else goto L9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    if ($t22) { goto L10; } else { goto L9; }

    // label L10 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
L10:

    // goto L11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    assume {:print "$at(2,18940,19018)"} true;
    goto L11;

    // label L9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:73+16
L9:

    // $t33 := 6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:73+16
    assume {:print "$at(2,19000,19016)"} true;
    $t33 := 6;
    assume $IsValid'u64'($t33);

    // $t34 := error::out_of_range($t33) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:53+37
    call $t34 := $1_error_out_of_range($t33);
    if ($abort_flag) {
        assume {:print "$at(2,18980,19017)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t34) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    assume {:print "$at(2,18940,19018)"} true;
    assume {:print "$track_abort(52,41):", $t34} $t34 == $t34;

    // $t9 := move($t34) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    $t9 := $t34;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:473:13+78
    goto L21;

    // label L11 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:475:13+75
    assume {:print "$at(2,19033,19108)"} true;
L11:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:24+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:24+11
    assume {:print "$at(2,19063,19074)"} true;
    assume {:print "$track_exp_sub(25410):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:39+15]($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:39+15
    assume {:print "$track_exp_sub(25411):", $t3} true;

    // assume Identical($t35, Eq<address>($t1, $t3)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:17+38
    assume ($t35 == $IsEqual'address'($t1, $t3));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:17+38]($t35) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:17+38
    assume {:print "$track_exp(25412):", $t35} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:17+38
    assume {:print "$track_global_mem(27369):", $1_object_ObjectCore_$memory} true;

    // assume Eq<address>($t1, $t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:476:17+38
    assume $IsEqual'address'($t1, $t3);

    // $t24 := exists<object::ObjectCore>($t3) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:482:17+6
    assume {:print "$at(2,19326,19332)"} true;
    $t24 := $ResourceExists($1_object_ObjectCore_$memory, $t3);

    // if ($t24) goto L13 else goto L12 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    assume {:print "$at(2,19301,19437)"} true;
    if ($t24) { goto L13; } else { goto L12; }

    // label L13 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
L13:

    // goto L14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    assume {:print "$at(2,19301,19437)"} true;
    goto L14;

    // label L12 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:483:42+17
    assume {:print "$at(2,19404,19421)"} true;
L12:

    // $t36 := 4 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:483:42+17
    assume {:print "$at(2,19404,19421)"} true;
    $t36 := 4;
    assume $IsValid'u64'($t36);

    // $t37 := error::permission_denied($t36) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:483:17+43
    call $t37 := $1_error_permission_denied($t36);
    if ($abort_flag) {
        assume {:print "$at(2,19379,19422)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t37) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    assume {:print "$at(2,19301,19437)"} true;
    assume {:print "$track_abort(52,41):", $t37} $t37 == $t37;

    // $t9 := move($t37) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    $t9 := $t37;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:481:13+136
    goto L21;

    // label L14 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:485:52+15
    assume {:print "$at(2,19490,19505)"} true;
L14:

    // $t26 := get_global<object::ObjectCore>($t3) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:485:26+13
    assume {:print "$at(2,19464,19477)"} true;
    if (!$ResourceExists($1_object_ObjectCore_$memory, $t3)) {
        call $ExecFailureAbort();
    } else {
        $t26 := $ResourceValue($1_object_ObjectCore_$memory, $t3);
    }
    if ($abort_flag) {
        assume {:print "$at(2,19464,19477)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_local[object#3]($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:485:17+6
    assume {:print "$track_local(52,41,5):", $t26} $t26 == $t26;

    // $t28 := get_field<object::ObjectCore>.allow_ungated_transfer($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:487:17+29
    assume {:print "$at(2,19545,19574)"} true;
    $t28 := $allow_ungated_transfer#$1_object_ObjectCore($t26);

    // if ($t28) goto L16 else goto L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    assume {:print "$at(2,19520,19654)"} true;
    if ($t28) { goto L16; } else { goto L15; }

    // label L16 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
L16:

    // goto L17 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    assume {:print "$at(2,19520,19654)"} true;
    goto L17;

    // label L15 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
L15:

    // $t38 := 3 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:488:42+21
    assume {:print "$at(2,19617,19638)"} true;
    $t38 := 3;
    assume $IsValid'u64'($t38);

    // $t39 := error::permission_denied($t38) on_abort goto L21 with $t9 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:488:17+47
    call $t39 := $1_error_permission_denied($t38);
    if ($abort_flag) {
        assume {:print "$at(2,19592,19639)"} true;
        $t9 := $abort_code;
        assume {:print "$track_abort(52,41):", $t9} $t9 == $t9;
        goto L21;
    }

    // trace_abort($t39) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    assume {:print "$at(2,19520,19654)"} true;
    assume {:print "$track_abort(52,41):", $t39} $t39 == $t39;

    // $t9 := move($t39) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    $t9 := $t39;

    // goto L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:486:13+134
    goto L21;

    // label L17 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:31+6
    assume {:print "$at(2,19687,19693)"} true;
L17:

    // $t30 := get_field<object::ObjectCore>.owner($t26) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:31+12
    assume {:print "$at(2,19687,19699)"} true;
    $t30 := $owner#$1_object_ObjectCore($t26);

    // trace_local[current_address#1]($t30) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:13+15
    assume {:print "$track_local(52,41,3):", $t30} $t30 == $t30;

    // goto L19 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:491:43+1
    goto L19;

    // label L6 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    assume {:print "$at(2,19711,19712)"} true;
L6:

    // goto L20 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    assume {:print "$at(2,19711,19712)"} true;
    goto L20;

    // label L19 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    // Loop invariant checking block for the loop started with header: L18
L19:

    // stop() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:493:10+1
    assume {:print "$at(2,19711,19712)"} true;
    assume false;
    return;

    // label L20 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:494:5+1
    assume {:print "$at(2,19717,19718)"} true;
L20:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:39+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:39+11
    assume {:print "$at(3,8345,8356)"} true;
    assume {:print "$track_exp_sub(25333):", $t1} true;

    // assume Identical($t40, exists[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:20+31
    assume ($t40 == $ResourceExists($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:20+31]($t40) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:20+31
    assume {:print "$track_exp_sub(25334):", $t40} true;

    // assume Identical($t41, Not(exists[@24]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43
    assume ($t41 == !$ResourceExists($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43]($t41) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43
    assume {:print "$track_exp(25335):", $t41} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43
    assume {:print "$track_global_mem(27370):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(exists[@24]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43
    assert {:msg "assert_failed(3,8315,8358): function does not abort under this condition"}
      !!$ResourceExists($1_object_ObjectCore_$memory#24, $t1);

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:39+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:39+11
    assume {:print "$at(3,8397,8408)"} true;
    assume {:print "$track_exp_sub(25340):", $t1} true;

    // assume Identical($t42, global[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:20+31
    assume ($t42 == $ResourceValue($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:20+31]($t42) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:20+31
    assume {:print "$track_exp_sub(25341):", $t42} true;

    // assume Identical($t43, Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66
    assume ($t43 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66]($t43) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66
    assume {:print "$track_exp(25342):", $t43} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66
    assume {:print "$track_global_mem(27371):", $1_object_ObjectCore_$memory} true;

    // assert Not(Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66
    assert {:msg "assert_failed(3,8367,8433): function does not abort under this condition"}
      !!$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:13+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:13+5
    assume {:print "$at(3,8507,8512)"} true;
    assume {:print "$track_exp_sub(25349):", $t0} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:22+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:22+11
    assume {:print "$track_exp_sub(25351):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:57+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:57+11
    assume {:print "$track_exp_sub(25353):", $t1} true;

    // assume Identical($t44, exists[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:38+31
    assume ($t44 == $ResourceExists($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:38+31]($t44) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:38+31
    assume {:print "$track_exp_sub(25354):", $t44} true;

    // assume Identical($t45, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(exists[@24]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122
    assume {:print "$at(3,8442,8564)"} true;
    assume ($t45 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t0, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#24, $t1)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122]($t45) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122
    assume {:print "$track_exp(25355):", $t45} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122
    assume {:print "$track_global_mem(27372):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(exists[@24]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122
    assert {:msg "assert_failed(3,8442,8564): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t0, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#24, $t1))))));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:13+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:13+5
    assume {:print "$at(3,8638,8643)"} true;
    assume {:print "$track_exp_sub(25362):", $t0} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:22+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:22+11
    assume {:print "$track_exp_sub(25364):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:57+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:57+11
    assume {:print "$track_exp_sub(25366):", $t1} true;

    // assume Identical($t46, global[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:38+31
    assume ($t46 == $ResourceValue($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:38+31]($t46) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:38+31
    assume {:print "$track_exp_sub(25367):", $t46} true;

    // assume Identical($t47, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    assume {:print "$at(3,8573,8718)"} true;
    assume ($t47 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t0, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145]($t47) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    assume {:print "$track_exp(25368):", $t47} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    assume {:print "$track_global_mem(27373):", $1_object_ObjectCore_$memory} true;

    // assert Not(exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    assert {:msg "assert_failed(3,8573,8718): function does not abort under this condition"}
      !(var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t0, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1)))))));

    // return () at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    return;

    // label L21 at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.move:494:5+1
    assume {:print "$at(2,19717,19718)"} true;
L21:

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:39+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:39+11
    assume {:print "$at(3,8345,8356)"} true;
    assume {:print "$track_exp_sub(25333):", $t1} true;

    // assume Identical($t48, exists[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:20+31
    assume ($t48 == $ResourceExists($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:20+31]($t48) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:20+31
    assume {:print "$track_exp_sub(25334):", $t48} true;

    // assume Identical($t49, Not(exists[@24]<object::ObjectCore>($t1))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43
    assume ($t49 == !$ResourceExists($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43]($t49) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:217:9+43
    assume {:print "$track_exp(25335):", $t49} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:39+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:39+11
    assume {:print "$at(3,8397,8408)"} true;
    assume {:print "$track_exp_sub(25340):", $t1} true;

    // assume Identical($t50, global[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:20+31
    assume ($t50 == $ResourceValue($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:20+31]($t50) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:20+31
    assume {:print "$track_exp_sub(25341):", $t50} true;

    // assume Identical($t51, Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66
    assume ($t51 == !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1)));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66]($t51) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:218:9+66
    assume {:print "$track_exp(25342):", $t51} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:13+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:13+5
    assume {:print "$at(3,8507,8512)"} true;
    assume {:print "$track_exp_sub(25349):", $t0} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:22+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:22+11
    assume {:print "$track_exp_sub(25351):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:57+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:57+11
    assume {:print "$track_exp_sub(25353):", $t1} true;

    // assume Identical($t52, exists[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:38+31
    assume ($t52 == $ResourceExists($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:38+31]($t52) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:220:38+31
    assume {:print "$track_exp_sub(25354):", $t52} true;

    // assume Identical($t53, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(exists[@24]<object::ObjectCore>($t1)))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122
    assume {:print "$at(3,8442,8564)"} true;
    assume ($t53 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t0, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#24, $t1)))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122]($t53) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:219:9+122
    assume {:print "$track_exp(25355):", $t53} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:13+5]($t0) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:13+5
    assume {:print "$at(3,8638,8643)"} true;
    assume {:print "$track_exp_sub(25362):", $t0} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:22+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:22+11
    assume {:print "$track_exp_sub(25364):", $t1} true;

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:57+11]($t1) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:57+11
    assume {:print "$track_exp_sub(25366):", $t1} true;

    // assume Identical($t54, global[@24]<object::ObjectCore>($t1)) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:38+31
    assume ($t54 == $ResourceValue($1_object_ObjectCore_$memory#24, $t1));

    // trace_exp[subauto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:38+31]($t54) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:222:38+31
    assume {:print "$track_exp_sub(25367):", $t54} true;

    // assume Identical($t55, exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    assume {:print "$at(3,8573,8718)"} true;
    assume ($t55 == (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t0, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1))))))));

    // trace_exp[auto, at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145]($t55) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    assume {:print "$track_exp(25368):", $t55} true;

    // trace_global_mem() at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:221:9+145
    assume {:print "$track_global_mem(27374):", $1_object_ObjectCore_$memory} true;

    // assert Or(Or(Or(Not(exists[@24]<object::ObjectCore>($t1)), Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(exists[@24]<object::ObjectCore>($t1)))), exists i: Range(0, Sub(8, 1)): And(Neq<address>($t0, $t1), Not(select object::ObjectCore.allow_ungated_transfer(global[@24]<object::ObjectCore>($t1))))) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:216:5+492
    assume {:print "$at(3,8232,8724)"} true;
    assert {:msg "assert_failed(3,8232,8724): abort not covered by any of the `aborts_if` clauses"}
      (((!$ResourceExists($1_object_ObjectCore_$memory#24, $t1) || !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1))) || (var $range_0 := $Range(0, (8 - 1)); (exists $i_1: int :: $InRange($range_0, $i_1) && (var i := $i_1;
    ((!$IsEqual'address'($t0, $t1) && !$ResourceExists($1_object_ObjectCore_$memory#24, $t1))))))) || (var $range_2 := $Range(0, (8 - 1)); (exists $i_3: int :: $InRange($range_2, $i_3) && (var i := $i_3;
    ((!$IsEqual'address'($t0, $t1) && !$allow_ungated_transfer#$1_object_ObjectCore($ResourceValue($1_object_ObjectCore_$memory#24, $t1))))))));

    // abort($t9) at /home/zr/Downloads/New/aptos-core/aptos-move/framework/aptos-framework/sources/object.spec.move:216:5+492
    $abort_code := $t9;
    $abort_flag := true;
    return;

}
