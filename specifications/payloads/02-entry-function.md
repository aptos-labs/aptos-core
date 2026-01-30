# Entry Function Payload Specification

> **Version:** 1.0.0  
> **Last Updated:** January 28, 2026

## Overview

The `EntryFunction` payload is the most common transaction payload type. It calls an existing function published on-chain that is marked with the `entry` modifier.

## Structure Definition

```rust
pub struct EntryFunction {
    module: ModuleId,           // Module containing the function
    function: Identifier,       // Function name
    ty_args: Vec<TypeTag>,      // Generic type arguments
    args: Vec<Vec<u8>>,         // BCS-encoded arguments
}
```

## BCS Layout

```
+------------------------------------------+
|          EntryFunction Payload           |
+------------------------------------------+
| module: ModuleId                         |
|   +-- address: [u8; 32]                  |
|   +-- name: Identifier                   |
|       +-- length: ULEB128                |
|       +-- bytes: [u8; length]            |
+------------------------------------------+
| function: Identifier                     |
|   +-- length: ULEB128                    |
|   +-- bytes: [u8; length]                |
+------------------------------------------+
| ty_args: Vec<TypeTag>                    |
|   +-- count: ULEB128                     |
|   +-- [TypeTag; count]                   |
+------------------------------------------+
| args: Vec<Vec<u8>>                       |
|   +-- count: ULEB128                     |
|   +-- for each arg:                      |
|       +-- length: ULEB128                |
|       +-- bytes: [u8; length]            |
+------------------------------------------+
```

### Byte-by-Byte Example

For calling `0x1::aptos_account::transfer(recipient, amount)`:

```
// TransactionPayload variant (EntryFunction = 2)
02

// ModuleId
// - address (32 bytes, address 0x1 left-padded)
0000000000000000000000000000000000000000000000000000000000000001
// - name length (13 = "aptos_account")
0d
// - name bytes
6170746f735f6163636f756e74  // "aptos_account"

// Function name
// - length (8 = "transfer")
08
// - bytes
7472616e73666572  // "transfer"

// Type arguments count (0)
00

// Arguments count (2)
02

// Argument 1: recipient address (BCS-encoded)
// - length (32 bytes)
20
// - BCS bytes (address)
<32 bytes recipient address>

// Argument 2: amount (BCS-encoded u64)
// - length (8 bytes)
08
// - BCS bytes (u64 little-endian)
<8 bytes amount>
```

## ModuleId Structure

```rust
pub struct ModuleId {
    pub address: AccountAddress,  // 32 bytes
    pub name: Identifier,         // Variable length string
}
```

### Common Module Addresses

| Address | Common Modules |
|---------|----------------|
| `0x1` | `aptos_account`, `coin`, `account`, `aptos_coin` |
| `0x3` | `token` (NFT v1) |
| `0x4` | `aptos_token`, `collection`, `token` (NFT v2) |

## Identifier

An identifier is a valid Move identifier string:
- Starts with a letter or underscore
- Contains only letters, digits, and underscores
- Maximum 128 characters

### BCS Encoding

```
| length (ULEB128) | bytes (UTF-8) |
```

## Function Arguments

Arguments are BCS-encoded values matching the function's parameter types.

### Argument Encoding Rules

1. Each argument is independently BCS-encoded
2. The `args` vector contains the raw BCS bytes for each argument
3. Arguments must match the function signature exactly

### Common Argument Types

| Move Type | BCS Encoding |
|-----------|--------------|
| `u8` | 1 byte |
| `u16` | 2 bytes (little-endian) |
| `u32` | 4 bytes (little-endian) |
| `u64` | 8 bytes (little-endian) |
| `u128` | 16 bytes (little-endian) |
| `u256` | 32 bytes (little-endian) |
| `bool` | 1 byte (0x00 or 0x01) |
| `address` | 32 bytes |
| `vector<u8>` | ULEB128 length + bytes |
| `String` | ULEB128 length + UTF-8 bytes |
| `vector<T>` | ULEB128 count + BCS(elements) |
| `Option<T>` | 0x00 (None) or 0x01 + BCS(value) |

