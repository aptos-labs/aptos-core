
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
function {:inline} $1_aggregator_limit(s: $1_aggregator_Aggregator): int {
    s->$limit
}
procedure {:inline 1} $1_aggregator_limit(s: $1_aggregator_Aggregator) returns (res: int) {
    res := s->$limit;
    return;
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

// ==================================================================================
// Native for function_info

procedure $1_function_info_is_identifier(s: Vec int) returns (res: bool);



// Uninterpreted function for all types

function $Arbitrary_value_of'#0'(): #0;

function $Arbitrary_value_of'$1_account_Account'(): $1_account_Account;

function $Arbitrary_value_of'$1_account_CapabilityOffer'$1_account_RotationCapability''(): $1_account_CapabilityOffer'$1_account_RotationCapability';

function $Arbitrary_value_of'$1_account_CapabilityOffer'$1_account_SignerCapability''(): $1_account_CapabilityOffer'$1_account_SignerCapability';

function $Arbitrary_value_of'$1_account_SignerCapability'(): $1_account_SignerCapability;

function $Arbitrary_value_of'$1_chain_status_GenesisEndMarker'(): $1_chain_status_GenesisEndMarker;

function $Arbitrary_value_of'$1_event_EventHandle'$1_account_CoinRegisterEvent''(): $1_event_EventHandle'$1_account_CoinRegisterEvent';

function $Arbitrary_value_of'$1_event_EventHandle'$1_account_KeyRotationEvent''(): $1_event_EventHandle'$1_account_KeyRotationEvent';

function $Arbitrary_value_of'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent';

function $Arbitrary_value_of'$1_features_Features'(): $1_features_Features;

function $Arbitrary_value_of'$1_guid_GUID'(): $1_guid_GUID;

function $Arbitrary_value_of'$1_guid_ID'(): $1_guid_ID;

function $Arbitrary_value_of'$1_option_Option'address''(): $1_option_Option'address';

function $Arbitrary_value_of'$1_permissioned_signer_GrantedPermissionHandles'(): $1_permissioned_signer_GrantedPermissionHandles;

function $Arbitrary_value_of'$1_reconfiguration_Configuration'(): $1_reconfiguration_Configuration;

function $Arbitrary_value_of'$1_timelock_AddCreators'(): $1_timelock_AddCreators;

function $Arbitrary_value_of'$1_timelock_AddExecutors'(): $1_timelock_AddExecutors;

function $Arbitrary_value_of'$1_timelock_CancelTransaction'(): $1_timelock_CancelTransaction;

function $Arbitrary_value_of'$1_timelock_CreateTransaction'(): $1_timelock_CreateTransaction;

function $Arbitrary_value_of'$1_timelock_RemoveCreators'(): $1_timelock_RemoveCreators;

function $Arbitrary_value_of'$1_timelock_RemoveExecutors'(): $1_timelock_RemoveExecutors;

function $Arbitrary_value_of'$1_timelock_TimelockAccount'(): $1_timelock_TimelockAccount;

function $Arbitrary_value_of'$1_timelock_TimelockTransaction'(): $1_timelock_TimelockTransaction;

function $Arbitrary_value_of'$1_timelock_UpdateMinNumSecondsExecute'(): $1_timelock_UpdateMinNumSecondsExecute;

function $Arbitrary_value_of'$1_timestamp_CurrentTimeMicroseconds'(): $1_timestamp_CurrentTimeMicroseconds;

function $Arbitrary_value_of'$1_type_info_TypeInfo'(): $1_type_info_TypeInfo;

function $Arbitrary_value_of'signer'(): $signer;

function $Arbitrary_value_of'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(): Table int ($1_timelock_TimelockTransaction);

function $Arbitrary_value_of'vec'#0''(): Vec (#0);

function $Arbitrary_value_of'vec'address''(): Vec (int);

function $Arbitrary_value_of'vec'u8''(): Vec (int);

function $Arbitrary_value_of'bool'(): bool;

function $Arbitrary_value_of'address'(): int;

function $Arbitrary_value_of'u256'(): int;

function $Arbitrary_value_of'u64'(): int;

function $Arbitrary_value_of'u8'(): int;

function $Arbitrary_value_of'vec'bv8''(): Vec (bv8);

function $Arbitrary_value_of'bv256'(): bv256;

function $Arbitrary_value_of'bv64'(): bv64;

function $Arbitrary_value_of'bv8'(): bv8;



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


function $castBv8to8(src: bv8) returns (bv8)
{
    src
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


function $castBv64to8(src: bv64) returns (bv8)
{
    if ($Gt'Bv64'(src, 255bv64)) then
        $Arbitrary_value_of'bv8'()
    else
    src[8:0]
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


function $castBv256to8(src: bv256) returns (bv8)
{
    if ($Gt'Bv256'(src, 255bv256)) then
        $Arbitrary_value_of'bv8'()
    else
    src[8:0]
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


function $castBv8to64(src: bv8) returns (bv64)
{
    0bv56 ++ src
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


function $castBv64to64(src: bv64) returns (bv64)
{
    src
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


function $castBv256to64(src: bv256) returns (bv64)
{
    if ($Gt'Bv256'(src, 18446744073709551615bv256)) then
        $Arbitrary_value_of'bv64'()
    else
    src[64:0]
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


function $castBv8to256(src: bv8) returns (bv256)
{
    0bv248 ++ src
}


function $shlBv256From8(src1: bv256, src2: bv8) returns (bv256)
{
    $Shl'Bv256'(src1, 0bv248 ++ src2)
}

procedure {:inline 1} $ShlBv256From8(src1: bv256, src2: bv8) returns (dst: bv256)
{
    assume $bv2int.8(src2) >= 0 && $bv2int.8(src2) < 256;
    dst := $Shl'Bv256'(src1, 0bv248 ++ src2);
}

function $shrBv256From8(src1: bv256, src2: bv8) returns (bv256)
{
    $Shr'Bv256'(src1, 0bv248 ++ src2)
}

procedure {:inline 1} $ShrBv256From8(src1: bv256, src2: bv8) returns (dst: bv256)
{
    assume $bv2int.8(src2) >= 0 && $bv2int.8(src2) < 256;
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


function $castBv64to256(src: bv64) returns (bv256)
{
    0bv192 ++ src
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


function $castBv256to256(src: bv256) returns (bv256)
{
    src
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
    dst := $Mutation(m->l, ExtendVec(m->p, index), ReadVec(v, index));
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

// ----------------------------------------------------------------------------------
// Native Table key encoding for type `vec'u8'`

function $EncodeKey'vec'u8''(k: Vec (int)): int;
axiom (
  forall k1, k2: Vec (int) :: {$EncodeKey'vec'u8''(k1), $EncodeKey'vec'u8''(k2)}
    $IsEqual'vec'u8''(k1, k2) <==> $EncodeKey'vec'u8''(k1) == $EncodeKey'vec'u8''(k2)
);


// ----------------------------------------------------------------------------------
// Native Table implementation for type `(vec'u8',$1_timelock_TimelockTransaction)`

function $IsEqual'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(t1: Table int ($1_timelock_TimelockTransaction), t2: Table int ($1_timelock_TimelockTransaction)): bool {
    LenTable(t1) == LenTable(t2) &&
    (forall k: int :: ContainsTable(t1, k) <==> ContainsTable(t2, k)) &&
    (forall k: int :: ContainsTable(t1, k) ==> GetTable(t1, k) == GetTable(t2, k)) &&
    (forall k: int :: ContainsTable(t2, k) ==> GetTable(t1, k) == GetTable(t2, k))
}

// Not inlined.
function $IsValid'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(t: Table int ($1_timelock_TimelockTransaction)): bool {
    $IsValid'u64'(LenTable(t)) &&
    (forall i: int:: ContainsTable(t, i) ==> $IsValid'$1_timelock_TimelockTransaction'(GetTable(t, i)))
}
procedure {:inline 2} $1_table_new'vec'u8'_$1_timelock_TimelockTransaction'() returns (v: Table int ($1_timelock_TimelockTransaction)) {
    v := EmptyTable();
}
procedure {:inline 2} $1_table_destroy_known_empty_unsafe'vec'u8'_$1_timelock_TimelockTransaction'(t: Table int ($1_timelock_TimelockTransaction)) {
    if (LenTable(t) != 0) {
        call $Abort($StdError(1/*INVALID_STATE*/, 102/*ENOT_EMPTY*/));
    }
}
procedure {:inline 2} $1_table_contains'vec'u8'_$1_timelock_TimelockTransaction'(t: (Table int ($1_timelock_TimelockTransaction)), k: Vec (int)) returns (r: bool) {
    r := ContainsTable(t, $EncodeKey'vec'u8''(k));
}
procedure {:inline 2} $1_table_add'vec'u8'_$1_timelock_TimelockTransaction'(m: $Mutation (Table int ($1_timelock_TimelockTransaction)), k: Vec (int), v: $1_timelock_TimelockTransaction) returns (m': $Mutation(Table int ($1_timelock_TimelockTransaction))) {
    var enc_k: int;
    var t: Table int ($1_timelock_TimelockTransaction);
    enc_k := $EncodeKey'vec'u8''(k);
    t := $Dereference(m);
    if (ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 100/*EALREADY_EXISTS*/));
    } else {
        m' := $UpdateMutation(m, AddTable(t, enc_k, v));
    }
}
procedure {:inline 2} $1_table_upsert'vec'u8'_$1_timelock_TimelockTransaction'(m: $Mutation (Table int ($1_timelock_TimelockTransaction)), k: Vec (int), v: $1_timelock_TimelockTransaction) returns (m': $Mutation(Table int ($1_timelock_TimelockTransaction))) {
    var enc_k: int;
    var t: Table int ($1_timelock_TimelockTransaction);
    enc_k := $EncodeKey'vec'u8''(k);
    t := $Dereference(m);
    if (ContainsTable(t, enc_k)) {
        m' := $UpdateMutation(m, UpdateTable(t, enc_k, v));
    } else {
        m' := $UpdateMutation(m, AddTable(t, enc_k, v));
    }
}
procedure {:inline 2} $1_table_remove'vec'u8'_$1_timelock_TimelockTransaction'(m: $Mutation (Table int ($1_timelock_TimelockTransaction)), k: Vec (int))
returns (v: $1_timelock_TimelockTransaction, m': $Mutation(Table int ($1_timelock_TimelockTransaction))) {
    var enc_k: int;
    var t: Table int ($1_timelock_TimelockTransaction);
    enc_k := $EncodeKey'vec'u8''(k);
    t := $Dereference(m);
    if (!ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        v := GetTable(t, enc_k);
        m' := $UpdateMutation(m, RemoveTable(t, enc_k));
    }
}
procedure {:inline 2} $1_table_borrow'vec'u8'_$1_timelock_TimelockTransaction'(t: Table int ($1_timelock_TimelockTransaction), k: Vec (int)) returns (v: $1_timelock_TimelockTransaction) {
    var enc_k: int;
    enc_k := $EncodeKey'vec'u8''(k);
    if (!ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        v := GetTable(t, $EncodeKey'vec'u8''(k));
    }
}
procedure {:inline 2} $1_table_borrow_mut'vec'u8'_$1_timelock_TimelockTransaction'(m: $Mutation (Table int ($1_timelock_TimelockTransaction)), k: Vec (int))
returns (dst: $Mutation ($1_timelock_TimelockTransaction), m': $Mutation (Table int ($1_timelock_TimelockTransaction))) {
    var enc_k: int;
    var t: Table int ($1_timelock_TimelockTransaction);
    enc_k := $EncodeKey'vec'u8''(k);
    t := $Dereference(m);
    if (!ContainsTable(t, enc_k)) {
        call $Abort($StdError(7/*INVALID_ARGUMENTS*/, 101/*ENOT_FOUND*/));
    } else {
        dst := $Mutation(m->l, ExtendVec(m->p, enc_k), GetTable(t, enc_k));
        m' := m;
    }
}
procedure {:inline 2} $1_table_borrow_mut_with_default'vec'u8'_$1_timelock_TimelockTransaction'(m: $Mutation (Table int ($1_timelock_TimelockTransaction)), k: Vec (int), default: $1_timelock_TimelockTransaction)
returns (dst: $Mutation ($1_timelock_TimelockTransaction), m': $Mutation (Table int ($1_timelock_TimelockTransaction))) {
    var enc_k: int;
    var t: Table int ($1_timelock_TimelockTransaction);
    var t': Table int ($1_timelock_TimelockTransaction);
    enc_k := $EncodeKey'vec'u8''(k);
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
procedure {:inline 2} $1_table_borrow_with_default'vec'u8'_$1_timelock_TimelockTransaction'(t: Table int ($1_timelock_TimelockTransaction), k: Vec (int), default: $1_timelock_TimelockTransaction) returns (v: $1_timelock_TimelockTransaction) {
    var enc_k: int;
    enc_k := $EncodeKey'vec'u8''(k);
    if (!ContainsTable(t, enc_k)) {
        v := default;
    } else {
        v := GetTable(t, $EncodeKey'vec'u8''(k));
    }
}
function {:inline} $1_table_spec_contains'vec'u8'_$1_timelock_TimelockTransaction'(t: (Table int ($1_timelock_TimelockTransaction)), k: Vec (int)): bool {
    ContainsTable(t, $EncodeKey'vec'u8''(k))
}
function {:inline} $1_table_spec_set'vec'u8'_$1_timelock_TimelockTransaction'(t: Table int ($1_timelock_TimelockTransaction), k: Vec (int), v: $1_timelock_TimelockTransaction): Table int ($1_timelock_TimelockTransaction) {
    (var enc_k := $EncodeKey'vec'u8''(k);
    if (ContainsTable(t, enc_k)) then
        UpdateTable(t, enc_k, v)
    else
        AddTable(t, enc_k, v))
}
function {:inline} $1_table_spec_remove'vec'u8'_$1_timelock_TimelockTransaction'(t: Table int ($1_timelock_TimelockTransaction), k: Vec (int)): Table int ($1_timelock_TimelockTransaction) {
    RemoveTable(t, $EncodeKey'vec'u8''(k))
}
function {:inline} $1_table_spec_get'vec'u8'_$1_timelock_TimelockTransaction'(t: Table int ($1_timelock_TimelockTransaction), k: Vec (int)): $1_timelock_TimelockTransaction {
    GetTable(t, $EncodeKey'vec'u8''(k))
}



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
    $signer($addr: int),
    $permissioned_signer($addr: int, $permission_addr: int)
}

function {:inline} $IsValid'signer'(s: $signer): bool {
    if s is $signer then
        $IsValid'address'(s->$addr)
    else
        $IsValid'address'(s->$addr) &&
        $IsValid'address'(s->$permission_addr)
}

function {:inline} $IsEqual'signer'(s1: $signer, s2: $signer): bool {
    if s1 is $signer && s2 is $signer then
        s1 == s2
    else if s1 is $permissioned_signer && s2 is $permissioned_signer then
        s1 == s2
    else
        false
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

// ----------------------------------------------------------------------------------
// Native BCS implementation for element type `u64`

// Serialize is modeled as an uninterpreted function, with an additional
// axiom to say it's an injection.

function $1_bcs_serialize'u64'(v: int): Vec int;

axiom (forall v1, v2: int :: {$1_bcs_serialize'u64'(v1), $1_bcs_serialize'u64'(v2)}
   $IsEqual'u64'(v1, v2) <==> $IsEqual'vec'u8''($1_bcs_serialize'u64'(v1), $1_bcs_serialize'u64'(v2)));

// This says that serialize returns a non-empty vec<u8>

axiom (forall v: int :: {$1_bcs_serialize'u64'(v)}
     ( var r := $1_bcs_serialize'u64'(v); $IsValid'vec'u8''(r) && LenVec(r) > 0 ));


procedure $1_bcs_to_bytes'u64'(v: int) returns (res: Vec int);
ensures res == $1_bcs_serialize'u64'(v);

function {:inline} $1_bcs_$to_bytes'u64'(v: int): Vec int {
    $1_bcs_serialize'u64'(v)
}





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

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'bool'(b1), $1_from_bcs_deserializable'bool'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u8'(b1), $1_from_bcs_deserializable'u8'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u64'(b1), $1_from_bcs_deserializable'u64'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'u256'(b1), $1_from_bcs_deserializable'u256'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'address'(b1), $1_from_bcs_deserializable'address'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'signer'(b1), $1_from_bcs_deserializable'signer'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'u8''(b1), $1_from_bcs_deserializable'vec'u8''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'address''(b1), $1_from_bcs_deserializable'vec'address''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'vec'#0''(b1), $1_from_bcs_deserializable'vec'#0''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::option::Option<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_option_Option'address''(b1), $1_from_bcs_deserializable'$1_option_Option'address''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::features::Features>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_features_Features'(b1), $1_from_bcs_deserializable'$1_features_Features'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::type_info::TypeInfo>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_type_info_TypeInfo'(b1), $1_from_bcs_deserializable'$1_type_info_TypeInfo'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::table::Table<vector<u8>, 0x1::timelock::TimelockTransaction>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(b1), $1_from_bcs_deserializable'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::chain_status::GenesisEndMarker>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_chain_status_GenesisEndMarker'(b1), $1_from_bcs_deserializable'$1_chain_status_GenesisEndMarker'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timestamp::CurrentTimeMicroseconds>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timestamp_CurrentTimeMicroseconds'(b1), $1_from_bcs_deserializable'$1_timestamp_CurrentTimeMicroseconds'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::permissioned_signer::GrantedPermissionHandles>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_permissioned_signer_GrantedPermissionHandles'(b1), $1_from_bcs_deserializable'$1_permissioned_signer_GrantedPermissionHandles'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_GUID'(b1), $1_from_bcs_deserializable'$1_guid_GUID'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_guid_ID'(b1), $1_from_bcs_deserializable'$1_guid_ID'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::event::EventHandle<0x1::account::CoinRegisterEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::event::EventHandle<0x1::account::KeyRotationEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::event::EventHandle<0x1::reconfiguration::NewEpochEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(b1), $1_from_bcs_deserializable'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::account::Account>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_account_Account'(b1), $1_from_bcs_deserializable'$1_account_Account'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::account::CapabilityOffer<0x1::account::RotationCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(b1), $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::account::CapabilityOffer<0x1::account::SignerCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(b1), $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::account::SignerCapability>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_account_SignerCapability'(b1), $1_from_bcs_deserializable'$1_account_SignerCapability'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::reconfiguration::Configuration>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_reconfiguration_Configuration'(b1), $1_from_bcs_deserializable'$1_reconfiguration_Configuration'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::CreateTransaction>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_CreateTransaction'(b1), $1_from_bcs_deserializable'$1_timelock_CreateTransaction'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::AddCreators>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_AddCreators'(b1), $1_from_bcs_deserializable'$1_timelock_AddCreators'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::AddExecutors>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_AddExecutors'(b1), $1_from_bcs_deserializable'$1_timelock_AddExecutors'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::CancelTransaction>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_CancelTransaction'(b1), $1_from_bcs_deserializable'$1_timelock_CancelTransaction'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::RemoveCreators>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_RemoveCreators'(b1), $1_from_bcs_deserializable'$1_timelock_RemoveCreators'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::RemoveExecutors>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_RemoveExecutors'(b1), $1_from_bcs_deserializable'$1_timelock_RemoveExecutors'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::TimelockAccount>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_TimelockAccount'(b1), $1_from_bcs_deserializable'$1_timelock_TimelockAccount'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::TimelockTransaction>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_TimelockTransaction'(b1), $1_from_bcs_deserializable'$1_timelock_TimelockTransaction'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <0x1::timelock::UpdateMinNumSecondsExecute>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'$1_timelock_UpdateMinNumSecondsExecute'(b1), $1_from_bcs_deserializable'$1_timelock_UpdateMinNumSecondsExecute'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:18:9+124, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserializable'#0'(b1), $1_from_bcs_deserializable'#0'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <bool>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'bool'($1_from_bcs_deserialize'bool'(b1), $1_from_bcs_deserialize'bool'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u8>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u8'($1_from_bcs_deserialize'u8'(b1), $1_from_bcs_deserialize'u8'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u64>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u64'($1_from_bcs_deserialize'u64'(b1), $1_from_bcs_deserialize'u64'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <u256>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'u256'($1_from_bcs_deserialize'u256'(b1), $1_from_bcs_deserialize'u256'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <address>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'address'($1_from_bcs_deserialize'address'(b1), $1_from_bcs_deserialize'address'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <signer>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'signer'($1_from_bcs_deserialize'signer'(b1), $1_from_bcs_deserialize'signer'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<u8>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'u8''($1_from_bcs_deserialize'vec'u8''(b1), $1_from_bcs_deserialize'vec'u8''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'address''($1_from_bcs_deserialize'vec'address''(b1), $1_from_bcs_deserialize'vec'address''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <vector<#0>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'vec'#0''($1_from_bcs_deserialize'vec'#0''(b1), $1_from_bcs_deserialize'vec'#0''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::option::Option<address>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_option_Option'address''($1_from_bcs_deserialize'$1_option_Option'address''(b1), $1_from_bcs_deserialize'$1_option_Option'address''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::features::Features>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_features_Features'($1_from_bcs_deserialize'$1_features_Features'(b1), $1_from_bcs_deserialize'$1_features_Features'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::type_info::TypeInfo>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_type_info_TypeInfo'($1_from_bcs_deserialize'$1_type_info_TypeInfo'(b1), $1_from_bcs_deserialize'$1_type_info_TypeInfo'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::table::Table<vector<u8>, 0x1::timelock::TimelockTransaction>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''($1_from_bcs_deserialize'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(b1), $1_from_bcs_deserialize'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::chain_status::GenesisEndMarker>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_chain_status_GenesisEndMarker'($1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(b1), $1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timestamp::CurrentTimeMicroseconds>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timestamp_CurrentTimeMicroseconds'($1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(b1), $1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::permissioned_signer::GrantedPermissionHandles>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_permissioned_signer_GrantedPermissionHandles'($1_from_bcs_deserialize'$1_permissioned_signer_GrantedPermissionHandles'(b1), $1_from_bcs_deserialize'$1_permissioned_signer_GrantedPermissionHandles'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::guid::GUID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_GUID'($1_from_bcs_deserialize'$1_guid_GUID'(b1), $1_from_bcs_deserialize'$1_guid_GUID'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::guid::ID>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_guid_ID'($1_from_bcs_deserialize'$1_guid_ID'(b1), $1_from_bcs_deserialize'$1_guid_ID'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::event::EventHandle<0x1::account::CoinRegisterEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::event::EventHandle<0x1::account::KeyRotationEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::event::EventHandle<0x1::reconfiguration::NewEpochEvent>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''($1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(b1), $1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::account::Account>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_account_Account'($1_from_bcs_deserialize'$1_account_Account'(b1), $1_from_bcs_deserialize'$1_account_Account'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::account::CapabilityOffer<0x1::account::RotationCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::account::CapabilityOffer<0x1::account::SignerCapability>>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''($1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b1), $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::account::SignerCapability>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_account_SignerCapability'($1_from_bcs_deserialize'$1_account_SignerCapability'(b1), $1_from_bcs_deserialize'$1_account_SignerCapability'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::reconfiguration::Configuration>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_reconfiguration_Configuration'($1_from_bcs_deserialize'$1_reconfiguration_Configuration'(b1), $1_from_bcs_deserialize'$1_reconfiguration_Configuration'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::CreateTransaction>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_CreateTransaction'($1_from_bcs_deserialize'$1_timelock_CreateTransaction'(b1), $1_from_bcs_deserialize'$1_timelock_CreateTransaction'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::AddCreators>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_AddCreators'($1_from_bcs_deserialize'$1_timelock_AddCreators'(b1), $1_from_bcs_deserialize'$1_timelock_AddCreators'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::AddExecutors>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_AddExecutors'($1_from_bcs_deserialize'$1_timelock_AddExecutors'(b1), $1_from_bcs_deserialize'$1_timelock_AddExecutors'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::CancelTransaction>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_CancelTransaction'($1_from_bcs_deserialize'$1_timelock_CancelTransaction'(b1), $1_from_bcs_deserialize'$1_timelock_CancelTransaction'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::RemoveCreators>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_RemoveCreators'($1_from_bcs_deserialize'$1_timelock_RemoveCreators'(b1), $1_from_bcs_deserialize'$1_timelock_RemoveCreators'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::RemoveExecutors>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_RemoveExecutors'($1_from_bcs_deserialize'$1_timelock_RemoveExecutors'(b1), $1_from_bcs_deserialize'$1_timelock_RemoveExecutors'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::TimelockAccount>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_TimelockAccount'($1_from_bcs_deserialize'$1_timelock_TimelockAccount'(b1), $1_from_bcs_deserialize'$1_timelock_TimelockAccount'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::TimelockTransaction>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_TimelockTransaction'($1_from_bcs_deserialize'$1_timelock_TimelockTransaction'(b1), $1_from_bcs_deserialize'$1_timelock_TimelockTransaction'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <0x1::timelock::UpdateMinNumSecondsExecute>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'$1_timelock_UpdateMinNumSecondsExecute'($1_from_bcs_deserialize'$1_timelock_UpdateMinNumSecondsExecute'(b1), $1_from_bcs_deserialize'$1_timelock_UpdateMinNumSecondsExecute'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:21:9+118, instance <#0>
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''(b1, b2) ==> $IsEqual'#0'($1_from_bcs_deserialize'#0'(b1), $1_from_bcs_deserialize'#0'(b2)))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/permissioned_signer.spec.move:5:9+288
axiom (forall a: $1_permissioned_signer_GrantedPermissionHandles :: $IsValid'$1_permissioned_signer_GrantedPermissionHandles'(a) ==> ((var $range_0 := $Range(0, LenVec(a->$active_handles)); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var i := $i_1;
((var $range_2 := $Range(0, LenVec(a->$active_handles)); (forall $i_3: int :: $InRange($range_2, $i_3) ==> (var j := $i_3;
((!$IsEqual'num'(i, j) ==> !$IsEqual'address'(ReadVec(a->$active_handles, i), ReadVec(a->$active_handles, j)))))))))))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:8:9+113
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_keccak256(b1), $1_aptos_hash_spec_keccak256(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:13:9+129
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_sha2_512_internal(b1), $1_aptos_hash_spec_sha2_512_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:18:9+129
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_sha3_512_internal(b1), $1_aptos_hash_spec_sha3_512_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:23:9+131
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_ripemd160_internal(b1), $1_aptos_hash_spec_ripemd160_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// axiom at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:28:9+135
axiom (forall b1: Vec (int), b2: Vec (int) :: $IsValid'vec'u8''(b1) ==> $IsValid'vec'u8''(b2) ==> (($IsEqual'vec'u8''($1_aptos_hash_spec_blake2b_256_internal(b1), $1_aptos_hash_spec_blake2b_256_internal(b2)) ==> $IsEqual'vec'u8''(b1, b2))));

// struct option::Option<address> at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/option.move:7:5+81
datatype $1_option_Option'address' {
    $1_option_Option'address'($vec: Vec (int))
}
function {:inline} $Update'$1_option_Option'address''_vec(s: $1_option_Option'address', x: Vec (int)): $1_option_Option'address' {
    $1_option_Option'address'(x)
}
function $IsValid'$1_option_Option'address''(s: $1_option_Option'address'): bool {
    $IsValid'vec'address''(s->$vec)
}
function {:inline} $IsEqual'$1_option_Option'address''(s1: $1_option_Option'address', s2: $1_option_Option'address'): bool {
    $IsEqual'vec'address''(s1->$vec, s2->$vec)}

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:26:5+77
function {:inline} $1_signer_$address_of(s: $signer): int {
    $1_signer_$borrow_address(s)
}

// fun signer::address_of [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:26:5+77
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
    // trace_local[s]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:26:5+1
    assume {:print "$at(16,794,795)"} true;
    assume {:print "$track_local(4,0,0):", $t0} $t0 == $t0;

    // $t1 := signer::borrow_address($t0) on_abort goto L2 with $t2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:27:10+17
    assume {:print "$at(16,848,865)"} true;
    call $t1 := $1_signer_borrow_address($t0);
    if ($abort_flag) {
        assume {:print "$at(16,848,865)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(4,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_return[0]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:27:9+18
    assume {:print "$track_return(4,0,0):", $t1} $t1 == $t1;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:28:5+1
    assume {:print "$at(16,870,871)"} true;
L1:

    // return $t1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:28:5+1
    assume {:print "$at(16,870,871)"} true;
    $ret0 := $t1;
    return;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:28:5+1
L2:

    // abort($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/signer.move:28:5+1
    assume {:print "$at(16,870,871)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun error::already_exists [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:3+71
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
    // trace_local[r]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:3+1
    assume {:print "$at(11,3585,3586)"} true;
    assume {:print "$track_local(5,1,0):", $t0} $t0 == $t0;

    // $t1 := 8 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:54+14
    $t1 := 8;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(11,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume {:print "$at(11,3626,3654)"} true;

    // assume WellFormed($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:44+28
    assume {:print "$track_return(5,1,0):", $t3} $t3 == $t3;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:73+1
L1:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:83:73+1
    assume {:print "$at(11,3655,3656)"} true;
    $ret0 := $t3;
    return;

}

// fun error::invalid_argument [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:3+76
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
    // trace_local[r]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:3+1
    assume {:print "$at(11,3082,3083)"} true;
    assume {:print "$track_local(5,4,0):", $t0} $t0 == $t0;

    // $t1 := 1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:57+16
    $t1 := 1;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(11,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume {:print "$at(11,3126,3156)"} true;

    // assume WellFormed($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:47+30
    assume {:print "$track_return(5,4,0):", $t3} $t3 == $t3;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:78+1
L1:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:76:78+1
    assume {:print "$at(11,3157,3158)"} true;
    $ret0 := $t3;
    return;

}

// fun error::invalid_state [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:3+70
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
    // trace_local[r]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:3+1
    assume {:print "$at(11,3232,3233)"} true;
    assume {:print "$track_local(5,5,0):", $t0} $t0 == $t0;

    // $t1 := 3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:54+13
    $t1 := 3;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(11,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume {:print "$at(11,3273,3300)"} true;

    // assume WellFormed($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:44+27
    assume {:print "$track_return(5,5,0):", $t3} $t3 == $t3;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:72+1
L1:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:78:72+1
    assume {:print "$at(11,3301,3302)"} true;
    $ret0 := $t3;
    return;

}

// fun error::not_found [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+61
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
    // trace_local[r]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:3+1
    assume {:print "$at(11,3461,3462)"} true;
    assume {:print "$track_local(5,6,0):", $t0} $t0 == $t0;

    // $t1 := 6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:49+9
    $t1 := 6;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(11,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$at(11,3497,3520)"} true;

    // assume WellFormed($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:39+23
    assume {:print "$track_return(5,6,0):", $t3} $t3 == $t3;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
L1:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:81:63+1
    assume {:print "$at(11,3521,3522)"} true;
    $ret0 := $t3;
    return;

}

// fun error::permission_denied [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:3+77
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
    // trace_local[r]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:3+1
    assume {:print "$at(11,3381,3382)"} true;
    assume {:print "$track_local(5,9,0):", $t0} $t0 == $t0;

    // $t1 := 5 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:57+17
    $t1 := 5;
    assume $IsValid'u64'($t1);

    // assume Identical($t2, Shl($t1, 16)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:69:5+29
    assume {:print "$at(11,2844,2873)"} true;
    assume ($t2 == $shlU64($t1, 16));

    // $t3 := opaque begin: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume {:print "$at(11,3425,3456)"} true;

    // assume WellFormed($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume $IsValid'u64'($t3);

    // assume Eq<u64>($t3, $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume $IsEqual'u64'($t3, $t1);

    // $t3 := opaque end: error::canonical($t1, $t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:47+31
    assume {:print "$track_return(5,9,0):", $t3} $t3 == $t3;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:79+1
L1:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/error.move:80:79+1
    assume {:print "$at(11,3457,3458)"} true;
    $ret0 := $t3;
    return;

}

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.spec.move:61:10+40
function  $1_features_spec_is_enabled(feature: int): bool;
axiom (forall feature: int ::
(var $$res := $1_features_spec_is_enabled(feature);
$IsValid'bool'($$res)));

// struct features::Features at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/../move-stdlib/sources/configs/features.move:800:5+61
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

// struct type_info::TypeInfo at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/type_info.move:19:5+145
datatype $1_type_info_TypeInfo {
    $1_type_info_TypeInfo($account_address: int, $module_name: Vec (int), $struct_name: Vec (int))
}
function {:inline} $Update'$1_type_info_TypeInfo'_account_address(s: $1_type_info_TypeInfo, x: int): $1_type_info_TypeInfo {
    $1_type_info_TypeInfo(x, s->$module_name, s->$struct_name)
}
function {:inline} $Update'$1_type_info_TypeInfo'_module_name(s: $1_type_info_TypeInfo, x: Vec (int)): $1_type_info_TypeInfo {
    $1_type_info_TypeInfo(s->$account_address, x, s->$struct_name)
}
function {:inline} $Update'$1_type_info_TypeInfo'_struct_name(s: $1_type_info_TypeInfo, x: Vec (int)): $1_type_info_TypeInfo {
    $1_type_info_TypeInfo(s->$account_address, s->$module_name, x)
}
function $IsValid'$1_type_info_TypeInfo'(s: $1_type_info_TypeInfo): bool {
    $IsValid'address'(s->$account_address)
      && $IsValid'vec'u8''(s->$module_name)
      && $IsValid'vec'u8''(s->$struct_name)
}
function {:inline} $IsEqual'$1_type_info_TypeInfo'(s1: $1_type_info_TypeInfo, s2: $1_type_info_TypeInfo): bool {
    $IsEqual'address'(s1->$account_address, s2->$account_address)
    && $IsEqual'vec'u8''(s1->$module_name, s2->$module_name)
    && $IsEqual'vec'u8''(s1->$struct_name, s2->$struct_name)}

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u8'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u8'(bytes);
$IsValid'u8'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u64'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u64'(bytes);
$IsValid'u64'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'u256'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'u256'(bytes);
$IsValid'u256'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'address'(bytes: Vec (int)): int;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'address'(bytes);
$IsValid'address'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'signer'(bytes: Vec (int)): $signer;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'signer'(bytes);
$IsValid'signer'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'u8''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'u8''(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'address''(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'address''(bytes);
$IsValid'vec'address''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'vec'#0''(bytes: Vec (int)): Vec (#0);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'vec'#0''(bytes);
$IsValid'vec'#0''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_option_Option'address''(bytes: Vec (int)): $1_option_Option'address';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_option_Option'address''(bytes);
$IsValid'$1_option_Option'address''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_features_Features'(bytes: Vec (int)): $1_features_Features;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_features_Features'(bytes);
$IsValid'$1_features_Features'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_type_info_TypeInfo'(bytes: Vec (int)): $1_type_info_TypeInfo;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_type_info_TypeInfo'(bytes);
$IsValid'$1_type_info_TypeInfo'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(bytes: Vec (int)): Table int ($1_timelock_TimelockTransaction);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(bytes);
$IsValid'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(bytes: Vec (int)): $1_chain_status_GenesisEndMarker;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_chain_status_GenesisEndMarker'(bytes);
$IsValid'$1_chain_status_GenesisEndMarker'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(bytes: Vec (int)): $1_timestamp_CurrentTimeMicroseconds;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timestamp_CurrentTimeMicroseconds'(bytes);
$IsValid'$1_timestamp_CurrentTimeMicroseconds'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_permissioned_signer_GrantedPermissionHandles'(bytes: Vec (int)): $1_permissioned_signer_GrantedPermissionHandles;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_permissioned_signer_GrantedPermissionHandles'(bytes);
$IsValid'$1_permissioned_signer_GrantedPermissionHandles'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_GUID'(bytes: Vec (int)): $1_guid_GUID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_GUID'(bytes);
$IsValid'$1_guid_GUID'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_guid_ID'(bytes: Vec (int)): $1_guid_ID;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_guid_ID'(bytes);
$IsValid'$1_guid_ID'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_account_CoinRegisterEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_account_CoinRegisterEvent''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_account_KeyRotationEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_account_KeyRotationEvent''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(bytes: Vec (int)): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(bytes);
$IsValid'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_Account'(bytes: Vec (int)): $1_account_Account;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_Account'(bytes);
$IsValid'$1_account_Account'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes: Vec (int)): $1_account_CapabilityOffer'$1_account_RotationCapability';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes);
$IsValid'$1_account_CapabilityOffer'$1_account_RotationCapability''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes: Vec (int)): $1_account_CapabilityOffer'$1_account_SignerCapability';
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes);
$IsValid'$1_account_CapabilityOffer'$1_account_SignerCapability''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_account_SignerCapability'(bytes: Vec (int)): $1_account_SignerCapability;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_account_SignerCapability'(bytes);
$IsValid'$1_account_SignerCapability'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_reconfiguration_Configuration'(bytes: Vec (int)): $1_reconfiguration_Configuration;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_reconfiguration_Configuration'(bytes);
$IsValid'$1_reconfiguration_Configuration'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_CreateTransaction'(bytes: Vec (int)): $1_timelock_CreateTransaction;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_CreateTransaction'(bytes);
$IsValid'$1_timelock_CreateTransaction'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_AddCreators'(bytes: Vec (int)): $1_timelock_AddCreators;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_AddCreators'(bytes);
$IsValid'$1_timelock_AddCreators'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_AddExecutors'(bytes: Vec (int)): $1_timelock_AddExecutors;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_AddExecutors'(bytes);
$IsValid'$1_timelock_AddExecutors'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_CancelTransaction'(bytes: Vec (int)): $1_timelock_CancelTransaction;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_CancelTransaction'(bytes);
$IsValid'$1_timelock_CancelTransaction'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_RemoveCreators'(bytes: Vec (int)): $1_timelock_RemoveCreators;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_RemoveCreators'(bytes);
$IsValid'$1_timelock_RemoveCreators'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_RemoveExecutors'(bytes: Vec (int)): $1_timelock_RemoveExecutors;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_RemoveExecutors'(bytes);
$IsValid'$1_timelock_RemoveExecutors'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_TimelockAccount'(bytes: Vec (int)): $1_timelock_TimelockAccount;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_TimelockAccount'(bytes);
$IsValid'$1_timelock_TimelockAccount'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_TimelockTransaction'(bytes: Vec (int)): $1_timelock_TimelockTransaction;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_TimelockTransaction'(bytes);
$IsValid'$1_timelock_TimelockTransaction'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'$1_timelock_UpdateMinNumSecondsExecute'(bytes: Vec (int)): $1_timelock_UpdateMinNumSecondsExecute;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'$1_timelock_UpdateMinNumSecondsExecute'(bytes);
$IsValid'$1_timelock_UpdateMinNumSecondsExecute'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:7:9+41
function  $1_from_bcs_deserialize'#0'(bytes: Vec (int)): #0;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserialize'#0'(bytes);
$IsValid'#0'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'bool'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'bool'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u8'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u8'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u64'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u64'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'u256'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'u256'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'address'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'address'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'signer'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'signer'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'u8''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'u8''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'address''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'address''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'vec'#0''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'vec'#0''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_option_Option'address''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_option_Option'address''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_features_Features'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_features_Features'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_type_info_TypeInfo'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_type_info_TypeInfo'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_chain_status_GenesisEndMarker'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_chain_status_GenesisEndMarker'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timestamp_CurrentTimeMicroseconds'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timestamp_CurrentTimeMicroseconds'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_permissioned_signer_GrantedPermissionHandles'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_permissioned_signer_GrantedPermissionHandles'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_GUID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_GUID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_guid_ID'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_guid_ID'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_CoinRegisterEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_account_KeyRotationEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_account_Account'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_account_Account'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_RotationCapability''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_account_CapabilityOffer'$1_account_SignerCapability''(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_account_SignerCapability'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_account_SignerCapability'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_reconfiguration_Configuration'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_reconfiguration_Configuration'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_CreateTransaction'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_CreateTransaction'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_AddCreators'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_AddCreators'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_AddExecutors'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_AddExecutors'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_CancelTransaction'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_CancelTransaction'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_RemoveCreators'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_RemoveCreators'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_RemoveExecutors'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_RemoveExecutors'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_TimelockAccount'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_TimelockAccount'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_TimelockTransaction'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_TimelockTransaction'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'$1_timelock_UpdateMinNumSecondsExecute'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'$1_timelock_UpdateMinNumSecondsExecute'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/from_bcs.spec.move:11:9+47
function  $1_from_bcs_deserializable'#0'(bytes: Vec (int)): bool;
axiom (forall bytes: Vec (int) ::
(var $$res := $1_from_bcs_deserializable'#0'(bytes);
$IsValid'bool'($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/chain_status.move:35:5+90
function {:inline} $1_chain_status_$is_operating($1_chain_status_GenesisEndMarker_$memory: $Memory $1_chain_status_GenesisEndMarker): bool {
    $ResourceExists($1_chain_status_GenesisEndMarker_$memory, 1)
}

// struct chain_status::GenesisEndMarker at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/chain_status.move:12:5+34
datatype $1_chain_status_GenesisEndMarker {
    $1_chain_status_GenesisEndMarker($dummy_field: bool)
}
function {:inline} $Update'$1_chain_status_GenesisEndMarker'_dummy_field(s: $1_chain_status_GenesisEndMarker, x: bool): $1_chain_status_GenesisEndMarker {
    $1_chain_status_GenesisEndMarker(x)
}
function $IsValid'$1_chain_status_GenesisEndMarker'(s: $1_chain_status_GenesisEndMarker): bool {
    $IsValid'bool'(s->$dummy_field)
}
function {:inline} $IsEqual'$1_chain_status_GenesisEndMarker'(s1: $1_chain_status_GenesisEndMarker, s2: $1_chain_status_GenesisEndMarker): bool {
    s1 == s2
}
var $1_chain_status_GenesisEndMarker_$memory: $Memory $1_chain_status_GenesisEndMarker;

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.spec.move:57:10+111
function {:inline} $1_timestamp_spec_now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds): int {
    $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, 1)->$microseconds
}

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:61:5+153
function {:inline} $1_timestamp_$now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds): int {
    $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, 1)->$microseconds
}

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:67:5+123
function {:inline} $1_timestamp_$now_seconds($1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds): int {
    ($1_timestamp_$now_microseconds($1_timestamp_CurrentTimeMicroseconds_$memory) div 1000000)
}

// struct timestamp::CurrentTimeMicroseconds at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:12:5+73
datatype $1_timestamp_CurrentTimeMicroseconds {
    $1_timestamp_CurrentTimeMicroseconds($microseconds: int)
}
function {:inline} $Update'$1_timestamp_CurrentTimeMicroseconds'_microseconds(s: $1_timestamp_CurrentTimeMicroseconds, x: int): $1_timestamp_CurrentTimeMicroseconds {
    $1_timestamp_CurrentTimeMicroseconds(x)
}
function $IsValid'$1_timestamp_CurrentTimeMicroseconds'(s: $1_timestamp_CurrentTimeMicroseconds): bool {
    $IsValid'u64'(s->$microseconds)
}
function {:inline} $IsEqual'$1_timestamp_CurrentTimeMicroseconds'(s1: $1_timestamp_CurrentTimeMicroseconds, s2: $1_timestamp_CurrentTimeMicroseconds): bool {
    s1 == s2
}
var $1_timestamp_CurrentTimeMicroseconds_$memory: $Memory $1_timestamp_CurrentTimeMicroseconds;

// fun timestamp::now_microseconds [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:61:5+153
procedure {:inline 1} $1_timestamp_now_microseconds() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: $1_timestamp_CurrentTimeMicroseconds;
    var $t2: int;
    var $t3: int;
    var $temp_0'u64': int;

    // bytecode translation starts here
    // $t0 := 0x1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:48+16
    assume {:print "$at(220,2511,2527)"} true;
    $t0 := 1;
    assume $IsValid'address'($t0);

    // $t1 := get_global<0x1::timestamp::CurrentTimeMicroseconds>($t0) on_abort goto L2 with $t2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:9+56
    if (!$ResourceExists($1_timestamp_CurrentTimeMicroseconds_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t1 := $ResourceValue($1_timestamp_CurrentTimeMicroseconds_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(220,2472,2528)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(22,0):", $t2} $t2 == $t2;
        goto L2;
    }

    // $t3 := get_field<0x1::timestamp::CurrentTimeMicroseconds>.microseconds($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:9+69
    $t3 := $t1->$microseconds;

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:62:9+69
    assume {:print "$track_return(22,0,0):", $t3} $t3 == $t3;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
    assume {:print "$at(220,2546,2547)"} true;
L1:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
    assume {:print "$at(220,2546,2547)"} true;
    $ret0 := $t3;
    return;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
L2:

    // abort($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:63:5+1
    assume {:print "$at(220,2546,2547)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun timestamp::now_seconds [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:67:5+123
procedure {:inline 1} $1_timestamp_now_seconds() returns ($ret0: int)
{
    // declare local variables
    var $t0: int;
    var $t1: int;
    var $t2: int;
    var $t3: int;
    var $temp_0'u64': int;

    // bytecode translation starts here
    // $t0 := timestamp::now_microseconds() on_abort goto L2 with $t1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:9+18
    assume {:print "$at(220,2680,2698)"} true;
    call $t0 := $1_timestamp_now_microseconds();
    if ($abort_flag) {
        assume {:print "$at(220,2680,2698)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(22,1):", $t1} $t1 == $t1;
        goto L2;
    }

    // $t2 := 1000000 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:30+23
    $t2 := 1000000;
    assume $IsValid'u64'($t2);

    // $t3 := /($t0, $t2) on_abort goto L2 with $t1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:9+44
    call $t3 := $Div($t0, $t2);
    if ($abort_flag) {
        assume {:print "$at(220,2680,2724)"} true;
        $t1 := $abort_code;
        assume {:print "$track_abort(22,1):", $t1} $t1 == $t1;
        goto L2;
    }

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:68:9+44
    assume {:print "$track_return(22,1,0):", $t3} $t3 == $t3;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
    assume {:print "$at(220,2729,2730)"} true;
L1:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
    assume {:print "$at(220,2729,2730)"} true;
    $ret0 := $t3;
    return;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
L2:

    // abort($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timestamp.move:69:5+1
    assume {:print "$at(220,2729,2730)"} true;
    $abort_code := $t1;
    $abort_flag := true;
    return;

}

// struct permissioned_signer::GrantedPermissionHandles at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/permissioned_signer.move:64:5+188
datatype $1_permissioned_signer_GrantedPermissionHandles {
    $1_permissioned_signer_GrantedPermissionHandles($active_handles: Vec (int))
}
function {:inline} $Update'$1_permissioned_signer_GrantedPermissionHandles'_active_handles(s: $1_permissioned_signer_GrantedPermissionHandles, x: Vec (int)): $1_permissioned_signer_GrantedPermissionHandles {
    $1_permissioned_signer_GrantedPermissionHandles(x)
}
function $IsValid'$1_permissioned_signer_GrantedPermissionHandles'(s: $1_permissioned_signer_GrantedPermissionHandles): bool {
    $IsValid'vec'address''(s->$active_handles)
}
function {:inline} $IsEqual'$1_permissioned_signer_GrantedPermissionHandles'(s1: $1_permissioned_signer_GrantedPermissionHandles, s2: $1_permissioned_signer_GrantedPermissionHandles): bool {
    $IsEqual'vec'address''(s1->$active_handles, s2->$active_handles)}
var $1_permissioned_signer_GrantedPermissionHandles_$memory: $Memory $1_permissioned_signer_GrantedPermissionHandles;

// struct guid::GUID at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:7:5+50
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

// struct guid::ID at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/guid.move:12:5+209
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

// struct event::EventHandle<0x1::account::CoinRegisterEvent> at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:37:5+224
datatype $1_event_EventHandle'$1_account_CoinRegisterEvent' {
    $1_event_EventHandle'$1_account_CoinRegisterEvent'($counter: int, $guid: $1_guid_GUID)
}
function {:inline} $Update'$1_event_EventHandle'$1_account_CoinRegisterEvent''_counter(s: $1_event_EventHandle'$1_account_CoinRegisterEvent', x: int): $1_event_EventHandle'$1_account_CoinRegisterEvent' {
    $1_event_EventHandle'$1_account_CoinRegisterEvent'(x, s->$guid)
}
function {:inline} $Update'$1_event_EventHandle'$1_account_CoinRegisterEvent''_guid(s: $1_event_EventHandle'$1_account_CoinRegisterEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_account_CoinRegisterEvent' {
    $1_event_EventHandle'$1_account_CoinRegisterEvent'(s->$counter, x)
}
function $IsValid'$1_event_EventHandle'$1_account_CoinRegisterEvent''(s: $1_event_EventHandle'$1_account_CoinRegisterEvent'): bool {
    $IsValid'u64'(s->$counter)
      && $IsValid'$1_guid_GUID'(s->$guid)
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''(s1: $1_event_EventHandle'$1_account_CoinRegisterEvent', s2: $1_event_EventHandle'$1_account_CoinRegisterEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<0x1::account::KeyRotationEvent> at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:37:5+224
datatype $1_event_EventHandle'$1_account_KeyRotationEvent' {
    $1_event_EventHandle'$1_account_KeyRotationEvent'($counter: int, $guid: $1_guid_GUID)
}
function {:inline} $Update'$1_event_EventHandle'$1_account_KeyRotationEvent''_counter(s: $1_event_EventHandle'$1_account_KeyRotationEvent', x: int): $1_event_EventHandle'$1_account_KeyRotationEvent' {
    $1_event_EventHandle'$1_account_KeyRotationEvent'(x, s->$guid)
}
function {:inline} $Update'$1_event_EventHandle'$1_account_KeyRotationEvent''_guid(s: $1_event_EventHandle'$1_account_KeyRotationEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_account_KeyRotationEvent' {
    $1_event_EventHandle'$1_account_KeyRotationEvent'(s->$counter, x)
}
function $IsValid'$1_event_EventHandle'$1_account_KeyRotationEvent''(s: $1_event_EventHandle'$1_account_KeyRotationEvent'): bool {
    $IsValid'u64'(s->$counter)
      && $IsValid'$1_guid_GUID'(s->$guid)
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''(s1: $1_event_EventHandle'$1_account_KeyRotationEvent', s2: $1_event_EventHandle'$1_account_KeyRotationEvent'): bool {
    s1 == s2
}

// struct event::EventHandle<0x1::reconfiguration::NewEpochEvent> at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/event.move:37:5+224
datatype $1_event_EventHandle'$1_reconfiguration_NewEpochEvent' {
    $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'($counter: int, $guid: $1_guid_GUID)
}
function {:inline} $Update'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''_counter(s: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent', x: int): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent' {
    $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'(x, s->$guid)
}
function {:inline} $Update'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''_guid(s: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent', x: $1_guid_GUID): $1_event_EventHandle'$1_reconfiguration_NewEpochEvent' {
    $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'(s->$counter, x)
}
function $IsValid'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(s: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'): bool {
    $IsValid'u64'(s->$counter)
      && $IsValid'$1_guid_GUID'(s->$guid)
}
function {:inline} $IsEqual'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(s1: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent', s2: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'): bool {
    s1 == s2
}

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.spec.move:598:10+77
function  $1_account_spec_create_resource_address(source: int, seed: Vec (int)): int;
axiom (forall source: int, seed: Vec (int) ::
(var $$res := $1_account_spec_create_resource_address(source, seed);
$IsValid'address'($$res)));

// struct account::Account at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:61:5+401
datatype $1_account_Account {
    $1_account_Account($authentication_key: Vec (int), $sequence_number: int, $guid_creation_num: int, $coin_register_events: $1_event_EventHandle'$1_account_CoinRegisterEvent', $key_rotation_events: $1_event_EventHandle'$1_account_KeyRotationEvent', $rotation_capability_offer: $1_account_CapabilityOffer'$1_account_RotationCapability', $signer_capability_offer: $1_account_CapabilityOffer'$1_account_SignerCapability')
}
function {:inline} $Update'$1_account_Account'_authentication_key(s: $1_account_Account, x: Vec (int)): $1_account_Account {
    $1_account_Account(x, s->$sequence_number, s->$guid_creation_num, s->$coin_register_events, s->$key_rotation_events, s->$rotation_capability_offer, s->$signer_capability_offer)
}
function {:inline} $Update'$1_account_Account'_sequence_number(s: $1_account_Account, x: int): $1_account_Account {
    $1_account_Account(s->$authentication_key, x, s->$guid_creation_num, s->$coin_register_events, s->$key_rotation_events, s->$rotation_capability_offer, s->$signer_capability_offer)
}
function {:inline} $Update'$1_account_Account'_guid_creation_num(s: $1_account_Account, x: int): $1_account_Account {
    $1_account_Account(s->$authentication_key, s->$sequence_number, x, s->$coin_register_events, s->$key_rotation_events, s->$rotation_capability_offer, s->$signer_capability_offer)
}
function {:inline} $Update'$1_account_Account'_coin_register_events(s: $1_account_Account, x: $1_event_EventHandle'$1_account_CoinRegisterEvent'): $1_account_Account {
    $1_account_Account(s->$authentication_key, s->$sequence_number, s->$guid_creation_num, x, s->$key_rotation_events, s->$rotation_capability_offer, s->$signer_capability_offer)
}
function {:inline} $Update'$1_account_Account'_key_rotation_events(s: $1_account_Account, x: $1_event_EventHandle'$1_account_KeyRotationEvent'): $1_account_Account {
    $1_account_Account(s->$authentication_key, s->$sequence_number, s->$guid_creation_num, s->$coin_register_events, x, s->$rotation_capability_offer, s->$signer_capability_offer)
}
function {:inline} $Update'$1_account_Account'_rotation_capability_offer(s: $1_account_Account, x: $1_account_CapabilityOffer'$1_account_RotationCapability'): $1_account_Account {
    $1_account_Account(s->$authentication_key, s->$sequence_number, s->$guid_creation_num, s->$coin_register_events, s->$key_rotation_events, x, s->$signer_capability_offer)
}
function {:inline} $Update'$1_account_Account'_signer_capability_offer(s: $1_account_Account, x: $1_account_CapabilityOffer'$1_account_SignerCapability'): $1_account_Account {
    $1_account_Account(s->$authentication_key, s->$sequence_number, s->$guid_creation_num, s->$coin_register_events, s->$key_rotation_events, s->$rotation_capability_offer, x)
}
function $IsValid'$1_account_Account'(s: $1_account_Account): bool {
    $IsValid'vec'u8''(s->$authentication_key)
      && $IsValid'u64'(s->$sequence_number)
      && $IsValid'u64'(s->$guid_creation_num)
      && $IsValid'$1_event_EventHandle'$1_account_CoinRegisterEvent''(s->$coin_register_events)
      && $IsValid'$1_event_EventHandle'$1_account_KeyRotationEvent''(s->$key_rotation_events)
      && $IsValid'$1_account_CapabilityOffer'$1_account_RotationCapability''(s->$rotation_capability_offer)
      && $IsValid'$1_account_CapabilityOffer'$1_account_SignerCapability''(s->$signer_capability_offer)
}
function {:inline} $IsEqual'$1_account_Account'(s1: $1_account_Account, s2: $1_account_Account): bool {
    $IsEqual'vec'u8''(s1->$authentication_key, s2->$authentication_key)
    && $IsEqual'u64'(s1->$sequence_number, s2->$sequence_number)
    && $IsEqual'u64'(s1->$guid_creation_num, s2->$guid_creation_num)
    && $IsEqual'$1_event_EventHandle'$1_account_CoinRegisterEvent''(s1->$coin_register_events, s2->$coin_register_events)
    && $IsEqual'$1_event_EventHandle'$1_account_KeyRotationEvent''(s1->$key_rotation_events, s2->$key_rotation_events)
    && $IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''(s1->$rotation_capability_offer, s2->$rotation_capability_offer)
    && $IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''(s1->$signer_capability_offer, s2->$signer_capability_offer)}
var $1_account_Account_$memory: $Memory $1_account_Account;

// struct account::CapabilityOffer<0x1::account::RotationCapability> at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:86:5+68
datatype $1_account_CapabilityOffer'$1_account_RotationCapability' {
    $1_account_CapabilityOffer'$1_account_RotationCapability'($for: $1_option_Option'address')
}
function {:inline} $Update'$1_account_CapabilityOffer'$1_account_RotationCapability''_for(s: $1_account_CapabilityOffer'$1_account_RotationCapability', x: $1_option_Option'address'): $1_account_CapabilityOffer'$1_account_RotationCapability' {
    $1_account_CapabilityOffer'$1_account_RotationCapability'(x)
}
function $IsValid'$1_account_CapabilityOffer'$1_account_RotationCapability''(s: $1_account_CapabilityOffer'$1_account_RotationCapability'): bool {
    $IsValid'$1_option_Option'address''(s->$for)
}
function {:inline} $IsEqual'$1_account_CapabilityOffer'$1_account_RotationCapability''(s1: $1_account_CapabilityOffer'$1_account_RotationCapability', s2: $1_account_CapabilityOffer'$1_account_RotationCapability'): bool {
    $IsEqual'$1_option_Option'address''(s1->$for, s2->$for)}

// struct account::CapabilityOffer<0x1::account::SignerCapability> at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:86:5+68
datatype $1_account_CapabilityOffer'$1_account_SignerCapability' {
    $1_account_CapabilityOffer'$1_account_SignerCapability'($for: $1_option_Option'address')
}
function {:inline} $Update'$1_account_CapabilityOffer'$1_account_SignerCapability''_for(s: $1_account_CapabilityOffer'$1_account_SignerCapability', x: $1_option_Option'address'): $1_account_CapabilityOffer'$1_account_SignerCapability' {
    $1_account_CapabilityOffer'$1_account_SignerCapability'(x)
}
function $IsValid'$1_account_CapabilityOffer'$1_account_SignerCapability''(s: $1_account_CapabilityOffer'$1_account_SignerCapability'): bool {
    $IsValid'$1_option_Option'address''(s->$for)
}
function {:inline} $IsEqual'$1_account_CapabilityOffer'$1_account_SignerCapability''(s1: $1_account_CapabilityOffer'$1_account_SignerCapability', s2: $1_account_CapabilityOffer'$1_account_SignerCapability'): bool {
    $IsEqual'$1_option_Option'address''(s1->$for, s2->$for)}

// struct account::CoinRegisterEvent at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:76:5+77
datatype $1_account_CoinRegisterEvent {
    $1_account_CoinRegisterEvent($type_info: $1_type_info_TypeInfo)
}
function {:inline} $Update'$1_account_CoinRegisterEvent'_type_info(s: $1_account_CoinRegisterEvent, x: $1_type_info_TypeInfo): $1_account_CoinRegisterEvent {
    $1_account_CoinRegisterEvent(x)
}
function $IsValid'$1_account_CoinRegisterEvent'(s: $1_account_CoinRegisterEvent): bool {
    $IsValid'$1_type_info_TypeInfo'(s->$type_info)
}
function {:inline} $IsEqual'$1_account_CoinRegisterEvent'(s1: $1_account_CoinRegisterEvent, s2: $1_account_CoinRegisterEvent): bool {
    $IsEqual'$1_type_info_TypeInfo'(s1->$type_info, s2->$type_info)}

// struct account::KeyRotationEvent at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:71:5+135
datatype $1_account_KeyRotationEvent {
    $1_account_KeyRotationEvent($old_authentication_key: Vec (int), $new_authentication_key: Vec (int))
}
function {:inline} $Update'$1_account_KeyRotationEvent'_old_authentication_key(s: $1_account_KeyRotationEvent, x: Vec (int)): $1_account_KeyRotationEvent {
    $1_account_KeyRotationEvent(x, s->$new_authentication_key)
}
function {:inline} $Update'$1_account_KeyRotationEvent'_new_authentication_key(s: $1_account_KeyRotationEvent, x: Vec (int)): $1_account_KeyRotationEvent {
    $1_account_KeyRotationEvent(s->$old_authentication_key, x)
}
function $IsValid'$1_account_KeyRotationEvent'(s: $1_account_KeyRotationEvent): bool {
    $IsValid'vec'u8''(s->$old_authentication_key)
      && $IsValid'vec'u8''(s->$new_authentication_key)
}
function {:inline} $IsEqual'$1_account_KeyRotationEvent'(s1: $1_account_KeyRotationEvent, s2: $1_account_KeyRotationEvent): bool {
    $IsEqual'vec'u8''(s1->$old_authentication_key, s2->$old_authentication_key)
    && $IsEqual'vec'u8''(s1->$new_authentication_key, s2->$new_authentication_key)}

// struct account::RotationCapability at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:88:5+62
datatype $1_account_RotationCapability {
    $1_account_RotationCapability($account: int)
}
function {:inline} $Update'$1_account_RotationCapability'_account(s: $1_account_RotationCapability, x: int): $1_account_RotationCapability {
    $1_account_RotationCapability(x)
}
function $IsValid'$1_account_RotationCapability'(s: $1_account_RotationCapability): bool {
    $IsValid'address'(s->$account)
}
function {:inline} $IsEqual'$1_account_RotationCapability'(s1: $1_account_RotationCapability, s2: $1_account_RotationCapability): bool {
    s1 == s2
}

// struct account::SignerCapability at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:90:5+60
datatype $1_account_SignerCapability {
    $1_account_SignerCapability($account: int)
}
function {:inline} $Update'$1_account_SignerCapability'_account(s: $1_account_SignerCapability, x: int): $1_account_SignerCapability {
    $1_account_SignerCapability(x)
}
function $IsValid'$1_account_SignerCapability'(s: $1_account_SignerCapability): bool {
    $IsValid'address'(s->$account)
}
function {:inline} $IsEqual'$1_account_SignerCapability'(s1: $1_account_SignerCapability, s2: $1_account_SignerCapability): bool {
    s1 == s2
}

// fun account::get_sequence_number [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:384:5+328
procedure {:inline 1} $1_account_get_sequence_number(_$t0: int) returns ($ret0: int)
{
    // declare local variables
    var $t1: int;
    var $t2: bool;
    var $t3: $1_account_Account;
    var $t4: int;
    var $t5: int;
    var $t6: bool;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t0: int;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[addr]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:384:5+1
    assume {:print "$at(98,18599,18600)"} true;
    assume {:print "$track_local(39,16,0):", $t0} $t0 == $t0;

    // $t2 := exists<0x1::account::Account>($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:360:9+21
    assume {:print "$at(98,17757,17778)"} true;
    $t2 := $ResourceExists($1_account_Account_$memory, $t0);

    // if ($t2) goto L1 else goto L0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:385:9+244
    assume {:print "$at(98,18677,18921)"} true;
    if ($t2) { goto L1; } else { goto L0; }

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:386:13+13
    assume {:print "$at(98,18721,18734)"} true;
L1:

    // $t3 := get_global<0x1::account::Account>($t0) on_abort goto L6 with $t4 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:386:13+13
    assume {:print "$at(98,18721,18734)"} true;
    if (!$ResourceExists($1_account_Account_$memory, $t0)) {
        call $ExecFailureAbort();
    } else {
        $t3 := $ResourceValue($1_account_Account_$memory, $t0);
    }
    if ($abort_flag) {
        assume {:print "$at(98,18721,18734)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(39,16):", $t4} $t4 == $t4;
        goto L6;
    }

    // $t5 := get_field<0x1::account::Account>.sequence_number($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:386:13+29
    $t5 := $t3->$sequence_number;

    // $t1 := $t5 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:386:13+29
    $t1 := $t5;

    // trace_local[return]($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:386:13+29
    assume {:print "$track_local(39,16,1):", $t5} $t5 == $t5;

    // label L4 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:385:9+244
    assume {:print "$at(98,18677,18921)"} true;
L4:

    // trace_return[0]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:385:9+244
    assume {:print "$at(98,18677,18921)"} true;
    assume {:print "$track_return(39,16,0):", $t1} $t1 == $t1;

    // goto L5 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:385:9+244
    goto L5;

    // label L0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:387:20+47
    assume {:print "$at(98,18770,18817)"} true;
L0:

    // $t6 := opaque begin: features::is_default_account_resource_enabled() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:387:20+47
    assume {:print "$at(98,18770,18817)"} true;

    // assume WellFormed($t6) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:387:20+47
    assume $IsValid'bool'($t6);

    // assume Eq<bool>($t6, features::spec_is_enabled(91)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:387:20+47
    assume $IsEqual'bool'($t6, $1_features_spec_is_enabled(91));

    // $t6 := opaque end: features::is_default_account_resource_enabled() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:387:20+47

    // if ($t6) goto L3 else goto L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:387:16+155
    if ($t6) { goto L3; } else { goto L2; }

    // label L3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:388:13+1
    assume {:print "$at(98,18833,18834)"} true;
L3:

    // $t7 := 0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:388:13+1
    assume {:print "$at(98,18833,18834)"} true;
    $t7 := 0;
    assume $IsValid'u64'($t7);

    // $t1 := $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:388:13+1
    $t1 := $t7;

    // trace_local[return]($t7) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:388:13+1
    assume {:print "$track_local(39,16,1):", $t7} $t7 == $t7;

    // goto L4 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:388:13+1
    goto L4;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:390:36+23
    assume {:print "$at(98,18887,18910)"} true;
L2:

    // $t8 := 2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:390:36+23
    assume {:print "$at(98,18887,18910)"} true;
    $t8 := 2;
    assume $IsValid'u64'($t8);

    // $t9 := error::not_found($t8) on_abort goto L6 with $t4 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:390:19+41
    call $t9 := $1_error_not_found($t8);
    if ($abort_flag) {
        assume {:print "$at(98,18870,18911)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(39,16):", $t4} $t4 == $t4;
        goto L6;
    }

    // trace_abort($t9) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:390:13+47
    assume {:print "$at(98,18864,18911)"} true;
    assume {:print "$track_abort(39,16):", $t9} $t9 == $t9;

    // $t4 := move($t9) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:390:13+47
    $t4 := $t9;

    // goto L6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:390:13+47
    goto L6;

    // label L5 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:392:5+1
    assume {:print "$at(98,18926,18927)"} true;
L5:

    // return $t1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:392:5+1
    assume {:print "$at(98,18926,18927)"} true;
    $ret0 := $t1;
    return;

    // label L6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:392:5+1
L6:

    // abort($t4) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/account/account.move:392:5+1
    assume {:print "$at(98,18926,18927)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:7:9+50
function  $1_aptos_hash_spec_keccak256(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_keccak256(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:12:9+58
function  $1_aptos_hash_spec_sha2_512_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_sha2_512_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:17:9+58
function  $1_aptos_hash_spec_sha3_512_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_sha3_512_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:22:9+59
function  $1_aptos_hash_spec_ripemd160_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_ripemd160_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/../aptos-stdlib/sources/hash.spec.move:27:9+61
function  $1_aptos_hash_spec_blake2b_256_internal(bytes: Vec (int)): Vec (int);
axiom (forall bytes: Vec (int) ::
(var $$res := $1_aptos_hash_spec_blake2b_256_internal(bytes);
$IsValid'vec'u8''($$res)));

// spec fun at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.move:168:5+155
function {:inline} $1_reconfiguration_$last_reconfiguration_time($1_reconfiguration_Configuration_$memory: $Memory $1_reconfiguration_Configuration): int {
    $ResourceValue($1_reconfiguration_Configuration_$memory, 1)->$last_reconfiguration_time
}

// struct reconfiguration::Configuration at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.move:43:5+306
datatype $1_reconfiguration_Configuration {
    $1_reconfiguration_Configuration($epoch: int, $last_reconfiguration_time: int, $events: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent')
}
function {:inline} $Update'$1_reconfiguration_Configuration'_epoch(s: $1_reconfiguration_Configuration, x: int): $1_reconfiguration_Configuration {
    $1_reconfiguration_Configuration(x, s->$last_reconfiguration_time, s->$events)
}
function {:inline} $Update'$1_reconfiguration_Configuration'_last_reconfiguration_time(s: $1_reconfiguration_Configuration, x: int): $1_reconfiguration_Configuration {
    $1_reconfiguration_Configuration(s->$epoch, x, s->$events)
}
function {:inline} $Update'$1_reconfiguration_Configuration'_events(s: $1_reconfiguration_Configuration, x: $1_event_EventHandle'$1_reconfiguration_NewEpochEvent'): $1_reconfiguration_Configuration {
    $1_reconfiguration_Configuration(s->$epoch, s->$last_reconfiguration_time, x)
}
function $IsValid'$1_reconfiguration_Configuration'(s: $1_reconfiguration_Configuration): bool {
    $IsValid'u64'(s->$epoch)
      && $IsValid'u64'(s->$last_reconfiguration_time)
      && $IsValid'$1_event_EventHandle'$1_reconfiguration_NewEpochEvent''(s->$events)
}
function {:inline} $IsEqual'$1_reconfiguration_Configuration'(s1: $1_reconfiguration_Configuration, s2: $1_reconfiguration_Configuration): bool {
    s1 == s2
}
var $1_reconfiguration_Configuration_$memory: $Memory $1_reconfiguration_Configuration;

// struct reconfiguration::NewEpochEvent at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/reconfiguration.move:30:5+64
datatype $1_reconfiguration_NewEpochEvent {
    $1_reconfiguration_NewEpochEvent($epoch: int)
}
function {:inline} $Update'$1_reconfiguration_NewEpochEvent'_epoch(s: $1_reconfiguration_NewEpochEvent, x: int): $1_reconfiguration_NewEpochEvent {
    $1_reconfiguration_NewEpochEvent(x)
}
function $IsValid'$1_reconfiguration_NewEpochEvent'(s: $1_reconfiguration_NewEpochEvent): bool {
    $IsValid'u64'(s->$epoch)
}
function {:inline} $IsEqual'$1_reconfiguration_NewEpochEvent'(s1: $1_reconfiguration_NewEpochEvent, s2: $1_reconfiguration_NewEpochEvent): bool {
    s1 == s2
}

// struct timelock::CreateTransaction at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:155:5+189
datatype $1_timelock_CreateTransaction {
    $1_timelock_CreateTransaction($timelock_account: int, $creator: int, $transaction_hash: Vec (int), $transaction: $1_timelock_TimelockTransaction)
}
function {:inline} $Update'$1_timelock_CreateTransaction'_timelock_account(s: $1_timelock_CreateTransaction, x: int): $1_timelock_CreateTransaction {
    $1_timelock_CreateTransaction(x, s->$creator, s->$transaction_hash, s->$transaction)
}
function {:inline} $Update'$1_timelock_CreateTransaction'_creator(s: $1_timelock_CreateTransaction, x: int): $1_timelock_CreateTransaction {
    $1_timelock_CreateTransaction(s->$timelock_account, x, s->$transaction_hash, s->$transaction)
}
function {:inline} $Update'$1_timelock_CreateTransaction'_transaction_hash(s: $1_timelock_CreateTransaction, x: Vec (int)): $1_timelock_CreateTransaction {
    $1_timelock_CreateTransaction(s->$timelock_account, s->$creator, x, s->$transaction)
}
function {:inline} $Update'$1_timelock_CreateTransaction'_transaction(s: $1_timelock_CreateTransaction, x: $1_timelock_TimelockTransaction): $1_timelock_CreateTransaction {
    $1_timelock_CreateTransaction(s->$timelock_account, s->$creator, s->$transaction_hash, x)
}
function $IsValid'$1_timelock_CreateTransaction'(s: $1_timelock_CreateTransaction): bool {
    $IsValid'address'(s->$timelock_account)
      && $IsValid'address'(s->$creator)
      && $IsValid'vec'u8''(s->$transaction_hash)
      && $IsValid'$1_timelock_TimelockTransaction'(s->$transaction)
}
function {:inline} $IsEqual'$1_timelock_CreateTransaction'(s1: $1_timelock_CreateTransaction, s2: $1_timelock_CreateTransaction): bool {
    $IsEqual'address'(s1->$timelock_account, s2->$timelock_account)
    && $IsEqual'address'(s1->$creator, s2->$creator)
    && $IsEqual'vec'u8''(s1->$transaction_hash, s2->$transaction_hash)
    && $IsEqual'$1_timelock_TimelockTransaction'(s1->$transaction, s2->$transaction)}

// struct timelock::AddCreators at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:124:5+118
datatype $1_timelock_AddCreators {
    $1_timelock_AddCreators($timelock_account: int, $creators_added: Vec (int))
}
function {:inline} $Update'$1_timelock_AddCreators'_timelock_account(s: $1_timelock_AddCreators, x: int): $1_timelock_AddCreators {
    $1_timelock_AddCreators(x, s->$creators_added)
}
function {:inline} $Update'$1_timelock_AddCreators'_creators_added(s: $1_timelock_AddCreators, x: Vec (int)): $1_timelock_AddCreators {
    $1_timelock_AddCreators(s->$timelock_account, x)
}
function $IsValid'$1_timelock_AddCreators'(s: $1_timelock_AddCreators): bool {
    $IsValid'address'(s->$timelock_account)
      && $IsValid'vec'address''(s->$creators_added)
}
function {:inline} $IsEqual'$1_timelock_AddCreators'(s1: $1_timelock_AddCreators, s2: $1_timelock_AddCreators): bool {
    $IsEqual'address'(s1->$timelock_account, s2->$timelock_account)
    && $IsEqual'vec'address''(s1->$creators_added, s2->$creators_added)}

// struct timelock::AddExecutors at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:136:5+120
datatype $1_timelock_AddExecutors {
    $1_timelock_AddExecutors($timelock_account: int, $executors_added: Vec (int))
}
function {:inline} $Update'$1_timelock_AddExecutors'_timelock_account(s: $1_timelock_AddExecutors, x: int): $1_timelock_AddExecutors {
    $1_timelock_AddExecutors(x, s->$executors_added)
}
function {:inline} $Update'$1_timelock_AddExecutors'_executors_added(s: $1_timelock_AddExecutors, x: Vec (int)): $1_timelock_AddExecutors {
    $1_timelock_AddExecutors(s->$timelock_account, x)
}
function $IsValid'$1_timelock_AddExecutors'(s: $1_timelock_AddExecutors): bool {
    $IsValid'address'(s->$timelock_account)
      && $IsValid'vec'address''(s->$executors_added)
}
function {:inline} $IsEqual'$1_timelock_AddExecutors'(s1: $1_timelock_AddExecutors, s2: $1_timelock_AddExecutors): bool {
    $IsEqual'address'(s1->$timelock_account, s2->$timelock_account)
    && $IsEqual'vec'address''(s1->$executors_added, s2->$executors_added)}

// struct timelock::CancelTransaction at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:163:5+145
datatype $1_timelock_CancelTransaction {
    $1_timelock_CancelTransaction($timelock_account: int, $actor: int, $transaction_hash: Vec (int))
}
function {:inline} $Update'$1_timelock_CancelTransaction'_timelock_account(s: $1_timelock_CancelTransaction, x: int): $1_timelock_CancelTransaction {
    $1_timelock_CancelTransaction(x, s->$actor, s->$transaction_hash)
}
function {:inline} $Update'$1_timelock_CancelTransaction'_actor(s: $1_timelock_CancelTransaction, x: int): $1_timelock_CancelTransaction {
    $1_timelock_CancelTransaction(s->$timelock_account, x, s->$transaction_hash)
}
function {:inline} $Update'$1_timelock_CancelTransaction'_transaction_hash(s: $1_timelock_CancelTransaction, x: Vec (int)): $1_timelock_CancelTransaction {
    $1_timelock_CancelTransaction(s->$timelock_account, s->$actor, x)
}
function $IsValid'$1_timelock_CancelTransaction'(s: $1_timelock_CancelTransaction): bool {
    $IsValid'address'(s->$timelock_account)
      && $IsValid'address'(s->$actor)
      && $IsValid'vec'u8''(s->$transaction_hash)
}
function {:inline} $IsEqual'$1_timelock_CancelTransaction'(s1: $1_timelock_CancelTransaction, s2: $1_timelock_CancelTransaction): bool {
    $IsEqual'address'(s1->$timelock_account, s2->$timelock_account)
    && $IsEqual'address'(s1->$actor, s2->$actor)
    && $IsEqual'vec'u8''(s1->$transaction_hash, s2->$transaction_hash)}

// struct timelock::RemoveCreators at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:130:5+123
datatype $1_timelock_RemoveCreators {
    $1_timelock_RemoveCreators($timelock_account: int, $creators_removed: Vec (int))
}
function {:inline} $Update'$1_timelock_RemoveCreators'_timelock_account(s: $1_timelock_RemoveCreators, x: int): $1_timelock_RemoveCreators {
    $1_timelock_RemoveCreators(x, s->$creators_removed)
}
function {:inline} $Update'$1_timelock_RemoveCreators'_creators_removed(s: $1_timelock_RemoveCreators, x: Vec (int)): $1_timelock_RemoveCreators {
    $1_timelock_RemoveCreators(s->$timelock_account, x)
}
function $IsValid'$1_timelock_RemoveCreators'(s: $1_timelock_RemoveCreators): bool {
    $IsValid'address'(s->$timelock_account)
      && $IsValid'vec'address''(s->$creators_removed)
}
function {:inline} $IsEqual'$1_timelock_RemoveCreators'(s1: $1_timelock_RemoveCreators, s2: $1_timelock_RemoveCreators): bool {
    $IsEqual'address'(s1->$timelock_account, s2->$timelock_account)
    && $IsEqual'vec'address''(s1->$creators_removed, s2->$creators_removed)}

// struct timelock::RemoveExecutors at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:142:5+125
datatype $1_timelock_RemoveExecutors {
    $1_timelock_RemoveExecutors($timelock_account: int, $executors_removed: Vec (int))
}
function {:inline} $Update'$1_timelock_RemoveExecutors'_timelock_account(s: $1_timelock_RemoveExecutors, x: int): $1_timelock_RemoveExecutors {
    $1_timelock_RemoveExecutors(x, s->$executors_removed)
}
function {:inline} $Update'$1_timelock_RemoveExecutors'_executors_removed(s: $1_timelock_RemoveExecutors, x: Vec (int)): $1_timelock_RemoveExecutors {
    $1_timelock_RemoveExecutors(s->$timelock_account, x)
}
function $IsValid'$1_timelock_RemoveExecutors'(s: $1_timelock_RemoveExecutors): bool {
    $IsValid'address'(s->$timelock_account)
      && $IsValid'vec'address''(s->$executors_removed)
}
function {:inline} $IsEqual'$1_timelock_RemoveExecutors'(s1: $1_timelock_RemoveExecutors, s2: $1_timelock_RemoveExecutors): bool {
    $IsEqual'address'(s1->$timelock_account, s2->$timelock_account)
    && $IsEqual'vec'address''(s1->$executors_removed, s2->$executors_removed)}

// struct timelock::TimelockAccount at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:88:5+784
datatype $1_timelock_TimelockAccount {
    $1_timelock_TimelockAccount($creators: Vec (int), $executors: Vec (int), $min_num_seconds_execute: int, $transactions: Table int ($1_timelock_TimelockTransaction), $signer_cap: $1_account_SignerCapability)
}
function {:inline} $Update'$1_timelock_TimelockAccount'_creators(s: $1_timelock_TimelockAccount, x: Vec (int)): $1_timelock_TimelockAccount {
    $1_timelock_TimelockAccount(x, s->$executors, s->$min_num_seconds_execute, s->$transactions, s->$signer_cap)
}
function {:inline} $Update'$1_timelock_TimelockAccount'_executors(s: $1_timelock_TimelockAccount, x: Vec (int)): $1_timelock_TimelockAccount {
    $1_timelock_TimelockAccount(s->$creators, x, s->$min_num_seconds_execute, s->$transactions, s->$signer_cap)
}
function {:inline} $Update'$1_timelock_TimelockAccount'_min_num_seconds_execute(s: $1_timelock_TimelockAccount, x: int): $1_timelock_TimelockAccount {
    $1_timelock_TimelockAccount(s->$creators, s->$executors, x, s->$transactions, s->$signer_cap)
}
function {:inline} $Update'$1_timelock_TimelockAccount'_transactions(s: $1_timelock_TimelockAccount, x: Table int ($1_timelock_TimelockTransaction)): $1_timelock_TimelockAccount {
    $1_timelock_TimelockAccount(s->$creators, s->$executors, s->$min_num_seconds_execute, x, s->$signer_cap)
}
function {:inline} $Update'$1_timelock_TimelockAccount'_signer_cap(s: $1_timelock_TimelockAccount, x: $1_account_SignerCapability): $1_timelock_TimelockAccount {
    $1_timelock_TimelockAccount(s->$creators, s->$executors, s->$min_num_seconds_execute, s->$transactions, x)
}
function $IsValid'$1_timelock_TimelockAccount'(s: $1_timelock_TimelockAccount): bool {
    $IsValid'vec'address''(s->$creators)
      && $IsValid'vec'address''(s->$executors)
      && $IsValid'u64'(s->$min_num_seconds_execute)
      && $IsValid'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(s->$transactions)
      && $IsValid'$1_account_SignerCapability'(s->$signer_cap)
}
function {:inline} $IsEqual'$1_timelock_TimelockAccount'(s1: $1_timelock_TimelockAccount, s2: $1_timelock_TimelockAccount): bool {
    $IsEqual'vec'address''(s1->$creators, s2->$creators)
    && $IsEqual'vec'address''(s1->$executors, s2->$executors)
    && $IsEqual'u64'(s1->$min_num_seconds_execute, s2->$min_num_seconds_execute)
    && $IsEqual'$1_table_Table'vec'u8'_$1_timelock_TimelockTransaction''(s1->$transactions, s2->$transactions)
    && $IsEqual'$1_account_SignerCapability'(s1->$signer_cap, s2->$signer_cap)}
var $1_timelock_TimelockAccount_$memory: $Memory $1_timelock_TimelockAccount;

// struct timelock::TimelockTransaction at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:107:5+631
datatype $1_timelock_TimelockTransaction {
    $1_timelock_TimelockTransaction($execution_hash: Vec (int), $creator: int, $creation_time_secs: int, $num_seconds_execute: int, $salt: Vec (int), $executed: bool)
}
function {:inline} $Update'$1_timelock_TimelockTransaction'_execution_hash(s: $1_timelock_TimelockTransaction, x: Vec (int)): $1_timelock_TimelockTransaction {
    $1_timelock_TimelockTransaction(x, s->$creator, s->$creation_time_secs, s->$num_seconds_execute, s->$salt, s->$executed)
}
function {:inline} $Update'$1_timelock_TimelockTransaction'_creator(s: $1_timelock_TimelockTransaction, x: int): $1_timelock_TimelockTransaction {
    $1_timelock_TimelockTransaction(s->$execution_hash, x, s->$creation_time_secs, s->$num_seconds_execute, s->$salt, s->$executed)
}
function {:inline} $Update'$1_timelock_TimelockTransaction'_creation_time_secs(s: $1_timelock_TimelockTransaction, x: int): $1_timelock_TimelockTransaction {
    $1_timelock_TimelockTransaction(s->$execution_hash, s->$creator, x, s->$num_seconds_execute, s->$salt, s->$executed)
}
function {:inline} $Update'$1_timelock_TimelockTransaction'_num_seconds_execute(s: $1_timelock_TimelockTransaction, x: int): $1_timelock_TimelockTransaction {
    $1_timelock_TimelockTransaction(s->$execution_hash, s->$creator, s->$creation_time_secs, x, s->$salt, s->$executed)
}
function {:inline} $Update'$1_timelock_TimelockTransaction'_salt(s: $1_timelock_TimelockTransaction, x: Vec (int)): $1_timelock_TimelockTransaction {
    $1_timelock_TimelockTransaction(s->$execution_hash, s->$creator, s->$creation_time_secs, s->$num_seconds_execute, x, s->$executed)
}
function {:inline} $Update'$1_timelock_TimelockTransaction'_executed(s: $1_timelock_TimelockTransaction, x: bool): $1_timelock_TimelockTransaction {
    $1_timelock_TimelockTransaction(s->$execution_hash, s->$creator, s->$creation_time_secs, s->$num_seconds_execute, s->$salt, x)
}
function $IsValid'$1_timelock_TimelockTransaction'(s: $1_timelock_TimelockTransaction): bool {
    $IsValid'vec'u8''(s->$execution_hash)
      && $IsValid'address'(s->$creator)
      && $IsValid'u64'(s->$creation_time_secs)
      && $IsValid'u64'(s->$num_seconds_execute)
      && $IsValid'vec'u8''(s->$salt)
      && $IsValid'bool'(s->$executed)
}
function {:inline} $IsEqual'$1_timelock_TimelockTransaction'(s1: $1_timelock_TimelockTransaction, s2: $1_timelock_TimelockTransaction): bool {
    $IsEqual'vec'u8''(s1->$execution_hash, s2->$execution_hash)
    && $IsEqual'address'(s1->$creator, s2->$creator)
    && $IsEqual'u64'(s1->$creation_time_secs, s2->$creation_time_secs)
    && $IsEqual'u64'(s1->$num_seconds_execute, s2->$num_seconds_execute)
    && $IsEqual'vec'u8''(s1->$salt, s2->$salt)
    && $IsEqual'bool'(s1->$executed, s2->$executed)}

// struct timelock::UpdateMinNumSecondsExecute at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:148:5+176
datatype $1_timelock_UpdateMinNumSecondsExecute {
    $1_timelock_UpdateMinNumSecondsExecute($timelock_account: int, $old_min_num_seconds_execute: int, $new_min_num_seconds_execute: int)
}
function {:inline} $Update'$1_timelock_UpdateMinNumSecondsExecute'_timelock_account(s: $1_timelock_UpdateMinNumSecondsExecute, x: int): $1_timelock_UpdateMinNumSecondsExecute {
    $1_timelock_UpdateMinNumSecondsExecute(x, s->$old_min_num_seconds_execute, s->$new_min_num_seconds_execute)
}
function {:inline} $Update'$1_timelock_UpdateMinNumSecondsExecute'_old_min_num_seconds_execute(s: $1_timelock_UpdateMinNumSecondsExecute, x: int): $1_timelock_UpdateMinNumSecondsExecute {
    $1_timelock_UpdateMinNumSecondsExecute(s->$timelock_account, x, s->$new_min_num_seconds_execute)
}
function {:inline} $Update'$1_timelock_UpdateMinNumSecondsExecute'_new_min_num_seconds_execute(s: $1_timelock_UpdateMinNumSecondsExecute, x: int): $1_timelock_UpdateMinNumSecondsExecute {
    $1_timelock_UpdateMinNumSecondsExecute(s->$timelock_account, s->$old_min_num_seconds_execute, x)
}
function $IsValid'$1_timelock_UpdateMinNumSecondsExecute'(s: $1_timelock_UpdateMinNumSecondsExecute): bool {
    $IsValid'address'(s->$timelock_account)
      && $IsValid'u64'(s->$old_min_num_seconds_execute)
      && $IsValid'u64'(s->$new_min_num_seconds_execute)
}
function {:inline} $IsEqual'$1_timelock_UpdateMinNumSecondsExecute'(s1: $1_timelock_UpdateMinNumSecondsExecute, s2: $1_timelock_UpdateMinNumSecondsExecute): bool {
    s1 == s2
}

// fun timelock::get_transaction_hash [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:256:5+196
procedure {:inline 1} $1_timelock_get_transaction_hash(_$t0: Vec (int), _$t1: Vec (int)) returns ($ret0: Vec (int))
{
    // declare local variables
    var $t2: Vec (int);
    var $t3: $Mutation (Vec (int));
    var $t4: int;
    var $t5: Vec (int);
    var $t6: Vec (int);
    var $t0: Vec (int);
    var $t1: Vec (int);
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[execution_hash]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:256:5+1
    assume {:print "$at(2,11385,11386)"} true;
    assume {:print "$track_local(102,0,0):", $t0} $t0 == $t0;

    // trace_local[salt]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:256:5+1
    assume {:print "$track_local(102,0,1):", $t1} $t1 == $t1;

    // $t2 := $t0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:257:21+19
    assume {:print "$at(2,11497,11516)"} true;
    $t2 := $t0;

    // trace_local[bytes]($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:257:21+19
    assume {:print "$track_local(102,0,2):", $t2} $t2 == $t2;

    // $t3 := borrow_local($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:258:9+23
    assume {:print "$at(2,11526,11549)"} true;
    $t3 := $Mutation($Local(2), EmptyVec(), $t2);

    // vector::append<u8>($t3, $t1) on_abort goto L2 with $t4 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:258:9+23
    call $t3 := $1_vector_append'u8'($t3, $t1);
    if ($abort_flag) {
        assume {:print "$at(2,11526,11549)"} true;
        $t4 := $abort_code;
        assume {:print "$track_abort(102,0):", $t4} $t4 == $t4;
        goto L2;
    }

    // write_back[LocalRoot($t2)@]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:258:9+23
    $t2 := $Dereference($t3);

    // trace_local[bytes]($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:258:9+23
    assume {:print "$track_local(102,0,2):", $t2} $t2 == $t2;

    // $t5 := move($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:259:9+16
    assume {:print "$at(2,11559,11575)"} true;
    $t5 := $t2;

    // $t6 := opaque begin: aptos_hash::keccak256($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:259:9+16

    // assume WellFormed($t6) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:259:9+16
    assume $IsValid'vec'u8''($t6);

    // assume Eq<vector<u8>>($t6, aptos_hash::spec_keccak256($t5)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:259:9+16
    assume $IsEqual'vec'u8''($t6, $1_aptos_hash_spec_keccak256($t5));

    // $t6 := opaque end: aptos_hash::keccak256($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:259:9+16

    // trace_return[0]($t6) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:256:95+106
    assume {:print "$at(2,11475,11581)"} true;
    assume {:print "$track_return(102,0,0):", $t6} $t6 == $t6;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:260:5+1
    assume {:print "$at(2,11580,11581)"} true;
L1:

    // return $t6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:260:5+1
    assume {:print "$at(2,11580,11581)"} true;
    $ret0 := $t6;
    return;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:260:5+1
L2:

    // abort($t4) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:260:5+1
    assume {:print "$at(2,11580,11581)"} true;
    $abort_code := $t4;
    $abort_flag := true;
    return;

}

// fun timelock::create_timelock_account_seed [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:571:5+210
procedure {:inline 1} $1_timelock_create_timelock_account_seed(_$t0: Vec (int)) returns ($ret0: Vec (int))
{
    // declare local variables
    var $t1: Vec (int);
    var $t2: int;
    var $t3: $Mutation (Vec (int));
    var $t4: Vec (int);
    var $t5: $Mutation (Vec (int));
    var $t6: Vec (int);
    var $t0: Vec (int);
    var $temp_0'vec'u8'': Vec (int);
    $t0 := _$t0;

    // bytecode translation starts here
    // trace_local[seed]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:571:5+1
    assume {:print "$at(2,26195,26196)"} true;
    assume {:print "$track_local(102,14,0):", $t0} $t0 == $t0;

    // $t1 := vector::empty<u8>() on_abort goto L2 with $t2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:572:28+6
    assume {:print "$at(2,26287,26293)"} true;
    call $t1 := $1_vector_empty'u8'();
    if ($abort_flag) {
        assume {:print "$at(2,26287,26293)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(102,14):", $t2} $t2 == $t2;
        goto L2;
    }

    // trace_local[account_seed]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:572:28+6
    assume {:print "$track_local(102,14,1):", $t1} $t1 == $t1;

    // $t3 := borrow_local($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:573:9+37
    assume {:print "$at(2,26305,26342)"} true;
    $t3 := $Mutation($Local(1), EmptyVec(), $t1);

    // $t4 := [97, 112, 116, 111, 115, 95, 102, 114, 97, 109, 101, 119, 111, 114, 107, 58, 58, 116, 105, 109, 101, 108, 111, 99, 107] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:573:29+16
    $t4 := ConcatVec(ConcatVec(ConcatVec(ConcatVec(ConcatVec(ConcatVec(MakeVec4(97, 112, 116, 111), MakeVec4(115, 95, 102, 114)), MakeVec4(97, 109, 101, 119)), MakeVec4(111, 114, 107, 58)), MakeVec4(58, 116, 105, 109)), MakeVec4(101, 108, 111, 99)), MakeVec1(107));
    assume $IsValid'vec'u8''($t4);

    // vector::append<u8>($t3, $t4) on_abort goto L2 with $t2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:573:9+37
    call $t3 := $1_vector_append'u8'($t3, $t4);
    if ($abort_flag) {
        assume {:print "$at(2,26305,26342)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(102,14):", $t2} $t2 == $t2;
        goto L2;
    }

    // write_back[LocalRoot($t1)@]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:573:9+37
    $t1 := $Dereference($t3);

    // trace_local[account_seed]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:573:9+37
    assume {:print "$track_local(102,14,1):", $t1} $t1 == $t1;

    // $t5 := borrow_local($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:574:9+25
    assume {:print "$at(2,26352,26377)"} true;
    $t5 := $Mutation($Local(1), EmptyVec(), $t1);

    // vector::append<u8>($t5, $t0) on_abort goto L2 with $t2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:574:9+25
    call $t5 := $1_vector_append'u8'($t5, $t0);
    if ($abort_flag) {
        assume {:print "$at(2,26352,26377)"} true;
        $t2 := $abort_code;
        assume {:print "$track_abort(102,14):", $t2} $t2 == $t2;
        goto L2;
    }

    // write_back[LocalRoot($t1)@]($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:574:9+25
    $t1 := $Dereference($t5);

    // trace_local[account_seed]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:574:9+25
    assume {:print "$track_local(102,14,1):", $t1} $t1 == $t1;

    // $t6 := move($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:575:9+12
    assume {:print "$at(2,26387,26399)"} true;
    $t6 := $t1;

    // trace_return[0]($t6) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:571:68+147
    assume {:print "$at(2,26258,26405)"} true;
    assume {:print "$track_return(102,14,0):", $t6} $t6 == $t6;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:576:5+1
    assume {:print "$at(2,26404,26405)"} true;
L1:

    // return $t6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:576:5+1
    assume {:print "$at(2,26404,26405)"} true;
    $ret0 := $t6;
    return;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:576:5+1
L2:

    // abort($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:576:5+1
    assume {:print "$at(2,26404,26405)"} true;
    $abort_code := $t2;
    $abort_flag := true;
    return;

}

// fun timelock::is_creator [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:199:5+184
procedure {:inline 1} $1_timelock_is_creator(_$t0: int, _$t1: int) returns ($ret0: bool)
{
    // declare local variables
    var $t2: $1_timelock_TimelockAccount;
    var $t3: int;
    var $t4: Vec (int);
    var $t5: bool;
    var $t0: int;
    var $t1: int;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // trace_local[addr]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:199:5+1
    assume {:print "$at(2,8797,8798)"} true;
    assume {:print "$track_local(102,16,0):", $t0} $t0 == $t0;

    // trace_local[timelock_account]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:199:5+1
    assume {:print "$track_local(102,16,1):", $t1} $t1 == $t1;

    // $t2 := get_global<0x1::timelock::TimelockAccount>($t1) on_abort goto L2 with $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:200:9+48
    assume {:print "$at(2,8902,8950)"} true;
    if (!$ResourceExists($1_timelock_TimelockAccount_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t2 := $ResourceValue($1_timelock_TimelockAccount_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,8902,8950)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(102,16):", $t3} $t3 == $t3;
        goto L2;
    }

    // $t4 := get_field<0x1::timelock::TimelockAccount>.creators($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:200:9+73
    $t4 := $t2->$creators;

    // $t5 := vector::contains<address>($t4, $t0) on_abort goto L2 with $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:200:9+73
    call $t5 := $1_vector_contains'address'($t4, $t0);
    if ($abort_flag) {
        assume {:print "$at(2,8902,8975)"} true;
        $t3 := $abort_code;
        assume {:print "$track_abort(102,16):", $t3} $t3 == $t3;
        goto L2;
    }

    // trace_return[0]($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:200:9+73
    assume {:print "$track_return(102,16,0):", $t5} $t5 == $t5;

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:201:5+1
    assume {:print "$at(2,8980,8981)"} true;
L1:

    // return $t5 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:201:5+1
    assume {:print "$at(2,8980,8981)"} true;
    $ret0 := $t5;
    return;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:201:5+1
L2:

    // abort($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:201:5+1
    assume {:print "$at(2,8980,8981)"} true;
    $abort_code := $t3;
    $abort_flag := true;
    return;

}

// fun timelock::is_executor [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:206:5+341
procedure {:inline 1} $1_timelock_is_executor(_$t0: int, _$t1: int) returns ($ret0: bool)
{
    // declare local variables
    var $t2: $1_timelock_TimelockAccount;
    var $t3: bool;
    var $t4: $1_timelock_TimelockAccount;
    var $t5: $1_timelock_TimelockAccount;
    var $t6: int;
    var $t7: Vec (int);
    var $t8: bool;
    var $t9: Vec (int);
    var $t10: bool;
    var $t11: Vec (int);
    var $t12: bool;
    var $t0: int;
    var $t1: int;
    var $temp_0'$1_timelock_TimelockAccount': $1_timelock_TimelockAccount;
    var $temp_0'address': int;
    var $temp_0'bool': bool;
    $t0 := _$t0;
    $t1 := _$t1;

    // bytecode translation starts here
    // assume Identical($t4, global<0x1::timelock::TimelockAccount>($t1)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.spec.move:128:9+57
    assume {:print "$at(3,8349,8406)"} true;
    assume ($t4 == $ResourceValue($1_timelock_TimelockAccount_$memory, $t1));

    // trace_local[addr]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:206:5+1
    assume {:print "$at(2,9159,9160)"} true;
    assume {:print "$track_local(102,17,0):", $t0} $t0 == $t0;

    // trace_local[timelock_account]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:206:5+1
    assume {:print "$track_local(102,17,1):", $t1} $t1 == $t1;

    // $t5 := get_global<0x1::timelock::TimelockAccount>($t1) on_abort goto L4 with $t6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:207:24+48
    assume {:print "$at(2,9280,9328)"} true;
    if (!$ResourceExists($1_timelock_TimelockAccount_$memory, $t1)) {
        call $ExecFailureAbort();
    } else {
        $t5 := $ResourceValue($1_timelock_TimelockAccount_$memory, $t1);
    }
    if ($abort_flag) {
        assume {:print "$at(2,9280,9328)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(102,17):", $t6} $t6 == $t6;
        goto L4;
    }

    // trace_local[timelock]($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:207:24+48
    assume {:print "$track_local(102,17,2):", $t5} $t5 == $t5;

    // $t7 := get_field<0x1::timelock::TimelockAccount>.executors($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:208:13+29
    assume {:print "$at(2,9342,9371)"} true;
    $t7 := $t5->$executors;

    // $t8 := vector::is_empty<address>($t7) on_abort goto L4 with $t6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:208:13+29
    call $t8 := $1_vector_is_empty'address'($t7);
    if ($abort_flag) {
        assume {:print "$at(2,9342,9371)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(102,17):", $t6} $t6 == $t6;
        goto L4;
    }

    // if ($t8) goto L1 else goto L0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:208:9+156
    if ($t8) { goto L1; } else { goto L0; }

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:209:13+33
    assume {:print "$at(2,9387,9420)"} true;
L1:

    // $t9 := get_field<0x1::timelock::TimelockAccount>.creators($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:209:13+33
    assume {:print "$at(2,9387,9420)"} true;
    $t9 := $t5->$creators;

    // $t10 := vector::contains<address>($t9, $t0) on_abort goto L4 with $t6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:209:13+33
    call $t10 := $1_vector_contains'address'($t9, $t0);
    if ($abort_flag) {
        assume {:print "$at(2,9387,9420)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(102,17):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t3 := $t10 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:209:13+33
    $t3 := $t10;

    // trace_local[$t4]($t10) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:209:13+33
    assume {:print "$track_local(102,17,3):", $t10} $t10 == $t10;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:208:9+156
    assume {:print "$at(2,9338,9494)"} true;
L2:

    // trace_return[0]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:208:9+156
    assume {:print "$at(2,9338,9494)"} true;
    assume {:print "$track_return(102,17,0):", $t3} $t3 == $t3;

    // goto L3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:208:9+156
    goto L3;

    // label L0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:211:13+34
    assume {:print "$at(2,9450,9484)"} true;
L0:

    // $t11 := get_field<0x1::timelock::TimelockAccount>.executors($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:211:13+34
    assume {:print "$at(2,9450,9484)"} true;
    $t11 := $t5->$executors;

    // $t12 := vector::contains<address>($t11, $t0) on_abort goto L4 with $t6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:211:13+34
    call $t12 := $1_vector_contains'address'($t11, $t0);
    if ($abort_flag) {
        assume {:print "$at(2,9450,9484)"} true;
        $t6 := $abort_code;
        assume {:print "$track_abort(102,17):", $t6} $t6 == $t6;
        goto L4;
    }

    // $t3 := $t12 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:211:13+34
    $t3 := $t12;

    // trace_local[$t4]($t12) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:211:13+34
    assume {:print "$track_local(102,17,3):", $t12} $t12 == $t12;

    // goto L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:211:13+34
    goto L2;

    // label L3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:213:5+1
    assume {:print "$at(2,9499,9500)"} true;
L3:

    // return $t3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:213:5+1
    assume {:print "$at(2,9499,9500)"} true;
    $ret0 := $t3;
    return;

    // label L4 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:213:5+1
L4:

    // abort($t6) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:213:5+1
    assume {:print "$at(2,9499,9500)"} true;
    $abort_code := $t6;
    $abort_flag := true;
    return;

}

// fun timelock::validate_members [baseline] at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:581:5+1008
procedure {:inline 1} $1_timelock_validate_members(_$t0: Vec (int), _$t1: int, _$t2: int) returns ()
{
    // declare local variables
    var $t3: Vec (int);
    var $t4: int;
    var $t5: int;
    var $t6: int;
    var $t7: int;
    var $t8: int;
    var $t9: int;
    var $t10: bool;
    var $t11: int;
    var $t12: bool;
    var $t13: Vec (int);
    var $t14: bool;
    var $t15: int;
    var $t16: int;
    var $t17: int;
    var $t18: $Mutation (Vec (int));
    var $t19: int;
    var $t20: int;
    var $t21: int;
    var $t0: Vec (int);
    var $t1: int;
    var $t2: int;
    var $temp_0'address': int;
    var $temp_0'u64': int;
    var $temp_0'vec'address'': Vec (int);
    $t0 := _$t0;
    $t1 := _$t1;
    $t2 := _$t2;

    // bytecode translation starts here
    // trace_local[members]($t0) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:581:5+1
    assume {:print "$at(2,26666,26667)"} true;
    assume {:print "$track_local(102,21,0):", $t0} $t0 == $t0;

    // trace_local[timelock_address]($t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:581:5+1
    assume {:print "$track_local(102,21,1):", $t1} $t1 == $t1;

    // trace_local[duplicate_error]($t2) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:581:5+1
    assume {:print "$track_local(102,21,2):", $t2} $t2 == $t2;

    // $t3 := vector::empty<address>() on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:582:41+6
    assume {:print "$at(2,26805,26811)"} true;
    call $t3 := $1_vector_empty'address'();
    if ($abort_flag) {
        assume {:print "$at(2,26805,26811)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // trace_local[distinct]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:582:41+6
    assume {:print "$track_local(102,21,3):", $t3} $t3 == $t3;

    // $t8 := vector::length<address>($t0) on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:583:21+16
    assume {:print "$at(2,26835,26851)"} true;
    call $t8 := $1_vector_length'address'($t0);
    if ($abort_flag) {
        assume {:print "$at(2,26835,26851)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // trace_local[total]($t8) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:583:21+16
    assume {:print "$track_local(102,21,4):", $t8} $t8 == $t8;

    // $t9 := 0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:584:17+1
    assume {:print "$at(2,26869,26870)"} true;
    $t9 := 0;
    assume $IsValid'u64'($t9);

    // trace_local[i]($t9) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:584:17+1
    assume {:print "$track_local(102,21,5):", $t9} $t9 == $t9;

    // label L6 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:586:13+339
    assume {:print "$at(2,26901,27240)"} true;
L6:

    // assert Le($t9, $t8) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:587:17+21
    assume {:print "$at(2,26924,26945)"} true;
    assert {:msg "assert_failed(2,26924,26945): base case of the loop invariant does not hold"}
      ($t9 <= $t8);

    // assert Eq<num>(Len<address>($t3), $t9) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:588:17+29
    assume {:print "$at(2,26962,26991)"} true;
    assert {:msg "assert_failed(2,26962,26991): base case of the loop invariant does not hold"}
      $IsEqual'num'(LenVec($t3), $t9);

    // assert forall k: num: Range(0, $t9): Eq<address>(Index($t3, k), Index($t0, k)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:589:17+54
    assume {:print "$at(2,27008,27062)"} true;
    assert {:msg "assert_failed(2,27008,27062): base case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t9); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    ($IsEqual'address'(ReadVec($t3, k), ReadVec($t0, k))))));

    // assert forall k: num: Range(0, $t9): Neq<address>(Index($t0, k), $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:590:17+59
    assume {:print "$at(2,27079,27138)"} true;
    assert {:msg "assert_failed(2,27079,27138): base case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t9); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    (!$IsEqual'address'(ReadVec($t0, k), $t1)))));

    // assert forall k: num: Range(0, $t9): forall l: num: Range(0, k): Neq<address>(Index($t0, k), Index($t0, l)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume {:print "$at(2,27155,27226)"} true;
    assert {:msg "assert_failed(2,27155,27226): base case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t9); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    ((var $range_2 := $Range(0, k); (forall $i_3: int :: $InRange($range_2, $i_3) ==> (var l := $i_3;
    (!$IsEqual'address'(ReadVec($t0, k), ReadVec($t0, l))))))))));

    // $t3 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t3;

    // assume WellFormed($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'vec'address''($t3);

    // $t5 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t5;

    // assume WellFormed($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'u64'($t5);

    // $t10 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t10;

    // assume WellFormed($t10) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'bool'($t10);

    // $t11 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t11;

    // assume WellFormed($t11) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'address'($t11);

    // $t12 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t12;

    // assume WellFormed($t12) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'bool'($t12);

    // $t13 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t13;

    // assume WellFormed($t13) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'vec'address''($t13);

    // $t14 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t14;

    // assume WellFormed($t14) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'bool'($t14);

    // $t15 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t15;

    // assume WellFormed($t15) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'u64'($t15);

    // $t16 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t16;

    // assume WellFormed($t16) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'u64'($t16);

    // $t17 := havoc[val]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t17;

    // assume WellFormed($t17) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'u64'($t17);

    // $t18 := havoc[mut_all]() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    havoc $t18;

    // assume WellFormed($t18) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume $IsValid'vec'address''($Dereference($t18));

    // trace_local[distinct]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume {:print "$info(): enter loop, variable(s) distinct, i havocked and reassigned"} true;
    assume {:print "$track_local(102,21,3):", $t3} $t3 == $t3;

    // trace_local[i]($t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume {:print "$track_local(102,21,5):", $t5} $t5 == $t5;

    // assume Not(AbortFlag()) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume {:print "$info(): loop invariant holds at current state"} true;
    assume !$abort_flag;

    // assume Le($t5, $t8) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:587:17+21
    assume {:print "$at(2,26924,26945)"} true;
    assume ($t5 <= $t8);

    // assume Eq<num>(Len<address>($t3), $t5) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:588:17+29
    assume {:print "$at(2,26962,26991)"} true;
    assume $IsEqual'num'(LenVec($t3), $t5);

    // assume forall k: num: Range(0, $t5): Eq<address>(Index($t3, k), Index($t0, k)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:589:17+54
    assume {:print "$at(2,27008,27062)"} true;
    assume (var $range_0 := $Range(0, $t5); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    ($IsEqual'address'(ReadVec($t3, k), ReadVec($t0, k))))));

    // assume forall k: num: Range(0, $t5): Neq<address>(Index($t0, k), $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:590:17+59
    assume {:print "$at(2,27079,27138)"} true;
    assume (var $range_0 := $Range(0, $t5); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    (!$IsEqual'address'(ReadVec($t0, k), $t1)))));

    // assume forall k: num: Range(0, $t5): forall l: num: Range(0, k): Neq<address>(Index($t0, k), Index($t0, l)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume {:print "$at(2,27155,27226)"} true;
    assume (var $range_0 := $Range(0, $t5); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    ((var $range_2 := $Range(0, k); (forall $i_3: int :: $InRange($range_2, $i_3) ==> (var l := $i_3;
    (!$IsEqual'address'(ReadVec($t0, k), ReadVec($t0, l))))))))));

    // $t10 := <($t5, $t8) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:593:13+9
    assume {:print "$at(2,27254,27263)"} true;
    call $t10 := $Lt($t5, $t8);

    // if ($t10) goto L1 else goto L0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:585:9+787
    assume {:print "$at(2,26880,27667)"} true;
    if ($t10) { goto L1; } else { goto L0; }

    // label L1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:595:27+7
    assume {:print "$at(2,27303,27310)"} true;
L1:

    // $t11 := vector::borrow<address>($t0, $t5) on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:595:27+17
    assume {:print "$at(2,27303,27320)"} true;
    call $t11 := $1_vector_borrow'address'($t0, $t5);
    if ($abort_flag) {
        assume {:print "$at(2,27303,27320)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // trace_local[member]($t11) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:595:26+18
    assume {:print "$track_local(102,21,6):", $t11} $t11 == $t11;

    // $t12 := !=($t11, $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:597:17+26
    assume {:print "$at(2,27359,27385)"} true;
    $t12 := !$IsEqual'address'($t11, $t1);

    // if ($t12) goto L3 else goto L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:596:13+6
    assume {:print "$at(2,27334,27340)"} true;
    if ($t12) { goto L3; } else { goto L2; }

    // label L3 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:600:30+26
    assume {:print "$at(2,27496,27522)"} true;
L3:

    // $t13 := copy($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:600:30+26
    assume {:print "$at(2,27496,27522)"} true;
    $t13 := $t3;

    // ($t14, $t15) := vector::index_of<address>($t13, $t11) on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:600:30+26
    call $t14,$t15 := $1_vector_index_of'address'($t13, $t11);
    if ($abort_flag) {
        assume {:print "$at(2,27496,27522)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // drop($t15) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:600:30+26

    // if ($t14) goto L4 else goto L5 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:601:21+6
    assume {:print "$at(2,27544,27550)"} true;
    if ($t14) { goto L4; } else { goto L5; }

    // label L5 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:602:13+26
    assume {:print "$at(2,27607,27633)"} true;
L5:

    // $t18 := borrow_local($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:602:13+26
    assume {:print "$at(2,27607,27633)"} true;
    $t18 := $Mutation($Local(3), EmptyVec(), $t3);

    // vector::push_back<address>($t18, $t11) on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:602:13+26
    call $t18 := $1_vector_push_back'address'($t18, $t11);
    if ($abort_flag) {
        assume {:print "$at(2,27607,27633)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // write_back[LocalRoot($t3)@]($t18) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:602:13+26
    $t3 := $Dereference($t18);

    // trace_local[distinct]($t3) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:602:13+26
    assume {:print "$track_local(102,21,3):", $t3} $t3 == $t3;

    // $t16 := 1 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:603:21+1
    assume {:print "$at(2,27655,27656)"} true;
    $t16 := 1;
    assume $IsValid'u64'($t16);

    // $t17 := +($t5, $t16) on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:603:17+5
    call $t17 := $AddU64($t5, $t16);
    if ($abort_flag) {
        assume {:print "$at(2,27651,27656)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // trace_local[i]($t17) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:603:13+9
    assume {:print "$track_local(102,21,5):", $t17} $t17 == $t17;

    // goto L7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:585:9+787
    assume {:print "$at(2,26880,27667)"} true;
    goto L7;

    // label L4 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:601:13+6
    assume {:print "$at(2,27536,27542)"} true;
L4:

    // $t19 := error::invalid_argument($t2) on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:601:29+40
    assume {:print "$at(2,27552,27592)"} true;
    call $t19 := $1_error_invalid_argument($t2);
    if ($abort_flag) {
        assume {:print "$at(2,27552,27592)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // trace_abort($t19) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:601:13+6
    assume {:print "$at(2,27536,27542)"} true;
    assume {:print "$track_abort(102,21):", $t19} $t19 == $t19;

    // $t7 := move($t19) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:601:13+6
    $t7 := $t19;

    // goto L9 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:601:13+6
    goto L9;

    // label L2 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:596:13+6
    assume {:print "$at(2,27334,27340)"} true;
L2:

    // $t20 := 10 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:598:41+22
    assume {:print "$at(2,27427,27449)"} true;
    $t20 := 10;
    assume $IsValid'u64'($t20);

    // $t21 := error::invalid_argument($t20) on_abort goto L9 with $t7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:598:17+47
    call $t21 := $1_error_invalid_argument($t20);
    if ($abort_flag) {
        assume {:print "$at(2,27403,27450)"} true;
        $t7 := $abort_code;
        assume {:print "$track_abort(102,21):", $t7} $t7 == $t7;
        goto L9;
    }

    // trace_abort($t21) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:596:13+6
    assume {:print "$at(2,27334,27340)"} true;
    assume {:print "$track_abort(102,21):", $t21} $t21 == $t21;

    // $t7 := move($t21) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:596:13+6
    $t7 := $t21;

    // goto L9 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:596:13+6
    goto L9;

    // label L0 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:585:9+787
    assume {:print "$at(2,26880,27667)"} true;
L0:

    // goto L8 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:581:102+911
    assume {:print "$at(2,26763,27674)"} true;
    goto L8;

    // label L7 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:585:9+787
    // Loop invariant checking block for the loop started with header: L6
    assume {:print "$at(2,26880,27667)"} true;
L7:

    // assert Le($t17, $t8) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:587:17+21
    assume {:print "$at(2,26924,26945)"} true;
    assert {:msg "assert_failed(2,26924,26945): induction case of the loop invariant does not hold"}
      ($t17 <= $t8);

    // assert Eq<num>(Len<address>($t3), $t17) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:588:17+29
    assume {:print "$at(2,26962,26991)"} true;
    assert {:msg "assert_failed(2,26962,26991): induction case of the loop invariant does not hold"}
      $IsEqual'num'(LenVec($t3), $t17);

    // assert forall k: num: Range(0, $t17): Eq<address>(Index($t3, k), Index($t0, k)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:589:17+54
    assume {:print "$at(2,27008,27062)"} true;
    assert {:msg "assert_failed(2,27008,27062): induction case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t17); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    ($IsEqual'address'(ReadVec($t3, k), ReadVec($t0, k))))));

    // assert forall k: num: Range(0, $t17): Neq<address>(Index($t0, k), $t1) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:590:17+59
    assume {:print "$at(2,27079,27138)"} true;
    assert {:msg "assert_failed(2,27079,27138): induction case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t17); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    (!$IsEqual'address'(ReadVec($t0, k), $t1)))));

    // assert forall k: num: Range(0, $t17): forall l: num: Range(0, k): Neq<address>(Index($t0, k), Index($t0, l)) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume {:print "$at(2,27155,27226)"} true;
    assert {:msg "assert_failed(2,27155,27226): induction case of the loop invariant does not hold"}
      (var $range_0 := $Range(0, $t17); (forall $i_1: int :: $InRange($range_0, $i_1) ==> (var k := $i_1;
    ((var $range_2 := $Range(0, k); (forall $i_3: int :: $InRange($range_2, $i_3) ==> (var l := $i_3;
    (!$IsEqual'address'(ReadVec($t0, k), ReadVec($t0, l))))))))));

    // stop() at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:591:17+71
    assume false;
    return;

    // label L8 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:605:5+1
    assume {:print "$at(2,27673,27674)"} true;
L8:

    // return () at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:605:5+1
    assume {:print "$at(2,27673,27674)"} true;
    return;

    // label L9 at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:605:5+1
L9:

    // abort($t7) at /Users/primata/movement/aptos-core/aptos-move/framework/aptos-framework/sources/timelock.move:605:5+1
    assume {:print "$at(2,27673,27674)"} true;
    $abort_code := $t7;
    $abort_flag := true;
    return;

}
