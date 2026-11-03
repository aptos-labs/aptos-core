-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Tests.Builder

/-!
# Reference-semantics tests

Each test runs a small program and checks the outcome class: `returned`,
`failed` (a language-defined failure), or `wrong` (a rule violation the
verifier is meant to exclude).
-/

namespace MoveBytecode.Tests

open MoveBytecode

def single (f : Fun) (constants : List Constant := []) : Program :=
  ⟨#[mkModule 0x42 "m" [f] [] constants]⟩

def i8v (n : Int) : Value := .int .i8 n
def u8v (n : Nat) : Value := .int .u8 n

/-! ## Arithmetic and control flow -/

def addFun : Fun :=
  { name := "add", params := [.u64, .u64], returns := [.u64],
    code := #[.copyLoc 0, .copyLoc 1, .add, .ret] }

#guard runFun (single addFun) 0 0 [u64 2, u64 3] == .returned [u64 5]
#guard runFun (single addFun) 0 0 [u64 (2 ^ 64 - 1), u64 1] == .failed .arithmetic

/-- `sum(n) = 1 + 2 + … + n`, with a loop. -/
def sumFun : Fun :=
  { name := "sum", params := [.u64], returns := [.u64], locals := [.u64, .u64],
    code := #[
      .ldU64 0, .stLoc 1, .ldU64 0, .stLoc 2,
      .copyLoc 1, .copyLoc 0, .lt, .brFalse 17,
      .copyLoc 1, .ldU64 1, .add, .stLoc 1,
      .copyLoc 2, .copyLoc 1, .add, .stLoc 2,
      .branch 4,
      .moveLoc 2, .ret] }

#guard runFun (single sumFun) 0 0 [u64 10] == .returned [u64 55]
#guard runFun (single sumFun) 0 0 [u64 1000] (fuel := 100) == .limit

#guard runFun (single { name := "abort", code := #[.ldU64 42, .abort] }) 0 0 [] ==
  .failed (.abort 42)

/-! ## Rule violations are `wrong` -/

#guard runFun (single { name := "f", returns := [.u64], code := #[.add, .ret] }) 0 0 [] ==
  .wrong .stackUnderflow
#guard runFun (single { name := "f", returns := [.u64], code := #[.ldTrue, .ldU64 1, .add, .ret] })
  0 0 [] == .wrong .typeMismatch
#guard runFun (single { name := "f", code := #[.ldU64 1, .pop] }) 0 0 [] == .wrong .fellOffEnd
#guard runFun (single { name := "f", code := #[.branch 10] }) 0 0 [] == .wrong .invalidJump
#guard runFun (single { name := "f", code := #[.ldU64 1, .ret] }) 0 0 [] ==
  .wrong .stackNotEmptyAtReturn
#guard runFun (single { name := "f", returns := [.u64], code := #[.ldTrue, .ret] }) 0 0 [] ==
  .wrong .typeMismatch