## Code Examples

### Rust

```rust
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::ModuleId,
};

/// Build an APT transfer entry function
fn build_transfer(recipient: AccountAddress, amount: u64) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::ONE,
            Identifier::new("aptos_account").unwrap(),
        ),
        Identifier::new("transfer").unwrap(),
        vec![],  // no type arguments
        vec![
            bcs::to_bytes(&recipient).unwrap(),  // recipient
            bcs::to_bytes(&amount).unwrap(),     // amount
        ],
    ))
}

/// Build a coin transfer with type argument
fn build_coin_transfer(
    coin_type: TypeTag,
    recipient: AccountAddress,
    amount: u64,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::ONE,
            Identifier::new("aptos_account").unwrap(),
        ),
        Identifier::new("transfer_coins").unwrap(),
        vec![coin_type],  // coin type argument
        vec![
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ))
}

/// Build a register coin entry function
fn build_register_coin(coin_type: TypeTag) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::ONE,
            Identifier::new("managed_coin").unwrap(),
        ),
        Identifier::new("register").unwrap(),
        vec![coin_type],
        vec![],  // no arguments
    ))
}
```

### Python

```python
from dataclasses import dataclass
from typing import List, Optional
import struct

def encode_uleb128(value: int) -> bytes:
    """Encode an integer as ULEB128."""
    result = []
    while True:
        byte = value & 0x7f
        value >>= 7
        if value != 0:
            byte |= 0x80
        result.append(byte)
        if value == 0:
            break
    return bytes(result)

def encode_identifier(name: str) -> bytes:
    """Encode a Move identifier."""
    name_bytes = name.encode('utf-8')
    return encode_uleb128(len(name_bytes)) + name_bytes

def encode_address(address: str) -> bytes:
    """Encode an account address (32 bytes)."""
    # Remove 0x prefix if present
    if address.startswith('0x'):
        address = address[2:]
    # Pad to 64 hex chars (32 bytes)
    address = address.zfill(64)
    return bytes.fromhex(address)

def encode_u64(value: int) -> bytes:
    """Encode a u64 value (little-endian)."""
    return struct.pack('<Q', value)

def encode_string(s: str) -> bytes:
    """Encode a string (BCS format)."""
    s_bytes = s.encode('utf-8')
    return encode_uleb128(len(s_bytes)) + s_bytes

def encode_vector(items: List[bytes]) -> bytes:
    """Encode a vector of pre-encoded items."""
    result = encode_uleb128(len(items))
    for item in items:
        result += item
    return result

@dataclass
class ModuleId:
    address: str
    name: str
    
    def encode(self) -> bytes:
        return encode_address(self.address) + encode_identifier(self.name)

@dataclass
class EntryFunction:
    module: ModuleId
    function: str
    type_args: List['TypeTag']
    args: List[bytes]  # Pre-BCS-encoded arguments
    
    def encode(self) -> bytes:
        result = self.module.encode()
        result += encode_identifier(self.function)
        
        # Type arguments
        result += encode_uleb128(len(self.type_args))
        for ty_arg in self.type_args:
            result += ty_arg.encode()
        
        # Arguments (each is already BCS-encoded, wrapped in length prefix)
        result += encode_uleb128(len(self.args))
        for arg in self.args:
            result += encode_uleb128(len(arg)) + arg
        
        return result
    
    def to_payload(self) -> bytes:
        """Encode as TransactionPayload::EntryFunction (variant 2)."""
        return bytes([0x02]) + self.encode()


def build_transfer_payload(recipient: str, amount: int) -> EntryFunction:
    """Build an APT transfer entry function."""
    return EntryFunction(
        module=ModuleId(address="0x1", name="aptos_account"),
        function="transfer",
        type_args=[],
        args=[
            encode_address(recipient),  # recipient
            encode_u64(amount),         # amount
        ],
    )


def build_coin_transfer_payload(
    coin_type: 'TypeTag',
    recipient: str,
    amount: int,
) -> EntryFunction:
    """Build a coin transfer with type argument."""
    return EntryFunction(
        module=ModuleId(address="0x1", name="aptos_account"),
        function="transfer_coins",
        type_args=[coin_type],
        args=[
            encode_address(recipient),
            encode_u64(amount),
        ],
    )


# Example usage
if __name__ == "__main__":
    # Build a transfer
    payload = build_transfer_payload(
        recipient="0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        amount=1_000_000,  # 0.01 APT (1M octas)
    )
    
    # Encode to bytes
    payload_bytes = payload.to_payload()
    print(f"Payload hex: {payload_bytes.hex()}")
```

