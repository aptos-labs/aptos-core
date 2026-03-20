# DVR-143: Fix "serializeAsBytes is not a function" in Petra fee payer flow

## Problem

After upgrading to Aptos TS SDK v6.2.0, using Petra wallet with the fee payer
(sign + submit) flow fails with:

```
Error: e.serializeAsBytes is not a function
```

## Root Cause

In commit `e5d3ebb1` ("performance pass"), all `serializeForEntryFunction` methods
were changed from:

```typescript
serializeForEntryFunction(serializer: Serializer): void {
  const bcsBytes = this.bcsToBytes();
  serializer.serializeBytes(bcsBytes);
}
```

to:

```typescript
serializeForEntryFunction(serializer: Serializer): void {
  serializer.serializeAsBytes(this);
}
```

The `serializeAsBytes` method was added to `Serializer` as a performance
optimization (uses pooled serializers to reduce allocations). However, this
creates a **cross-version compatibility issue**.

### The Cross-Version Problem

When the wallet-standard `signTransaction` method is called:

1. The dapp builds a transaction using SDK v6 → returns `SimpleTransaction` with
   v6 prototype chain
2. The wallet adapter passes this object directly to the wallet's `signTransaction`
3. The wallet (e.g. Petra) receives the actual JavaScript object (same JS context)
4. The wallet creates a `Serializer` from its **own bundled SDK version** (older)
5. The wallet calls `transaction.rawTransaction.serialize(oldSerializer)`
6. The v6 `RawTransaction.serialize()` calls `this.payload.serialize(oldSerializer)`
7. Eventually `serializeForEntryFunction(oldSerializer)` is called
8. The v6 code calls `oldSerializer.serializeAsBytes(this)` → **FAILS** because
   the old serializer doesn't have `serializeAsBytes`

## Fix (applied to `aptos-labs/aptos-ts-sdk`)

### Patch: `0001-ts-sdk-fix.patch`

Apply to the `aptos-ts-sdk` repository:

```bash
cd aptos-ts-sdk
git apply 0001-ts-sdk-fix.patch
```

### Changes Summary

1. **New helper function** `serializeEntryFunctionBytesCompat()` in
   `src/bcs/serializer.ts` — performs a runtime check: if the serializer has
   `serializeAsBytes`, use it (optimal path); otherwise fall back to the pre-v6
   `bcsToBytes()` + `serializeBytes()` pattern.

2. **Updated 17 `serializeForEntryFunction` implementations** across:
   - `src/bcs/serializable/movePrimitives.ts` — Bool, U8, U16, U32, U64, U128,
     U256, I8, I16, I32, I64, I128, I256
   - `src/bcs/serializable/moveStructs.ts` — MoveVector, MoveString, MoveOption
   - `src/core/accountAddress.ts` — AccountAddress

3. **25 new unit tests** in `tests/unit/walletSerializerCompat.test.ts` that:
   - Create a "legacy" serializer (with `serializeAsBytes` removed)
   - Verify all Move primitive types produce identical bytes via both paths
   - Verify complex types (AccountAddress, MoveVector, MoveString, MoveOption)
   - Verify full `RawTransaction`, `SimpleTransaction`, and
     `FeePayerRawTransaction` serialization
   - Simulate the exact wallet adapter flow scenario

### Affected Files

```
src/bcs/serializer.ts                          (+22 lines)
src/bcs/serializable/movePrimitives.ts         (13 changed lines)
src/bcs/serializable/moveStructs.ts            (3 changed lines)
src/core/accountAddress.ts                     (1 changed line)
tests/unit/walletSerializerCompat.test.ts      (new, 298 lines)
```

## Wallet Adapter Considerations

The wallet adapter (`aptos-labs/aptos-wallet-adapter`) does not need code changes
for this specific issue. The bug occurs inside the wallet's `signTransaction`
implementation (e.g. Petra), where it uses an older SDK's serializer. The fix in
the TS SDK ensures v6 objects are compatible with older serializers.

However, for additional robustness, the wallet adapter could consider:
- Serializing the transaction to bytes before passing to the wallet (line 912 of
  `WalletCore.ts`), so the wallet can deserialize with its own SDK
- Adding defensive checks for `bcsToBytes` availability on wallet response objects
  (line 957 of `WalletCore.ts`)