def useAfterMove : Fun :=
  { name := "f", params := [.u64], returns := [.u64, .u64],
    code := #[.moveLoc 0, .moveLoc 0, .ret] }

#guard runFun (single useAfterMove) 0 0 [u64 1] == .wrong .unavailableLocal
#guard runFun (single { name := "f", locals := [.u64], code := #[.ldTrue, .stLoc 0, .ret] })
  0 0 [] == .wrong .typeMismatch
#guard runFun (single { name := "f", code := #[.copyLoc 3, .ret] }) 0 0 [] ==
  .wrong .localOutOfRange
#guard runFun (single { name := "f", code := #[.pack 0, .ret] }) 0 0 [] == .wrong .badIndex
#guard runFun (single { name := "f", code := #[.packGeneric 0, .ret] }) 0 0 [] == .wrong .badIndex
#guard runFun (single { name := "f", params := [.structInstantiation 0 [.u64]], code := #[.ret] })
  0 0 [] == .wrong (.unsupported "no initial state")

/-! ## Signed integers, casts and shifts -/

def binopI8 (op : Bytecode) : Fun :=
  { name := "op", params := [.i8, .i8], returns := [.i8],
    code := #[.copyLoc 0, .copyLoc 1, op, .ret] }

#guard runFun (single (binopI8 .div)) 0 0 [i8v (-7), i8v 2] == .returned [i8v (-3)]
#guard runFun (single (binopI8 .mod)) 0 0 [i8v (-7), i8v 2] == .returned [i8v (-1)]
#guard runFun (single (binopI8 .div)) 0 0 [i8v (-128), i8v (-1)] == .failed .arithmetic
#guard runFun (single (binopI8 .mod)) 0 0 [i8v (-128), i8v (-1)] == .failed .arithmetic
#guard runFun (single (binopI8 .div)) 0 0 [i8v 1, i8v 0] == .failed .arithmetic
#guard runFun (single (binopI8 .bitOr)) 0 0 [i8v 1, i8v 2] == .wrong .typeMismatch

def unary (param : SignatureToken) (ret : SignatureToken) (op : Bytecode) : Fun :=
  { name := "op", params := [param], returns := [ret], code := #[.copyLoc 0, op, .ret] }

#guard runFun (single (unary .i8 .i8 .negate)) 0 0 [i8v 5] == .returned [i8v (-5)]
#guard runFun (single (unary .i8 .i8 .negate)) 0 0 [i8v (-128)] == .failed .arithmetic
#guard runFun (single (unary .u8 .u8 .negate)) 0 0 [u8v 5] == .wrong .typeMismatch
#guard runFun (single (unary .i8 .u8 .castU8)) 0 0 [i8v (-1)] == .failed .arithmetic
#guard runFun (single (unary .u64 .i8 .castI8)) 0 0 [u64 127] == .returned [i8v 127]
#guard runFun (single (unary .u64 .u8 .castU8)) 0 0 [u64 300] == .failed .arithmetic

def shiftU8 (op : Bytecode) (amount : Nat) : Fun :=
  { name := "shift", params := [.u8], returns := [.u8],
    code := #[.copyLoc 0, .ldU8 (BitVec.ofNat 8 amount), op, .ret] }

#guard runFun (single (shiftU8 .shl 1)) 0 0 [u8v 255] == .returned [u8v 254]
#guard runFun (single (shiftU8 .shr 4)) 0 0 [u8v 255] == .returned [u8v 15]
#guard runFun (single (shiftU8 .shl 8)) 0 0 [u8v 1] == .failed .arithmetic
def shiftByU64 : Fun :=
  { name := "f", returns := [.u8], code := #[.ldU8 1, .ldU64 1, .shl, .ret] }

#guard runFun (single shiftByU64) 0 0 [] == .wrong .typeMismatch

/-! ## Constants -/

#guard runFun (single { name := "c", returns := [.u64], code := #[.ldConst 0, .ret] }
    [{ type := .u64, data := [2, 1, 0, 0, 0, 0, 0, 0] }]) 0 0 [] == .returned [u64 258]
#guard runFun (single { name := "c", returns := [.i16], code := #[.ldConst 0, .ret] }
    [{ type := .i16, data := [0xfe, 0xff] }]) 0 0 [] == .returned [.int .i16 (-2)]
#guard runFun (single { name := "c", returns := [.u64], code := #[.ldConst 0, .ret] }
    [{ type := .u64, data := [2, 1] }]) 0 0 [] == .wrong .badConstant

/-! ## Calls across modules -/

def moduleA : CompiledModule :=
  mkModule 0x1 "a"
    [{ name := "f", params := [.u64], returns := [.u64],
       code := #[.copyLoc 0, .ldU64 1, .add, .ret] },
     { name := "g", returns := [.u64], visibility := .private_, code := #[.ldU64 7, .ret] }]

def moduleB : CompiledModule :=
  mkModule 0x2 "b"
    [{ name := "callF", returns := [.u64], code := #[.ldU64 5, .call 2, .ret] },
     { name := "callG", returns := [.u64], code := #[.call 3, .ret] }]
    [{ address := 0x1, module := "a", name := "f", params := [.u64], returns := [.u64] },
     { address := 0x1, module := "a", name := "g", returns := [.u64] }]

#guard runFun ⟨#[moduleA, moduleB]⟩ 1 0 [] == .returned [u64 6]
#guard runFun ⟨#[moduleA, moduleB]⟩ 1 1 [] == .wrong .visibility

/-! ## The module lock (R4)

`a::start` calls `b::locked`, which calls back into `a`. With
`#[module_lock]` on `b::locked`, re-entering `a` fails; without it, the call
succeeds. -/

def lockA : CompiledModule :=
  mkModule 0x1 "a"
    [{ name := "start", code := #[.call 2, .ret] },
     { name := "h", code := #[.ret] }]
    [{ address := 0x2, module := "b", name := "locked" }]

def lockB (attributes : List FunctionAttribute) : CompiledModule :=
  mkModule 0x2 "b"
    [{ name := "locked", attributes, code := #[.call 1, .ret] }]
    [{ address := 0x1, module := "a", name := "h" }]

#guard runFun ⟨#[lockA, lockB [.moduleLock]]⟩ 0 0 [] == .failed .moduleLocked
#guard runFun ⟨#[lockA, lockB []]⟩ 0 0 [] == .returned []

/-! ## Natives -/

def nativeModule : CompiledModule :=
  mkModule 0x1 "n"
    [{ name := "twice", params := [.u64], returns := [.u64], native := true },
     { name := "main", returns := [.u64], code := #[.ldU64 21, .call 0, .ret] }]

def twice : NativeImpl := fun _ args =>
  match args with
  | [.int .u64 x] => .returned [u64 (2 * x).toNat] [] []
  | _ => .failed (.abort 1)

def badNative : NativeImpl := fun _ _ => .returned [.bool true] [] []

#guard (initialState ⟨#[nativeModule]⟩ ⟨0, 1, []⟩ []).map (run ⟨#[nativeModule]⟩ twice 100) ==
  some (.returned [u64 42])
#guard (initialState ⟨#[nativeModule]⟩ ⟨0, 1, []⟩ []).map (run ⟨#[nativeModule]⟩ badNative 100) ==
  some (.wrong .nativeContract)

/-! ## Data by value (Phase 2a)

Module `0x42::m` declares four types:

- struct 0, `S has copy, drop, store { u64, bool }`;
- struct 1, `R {}`, with no abilities;
- struct 2, `Bad has copy, drop { R }`, whose declared abilities its field
  does not justify (an L6 violation);
- struct 3, `enum E has copy, drop { A(u64), B }`.

Variant handle 0 is `E::A`, 1 is `E::B`, 2 names the struct `S`, and 3 names
a variant `E` does not have. With one function and no imports, extra
signature `k` has index `3 + k`: 3 is `[u64]`, 4 `[u8]`, 5 `[R]`, 6
`[u64, bool]`, 7 `[bool]`, 8 `[S]`, 9 `[]`. -/

def dataStructs : List StructDecl :=
  [{ name := "S", abilities := ⟨true, true, true, false⟩, fields := [.u64, .bool] },
   { name := "R" },
   { name := "Bad", abilities := ⟨true, true, false, false⟩, fields := [.struct 1] },
   { name := "E", abilities := ⟨true, true, false, false⟩,
     variants := some [("A", [.u64]), ("B", [])] }]

def withData (f : Fun) (constants : List Constant := []) : Program :=
  ⟨#[mkModule 0x42 "m" [f] [] constants dataStructs [] [⟨3, 0⟩, ⟨3, 1⟩, ⟨0, 0⟩, ⟨3, 5⟩]
      [[.u64], [.u8], [.struct 1], [.u64, .bool], [.bool], [.struct 0], []]]⟩

def runData (f : Fun) (args : List Value := []) (constants : List Constant := []) : RunResult :=
  runFun (withData f constants) 0 0 args

/-- A function `f` of the data module with the given code, returns and
locals. -/
def dataFun (code : Array Bytecode) (returns : Signature := []) (locals : Signature := [])
    (params : Signature := []) : Fun :=
  { name := "f", params, returns, locals, code }

def dataModule : ModuleId := ⟨0x42, "m"⟩
def typeS : StructId := ⟨dataModule, "S", []⟩
def typeR : StructId := ⟨dataModule, "R", []⟩
def typeE : StructId := ⟨dataModule, "E", []⟩
def valS (a : Nat) (b : Bool) : Value := .struct typeS [u64 a, .bool b]

/-! ### Structs -/

#guard runData (dataFun #[.copyLoc 0, .ldTrue, .pack 0, .unpack 0, .ret] [.u64, .bool] [] [.u64])
  [u64 7] == .returned [u64 7, .bool true]
#guard runData (dataFun #[.ldU64 1, .ldFalse, .pack 0, .ret] [.struct 0]) == .returned [valS 1 false]
#guard runData (dataFun #[.ldFalse, .ldU64 1, .pack 0, .ret] [.struct 0]) == .wrong .typeMismatch
#guard runData (dataFun #[.ldU64 1, .pack 0, .ret] [.struct 0]) == .wrong .stackUnderflow
#guard runData (dataFun #[.pack 1, .unpack 0, .ret]) == .wrong .typeMismatch
#guard runData (dataFun #[.pack 3, .ret]) == .wrong .badOperand
#guard runData (dataFun #[.packVariant 1, .unpack 3, .ret]) == .wrong .badOperand
#guard runData (dataFun #[.pack 9, .ret]) == .wrong .badIndex

-- Two definitions named `T`: the name resolves to neither.
#guard runFun ⟨#[mkModule 0x42 "m" [dataFun #[.pack 0, .ret]] [] []
    [{ name := "T" }, { name := "T" }]]⟩ 0 0 [] == .wrong .unresolvedStruct
#guard runFun ⟨#[mkModule 0x42 "m" [dataFun #[.pack 0, .ret]] [] []
    [{ name := "G", typeParameters := [⟨{}, false⟩] }]]⟩ 0 0 [] ==
  .wrong .badOperand
#guard runFun ⟨#[mkModule 0x42 "m" [dataFun #[.pack 0, .ret]] [] []
    [{ name := "N", native := true }]]⟩ 0 0 [] == .wrong .badOperand

/-! ### Abilities come from declarations -/

#guard runData (dataFun #[.pack 1, .stLoc 0, .copyLoc 0, .ret] [] [.struct 1]) ==
  .wrong .copyWithoutAbility
#guard runData (dataFun #[.pack 1, .pop, .ret]) == .wrong .dropWithoutAbility
#guard runData (dataFun #[.pack 1, .stLoc 0, .pack 1, .stLoc 0, .ret] [] [.struct 1]) ==
  .wrong .dropWithoutAbility
#guard runData (dataFun #[.pack 1, .stLoc 0, .ret] [] [.struct 1]) == .wrong .dropWithoutAbility
#guard runData (dataFun #[.pack 1, .stLoc 0, .moveLoc 0, .ret] [.struct 1] [.struct 1]) ==
  .returned [.struct typeR []]
#guard runData (dataFun #[.pack 1, .pack 1, .eq, .ret] [.bool]) == .wrong .dropWithoutAbility
#guard runData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .stLoc 0, .copyLoc 0, .moveLoc 0, .ret]
    [.struct 0, .struct 0] [.struct 0]) == .returned [valS 1 true, valS 1 true]

-- `Bad` is declared `copy, drop` but holds an `R`, which has neither: copying
-- or dropping it would copy or drop the `R`.
#guard runData (dataFun #[.pack 1, .pack 2, .stLoc 0, .copyLoc 0, .ret] [] [.struct 2]) ==
  .wrong .copyWithoutAbility
#guard runData (dataFun #[.pack 1, .pack 2, .pop, .ret]) == .wrong .dropWithoutAbility

/-! ### Structural equality -/

#guard runData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .ldU64 1, .ldTrue, .pack 0, .eq, .ret] [.bool]) ==
  .returned [.bool true]
#guard runData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .ldU64 2, .ldTrue, .pack 0, .eq, .ret] [.bool]) ==
  .returned [.bool false]
#guard runData (dataFun #[.ldU64 5, .packVariant 0, .packVariant 1, .neq, .ret] [.bool]) ==
  .returned [.bool true]
#guard runData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .packVariant 1, .eq, .ret] [.bool]) ==
  .wrong .typeMismatch

/-! ### Enums and R1 -/

#guard runData (dataFun #[.ldU64 5, .packVariant 0, .unpackVariant 0, .ret] [.u64]) ==
  .returned [u64 5]
#guard runData (dataFun #[.packVariant 1, .unpackVariant 0, .ret] [.u64]) == .failed .variantMismatch
#guard runData (dataFun #[.packVariant 1, .ret] [.struct 3]) == .returned [.variant typeE 1 []]
#guard runData (dataFun #[.packVariant 1, .unpack 0, .ret]) == .wrong .typeMismatch
#guard runData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .unpackVariant 0, .ret]) == .wrong .typeMismatch
#guard runData (dataFun #[.ldTrue, .packVariant 0, .ret]) == .wrong .typeMismatch
#guard runData (dataFun #[.packVariant 2, .ret]) == .wrong .badIndex
#guard runData (dataFun #[.packVariant 3, .ret]) == .wrong .badIndex
#guard runData (dataFun #[.packVariant 9, .ret]) == .wrong .badIndex

/-! ### Vectors and R2 -/

#guard runData (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .ret] [.vector .u64]) ==
  .returned [.vector (.int .u64) [u64 1, u64 2]]
#guard runData (dataFun #[.vecPack 3 0, .ret] [.vector .u64]) == .returned [.vector (.int .u64) []]
#guard runData (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .vecUnpack 3 2, .ret] [.u64, .u64]) ==
  .returned [u64 1, u64 2]
#guard runData (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .vecUnpack 3 3, .ret] [.u64, .u64]) ==
  .failed .vectorLength
#guard runData (dataFun #[.ldU64 1, .vecPack 3 2, .ret]) == .wrong .stackUnderflow
#guard runData (dataFun #[.ldTrue, .vecPack 3 1, .ret]) == .wrong .typeMismatch
#guard runData (dataFun #[.ldU64 1, .vecUnpack 3 1, .ret]) == .wrong .typeMismatch
#guard runData (dataFun #[.ldU64 1, .vecPack 3 1, .vecUnpack 7 1, .ret]) == .wrong .typeMismatch
#guard runData (dataFun #[.vecPack 6 0, .ret]) == .wrong .badOperand
#guard runData (dataFun #[.vecPack 9 0, .ret]) == .wrong .badOperand
#guard runData (dataFun #[.vecPack 99 0, .ret]) == .wrong .badIndex
#guard runData (dataFun #[.pack 1, .vecPack 5 1, .pop, .ret]) == .wrong .dropWithoutAbility
#guard runData (dataFun #[.vecPack 5 0, .pop, .ret]) == .wrong .dropWithoutAbility
#guard runData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .vecPack 8 1, .stLoc 0, .copyLoc 0, .moveLoc 0, .ret]
    [.vector (.struct 0), .vector (.struct 0)] [.vector (.struct 0)]) ==
  .returned [.vector (.struct typeS) [valS 1 true], .vector (.struct typeS) [valS 1 true]]
#guard runData (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .ldU64 1, .ldU64 2, .vecPack 3 2, .eq, .ret]
    [.bool]) == .returned [.bool true]

/-! ### Signers -/

def signerAt (a : Nat) : Value := .signer (BitVec.ofNat 256 a)

#guard runData (dataFun #[.moveLoc 0, .pop, .ret] [] [] [.signer]) [signerAt 1] == .returned []
#guard runData (dataFun #[.ret] [] [] [.signer]) [signerAt 1] == .returned []
#guard runData (dataFun #[.copyLoc 0, .pop, .ret] [] [] [.signer]) [signerAt 1] ==
  .wrong .copyWithoutAbility
#guard runData (dataFun #[.moveLoc 0, .ret] [.signer] [] [.signer]) [signerAt 1] ==
  .returned [signerAt 1]
#guard runData (dataFun #[.moveLoc 0, .moveLoc 1, .eq, .ret] [.bool] [] [.signer, .signer])
  [signerAt 1, signerAt 2] == .returned [.bool false]

/-! ### Vector constants

A vector constant is a ULEB128 length, canonical and at most `2^31 - 1`,
followed by its elements. -/

def loadConst (t : SignatureToken) (data : List UInt8) : RunResult :=
  runData (dataFun #[.ldConst 0, .ret] [t]) [] [{ type := t, data }]

def u8c (n : Nat) : Value := .int .u8 n

#guard loadConst (.vector .u8) [3, 1, 2, 3] == .returned [.vector (.int .u8) [u8c 1, u8c 2, u8c 3]]
#guard loadConst (.vector .u8) [0] == .returned [.vector (.int .u8) []]
#guard loadConst (.vector .u8) ([0x80, 0x01] ++ List.replicate 128 0) ==
  .returned [.vector (.int .u8) (List.replicate 128 (u8c 0))]
#guard loadConst (.vector (.vector .u8)) [2, 1, 7, 0] ==
  .returned [.vector (.vector (.int .u8)) [.vector (.int .u8) [u8c 7], .vector (.int .u8) []]]
#guard loadConst (.vector .address) ([1] ++ List.replicate 31 0 ++ [0x42]) ==
  .returned [.vector .address [.address 0x42]]
#guard loadConst (.vector .u8) [0x83, 0x00, 1, 2, 3] == .wrong .badConstant
#guard loadConst (.vector .u8) [5, 1, 2] == .wrong .badConstant
#guard loadConst (.vector .u8) [1, 1, 2] == .wrong .badConstant
#guard loadConst (.vector .bool) [1, 2] == .wrong .badConstant
#guard loadConst .signer (List.replicate 32 0) == .wrong .badConstant
#guard loadConst (.struct 0) [1, 0, 0, 0, 0, 0, 0, 0, 1] == .wrong .badConstant

#guard decodeLength [0x80, 0x01, 9] == some (128, [9])
#guard decodeLength [0xff, 0xff, 0xff, 0xff, 0x07] == some (2 ^ 31 - 1, [])
#guard decodeLength [0x80, 0x80, 0x80, 0x80, 0x08] == none
#guard decodeLength [0x80, 0x00] == none
#guard decodeLength [0x80] == none

/-! ### `AbortMsg`

The message is on top, the code below it. A message must be valid UTF-8 of at
most 1024 bytes. -/

/-- Abort with code 7 and a message of the given bytes, loaded from a
`vector<u8>` constant whose length takes one or two ULEB128 bytes (messages
shorter than `2^14`). -/
def abortWith (message : List UInt8) : RunResult :=
  let len : List UInt8 :=
    if message.length < 128 then [message.length.toUInt8]
    else [(message.length % 128 + 128).toUInt8, (message.length / 128).toUInt8]
  runData (dataFun #[.ldU64 7, .ldConst 0, .abortMsg]) []
    [{ type := .vector .u8, data := len ++ message }]

#guard abortWith [0x6f, 0x6f, 0x70, 0x73] == .failed (.abortMessage 7 [0x6f, 0x6f, 0x70, 0x73])
#guard abortWith [0xe2, 0x82, 0xac] == .failed (.abortMessage 7 [0xe2, 0x82, 0xac])
#guard abortWith [0xff] == .failed .invalidAbortMessage
#guard abortWith [0xed, 0xa0, 0x80] == .failed .invalidAbortMessage
#guard abortWith (List.replicate 1024 0x61) == .failed (.abortMessage 7 (List.replicate 1024 0x61))
#guard abortWith (List.replicate 1025 0x61) == .failed .invalidAbortMessage
#guard runData (dataFun #[.ldConst 0, .ldU64 7, .abortMsg]) [] [{ type := .vector .u8, data := [0] }] ==
  .wrong .typeMismatch
#guard runData (dataFun #[.ldU64 7, .ldU64 1, .vecPack 3 1, .abortMsg]) == .wrong .typeMismatch

/-! ### Struct types across modules

`a` defines `S has drop { u64 }`, `T has drop { u64 }` and `mk(u64): S`. `b`
imports `mk` declaring its result as `b`'s struct handle 0, which names
`a::S` or `a::T`, with the abilities `b` declares for it. K1 lets a handle
declare fewer abilities than the definition, never more; either way the
semantics takes abilities from the definition. -/

def structA : CompiledModule :=
  mkModule 0x1 "a"
    [{ name := "mk", params := [.u64], returns := [.struct 0], code := #[.copyLoc 0, .pack 0, .ret] }]
    [] [] [{ name := "S", abilities := { drop := true }, fields := [.u64] },
           { name := "T", abilities := { drop := true }, fields := [.u64] }]

def structB (name : String) (abilities : AbilitySet) (code : Array Bytecode)
    (locals : Signature := []) (returns : Signature := []) : CompiledModule :=
  mkModule 0x2 "b" [{ name := "main", returns, locals, code }]
    [{ address := 0x1, module := "a", name := "mk", params := [.u64], returns := [.struct 0] }]
    [] [] [{ address := 0x1, module := "a", name, abilities }]

#guard runFun ⟨#[structA, structB "S" { drop := true } #[.ldU64 3, .call 1, .pop, .ret]]⟩ 1 0 [] ==
  .returned []
#guard runFun ⟨#[structA, structB "T" { drop := true } #[.ldU64 3, .call 1, .pop, .ret]]⟩ 1 0 [] ==
  .wrong .linkMismatch
-- `b` claims `a::S` has `copy`, which link checking rejects (K1); the
-- definition decides.
#guard runFun ⟨#[structA, structB "S" ⟨true, true, false, false⟩
    #[.ldU64 3, .call 1, .stLoc 0, .copyLoc 0, .pop, .ret] [.struct 0]]⟩ 1 0 [] ==
  .wrong .copyWithoutAbility
-- `b` declares `a::S` without `drop`, which K1 admits: `b`'s code must then
-- move the value on, and may not drop it, though the semantics would allow
-- that too.
#guard runFun ⟨#[structA, structB "S" {} #[.ldU64 3, .call 1, .ret] [] [.struct 0]]⟩ 1 0 [] ==
  .returned [.struct ⟨⟨0x1, "a"⟩, "S", []⟩ [u64 3]]
#guard runFun ⟨#[structA, structB "S" {} #[.ldU64 3, .call 1, .pop, .ret]]⟩ 1 0 [] == .returned []

/-! ### Natives returning structs: the result must conform deeply -/

def nativeStructs : Program :=
  ⟨#[mkModule 0x1 "n"
      [{ name := "mkS", returns := [.struct 0], native := true },
       { name := "main", returns := [.struct 0], code := #[.call 0, .ret] }]
      [] [] [{ name := "S", abilities := ⟨true, true, true, false⟩, fields := [.u64, .bool] }]]⟩

def nativeS (fields : List Value) : NativeImpl := fun _ _ =>
  .returned [.struct ⟨⟨0x1, "n"⟩, "S", []⟩ fields] [] []

def runNativeS (fields : List Value) : Option RunResult :=
  (initialState nativeStructs ⟨0, 1, []⟩ []).map (run nativeStructs (nativeS fields) 100)

#guard runNativeS [u64 1, .bool true] == some (.returned [.struct ⟨⟨0x1, "n"⟩, "S", []⟩ [u64 1, .bool true]])
#guard runNativeS [u64 1] == some (.wrong .nativeContract)
#guard runNativeS [.int .u64 (2 ^ 64), .bool true] == some (.wrong .nativeContract)

/-! ## The ghost layer (Phase 2b)

`traceFun` runs a function with the ghost layer (`runTraced`). Its outcome
must be `run`'s on every program; its events record creation and
consumption with identities. `violations` evaluates the D2 and D3 checks
(`Spec.Properties.Ownership`, `Spec.Properties.Encapsulation`) at every state
and step of a run, and names those that fail. -/

/-- Flows that create every result node of a native. -/
def allFresh : NativeFlows := fun _ _ _ _ _ => .fresh

def traceFun (P : Program) (module index : Nat) (args : List Value)
    (natives : NativeImpl := noNatives) (ν : NativeFlows := allFresh) (fuel : Nat := 10000)
    (storage : GlobalStorage := []) (held : List Value := []) (typeArgs : List RuntimeType := []) :
    TracedResult :=
  match initialState P (fn module index typeArgs) args storage held with
  | some s => let (h, evs) := Shadow.initial s; runTraced P natives ν fuel s h evs
  | none => .unaccounted []

/-- The ghost layer changes no outcome, and accounts for every step that
continues. -/
def agrees (P : Program) (module index : Nat) (args : List Value)
    (natives : NativeImpl := noNatives) (ν : NativeFlows := allFresh)
    (storage : GlobalStorage := []) (held : List Value := []) (typeArgs : List RuntimeType := []) :
    Bool :=
  (traceFun P module index args natives ν (storage := storage) (held := held)
      (typeArgs := typeArgs)).erase ==
    (initialState P (fn module index typeArgs) args storage held).map (run P natives 10000)

/-- The D2 and D3 events of a run: creations and consumptions. -/
def _root_.MoveBytecode.TracedResult.ownershipEvents (r : TracedResult) : List Event :=
  r.events.filter fun e => match e with
    | .created .. | .consumed .. => true
    | .borrowed .. | .used .. | .accessed .. | .vacated .. | .entered .. | .frameEnded ..
    | .privileged .. => false

/-- The identities of the returned values, in pre-order. -/
def returnedIds : TracedResult → Option (List Nat)
  | .returned vals _ ids _ => some ((IdTree.occurrencesAll ids vals).map (·.1))
  | _ => none

/-- The D2a and D3 checks on the occurrences `occ` of a state, after events
`evs`. -/
def stateViolations (P : Program) (π : NativePolicy) (auth : List Address) (A : List StructId)
    (occ : List (Nat × Value)) (evs : List Event) : List String :=
  (if Ownership.unique P occ then [] else ["D2a"]) ++
  (if evs.all (Encapsulation.structsRespected π A) && Encapsulation.structsAuthentic π A occ evs
   then [] else ["D3a"]) ++
  (if evs.all (Encapsulation.signersRespected π auth) &&
      Encapsulation.signersAuthentic π auth occ evs then [] else ["D3b"]) ++
  (if evs.all (Encapsulation.closuresRespected P) && Encapsulation.closuresAuthentic P occ evs
   then [] else ["D3c"])

/-- The checks of D2 and D3 along a run, from state `s` with shadow `h` after
events `evs`: D2a and D3 at every state, D2b at every step, D2c when the run
returns, and accounting for every step that continues. -/
def violationsFrom (P : Program) (natives : NativeImpl) (ν : NativeFlows) (π : NativePolicy)
    (auth : List Address) (A : List StructId) : Nat → State → Shadow → List Event → List String
  | 0, _, _, _ => []
  | fuel + 1, s, h, evs =>
    let pre := h.occurrences s
    let here := stateViolations P π auth A pre evs
    let stepChecks (o : Outcome) (post : ShadowPost) (e : List Event) : List String :=
      (if Ownership.destroysNothing P π pre (post.occurrences o) e then [] else ["D2b"]) ++
        stateViolations P π auth A (post.occurrences o) (evs ++ e)
    match traceStep P natives ν s h with
    | (o@(.running s'), some (post@(.running h'), e)) =>
      here ++ stepChecks o post e ++ violationsFrom P natives ν π auth A fuel s' h' (evs ++ e)
    | (o@(.returned _ _), some (post, e)) =>
      here ++ stepChecks o post e ++
        (if Ownership.conserved P π (evs ++ e) (post.occurrences o) then [] else ["D2c"])
    | (.running _, _) => here ++ ["unaccounted"]
    | (.returned _ _, none) => here ++ ["unaccounted"]
    | (.failed _, _) => here
    | (.wrong _, _) => here

/-- The D2 and D3 checks that fail on a run of function `index` of module
`module` started by a transaction authenticated for `auth`, whose layer
accepts struct types `A`. -/
def violations (P : Program) (module index : Nat) (args : List Value)
    (natives : NativeImpl := noNatives) (ν : NativeFlows := allFresh) (π : NativePolicy := {})
    (auth : List Address := []) (A : List StructId := []) (storage : GlobalStorage := [])
    (held : List Value := []) (typeArgs : List RuntimeType := []) : List String :=
  match initialState P (fn module index typeArgs) args storage held with
  | some s =>
    let (h, evs) := Shadow.initial s
    (violationsFrom P natives ν π auth A 10000 s h evs).eraseDups
  | none => ["no initial state"]

/-! ### The ghost layer changes no outcome, and its checks hold

Every program of the data-by-value tests above, run with the ghost layer:
the outcome is `run`'s, every continuing step is accounted, and no D2 or D3
check fails before the run ends (the signers are authenticated). -/

def dataCases : List (Fun × List Value × List Constant) :=
  [(dataFun #[.copyLoc 0, .ldTrue, .pack 0, .unpack 0, .ret] [.u64, .bool] [] [.u64], [u64 7], []),
   (dataFun #[.ldU64 1, .ldFalse, .pack 0, .ret] [.struct 0], [], []),
   (dataFun #[.ldFalse, .ldU64 1, .pack 0, .ret] [.struct 0], [], []),
   (dataFun #[.pack 1, .unpack 0, .ret], [], []),
   (dataFun #[.pack 3, .ret], [], []),
   (dataFun #[.pack 1, .stLoc 0, .copyLoc 0, .ret] [] [.struct 1], [], []),
   (dataFun #[.pack 1, .pop, .ret], [], []),
   (dataFun #[.pack 1, .stLoc 0, .pack 1, .stLoc 0, .ret] [] [.struct 1], [], []),
   (dataFun #[.pack 1, .stLoc 0, .ret] [] [.struct 1], [], []),
   (dataFun #[.pack 1, .stLoc 0, .moveLoc 0, .ret] [.struct 1] [.struct 1], [], []),
   (dataFun #[.pack 1, .pack 1, .eq, .ret] [.bool], [], []),
   (dataFun #[.ldU64 1, .ldTrue, .pack 0, .stLoc 0, .copyLoc 0, .moveLoc 0, .ret]
      [.struct 0, .struct 0] [.struct 0], [], []),
   (dataFun #[.pack 1, .pack 2, .stLoc 0, .copyLoc 0, .ret] [] [.struct 2], [], []),
   (dataFun #[.pack 1, .pack 2, .pop, .ret], [], []),
   (dataFun #[.ldU64 1, .ldTrue, .pack 0, .ldU64 1, .ldTrue, .pack 0, .eq, .ret] [.bool], [], []),
   (dataFun #[.ldU64 5, .packVariant 0, .packVariant 1, .neq, .ret] [.bool], [], []),
   (dataFun #[.ldU64 5, .packVariant 0, .unpackVariant 0, .ret] [.u64], [], []),
   (dataFun #[.packVariant 1, .unpackVariant 0, .ret] [.u64], [], []),
   (dataFun #[.packVariant 1, .ret] [.struct 3], [], []),
   (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .ret] [.vector .u64], [], []),
   (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .vecUnpack 3 2, .ret] [.u64, .u64], [], []),
   (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .vecUnpack 3 3, .ret] [.u64, .u64], [], []),
   (dataFun #[.pack 1, .vecPack 5 1, .pop, .ret], [], []),
   (dataFun #[.vecPack 5 0, .pop, .ret], [], []),
   (dataFun #[.ldU64 1, .ldTrue, .pack 0, .vecPack 8 1, .stLoc 0, .copyLoc 0, .moveLoc 0, .ret]
      [.vector (.struct 0), .vector (.struct 0)] [.vector (.struct 0)], [], []),
   (dataFun #[.moveLoc 0, .pop, .ret] [] [] [.signer], [signerAt 1], []),
   (dataFun #[.ret] [] [] [.signer], [signerAt 1], []),
   (dataFun #[.copyLoc 0, .pop, .ret] [] [] [.signer], [signerAt 1], []),
   (dataFun #[.moveLoc 0, .ret] [.signer] [] [.signer], [signerAt 1], []),
   (dataFun #[.moveLoc 0, .moveLoc 1, .eq, .ret] [.bool] [] [.signer, .signer],
      [signerAt 1, signerAt 2], []),
   (dataFun #[.ldConst 0, .ret] [.vector (.vector .u8)], [],
      [{ type := .vector (.vector .u8), data := [2, 1, 7, 0] }]),
   (dataFun #[.ldU64 7, .ldConst 0, .abortMsg], [], [{ type := .vector .u8, data := [1, 0x61] }])]

#guard dataCases.all fun (f, args, constants) => agrees (withData f constants) 0 0 args
#guard dataCases.all fun (f, args, constants) =>
  violations (withData f constants) 0 0 args (auth := [1, 2]) == []

#guard agrees (single addFun) 0 0 [u64 2, u64 3]
#guard agrees (single addFun) 0 0 [u64 (2 ^ 64 - 1), u64 1]
#guard agrees (single sumFun) 0 0 [u64 10]
#guard agrees (single useAfterMove) 0 0 [u64 1]
#guard agrees ⟨#[moduleA, moduleB]⟩ 1 0 []
#guard agrees ⟨#[moduleA, moduleB]⟩ 1 1 []
#guard agrees ⟨#[lockA, lockB [.moduleLock]]⟩ 0 0 []
#guard agrees ⟨#[lockA, lockB []]⟩ 0 0 []
#guard agrees ⟨#[nativeModule]⟩ 0 1 [] twice
#guard agrees ⟨#[nativeModule]⟩ 0 1 [] badNative
#guard agrees nativeStructs 0 1 [] (nativeS [u64 1, .bool true])
#guard agrees nativeStructs 0 1 [] (nativeS [u64 1])
#guard agrees ⟨#[structA, structB "S" { drop := true } #[.ldU64 3, .call 1, .pop, .ret]]⟩ 1 0 []
#guard agrees ⟨#[structA, structB "S" {} #[.ldU64 3, .call 1, .ret] [] [.struct 0]]⟩ 1 0 []
#guard violations (single sumFun) 0 0 [u64 10] == []
#guard violations ⟨#[moduleA, moduleB]⟩ 1 0 [] == []
#guard violations ⟨#[structA, structB "S" {} #[.ldU64 3, .call 1, .ret] [] [.struct 0]]⟩ 1 0 [] == []

/-! ### Creation, transfer, copy, consumption -/

def byData (i : Bytecode) : Creator := .instruction dataModule i

-- `Pack` creates the struct value around the identities of its fields.
#guard (traceFun (withData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .ret] [.struct 0])) 0 0 []).ownershipEvents ==
  [.created 0 (u64 1) (byData (.ldU64 1)), .created 1 (.bool true) (byData .ldTrue),
   .created 2 (valS 1 true) (byData (.pack 0))]
#guard returnedIds (traceFun (withData (dataFun #[.ldU64 1, .ldTrue, .pack 0, .ret] [.struct 0])) 0 0 []) ==
  some [2, 0, 1]

-- Moving keeps the identity.
#guard returnedIds (traceFun (withData
    (dataFun #[.pack 1, .stLoc 0, .moveLoc 0, .ret] [.struct 1] [.struct 1])) 0 0 []) == some [0]

-- Copying a value with `copy` repeats its identities: D2a does not apply.
def copyS : Fun :=
  dataFun #[.ldU64 1, .ldTrue, .pack 0, .stLoc 0, .copyLoc 0, .moveLoc 0, .ret]
    [.struct 0, .struct 0] [.struct 0]

#guard returnedIds (traceFun (withData copyS) 0 0 []) == some [2, 0, 1, 2, 0, 1]
#guard (traceFun (withData copyS) 0 0 []).ownershipEvents.length == 3

-- An argument is created by the transaction; `CopyLoc` of a `u64` keeps its
-- identity; `Unpack` consumes the struct in its defining module and transfers
-- the fields.
#guard (traceFun (withData
    (dataFun #[.copyLoc 0, .ldTrue, .pack 0, .unpack 0, .ret] [.u64, .bool] [] [.u64])) 0 0 [u64 7]).ownershipEvents ==
  [.created 0 (u64 7) .transaction, .created 1 (.bool true) (byData .ldTrue),
   .created 2 (valS 7 true) (byData (.pack 0)), .consumed 2 (valS 7 true) (.unpack dataModule)]
#guard returnedIds (traceFun (withData
    (dataFun #[.copyLoc 0, .ldTrue, .pack 0, .unpack 0, .ret] [.u64, .bool] [] [.u64])) 0 0 [u64 7]) ==
  some [0, 1]

-- Enums: `UnpackVariant` consumes the enum value.
#guard (traceFun (withData (dataFun #[.ldU64 5, .packVariant 0, .unpackVariant 0, .ret] [.u64])) 0 0 []).ownershipEvents ==
  [.created 0 (u64 5) (byData (.ldU64 5)), .created 1 (.variant typeE 0 [u64 5]) (byData (.packVariant 0)),
   .consumed 1 (.variant typeE 0 [u64 5]) (.unpack dataModule)]

-- Vectors: `VecPack` creates the vector, `VecUnpack` consumes it.
#guard (traceFun (withData
    (dataFun #[.ldU64 1, .ldU64 2, .vecPack 3 2, .vecUnpack 3 2, .ret] [.u64, .u64])) 0 0 []).ownershipEvents ==
  [.created 0 (u64 1) (byData (.ldU64 1)), .created 1 (u64 2) (byData (.ldU64 2)),
   .created 2 (.vector (.int .u64) [u64 1, u64 2]) (byData (.vecPack 3 2)),
   .consumed 2 (.vector (.int .u64) [u64 1, u64 2]) .vecUnpack]

-- A constant is created whole by `LdConst`, node by node.
#guard (traceFun (withData (dataFun #[.ldConst 0, .ret] [.vector (.vector .u8)])
    [{ type := .vector (.vector .u8), data := [2, 1, 7, 0] }]) 0 0 []).ownershipEvents ==
  [.created 0 (.vector (.vector (.int .u8)) [.vector (.int .u8) [u8c 7], .vector (.int .u8) []])
     (byData (.ldConst 0)),
   .created 1 (.vector (.int .u8) [u8c 7]) (byData (.ldConst 0)),
   .created 2 (u8c 7) (byData (.ldConst 0)),
   .created 3 (.vector (.int .u8) []) (byData (.ldConst 0))]

-- A failed run rolls back: the abort itself records nothing.
#guard (traceFun (withData (dataFun #[.packVariant 1, .unpackVariant 0, .ret] [.u64])) 0 0 []).ownershipEvents ==
  [.created 0 (.variant typeE 1 []) (byData (.packVariant 1))]

-- Across modules: `a::mk` packs `a::S` in `a`; `b` only moves it.
#guard (traceFun ⟨#[structA, structB "S" {} #[.ldU64 3, .call 1, .ret] [] [.struct 0]]⟩ 1 0 []).ownershipEvents ==
  [.created 0 (u64 3) (.instruction ⟨0x2, "b"⟩ (.ldU64 3)),
   .created 1 (.struct ⟨⟨0x1, "a"⟩, "S", []⟩ [u64 3]) (.instruction ⟨0x1, "a"⟩ (.pack 0))]

/-! ### Natives and their flows

Module `0x1::r` defines `R {}`, with no abilities, and `mk(): R`. Module
`0x2::n` imports both and declares natives `keep(R): R`, `dup(R): (R, R)`,
`forge(): R` and `sig(): signer`, and functions calling them. -/

def typeRr : StructId := ⟨⟨0x1, "r"⟩, "R", []⟩
def valRr : Value := .struct typeRr []

def moduleR : CompiledModule :=
  mkModule 0x1 "r" [{ name := "mk", returns := [.struct 0], code := #[.pack 0, .ret] }] [] []
    [{ name := "R" }]

def moduleN : CompiledModule :=
  mkModule 0x2 "n"
    [{ name := "keep", params := [.struct 0], returns := [.struct 0], native := true },
     { name := "dup", params := [.struct 0], returns := [.struct 0, .struct 0], native := true },
     { name := "forge", returns := [.struct 0], native := true },
     { name := "sig", returns := [.signer], native := true },
     { name := "callKeep", returns := [.struct 0], code := #[.call 8, .call 0, .ret] },
     { name := "callDup", returns := [.struct 0, .struct 0], code := #[.call 8, .call 1, .ret] },
     { name := "callForge", returns := [.struct 0], code := #[.call 2, .ret] },
     { name := "callSig", returns := [.signer], code := #[.call 3, .ret] }]
    [{ address := 0x1, module := "r", name := "mk", returns := [.struct 0] }]
    [] [] [{ address := 0x1, module := "r", name := "R" }]

def programRN : Program := ⟨#[moduleR, moduleN]⟩

def nNatives : NativeImpl := fun f args =>
  match f.index, args with
  | 0, [r] => .returned [r] [] []
  | 1, [r] => .returned [r, r] [] []
  | 2, [] => .returned [valRr] [] []
  | 3, [] => .returned [signerAt 5] [] []
  | _, _ => .failed (.abort 1)

/-- `keep` and `dup` pass their argument on; `forge` and `sig` create. -/
def nFlows : NativeFlows := fun f _ _ _ p =>
  match f.index with
  | 0 | 1 => .arg 0 p
  | _ => .fresh

#guard agrees programRN 1 4 [] nNatives nFlows
#guard agrees programRN 1 5 [] nNatives nFlows
#guard agrees programRN 1 6 [] nNatives nFlows
#guard agrees programRN 1 7 [] nNatives nFlows

-- `keep` passes `R` on: same identity, nothing created or consumed.
#guard returnedIds (traceFun programRN 1 4 [] nNatives nFlows) == some [0]
#guard (traceFun programRN 1 4 [] nNatives nFlows).ownershipEvents ==
  [.created 0 valRr (.instruction ⟨0x1, "r"⟩ (.pack 0))]
#guard violations programRN 1 4 [] nNatives nFlows == []
#guard NativeCallRespects programRN {} ⟨1, 0, []⟩ ⟨0x2, "n"⟩ [valRr] [valRr]
  (nFlows ⟨1, 0, []⟩ [valRr] [valRr])

-- A plain native: the argument is consumed, the result created.
#guard (traceFun ⟨#[nativeModule]⟩ 0 1 [] twice).ownershipEvents ==
  [.created 0 (u64 21) (.instruction ⟨0x1, "n"⟩ (.ldU64 21)),
   .consumed 0 (u64 21) (.native ⟨0, 0, []⟩ ⟨0x1, "n"⟩ true),
   .created 1 (u64 42) (.native ⟨0, 0, []⟩ ⟨0x1, "n"⟩ [u64 21])]
#guard violations ⟨#[nativeModule]⟩ 0 1 [] twice == []

-- A native creating a value of its own module's type has the module's
-- privileges.
#guard violations nativeStructs 0 1 [] (nativeS [u64 1, .bool true]) == []

-- A native minting a signer needs the policy's permission.
def mints5 : NativePolicy := { mintsSigner := fun f _ a => f == ⟨1, 3, []⟩ && a == 5 }

#guard violations programRN 1 7 [] nNatives nFlows (π := mints5) == []

/-! ### Entry premises (E1, E2) -/

#guard (RuntimeType.vector (.int .u8)).IsTransactionArgument [] == true
#guard RuntimeType.signer.IsTransactionArgument [] == false
#guard (RuntimeType.vector .signer).IsTransactionArgument [] == false
#guard (RuntimeType.struct typeS).IsTransactionArgument [] == false
#guard (RuntimeType.struct typeS).IsTransactionArgument [typeS] == true

-- An authenticated signer, and an argument of an accepted struct type.
#guard violations (withData (dataFun #[.moveLoc 0, .ret] [.signer] [] [.signer])) 0 0 [signerAt 1]
  (auth := [1]) == []
#guard violations (withData (dataFun #[.moveLoc 0, .ret] [.struct 0] [] [.struct 0])) 0 0 [valS 1 true]
  (A := [typeS]) == []

-- An entry function with a return type starts no transaction (E1;
-- language-owner ruling 2026-11-03): the function above that returns its
-- signer, marked `entry`, has no `EntryInitial` state.
example (auth : List Address) (A : List StructId) (s₀ : State) :
    ¬ EntryInitial (withData { dataFun #[.moveLoc 0, .ret] [.signer] [] [.signer] with isEntry := true })
      auth A s₀ := by
  rintro ⟨-, ⟨_ | _, _ | _, _⟩, _, _, _, _, -, hret, -⟩ <;> nomatch hret

/-! ## References and global storage (Phase 3, stage A)

Module `0x42::m` declares seven types:

- struct 0, `S has copy, drop, store { u64, bool }`;
- struct 1, `N {}`, with no abilities;
- struct 2, `W has copy, drop { S }`;
- struct 3, `enum E has copy, drop { A(u64), B(u64), C }`;
- struct 4, `Res has key { u64 }`;
- struct 5, `K has key { N }`, whose `key` its field does not justify (an L6
  violation);
- struct 6, `T has store { u64 }`, without `key`.

Field handles: 0 is `S.0`, 1 `S.1`, 2 `W.0`, 3 `Res.0`; 4 names a field of
the enum and 5 a field `S` does not have. Variant field handles: 0 is field 0
of `E::A` and `E::B`, 1 of `E::A` only, 2 of `E::C`, which has none. Variant
handles: 0 `E::A`, 1 `E::B`, 2 `E::C`. With one function, signature 3 is
`[u64]` and 4 is `[bool]`. -/

def refStructs : List StructDecl :=
  [{ name := "S", abilities := ⟨true, true, true, false⟩, fields := [.u64, .bool] },
   { name := "N" },
   { name := "W", abilities := ⟨true, true, false, false⟩, fields := [.struct 0] },
   { name := "E", abilities := ⟨true, true, false, false⟩,
     variants := some [("A", [.u64]), ("B", [.u64]), ("C", [])] },
   { name := "Res", abilities := { key := true }, fields := [.u64] },
   { name := "K", abilities := { key := true }, fields := [.struct 1] },
   { name := "T", abilities := { store := true }, fields := [.u64] }]

def refModule (funs : List Fun) (imports : List Import := []) (constants : List Constant := []) :
    CompiledModule :=
  mkModule 0x42 "m" funs imports constants refStructs [] [⟨3, 0⟩, ⟨3, 1⟩, ⟨3, 2⟩]
    [[.u64], [.bool]] [⟨0, 0⟩, ⟨0, 1⟩, ⟨2, 0⟩, ⟨4, 0⟩, ⟨3, 0⟩, ⟨0, 7⟩]
    [⟨3, [0, 1], 0⟩, ⟨3, [0], 0⟩, ⟨3, [2], 0⟩]

def withRefs (f : Fun) : Program := ⟨#[refModule [f]]⟩

def runRefs (f : Fun) (args : List Value := []) (storage : GlobalStorage := []) : RunResult :=
  runFun (withRefs f) 0 0 args (storage := storage)

def typeN : StructId := ⟨dataModule, "N", []⟩
def typeW : StructId := ⟨dataModule, "W", []⟩
def typeRes : StructId := ⟨dataModule, "Res", []⟩
def typeT : StructId := ⟨dataModule, "T", []⟩
def typeK : StructId := ⟨dataModule, "K", []⟩
def valW (s : Value) : Value := .struct typeW [s]
def valE (tag : Nat) (fields : List Value) : Value := .variant typeE tag fields
def valRes (n : Nat) : Value := .struct typeRes [u64 n]
def vecU64 (ns : List Nat) : Value := .vector (.int .u64) (ns.map u64)

/-! ### Locals: borrow, read, write, freeze -/

#guard runRefs (dataFun #[.immBorrowLoc 0, .readRef, .ret] [.u64] [] [.u64]) [u64 5] ==
  .returned [u64 5]
#guard runRefs (dataFun #[.ldU64 7, .mutBorrowLoc 0, .writeRef, .moveLoc 0, .ret] [.u64] [] [.u64])
  [u64 5] == .returned [u64 7]
#guard runRefs (dataFun #[.mutBorrowLoc 0, .freezeRef, .readRef, .ret] [.u64] [] [.u64]) [u64 5] ==
  .returned [u64 5]
-- Only mutable references are written or frozen.
#guard runRefs (dataFun #[.ldU64 7, .immBorrowLoc 0, .writeRef, .ret] [] [] [.u64]) [u64 5] ==
  .wrong .immutableReference
#guard runRefs (dataFun #[.immBorrowLoc 0, .freezeRef, .pop, .ret] [] [] [.u64]) [u64 5] ==
  .wrong .immutableReference
#guard runRefs (dataFun #[.ldU64 1, .readRef, .ret] [.u64]) == .wrong .typeMismatch
#guard runRefs (dataFun #[.ldU64 1, .freezeRef, .ret]) == .wrong .typeMismatch
#guard runRefs (dataFun #[.ldTrue, .mutBorrowLoc 0, .writeRef, .ret] [] [] [.u64]) [u64 5] ==
  .wrong .typeMismatch
-- Reading copies (`copy`); writing discards the old value (`drop`). A signer
-- cannot be copied "through a dereference `*`" either.
#guard runRefs (dataFun #[.pack 1, .stLoc 0, .immBorrowLoc 0, .readRef, .ret] [] [.struct 1]) ==
  .wrong .copyWithoutAbility
#guard runRefs (dataFun #[.immBorrowLoc 0, .readRef, .ret] [.signer] [] [.signer]) [signerAt 1] ==
  .wrong .copyWithoutAbility
#guard runRefs (dataFun #[.pack 1, .stLoc 0, .pack 1, .mutBorrowLoc 0, .writeRef, .ret] [] [.struct 1]) ==
  .wrong .dropWithoutAbility
-- A borrowed local must exist, be available, and not hold a reference.
#guard runRefs (dataFun #[.immBorrowLoc 0, .ret] [] [.u64]) == .wrong .unavailableLocal
#guard runRefs (dataFun #[.immBorrowLoc 5, .ret]) == .wrong .localOutOfRange
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .immBorrowLoc 1, .ret] [] [.reference .u64] [.u64])
  [u64 5] == .wrong .typeMismatch
-- References have `copy` and `drop`, whatever they designate.
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .copyLoc 1, .readRef, .moveLoc 1, .readRef, .add, .ret]
  [.u64] [.reference .u64] [.u64]) [u64 5] == .returned [u64 10]
#guard runRefs (dataFun #[.pack 1, .stLoc 0, .immBorrowLoc 0, .pop, .moveLoc 0, .ret] [.struct 1]
  [.struct 1]) == .returned [.struct typeN []]
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .ret] [] [.reference .u64] [.u64]) [u64 5] ==
  .returned []

/-! ### When a location in a local stops being valid -/

-- Moving the local out invalidates every location in it, for good.
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .moveLoc 0, .pop, .moveLoc 1, .readRef, .ret]
  [.u64] [.reference .u64] [.u64]) [u64 5] == .wrong .invalidReference
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .moveLoc 0, .stLoc 0, .moveLoc 1, .readRef, .ret]
  [.u64] [.reference .u64] [.u64]) [u64 5] == .wrong .invalidReference
-- A reference taken after the local is assigned again is valid.
#guard runRefs (dataFun #[.moveLoc 0, .stLoc 0, .immBorrowLoc 0, .readRef, .ret] [.u64] [] [.u64])
  [u64 5] == .returned [u64 5]
-- Overwriting the local keeps the local itself valid...
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .ldU64 9, .stLoc 0, .moveLoc 1, .readRef, .ret]
  [.u64] [.reference .u64] [.u64]) [u64 5] == .returned [u64 9]
-- ... but not the locations inside its old value.
#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 0, .stLoc 1, .ldU64 2, .ldFalse, .pack 0,
    .stLoc 0, .moveLoc 1, .readRef, .ret] [.u64] [.reference .u64] [.struct 0]) [valS 1 true] ==
  .wrong .invalidReference
-- A write through a reference invalidates the locations strictly inside the
-- old value, not the written location, its copies, or locations beside it.
def writeParent (finish : Array Bytecode) (returns : Signature) : Fun :=
  dataFun (#[.mutBorrowLoc 0, .stLoc 1, .copyLoc 1, .mutBorrowField 0, .stLoc 2,
      .ldU64 2, .ldFalse, .pack 0, .copyLoc 1, .writeRef] ++ finish)
    returns [.mutableReference (.struct 0), .mutableReference .u64] [.struct 0]

#guard runRefs (writeParent #[.moveLoc 2, .readRef, .ret] [.u64]) [valS 1 true] ==
  .wrong .invalidReference
#guard runRefs (writeParent #[.moveLoc 1, .readRef, .ret] [.struct 0]) [valS 1 true] ==
  .returned [valS 2 false]
#guard runRefs (dataFun #[.ldU64 9, .mutBorrowLoc 0, .mutBorrowField 0, .writeRef, .moveLoc 0, .ret]
  [.struct 0] [] [.struct 0]) [valS 1 true] == .returned [valS 9 true]
#guard runRefs (dataFun #[.mutBorrowLoc 0, .stLoc 1, .ldU64 3, .copyLoc 1, .writeRef, .moveLoc 1,
    .readRef, .ret] [.u64] [.mutableReference .u64] [.u64]) [u64 5] == .returned [u64 3]
#guard runRefs (dataFun #[.mutBorrowLoc 0, .mutBorrowField 0, .stLoc 1, .mutBorrowLoc 0,
    .mutBorrowField 1, .stLoc 2, .ldU64 4, .moveLoc 1, .writeRef, .moveLoc 2, .readRef, .ret]
  [.bool] [.mutableReference .u64, .mutableReference .bool] [.struct 0]) [valS 1 true] ==
  .returned [.bool true]

/-! ### Fields -/

#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 2, .immBorrowField 1, .readRef, .ret]
  [.bool] [] [.struct 2]) [valW (valS 1 true)] == .returned [.bool true]
#guard runRefs (dataFun #[.ldU64 42, .mutBorrowLoc 0, .mutBorrowField 2, .mutBorrowField 0, .writeRef,
    .moveLoc 0, .ret] [.struct 2] [] [.struct 2]) [valW (valS 1 true)] ==
  .returned [valW (valS 42 true)]
-- `ImmBorrowField` accepts `&mut`; `MutBorrowField` needs it.
#guard runRefs (dataFun #[.mutBorrowLoc 0, .immBorrowField 0, .readRef, .ret] [.u64] [] [.struct 0])
  [valS 1 true] == .returned [u64 1]
#guard runRefs (dataFun #[.immBorrowLoc 0, .mutBorrowField 0, .readRef, .ret] [.u64] [] [.struct 0])
  [valS 1 true] == .wrong .immutableReference
-- The reference must be to the field's struct.
#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 2, .ret] [] [] [.struct 0]) [valS 1 true] ==
  .wrong .typeMismatch
#guard runRefs (dataFun #[.ldU64 1, .immBorrowField 0, .ret]) == .wrong .typeMismatch
-- A field handle must name a field of a declared struct (L1).
#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 4, .ret] [] [] [.struct 3]) [valE 0 [u64 5]] ==
  .wrong .badIndex
#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 5, .ret] [] [] [.struct 0]) [valS 1 true] ==
  .wrong .badIndex
#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 99, .ret] [] [] [.struct 0]) [valS 1 true] ==
  .wrong .badIndex
-- Borrowing inside a location that is no longer valid accesses it.
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .moveLoc 0, .pop, .moveLoc 1, .immBorrowField 0, .ret]
  [] [.reference (.struct 0)] [.struct 0]) [valS 1 true] == .wrong .invalidReference

/-! ### Variant fields (R1) and `TestVariant` -/

def readVariantField (h : Nat) : Fun :=
  dataFun #[.immBorrowLoc 0, .immBorrowVariantField h, .readRef, .ret] [.u64] [] [.struct 3]

#guard runRefs (readVariantField 0) [valE 0 [u64 5]] == .returned [u64 5]
#guard runRefs (readVariantField 0) [valE 1 [u64 6]] == .returned [u64 6]
#guard runRefs (readVariantField 0) [valE 2 []] == .failed .variantMismatch
#guard runRefs (readVariantField 1) [valE 1 [u64 6]] == .failed .variantMismatch
#guard runRefs (readVariantField 2) [valE 2 []] == .wrong .badIndex
#guard runRefs (dataFun #[.immBorrowLoc 0, .mutBorrowVariantField 0, .readRef, .ret] [.u64] []
  [.struct 3]) [valE 0 [u64 5]] == .wrong .immutableReference
#guard runRefs (dataFun #[.ldU64 9, .mutBorrowLoc 0, .mutBorrowVariantField 0, .writeRef, .moveLoc 0,
  .ret] [.struct 3] [] [.struct 3]) [valE 1 [u64 6]] == .returned [valE 1 [u64 9]]
-- Changing the enum value, to another variant or to the same one, invalidates
-- the locations inside it.
def replaceEnum (replacement : Array Bytecode) : Fun :=
  dataFun (#[.mutBorrowLoc 0, .stLoc 1, .copyLoc 1, .mutBorrowVariantField 0, .stLoc 2] ++
      replacement ++ #[.moveLoc 1, .writeRef, .moveLoc 2, .readRef, .ret])
    [.u64] [.mutableReference (.struct 3), .mutableReference .u64] [.struct 3]

#guard runRefs (replaceEnum #[.packVariant 2]) [valE 0 [u64 5]] == .wrong .invalidReference
#guard runRefs (replaceEnum #[.ldU64 7, .packVariant 0]) [valE 0 [u64 5]] == .wrong .invalidReference

def testVariant (h : Nat) (borrow : Bytecode := .immBorrowLoc 0) (param : SignatureToken := .struct 3) :
    Fun :=
  dataFun #[borrow, .testVariant h, .ret] [.bool] [] [param]

#guard runRefs (testVariant 0) [valE 0 [u64 5]] == .returned [.bool true]
#guard runRefs (testVariant 0) [valE 1 [u64 5]] == .returned [.bool false]
#guard runRefs (testVariant 2 (.mutBorrowLoc 0)) [valE 2 []] == .returned [.bool true]
#guard runRefs (testVariant 0 (param := .struct 0)) [valS 1 true] == .wrong .typeMismatch
#guard runRefs (testVariant 9) [valE 0 [u64 5]] == .wrong .badIndex

/-! ### Vectors through references (R6) -/

#guard runRefs (dataFun #[.immBorrowLoc 0, .vecLen 3, .ret] [.u64] [] [.vector .u64]) [vecU64 [1, 2, 3]] ==
  .returned [u64 3]
#guard runRefs (dataFun #[.mutBorrowLoc 0, .vecLen 3, .ret] [.u64] [] [.vector .u64]) [vecU64 []] ==
  .returned [u64 0]
#guard runRefs (dataFun #[.immBorrowLoc 0, .ldU64 1, .vecImmBorrow 3, .readRef, .ret] [.u64] []
  [.vector .u64]) [vecU64 [1, 2, 3]] == .returned [u64 2]
#guard runRefs (dataFun #[.immBorrowLoc 0, .ldU64 3, .vecImmBorrow 3, .readRef, .ret] [.u64] []
  [.vector .u64]) [vecU64 [1, 2, 3]] == .failed .vectorIndex
#guard runRefs (dataFun #[.ldU64 9, .mutBorrowLoc 0, .ldU64 0, .vecMutBorrow 3, .writeRef, .moveLoc 0, .ret]
  [.vector .u64] [] [.vector .u64]) [vecU64 [1, 2, 3]] == .returned [vecU64 [9, 2, 3]]
#guard runRefs (dataFun #[.immBorrowLoc 0, .ldU64 0, .vecMutBorrow 3, .ret] [] [] [.vector .u64])
  [vecU64 [1, 2, 3]] == .wrong .immutableReference
#guard runRefs (dataFun #[.mutBorrowLoc 0, .ldU64 4, .vecPushBack 3, .moveLoc 0, .ret] [.vector .u64] []
  [.vector .u64]) [vecU64 [1, 2, 3]] == .returned [vecU64 [1, 2, 3, 4]]
#guard runRefs (dataFun #[.mutBorrowLoc 0, .ldTrue, .vecPushBack 3, .ret] [] [] [.vector .u64])
  [vecU64 [1]] == .wrong .typeMismatch
#guard runRefs (dataFun #[.immBorrowLoc 0, .ldU64 4, .vecPushBack 3, .ret] [] [] [.vector .u64])
  [vecU64 [1]] == .wrong .immutableReference
#guard runRefs (dataFun #[.mutBorrowLoc 0, .vecPopBack 3, .moveLoc 0, .ret] [.u64, .vector .u64] []
  [.vector .u64]) [vecU64 [1, 2, 3]] == .returned [u64 3, vecU64 [1, 2]]
#guard runRefs (dataFun #[.mutBorrowLoc 0, .vecPopBack 3, .ret] [.u64] [] [.vector .u64]) [vecU64 []] ==
  .failed .emptyVector
#guard runRefs (dataFun #[.mutBorrowLoc 0, .ldU64 0, .ldU64 2, .vecSwap 3, .moveLoc 0, .ret] [.vector .u64]
  [] [.vector .u64]) [vecU64 [1, 2, 3]] == .returned [vecU64 [3, 2, 1]]
#guard runRefs (dataFun #[.mutBorrowLoc 0, .ldU64 3, .ldU64 0, .vecSwap 3, .ret] [] [] [.vector .u64])
  [vecU64 [1, 2, 3]] == .failed .vectorIndex
#guard runRefs (dataFun #[.mutBorrowLoc 0, .ldU64 0, .ldU64 3, .vecSwap 3, .ret] [] [] [.vector .u64])
  [vecU64 [1, 2, 3]] == .failed .vectorIndex
#guard runRefs (dataFun #[.mutBorrowLoc 0, .ldTrue, .ldU64 0, .vecSwap 3, .ret] [] [] [.vector .u64])
  [vecU64 [1, 2, 3]] == .wrong .typeMismatch
-- The element type operand must be the vector's.
#guard runRefs (dataFun #[.immBorrowLoc 0, .vecLen 4, .ret] [.u64] [] [.vector .u64]) [vecU64 [1]] ==
  .wrong .typeMismatch
-- Popping, swapping or pushing changes the vector in place: references to its
-- elements become invalid, even when an element is at that index again.
def changeVector (change : Array Bytecode) : Fun :=
  dataFun (#[.immBorrowLoc 0, .ldU64 2, .vecImmBorrow 3, .stLoc 1] ++ change ++
      #[.moveLoc 1, .readRef, .ret])
    [.u64] [.reference .u64] [.vector .u64]

#guard runRefs (changeVector #[]) [vecU64 [1, 2, 3]] == .returned [u64 3]
#guard runRefs (changeVector #[.immBorrowLoc 0, .vecLen 3, .pop, .mutBorrowLoc 0, .ldU64 0,
  .vecImmBorrow 3, .readRef, .pop]) [vecU64 [1, 2, 3]] == .returned [u64 3]
#guard runRefs (changeVector #[.mutBorrowLoc 0, .vecPopBack 3, .pop, .mutBorrowLoc 0, .ldU64 3,
  .vecPushBack 3]) [vecU64 [1, 2, 3]] == .wrong .invalidReference
#guard runRefs (changeVector #[.mutBorrowLoc 0, .ldU64 2, .ldU64 2, .vecSwap 3]) [vecU64 [1, 2, 3]] ==
  .wrong .invalidReference
#guard runRefs (changeVector #[.mutBorrowLoc 0, .ldU64 4, .vecPushBack 3]) [vecU64 [1, 2, 3]] ==
  .wrong .invalidReference

/-! ### Equality on references

References compare the values they designate, which need neither `copy` nor
`drop`; both operands must have the same type, mutability included. -/

#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowLoc 1, .eq, .ret] [.bool] [] [.u64, .u64])
  [u64 5, u64 5] == .returned [.bool true]
#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowLoc 1, .neq, .ret] [.bool] [] [.u64, .u64])
  [u64 5, u64 6] == .returned [.bool true]
#guard runRefs (dataFun #[.immBorrowLoc 0, .mutBorrowLoc 1, .eq, .ret] [.bool] [] [.u64, .u64])
  [u64 5, u64 5] == .wrong .typeMismatch
#guard runRefs (dataFun #[.pack 1, .stLoc 0, .immBorrowLoc 0, .immBorrowLoc 0, .eq, .moveLoc 0, .unpack 1,
  .ret] [.bool] [.struct 1]) == .returned [.bool true]
#guard runRefs (dataFun #[.immBorrowLoc 0, .stLoc 1, .moveLoc 0, .pop, .copyLoc 1, .moveLoc 1, .eq, .ret]
  [.bool] [.reference .u64] [.u64]) [u64 5] == .wrong .invalidReference

/-! ### Calls, returns and frames

Function handles: 0 `id(&u64): &u64`, 1 `set(&mut u64)`, 2 `dangle(): &u64`,
which returns a reference to its own local, 3 `read(&u64): u64`, 4
`retMut(&mut u64): &u64`, which returns a `&mut` where `&` is declared, 13
`dangleMut(): &mut u64` and 17 `borrowEnum(): &E`, which also return
references to their own locals; the others call them. -/

def callFuns : List Fun :=
  [{ name := "id", params := [.reference .u64], returns := [.reference .u64], code := #[.moveLoc 0, .ret] },
   { name := "set", params := [.mutableReference .u64],
     code := #[.ldU64 9, .moveLoc 0, .writeRef, .ret] },
   { name := "dangle", returns := [.reference .u64], locals := [.u64],
     code := #[.ldU64 1, .stLoc 0, .immBorrowLoc 0, .ret] },
   { name := "read", params := [.reference .u64], returns := [.u64], code := #[.moveLoc 0, .readRef, .ret] },
   { name := "retMut", params := [.mutableReference .u64], returns := [.reference .u64],
     code := #[.moveLoc 0, .ret] },
   { name := "useId", params := [.u64], returns := [.u64],
     code := #[.immBorrowLoc 0, .call 0, .readRef, .ret] },
   { name := "useSet", params := [.u64], returns := [.u64],
     code := #[.mutBorrowLoc 0, .call 1, .moveLoc 0, .ret] },
   { name := "useDangle", returns := [.u64], code := #[.call 2, .readRef, .ret] },
   { name := "readDangle", returns := [.u64], code := #[.call 2, .call 3, .ret] },
   { name := "passMut", params := [.u64], returns := [.u64],
     code := #[.mutBorrowLoc 0, .call 0, .readRef, .ret] },
   { name := "useRetMut", params := [.u64], returns := [.u64],
     code := #[.mutBorrowLoc 0, .call 4, .readRef, .ret] },
   { name := "keepDangle", locals := [.reference .u64],
     code := #[.call 2, .stLoc 0, .copyLoc 0, .pop, .call 2, .pop, .ret] },
   { name := "storeMut", params := [.u64], locals := [.reference .u64],
     code := #[.mutBorrowLoc 0, .stLoc 1, .ret] },
   { name := "dangleMut", returns := [.mutableReference .u64], locals := [.u64],
     code := #[.ldU64 1, .stLoc 0, .mutBorrowLoc 0, .ret] },
   { name := "freezeDangle", code := #[.call 13, .freezeRef, .pop, .ret] },
   { name := "readFrozenDangle", returns := [.u64], code := #[.call 13, .freezeRef, .readRef, .ret] },
   { name := "testDangle", returns := [.bool], locals := [.struct 3],
     code := #[.packVariant 2, .stLoc 0, .call 17, .testVariant 2, .ret] },
   { name := "borrowEnum", returns := [.reference (.struct 3)], locals := [.struct 3],
     code := #[.packVariant 2, .stLoc 0, .immBorrowLoc 0, .ret] }]

def callProgram : Program := ⟨#[refModule callFuns]⟩

-- References to the caller's locals are passed and returned; a callee writes
-- the caller's local through one.
#guard runFun callProgram 0 5 [u64 5] == .returned [u64 5]
#guard runFun callProgram 0 6 [u64 5] == .returned [u64 9]
-- A reference into a frame that returned designates nothing, even inside a
-- later frame at the same depth; returning, copying and dropping it do not
-- access it.
#guard runFun callProgram 0 7 [] == .wrong .invalidReference
#guard runFun callProgram 0 8 [] == .wrong .invalidReference
#guard runFun callProgram 0 11 [] == .returned []
#guard runFun callProgram 0 14 [] == .returned []
#guard runFun callProgram 0 15 [] == .wrong .invalidReference
#guard runFun callProgram 0 16 [] == .wrong .invalidReference
-- A `&mut` value does not conform to a `&` type: bytecode freezes explicitly.
#guard runFun callProgram 0 9 [u64 5] == .wrong .typeMismatch
#guard runFun callProgram 0 10 [u64 5] == .wrong .typeMismatch
#guard runFun callProgram 0 12 [u64 5] == .wrong .typeMismatch

/-! ### Global storage (R3, R5)

`Res has key { u64 }` at address `0x1`, published, read, written and moved
out. A returning execution commits its storage; a failing one rolls it back
(`RunResult.committed`). -/

def atOne : GlobalStorage := [⟨1, typeRes, valRes 5⟩]

def publish : Fun := dataFun #[.immBorrowLoc 0, .ldU64 5, .pack 4, .moveTo 4, .ret] [] [] [.signer]

#guard runRefs publish [signerAt 1] == .returned [] atOne
#guard (runRefs publish [signerAt 1]).committed [] == atOne
#guard runRefs publish [signerAt 1] atOne == .failed .resourceExists
#guard (runRefs publish [signerAt 1] atOne).committed atOne == atOne
#guard runRefs publish [signerAt 2] atOne == .returned [] (atOne ++ [⟨2, typeRes, valRes 5⟩])
-- Rollback: publishing, then aborting, commits nothing.
#guard runRefs (dataFun #[.immBorrowLoc 0, .ldU64 5, .pack 4, .moveTo 4, .ldU64 3, .abort] [] [] [.signer])
  [signerAt 1] == .failed (.abort 3)
#guard (runRefs (dataFun #[.immBorrowLoc 0, .ldU64 5, .pack 4, .moveTo 4, .ldU64 3, .abort] [] []
  [.signer]) [signerAt 1]).committed [] == []
-- `MoveTo` takes `&signer`, a value of the type, and a type with `key` all
-- the way down.
#guard runRefs (dataFun #[.mutBorrowLoc 0, .ldU64 5, .pack 4, .moveTo 4, .ret] [] [] [.signer])
  [signerAt 1] == .wrong .typeMismatch
#guard runRefs (dataFun #[.immBorrowLoc 0, .ldU64 5, .moveTo 4, .ret] [] [] [.signer]) [signerAt 1] ==
  .wrong .typeMismatch
#guard runRefs (dataFun #[.immBorrowLoc 0, .ldU64 5, .pack 6, .moveTo 6, .ret] [] [] [.signer])
  [signerAt 1] == .wrong .globalWithoutKey
#guard runRefs (dataFun #[.immBorrowLoc 0, .pack 1, .pack 5, .moveTo 5, .ret] [] [] [.signer])
  [signerAt 1] == .wrong .globalWithoutKey

def existsAt : Fun := dataFun #[.copyLoc 0, .exists_ 4, .ret] [.bool] [] [.address]

#guard runRefs existsAt [.address 1] atOne == .returned [.bool true] atOne
#guard runRefs existsAt [.address 2] atOne == .returned [.bool false] atOne
#guard runRefs (dataFun #[.copyLoc 0, .exists_ 6, .ret] [.bool] [] [.address]) [.address 1] ==
  .wrong .globalWithoutKey
#guard runRefs (dataFun #[.ldU64 1, .exists_ 4, .ret] [.bool]) == .wrong .typeMismatch

def moveFromAt : Fun := dataFun #[.copyLoc 0, .moveFrom 4, .unpack 4, .ret] [.u64] [] [.address]

#guard runRefs moveFromAt [.address 1] atOne == .returned [u64 5] []
#guard runRefs moveFromAt [.address 2] atOne == .failed .missingResource
#guard (runRefs moveFromAt [.address 2] atOne).committed atOne == atOne

#guard runRefs (dataFun #[.copyLoc 0, .immBorrowGlobal 4, .immBorrowField 3, .readRef, .ret] [.u64] []
  [.address]) [.address 1] atOne == .returned [u64 5] atOne
#guard runRefs (dataFun #[.ldU64 9, .copyLoc 0, .mutBorrowGlobal 4, .mutBorrowField 3, .writeRef, .ret]
  [] [] [.address]) [.address 1] atOne == .returned [] [⟨1, typeRes, valRes 9⟩]
#guard runRefs (dataFun #[.copyLoc 0, .immBorrowGlobal 4, .ret] [.reference (.struct 4)] [] [.address])
  [.address 2] atOne == .failed .missingResource
#guard runRefs (dataFun #[.copyLoc 0, .mutBorrowGlobal 4, .ret] [.mutableReference (.struct 4)] []
  [.address]) [.address 2] atOne == .failed .missingResource
#guard runRefs (dataFun #[.ldU64 1, .immBorrowGlobal 4, .ret] [.reference (.struct 4)]) ==
  .wrong .typeMismatch
#guard runRefs (dataFun #[.copyLoc 0, .immBorrowGlobal 6, .ret] [.reference (.struct 6)] [] [.address])
  [.address 1] == .wrong .globalWithoutKey
-- Moving a resource out invalidates the references to it, also after it is
-- published again.
#guard runRefs (dataFun #[.copyLoc 0, .immBorrowGlobal 4, .stLoc 1, .copyLoc 0, .moveFrom 4, .unpack 4,
    .pop, .moveLoc 1, .immBorrowField 3, .readRef, .ret] [.u64] [.reference (.struct 4)] [.address])
  [.address 1] atOne == .wrong .invalidReference
#guard runRefs (dataFun #[.copyLoc 1, .immBorrowGlobal 4, .stLoc 2, .copyLoc 1, .moveFrom 4, .stLoc 3,
    .immBorrowLoc 0, .moveLoc 3, .moveTo 4, .moveLoc 2, .immBorrowField 3, .readRef, .ret] [.u64]
    [.reference (.struct 4), .struct 4] [.signer, .address])
  [signerAt 1, .address 1] atOne == .wrong .invalidReference

-- R3: `a::start` calls `b::f`, which calls back into `a::check`, and
-- `a::startPeek` calls `b::g`, which calls back into `a::peek`; `a` is then
-- active twice, and its resources are locked. The lock comes before the
-- existence check R5.
def constOne : Constant := { type := .address, data := List.replicate 31 0 ++ [1] }

def reenterA : CompiledModule :=
  mkModule 0x1 "a"
    [{ name := "start", returns := [.bool], code := #[.call 4, .ret] },
     { name := "check", returns := [.bool], code := #[.ldConst 0, .exists_ 0, .ret] },
     { name := "startPeek", code := #[.call 5, .ret] },
     { name := "peek", code := #[.ldConst 0, .immBorrowGlobal 0, .pop, .ret] }]
    [{ address := 0x2, module := "b", name := "f", returns := [.bool] },
     { address := 0x2, module := "b", name := "g" }] [constOne]
    [{ name := "Res", abilities := { key := true }, fields := [.u64] }]

def reenterB : CompiledModule :=
  mkModule 0x2 "b"
    [{ name := "f", returns := [.bool], code := #[.call 2, .ret] },
     { name := "g", code := #[.call 3, .ret] }]
    [{ address := 0x1, module := "a", name := "check", returns := [.bool] },
     { address := 0x1, module := "a", name := "peek" }]

#guard runFun ⟨#[reenterA, reenterB]⟩ 0 1 [] == .returned [.bool false]
#guard runFun ⟨#[reenterA, reenterB]⟩ 0 0 [] == .failed .resourceLocked
#guard runFun ⟨#[reenterA, reenterB]⟩ 0 3 [] == .failed .missingResource
#guard runFun ⟨#[reenterA, reenterB]⟩ 0 2 [] == .failed .resourceLocked

-- R3 before R5 for `MoveFrom` and `MoveTo` (language-owner ruling
-- 2026-11-03): `a::startTake` reaches `a::take` through `b::relayTake`, and
-- `a::startPut` reaches `a::put` through `b::relayPut`. Locked, a `MoveFrom`
-- of a missing resource and a `MoveTo` onto an existing one fail as locked;
-- unlocked (`a::take` directly, `a::putHere` calling `a::put`), as missing
-- and as existing.
def typeResA : StructId := ⟨⟨0x1, "a"⟩, "Res", []⟩

def resAtOne : GlobalStorage := [⟨1, typeResA, .struct typeResA [u64 5]⟩]

def lockedA : CompiledModule :=
  mkModule 0x1 "a"
    [{ name := "startTake", code := #[.call 5, .ret] },
     { name := "take", code := #[.ldConst 0, .moveFrom 0, .unpack 0, .pop, .ret] },
     { name := "startPut", params := [.signer], code := #[.immBorrowLoc 0, .call 6, .ret] },
     { name := "put", params := [.reference .signer],
       code := #[.moveLoc 0, .ldU64 5, .pack 0, .moveTo 0, .ret] },
     { name := "putHere", params := [.signer], code := #[.immBorrowLoc 0, .call 3, .ret] }]
    [{ address := 0x2, module := "b", name := "relayTake" },
     { address := 0x2, module := "b", name := "relayPut", params := [.reference .signer] }] [constOne]
    [{ name := "Res", abilities := { key := true }, fields := [.u64] }]

def lockedB : CompiledModule :=
  mkModule 0x2 "b"
    [{ name := "relayTake", code := #[.call 2, .ret] },
     { name := "relayPut", params := [.reference .signer], code := #[.moveLoc 0, .call 3, .ret] }]
    [{ address := 0x1, module := "a", name := "take" },
     { address := 0x1, module := "a", name := "put", params := [.reference .signer] }]

#guard runFun ⟨#[lockedA, lockedB]⟩ 0 0 [] == .failed .resourceLocked
#guard runFun ⟨#[lockedA, lockedB]⟩ 0 1 [] == .failed .missingResource
#guard runFun ⟨#[lockedA, lockedB]⟩ 0 1 [] (storage := resAtOne) == .returned [] []
#guard runFun ⟨#[lockedA, lockedB]⟩ 0 2 [signerAt 1] (storage := resAtOne) == .failed .resourceLocked
#guard runFun ⟨#[lockedA, lockedB]⟩ 0 4 [signerAt 1] (storage := resAtOne) == .failed .resourceExists
#guard runFun ⟨#[lockedA, lockedB]⟩ 0 4 [signerAt 1] == .returned [] resAtOne

/-! ### Initial states

Initial global storage is well typed (`GlobalStorage.wellTyped`), and the
arguments are values. -/

#guard GlobalStorage.wellTyped (withRefs publish) atOne
#guard GlobalStorage.wellTyped (withRefs publish) []
#guard !GlobalStorage.wellTyped (withRefs publish) (atOne ++ atOne)
#guard !GlobalStorage.wellTyped (withRefs publish) [⟨1, typeRes, valS 1 true⟩]
#guard !GlobalStorage.wellTyped (withRefs publish) [⟨1, typeT, .struct typeT [u64 1]⟩]
#guard !GlobalStorage.wellTyped (withRefs publish) [⟨1, typeK, .struct typeK [.struct typeN []]⟩]
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.reference .u64])) ⟨0, 0, []⟩
  [.ref ⟨false, .int .u64, ⟨.local_ 0 0, []⟩, 0⟩]).isNone
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.u64])) ⟨0, 0, []⟩ [u64 1] atOne).map (·.storage) ==
  some atOne

/-! ### The static typing discipline (C1) of the new instructions -/

def refTypeAfter (instr : Bytecode) (stack : List RuntimeType) : Option (List RuntimeType) :=
  typeAfter (withRefs publish) ⟨0, 0, []⟩ (refModule [publish]) instr stack

#guard refTypeAfter .readRef [.ref false (.int .u64)] == some [.int .u64]
#guard refTypeAfter .readRef [.ref true .signer] == none
#guard refTypeAfter .writeRef [.ref true (.int .u64), .int .u64] == some []
#guard refTypeAfter .writeRef [.ref false (.int .u64), .int .u64] == none
#guard refTypeAfter .freezeRef [.ref true .bool] == some [.ref false .bool]
#guard refTypeAfter (.immBorrowLoc 0) [] == some [.ref false .signer]
#guard refTypeAfter (.mutBorrowField 0) [.ref true (.struct typeS)] == some [.ref true (.int .u64)]
#guard refTypeAfter (.mutBorrowField 0) [.ref false (.struct typeS)] == none
#guard refTypeAfter (.immBorrowField 1) [.ref true (.struct typeS)] == some [.ref false .bool]
#guard refTypeAfter (.immBorrowVariantField 0) [.ref false (.struct typeE)] ==
  some [.ref false (.int .u64)]
#guard refTypeAfter (.immBorrowVariantField 2) [.ref false (.struct typeE)] == none
#guard refTypeAfter (.testVariant 1) [.ref true (.struct typeE)] == some [.bool]
#guard refTypeAfter (.vecMutBorrow 3) [.int .u64, .ref true (.vector (.int .u64))] ==
  some [.ref true (.int .u64)]
#guard refTypeAfter (.vecPushBack 3) [.int .u64, .ref false (.vector (.int .u64))] == none
#guard refTypeAfter (.vecSwap 3) [.int .u64, .int .u64, .ref true (.vector (.int .u64))] == some []
#guard refTypeAfter (.moveTo 4) [.struct typeRes, .ref false .signer] == some []
#guard refTypeAfter (.moveTo 4) [.struct typeRes, .ref true .signer] == none
#guard refTypeAfter (.mutBorrowGlobal 4) [.address] == some [.ref true (.struct typeRes)]
#guard refTypeAfter (.exists_ 6) [.address] == none

/-! ### Natives with references are outside the subset -/

def nativeRefs : Program :=
  ⟨#[mkModule 0x1 "n"
      [{ name := "len", params := [.reference (.vector .u64)], returns := [.u64], native := true },
       { name := "main", params := [.vector .u64], returns := [.u64],
         code := #[.immBorrowLoc 0, .call 0, .ret] }]]⟩

#guard runFun nativeRefs 0 1 [vecU64 [1]] == .wrong (.unsupported "native with references")

/-! ### Generic forms name an instantiation table entry (L1) -/

#guard runRefs (dataFun #[.immBorrowLoc 0, .immBorrowFieldGeneric 0, .ret] [] [] [.struct 0])
  [valS 1 true] == .wrong .badIndex
#guard runRefs (dataFun #[.copyLoc 0, .existsGeneric 0, .ret] [.bool] [] [.address]) [.address 1] ==
  .wrong .badIndex

/-! ## The ghost layer on references and global storage (Phase 3, stage B)

`refViolations` evaluates the D4 checks (`Spec.Properties.References`) on a
run: D4a and the storage part of D4d at every step, and the trace checks
(D4b′, D4b″, D4c, the returns part of D4d, D4e) on all of the run's events,
which covers every prefix: a violation in a prefix stays one in the whole
run. -/

/-- The D4 trace checks that fail on events `evs`. -/
def traceRefViolations (evs : List Event) : List String :=
  (if RefSafety.contained evs then [] else ["D4b′"]) ++
  (if RefSafety.paramsExclusive evs then [] else ["D4b″"]) ++
  (if RefSafety.stable evs then [] else ["D4c"]) ++
  (if RefSafety.returnsDerived evs then [] else ["D4d"]) ++
  (if RefSafety.deadSlotsUnborrowed evs then [] else ["D4e"])

/-- The D4 checks along a run from state `s` with shadow `h` after events
`evs`. -/
def refViolationsFrom (P : Program) (natives : NativeImpl) (ν : NativeFlows) :
    Nat → State → Shadow → List Event → List String
  | 0, _, _, evs => traceRefViolations evs
  | fuel + 1, s, h, evs =>
    let here := if RefSafety.unnested s then [] else ["D4d"]
    let stepChecks (o : Outcome) (e : List Event) : List String :=
      (if RefSafety.usesValid s e then [] else ["D4a"]) ++
      (if RefSafety.unnestedAfter o then [] else ["D4d"])
    match traceStep P natives ν s h with
    | (o@(.running s'), some (.running h', e)) =>
      here ++ stepChecks o e ++ refViolationsFrom P natives ν fuel s' h' (evs ++ e)
    | (o@(.returned _ _), some (_, e)) => here ++ stepChecks o e ++ traceRefViolations (evs ++ e)
    | (.running _, _) => here ++ ["unaccounted"]
    | (.returned _ _, none) => here ++ ["unaccounted"]
    | (.failed _, _) => here ++ traceRefViolations evs
    | (.wrong _, _) => here ++ traceRefViolations evs

/-- The D4 checks that fail on a run of function `index` of module `module`. -/
def refViolations (P : Program) (module index : Nat) (args : List Value)
    (storage : GlobalStorage := []) (held : List Value := []) (typeArgs : List RuntimeType := []) :
    List String :=
  match initialState P (fn module index typeArgs) args storage held with
  | some s =>
    let (h, evs) := Shadow.initial s
    (refViolationsFrom P noNatives allFresh 10000 s h evs).eraseDups
  | none => ["no initial state"]

/-! ### Every step is accounted, and no check fails

Programs of the reference and global storage tests above that return: the
ghost layer changes no outcome and accounts for every step, and no D2, D3 or
D4 check fails. Two of them matter most for D4: field references borrowed
through separate borrows of one local do not conflict (`siblings`), and an
element reference does not conflict with reading the vector's length. -/

def siblings : Fun :=
  dataFun #[.mutBorrowLoc 0, .mutBorrowField 0, .stLoc 1, .mutBorrowLoc 0, .mutBorrowField 1, .stLoc 2,
      .ldU64 4, .moveLoc 1, .writeRef, .moveLoc 2, .readRef, .ret]
    [.bool] [.mutableReference .u64, .mutableReference .bool] [.struct 0]

/-- Borrow element 0 of the vector parameter, run `between`, then write 9
through the element reference and return the vector. -/
def aroundElement (between : Array Bytecode) : Fun :=
  dataFun (#[.mutBorrowLoc 0, .ldU64 0, .vecMutBorrow 3, .stLoc 1] ++ between ++
      #[.ldU64 9, .moveLoc 1, .writeRef, .moveLoc 0, .ret])
    [.vector .u64] [.mutableReference .u64] [.vector .u64]

def refCases : List (Fun × List Value × GlobalStorage) :=
  [(dataFun #[.immBorrowLoc 0, .readRef, .ret] [.u64] [] [.u64], [u64 5], []),
   (dataFun #[.ldU64 7, .mutBorrowLoc 0, .writeRef, .moveLoc 0, .ret] [.u64] [] [.u64], [u64 5], []),
   (dataFun #[.mutBorrowLoc 0, .freezeRef, .readRef, .ret] [.u64] [] [.u64], [u64 5], []),
   (dataFun #[.immBorrowLoc 0, .stLoc 1, .copyLoc 1, .readRef, .moveLoc 1, .readRef, .add, .ret]
      [.u64] [.reference .u64] [.u64], [u64 5], []),
   (dataFun #[.pack 1, .stLoc 0, .immBorrowLoc 0, .pop, .moveLoc 0, .ret] [.struct 1] [.struct 1], [], []),
   (dataFun #[.immBorrowLoc 0, .stLoc 1, .ret] [] [.reference .u64] [.u64], [u64 5], []),
   (dataFun #[.moveLoc 0, .stLoc 0, .immBorrowLoc 0, .readRef, .ret] [.u64] [] [.u64], [u64 5], []),
   (writeParent #[.moveLoc 1, .readRef, .ret] [.struct 0], [valS 1 true], []),
   (dataFun #[.ldU64 9, .mutBorrowLoc 0, .mutBorrowField 0, .writeRef, .moveLoc 0, .ret]
      [.struct 0] [] [.struct 0], [valS 1 true], []),
   (dataFun #[.mutBorrowLoc 0, .stLoc 1, .ldU64 3, .copyLoc 1, .writeRef, .moveLoc 1, .readRef, .ret]
      [.u64] [.mutableReference .u64] [.u64], [u64 5], []),
   (siblings, [valS 1 true], []),
   (dataFun #[.immBorrowLoc 0, .immBorrowField 2, .immBorrowField 1, .readRef, .ret] [.bool] []
      [.struct 2], [valW (valS 1 true)], []),
   (dataFun #[.ldU64 42, .mutBorrowLoc 0, .mutBorrowField 2, .mutBorrowField 0, .writeRef, .moveLoc 0,
      .ret] [.struct 2] [] [.struct 2], [valW (valS 1 true)], []),
   (readVariantField 0, [valE 1 [u64 6]], []),
   (dataFun #[.ldU64 9, .mutBorrowLoc 0, .mutBorrowVariantField 0, .writeRef, .moveLoc 0, .ret]
      [.struct 3] [] [.struct 3], [valE 1 [u64 6]], []),
   (testVariant 0, [valE 0 [u64 5]], []),
   (dataFun #[.immBorrowLoc 0, .vecLen 3, .ret] [.u64] [] [.vector .u64], [vecU64 [1, 2, 3]], []),
   (dataFun #[.ldU64 9, .mutBorrowLoc 0, .ldU64 0, .vecMutBorrow 3, .writeRef, .moveLoc 0, .ret]
      [.vector .u64] [] [.vector .u64], [vecU64 [1, 2, 3]], []),
   (dataFun #[.mutBorrowLoc 0, .ldU64 4, .vecPushBack 3, .moveLoc 0, .ret] [.vector .u64] []
      [.vector .u64], [vecU64 [1, 2, 3]], []),
   (dataFun #[.mutBorrowLoc 0, .vecPopBack 3, .moveLoc 0, .ret] [.u64, .vector .u64] []
      [.vector .u64], [vecU64 [1, 2, 3]], []),
   (dataFun #[.mutBorrowLoc 0, .ldU64 0, .ldU64 2, .vecSwap 3, .moveLoc 0, .ret] [.vector .u64] []
      [.vector .u64], [vecU64 [1, 2, 3]], []),
   (changeVector #[.immBorrowLoc 0, .vecLen 3, .pop, .mutBorrowLoc 0, .ldU64 0, .vecImmBorrow 3,
      .readRef, .pop], [vecU64 [1, 2, 3]], []),
   (aroundElement #[.immBorrowLoc 0, .vecLen 3, .pop], [vecU64 [1, 2, 3]], []),
   (dataFun #[.immBorrowLoc 0, .immBorrowLoc 1, .eq, .ret] [.bool] [] [.u64, .u64], [u64 5, u64 5], []),
   (dataFun #[.pack 1, .stLoc 0, .immBorrowLoc 0, .immBorrowLoc 0, .eq, .moveLoc 0, .unpack 1, .ret]
      [.bool] [.struct 1], [], []),
   (publish, [signerAt 1], []),
   (publish, [signerAt 2], atOne),
   (existsAt, [.address 1], atOne),
   (moveFromAt, [.address 1], atOne),
   (dataFun #[.copyLoc 0, .immBorrowGlobal 4, .immBorrowField 3, .readRef, .ret] [.u64] [] [.address],
      [.address 1], atOne),
   (dataFun #[.ldU64 9, .copyLoc 0, .mutBorrowGlobal 4, .mutBorrowField 3, .writeRef, .ret] [] []
      [.address], [.address 1], atOne)]

#guard refCases.all fun (f, args, st) =>
  runRefs f args st matches .returned _ _
#guard refCases.all fun (f, args, st) => agrees (withRefs f) 0 0 args (storage := st)
#guard refCases.all fun (f, args, st) =>
  violations (withRefs f) 0 0 args (auth := [1, 2]) (A := [typeS, typeW, typeE]) (storage := st) == []
-- The struct arguments are the transaction's, so their types must be accepted
-- (E1).
#guard violations (withRefs siblings) 0 0 [valS 1 true] == ["D3a"]
#guard refCases.all fun (f, args, st) => refViolations (withRefs f) 0 0 args st == []

-- Calls passing and returning references, and a callee writing its caller's
-- local.
#guard [5, 6].all fun i => agrees callProgram 0 i [u64 5]
#guard [5, 6].all fun i => violations callProgram 0 i [u64 5] == []
#guard [5, 6].all fun i => refViolations callProgram 0 i [u64 5] == []

-- Runs that fail or go wrong are accounted up to the step that ends them.
#guard agrees (withRefs publish) 0 0 [signerAt 1] (storage := atOne)
#guard agrees (withRefs moveFromAt) 0 0 [.address 2] (storage := atOne)
#guard agrees (withRefs (writeParent #[.moveLoc 2, .readRef, .ret] [.u64])) 0 0 [valS 1 true]
#guard agrees (withRefs (readVariantField 0)) 0 0 [valE 2 []]
#guard agrees ⟨#[reenterA, reenterB]⟩ 0 0 []
#guard [7, 8, 15, 16].all fun i => agrees callProgram 0 i []
#guard [9, 10].all fun i => agrees callProgram 0 i [u64 5]

/-! ### What the ghost layer records -/

def refU64 (l : Location) (mutable : Bool := false) (since : Nat := 0) : Reference :=
  ⟨mutable, .int .u64, l, since⟩

-- A borrow creates a reference, a read accesses its location and uses it, and
-- the value read keeps the location's identities (reading copies).
#guard (traceFun (withRefs (dataFun #[.immBorrowLoc 0, .readRef, .ret] [.u64] [] [.u64])) 0 0
    [u64 5]).events ==
  [.created 0 (u64 5) .transaction, .entered 0 [],
   .borrowed 1 (refU64 ⟨.local_ 0 0, []⟩) none,
   .accessed .read ⟨.local_ 0 0, []⟩ (some 1), .used 1 (refU64 ⟨.local_ 0 0, []⟩),
   .frameEnded 0 []]
#guard returnedIds (traceFun (withRefs (dataFun #[.immBorrowLoc 0, .readRef, .ret] [.u64] [] [.u64]))
  0 0 [u64 5]) == some [0]

-- A field borrow derives from the reference it consumes, and records the
-- struct type and the module (D3a).
#guard ((traceFun (withRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 0, .readRef, .ret] [.u64] []
    [.struct 0])) 0 0 [valS 1 true]).events.filterMap Event.borrowed?).map (fun (i, _, p) => (i, p)) ==
  [(3, none), (4, some 3)]
#guard (traceFun (withRefs (dataFun #[.immBorrowLoc 0, .immBorrowField 0, .readRef, .ret] [.u64] []
    [.struct 0])) 0 0 [valS 1 true]).events.contains
  (.privileged dataModule typeS (.immBorrowField 0))

-- Global storage values are occurrences, created before the transaction;
-- `MoveFrom` moves one onto the stack and `Unpack` consumes it in its module.
#guard (traceFun (withRefs moveFromAt) 0 0 [.address 1] (storage := atOne)).ownershipEvents ==
  [.created 0 (.address 1) .transaction, .created 1 (valRes 5) .storage, .created 2 (u64 5) .storage,
   .consumed 1 (valRes 5) (.unpack dataModule)]
#guard (traceFun (withRefs moveFromAt) 0 0 [.address 1] (storage := atOne)).events.contains
  (.privileged dataModule typeRes (.moveFrom 4))
-- `MoveTo` moves the packed value into global storage, where D2c finds it.
#guard match traceFun (withRefs publish) 0 0 [signerAt 1] with
  | .returned [] st _ evs => st == atOne && evs.contains (.privileged dataModule typeRes (.moveTo 4))
  | _ => false

/-! ### D4 rejects what D1 lets through

Each program below runs to its end in the semantics (D1 checks validity only
where a location is accessed), but breaks a D4 property. -/

-- D4a: copying and storing a reference into a frame that returned
-- (`keepDangle`); the returned reference did not derive from a parameter
-- (D4d), and the frame ended while it was live (D4e).
#guard runFun callProgram 0 11 [] == .returned []
#guard refViolations callProgram 0 11 [] == ["D4a", "D4d", "D4e"]
-- D4c: overwriting a local while a `&` reference to it is live.
def overwriteBorrowed : Fun :=
  dataFun #[.immBorrowLoc 0, .stLoc 1, .ldU64 9, .stLoc 0, .moveLoc 1, .readRef, .ret]
    [.u64] [.reference .u64] [.u64]

#guard runRefs overwriteBorrowed [u64 5] == .returned [u64 9]
#guard refViolations (withRefs overwriteBorrowed) 0 0 [u64 5] == ["D4c"]
-- D4c: writing through a `&mut` reference while a frozen reference to the
-- same location is live.
#guard runRefs (dataFun #[.mutBorrowLoc 0, .stLoc 1, .copyLoc 1, .freezeRef, .stLoc 2, .ldU64 9,
    .copyLoc 1, .writeRef, .moveLoc 2, .readRef, .ret] [.u64] [.mutableReference .u64, .reference .u64]
    [.u64]) [u64 5] == .returned [u64 9]
#guard refViolations (withRefs (dataFun #[.mutBorrowLoc 0, .stLoc 1, .copyLoc 1, .freezeRef, .stLoc 2,
    .ldU64 9, .copyLoc 1, .writeRef, .moveLoc 2, .readRef, .ret] [.u64]
    [.mutableReference .u64, .reference .u64] [.u64])) 0 0 [u64 5] == ["D4c"]
-- D4b′: reading the whole vector while an element is mutably borrowed, where
-- reading its length is not a conflict (`aroundElement`, above).
#guard runRefs (aroundElement #[.immBorrowLoc 0, .readRef, .pop]) [vecU64 [1, 2, 3]] ==
  .returned [vecU64 [9, 2, 3]]
#guard refViolations (withRefs (aroundElement #[.immBorrowLoc 0, .readRef, .pop])) 0 0
  [vecU64 [1, 2, 3]] == ["D4b′"]
-- D4b′: writing the struct through a reference, its parent's copy, while a
-- `&mut` reference to a field is still to be used. The write invalidates the
-- field reference, whose next use, a move, is dangling (D4a).
#guard runRefs (writeParent #[.moveLoc 2, .pop, .ret] []) [valS 1 true] == .returned []
#guard refViolations (withRefs (writeParent #[.moveLoc 2, .pop, .ret] [])) 0 0 [valS 1 true] ==
  ["D4a", "D4b′"]
-- D4d: returning a reference to the function's own local, or to global
-- storage.
#guard runFun callProgram 0 2 [] matches .returned [.ref _] _
#guard refViolations callProgram 0 2 [] == ["D4d"]
#guard refViolations (withRefs (dataFun #[.copyLoc 0, .immBorrowGlobal 4, .ret] [.reference (.struct 4)]
  [] [.address])) 0 0 [.address 1] atOne == ["D4d"]
-- D4e: moving a local out while a reference to it is live; using the
-- reference afterwards is dangling (D4a), and the move is a write while the
-- `&` reference is live (D4c).
def moveBorrowed : Fun :=
  dataFun #[.immBorrowLoc 0, .stLoc 1, .moveLoc 0, .pop, .moveLoc 1, .pop, .ret] [] [.reference .u64]
    [.u64]

#guard runRefs moveBorrowed [u64 5] == .returned []
#guard refViolations (withRefs moveBorrowed) 0 0 [u64 5] == ["D4a", "D4c", "D4e"]

/-! ### Aliasing `&mut` references (D4b′, D4b″)

Within a frame, references may alias at or inside a live `&mut` reference's
location: neither invalidates the other, and the frame's code shows the
aliasing. Reaching over it, through a reference from strictly above or
directly through the local, is a D4b′ violation. Across a call, a frame's
`&mut` reference parameters are exclusive while it is active (D4b″). -/

-- Two `&mut` borrows of one local, written in turn while both are live, then
-- one read: allowed. Full mutable exclusivity, which D4b′ and D4b″ replace,
-- rejected this (spec-owner decision, 2026-10-05).
def twoMutable : Fun :=
  dataFun #[.mutBorrowLoc 0, .stLoc 1, .mutBorrowLoc 0, .stLoc 2, .ldU64 1, .copyLoc 1, .writeRef,
      .ldU64 2, .copyLoc 2, .writeRef, .moveLoc 1, .readRef, .ret]
    [.u64] [.mutableReference .u64, .mutableReference .u64] [.u64]

#guard runRefs twoMutable [u64 5] == .returned [u64 2]
#guard refViolations (withRefs twoMutable) 0 0 [u64 5] == []
-- Reading the local through an `&` reference while a `&mut` reference to it is
-- live is allowed; reading it directly, `CopyLoc`, is not.
def readAround (read : Array Bytecode) : Fun :=
  dataFun (#[.mutBorrowLoc 0, .stLoc 1] ++ read ++ #[.ldU64 9, .moveLoc 1, .writeRef, .ret])
    [.u64] [.mutableReference .u64] [.u64]

#guard runRefs (readAround #[.immBorrowLoc 0, .readRef]) [u64 5] == .returned [u64 5]
#guard refViolations (withRefs (readAround #[.immBorrowLoc 0, .readRef])) 0 0 [u64 5] == []
#guard runRefs (readAround #[.copyLoc 0]) [u64 5] == .returned [u64 5]
#guard refViolations (withRefs (readAround #[.copyLoc 0])) 0 0 [u64 5] == ["D4b′"]

-- `alias` receives two `&mut` references, writes through the first and reads
-- through the second. `useAlias` passes it two borrows of one local, which
-- `alias` cannot see; `useDistinct` borrows two locals.
def aliasFuns : List Fun :=
  [{ name := "alias", params := [.mutableReference .u64, .mutableReference .u64], returns := [.u64],
     code := #[.ldU64 1, .moveLoc 0, .writeRef, .moveLoc 1, .readRef, .ret] },
   { name := "useAlias", params := [.u64], returns := [.u64],
     code := #[.mutBorrowLoc 0, .mutBorrowLoc 0, .call 0, .ret] },
   { name := "useDistinct", params := [.u64, .u64], returns := [.u64],
     code := #[.mutBorrowLoc 0, .mutBorrowLoc 1, .call 0, .ret] }]

def aliasProgram : Program := ⟨#[refModule aliasFuns]⟩

#guard runFun aliasProgram 0 1 [u64 5] == .returned [u64 1]
#guard refViolations aliasProgram 0 1 [u64 5] == ["D4b″"]
#guard runFun aliasProgram 0 2 [u64 5, u64 6] == .returned [u64 6]
#guard refViolations aliasProgram 0 2 [u64 5, u64 6] == []

/-! ### Entry functions with `&signer` (E1, E2)

The transaction holds the signers of `&signer` parameters and lends them by
reference (`Root.held`). -/

def signerRef (k : Nat) : Value := .ref ⟨false, .signer, ⟨.held k, []⟩, 0⟩

def publishRef : Fun :=
  dataFun #[.moveLoc 0, .ldU64 5, .pack 4, .moveTo 4, .ret] [] [] [.reference .signer]

#guard runFun (withRefs publishRef) 0 0 [signerRef 0] (held := [signerAt 1]) == .returned [] atOne
#guard agrees (withRefs publishRef) 0 0 [signerRef 0] (held := [signerAt 1])
#guard violations (withRefs publishRef) 0 0 [signerRef 0] (auth := [1]) (held := [signerAt 1]) == []
#guard refViolations (withRefs publishRef) 0 0 [signerRef 0] (held := [signerAt 1]) == []
-- A held signer for an address nobody authenticated (D3b).
#guard violations (withRefs publishRef) 0 0 [signerRef 0] (auth := [2]) (held := [signerAt 1]) ==
  ["D3b"]
-- Returning the `&signer` parameter: it derives from a reference the
-- transaction lent.
#guard refViolations (withRefs (dataFun #[.moveLoc 0, .ret] [.reference .signer] [] [.reference .signer]))
  0 0 [signerRef 0] (held := [signerAt 1]) == []
-- The leading signer parameters of a transaction.
#guard signerArgs [(1, true), (2, false), (3, true)] 0 == [signerRef 0, signerAt 2, signerRef 1]
#guard heldSigners [(1, true), (2, false), (3, true)] == [signerAt 1, signerAt 3]
-- A reference argument must designate a whole held value of its type, taken
-- before anything was invalidated, and no other argument may designate it.
def twoSignerRefs : Fun :=
  dataFun #[.ret] [] [] [.reference .signer, .reference .signer]

#guard (initialState (withRefs twoSignerRefs) ⟨0, 0, []⟩ [signerRef 0, signerRef 1] [] [signerAt 1, signerAt 2]).isSome
#guard (initialState (withRefs twoSignerRefs) ⟨0, 0, []⟩ [signerRef 0, signerRef 0] [] [signerAt 1]).isNone
#guard (initialState (withRefs twoSignerRefs) ⟨0, 0, []⟩ [signerRef 0, signerRef 1] [] [signerAt 1]).isNone
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.reference .u64])) ⟨0, 0, []⟩
  [.ref (refU64 ⟨.held 0, []⟩)] [] [signerAt 1]).isNone
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.reference .u64])) ⟨0, 0, []⟩
  [.ref (refU64 ⟨.held 0, [.field 0]⟩)] [] [valS 1 true]).isNone
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.reference .signer])) ⟨0, 0, []⟩
  [.ref ⟨false, .signer, ⟨.held 0, []⟩, 1⟩] [] [signerAt 1]).isNone
-- Every held value is well formed, of a value type, whether or not an
-- argument designates it: a vector of `u64` holding a reference, or an
-- out-of-range integer, is not one.
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.reference .signer])) ⟨0, 0, []⟩
  [signerRef 0] [] [signerAt 1, .vector (.int .u64) [u64 7]]).isSome
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.reference .signer])) ⟨0, 0, []⟩
  [signerRef 0] [] [signerAt 1, .vector (.int .u64) [.ref (refU64 ⟨.held 0, []⟩)]]).isNone
#guard (initialState (withRefs (dataFun #[.ret] [] [] [.reference .signer])) ⟨0, 0, []⟩
  [signerRef 0] [] [signerAt 1, u8v 256]).isNone

/-! ## Generics (Phase 4, stage A)

Module `0x42::g` declares six types:

- struct 0, `Box<T> has copy, drop, store { T }`;
- struct 1, `Coin<phantom C> has store, key { u64 }`;
- struct 2, `enum Opt<T> has copy, drop { None, Some(T) }`;
- struct 3, `R {}`, with no abilities;
- struct 4, `Cup<T: copy> has copy, drop { T }`;
- struct 5, `Holder<T> has key { T }`,

and the 23 functions of `genFuns` (handle `i` is function `i`), then one test
function, number 23. `gs k` is extra signature `k`: `[u64]`, `[bool]`, `[T]`,
`[R]`, `[Box<u64>]`, `[]`, `[u64, bool]`, `[&u64]`, `[Box<T>]`. Variant
handles: 0 `Opt::None`, 1 `Opt::Some`. Field handles: 0 `Box.0`, 1
`Holder.0`. Variant field handle 0 is `Opt::Some.0`. The instantiation tables
are listed in `genModule`. -/

def genCount : Nat := 24

def gs (k : Nat) : SignatureIndex := 3 * genCount + k

def tp : SignatureToken := .typeParameter 0
def boxOf (t : SignatureToken) : SignatureToken := .structInstantiation 0 [t]
def optOf (t : SignatureToken) : SignatureToken := .structInstantiation 2 [t]

def genStructs : List StructDecl :=
  [{ name := "Box", abilities := ⟨true, true, true, false⟩, typeParameters := [⟨{}, false⟩],
     fields := [tp] },
   { name := "Coin", abilities := { store := true, key := true }, typeParameters := [⟨{}, true⟩],
     fields := [.u64] },
   { name := "Opt", abilities := ⟨true, true, false, false⟩, typeParameters := [⟨{}, false⟩],
     variants := some [("None", []), ("Some", [tp])] },
   { name := "R" },
   { name := "Cup", abilities := ⟨true, true, false, false⟩,
     typeParameters := [⟨{ copy := true }, false⟩], fields := [tp] },
   { name := "Holder", abilities := { key := true }, typeParameters := [⟨{}, false⟩],
     fields := [tp] }]

def genFuns : List Fun :=
  [{ name := "id", typeParameters := [{}], params := [tp], returns := [tp],
     code := #[.moveLoc 0, .ret] },
   { name := "useId", returns := [.u64, .bool],
     code := #[.ldU64 5, .callGeneric 0, .ldTrue, .callGeneric 1, .ret] },
   { name := "dup", typeParameters := [{ copy := true }], params := [tp], returns := [tp, tp],
     code := #[.copyLoc 0, .moveLoc 0, .ret] },
   { name := "dupR", returns := [.struct 3, .struct 3], code := #[.pack 3, .callGeneric 2, .ret] },
   { name := "mkBox", typeParameters := [{}], params := [tp], returns := [boxOf tp],
     code := #[.moveLoc 0, .packGeneric 0, .ret] },
   { name := "unbox", typeParameters := [{}], params := [boxOf tp], returns := [tp],
     code := #[.moveLoc 0, .unpackGeneric 0, .ret] },
   { name := "boxes", returns := [.u64], code := #[.ldU64 7, .callGeneric 3, .callGeneric 4, .ret] },
   { name := "some", typeParameters := [{}], params := [tp], returns := [optOf tp],
     code := #[.moveLoc 0, .packVariantGeneric 0, .ret] },
   { name := "get", typeParameters := [{}], params := [optOf tp], returns := [tp],
     code := #[.moveLoc 0, .unpackVariantGeneric 0, .ret] },
   { name := "isSome", typeParameters := [{}], params := [.reference (optOf tp)], returns := [.bool],
     code := #[.moveLoc 0, .testVariantGeneric 0, .ret] },
   { name := "boxRef", typeParameters := [{}], params := [.reference (boxOf tp)],
     returns := [.reference tp], code := #[.moveLoc 0, .immBorrowFieldGeneric 0, .ret] },
   { name := "setBox", typeParameters := [{ drop := true }],
     params := [.mutableReference (boxOf tp), tp],
     code := #[.moveLoc 1, .moveLoc 0, .mutBorrowFieldGeneric 0, .writeRef, .ret] },
   { name := "optRef", typeParameters := [{}], params := [.reference (optOf tp)],
     returns := [.reference tp], code := #[.moveLoc 0, .immBorrowVariantFieldGeneric 0, .ret] },
   { name := "setOpt", typeParameters := [{ drop := true }],
     params := [.mutableReference (optOf tp), tp],
     code := #[.moveLoc 1, .moveLoc 0, .mutBorrowVariantFieldGeneric 0, .writeRef, .ret] },
   { name := "publish", typeParameters := [{ store := true }], params := [.reference .signer, tp],
     code := #[.moveLoc 0, .moveLoc 1, .packGeneric 3, .moveToGeneric 3, .ret] },
   { name := "has", typeParameters := [{}], params := [.address], returns := [.bool],
     code := #[.moveLoc 0, .existsGeneric 3, .ret] },
   { name := "take", typeParameters := [{}], params := [.address], returns := [tp],
     code := #[.moveLoc 0, .moveFromGeneric 3, .unpackGeneric 3, .ret] },
   { name := "peek", typeParameters := [{ copy := true }], params := [.address], returns := [tp],
     code := #[.moveLoc 0, .immBorrowGlobalGeneric 3, .immBorrowFieldGeneric 1, .readRef, .ret] },
   { name := "poke", typeParameters := [{ drop := true }], params := [.address, tp],
     code := #[.moveLoc 1, .moveLoc 0, .mutBorrowGlobalGeneric 3, .mutBorrowFieldGeneric 1,
       .writeRef, .ret] },
   { name := "wrap", typeParameters := [{}], params := [tp], returns := [boxOf tp],
     code := #[.moveLoc 0, .callGeneric 9, .ret] },
   { name := "keep", typeParameters := [{}], params := [tp], returns := [.vector tp], locals := [tp],
     code := #[.moveLoc 0, .stLoc 1, .moveLoc 1, .vecPack (gs 2) 1, .ret] },
   { name := "grow", typeParameters := [{}], code := #[.callGeneric 15, .ret] },
   { name := "pubBoth", params := [.reference .signer],
     code := #[.copyLoc 0, .ldU64 1, .callGeneric 10, .moveLoc 0, .ldTrue, .callGeneric 11, .ret] }]

#guard genFuns.length + 1 == genCount

/-- The generic module, with test function `test` last. -/
def genModule (test : Fun) : CompiledModule :=
  mkModule 0x42 "g" (genFuns ++ [test]) [] [] genStructs [] [⟨2, 0⟩, ⟨2, 1⟩]
    [[.u64], [.bool], [tp], [.struct 3], [boxOf .u64], [], [.u64, .bool], [.reference .u64],
      [boxOf tp]]
    [⟨0, 0⟩, ⟨5, 0⟩] [⟨2, [1], 0⟩]
    (functionInstantiations :=
      [⟨0, gs 0⟩, ⟨0, gs 1⟩, ⟨2, gs 3⟩, ⟨4, gs 0⟩, ⟨5, gs 0⟩, ⟨0, gs 6⟩, ⟨1, gs 0⟩, ⟨0, gs 5⟩,
       ⟨0, gs 7⟩, ⟨4, gs 2⟩, ⟨14, gs 0⟩, ⟨14, gs 1⟩, ⟨2, gs 0⟩, ⟨19, gs 4⟩, ⟨0, 999⟩,
       ⟨21, gs 8⟩, ⟨9, gs 0⟩, ⟨10, gs 0⟩, ⟨11, gs 0⟩, ⟨12, gs 0⟩, ⟨13, gs 0⟩, ⟨15, gs 0⟩,
       ⟨16, gs 0⟩, ⟨17, gs 0⟩, ⟨18, gs 0⟩, ⟨15, gs 1⟩])
    (structDefInstantiations :=
      [⟨0, gs 2⟩, ⟨0, gs 0⟩, ⟨0, gs 1⟩, ⟨5, gs 2⟩, ⟨4, gs 3⟩, ⟨0, gs 6⟩, ⟨3, gs 0⟩, ⟨1, gs 3⟩,
       ⟨5, gs 3⟩, ⟨2, gs 0⟩, ⟨0, gs 5⟩, ⟨5, gs 0⟩, ⟨4, gs 0⟩])
    (fieldInstantiations := [⟨0, gs 2⟩, ⟨1, gs 2⟩, ⟨0, gs 0⟩])
    (structVariantInstantiations := [⟨1, gs 2⟩, ⟨0, gs 2⟩, ⟨1, gs 0⟩, ⟨0, gs 0⟩])
    (variantFieldInstantiations := [⟨0, gs 2⟩, ⟨0, gs 0⟩])

def genTest : Fun := { name := "test", code := #[.ret] }

def genP (test : Fun := genTest) : Program := ⟨#[genModule test]⟩

def gMod : ModuleId := ⟨0x42, "g"⟩
def gType (name : String) (typeArgs : List RuntimeType := []) : StructId := ⟨gMod, name, typeArgs⟩
def tU64 : RuntimeType := .int .u64
def tR : RuntimeType := .struct (gType "R")
def tBox (t : RuntimeType) : RuntimeType := .struct (gType "Box" [t])
def tOpt (t : RuntimeType) : RuntimeType := .struct (gType "Opt" [t])
def boxV (t : RuntimeType) (v : Value) : Value := .struct (gType "Box" [t]) [v]
def holderV (t : RuntimeType) (v : Value) : Value := .struct (gType "Holder" [t]) [v]
def someV (t : RuntimeType) (v : Value) : Value := .variant (gType "Opt" [t]) 1 [v]
def noneV (t : RuntimeType) : Value := .variant (gType "Opt" [t]) 0 []
def valRg : Value := .struct (gType "R") []

/-- Run function `index` of the generic module at type arguments `typeArgs`. -/
def runGen (index : Nat) (args : List Value) (typeArgs : List RuntimeType := [])
    (storage : GlobalStorage := []) (held : List Value := []) (fuel : Nat := 10000) : RunResult :=
  runFun (genP) 0 index args fuel storage held typeArgs

/-- A non-generic test function of the generic module. -/
def genFun (code : Array Bytecode) (returns : Signature := []) (params : Signature := [])
    (locals : Signature := []) : Fun :=
  { name := "test", params, returns, locals, code }

/-- Run a test function of the generic module. -/
def runTest (f : Fun) (args : List Value := []) (storage : GlobalStorage := [])
    (held : List Value := []) : RunResult :=
  runFun (genP f) 0 23 args (storage := storage) (held := held)

/-! ### One generic function, two instantiations -/

-- `useId` calls `id<u64>` and `id<bool>`.
#guard runGen 1 [] == .returned [u64 5, .bool true]
-- A transaction calls an instance: its type arguments are checked like a
-- call site's (one per type parameter, satisfying the constraints, types of
-- the program, no references).
#guard runGen 0 [u64 3] (typeArgs := [tU64]) == .returned [u64 3]
#guard runGen 0 [.bool true] (typeArgs := [.bool]) == .returned [.bool true]
#guard runGen 0 [.bool true] (typeArgs := [tU64]) == .wrong .typeMismatch
#guard runGen 0 [u64 3] == .wrong (.unsupported "no initial state")
#guard runGen 0 [u64 3] (typeArgs := [tU64, tU64]) == .wrong (.unsupported "no initial state")
#guard runGen 2 [valRg] (typeArgs := [tR]) == .wrong (.unsupported "no initial state")
#guard runGen 0 [u64 3] (typeArgs := [.ref false tU64]) == .wrong (.unsupported "no initial state")
#guard runGen 0 [u64 3] (typeArgs := [.struct (gType "Cup" [tR])]) ==
  .wrong (.unsupported "no initial state")
#guard runGen 1 [] (typeArgs := [tU64]) == .wrong (.unsupported "no initial state")
-- `dup<T: copy>`: at `u64`, and called at `R`, which has no `copy`.
#guard runGen 2 [u64 4] (typeArgs := [tU64]) == .returned [u64 4, u64 4]
#guard runGen 3 [] == .wrong .constraintViolation

/-! ### Structs at an instance -/

#guard runGen 6 [] == .returned [u64 7]
#guard runGen 4 [.bool true] (typeArgs := [.bool]) == .returned [boxV .bool (.bool true)]
#guard runGen 5 [boxV .bool (.bool true)] (typeArgs := [.bool]) == .returned [.bool true]
#guard runGen 5 [boxV .bool (.bool true)] (typeArgs := [tU64]) == .wrong .typeMismatch
-- `wrap<T>` calls `mkBox<T>`: the instantiation names the caller's type
-- parameter, translated under the caller's type arguments.
#guard runGen 19 [u64 7] (typeArgs := [tU64]) == .returned [boxV tU64 (u64 7)]
#guard runGen 19 [boxV tU64 (u64 7)] (typeArgs := [tBox tU64]) ==
  .returned [boxV (tBox tU64) (boxV tU64 (u64 7))]
-- A local of type `T` and a vector of `T`, at `bool`.
#guard runGen 20 [.bool true] (typeArgs := [.bool]) == .returned [.vector .bool [.bool true]]
-- `UnpackGeneric` of `Box<u64>` given a `Box<bool>`.
#guard runTest (genFun #[.ldTrue, .packGeneric 2, .unpackGeneric 1, .ret] [.u64]) == .wrong .typeMismatch
-- `Cup<T: copy>` at `R`, which has no `copy`; at `u64`.
#guard runTest (genFun #[.pack 3, .packGeneric 4, .pop, .ret]) == .wrong .constraintViolation
#guard runTest (genFun #[.ldU64 1, .packGeneric 12, .pop, .ret]) == .returned []
-- Generic forms name generic entities with one type argument per type
-- parameter (L9); non-generic forms name non-generic ones.
#guard runTest (genFun #[.ldU64 1, .ldTrue, .packGeneric 5, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.packGeneric 6, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.packGeneric 10, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.ldU64 1, .pack 0, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.ldU64 1, .packGeneric 9, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.packGeneric 99, .ret]) == .wrong .badIndex
-- The same for calls.
#guard runTest (genFun #[.ldU64 1, .callGeneric 5, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.callGeneric 6, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.ldU64 1, .call 0, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.callGeneric 7, .ret]) == .wrong .badOperand
#guard runTest (genFun #[.callGeneric 14, .ret]) == .wrong .badIndex
#guard runTest (genFun #[.callGeneric 99, .ret]) == .wrong .badIndex
-- A reference as a type argument is not a value type.
#guard runTest (genFun #[.callGeneric 8, .ret]) == .wrong (.unsupported "type arguments")

/-! ### Enums at an instance (R1) -/

#guard runGen 7 [u64 5] (typeArgs := [tU64]) == .returned [someV tU64 (u64 5)]
#guard runGen 8 [someV tU64 (u64 5)] (typeArgs := [tU64]) == .returned [u64 5]
#guard runGen 8 [noneV tU64] (typeArgs := [tU64]) == .failed .variantMismatch
#guard runGen 8 [someV .bool (.bool true)] (typeArgs := [tU64]) == .wrong .typeMismatch
#guard runTest (genFun #[.immBorrowLoc 0, .testVariantGeneric 2, .ret] [.bool] [optOf .u64])
  [someV tU64 (u64 5)] == .returned [.bool true]
#guard runTest (genFun #[.immBorrowLoc 0, .testVariantGeneric 3, .ret] [.bool] [optOf .u64])
  [someV tU64 (u64 5)] == .returned [.bool false]
#guard runTest (genFun #[.immBorrowLoc 0, .immBorrowVariantFieldGeneric 1, .readRef, .ret] [.u64]
  [optOf .u64]) [someV tU64 (u64 5)] == .returned [u64 5]
#guard runTest (genFun #[.immBorrowLoc 0, .immBorrowVariantFieldGeneric 1, .readRef, .ret] [.u64]
  [optOf .u64]) [noneV tU64] == .failed .variantMismatch
#guard runTest (genFun #[.ldU64 9, .mutBorrowLoc 0, .mutBorrowVariantFieldGeneric 1, .writeRef,
  .moveLoc 0, .ret] [optOf .u64] [optOf .u64]) [someV tU64 (u64 5)] == .returned [someV tU64 (u64 9)]

/-! ### References into generic values, through generic functions -/

-- `isSome<u64>`, `boxRef<u64>`, `setBox<u64>`, `optRef<u64>`, `setOpt<u64>`.
def refThroughGeneric : List (Fun × List Value × RunResult) :=
  [(genFun #[.immBorrowLoc 0, .callGeneric 16, .ret] [.bool] [optOf .u64], [someV tU64 (u64 5)],
     .returned [.bool true]),
   (genFun #[.immBorrowLoc 0, .callGeneric 17, .readRef, .ret] [.u64] [boxOf .u64],
     [boxV tU64 (u64 7)], .returned [u64 7]),
   (genFun #[.mutBorrowLoc 0, .ldU64 9, .callGeneric 18, .moveLoc 0, .ret] [boxOf .u64] [boxOf .u64],
     [boxV tU64 (u64 7)], .returned [boxV tU64 (u64 9)]),
   (genFun #[.immBorrowLoc 0, .callGeneric 19, .readRef, .ret] [.u64] [optOf .u64],
     [someV tU64 (u64 5)], .returned [u64 5]),
   (genFun #[.mutBorrowLoc 0, .ldU64 9, .callGeneric 20, .moveLoc 0, .ret] [optOf .u64] [optOf .u64],
     [someV tU64 (u64 5)], .returned [someV tU64 (u64 9)])]

#guard refThroughGeneric.all fun (f, args, r) => runTest f args == r

/-! ### Global storage at two instances of one generic type (R3, R5) -/

def holderAt (t : RuntimeType) (v : Value) : Resource := ⟨1, gType "Holder" [t], holderV t v⟩

-- `publish<u64>` and `publish<bool>` under one address: two resources.
#guard runGen 22 [signerRef 0] (held := [signerAt 1]) ==
  .returned [] [holderAt tU64 (u64 1), holderAt .bool (.bool true)]
-- `Holder<u64>` exists, `Holder<bool>` does not.
#guard runGen 15 [.address 1] (typeArgs := [tU64]) (storage := [holderAt tU64 (u64 1)]) ==
  .returned [.bool true] [holderAt tU64 (u64 1)]
#guard runGen 15 [.address 1] (typeArgs := [.bool]) (storage := [holderAt tU64 (u64 1)]) ==
  .returned [.bool false] [holderAt tU64 (u64 1)]
-- `take<bool>` moves out the `bool` one only.
#guard runGen 16 [.address 1] (typeArgs := [.bool])
    (storage := [holderAt tU64 (u64 1), holderAt .bool (.bool true)]) ==
  .returned [.bool true] [holderAt tU64 (u64 1)]
#guard runGen 16 [.address 1] (typeArgs := [.bool]) (storage := [holderAt tU64 (u64 1)]) ==
  .failed .missingResource
#guard runGen 17 [.address 1] (typeArgs := [tU64]) (storage := [holderAt tU64 (u64 1)]) ==
  .returned [u64 1] [holderAt tU64 (u64 1)]
#guard runGen 18 [.address 1, u64 9] (typeArgs := [tU64]) (storage := [holderAt tU64 (u64 1)]) ==
  .returned [] [holderAt tU64 (u64 9)]
-- Publishing `Holder<u64>` twice under one address.
#guard runTest (genFun #[.copyLoc 0, .ldU64 1, .callGeneric 10, .moveLoc 0, .ldU64 2, .callGeneric 10,
  .ret] [] [.reference .signer]) [signerRef 0] (held := [signerAt 1]) == .failed .resourceExists
-- `Holder<R>` has no `key`: `R` has no `store`.
#guard runTest (genFun #[.moveLoc 0, .pack 3, .packGeneric 8, .moveToGeneric 8, .ret] []
  [.reference .signer]) [signerRef 0] (held := [signerAt 1]) == .wrong .globalWithoutKey
#guard runTest (genFun #[.ldU64 1, .existsGeneric 8, .ret] [.bool]) == .wrong .globalWithoutKey

/-! ### Phantom type parameters -/

-- "The arguments to phantom parameters are excluded when deriving the struct
-- abilities": `Coin<R>` has `store` and `key`, `Box<R>` and `Holder<R>`
-- nothing.
#guard (RuntimeType.struct (gType "Coin" [tR])).abilities genP == { store := true, key := true }
#guard (RuntimeType.struct (gType "Coin" [tBox tR])).abilities genP == { store := true, key := true }
#guard (tBox tR).abilities genP == {}
#guard (tBox tU64).abilities genP == ⟨true, true, true, false⟩
#guard (RuntimeType.struct (gType "Holder" [tR])).abilities genP == {}
#guard (RuntimeType.struct (gType "Holder" [tU64])).abilities genP == { key := true }
#guard (tOpt (.vector .signer)).abilities genP == { drop := true }
-- So `Coin<R>` goes to global storage, though `R` cannot.
#guard runTest (genFun #[.moveLoc 0, .ldU64 3, .packGeneric 7, .moveToGeneric 7, .ret] []
  [.reference .signer]) [signerRef 0] (held := [signerAt 1]) ==
  .returned [] [⟨1, gType "Coin" [tR], .struct (gType "Coin" [tR]) [u64 3]⟩]
-- Constraints apply to phantom arguments too, and to type arguments all the
-- way down; an instance has one type argument per type parameter.
#guard (RuntimeType.struct (gType "Cup" [tR])).wellFormed genP == false
#guard (RuntimeType.struct (gType "Cup" [tU64])).wellFormed genP == true
#guard (RuntimeType.struct (gType "Coin" [.struct (gType "Cup" [tR])])).wellFormed genP == false
#guard (RuntimeType.struct (gType "Box" [tU64, tU64])).wellFormed genP == false
#guard (RuntimeType.struct (gType "Box")).wellFormed genP == false
#guard (RuntimeType.struct (gType "Box" [.ref false tU64])).wellFormed genP == false
#guard (RuntimeType.struct (gType "R")).wellFormed genP == true
-- A struct type with a reference among its type arguments does not resolve:
-- no value has it, so no value of a value type holds a reference. A `Box<&u64>`
-- holding a reference is not well formed, and its type has no ability.
#guard (genP.structType? (gType "Box" [.ref false tU64])).isNone
#guard (genP.structType? (gType "Box" [.vector (.ref true tU64)])).isNone
#guard (genP.structType? (gType "Box" [tU64])).isSome
#guard !(Value.struct (gType "Box" [.ref false tU64]) [.ref (refU64 ⟨.held 0, []⟩)]).wellFormed genP
#guard (RuntimeType.struct (gType "Box" [.ref false tU64])).abilities genP == {}
#guard (genP.fieldTypes (gType "Box" [.ref false tU64])).isNone

/-! ### Generic natives

`0x1::nat` declares the native `empty<T>(): vector<T>`, and `main` calls it at
`u64` and at `bool`. The native receives its type arguments with its function
(`FunctionRef.typeArgs`). -/

def natModule : CompiledModule :=
  mkModule 0x1 "nat"
    [{ name := "empty", typeParameters := [{}], returns := [.vector tp], native := true },
     { name := "main", returns := [.vector .u64, .vector .bool],
       code := #[.callGeneric 0, .callGeneric 1, .ret] }]
    (extraSignatures := [[.u64], [.bool]]) (functionInstantiations := [⟨0, 6⟩, ⟨0, 7⟩])

def emptyNative : NativeImpl := fun f _ =>
  match f.typeArgs with
  | [t] => .returned [.vector t []] [] []
  | _ => .failed (.abort 1)

def u64Native : NativeImpl := fun _ _ => .returned [.vector (.int .u64) []] [] []

#guard (initialState ⟨#[natModule]⟩ (fn 0 1) []).map (run ⟨#[natModule]⟩ emptyNative 100) ==
  some (.returned [.vector (.int .u64) [], .vector .bool []])
#guard (initialState ⟨#[natModule]⟩ (fn 0 1) []).map (run ⟨#[natModule]⟩ u64Native 100) ==
  some (.wrong .nativeContract)

/-! ### Linking generic functions (K1)

`0x1::a` defines `id<T>(x: T): T` and `cp<T: copy>(x: T): T`; `0x2::b`
imports one of them with the given type parameters and return type and calls
it at one type argument, held in extra signature 5. -/

def linkA : CompiledModule :=
  mkModule 0x1 "a"
    [{ name := "id", typeParameters := [{}], params := [tp], returns := [tp], code := #[.moveLoc 0, .ret] },
     { name := "cp", typeParameters := [{ copy := true }], params := [tp], returns := [tp],
       code := #[.moveLoc 0, .ret] }]

def linkB (name : String) (typeParameters : List AbilitySet) (returns : Signature)
    (typeArgs : Signature) (code : Array Bytecode) (mainReturns : Signature) : CompiledModule :=
  mkModule 0x2 "b" [{ name := "main", returns := mainReturns, code }]
    [{ address := 0x1, module := "a", name, params := [tp], returns, typeParameters }]
    [] [{ name := "N" }] (extraSignatures := [typeArgs]) (functionInstantiations := [⟨1, 5⟩])

#guard runFun ⟨#[linkA, linkB "id" [{}] [tp] [.u64] #[.ldU64 3, .callGeneric 0, .ret] [.u64]]⟩ 1 0 [] ==
  .returned [u64 3]
-- The handle has two type parameters, the definition one.
#guard runFun ⟨#[linkA, linkB "id" [{}, {}] [tp] [.u64, .bool] #[.ldU64 3, .callGeneric 0, .ret]
  [.u64]]⟩ 1 0 [] == .wrong .linkMismatch
-- The handle returns `u64` where the definition returns `T`: they agree at
-- `u64` only.
#guard runFun ⟨#[linkA, linkB "id" [{}] [.u64] [.u64] #[.ldU64 3, .callGeneric 0, .ret] [.u64]]⟩
  1 0 [] == .returned [u64 3]
#guard runFun ⟨#[linkA, linkB "id" [{}] [.u64] [.bool] #[.ldTrue, .callGeneric 0, .ret] [.u64]]⟩
  1 0 [] == .wrong .linkMismatch
-- The handle omits `cp`'s `copy` constraint (K1 excludes it): the semantics
-- checks the definition's.
#guard runFun ⟨#[linkA, linkB "cp" [{}] [tp] [.struct 0] #[.pack 0, .callGeneric 0, .ret]
  [.struct 0]]⟩ 1 0 [] == .wrong .constraintViolation

/-! ### Type specialization without bound (R7)

`grow<T>` calls `grow<Box<T>>`: each frame's type argument is larger than its
caller's. The semantics runs it until a limit stops it. -/

#guard runGen 21 [] (typeArgs := [tU64]) (fuel := 100) == .limit

def frameTypeArgs (fuel : Nat) : Option (List RuntimeType) := do
  let rec go : Nat → State → Option State
    | 0, s => some s
    | n + 1, s => match step genP s with
      | .done (.running s') => go n s'
      | _ => none
  let s ← go fuel (← initialState genP (fn 0 21 [tU64]) [])
  pure (← s.frames.head?).function.typeArgs

#guard frameTypeArgs 3 == some [tBox (tBox (tBox tU64))]

/-! ### Static types (C1) of the generic forms -/

def genTypeAfter (f : FunctionRef) (instr : Bytecode) (stack : List RuntimeType) :
    Option (List RuntimeType) :=
  typeAfter genP f (genModule genTest) instr stack

#guard genTypeAfter (fn 0 4 [tU64]) (.packGeneric 0) [tU64] == some [tBox tU64]
#guard genTypeAfter (fn 0 5 [tU64]) (.unpackGeneric 0) [tBox tU64] == some [tU64]
#guard genTypeAfter (fn 0 23) (.packGeneric 4) [tR] == none
#guard genTypeAfter (fn 0 23) (.packGeneric 12) [tU64] == some [.struct (gType "Cup" [tU64])]
#guard genTypeAfter (fn 0 1) (.callGeneric 0) [tU64] == some [tU64]
#guard genTypeAfter (fn 0 3) (.callGeneric 2) [tR] == none
#guard genTypeAfter (fn 0 23) (.call 0) [tU64] == none
#guard genTypeAfter (fn 0 19 [.bool]) (.callGeneric 9) [.bool] == some [tBox .bool]
#guard genTypeAfter (fn 0 20 [.bool]) (.vecPack (gs 2) 1) [.bool] == some [.vector .bool]
#guard genTypeAfter (fn 0 7 [tU64]) (.packVariantGeneric 0) [tU64] == some [tOpt tU64]
#guard genTypeAfter (fn 0 9 [tU64]) (.testVariantGeneric 0) [.ref false (tOpt tU64)] == some [.bool]
#guard genTypeAfter (fn 0 12 [tU64]) (.immBorrowVariantFieldGeneric 0) [.ref false (tOpt tU64)] ==
  some [.ref false tU64]
#guard genTypeAfter (fn 0 11 [tU64]) (.mutBorrowFieldGeneric 0) [.ref false (tBox tU64)] == none
#guard genTypeAfter (fn 0 15 [tU64]) (.existsGeneric 3) [.address] == some [.bool]
#guard genTypeAfter (fn 0 23) (.existsGeneric 8) [.address] == none
#guard genTypeAfter (fn 0 17 [tU64]) (.immBorrowGlobalGeneric 3) [.address] ==
  some [.ref false (.struct (gType "Holder" [tU64]))]

/-! ### Local instantiation (L7)

The instances of the module's own functions that a call instruction reaches,
under the caller's type arguments. `grow` is the book's "infinitely many types
-- NOT allowed" pattern: each step wraps its type argument once more, so L7
fails for this module. -/

#guard (genModule genTest).ownCalls 1 [] == [(0, [tU64]), (0, [.bool])]
#guard (genModule genTest).ownCalls 19 [tU64] == [(4, [tU64])]
#guard (genModule genTest).ownCalls 21 [tU64] == [(21, [tBox tU64])]
#guard (genModule genTest).ownCalls 21 [tBox tU64] == [(21, [tBox (tBox tU64)])]
-- `natModule`'s `main` calls no function of its own with code: `empty` is its
-- own native, which counts.
#guard natModule.ownCalls 1 [] == [(0, [tU64]), (0, [.bool])]

/-! ### Syntactic sizes (L11) -/

#guard (boxOf (.vector .u64)).nodeCount == 3
#guard (boxOf (.vector .u64)).depth == 3
#guard (SignatureToken.function [.u64, .reference tp] [] {}).nodeCount == 4
#guard (SignatureToken.function [.u64, .reference tp] [] {}).depth == 3

/-! ### The ghost layer on generic code

Every returning run above is accounted, changes no outcome, and passes the
D2, D3 and D4 checks (the struct arguments are of types the transaction
layer accepts). Each generic form is annotated as its non-generic form, and
its D3a event records the instance. -/

def genCases : List (Nat × List Value × List RuntimeType × GlobalStorage × List Value) :=
  [(1, [], [], [], []), (0, [u64 3], [tU64], [], []), (2, [u64 4], [tU64], [], []),
   (6, [], [], [], []), (4, [.bool true], [.bool], [], []),
   (5, [boxV .bool (.bool true)], [.bool], [], []), (19, [u64 7], [tU64], [], []),
   (19, [boxV tU64 (u64 7)], [tBox tU64], [], []), (20, [.bool true], [.bool], [], []),
   (7, [u64 5], [tU64], [], []), (8, [someV tU64 (u64 5)], [tU64], [], []),
   (22, [signerRef 0], [], [], [signerAt 1]),
   (15, [.address 1], [tU64], [holderAt tU64 (u64 1)], []),
   (16, [.address 1], [.bool], [holderAt tU64 (u64 1), holderAt .bool (.bool true)], []),
   (17, [.address 1], [tU64], [holderAt tU64 (u64 1)], []),
   (18, [.address 1, u64 9], [tU64], [holderAt tU64 (u64 1)], [])]

def genAccepted : List StructId := [gType "Box" [.bool], gType "Box" [tU64], gType "Opt" [tU64]]

#guard genCases.all fun (i, args, tys, st, held) =>
  runGen i args tys st held matches .returned _ _
#guard genCases.all fun (i, args, tys, st, held) =>
  agrees genP 0 i args (storage := st) (held := held) (typeArgs := tys)
#guard genCases.all fun (i, args, tys, st, held) =>
  violations genP 0 i args (auth := [1]) (A := genAccepted) (storage := st) (held := held)
    (typeArgs := tys) == []
#guard genCases.all fun (i, args, tys, st, held) =>
  refViolations genP 0 i args st held tys == []
#guard refThroughGeneric.all fun (f, args, _) =>
  agrees (genP f) 0 23 args && violations (genP f) 0 23 args (A := genAccepted) == [] &&
    refViolations (genP f) 0 23 args == []
-- Runs that fail or go wrong are accounted up to the step that ends them.
#guard agrees genP 0 3 []
#guard agrees genP 0 8 [noneV tU64] (typeArgs := [tU64])
#guard agrees (genP (genFun #[.pack 3, .packGeneric 4, .pop, .ret])) 0 23 []

-- `ExistsGeneric` and `MoveToGeneric` record the instance they act on (D3a).
#guard (traceFun genP 0 15 [.address 1] (storage := [holderAt tU64 (u64 1)]) (typeArgs := [tU64])).events.contains
  (.privileged gMod (gType "Holder" [tU64]) (.existsGeneric 3))
#guard (traceFun genP 0 22 [signerRef 0] (held := [signerAt 1])).events.contains
  (.privileged gMod (gType "Holder" [.bool]) (.moveToGeneric 3))
-- `PackGeneric` creates the instance's value, `UnpackGeneric` consumes it, in
-- the type's module.
#guard (traceFun genP 0 6 []).ownershipEvents.contains
  (.created 1 (boxV tU64 (u64 7)) (.instruction gMod (.packGeneric 0)))
#guard (traceFun genP 0 6 []).ownershipEvents.contains
  (.consumed 1 (boxV tU64 (u64 7)) (.unpack gMod))
-- A `Box<u64>` argument of a type the transaction layer does not accept (E1).
#guard violations genP 0 5 [boxV tU64 (u64 1)] (typeArgs := [tU64]) == ["D3a"]

/-! ## Function values (Phase 4, stage B)

Module `0x42::c` declares four types:

- struct 0, `R {}`, with no abilities;
- struct 1, `Holder has key { |u64|u64 has store }`;
- struct 2, `Leaky has key { |u64|u64 }`, whose `key` its field does not
  justify (an L6 violation);
- struct 3, `D has drop {}`,

and the 14 functions of `closFuns` (handle `i` is function `i`), then one test
function, number 14. `cs k` is extra signature `k`: `[u64]`,
`[|u64|u64 has copy+drop]`, `[|u64|u64]`, `[|u64|u64 has store]`,
`[|&u64|u64]`, `[|address|bool]`, `[|u64, u64|u64]`, `[||u64]`, `[R]`,
`[|u64|u64 has drop]`, `[T]`. Function instantiations: 0 `id<u64>`,
1 `id<|u64|u64 has copy+drop>`, 2 `cp<R>`, 3 `sub<u64>` (of a non-generic
function), 4 `id<T>`. A mask's bit `i` captures parameter `i`. -/

def closCount : Nat := 15

def cs (k : Nat) : SignatureIndex := 3 * closCount + k

def fnTok (ps rs : List SignatureToken) (a : AbilitySet := {}) : SignatureToken :=
  .function ps rs a

def copyDrop : AbilitySet := ⟨true, true, false, false⟩
def copyDropStore : AbilitySet := ⟨true, true, true, false⟩

def closStructs : List StructDecl :=
  [{ name := "R" },
   { name := "Holder", abilities := { key := true }, fields := [fnTok [.u64] [.u64] { store := true }] },
   { name := "Leaky", abilities := { key := true }, fields := [fnTok [.u64] [.u64]] },
   { name := "D", abilities := { drop := true } }]

def closFuns : List Fun :=
  [{ name := "sub", params := [.u64, .u64], returns := [.u64],
     code := #[.moveLoc 0, .moveLoc 1, .sub, .ret] },
   { name := "subPriv", params := [.u64, .u64], returns := [.u64], visibility := .private_,
     code := #[.moveLoc 0, .moveLoc 1, .sub, .ret] },
   { name := "subPers", params := [.u64, .u64], returns := [.u64], visibility := .private_,
     attributes := [.persistent], code := #[.moveLoc 0, .moveLoc 1, .sub, .ret] },
   { name := "id", typeParameters := [{}], params := [tp], returns := [tp], code := #[.moveLoc 0, .ret] },
   { name := "takeR", params := [.struct 0, .u64], returns := [.u64], visibility := .private_,
     code := #[.moveLoc 0, .unpack 0, .moveLoc 1, .ret] },
   { name := "readRef", params := [.reference .u64], returns := [.u64],
     code := #[.moveLoc 0, .readRef, .ret] },
   { name := "capRef", params := [.reference .u64, .u64], returns := [.u64],
     code := #[.moveLoc 1, .ret] },
   { name := "peek", params := [.address], returns := [.bool], code := #[.moveLoc 0, .exists_ 1, .ret] },
   { name := "nat", params := [.u64], returns := [.u64], native := true },
   { name := "locked", params := [fnTok [.u64] [.u64]], returns := [.u64], attributes := [.moduleLock],
     code := #[.ldU64 1, .moveLoc 0, .callClosure (cs 2), .ret] },
   { name := "unlocked", params := [fnTok [.u64] [.u64]], returns := [.u64],
     code := #[.ldU64 1, .moveLoc 0, .callClosure (cs 2), .ret] },
   { name := "inc", params := [.u64], returns := [.u64], code := #[.moveLoc 0, .ldU64 1, .add, .ret] },
   { name := "cp", typeParameters := [{ copy := true }], params := [tp], returns := [tp],
     code := #[.moveLoc 0, .ret] },
   { name := "dropD", params := [.struct 3, .u64], returns := [.u64], code := #[.moveLoc 1, .ret] }]

#guard closFuns.length + 1 == closCount

/-- The closure module, with test function `test` last. -/
def closModule (test : Fun) : CompiledModule :=
  mkModule 0x42 "c" (closFuns ++ [test]) [] [] closStructs [] []
    [[.u64], [fnTok [.u64] [.u64] copyDrop], [fnTok [.u64] [.u64]], [fnTok [.u64] [.u64] { store := true }],
     [fnTok [.reference .u64] [.u64]], [fnTok [.address] [.bool]], [fnTok [.u64, .u64] [.u64]],
     [fnTok [] [.u64]], [.struct 0], [fnTok [.u64] [.u64] { drop := true }], [tp]]
    (functionInstantiations := [⟨3, cs 0⟩, ⟨3, cs 1⟩, ⟨12, cs 8⟩, ⟨0, cs 0⟩, ⟨3, cs 10⟩])

def closP (test : Fun) : Program := ⟨#[closModule test]⟩

/-- Run a test function of the closure module. -/
def runClos (code : Array Bytecode) (returns : Signature := []) (params : Signature := [])
    (locals : Signature := []) (args : List Value := []) (storage : GlobalStorage := [])
    (held : List Value := []) : RunResult :=
  runFun (closP { name := "test", params, returns, locals, code }) 0 14 args (storage := storage)
    (held := held)

def closTest (code : Array Bytecode) (returns : Signature := []) (params : Signature := [])
    (locals : Signature := []) : Fun :=
  { name := "test", params, returns, locals, code }

def cMod : ModuleId := ⟨0x42, "c"⟩
def tFn (ps rs : List RuntimeType) (a : AbilitySet := {}) : RuntimeType := .function ps rs a
/-- The closure over `sub` capturing `x`. -/
def subX (x : Nat) : Value := .closure (fn 0 0) 1#64 [u64 x] [tU64] [tU64] copyDropStore

/-! ### Masks -/

#guard maskSplit 2#64 [1, 2, 3] == ([2], [1, 3])
#guard maskCompose 2#64 [9] [1, 3] == [1, 9, 3]
#guard maskCompose 5#64 [7, 8] [0] == [7, 0, 8]
#guard maskWithin 3#64 2 && !maskWithin 4#64 2

/-! ### `PackClosure` and `CallClosure`

`sub(x, y) = x - y`: capturing `x = 10` and providing `y = 3` gives `7`;
capturing `y = 10` and providing `x = 3` underflows. -/

#guard runClos #[.ldU64 3, .ldU64 10, .packClosure 0 1#64, .callClosure (cs 1), .ret] [.u64] ==
  .returned [u64 7]
#guard runClos #[.ldU64 3, .ldU64 10, .packClosure 0 2#64, .callClosure (cs 1), .ret] [.u64] ==
  .failed .arithmetic
-- Nothing captured: the closure takes both arguments.
#guard runClos #[.ldU64 10, .ldU64 3, .packClosure 0 0#64, .callClosure (cs 6), .ret] [.u64] ==
  .returned [u64 7]
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .ret] [fnTok [.u64] [.u64] copyDropStore] ==
  .returned [subX 10]
-- A closure over a private function of the module, and over a persistent one.
#guard runClos #[.ldU64 3, .ldU64 10, .packClosure 1 1#64, .callClosure (cs 1), .ret] [.u64] ==
  .returned [u64 7]
#guard runClos #[.ldU64 10, .packClosure 1 1#64, .ret] [fnTok [.u64] [.u64] copyDrop] ==
  .returned [.closure (fn 0 1) 1#64 [u64 10] [tU64] [tU64] copyDrop]
#guard runClos #[.ldU64 10, .packClosure 2 1#64, .ret] [fnTok [.u64] [.u64] copyDropStore] ==
  .returned [.closure (fn 0 2) 1#64 [u64 10] [tU64] [tU64] copyDropStore]
-- A closure taking a reference: the parameters of a function type may be
-- references.
#guard runClos #[.immBorrowLoc 0, .packClosure 5 0#64, .callClosure (cs 4), .ret] [.u64] [.u64]
  (args := [u64 5]) == .returned [u64 5]

/-! ### What `PackClosure` rejects -/

-- Capturing a reference (D5a, D4d).
#guard runClos #[.packClosure 6 1#64, .ret] == .wrong .capturedReference
-- A mask bit naming a parameter `sub` does not have.
#guard runClos #[.ldU64 1, .packClosure 0 4#64, .ret] == .wrong .badOperand
-- A captured value of the wrong type, and too few.
#guard runClos #[.ldTrue, .packClosure 0 1#64, .ret] == .wrong .typeMismatch
#guard runClos #[.packClosure 0 1#64, .ret] == .wrong .stackUnderflow
-- `PackClosure` of a generic function, `PackClosureGeneric` of a non-generic one,
-- and type arguments violating a constraint (`cp<R>`, `cp<T: copy>`).
#guard runClos #[.packClosure 3 0#64, .ret] == .wrong .badOperand
#guard runClos #[.packClosureGeneric 3 0#64, .ret] == .wrong .badOperand
#guard runClos #[.packClosureGeneric 2 0#64, .pop, .ret] == .wrong .constraintViolation
#guard runClos #[.packClosureGeneric 99 0#64, .ret] == .wrong .badIndex
#guard runClos #[.packClosure 99 0#64, .ret] == .wrong .unresolvedFunction
-- A closure over a native is outside the subset.
#guard runClos #[.packClosure 8 0#64, .pop, .ret] == .wrong (.unsupported "closure over a native")

/-! ### What `CallClosure` rejects -/

-- The closure's type must have the instruction's parameters and results, and
-- at least its abilities.
#guard runClos #[.ldU64 1, .ldU64 2, .ldU64 10, .packClosure 0 1#64, .callClosure (cs 6), .ret] [.u64] ==
  .wrong .typeMismatch
#guard runClos #[.ldU64 3, .ldU64 10, .packClosure 1 1#64, .callClosure (cs 3), .ret] [.u64] ==
  .wrong .typeMismatch
#guard runClos #[.ldU64 1, .ldU64 2, .callClosure (cs 2), .ret] [.u64] == .wrong .typeMismatch
-- The arguments must conform, and be there.
#guard runClos #[.ldTrue, .ldU64 10, .packClosure 0 1#64, .callClosure (cs 2), .ret] [.u64] ==
  .wrong .typeMismatch
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .callClosure (cs 2), .ret] [.u64] ==
  .wrong .stackUnderflow
-- The signature operand is one function type (L9).
#guard runClos #[.ldU64 1, .callClosure (cs 0), .ret] [.u64] == .wrong .badOperand
#guard runClos #[.callClosure 999, .ret] == .wrong .badIndex

/-! ### Generic targets -/

#guard runClos #[.ldU64 4, .packClosureGeneric 0 0#64, .callClosure (cs 2), .ret] [.u64] ==
  .returned [u64 4]
#guard runClos #[.ldU64 5, .packClosureGeneric 0 1#64, .callClosure (cs 7), .ret] [.u64] ==
  .returned [u64 5]
-- A closure through `id<|u64|u64 has copy+drop>`: a function type is a type
-- argument, and the closure, with more abilities, is assignable to it.
#guard runClos #[.ldU64 3, .ldU64 10, .packClosure 0 1#64, .callGeneric 1, .callClosure (cs 1), .ret]
  [.u64] == .returned [u64 7]

/-! ### Abilities and subsumption

A closure's abilities are its target's, `copy` and `drop`, and `store` for a
storable target, intersected with its captured parameters'. A value with more
abilities is assignable where a function type with fewer is expected. -/

-- A closure capturing an `R` has no ability: it can be called, not dropped or
-- copied.
#guard runClos #[.ldU64 7, .pack 0, .packClosure 4 1#64, .callClosure (cs 2), .ret] [.u64] ==
  .returned [u64 7]
#guard runClos #[.pack 0, .packClosure 4 1#64, .pop, .ret] == .wrong .dropWithoutAbility
#guard runClos #[.pack 0, .packClosure 4 1#64, .stLoc 0, .copyLoc 0, .ret] [] [] [fnTok [.u64] [.u64]] ==
  .wrong .copyWithoutAbility
-- The captured values' abilities count, not the captured parameters' declared
-- types: `unlocked` takes a `|u64|u64` without abilities, and captures a
-- closure over `inc` with `copy`, `drop` and `store`, so the new closure has
-- them too and may be dropped.
def incClosure : Value := .closure (fn 0 11) 0#64 [] [tU64] [tU64] copyDropStore

#guard runClos #[.packClosure 11 0#64, .packClosure 10 1#64, .pop, .ret] == .returned []
#guard runClos #[.packClosure 11 0#64, .packClosure 10 1#64, .ret] [fnTok [] [.u64] copyDropStore] ==
  .returned [.closure (fn 0 10) 1#64 [incClosure] [] [tU64] copyDropStore]
#guard runClos #[.packClosure 11 0#64, .packClosure 10 1#64, .callClosure (cs 7), .ret] [.u64] ==
  .returned [u64 2]
#guard (Value.closure (fn 0 10) 1#64 [incClosure] [] [tU64] {}).wellFormed (closP genTest) == false
-- More abilities where fewer are expected: in a local, a return value, a
-- parameter, a vector element.
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .stLoc 0, .moveLoc 0, .ret] [fnTok [.u64] [.u64]] []
  [fnTok [.u64] [.u64] { drop := true }] == .returned [subX 10]
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .ret] [fnTok [.u64] [.u64]] == .returned [subX 10]
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .call 10, .ret] [.u64] == .returned [u64 9]
#guard runClos #[.ldU64 1, .packClosure 0 1#64, .ldU64 2, .packClosure 1 1#64, .vecPack (cs 9) 2, .ret]
  [.vector (fnTok [.u64] [.u64] { drop := true })] ==
  .returned [.vector (tFn [tU64] [tU64] { drop := true })
    [subX 1, .closure (fn 0 1) 1#64 [u64 2] [tU64] [tU64] copyDrop]]
-- Fewer abilities where more are expected.
#guard runClos #[.ldU64 10, .packClosure 1 1#64, .ret] [fnTok [.u64] [.u64] { store := true }] ==
  .wrong .typeMismatch
#guard (subX 10).conforms (closP genTest) (tFn [tU64] [tU64] { drop := true })
#guard !(subX 10).conforms (closP genTest) (tFn [tU64] [tU64] ⟨true, true, true, true⟩)
#guard !(subX 10).conforms (closP genTest) (tFn [tU64, tU64] [tU64])
#guard (Value.closure (fn 0 0) 1#64 [u64 10] [tU64] [tU64] copyDrop).wellFormed (closP genTest) == false
#guard (Value.closure (fn 0 1) 1#64 [u64 10] [tU64] [tU64] copyDropStore).wellFormed (closP genTest) ==
  false
-- A mutable reference to a local of a function type has the local's declared
-- type: a closure with fewer abilities than the one it replaces, but as many
-- as the local's type, may be written through it.
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .stLoc 0, .pack 3, .packClosure 13 1#64, .mutBorrowLoc 0,
    .writeRef, .ldU64 5, .moveLoc 0, .callClosure (cs 9), .ret] [.u64] []
  [fnTok [.u64] [.u64] { drop := true }] == .returned [u64 5]

/-! ### Equality

Closures are equal when they are the same function instance with the same
mask and captured values; two closures whose types differ in abilities only
may be compared. -/

#guard runClos #[.ldU64 10, .packClosure 0 1#64, .ldU64 10, .packClosure 0 1#64, .eq, .ret] [.bool] ==
  .returned [.bool true]
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .ldU64 11, .packClosure 0 1#64, .eq, .ret] [.bool] ==
  .returned [.bool false]
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .ldU64 10, .packClosure 1 1#64, .eq, .ret] [.bool] ==
  .returned [.bool false]
#guard runClos #[.ldU64 10, .packClosure 0 1#64, .packClosure 11 0#64, .neq, .ret] [.bool] ==
  .returned [.bool true]
#guard runClos #[.packClosure 0 0#64, .packClosure 11 0#64, .eq, .ret] [.bool] == .wrong .typeMismatch

/-- Two immutable references of static type `&F`, `F = |u64|u64 has copy+drop`,
whose targets are `F1 = |u64|u64 has copy+drop+store` and `F`: a closure in a
local `f : F1`, copied into a local `l : F`; `&f`, stored in a local `x : &F`
(`&` is covariant), and `&l`. `Eq` compares the closures they designate. -/
def refsToClosures : Fun :=
  closTest #[.ldU64 10, .packClosure 0 1#64, .stLoc 0, .copyLoc 0, .stLoc 1, .immBorrowLoc 0, .stLoc 2,
      .moveLoc 2, .immBorrowLoc 1, .eq, .ret] [.bool] []
    [fnTok [.u64] [.u64] copyDropStore, fnTok [.u64] [.u64] copyDrop, .reference (fnTok [.u64] [.u64] copyDrop)]

#guard runFun (closP refsToClosures) 0 14 [] == .returned [.bool true]
-- The targets differ: equality of the types would reject the comparison.
#guard (RuntimeType.ref false (tFn [tU64] [tU64] copyDropStore)) != .ref false (tFn [tU64] [tU64] copyDrop)

/-! `RuntimeType.comparable` is "assignable to a common type": checked on every
pair of a set of types closed under assignability (function types `|u64|u64`
at every set of abilities without `key`, `&`, `&mut` and vectors of them, and
`u64`). -/

def abilitySubsets : List AbilitySet :=
  [false, true].flatMap fun c => [false, true].flatMap fun d => [false, true].map fun st =>
    ⟨c, d, st, false⟩

def comparableUniverse : List RuntimeType :=
  tU64 :: abilitySubsets.flatMap fun a =>
    let f := tFn [tU64] [tU64] a
    [f, .ref false f, .ref true f, .vector f]

#guard comparableUniverse.all fun t => comparableUniverse.all fun u =>
  RuntimeType.comparable t u == comparableUniverse.any fun w => t.assignable w && u.assignable w

/-! ### Storability

Only a closure over a storable function (`public` or `persistent`) has
`store`, so only such a closure goes to global storage. -/

def holderOf (v : Value) : Resource := ⟨1, ⟨cMod, "Holder", []⟩, .struct ⟨cMod, "Holder", []⟩ [v]⟩

#guard runClos #[.moveLoc 0, .ldU64 1, .packClosure 0 1#64, .pack 1, .moveTo 1, .ret] [] [.reference .signer]
  (args := [signerRef 0]) (held := [signerAt 1]) == .returned [] [holderOf (subX 1)]
#guard runClos #[.moveLoc 0, .ldU64 1, .packClosure 2 1#64, .pack 1, .moveTo 1, .ret] [] [.reference .signer]
  (args := [signerRef 0]) (held := [signerAt 1]) ==
  .returned [] [holderOf (.closure (fn 0 2) 1#64 [u64 1] [tU64] [tU64] copyDropStore)]
#guard runClos #[.moveLoc 0, .ldU64 1, .packClosure 1 1#64, .pack 1, .moveTo 1, .ret] [] [.reference .signer]
  (args := [signerRef 0]) (held := [signerAt 1]) == .wrong .typeMismatch
-- `Leaky` breaks L6, but the value check still keeps a non-storable closure
-- out of global storage.
#guard runClos #[.moveLoc 0, .ldU64 1, .packClosure 1 1#64, .pack 2, .moveTo 2, .ret] [] [.reference .signer]
  (args := [signerRef 0]) (held := [signerAt 1]) == .wrong .globalWithoutKey
-- A stored closure is called after it is moved out.
#guard runClos #[.ldU64 4, .moveLoc 0, .moveFrom 1, .unpack 1, .callClosure (cs 2), .ret] [.u64] [.address]
  (args := [.address 1]) (storage := [holderOf (subX 10)]) == .returned [u64 6] []

/-! `entry` alone does not make a function storable (ruling 2026-11-03).
`0x43::s` has three `|u64|u64` functions, a public `pub`, a private `entry`
function `ent` without `persistent`, and a private `pers` with it; struct 0,
`Holder has key { |u64|u64 has store }`; and test function 3. -/

def storModule (test : Fun) : CompiledModule :=
  mkModule 0x43 "s"
    [{ name := "pub", params := [.u64], returns := [.u64], code := #[.moveLoc 0, .ret] },
     { name := "ent", params := [.u64], returns := [.u64], visibility := .private_, isEntry := true,
       code := #[.moveLoc 0, .ret] },
     { name := "pers", params := [.u64], returns := [.u64], visibility := .private_,
       attributes := [.persistent], code := #[.moveLoc 0, .ret] },
     test]
    (structs := [{ name := "Holder", abilities := { key := true },
                   fields := [fnTok [.u64] [.u64] { store := true }] }])

def storP (test : Fun) : Program := ⟨#[storModule test]⟩

/-- Store a closure over function `k` of `0x43::s` at the signer's address. -/
def storeClosureOf (k : Nat) : RunResult :=
  runFun (storP (closTest #[.moveLoc 0, .packClosure k 0#64, .pack 0, .moveTo 0, .ret] [] [.reference .signer]))
    0 3 [signerRef 0] (held := [signerAt 1])

def sHolderOf (v : Value) : Resource :=
  ⟨1, ⟨⟨0x43, "s"⟩, "Holder", []⟩, .struct ⟨⟨0x43, "s"⟩, "Holder", []⟩ [v]⟩

#guard (storP genTest).storable (fn 0 0) && !(storP genTest).storable (fn 0 1) &&
  (storP genTest).storable (fn 0 2)
#guard (storP genTest).closureAbilities (fn 0 1) == copyDrop
-- A closure over `ent` has `copy` and `drop`, not `store`.
#guard runFun (storP (closTest #[.packClosure 1 0#64, .ret] [fnTok [.u64] [.u64] copyDrop])) 0 3 [] ==
  .returned [.closure (fn 0 1) 0#64 [] [tU64] [tU64] copyDrop]
#guard runFun (storP (closTest #[.packClosure 1 0#64, .ret] [fnTok [.u64] [.u64] { store := true }])) 0 3 [] ==
  .wrong .typeMismatch
-- Closures over `pub` and `pers` go to global storage; one over `ent` does not.
#guard storeClosureOf 0 == .returned [] [sHolderOf (.closure (fn 0 0) 0#64 [] [tU64] [tU64] copyDropStore)]
#guard storeClosureOf 2 == .returned [] [sHolderOf (.closure (fn 0 2) 0#64 [] [tU64] [tU64] copyDropStore)]
#guard storeClosureOf 1 == .wrong .typeMismatch

/-! ### Visibility (K2) and dispatch to private functions

`0x1::a` has a private `secret(): u64` and a public `give(): || u64` that
returns a closure over it; `0x2::b` imports both. Creating a closure over
`secret` in `b` is a visibility violation; calling the one `a` hands out is
not. -/

def visA : CompiledModule :=
  mkModule 0x1 "a"
    [{ name := "secret", returns := [.u64], visibility := .private_, code := #[.ldU64 7, .ret] },
     { name := "give", returns := [fnTok [] [.u64] copyDrop], code := #[.packClosure 0 0#64, .ret] }]

def visB : CompiledModule :=
  mkModule 0x2 "b"
    [{ name := "forge", code := #[.packClosure 2 0#64, .pop, .ret] },
     { name := "useGive", returns := [.u64], code := #[.call 3, .callClosure 10, .ret] }]
    [{ address := 0x1, module := "a", name := "secret", returns := [.u64] },
     { address := 0x1, module := "a", name := "give", returns := [fnTok [] [.u64] copyDrop] }]
    (extraSignatures := [[fnTok [] [.u64] copyDrop]])

#guard runFun ⟨#[visA, visB]⟩ 1 0 [] == .wrong .visibility
#guard runFun ⟨#[visA, visB]⟩ 1 1 [] == .returned [u64 7]

/-! ### Reentrancy through closures (R3, R4)

Closure dispatch counts the target's module as active even within one module:
`peek` checks a resource of `c`, from `c`'s own closure. -/

#guard runClos #[.moveLoc 0, .call 7, .ret] [.bool] [.address] (args := [.address 1]) ==
  .returned [.bool false]
#guard runClos #[.moveLoc 0, .packClosure 7 0#64, .callClosure (cs 5), .ret] [.bool] [.address]
  (args := [.address 1]) == .failed .resourceLocked
-- Under `#[module_lock]`, dispatching to an active module fails.
#guard runClos #[.packClosure 11 0#64, .call 9, .ret] [.u64] == .failed .moduleLocked
#guard runClos #[.packClosure 11 0#64, .call 10, .ret] [.u64] == .returned [u64 2]

/-! ### Static types (C1) -/

def closTypeAfter (instr : Bytecode) (stack : List RuntimeType)
    (test : Fun := closTest #[.ret] [] [] [fnTok [.u64] [.u64] { drop := true }]) (f : FunctionRef := fn 0 14) :
    Option (List RuntimeType) :=
  typeAfter (closP test) f (closModule test) instr stack

-- The closure's static abilities are its captured values' stack types', not the
-- captured parameters' declared ones.
#guard closTypeAfter (.packClosure 10 1#64) [tFn [tU64] [tU64] copyDropStore] ==
  some [tFn [] [tU64] copyDropStore]
#guard closTypeAfter (.packClosure 10 1#64) [tFn [tU64] [tU64] copyDrop] == some [tFn [] [tU64] copyDrop]
#guard closTypeAfter (.packClosure 10 1#64) [tFn [tU64] [tU64] {}] == some [tFn [] [tU64] {}]
-- In `id<T>`, `PackClosureGeneric(id<T>, 0b1)` capturing a `T` has an
-- instance-dependent type: `copy`, `drop` and `store` at `u64`, `drop` at `D`,
-- none at `R`. One token cannot name it at every instance, so a certificate's
-- successor types are supertypes of the result (`TypeCertificate.Consistent`).
#guard closTypeAfter (.packClosureGeneric 4 1#64) [tU64] (f := fn 0 3 [tU64]) ==
  some [tFn [] [tU64] copyDropStore]
#guard closTypeAfter (.packClosureGeneric 4 1#64) [.struct ⟨cMod, "D", []⟩] (f := fn 0 3 [.struct ⟨cMod, "D", []⟩]) ==
  some [tFn [] [.struct ⟨cMod, "D", []⟩] { drop := true }]
#guard closTypeAfter (.packClosureGeneric 4 1#64) [.struct ⟨cMod, "R", []⟩] (f := fn 0 3 [.struct ⟨cMod, "R", []⟩]) ==
  some [tFn [] [.struct ⟨cMod, "R", []⟩] {}]
-- A certified type with fewer abilities is a supertype of each.
#guard RuntimeType.assignableAll [tFn [] [tU64] copyDropStore] [tFn [] [tU64] {}]

#guard closTypeAfter (.packClosure 0 1#64) [tU64] == some [tFn [tU64] [tU64] copyDropStore]
#guard closTypeAfter (.packClosure 1 1#64) [tU64] == some [tFn [tU64] [tU64] copyDrop]
#guard closTypeAfter (.packClosure 4 1#64) [tR] == none
#guard closTypeAfter (.packClosure 4 1#64) [.struct ⟨cMod, "R", []⟩] == some [tFn [tU64] [tU64]]
#guard closTypeAfter (.packClosure 6 1#64) [] == none
#guard closTypeAfter (.packClosureGeneric 0 1#64) [tU64] == some [tFn [] [tU64] copyDropStore]
#guard closTypeAfter (.callClosure (cs 2)) [tFn [tU64] [tU64] copyDrop, tU64] == some [tU64]
#guard closTypeAfter (.callClosure (cs 3)) [tFn [tU64] [tU64] copyDrop, tU64] == none
#guard closTypeAfter (.stLoc 0) [tFn [tU64] [tU64] copyDropStore] == some []
#guard closTypeAfter (.stLoc 0) [tFn [tU64] [tU64] {}] == none
#guard closTypeAfter .eq [tFn [tU64] [tU64] copyDrop, tFn [tU64] [tU64] copyDropStore] == some [.bool]
#guard closTypeAfter .eq [.ref false (tFn [tU64] [tU64] copyDrop), .ref false (tFn [tU64] [tU64] copyDropStore)]
  == some [.bool]
#guard closTypeAfter .eq [.ref true (tFn [tU64] [tU64] copyDrop), .ref true (tFn [tU64] [tU64] copyDropStore)]
  == none

/-! ### Local instantiation (L7) follows closure creation -/

#guard (closModule (closTest #[.packClosureGeneric 0 0#64, .packClosure 0 0#64, .pop, .pop, .ret])).ownCalls
  14 [] == [(3, [tU64]), (0, [])]

/-! ### The ghost layer on closures (D2, D3c, D4)

Every returning closure run above is accounted and changes no outcome, and no
D2, D3 or D4 check fails. A closure's captured values are its children:
`CallClosure` consumes the closure and moves them into the target's
parameters. -/

def closCases : List (Fun × List Value × GlobalStorage × List Value) :=
  [(closTest #[.ldU64 3, .ldU64 10, .packClosure 0 1#64, .callClosure (cs 1), .ret] [.u64], [], [], []),
   (closTest #[.ldU64 10, .ldU64 3, .packClosure 0 0#64, .callClosure (cs 6), .ret] [.u64], [], [], []),
   (closTest #[.ldU64 10, .packClosure 0 1#64, .ret] [fnTok [.u64] [.u64] copyDropStore], [], [], []),
   (closTest #[.immBorrowLoc 0, .packClosure 5 0#64, .callClosure (cs 4), .ret] [.u64] [.u64],
     [u64 5], [], []),
   (closTest #[.ldU64 4, .packClosureGeneric 0 0#64, .callClosure (cs 2), .ret] [.u64], [], [], []),
   (closTest #[.ldU64 3, .ldU64 10, .packClosure 0 1#64, .callGeneric 1, .callClosure (cs 1), .ret] [.u64],
     [], [], []),
   (closTest #[.ldU64 7, .pack 0, .packClosure 4 1#64, .callClosure (cs 2), .ret] [.u64], [], [], []),
   (closTest #[.ldU64 10, .packClosure 0 1#64, .stLoc 0, .moveLoc 0, .ret] [fnTok [.u64] [.u64]] []
     [fnTok [.u64] [.u64] { drop := true }], [], [], []),
   (closTest #[.ldU64 1, .packClosure 0 1#64, .ldU64 2, .packClosure 1 1#64, .vecPack (cs 9) 2, .ret]
     [.vector (fnTok [.u64] [.u64] { drop := true })], [], [], []),
   (closTest #[.ldU64 10, .packClosure 0 1#64, .stLoc 0, .pack 3, .packClosure 13 1#64, .mutBorrowLoc 0,
     .writeRef, .ldU64 5, .moveLoc 0, .callClosure (cs 9), .ret] [.u64] []
     [fnTok [.u64] [.u64] { drop := true }], [], [], []),
   (closTest #[.ldU64 10, .packClosure 0 1#64, .ldU64 10, .packClosure 1 1#64, .eq, .ret] [.bool], [], [], []),
   (closTest #[.moveLoc 0, .ldU64 1, .packClosure 0 1#64, .pack 1, .moveTo 1, .ret] [] [.reference .signer],
     [signerRef 0], [], [signerAt 1]),
   (closTest #[.ldU64 4, .moveLoc 0, .moveFrom 1, .unpack 1, .callClosure (cs 2), .ret] [.u64] [.address],
     [.address 1], [holderOf (subX 10)], []),
   (closTest #[.packClosure 11 0#64, .call 10, .ret] [.u64], [], [], []),
   (refsToClosures, [], [], []),
   (closTest #[.packClosure 11 0#64, .packClosure 10 1#64, .pop, .ret], [], [], []),
   (closTest #[.packClosure 11 0#64, .packClosure 10 1#64, .callClosure (cs 7), .ret] [.u64], [], [], [])]

#guard closCases.all fun (f, args, st, held) =>
  runFun (closP f) 0 14 args (storage := st) (held := held) matches .returned _ _
#guard closCases.all fun (f, args, st, held) => agrees (closP f) 0 14 args (storage := st) (held := held)
#guard closCases.all fun (f, args, st, held) =>
  violations (closP f) 0 14 args (auth := [1]) (storage := st) (held := held) == []
#guard closCases.all fun (f, args, st, held) => refViolations (closP f) 0 14 args st held == []
#guard [(#[.moveLoc 0, .packClosure 7 0#64, .callClosure (cs 5), .ret], [Value.address 1])].all
  fun (code, args) => agrees (closP (closTest code [.bool] [.address])) 0 14 args
#guard agrees (closP (closTest #[.packClosure 11 0#64, .call 9, .ret] [.u64])) 0 14 []

-- `CallClosure` consumes the closure; its captured `u64` moves into `sub`'s
-- first parameter.
#guard (traceFun (closP (closTest #[.ldU64 3, .ldU64 10, .packClosure 0 1#64, .callClosure (cs 1), .ret]
    [.u64])) 0 14 []).ownershipEvents ==
  [.created 0 (u64 3) (.instruction cMod (.ldU64 3)), .created 1 (u64 10) (.instruction cMod (.ldU64 10)),
   .created 2 (subX 10) (.instruction cMod (.packClosure 0 1#64)),
   .consumed 2 (subX 10) (.callClosure cMod),
   .created 3 (u64 7) (.instruction cMod .sub)]
-- A closure the transaction supplies breaks D3c: no transaction creates one.
#guard violations (closP (closTest #[.ldU64 3, .moveLoc 0, .callClosure (cs 1), .ret] [.u64]
  [fnTok [.u64] [.u64] copyDrop])) 0 14 [subX 10] == ["D3c"]
-- The reference passed to a closure over `readRef` is used, and the target is
-- entered with it as its reference parameter.
#guard ((traceFun (closP (closTest #[.immBorrowLoc 0, .packClosure 5 0#64, .callClosure (cs 4), .ret] [.u64]
    [.u64])) 0 14 [u64 5]).events.filterMap fun e => match e with
      | .entered f ps => some (f, ps.length)
      | _ => none) == [(0, 0), (1, 1)]

/-! ### Function values (D5)

`d5Violations` evaluates D5 (`Spec.Properties.FunctionValues`) at every state
and step of a run. -/

def d5From (P : Program) (natives : NativeImpl) : Nat → State → List String
  | 0, _ => []
  | fuel + 1, s =>
    let o : Outcome := match step P s with
      | .done o => o
      | .callNative s' f args views => resumeNative P s' f args (natives f views)
    (if FunctionValues.stateOk P s then [] else ["D5 state"]) ++
      (if FunctionValues.outcomeOk P o then [] else ["D5 outcome"]) ++
      (if FunctionValues.callEntersOk P s o then [] else ["D5c"]) ++
      match o with
      | .running s' => d5From P natives fuel s'
      | _ => []

def d5Violations (P : Program) (module index : Nat) (args : List Value)
    (storage : GlobalStorage := []) (held : List Value := []) : List String :=
  match initialState P (fn module index) args storage held with
  | some s => (d5From P noNatives 10000 s).eraseDups
  | none => ["no initial state"]

#guard closCases.all fun (f, args, st, held) => d5Violations (closP f) 0 14 args st held == []
-- What D5 rejects: a closure capturing a reference, one claiming `store` over
-- a private function, and a non-storable closure in global storage.
#guard !FunctionValues.closureOk (closP genTest)
  (.closure (fn 0 6) 1#64 [.ref (refU64 ⟨.held 0, []⟩)] [tU64] [tU64] copyDrop)
#guard !FunctionValues.closureOk (closP genTest)
  (.closure (fn 0 1) 1#64 [u64 10] [tU64] [tU64] copyDropStore)
#guard FunctionValues.closureOk (closP genTest) (subX 10)
#guard !FunctionValues.storedOk (closP genTest)
  [holderOf (.closure (fn 0 1) 1#64 [u64 10] [tU64] [tU64] copyDrop)]
#guard FunctionValues.storedOk (closP genTest) [holderOf (subX 10)]

/-! ## Natives with references (language-owner rulings, 2026-11-03)

Mock modules of the standard library declare the modeled natives as
`move-stdlib/sources` declares them (`Spec.Semantics.ModeledNatives`), with
test functions that call them. -/

/-- Run function `index` of module `module` with `natives`, the transaction
holding `held`. -/
def runNat (P : Program) (natives : NativeImpl) (module index : Nat) (args : List Value)
    (held : List Value := []) (typeArgs : List RuntimeType := []) : RunResult :=
  match initialState P (fn module index typeArgs) args [] held with
  | some s => run P natives 10000 s
  | none => .wrong (.unsupported "no initial state")

/-- The D4 checks that fail on a run with `natives` and flows `ν`. -/
def refViolationsWith (P : Program) (natives : NativeImpl) (ν : NativeFlows) (module index : Nat)
    (args : List Value) (held : List Value := []) (typeArgs : List RuntimeType := []) :
    List String :=
  match initialState P (fn module index typeArgs) args [] held with
  | some s =>
    let (h, evs) := Shadow.initial s
    (refViolationsFrom P natives ν 10000 s h evs).eraseDups
  | none => ["no initial state"]

/-- A token is a value type naming no function type (the old native clause of
`CompiledModule.Supported`, on the tokens: that they translate at every instance
is the other clauses' business). -/
def isValueToken (t : SignatureToken) : Bool :=
  !t.mentionsFunction && match t with
    | .reference _ | .mutableReference _ => false
    | _ => true

/-- The native clause of `CompiledModule.Supported`, decided on the tokens:
every native definition of `m` is modeled, or declares values only and no
function type. -/
def nativesInSubset (m : CompiledModule) : Bool :=
  m.functionDefs.toList.all fun d => d.code.isSome || (m.modeledNative? d).isSome ||
    match m.functionDefHandle d with
    | some fh =>
      ((m.signatures[fh.parameters]?).getD [] ++ (m.signatures[fh.returns]?).getD []).all isValueToken
    | none => false

/-! ### `std::vector`: shape natives, with no definition

`0x1::vector` declares the eight natives marked `#[bytecode_instruction]` and
`move_range`, as `vector.move` does (functions 0–8). By the ruling of
2026-11-03 they are not their instructions: `length` is a read-shape native,
`push_back`, `pop_back`, `swap` and `move_range` update-shape ones, `borrow`
and `borrow_mut` borrow-shape ones, and `empty` and `destroy_empty` take and
return values only, as before. Test functions call each at `u64` (9–18), and
two more read an element's vector while a `&mut` into the element is live,
through `length` and through `VecLen` (19, 20). Signature 63 is `[u64]`;
instantiation `k` is native `k` at `u64`. -/

def tpv : SignatureToken := .vector tp
def vU64 : SignatureToken := .vector .u64
def u64Sig : SignatureIndex := 63

def vectorNatives : List Fun :=
  [{ name := "empty", typeParameters := [{}], returns := [tpv], native := true },
   { name := "length", typeParameters := [{}], params := [.reference tpv], returns := [.u64],
     native := true },
   { name := "borrow", typeParameters := [{}], params := [.reference tpv, .u64],
     returns := [.reference tp], native := true },
   { name := "push_back", typeParameters := [{}], params := [.mutableReference tpv, tp],
     native := true },
   { name := "borrow_mut", typeParameters := [{}], params := [.mutableReference tpv, .u64],
     returns := [.mutableReference tp], native := true },
   { name := "pop_back", typeParameters := [{}], params := [.mutableReference tpv], returns := [tp],
     native := true },
   { name := "destroy_empty", typeParameters := [{}], params := [tpv], native := true },
   { name := "swap", typeParameters := [{}], params := [.mutableReference tpv, .u64, .u64],
     native := true },
   { name := "move_range", typeParameters := [{}],
     params := [.mutableReference tpv, .u64, .u64, .mutableReference tpv, .u64], native := true }]

def vectorTests : List Fun :=
  [{ name := "t_empty", returns := [vU64], code := #[.callGeneric 0, .ret] },
   { name := "t_length", params := [vU64], returns := [.u64],
     code := #[.immBorrowLoc 0, .callGeneric 1, .ret] },
   { name := "t_borrow", params := [vU64, .u64], returns := [.u64],
     code := #[.immBorrowLoc 0, .copyLoc 1, .callGeneric 2, .readRef, .ret] },
   { name := "t_push_back", params := [vU64, .u64], returns := [vU64],
     code := #[.mutBorrowLoc 0, .moveLoc 1, .callGeneric 3, .moveLoc 0, .ret] },
   -- `v[i] = e`, which the book writes `*vector::borrow_mut(&mut v, i) = x`.
   { name := "t_borrow_mut", params := [vU64, .u64, .u64], returns := [vU64],
     code := #[.copyLoc 2, .mutBorrowLoc 0, .copyLoc 1, .callGeneric 4, .writeRef, .moveLoc 0, .ret] },
   { name := "t_pop_back", params := [vU64], returns := [.u64, vU64],
     code := #[.mutBorrowLoc 0, .callGeneric 5, .moveLoc 0, .ret] },
   { name := "t_destroy_empty", params := [vU64], code := #[.moveLoc 0, .callGeneric 6, .ret] },
   { name := "t_swap", params := [vU64, .u64, .u64], returns := [vU64],
     code := #[.mutBorrowLoc 0, .copyLoc 1, .copyLoc 2, .callGeneric 7, .moveLoc 0, .ret] },
   { name := "t_move_range", params := [vU64, .u64, .u64, vU64, .u64], returns := [vU64, vU64],
     code := #[.mutBorrowLoc 0, .copyLoc 1, .copyLoc 2, .mutBorrowLoc 3, .copyLoc 4, .callGeneric 8,
       .moveLoc 0, .moveLoc 3, .ret] },
   -- `move_range` from a vector into itself.
   { name := "t_move_alias", params := [vU64],
     code := #[.mutBorrowLoc 0, .ldU64 0, .ldU64 0, .mutBorrowLoc 0, .ldU64 0, .callGeneric 8, .ret] },
   -- The length of `v` while a `&mut` to `v[0]` is still to be written through.
   { name := "t_length_under_mut", params := [vU64], returns := [vU64], locals := [.mutableReference .u64],
     code := #[.mutBorrowLoc 0, .ldU64 0, .callGeneric 4, .stLoc 1, .immBorrowLoc 0, .callGeneric 1, .pop,
       .ldU64 7, .moveLoc 1, .writeRef, .moveLoc 0, .ret] },
   { name := "t_veclen_under_mut", params := [vU64], returns := [vU64],
     locals := [.mutableReference .u64],
     code := #[.mutBorrowLoc 0, .ldU64 0, .callGeneric 4, .stLoc 1, .immBorrowLoc 0, .vecLen u64Sig,
       .pop, .ldU64 7, .moveLoc 1, .writeRef, .moveLoc 0, .ret] }]

def vectorModule : CompiledModule :=
  mkModule 0x1 "vector" (vectorNatives ++ vectorTests) (extraSignatures := [[.u64]])
    (functionInstantiations := (List.range 9).map fun k => ⟨k, u64Sig⟩)

def vectorP : Program := ⟨#[vectorModule]⟩

-- The natives with references are modeled, with their shapes; `empty` and
-- `destroy_empty` are not: they take and return values only, as before.
-- Functions with code are not natives.
#guard (vectorModule.functionDefs.toList.take 9).map vectorModule.modeledNative? ==
  [none, some .read, some .borrow, some .update, some .borrow, some .update, none, some .update,
   some .update]
#guard (vectorModule.functionDefs.toList.drop 9).all fun d => vectorModule.modeledNative? d == none
#guard nativesInSubset vectorModule

/-- "The index into the vector is out of bounds" (`vector.move`). -/
def eIndex : Nat := 0x20000

/-- `move_range`: moves `[rp, rp + len)` of `from` into `to` at `ip`, failing
out of range. -/
def moveRangeNative : NativeImpl := fun _ views =>
  match views with
  | [.vector t src, .int _ rp, .int _ len, .vector t' dst, .int _ ip] =>
    if rp.toNat + len.toNat ≤ src.length ∧ ip.toNat ≤ dst.length then
      .returned []
        [.vector t (src.take rp.toNat ++ src.drop (rp.toNat + len.toNat)),
         .vector t' (dst.take ip.toNat ++ (src.drop rp.toNat).take len.toNat ++ dst.drop ip.toNat)] []
    else .failed (.abort eIndex)
  | _ => .failed (.abort 1)

/-- `move_range` moves the elements, keeping their identities, from one vector
into the other. Each vector written back is a new node: its value differs from
the one the native saw, and a node passed on is unchanged
(`NativeCallRespects`). -/
def moveRangeFlow : NativeFlows := fun _ views _ j p =>
  match views with
  | [_, .int _ rp, .int _ len, _, .int _ ip] =>
    let rp := rp.toNat
    let len := len.toNat
    let ip := ip.toNat
    match j, p with
    | 0, [] => .fresh
    | 1, [] => .fresh
    | 0, i :: q => .arg 0 ((if i < rp then i else i + len) :: q)
    | 1, i :: q =>
      if i < ip then .arg 3 (i :: q)
      else if i < ip + len then .arg 0 ((rp + (i - ip)) :: q)
      else .arg 3 ((i - len) :: q)
    | _, _ => .fresh
  | _ => .fresh

/-- A sample relation for the vector natives that meets each shape: it does
what the docs of `vector.move` say, and aborts out of bounds. -/
def vecNative : NativeImpl := fun f views =>
  match f.index, views with
  | 0, [] =>
    match f.typeArgs with
    | [t] => .returned [.vector t []] [] []
    | _ => .failed (.abort 1)
  | 1, [.vector _ es] => .returned [u64 es.length] [] []
  | 2, [.vector _ es, .int _ i] | 4, [.vector _ es, .int _ i] =>
    if i.toNat < es.length then .returned [] [] [⟨0, [.element i.toNat]⟩] else .failed (.abort eIndex)
  | 3, [.vector t es, e] => .returned [] [.vector t (es ++ [e])] []
  | 5, [.vector t es] =>
    match es.getLast? with
    | some last => .returned [last] [.vector t es.dropLast] []
    | none => .failed (.abort eIndex)
  | 6, [.vector _ es] => if es.isEmpty then .returned [] [] [] else .failed (.abort eIndex)
  | 7, [.vector t es, .int _ i, .int _ j] =>
    match es[i.toNat]?, es[j.toNat]? with
    | some x, some y => .returned [] [.vector t ((es.set i.toNat y).set j.toNat x)] []
    | _, _ => .failed (.abort eIndex)
  | 8, _ => moveRangeNative f views
  | _, _ => .failed (.abort 1)

/-- Its flows: `push_back` keeps the vector's elements and moves the pushed
value in; `pop_back` moves the last element out and keeps the rest; `swap`
exchanges two elements; `move_range` as above. The vector written back is a
new node, as for `move_range`; results of `length` and `empty` are new. -/
def vecFlow : NativeFlows := fun f views outs j p =>
  match f.index, views with
  | 3, [.vector _ es, _] =>
    match p with
    | [] => .fresh
    | i :: q => if i < es.length then .arg 0 (i :: q) else .arg 1 q
  | 5, [.vector _ es] =>
    if j == 0 then .arg 0 ((es.length - 1) :: p)
    else
      match p with
      | [] => .fresh
      | i :: q => .arg 0 (i :: q)
  | 7, [_, .int _ i, .int _ k] =>
    match p with
    | [] => .fresh
    | n :: q => .arg 0 ((if n == i.toNat then k.toNat else if n == k.toNat then i.toNat else n) :: q)
  | 8, _ => moveRangeFlow f views outs j p
  | _, _ => .fresh

def moveRangeArgs : List Value := [vecU64 [1, 2, 3, 4], u64 1, u64 2, vecU64 [9], u64 1]

/-- Test function `9 + k`, its arguments, and its outcome under `vecNative`. -/
def vectorCases : List (Nat × List Value × RunResult) :=
  [(0, [], .returned [vecU64 []]),
   (1, [vecU64 [1, 2, 3]], .returned [u64 3]),
   (2, [vecU64 [1, 2, 3], u64 1], .returned [u64 2]),
   (2, [vecU64 [1, 2, 3], u64 3], .failed (.abort eIndex)),
   (3, [vecU64 [1, 2, 3], u64 4], .returned [vecU64 [1, 2, 3, 4]]),
   (4, [vecU64 [1, 2, 3], u64 0, u64 9], .returned [vecU64 [9, 2, 3]]),
   (4, [vecU64 [1], u64 1, u64 9], .failed (.abort eIndex)),
   (5, [vecU64 [1, 2, 3]], .returned [u64 3, vecU64 [1, 2]]),
   (5, [vecU64 []], .failed (.abort eIndex)),
   (6, [vecU64 []], .returned []),
   (6, [vecU64 [1]], .failed (.abort eIndex)),
   (7, [vecU64 [1, 2, 3], u64 0, u64 2], .returned [vecU64 [3, 2, 1]]),
   (7, [vecU64 [1, 2, 3], u64 2, u64 3], .failed (.abort eIndex)),
   (8, [vecU64 [1, 2, 3, 4], u64 1, u64 2, vecU64 [9], u64 1], .returned [vecU64 [1, 4], vecU64 [9, 2, 3]]),
   (8, [vecU64 [1], u64 1, u64 2, vecU64 [], u64 0], .failed (.abort eIndex))]

-- Each call works under the sample relation: `push_back`, `borrow`,
-- `borrow_mut` with a write through the result, `pop_back`, `swap`, `length`,
-- `move_range`, and the out-of-bounds failures; the ghost layer accounts for
-- every step, with no D2, D3 or D4 violation.
#guard vectorCases.all fun (k, args, r) => runNat vectorP vecNative 0 (9 + k) args == r
#guard vectorCases.all fun (k, args, _) =>
  agrees vectorP 0 (9 + k) args vecNative vecFlow &&
    violations vectorP 0 (9 + k) args vecNative vecFlow == [] &&
    refViolationsWith vectorP vecNative vecFlow 0 (9 + k) args == []
-- The semantics assumes nothing more: a relation that only fails meets the
-- shapes too, and every call fails.
#guard vectorCases.all fun (k, args, _) =>
  (runFun vectorP 0 (9 + k) args == .failed (.abort 0)) && agrees vectorP 0 (9 + k) args &&
    violations vectorP 0 (9 + k) args == [] && refViolations vectorP 0 (9 + k) args == []

-- Identities: `borrow` reads element 1 (identity 2) through the new reference;
-- `push_back` moves the pushed value (4) in; `pop_back` moves the last element
-- (3) out; `swap` exchanges elements 1 and 3; `move_range` moves elements 2
-- and 3 from one vector into the other. Each vector written back is a new
-- node; its elements keep their identities.
#guard returnedIds (traceFun vectorP 0 11 [vecU64 [1, 2, 3], u64 1] vecNative vecFlow) == some [2]
#guard returnedIds (traceFun vectorP 0 12 [vecU64 [1, 2, 3], u64 4] vecNative vecFlow) ==
  some [6, 1, 2, 3, 4]
#guard returnedIds (traceFun vectorP 0 14 [vecU64 [1, 2, 3]] vecNative vecFlow) == some [3, 5, 1, 2]
#guard returnedIds (traceFun vectorP 0 16 [vecU64 [1, 2, 3], u64 0, u64 2] vecNative vecFlow) ==
  some [7, 3, 2, 1]
#guard returnedIds (traceFun vectorP 0 17
  [vecU64 [1, 2, 3, 4], u64 1, u64 2, vecU64 [9], u64 1] vecNative vecFlow) ==
  some [12, 1, 4, 13, 8, 2, 3]
-- `borrow` reads the vector through its reference, uses it, and makes a new
-- reference derived from it.
#guard (traceFun vectorP 0 11 [vecU64 [1, 2, 3], u64 1] vecNative vecFlow).events.contains
  (.accessed .read ⟨.local_ 0 0, []⟩ (some 5))
#guard ((traceFun vectorP 0 11 [vecU64 [1, 2, 3], u64 1] vecNative vecFlow).events.filterMap
  Event.borrowed?).map (fun (i, _, p) => (i, p)) == [(5, none), (6, some 5)]
-- `move_range` reads and writes both vectors through its references, and
-- consumes the two vector nodes it replaced (0, 7) and its `u64` arguments.
#guard (traceFun vectorP 0 17 [vecU64 [1, 2, 3, 4], u64 1, u64 2, vecU64 [9], u64 1] vecNative
    vecFlow).events.filter (fun e => match e with
      | .accessed _ _ (some _) => true
      | _ => false) ==
  [.accessed .read ⟨.local_ 0 0, []⟩ (some 10), .accessed .read ⟨.local_ 0 3, []⟩ (some 11),
   .accessed .write ⟨.local_ 0 0, []⟩ (some 10), .accessed .write ⟨.local_ 0 3, []⟩ (some 11)]
#guard ((traceFun vectorP 0 17 [vecU64 [1, 2, 3, 4], u64 1, u64 2, vecU64 [9], u64 1] vecNative
    vecFlow).ownershipEvents.filterMap fun e => match e with
      | .consumed i _ _ => some i
      | _ => none) == [0, 5, 6, 7, 9]
-- `borrow_mut` writes nothing back: the vector stays where it is, and the
-- native consumes only its index (identity 4).
#guard ((traceFun vectorP 0 13 [vecU64 [1, 2, 3], u64 0, u64 9] vecNative vecFlow).ownershipEvents.filterMap
    fun e => match e with
      | .consumed i _ _ => some i
      | _ => none) == [4]
#guard ((traceFun vectorP 0 13 [vecU64 [1, 2, 3], u64 0, u64 9] vecNative vecFlow).events.filter
    fun e => match e with
      | .accessed .write l (some _) => l.path.isEmpty
      | _ => false) == []
-- `move_range` from a vector into itself: its two `&mut` arguments overlap.
#guard runNat vectorP vecNative 0 18 [vecU64 [1]] == .wrong .overlappingReferences
-- A native sees the whole vector: `length` while a `&mut` into an element is
-- still to be used reaches over it (D4b′), where `VecLen` only inspects the
-- length. Both run (D1 checks only validity).
#guard runNat vectorP vecNative 0 19 [vecU64 [1, 2]] == .returned [vecU64 [7, 2]]
#guard runNat vectorP vecNative 0 20 [vecU64 [1, 2]] == .returned [vecU64 [7, 2]]
#guard refViolationsWith vectorP vecNative vecFlow 0 19 [vecU64 [1, 2]] == ["D4b′"]
#guard refViolationsWith vectorP vecNative vecFlow 0 20 [vecU64 [1, 2]] == []

/-- Native `k` of `vectorP` at `u64`, seeing `views`, returns under `vecNative`,
and its flow meets the ownership contract (`NativeCallRespects`): every node
it passes on is unchanged, and it consumes and creates only as it may. -/
def vecRespects (k : Nat) (views : List Value) : Bool :=
  let f := fn 0 k [tU64]
  match vecNative f views with
  | .returned vals writes _ =>
    NativeCallRespects vectorP {} f ⟨0x1, "vector"⟩ views (vals ++ writes)
      (vecFlow f views (vals ++ writes))
  | .failed _ => false

-- Every native of the cases above, as it is called there.
#guard [(0, []), (1, [vecU64 [1, 2, 3]]), (2, [vecU64 [1, 2, 3], u64 1]),
  (3, [vecU64 [1, 2, 3], u64 4]), (4, [vecU64 [1, 2, 3], u64 0]), (5, [vecU64 [1, 2, 3]]),
  (6, [vecU64 []]), (7, [vecU64 [1, 2, 3], u64 0, u64 2]), (8, moveRangeArgs)].all
  fun (k, views) => vecRespects k views

/-- `vecNative`, with native `k`'s result replaced. -/
def vecWith (k : Nat) (r : List Value → NativeResult) : NativeImpl := fun f views =>
  if f.index == k then r views else vecNative f views

-- Outside the shapes: `borrow` returning a place that does not exist, a place
-- of the wrong type (the vector, not an element), or a place in its `u64`
-- argument; `borrow_mut` returning a value besides its place; `push_back`
-- writing back a vector of another type, or returning a place; `pop_back`
-- returning a value of another type; `length` writing back.
#guard runNat vectorP (vecWith 2 fun _ => .returned [] [] [⟨0, [.element 7]⟩]) 0 11
  [vecU64 [1, 2, 3], u64 1] == .wrong .nativeContract
#guard runNat vectorP (vecWith 2 fun _ => .returned [] [] [⟨0, []⟩]) 0 11
  [vecU64 [1, 2, 3], u64 1] == .wrong .nativeContract
#guard runNat vectorP (vecWith 2 fun _ => .returned [] [] [⟨1, []⟩]) 0 11
  [vecU64 [1, 2, 3], u64 1] == .wrong .nativeContract
#guard runNat vectorP (vecWith 4 fun _ => .returned [u64 0] [] [⟨0, [.element 0]⟩]) 0 13
  [vecU64 [1, 2, 3], u64 0, u64 9] == .wrong .nativeContract
#guard runNat vectorP (vecWith 3 fun _ => .returned [] [.vector .bool []] []) 0 12
  [vecU64 [1, 2, 3], u64 4] == .wrong .nativeContract
#guard runNat vectorP (vecWith 3 fun views => .returned [] views.reverse.tail [⟨0, []⟩]) 0 12
  [vecU64 [1, 2, 3], u64 4] == .wrong .nativeContract
#guard runNat vectorP (vecWith 5 fun views => .returned [.bool true] views []) 0 14
  [vecU64 [1, 2, 3]] == .wrong .nativeContract
#guard runNat vectorP (vecWith 1 fun views => .returned [u64 3] views []) 0 10 [vecU64 [1, 2, 3]] ==
  .wrong .nativeContract

-- `empty` and `destroy_empty` are value natives as before: any conforming
-- result is theirs (nothing makes `empty` empty), and anything else, a value
-- of another type or a written-back value, is outside the contract.
#guard runNat vectorP (vecWith 0 fun _ => .returned [vecU64 [5]] [] []) 0 9 [] == .returned [vecU64 [5]]
#guard runNat vectorP (vecWith 0 fun _ => .returned [.vector .bool []] [] []) 0 9 [] ==
  .wrong .nativeContract
#guard runNat vectorP (vecWith 6 fun _ => .returned [] [vecU64 []] []) 0 15 [vecU64 []] ==
  .wrong .nativeContract
#guard runNat vectorP (vecWith 6 fun _ => .returned [] [] []) 0 15 [vecU64 [1]] == .returned []

/-! ### `vector::move_range` (update shape)

The sample relation alone, with its flow. -/

#guard runNat vectorP moveRangeNative 0 17 moveRangeArgs == .returned [vecU64 [1, 4], vecU64 [9, 2, 3]]
#guard agrees vectorP 0 17 moveRangeArgs moveRangeNative moveRangeFlow
#guard violations vectorP 0 17 moveRangeArgs moveRangeNative moveRangeFlow == []
#guard refViolationsWith vectorP moveRangeNative moveRangeFlow 0 17 moveRangeArgs == []

/-! ### `signer::borrow_address` (borrow shape)

The native returns the place of the address inside the signer its argument
designates; the semantics makes the reference. -/

def signerModule : CompiledModule :=
  mkModule 0x1 "signer"
    [{ name := "borrow_address", params := [.reference .signer], returns := [.reference .address],
       native := true },
     { name := "address_of", params := [.reference .signer], returns := [.address],
       code := #[.moveLoc 0, .call 0, .readRef, .ret] }]

def signerP : Program := ⟨#[signerModule]⟩

def borrowAddressWith (place : NativePlace) (vals writes : List Value := []) : NativeImpl :=
  fun _ _ => .returned vals writes [place]

def borrowAddress : NativeImpl := borrowAddressWith ⟨0, [.signerAddress]⟩

#guard signerModule.modeledNative? signerModule.functionDefs[0]! == some .borrow
#guard nativesInSubset signerModule
#guard runNat signerP borrowAddress 0 1 [signerRef 0] [signerAt 7] == .returned [.address 7]
#guard agrees signerP 0 1 [signerRef 0] borrowAddress (held := [signerAt 7])
#guard violations signerP 0 1 [signerRef 0] borrowAddress (auth := [7]) (held := [signerAt 7]) == []
#guard refViolationsWith signerP borrowAddress allFresh 0 1 [signerRef 0] [signerAt 7] == []
-- The new reference derives from the `&signer` argument (identity 0), and
-- reading through it reads the address node inside the held signer (the
-- signer is 1, its address 2).
#guard ((traceFun signerP 0 1 [signerRef 0] borrowAddress (held := [signerAt 7])).events.filterMap
    Event.borrowed?).map (fun (i, r, p) => (i, r.location, p)) ==
  [(0, ⟨.held 0, []⟩, none), (3, ⟨.held 0, [.signerAddress]⟩, some 0)]
#guard returnedIds (traceFun signerP 0 1 [signerRef 0] borrowAddress (held := [signerAt 7])) ==
  some [2]
-- The address in a signer is read, never replaced.
#guard (signerAt 7).get? [.signerAddress] == some (.address 7)
#guard (signerAt 7).set? [.signerAddress] (.address 8) == none
-- Outside the shape: a place that does not exist, the signer itself (a
-- `signer`, not an `address`), an argument that does not exist, a written-back
-- value, an extra value, and no place.
#guard runNat signerP (borrowAddressWith ⟨0, [.field 0]⟩) 0 1 [signerRef 0] [signerAt 7] ==
  .wrong .nativeContract
#guard runNat signerP (borrowAddressWith ⟨0, []⟩) 0 1 [signerRef 0] [signerAt 7] ==
  .wrong .nativeContract
#guard runNat signerP (borrowAddressWith ⟨1, [.signerAddress]⟩) 0 1 [signerRef 0] [signerAt 7] ==
  .wrong .nativeContract
#guard runNat signerP (borrowAddressWith ⟨0, [.signerAddress]⟩ (writes := [.address 7])) 0 1
  [signerRef 0] [signerAt 7] == .wrong .nativeContract
#guard runNat signerP (borrowAddressWith ⟨0, [.signerAddress]⟩ (vals := [.address 7])) 0 1
  [signerRef 0] [signerAt 7] == .wrong .nativeContract
#guard runNat signerP (fun _ _ => .returned [] [] []) 0 1 [signerRef 0] [signerAt 7] ==
  .wrong .nativeContract
-- A mutable result needs a mutable argument (no `borrow_address` returns one,
-- so the rule is checked directly); an immutable result may come from either.
#guard nativePlaceFits signerP [.ref false .signer] [signerAt 7] (.ref false .address)
  ⟨0, [.signerAddress]⟩
#guard !nativePlaceFits signerP [.ref false .signer] [signerAt 7] (.ref true .address)
  ⟨0, [.signerAddress]⟩
#guard nativePlaceFits signerP [.ref true .signer] [signerAt 7] (.ref true .address)
  ⟨0, [.signerAddress]⟩
#guard nativePlaceFits signerP [.ref true .signer] [signerAt 7] (.ref false .address)
  ⟨0, [.signerAddress]⟩
-- A place's declared type must be the result's target, and its argument a
-- reference.
#guard !nativePlaceFits signerP [.ref false .signer] [signerAt 7] (.ref false .signer)
  ⟨0, [.signerAddress]⟩
#guard !nativePlaceFits signerP [.int .u64] [u64 7] (.ref false (.int .u64)) ⟨0, []⟩

/-! ### `mem::swap` (update shape)

`0x2::user` defines `R { u64 }`, with no abilities, and calls `mem::swap`
(handle 3) at `u64` (instantiation 0) and at `R` (instantiation 1). A sample
relation exchanges the two values, and its flow exchanges their identities. -/

/-- `std::mem` as `mem.move` declares it: the native `swap`, and `replace<T>(ref:
&mut T, new: T): T { swap(ref, &mut new); new }`, which calls it at `T`
(signature 6). -/
def memModule : CompiledModule :=
  mkModule 0x1 "mem"
    [{ name := "swap", typeParameters := [{}], params := [.mutableReference tp, .mutableReference tp],
       native := true },
     { name := "replace", typeParameters := [{}], params := [.mutableReference tp, tp],
       returns := [tp], code := #[.moveLoc 0, .mutBorrowLoc 1, .callGeneric 0, .moveLoc 1, .ret] }]
    (extraSignatures := [[tp]]) (functionInstantiations := [⟨0, 6⟩])

def userModule : CompiledModule :=
  mkModule 0x2 "user"
    [{ name := "swap_u64", params := [.u64, .u64], returns := [.u64, .u64],
       code := #[.mutBorrowLoc 0, .mutBorrowLoc 1, .callGeneric 0, .moveLoc 0, .moveLoc 1, .ret] },
     { name := "swap_r", returns := [.struct 0, .struct 0], locals := [.struct 0, .struct 0],
       code := #[.ldU64 1, .pack 0, .stLoc 0, .ldU64 2, .pack 0, .stLoc 1, .mutBorrowLoc 0,
         .mutBorrowLoc 1, .callGeneric 1, .moveLoc 0, .moveLoc 1, .ret] },
     { name := "swap_alias", params := [.u64],
       code := #[.mutBorrowLoc 0, .mutBorrowLoc 0, .callGeneric 0, .ret] }]
    [{ address := 0x1, module := "mem", name := "swap", typeParameters := [{}],
       params := [.mutableReference tp, .mutableReference tp] }]
    [] [{ name := "R", fields := [.u64] }]
    (extraSignatures := [[.u64], [.struct 0]]) (functionInstantiations := [⟨3, 11⟩, ⟨3, 12⟩])

def memP : Program := ⟨#[memModule, userModule]⟩

def typeUserR : StructId := ⟨⟨0x2, "user"⟩, "R", []⟩
def valR (n : Nat) : Value := .struct typeUserR [u64 n]

def swapNative : NativeImpl := fun _ views =>
  match views with
  | [a, b] => .returned [] [b, a] []
  | _ => .failed (.abort 1)

/-- The new left value is the old right one, and the other way round. -/
def swapFlow : NativeFlows := fun _ _ _ j p => .arg (1 - j) p

#guard memModule.modeledNative? memModule.functionDefs[0]! == some .update
#guard nativesInSubset memModule
#guard runNat memP swapNative 1 0 [u64 1, u64 2] == .returned [u64 2, u64 1]
#guard runNat memP swapNative 1 1 [] == .returned [valR 2, valR 1]
#guard agrees memP 1 0 [u64 1, u64 2] swapNative swapFlow
#guard agrees memP 1 1 [] swapNative swapFlow
#guard violations memP 1 1 [] swapNative swapFlow == []
#guard refViolationsWith memP swapNative swapFlow 1 1 [] == []
-- The resources change places with their identities (`R{1}` is 1 with field
-- 0, `R{2}` is 3 with field 2); the native creates and consumes nothing.
#guard returnedIds (traceFun memP 1 1 [] swapNative swapFlow) == some [3, 2, 1, 0]
#guard (traceFun memP 1 1 [] swapNative swapFlow).ownershipEvents.all fun e => match e with
  | .created _ _ (.instruction _ _) => true
  | _ => false
#guard NativeCallRespects memP {} (fn 0 0 [.struct typeUserR]) ⟨0x1, "mem"⟩ [valR 1, valR 2]
  [valR 2, valR 1] (swapFlow (fn 0 0 [.struct typeUserR]) [valR 1, valR 2] [valR 2, valR 1])
-- `mem::replace<u64>` through a `&mut u64` to a value the transaction holds:
-- the held value becomes the new one, and the old one is returned.
def heldU64Ref : Value := .ref ⟨true, .int .u64, ⟨.held 0, []⟩, 0⟩

#guard runNat memP swapNative 0 1 [heldU64Ref, u64 5] [u64 1] [tU64] == .returned [u64 1]
#guard agrees memP 0 1 [heldU64Ref, u64 5] swapNative swapFlow (held := [u64 1]) (typeArgs := [tU64])
#guard violations memP 0 1 [heldU64Ref, u64 5] swapNative swapFlow (held := [u64 1])
  (typeArgs := [tU64]) == []
#guard refViolationsWith memP swapNative swapFlow 0 1 [heldU64Ref, u64 5] [u64 1] [tU64] == []
-- Two mutable arguments that overlap make the call wrong.
#guard runNat memP swapNative 1 2 [u64 1] == .wrong .overlappingReferences
-- Outside the shape: a written-back value of another type, one too few, a
-- place, a value result.
#guard runNat memP (fun _ views => .returned [] (views.reverse.take 1 ++ [.bool true]) []) 1 0
  [u64 1, u64 2] == .wrong .nativeContract
#guard runNat memP (fun _ views => .returned [] (views.reverse.take 1) []) 1 0 [u64 1, u64 2] ==
  .wrong .nativeContract
#guard runNat memP (fun _ views => .returned [] views.reverse [⟨0, []⟩]) 1 0 [u64 1, u64 2] ==
  .wrong .nativeContract
#guard runNat memP (fun _ views => .returned [u64 0] views.reverse []) 1 0 [u64 1, u64 2] ==
  .wrong .nativeContract

/-! ### What the contracts reject

A swap that writes the left value into both places duplicates a resource
(D2a) and destroys the other (D2b), and breaks its contract. -/

def dupSwap : NativeImpl := fun _ views =>
  match views with
  | [a, _] => .returned [] [a, a] []
  | _ => .failed (.abort 1)

def dupFlow : NativeFlows := fun _ _ _ _ p => .arg 0 p

#guard agrees memP 1 1 [] dupSwap dupFlow
#guard (violations memP 1 1 [] dupSwap dupFlow).contains "D2a"
#guard (violations memP 1 1 [] dupSwap dupFlow).contains "D2b"
#guard !NativeCallRespects memP {} (fn 0 0 [.struct typeUserR]) ⟨0x1, "mem"⟩ [valR 1, valR 2]
  [valR 1, valR 1] (dupFlow (fn 0 0 [.struct typeUserR]) [valR 1, valR 2] [valR 1, valR 1])

/-! ### `cmp::compare` and the other read-shape natives

`0x1::cmp` declares `enum Ordering has copy, drop { Less, Equal, Greater }` and
`compare<T>(first: &T, second: &T): Ordering`; `0x2::client` defines
`C has drop { u64 }` and compares two of its values. A sample relation
compares `u64`s and `C`s. -/

def cmpModule : CompiledModule :=
  mkModule 0x1 "cmp"
    [{ name := "compare", typeParameters := [{}], params := [.reference tp, .reference tp],
       returns := [.struct 0], native := true },
     { name := "cmp_u64", params := [.u64, .u64], returns := [.struct 0],
       code := #[.immBorrowLoc 0, .immBorrowLoc 1, .callGeneric 0, .ret] },
     { name := "cmp_self", params := [.u64], returns := [.struct 0],
       code := #[.immBorrowLoc 0, .immBorrowLoc 0, .callGeneric 0, .ret] },
     -- Both references dangle when the native is called: the local was moved out.
     { name := "cmp_dangling", params := [.u64], returns := [.struct 0],
       code := #[.immBorrowLoc 0, .immBorrowLoc 0, .moveLoc 0, .pop, .callGeneric 0, .ret] }]
    [] [] [{ name := "Ordering", abilities := ⟨true, true, false, false⟩,
             variants := some [("Less", []), ("Equal", []), ("Greater", [])] }]
    (extraSignatures := [[.u64]]) (functionInstantiations := [⟨0, 12⟩])

def clientModule : CompiledModule :=
  mkModule 0x2 "client"
    [{ name := "cmp_c", returns := [.struct 1], locals := [.struct 0, .struct 0],
       code := #[.ldU64 1, .pack 0, .stLoc 0, .ldU64 2, .pack 0, .stLoc 1, .immBorrowLoc 0,
         .immBorrowLoc 1, .callGeneric 0, .ret] }]
    [{ address := 0x1, module := "cmp", name := "compare", typeParameters := [{}],
       params := [.reference tp, .reference tp], returns := [.struct 1] }]
    [] [{ name := "C", abilities := { drop := true }, fields := [.u64] }]
    [{ address := 0x1, module := "cmp", name := "Ordering", abilities := ⟨true, true, false, false⟩ }]
    (extraSignatures := [[.struct 0]]) (functionInstantiations := [⟨1, 5⟩])

def cmpP : Program := ⟨#[cmpModule, clientModule]⟩

def typeOrdering : StructId := ⟨⟨0x1, "cmp"⟩, "Ordering", []⟩

def ordering (a b : Int) : Value :=
  .variant typeOrdering (if a < b then 0 else if a = b then 1 else 2) []

def compareNative : NativeImpl := fun _ views =>
  match views with
  | [.int _ a, .int _ b] => .returned [ordering a b] [] []
  | [.struct _ [.int _ a], .struct _ [.int _ b]] => .returned [ordering a b] [] []
  | _ => .failed (.abort 1)

#guard cmpModule.modeledNative? cmpModule.functionDefs[0]! == some .read
#guard nativesInSubset cmpModule
#guard runNat cmpP compareNative 0 1 [u64 1, u64 2] == .returned [.variant typeOrdering 0 []]
-- Two immutable references to one local may be passed together; dangling ones
-- may not.
#guard runNat cmpP compareNative 0 2 [u64 5] == .returned [.variant typeOrdering 1 []]
#guard runNat cmpP compareNative 0 3 [u64 5] == .wrong .invalidReference
#guard runNat cmpP compareNative 1 0 [] == .returned [.variant typeOrdering 0 []]
#guard agrees cmpP 0 1 [u64 1, u64 2] compareNative
#guard agrees cmpP 1 0 [] compareNative
#guard violations cmpP 0 1 [u64 1, u64 2] compareNative == []
#guard refViolationsWith cmpP compareNative allFresh 0 1 [u64 1, u64 2] == []
#guard refViolationsWith cmpP compareNative allFresh 0 2 [u64 5] == []
-- Comparing values of another module's struct is no D3 violation: the native
-- reads them and creates an `Ordering`, a value of its own module's type.
#guard violations cmpP 1 0 [] compareNative == []
#guard refViolationsWith cmpP compareNative allFresh 1 0 [] == []
#guard ((traceFun cmpP 1 0 [] compareNative).events.filter fun e => match e with
    | .privileged .. => true
    | _ => false) == []
-- The native reads both locations and uses both references, and the
-- arguments stay where they are: nothing is consumed.
#guard (traceFun cmpP 0 1 [u64 1, u64 2] compareNative).events.contains
  (.accessed .read ⟨.local_ 0 0, []⟩ (some 2))
#guard (traceFun cmpP 0 1 [u64 1, u64 2] compareNative).events.contains
  (.accessed .read ⟨.local_ 0 1, []⟩ (some 3))
#guard (traceFun cmpP 1 0 [] compareNative).ownershipEvents.all fun e => match e with
  | .consumed .. => false
  | _ => true
-- Outside the shape: a written-back value, a place, a result that does not
-- conform (no variant 3).
#guard runNat cmpP (fun _ _ => .returned [.variant typeOrdering 1 []] [u64 0] []) 0 1
  [u64 1, u64 2] == .wrong .nativeContract
#guard runNat cmpP (fun _ _ => .returned [.variant typeOrdering 1 []] [] [⟨0, []⟩]) 0 1
  [u64 1, u64 2] == .wrong .nativeContract
#guard runNat cmpP (fun _ _ => .returned [.variant typeOrdering 3 []] [] []) 0 1 [u64 1, u64 2] ==
  .wrong .nativeContract
-- Passing on a node of what an immutable reference argument designates copies
-- it: allowed for a `u64`, not for a `C`, which lacks `copy`. Nothing of it is
-- consumed, so no consumption needs permission.
def typeC : StructId := ⟨⟨0x2, "client"⟩, "C", []⟩
def valC (n : Nat) : Value := .struct typeC [u64 n]

#guard NativeCallRespects cmpP {} (fn 0 0 [.int .u64]) ⟨0x1, "cmp"⟩ [u64 1, u64 2] [u64 1]
  (fun _ p => .arg 0 p)
#guard !NativeCallRespects cmpP {} (fn 0 0 [.struct typeC]) ⟨0x1, "cmp"⟩ [valC 1, valC 2] [valC 1]
  (fun _ p => .arg 0 p)
#guard NativeCallRespects cmpP {} (fn 0 0 [.struct typeC]) ⟨0x1, "cmp"⟩ [valC 1, valC 2]
  [ordering 1 2] (fun _ _ => .fresh)
-- What stays in place: what a reference designates, unless an update-shape
-- native writes it back.
#guard cmpP.nativeKeeps (fn 0 0 [tU64]) (.ref false tU64)
#guard !memP.nativeKeeps (fn 0 0 [tU64]) (.ref true tU64)
#guard signerP.nativeKeeps (fn 0 0) (.ref true .signer)
#guard !cmpP.nativeKeeps (fn 0 0 [tU64]) tU64

-- The natives of `std::string` and `std::bcs`, declared as their sources do.
def stringModule : CompiledModule :=
  mkModule 0x1 "string"
    [{ name := "internal_check_utf8", params := [.reference (.vector .u8)], returns := [.bool],
       native := true },
     { name := "internal_is_char_boundary", params := [.reference (.vector .u8), .u64],
       returns := [.bool], native := true, visibility := .private_ },
     { name := "internal_sub_string", params := [.reference (.vector .u8), .u64, .u64],
       returns := [.vector .u8], native := true, visibility := .private_ },
     { name := "internal_index_of", params := [.reference (.vector .u8), .reference (.vector .u8)],
       returns := [.u64], native := true, visibility := .private_ },
     { name := "check", params := [.vector .u8], returns := [.bool],
       code := #[.immBorrowLoc 0, .call 0, .ret] }]

def bcsModule : CompiledModule :=
  mkModule 0x1 "bcs"
    [{ name := "to_bytes", typeParameters := [{}], params := [.reference tp], returns := [.vector .u8],
       native := true },
     { name := "serialized_size", typeParameters := [{}], params := [.reference tp],
       returns := [.u64], native := true }]

#guard (stringModule.functionDefs.toList.take 4).map stringModule.modeledNative? ==
  [some .read, some .read, some .read, some .read]
#guard bcsModule.functionDefs.toList.map bcsModule.modeledNative? == [some .read, some .read]
#guard nativesInSubset stringModule && nativesInSubset bcsModule
#guard runNat ⟨#[stringModule]⟩ (fun _ _ => .returned [.bool true] [] []) 0 4 [.vector (.int .u8) [u8c 0x61]] ==
  .returned [.bool true]

/-! ### Natives that are not modeled stay outside the subset

A native matching a modeled one by name but not by signature is not modeled:
`vector::borrow` declared at `u64` only, or with a constraint, or in a module
of another address; `vector::borrow_mut` taking `&vector`; `mem::swap` taking
`&` references. Nor is a `table::borrow_box` declared otherwise than
`table.move` declares it (non-generic, at `u64`), whose call is still
unsupported. -/

def notModeled : List CompiledModule :=
  [mkModule 0x1 "vector"
    [{ name := "borrow", params := [.reference vU64, .u64], returns := [.reference .u64],
       native := true }],
   mkModule 0x1 "vector"
    [{ name := "borrow", typeParameters := [⟨true, false, false, false⟩],
       params := [.reference tpv, .u64], returns := [.reference tp], native := true }],
   mkModule 0x2 "vector"
    [{ name := "borrow", typeParameters := [{}], params := [.reference tpv, .u64],
       returns := [.reference tp], native := true }],
   mkModule 0x1 "vector"
    [{ name := "borrow_mut", typeParameters := [{}], params := [.reference tpv, .u64],
       returns := [.mutableReference tp], native := true }],
   mkModule 0x1 "mem"
    [{ name := "swap", typeParameters := [{}], params := [.reference tp, .reference tp],
       native := true }],
   mkModule 0x1 "cmp"
    [{ name := "compare", typeParameters := [{}], params := [.reference tp, .reference tp],
       returns := [.u64], native := true }]]

#guard notModeled.all fun m => !nativesInSubset m

def tableModule : CompiledModule :=
  mkModule 0x1 "table"
    [{ name := "borrow_box", params := [.reference (.struct 0), .u64], returns := [.reference .u64],
       native := true },
     { name := "get", params := [.struct 0], returns := [.u64],
       code := #[.immBorrowLoc 0, .ldU64 0, .call 0, .readRef, .ret] }]
    [] [] [{ name := "Table", abilities := { drop := true }, fields := [.u64] }]

#guard !nativesInSubset tableModule
#guard runNat ⟨#[tableModule]⟩ (fun _ _ => .returned [] [] [⟨0, [.field 0]⟩]) 0 1
  [.table ⟨⟨0x1, "table"⟩, "Table", []⟩ [u64 5] [] []] == .wrong (.unsupported "native with references")
/-! ### The subset, kernel-checked

`std::mem`, as `mem.move` declares it (the native `swap` and `replace`, which
calls it at a type parameter), is in the subset; a module declaring
`borrow_box` otherwise than `table.move` does is not. -/

example : memModule.Supported := by
  have hh : memModule.functionHandles.toList =
      [memModule.functionHandles[0]!, memModule.functionHandles[1]!] := by decide +kernel
  have hd : memModule.functionDefs.toList =
      [memModule.functionDefs[0]!, memModule.functionDefs[1]!] := by decide +kernel
  have hs : memModule.structDefs.toList = [] := by decide +kernel
  have hc : memModule.constantPool.toList = [] := by decide +kernel
  have hn : (memModule.functionHandles[1]!).typeParameters.length = 1 := by decide +kernel
  have hcd : memModule.functionDefs[1]!.code = some
      { locals := 5, code := #[.moveLoc 0, .mutBorrowLoc 1, .callGeneric 0, .moveLoc 1, .ret] } := by
    decide +kernel
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · rw [hh]
    intro fh hfh
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hfh
    rcases hfh with rfl | rfl
    · refine ⟨⟨[.mutableReference tp, .mutableReference tp], by decide +kernel, ?_⟩,
        ⟨[], by decide +kernel, fun _ _ => rfl⟩⟩
      intro tys htys
      have : (memModule.functionHandles[0]!).typeParameters.length = 1 := by decide +kernel
      rw [this] at htys
      match tys, htys with
      | [_], _ => rfl
    · refine ⟨⟨[.mutableReference tp, tp], by decide +kernel, ?_⟩, ⟨[tp], by decide +kernel, ?_⟩⟩
      all_goals
        intro tys htys
        rw [hn] at htys
        match tys, htys with
        | [_], _ => rfl
  · rw [hd]
    intro d hdm hcode
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hdm
    rcases hdm with rfl | rfl
    · exact Or.inl (by decide +kernel)
    · rw [hcd] at hcode
      cases hcode
  · rw [hs]
    intro d hdm
    exact absurd hdm List.not_mem_nil
  · rw [hc]
    intro c hcm
    exact absurd hcm List.not_mem_nil
  · rw [hd]
    intro d hdm code hcode
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hdm
    rcases hdm with rfl | rfl
    · have : memModule.functionDefs[0]!.code = none := by decide +kernel
      rw [this] at hcode
      cases hcode
    · rw [hcd] at hcode
      cases hcode
      refine ⟨memModule.functionHandles[1]!, by decide +kernel, ⟨[], by decide +kernel, fun _ _ => rfl⟩,
        ?_⟩
      rw [hn]
      intro i hi
      simp only [List.mem_cons, List.not_mem_nil, or_false] at hi
      rcases hi with rfl | rfl | rfl | rfl | rfl
      · exact ⟨by decide, trivial⟩
      · exact ⟨by decide, trivial⟩
      · refine ⟨by decide, ⟨⟨0, 6⟩, by decide +kernel, [tp], by decide +kernel, ?_⟩⟩
        intro tys htys
        match tys, htys with
        | [_], _ => rfl
      · exact ⟨by decide, trivial⟩
      · exact ⟨by decide, trivial⟩

example : ¬ tableModule.Supported := by
  intro ⟨_, hn, _⟩
  have hd : tableModule.functionDefs[0]! ∈ tableModule.functionDefs.toList := by decide +kernel
  rcases hn _ hd (by decide +kernel) with h | ⟨fh, hfh, ⟨sig, hsig, hall⟩, _⟩
  · exact absurd h (by decide +kernel)
  · have e1 : tableModule.functionHandles[tableModule.functionDefs[0]!.function]? =
        some tableModule.functionHandles[0]! := by decide +kernel
    rw [e1] at hfh
    cases hfh
    have e2 : tableModule.signatures[tableModule.functionHandles[0]!.parameters]? =
        some [.reference (.struct 0), .u64] := by decide +kernel
    rw [e2] at hsig
    cases hsig
    exact absurd (hall [] rfl) (by decide +kernel)

-- The vector native declared at `u64` only is a native with references, not
-- the instruction.
#guard runFun ⟨#[mkModule 0x1 "vector"
    [{ name := "borrow", params := [.reference vU64, .u64], returns := [.reference .u64], native := true },
     { name := "t", params := [vU64], returns := [.u64],
       code := #[.immBorrowLoc 0, .ldU64 0, .call 0, .readRef, .ret] }]]⟩ 0 1 [vecU64 [1]] ==
  .wrong (.unsupported "native with references")

/-! ## Table natives (language-owner rulings, 2026-11-04)

A mock `0x1::table` declares `Table<phantom K: copy + drop, phantom V> has
store { handle: address }` (struct 0) and `Box<V> has key, drop, store { val: V
}` (struct 1), and the natives as `table.move` declares them, private as there
(functions 0–7: `new_table_handle`, `add_box`, `borrow_box`, `borrow_box_mut`,
`contains_box`, `remove_box`, `destroy_empty_box`, `drop_unchecked_box`); then
the functions of `table.move` that call them, with the same code (8–15: `new`,
`add`, `borrow`, `borrow_mut`, `contains`, `remove`, `drop_unchecked`,
`destroy_known_empty_unsafe`); then test functions. `0x2::coin` defines `Coin
has store { u64 }`, a resource without `drop`, and `mint`, which the table
module imports (struct handle 2). Extra signature `k` of a table module with
`n` functions is `tblSig n k`: `[K, V]`, `[K, V, Box<V>]`, `[V]`, `[u64, u64]`,
`[u64, Coin]`, `[K, V, V]`. Function instantiation 0 is `new_table_handle<K,
V>`, 1–7 the box natives at `<K, V, Box<V>>`, 8–15 functions 8–15 at `<u64,
u64>`, 16–18 `new`, `add` and `drop_unchecked` at `<u64, Coin>`, and 19
`add_box<K, V, V>`. Struct instantiation 0 is `Table<K, V>`, 1 `Box<V>`; field
handle 0 is `Box.val`, at `Box<V>` by field instantiation 0, and field handle 1
`Table.handle`, at `Table<K, V>` by field instantiation 1. -/

def tK : SignatureToken := .typeParameter 0
def tV : SignatureToken := .typeParameter 1
def tblTok (k v : SignatureToken) : SignatureToken := .structInstantiation 0 [k, v]
def boxTok (v : SignatureToken) : SignatureToken := .structInstantiation 1 [v]
def coinTok : SignatureToken := .struct 2
def tblU64 : SignatureToken := tblTok .u64 .u64
def tblCoin : SignatureToken := tblTok .u64 coinTok

/-- The type parameters of a box native: `K: copy + drop, V, B`. -/
def boxTps : List AbilitySet := [copyDrop, {}, {}]

/-- The structs of `table.move`, `Table` declared with `abilities` and the
fields `fields`. -/
def tblStructsWith (abilities : AbilitySet) (fields : List (String × SignatureToken)) :
    List StructDecl :=
  [{ name := "Table", abilities, typeParameters := [⟨copyDrop, true⟩, ⟨{}, true⟩],
     fields := fields.map (·.2), fieldNames := fields.map (·.1) },
   { name := "Box", abilities := ⟨false, true, true, true⟩, typeParameters := [⟨{}, false⟩],
     fields := [.typeParameter 0], fieldNames := ["val"] }]

def tblStructs : List StructDecl := tblStructsWith { store := true } [("handle", .address)]

/-- The natives of `table.move`, the box natives with type parameters `tps`. -/
def tblNatives (tps : List AbilitySet := boxTps) : List Fun :=
  [{ name := "new_table_handle", typeParameters := [{}, {}], returns := [.address], native := true,
     visibility := .private_ },
   { name := "add_box", typeParameters := tps,
     params := [.mutableReference (tblTok tK tV), tK, boxTok tV], native := true,
     visibility := .private_ },
   { name := "borrow_box", typeParameters := tps, params := [.reference (tblTok tK tV), tK],
     returns := [.reference (boxTok tV)], native := true, visibility := .private_ },
   { name := "borrow_box_mut", typeParameters := tps,
     params := [.mutableReference (tblTok tK tV), tK], returns := [.mutableReference (boxTok tV)],
     native := true, visibility := .private_ },
   { name := "contains_box", typeParameters := tps, params := [.reference (tblTok tK tV), tK],
     returns := [.bool], native := true, visibility := .private_ },
   { name := "remove_box", typeParameters := tps, params := [.mutableReference (tblTok tK tV), tK],
     returns := [boxTok tV], native := true, visibility := .private_ },
   { name := "destroy_empty_box", typeParameters := tps, params := [.reference (tblTok tK tV)],
     native := true, visibility := .private_ },
   { name := "drop_unchecked_box", typeParameters := tps, params := [tblTok tK tV], native := true,
     visibility := .private_ }]

/-- The functions of `table.move` that call the natives, compiled as their
sources read. -/
def tblFuns : List Fun :=
  [{ name := "new", typeParameters := [copyDrop, { store := true }], returns := [tblTok tK tV],
     code := #[.callGeneric 0, .packGeneric 0, .ret] },
   { name := "add", typeParameters := [copyDrop, {}],
     params := [.mutableReference (tblTok tK tV), tK, tV],
     code := #[.moveLoc 0, .moveLoc 1, .moveLoc 2, .packGeneric 1, .callGeneric 1, .ret] },
   { name := "borrow", typeParameters := [copyDrop, {}], params := [.reference (tblTok tK tV), tK],
     returns := [.reference tV],
     code := #[.moveLoc 0, .moveLoc 1, .callGeneric 2, .immBorrowFieldGeneric 0, .ret] },
   { name := "borrow_mut", typeParameters := [copyDrop, {}],
     params := [.mutableReference (tblTok tK tV), tK], returns := [.mutableReference tV],
     code := #[.moveLoc 0, .moveLoc 1, .callGeneric 3, .mutBorrowFieldGeneric 0, .ret] },
   { name := "contains", typeParameters := [copyDrop, {}], params := [.reference (tblTok tK tV), tK],
     returns := [.bool], code := #[.moveLoc 0, .moveLoc 1, .callGeneric 4, .ret] },
   { name := "remove", typeParameters := [copyDrop, {}],
     params := [.mutableReference (tblTok tK tV), tK], returns := [tV],
     code := #[.moveLoc 0, .moveLoc 1, .callGeneric 5, .unpackGeneric 1, .ret] },
   { name := "drop_unchecked", typeParameters := [copyDrop, {}], params := [tblTok tK tV],
     code := #[.moveLoc 0, .callGeneric 7, .ret] },
   { name := "destroy_known_empty_unsafe", typeParameters := [copyDrop, {}],
     params := [tblTok tK tV], visibility := .friend,
     code := #[.immBorrowLoc 0, .callGeneric 6, .moveLoc 0, .callGeneric 7, .ret] }]

def tblSig (n k : Nat) : SignatureIndex := 3 * n + 2 + k

def tblExtraSigs : List Signature :=
  [[tK, tV], [tK, tV, boxTok tV], [tV], [.u64, .u64], [.u64, coinTok], [tK, tV, tV]]

def tblFunInsts (n : Nat) : List FunctionInstantiation :=
  [⟨0, tblSig n 0⟩] ++ (List.range 7).map (fun k => ⟨k + 1, tblSig n 1⟩) ++
    (List.range 8).map (fun k => ⟨k + 8, tblSig n 3⟩) ++
    [⟨8, tblSig n 4⟩, ⟨9, tblSig n 4⟩, ⟨14, tblSig n 4⟩, ⟨1, tblSig n 5⟩]

/-- A module `address::name` declaring `natives`, then `table.move`'s functions,
then `funs`, importing `coin::mint`. -/
def tblModuleWith (funs : List Fun) (natives : List Fun := tblNatives) (address : Nat := 0x1)
    (name : String := "table") (structs : List StructDecl := tblStructs) : CompiledModule :=
  let n := natives.length + tblFuns.length + funs.length
  mkModule address name (natives ++ tblFuns ++ funs)
    [{ address := 0x2, module := "coin", name := "mint", params := [.u64], returns := [coinTok] }]
    [] structs [{ address := 0x2, module := "coin", name := "Coin", abilities := { store := true } }]
    (extraSignatures := tblExtraSigs) (fieldHandles := [⟨1, 0⟩, ⟨0, 0⟩])
    (functionInstantiations := tblFunInsts n)
    (structDefInstantiations := [⟨0, tblSig n 0⟩, ⟨1, tblSig n 2⟩])
    (fieldInstantiations := [⟨0, tblSig n 2⟩, ⟨1, tblSig n 0⟩])

/-- Test functions 16–25 of `tblModule`, calling `table.move`'s functions at
`<u64, u64>` (instantiations 8–15) and at `<u64, Coin>` (16–18); `mint` is the
handle of `coin::mint`. -/
def tblTests (mint : FunctionHandleIndex) : List Fun :=
  [ -- 16: `add(&mut t, 1, 10)`; the table is returned as the native wrote it back.
    { name := "t_add", returns := [tblU64], locals := [tblU64],
      code := #[.callGeneric 8, .stLoc 0, .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .moveLoc 0, .ret] },
    -- 17: add 1 and 2, then `*borrow(&t, 1)`, then `remove(&mut t, 2)`.
    { name := "t_add_borrow_remove", returns := [.u64, .u64, tblU64], locals := [tblU64],
      code := #[.callGeneric 8, .stLoc 0,
        .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .mutBorrowLoc 0, .ldU64 2, .ldU64 20, .callGeneric 9,
        .immBorrowLoc 0, .ldU64 1, .callGeneric 10, .readRef,
        .mutBorrowLoc 0, .ldU64 2, .callGeneric 13,
        .moveLoc 0, .ret] },
    -- 18: `*borrow_mut(&mut t, 1) = 7`, then `*borrow(&t, 1)`.
    { name := "t_write_entry", returns := [.u64, tblU64], locals := [tblU64],
      code := #[.callGeneric 8, .stLoc 0,
        .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .ldU64 7, .mutBorrowLoc 0, .ldU64 1, .callGeneric 11, .writeRef,
        .immBorrowLoc 0, .ldU64 1, .callGeneric 10, .readRef,
        .moveLoc 0, .ret] },
    -- 19: `contains(&t, 1)` and `contains(&t, 2)`.
    { name := "t_contains", returns := [.bool, .bool, tblU64], locals := [tblU64],
      code := #[.callGeneric 8, .stLoc 0,
        .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .immBorrowLoc 0, .ldU64 1, .callGeneric 12,
        .immBorrowLoc 0, .ldU64 2, .callGeneric 12,
        .moveLoc 0, .ret] },
    -- 20: `drop_unchecked` of a table with an entry.
    { name := "t_drop", locals := [tblU64],
      code := #[.callGeneric 8, .stLoc 0, .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .moveLoc 0, .callGeneric 14, .ret] },
    -- 21: `drop_unchecked` of a table holding a coin.
    { name := "t_drop_coin", locals := [tblCoin],
      code := #[.callGeneric 16, .stLoc 0,
        .mutBorrowLoc 0, .ldU64 1, .ldU64 5, .call mint, .callGeneric 17,
        .moveLoc 0, .callGeneric 18, .ret] },
    -- 22: `contains(&t, 1)` while a `&mut` into the entry is still to be written through.
    { name := "t_contains_under_mut", returns := [tblU64], locals := [tblU64, .mutableReference .u64],
      code := #[.callGeneric 8, .stLoc 0,
        .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .mutBorrowLoc 0, .ldU64 1, .callGeneric 11, .stLoc 1,
        .immBorrowLoc 0, .ldU64 1, .callGeneric 12, .pop,
        .ldU64 7, .moveLoc 1, .writeRef,
        .moveLoc 0, .ret] },
    -- 23: `add(&mut t, 2, 20)` while a `&mut` into entry 1 is live, then a write through it.
    { name := "t_add_under_mut", returns := [tblU64], locals := [tblU64, .mutableReference .u64],
      code := #[.callGeneric 8, .stLoc 0,
        .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .mutBorrowLoc 0, .ldU64 1, .callGeneric 11, .stLoc 1,
        .mutBorrowLoc 0, .ldU64 2, .ldU64 20, .callGeneric 9,
        .ldU64 7, .moveLoc 1, .writeRef,
        .moveLoc 0, .ret] },
    -- 24: `remove(&mut t, 1)` of a table without that key.
    { name := "t_remove_missing", returns := [.u64, tblU64], locals := [tblU64],
      code := #[.callGeneric 8, .stLoc 0, .mutBorrowLoc 0, .ldU64 1, .callGeneric 13, .moveLoc 0,
        .ret] },
    -- 25: add and remove, then `destroy_known_empty_unsafe`.
    { name := "t_destroy_empty", returns := [.u64], locals := [tblU64],
      code := #[.callGeneric 8, .stLoc 0,
        .mutBorrowLoc 0, .ldU64 1, .ldU64 10, .callGeneric 9,
        .mutBorrowLoc 0, .ldU64 1, .callGeneric 13,
        .moveLoc 0, .callGeneric 15, .ret] }]

def tblModule : CompiledModule := tblModuleWith (tblTests 26)

def coinModule : CompiledModule :=
  mkModule 0x2 "coin"
    [{ name := "mint", params := [.u64], returns := [.struct 0], code := #[.moveLoc 0, .pack 0, .ret] }]
    [] [] [{ name := "Coin", abilities := { store := true }, fields := [.u64] }]

def tblP : Program := ⟨#[tblModule, coinModule]⟩

def tblT (v : RuntimeType) : StructId := ⟨tableModuleId, "Table", [tU64, v]⟩
def boxT (v : RuntimeType) : StructId := ⟨tableModuleId, "Box", [v]⟩
def coinT : StructId := ⟨⟨0x2, "coin"⟩, "Coin", []⟩
def handle77 : Value := .address 0x77
def entryV (v : RuntimeType) (x : Value) : Value := .struct (boxT v) [x]

/-- A `Table<u64, u64>` with handle `0x77` mapping `keys` to boxes of `vals`. -/
def tblV (keys vals : List Nat) : Value :=
  .table (tblT tU64) [handle77] (keys.map u64) (vals.map fun x => entryV tU64 (u64 x))

-- The six natives with references are modeled, with their shapes, and the
-- module keeps the table rules. `new_table_handle` and `drop_unchecked_box`
-- take and return values only and are not modeled; they are natives as any
-- other such.
#guard (tblModule.functionDefs.toList.take 8).map tblModule.modeledNative? ==
  [none, some .update, some .borrow, some .borrow, some .read, some .update, some .read, none]
#guard tblModule.followsTableRules && tblP.tablesHoldEntries
#guard nativesInSubset tblModule && nativesInSubset coinModule

/-- Sample error codes of the sample relation. -/
def eExists : Nat := 1
def eMissing : Nat := 2
def eNotEmpty : Nat := 3

/-- A sample relation for the table natives that meets each shape: it keeps
the entries in the order they were added, and fails on a key it has (`add_box`)
or lacks (the others), and `destroy_empty_box` on a table with entries. The
handle is always `0x77`: the spec does not ask for fresh handles. -/
def tblNative : NativeImpl := fun f views =>
  match f.index, views with
  | 0, [] => .returned [handle77] [] []
  | 1, [.table s fs ks es, k, b] =>
    if ks.contains k then .failed (.abort eExists)
    else .returned [] [.table s fs (ks ++ [k]) (es ++ [b])] []
  | 2, [.table _ _ ks _, k] | 3, [.table _ _ ks _, k] =>
    match ks.findIdx? (· == k) with
    | some i => .returned [] [] [⟨0, [.tableEntry i]⟩]
    | none => .failed (.abort eMissing)
  | 4, [.table _ _ ks _, k] => .returned [.bool (ks.contains k)] [] []
  | 5, [.table s fs ks es, k] =>
    match ks.findIdx? (· == k) with
    | some i =>
      match es[i]? with
      | some e => .returned [e] [.table s fs (ks.eraseIdx i) (es.eraseIdx i)] []
      | none => .failed (.abort eMissing)
    | none => .failed (.abort eMissing)
  | 6, [.table _ _ _ es] => if es.isEmpty then .returned [] [] [] else .failed (.abort eNotEmpty)
  | 7, [.table ..] => .returned [] [] []
  | _, _ => .failed (.abort 1)

/-- Its flows: `add_box` keeps the table's field and entries, and moves the
new entry in after them; `remove_box` moves the removed entry out into its
result and keeps the rest. The table written back is a new node: its value
differs from the one the native saw, and a node passed on is unchanged
(`NativeCallRespects`). The handle and `contains_box`'s result are new too. -/
def tblFlow : NativeFlows := fun f views _ j p =>
  match f.index, views with
  | 1, [.table _ fs _ es, _, _] =>
    match p with
    | [] => .fresh
    | c :: q => if c < fs.length + es.length then .arg 0 (c :: q) else .arg 2 q
  | 5, [.table _ fs ks _, k] =>
    let i := (ks.findIdx? (· == k)).getD 0
    if j == 0 then .arg 0 ((fs.length + i) :: p)
    else
      match p with
      | [] => .fresh
      | c :: q => .arg 0 ((if c < fs.length + i then c else c + 1) :: q)
  | _, _ => .fresh

/-- Test function `k` of `tblModule` and its outcome under `tblNative`. -/
def tblCases : List (Nat × RunResult) :=
  [(16, .returned [tblV [1] [10]]),
   (17, .returned [u64 10, u64 20, tblV [1] [10]]),
   (18, .returned [u64 7, tblV [1] [7]]),
   (19, .returned [.bool true, .bool false, tblV [1] [10]]),
   (20, .returned []),
   (21, .returned []),
   (22, .returned [tblV [1] [7]]),
   (23, .wrong .invalidReference),
   (24, .failed (.abort eMissing)),
   (25, .returned [u64 10])]

-- Packing a table gives one with no entries.
#guard runNat tblP tblNative 0 8 [] (typeArgs := [tU64, tU64]) == .returned [tblV [] []]
-- Add, borrow, remove, a write through a borrowed entry, `contains`,
-- `drop_unchecked` of a table with entries, and the failures; the native
-- writes the table back. `add` while a `&mut` into an entry is live makes that
-- reference invalid, as a vector's push does (`Invalidation.mutated`).
#guard tblCases.all fun (k, r) => runNat tblP tblNative 0 k [] == r
-- The ghost layer accounts for every step, with no D2, D3 or D4 violation, but
-- for the coin dropped with its table and the two calls under a live `&mut`
-- (below).
#guard tblCases.all fun (k, _) => agrees tblP 0 k [] tblNative tblFlow
#guard [16, 17, 18, 19, 20, 22, 23, 24, 25].all fun k => violations tblP 0 k [] tblNative tblFlow == []
#guard [16, 17, 18, 19, 20, 21, 24, 25].all fun k =>
  refViolationsWith tblP tblNative tblFlow 0 k [] == []
-- The semantics assumes nothing more: a relation that only fails meets the
-- shapes too, and every call fails.
#guard tblCases.all fun (k, _) =>
  runFun tblP 0 k [] == .failed (.abort 0) && agrees tblP 0 k [] &&
    violations tblP 0 k [] == [] && refViolations tblP 0 k [] == []
-- The flows meet the ownership contract: `add_box` and `remove_box` take the
-- table apart and make a new one, which a native of `0x1::table` may.
def addBoxU64 : FunctionRef := fn 0 1 [tU64, tU64, .struct (boxT tU64)]
def removeBoxU64 : FunctionRef := fn 0 5 [tU64, tU64, .struct (boxT tU64)]

#guard NativeCallRespects tblP {} addBoxU64 tableModuleId [tblV [1] [10], u64 2, entryV tU64 (u64 20)]
  [tblV [1, 2] [10, 20]]
  (tblFlow addBoxU64 [tblV [1] [10], u64 2, entryV tU64 (u64 20)] [tblV [1, 2] [10, 20]])
#guard NativeCallRespects tblP {} removeBoxU64 tableModuleId [tblV [1, 2] [10, 20], u64 1]
  [entryV tU64 (u64 10), tblV [2] [20]]
  (tblFlow removeBoxU64 [tblV [1, 2] [10, 20], u64 1] [entryV tU64 (u64 10), tblV [2] [20]])

-- Identities. `new_table_handle` creates the handle (identity 0), which `new`
-- packs into a table (1). `add_box` moves the handle and the box `add` packed
-- (5) around the value loaded (4) into the table it writes back, a new node (6).
#guard returnedIds (traceFun tblP 0 16 [] tblNative tblFlow) == some [6, 0, 5, 4]
-- Reading through the borrowed entry copies its value (4); `remove` moves the
-- second entry's value (9) out, in the box the native returns, which `remove`
-- unpacks; the first entry and the handle stay in the table.
#guard returnedIds (traceFun tblP 0 17 [] tblNative tblFlow) == some [4, 9, 18, 0, 5, 4]
-- `add_box` reads and writes the table through its reference (2), and consumes
-- the table node it replaced (1) and its key (3).
#guard (traceFun tblP 0 16 [] tblNative tblFlow).events.filter (fun e => match e with
    | .accessed _ _ (some _) => true
    | _ => false) ==
  [.accessed .read ⟨.local_ 0 0, []⟩ (some 2), .accessed .write ⟨.local_ 0 0, []⟩ (some 2)]
#guard ((traceFun tblP 0 16 [] tblNative tblFlow).ownershipEvents.filterMap fun e => match e with
    | .consumed i v _ => some (i, v)
    | _ => none) == [(1, tblV [] []), (3, u64 1)]
-- `borrow_box` reads the table through its reference and makes a reference to
-- the entry, derived from it; `borrow` then borrows the entry's field.
#guard ((traceFun tblP 0 17 [] tblNative tblFlow).events.filterMap Event.borrowed?).any
  fun (_, r, p) => r.location == ⟨.local_ 0 0, [.tableEntry 0]⟩ && !r.mutable && p.isSome
#guard ((traceFun tblP 0 17 [] tblNative tblFlow).events.filterMap Event.borrowed?).any
  fun (_, r, p) => r.location == ⟨.local_ 0 0, [.tableEntry 0, .field 0]⟩ && p.isSome
#guard ((traceFun tblP 0 18 [] tblNative tblFlow).events.filterMap Event.borrowed?).any
  fun (_, r, _) => r.location == ⟨.local_ 0 0, [.tableEntry 0, .field 0]⟩ && r.mutable
-- `contains_box` reads the whole table, as every native reads what its
-- reference arguments designate: under a live `&mut` into an entry, it reaches
-- over it (D4b′). `add_box` writes the table over it too, and the write through
-- the reference it invalidated is `wrong`.
#guard refViolationsWith tblP tblNative tblFlow 0 22 [] == ["D4b′"]
#guard (refViolationsWith tblP tblNative tblFlow 0 23 []).contains "D4b′"

/-! ### `drop_unchecked_box` consumes the table with its entries

The native takes the table by value and returns nothing: every node of the
table is consumed by it. A table of `u64`s holds only values with `drop`, but a
table holding a coin does not, and the native policy must allow the coin's
destruction (`NativePolicy.destroys`, language-owner ruling 3). -/

def tblWithCoin : Value :=
  .table (tblT (.struct coinT)) [handle77] [u64 1] [entryV (.struct coinT) (.struct coinT [u64 5])]

def destroysCoins : NativePolicy :=
  { destroys := fun f t => f.module == 0 && f.index == 7 && t == .struct coinT }

-- Every node of the table, its handle, its entry, the coin in it and the
-- coin's value, is consumed by the native, which passed nothing on.
#guard ((traceFun tblP 0 21 [] tblNative tblFlow).ownershipEvents.filterMap fun e => match e with
    | .consumed _ v (.native f m true) => if f.index == 7 && m == tableModuleId then some v else none
    | _ => none) ==
  [tblWithCoin, handle77, entryV (.struct coinT) (.struct coinT [u64 5]), .struct coinT [u64 5], u64 5]
#guard violations tblP 0 21 [] tblNative tblFlow == ["D2b", "D2c"]
#guard violations tblP 0 21 [] tblNative tblFlow (π := destroysCoins) == []
def dropCoinTable : FunctionRef := fn 0 7 [tU64, .struct coinT, .struct (boxT (.struct coinT))]

#guard !NativeCallRespects tblP {} dropCoinTable tableModuleId [tblWithCoin] [] (fun _ _ => .fresh)
#guard NativeCallRespects tblP destroysCoins dropCoinTable tableModuleId [tblWithCoin] []
  (fun _ _ => .fresh)
-- The table of `u64`s needs no permission: the native has the privileges of
-- `Table` and `Box`, and the rest has `drop`.
#guard NativeCallRespects tblP {} (fn 0 7 [tU64, tU64, .struct (boxT tU64)]) tableModuleId
  [tblV [1] [10]] [] (fun _ _ => .fresh)

/-! ### What the shapes reject -/

/-- `tblNative`, with native `k`'s result replaced. -/
def tblWith (k : Nat) (r : List Value → NativeResult) : NativeImpl := fun f views =>
  if f.index == k then r views else tblNative f views

-- `borrow_box` returning a place that does not exist, the table itself (a
-- `Table`, not a `Box`), its key argument, or the entry's field (a `u64`);
-- `add_box` writing back a table with a key twice, a plain struct value for the
-- table, or a table of another type, or returning a place; `remove_box`
-- returning a value of another type; `contains_box` writing back.
#guard runNat tblP (tblWith 2 fun _ => .returned [] [] [⟨0, [.tableEntry 5]⟩]) 0 17 [] ==
  .wrong .nativeContract
#guard runNat tblP (tblWith 2 fun _ => .returned [] [] [⟨0, []⟩]) 0 17 [] == .wrong .nativeContract
#guard runNat tblP (tblWith 2 fun _ => .returned [] [] [⟨1, []⟩]) 0 17 [] == .wrong .nativeContract
#guard runNat tblP (tblWith 3 fun _ => .returned [] [] [⟨0, [.tableEntry 0, .field 0]⟩]) 0 18 [] ==
  .wrong .nativeContract
#guard runNat tblP (tblWith 1 fun views => match views with
    | [.table s fs ks es, k, b] => .returned [] [.table s fs (ks ++ [k, k]) (es ++ [b, b])] []
    | _ => .failed (.abort 1)) 0 16 [] == .wrong .nativeContract
#guard runNat tblP (tblWith 1 fun _ => .returned [] [.struct (tblT tU64) [handle77]] []) 0 16 [] ==
  .wrong .nativeContract
#guard runNat tblP (tblWith 1 fun _ => .returned [] [.table (tblT .bool) [handle77] [] []] []) 0 16
  [] == .wrong .nativeContract
#guard runNat tblP (tblWith 1 fun views => .returned [] (views.take 1) [⟨0, []⟩]) 0 16 [] ==
  .wrong .nativeContract
#guard runNat tblP (tblWith 5 fun views => .returned [u64 20] (views.take 1) []) 0 17 [] ==
  .wrong .nativeContract
#guard runNat tblP (tblWith 4 fun views => .returned [.bool true] (views.take 1) []) 0 19 [] ==
  .wrong .nativeContract
-- `destroy_empty_box` only reads, and promises nothing about emptiness: a
-- relation for which it returns on a table with entries meets its shape.
#guard runNat tblP (tblWith 6 fun _ => .returned [] [] []) 0 15 [tblV [1] [10]]
  (typeArgs := [tU64, tU64]) == .returned []
#guard runNat tblP tblNative 0 15 [tblV [1] [10]] (typeArgs := [tU64, tU64]) ==
  .failed (.abort eNotEmpty)

/-! ### Conformance of tables

A value of the table type is a table. One with entries is well formed only
where the table module keeps the table rules, with one distinct key per entry,
and with entries whose type has the abilities the table's type has: under
`table.move`'s `Table`, `store`. -/

#guard (tblV [1, 2] [10, 20]).wellFormed tblP && (tblV [] []).wellFormed tblP
#guard (tblV [1, 2] [10, 20]).abilities tblP == { store := true } && tblWithCoin.wellFormed tblP
#guard !(Value.struct (tblT tU64) [handle77]).wellFormed tblP
#guard !(Value.table (tblT tU64) [handle77] [u64 1, u64 1]
  [entryV tU64 (u64 1), entryV tU64 (u64 2)]).wellFormed tblP
#guard !(Value.table (tblT tU64) [handle77] [u64 1] []).wellFormed tblP
-- `Box<signer>` lacks `store`: a `Table<u64, signer>` holds no entries.
#guard (Value.table (tblT .signer) [handle77] [] []).wellFormed tblP
#guard !(Value.table (tblT .signer) [handle77] [u64 1] [entryV .signer (signerAt 1)]).wellFormed tblP
-- A reference to an entry reads it; an entry is replaced in place.
#guard (tblV [1, 2] [10, 20]).get? [.tableEntry 1, .field 0] == some (u64 20)
#guard (tblV [1, 2] [10, 20]).set? [.tableEntry 0] (entryV tU64 (u64 7)) == some (tblV [1, 2] [7, 20])
#guard (tblV [1] [10]).get? [.tableEntry 1] == none

/-! ### The table rules

Each module below breaks one rule (ruling 2, with rules 5 to 7 refining it)
and adds a test function `t` (function 17) calling `contains` on a new table.
Its table natives are not modeled, it is not in the subset, the call is
`unsupported`, and its tables hold no entries. -/

def tblProbe : Fun :=
  { name := "t", returns := [.bool, tblU64], locals := [tblU64],
    code := #[.callGeneric 8, .stLoc 0, .immBorrowLoc 0, .ldU64 1, .callGeneric 12, .moveLoc 0, .ret] }

/-- Rule 1: packs an address it did not get from `new_table_handle`. -/
def tblForge : Fun :=
  { name := "forge", typeParameters := [copyDrop, { store := true }], params := [.address],
    returns := [tblTok tK tV], code := #[.moveLoc 0, .packGeneric 0, .ret] }

/-- Rule 1: packs at an offset a jump targets. -/
def tblJumpIn : Fun :=
  { name := "new_jump", typeParameters := [copyDrop, { store := true }], returns := [tblTok tK tV],
    code := #[.callGeneric 0, .packGeneric 0, .ret, .branch 1] }

/-- Rule 2: unpacks a table. -/
def tblUnpack : Fun :=
  { name := "handle", typeParameters := [copyDrop, {}], params := [tblTok tK tV],
    returns := [.address], code := #[.moveLoc 0, .unpackGeneric 0, .ret] }

/-- Rule 3: packs at a `V` without `store`. -/
def tblNewAny : Fun :=
  { name := "new_any", typeParameters := [copyDrop, {}], returns := [tblTok tK tV],
    code := #[.callGeneric 0, .packGeneric 0, .ret] }

/-- Rule 4: calls `add_box` with `B = V`. -/
def tblAddRaw : Fun :=
  { name := "add_raw", typeParameters := [copyDrop, {}],
    params := [.mutableReference (tblTok tK tV), tK, boxTok tV],
    code := #[.moveLoc 0, .moveLoc 1, .moveLoc 2, .callGeneric 19, .ret] }

/-- Rule 6: `t.handle = h`, a mutable borrow of a table's field. -/
def tblSetHandle : Fun :=
  { name := "set_handle", typeParameters := [copyDrop, {}],
    params := [.mutableReference (tblTok tK tV), .address],
    code := #[.moveLoc 1, .moveLoc 0, .mutBorrowFieldGeneric 1, .writeRef, .ret] }

/-- `*&t.handle`: an immutable borrow of a table's field, which the rules allow. -/
def tblHandleOf : Fun :=
  { name := "handle_of", typeParameters := [copyDrop, {}], params := [.reference (tblTok tK tV)],
    returns := [.address], code := #[.moveLoc 0, .immBorrowFieldGeneric 1, .readRef, .ret] }

/-- The natives of `table.move`, with native `name` declared with visibility
`v`. -/
def tblNativesWith (name : String) (v : Visibility) : List Fun :=
  tblNatives.map fun f => if f.name == name then { f with visibility := v } else f

def tblBreaking : List CompiledModule :=
  [tblForge, tblJumpIn, tblUnpack, tblNewAny, tblAddRaw, tblSetHandle].map
      (fun f => tblModuleWith [f, tblProbe]) ++
    -- Rule 7: `add_box` public, `new_table_handle` friend.
    [tblNativesWith "add_box" .public_, tblNativesWith "new_table_handle" .friend].map
      (fun natives => tblModuleWith [tblHandleOf, tblProbe] (natives := natives)) ++
    -- Rule 5: `Table` declared with `drop` or `copy`, with its field named
    -- otherwise, or with a second field.
    [tblStructsWith { drop := true, store := true } [("handle", .address)],
     tblStructsWith { copy := true, store := true } [("handle", .address)],
     tblStructsWith { store := true } [("h", .address)],
     tblStructsWith { store := true } [("handle", .address), ("size", .u64)]].map
      (fun structs => tblModuleWith [tblHandleOf, tblProbe] (structs := structs))

#guard tblBreaking.all fun m =>
  !m.followsTableRules && (m.functionDefs.toList.take 8).all (m.modeledNative? · == none) &&
    !nativesInSubset m
-- But for the last, whose `Table` has two fields, so that `new` packs too few
-- and its tables have another shape.
#guard tblBreaking.dropLast.all fun m =>
  runNat ⟨#[m, coinModule]⟩ tblNative 0 17 [] == .wrong (.unsupported "native with references")
#guard tblBreaking.dropLast.all fun m =>
  !(tblV [1] [10]).wellFormed ⟨#[m, coinModule]⟩ && (tblV [] []).wellFormed ⟨#[m, coinModule]⟩

-- Reading a table's handle through an immutable borrow keeps the rules, and
-- works on a table with entries the transaction holds.
def tblReading : CompiledModule := tblModuleWith [tblHandleOf, tblProbe]

#guard tblReading.followsTableRules
#guard runNat ⟨#[tblReading, coinModule]⟩ tblNative 0 16
  [.ref ⟨false, .struct (tblT tU64), ⟨.held 0, []⟩, 0⟩] [tblV [1] [10]] [tU64, tU64] ==
  .returned [handle77]


-- Where a module unpacks a table, `Unpack` takes its fields.
#guard runNat ⟨#[tblBreaking[2]!, coinModule]⟩ tblNative 0 16 [tblV [] []]
  (typeArgs := [tU64, tU64]) == .returned [handle77]

-- Near misses of the identification: `K` with `copy` only, `V` with `store`, no
-- `B`, the module at another address, or under another name.
def tblNearMisses : List CompiledModule :=
  [tblModuleWith [] (natives := tblNatives [{ copy := true }, {}, {}]),
   tblModuleWith [] (natives := tblNatives [copyDrop, { store := true }, {}]),
   tblModuleWith [] (natives := tblNatives [copyDrop, {}]),
   tblModuleWith [] (address := 0x2),
   tblModuleWith [] (name := "tables")]

#guard tblNearMisses.all fun m => (m.functionDefs.toList.take 8).all (m.modeledNative? · == none)
#guard tblNearMisses.all fun m => !nativesInSubset m

end MoveBytecode.Tests