### TypeScript

```typescript
function encodeULEB128(value: number): Uint8Array {
  const result: number[] = [];
  while (true) {
    let byte = value & 0x7f;
    value >>>= 7;
    if (value !== 0) {
      byte |= 0x80;
    }
    result.push(byte);
    if (value === 0) break;
  }
  return new Uint8Array(result);
}

function encodeIdentifier(name: string): Uint8Array {
  const nameBytes = new TextEncoder().encode(name);
  const length = encodeULEB128(nameBytes.length);
  const result = new Uint8Array(length.length + nameBytes.length);
  result.set(length, 0);
  result.set(nameBytes, length.length);
  return result;
}

function encodeAddress(address: string): Uint8Array {
  // Remove 0x prefix
  const hex = address.startsWith('0x') ? address.slice(2) : address;
  // Pad to 64 hex chars
  const padded = hex.padStart(64, '0');
  // Convert to bytes
  const result = new Uint8Array(32);
  for (let i = 0; i < 32; i++) {
    result[i] = parseInt(padded.slice(i * 2, i * 2 + 2), 16);
  }
  return result;
}

function encodeU64(value: bigint): Uint8Array {
  const result = new Uint8Array(8);
  const view = new DataView(result.buffer);
  view.setBigUint64(0, value, true); // little-endian
  return result;
}

function encodeString(s: string): Uint8Array {
  const bytes = new TextEncoder().encode(s);
  const length = encodeULEB128(bytes.length);
  const result = new Uint8Array(length.length + bytes.length);
  result.set(length, 0);
  result.set(bytes, length.length);
  return result;
}

interface ModuleId {
  address: string;
  name: string;
}

function encodeModuleId(moduleId: ModuleId): Uint8Array {
  const address = encodeAddress(moduleId.address);
  const name = encodeIdentifier(moduleId.name);
  const result = new Uint8Array(address.length + name.length);
  result.set(address, 0);
  result.set(name, address.length);
  return result;
}

interface EntryFunction {
  module: ModuleId;
  function: string;
  typeArgs: Uint8Array[];  // Pre-encoded type tags
  args: Uint8Array[];       // Pre-BCS-encoded arguments
}

function encodeEntryFunction(ef: EntryFunction): Uint8Array {
  const parts: Uint8Array[] = [];
  
  // Module ID
  parts.push(encodeModuleId(ef.module));
  
  // Function name
  parts.push(encodeIdentifier(ef.function));
  
  // Type arguments
  parts.push(encodeULEB128(ef.typeArgs.length));
  for (const typeArg of ef.typeArgs) {
    parts.push(typeArg);
  }
  
  // Arguments (each wrapped in length prefix)
  parts.push(encodeULEB128(ef.args.length));
  for (const arg of ef.args) {
    parts.push(encodeULEB128(arg.length));
    parts.push(arg);
  }
  
  // Combine all parts
  const totalLength = parts.reduce((sum, p) => sum + p.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  
  return result;
}

function encodeEntryFunctionPayload(ef: EntryFunction): Uint8Array {
  const encoded = encodeEntryFunction(ef);
  const result = new Uint8Array(1 + encoded.length);
  result[0] = 0x02;  // EntryFunction variant
  result.set(encoded, 1);
  return result;
}

// Build transfer payload
function buildTransferPayload(recipient: string, amount: bigint): EntryFunction {
  return {
    module: { address: '0x1', name: 'aptos_account' },
    function: 'transfer',
    typeArgs: [],
    args: [
      encodeAddress(recipient),
      encodeU64(amount),
    ],
  };
}

// Example usage
const payload = buildTransferPayload(
  '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef',
  BigInt(1_000_000),
);

const payloadBytes = encodeEntryFunctionPayload(payload);
console.log('Payload hex:', Buffer.from(payloadBytes).toString('hex'));

export {
  encodeULEB128,
  encodeIdentifier,
  encodeAddress,
  encodeU64,
  encodeModuleId,
  encodeEntryFunction,
  encodeEntryFunctionPayload,
  buildTransferPayload,
};
```

## Common Entry Functions

### Token Transfers

| Function | Module | Type Args | Arguments |
|----------|--------|-----------|-----------|
| `transfer` | `0x1::aptos_account` | - | (to: address, amount: u64) |
| `transfer_coins` | `0x1::aptos_account` | CoinType | (to: address, amount: u64) |
| `transfer` | `0x1::coin` | CoinType | (to: address, amount: u64) |

### Account Management

| Function | Module | Type Args | Arguments |
|----------|--------|-----------|-----------|
| `create_account` | `0x1::aptos_account` | - | (auth_key: address) |
| `rotate_authentication_key` | `0x1::account` | - | (new_auth_key: vector<u8>) |
| `register` | `0x1::managed_coin` | CoinType | - |

### Staking

| Function | Module | Type Args | Arguments |
|----------|--------|-----------|-----------|
| `add_stake` | `0x1::stake` | - | (amount: u64) |
| `unlock` | `0x1::stake` | - | (amount: u64) |
| `withdraw` | `0x1::stake` | - | - |

## Validation Rules

1. **Module Existence**: The module must be published at the specified address
2. **Function Existence**: The function must exist in the module
3. **Entry Modifier**: Function must have `entry` modifier
4. **Visibility**: Function must be `public` or `public(friend)`
5. **Type Argument Count**: Must match function's generic parameters
6. **Type Argument Constraints**: Types must satisfy constraints
7. **Argument Count**: Must match function's parameters (excluding signer)
8. **Argument Types**: BCS bytes must deserialize to correct types

## Error Codes

| Error | Description |
|-------|-------------|
| `FUNCTION_NOT_FOUND` | Function doesn't exist |
| `TYPE_MISMATCH` | Argument type doesn't match |
| `INVALID_ARGUMENT_LENGTH` | Wrong number of arguments |
| `MODULE_NOT_FOUND` | Module doesn't exist |

## Test Vector

### Input
- Module: `0x1::aptos_account`
- Function: `transfer`
- Type args: none
- Arguments:
  - recipient: `0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`
  - amount: `100000000` (1 APT)

### Expected BCS Output

```
02                                                              // EntryFunction variant
0000000000000000000000000000000000000000000000000000000000000001  // module address
0d                                                              // name length
6170746f735f6163636f756e74                                      // "aptos_account"
08                                                              // function length
7472616e73666572                                                // "transfer"
00                                                              // 0 type args
02                                                              // 2 arguments
20                                                              // arg1 length (32)
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  // recipient
08                                                              // arg2 length (8)
00e1f50500000000                                                // 100000000 as u64 LE
```

## Related Documents

- [Payload Overview](01-payload-overview.md) - All payload types
- [Script Payload](03-script-payload.md) - Script payload specification
- [Move Types](04-move-types.md) - TypeTag and argument encoding
