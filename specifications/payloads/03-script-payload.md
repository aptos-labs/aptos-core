# Script Payload Specification

> **Version:** 1.0.0  
> **Last Updated:** January 28, 2026

## Overview

The `Script` payload allows executing custom Move bytecode directly within a transaction. Unlike entry functions, scripts don't require prior module publication - the bytecode is included in the transaction itself.

## Structure Definition

```rust
pub struct Script {
    code: Vec<u8>,                      // Compiled Move script bytecode
    ty_args: Vec<TypeTag>,              // Generic type arguments
    args: Vec<TransactionArgument>,     // Script arguments
}
```

## BCS Layout

```
+------------------------------------------+
|             Script Payload               |
+------------------------------------------+
| code: Vec<u8>                            |
|   +-- length: ULEB128                    |
|   +-- bytecode: [u8; length]             |
+------------------------------------------+
| ty_args: Vec<TypeTag>                    |
|   +-- count: ULEB128                     |
|   +-- [TypeTag; count]                   |
+------------------------------------------+
| args: Vec<TransactionArgument>           |
|   +-- count: ULEB128                     |
|   +-- [TransactionArgument; count]       |
+------------------------------------------+
```

### Byte-by-Byte Layout

```
// TransactionPayload variant (Script = 0)
00

// Code
// - length (ULEB128)
<bytecode_length>
// - bytecode bytes
<compiled_script_bytecode>

// Type arguments
// - count (ULEB128)
<type_arg_count>
// - each type argument
<type_tag_0>
<type_tag_1>
...

// Arguments
// - count (ULEB128)
<arg_count>
// - each argument (TransactionArgument enum)
<arg_0>
<arg_1>
...
```

## TransactionArgument Enum

Script arguments use the `TransactionArgument` enum, which differs from BCS-encoded bytes used in entry functions:

```rust
pub enum TransactionArgument {
    U8(u8),           // Variant 0
    U64(u64),         // Variant 1
    U128(u128),       // Variant 2
    Address(Address), // Variant 3
    U8Vector(Vec<u8>),// Variant 4
    Bool(bool),       // Variant 5
    U16(u16),         // Variant 6
    U32(u32),         // Variant 7
    U256(U256),       // Variant 8
    Serialized(Vec<u8>), // Variant 9 (feature-gated)
}
```

### TransactionArgument BCS Layout

| Variant | Index | Payload |
|---------|-------|---------|
| U8 | 0x00 | 1 byte value |
| U64 | 0x01 | 8 bytes (little-endian) |
| U128 | 0x02 | 16 bytes (little-endian) |
| Address | 0x03 | 32 bytes |
| U8Vector | 0x04 | ULEB128 length + bytes |
| Bool | 0x05 | 1 byte (0x00 or 0x01) |
| U16 | 0x06 | 2 bytes (little-endian) |
| U32 | 0x07 | 4 bytes (little-endian) |
| U256 | 0x08 | 32 bytes (little-endian) |
| Serialized | 0x09 | ULEB128 length + BCS bytes |

## Script vs Entry Function Arguments

**Key Difference**: Scripts use `TransactionArgument` with explicit type tags, while entry functions use raw BCS-encoded bytes.

| Feature | Script | Entry Function |
|---------|--------|----------------|
| Argument encoding | TransactionArgument enum | Raw BCS bytes |
| Type information | Embedded in argument | Inferred from function signature |
| Complex types | Limited (via Serialized) | Full BCS support |
| String support | Via U8Vector | Native BCS String |

## Code Examples

### Rust

```rust
use aptos_types::transaction::{Script, TransactionPayload, TransactionArgument};
use move_core_types::language_storage::TypeTag;

/// Build a script payload
fn build_script_payload(
    bytecode: Vec<u8>,
    type_args: Vec<TypeTag>,
    args: Vec<TransactionArgument>,
) -> TransactionPayload {
    TransactionPayload::Script(Script::new(bytecode, type_args, args))
}

/// Example: Simple transfer script
fn build_transfer_script(
    bytecode: Vec<u8>,
    recipient: AccountAddress,
    amount: u64,
) -> TransactionPayload {
    TransactionPayload::Script(Script::new(
        bytecode,
        vec![],  // no type arguments
        vec![
            TransactionArgument::Address(recipient),
            TransactionArgument::U64(amount),
        ],
    ))
}

/// Example: Script with type argument
fn build_generic_script(
    bytecode: Vec<u8>,
    coin_type: TypeTag,
    amount: u64,
) -> TransactionPayload {
    TransactionPayload::Script(Script::new(
        bytecode,
        vec![coin_type],
        vec![TransactionArgument::U64(amount)],
    ))
}
```

### Python

```python
from dataclasses import dataclass
from typing import List, Union
from enum import IntEnum

class TransactionArgumentType(IntEnum):
    U8 = 0
    U64 = 1
    U128 = 2
    ADDRESS = 3
    U8_VECTOR = 4
    BOOL = 5
    U16 = 6
    U32 = 7
    U256 = 8
    SERIALIZED = 9

@dataclass
class TransactionArgument:
    arg_type: TransactionArgumentType
    value: Union[int, bool, str, bytes]
    
    def encode(self) -> bytes:
        result = bytes([self.arg_type])
        
        if self.arg_type == TransactionArgumentType.U8:
            result += bytes([self.value])
        elif self.arg_type == TransactionArgumentType.U64:
            result += self.value.to_bytes(8, 'little')
        elif self.arg_type == TransactionArgumentType.U128:
            result += self.value.to_bytes(16, 'little')
        elif self.arg_type == TransactionArgumentType.ADDRESS:
            result += encode_address(self.value)
        elif self.arg_type == TransactionArgumentType.U8_VECTOR:
            data = self.value if isinstance(self.value, bytes) else bytes.fromhex(self.value)
            result += encode_uleb128(len(data)) + data
        elif self.arg_type == TransactionArgumentType.BOOL:
            result += bytes([0x01 if self.value else 0x00])
        elif self.arg_type == TransactionArgumentType.U16:
            result += self.value.to_bytes(2, 'little')
        elif self.arg_type == TransactionArgumentType.U32:
            result += self.value.to_bytes(4, 'little')
        elif self.arg_type == TransactionArgumentType.U256:
            result += self.value.to_bytes(32, 'little')
        elif self.arg_type == TransactionArgumentType.SERIALIZED:
            data = self.value if isinstance(self.value, bytes) else bytes.fromhex(self.value)
            result += encode_uleb128(len(data)) + data
        
        return result


@dataclass
class Script:
    code: bytes
    type_args: List['TypeTag']
    args: List[TransactionArgument]
    
    def encode(self) -> bytes:
        # Code
        result = encode_uleb128(len(self.code)) + self.code
        
        # Type arguments
        result += encode_uleb128(len(self.type_args))
        for ty_arg in self.type_args:
            result += ty_arg.encode()
        
        # Arguments
        result += encode_uleb128(len(self.args))
        for arg in self.args:
            result += arg.encode()
        
        return result
    
    def to_payload(self) -> bytes:
        """Encode as TransactionPayload::Script (variant 0)."""
        return bytes([0x00]) + self.encode()


# Helper functions for creating arguments
def arg_u8(value: int) -> TransactionArgument:
    return TransactionArgument(TransactionArgumentType.U8, value)

def arg_u64(value: int) -> TransactionArgument:
    return TransactionArgument(TransactionArgumentType.U64, value)

def arg_u128(value: int) -> TransactionArgument:
    return TransactionArgument(TransactionArgumentType.U128, value)

def arg_address(address: str) -> TransactionArgument:
    return TransactionArgument(TransactionArgumentType.ADDRESS, address)

def arg_bytes(data: bytes) -> TransactionArgument:
    return TransactionArgument(TransactionArgumentType.U8_VECTOR, data)

def arg_bool(value: bool) -> TransactionArgument:
    return TransactionArgument(TransactionArgumentType.BOOL, value)


# Example usage
def build_transfer_script(
    bytecode: bytes,
    recipient: str,
    amount: int
) -> Script:
    """Build a transfer script payload."""
    return Script(
        code=bytecode,
        type_args=[],
        args=[
            arg_address(recipient),
            arg_u64(amount),
        ],
    )


if __name__ == "__main__":
    # Example script bytecode (placeholder)
    bytecode = bytes.fromhex("a11ceb0b...")
    
    script = build_transfer_script(
        bytecode=bytecode,
        recipient="0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        amount=1_000_000,
    )
    
    payload_bytes = script.to_payload()
    print(f"Payload hex: {payload_bytes.hex()}")
```

### TypeScript

```typescript
enum TransactionArgumentType {
  U8 = 0,
  U64 = 1,
  U128 = 2,
  Address = 3,
  U8Vector = 4,
  Bool = 5,
  U16 = 6,
  U32 = 7,
  U256 = 8,
  Serialized = 9,
}

interface TransactionArgument {
  type: TransactionArgumentType;
  value: number | bigint | boolean | string | Uint8Array;
}

function encodeTransactionArgument(arg: TransactionArgument): Uint8Array {
  const parts: Uint8Array[] = [new Uint8Array([arg.type])];
  
  switch (arg.type) {
    case TransactionArgumentType.U8:
      parts.push(new Uint8Array([Number(arg.value)]));
      break;
      
    case TransactionArgumentType.U64: {
      const buf = new Uint8Array(8);
      const view = new DataView(buf.buffer);
      view.setBigUint64(0, BigInt(arg.value), true);
      parts.push(buf);
      break;
    }
    
    case TransactionArgumentType.U128: {
      const buf = new Uint8Array(16);
      let val = BigInt(arg.value);
      for (let i = 0; i < 16; i++) {
        buf[i] = Number(val & BigInt(0xff));
        val >>= BigInt(8);
      }
      parts.push(buf);
      break;
    }
    
    case TransactionArgumentType.Address:
      parts.push(encodeAddress(arg.value as string));
      break;
      
    case TransactionArgumentType.U8Vector: {
      const data = arg.value as Uint8Array;
      parts.push(encodeULEB128(data.length));
      parts.push(data);
      break;
    }
    
    case TransactionArgumentType.Bool:
      parts.push(new Uint8Array([arg.value ? 1 : 0]));
      break;
      
    case TransactionArgumentType.U16: {
      const buf = new Uint8Array(2);
      const view = new DataView(buf.buffer);
      view.setUint16(0, Number(arg.value), true);
      parts.push(buf);
      break;
    }
    
    case TransactionArgumentType.U32: {
      const buf = new Uint8Array(4);
      const view = new DataView(buf.buffer);
      view.setUint32(0, Number(arg.value), true);
      parts.push(buf);
      break;
    }
    
    case TransactionArgumentType.U256: {
      const buf = new Uint8Array(32);
      let val = BigInt(arg.value);
      for (let i = 0; i < 32; i++) {
        buf[i] = Number(val & BigInt(0xff));
        val >>= BigInt(8);
      }
      parts.push(buf);
      break;
    }
    
    case TransactionArgumentType.Serialized: {
      const data = arg.value as Uint8Array;
      parts.push(encodeULEB128(data.length));
      parts.push(data);
      break;
    }
  }
  
  return concatBytes(parts);
}

interface Script {
  code: Uint8Array;
  typeArgs: Uint8Array[];  // Pre-encoded TypeTags
  args: TransactionArgument[];
}

function encodeScript(script: Script): Uint8Array {
  const parts: Uint8Array[] = [];
  
  // Code
  parts.push(encodeULEB128(script.code.length));
  parts.push(script.code);
  
  // Type arguments
  parts.push(encodeULEB128(script.typeArgs.length));
  for (const typeArg of script.typeArgs) {
    parts.push(typeArg);
  }
  
  // Arguments
  parts.push(encodeULEB128(script.args.length));
  for (const arg of script.args) {
    parts.push(encodeTransactionArgument(arg));
  }
  
  return concatBytes(parts);
}

function encodeScriptPayload(script: Script): Uint8Array {
  const encoded = encodeScript(script);
  const result = new Uint8Array(1 + encoded.length);
  result[0] = 0x00;  // Script variant
  result.set(encoded, 1);
  return result;
}

// Helper functions
function argU64(value: bigint): TransactionArgument {
  return { type: TransactionArgumentType.U64, value };
}

function argAddress(address: string): TransactionArgument {
  return { type: TransactionArgumentType.Address, value: address };
}

function argBytes(data: Uint8Array): TransactionArgument {
  return { type: TransactionArgumentType.U8Vector, value: data };
}

function argBool(value: boolean): TransactionArgument {
  return { type: TransactionArgumentType.Bool, value };
}

// Utility
function concatBytes(arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

// Example usage
function buildTransferScript(
  bytecode: Uint8Array,
  recipient: string,
  amount: bigint
): Script {
  return {
    code: bytecode,
    typeArgs: [],
    args: [
      argAddress(recipient),
      argU64(amount),
    ],
  };
}

export {
  TransactionArgumentType,
  encodeTransactionArgument,
  encodeScript,
  encodeScriptPayload,
  argU64,
  argAddress,
  argBytes,
  argBool,
};
```

## Compiling Move Scripts

Scripts must be compiled to bytecode before use. Here's an example Move script:

```move
script {
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    
    fun main(sender: &signer, recipient: address, amount: u64) {
        coin::transfer<AptosCoin>(sender, recipient, amount);
    }
}
```

### Compilation

Using the Aptos CLI:

```bash
aptos move compile --named-addresses addr=0x1
```

The compiled bytecode is in `.mv` files.

## Script Validation Rules

1. **Valid Bytecode**: Must be valid Move bytecode
2. **Script Type**: Must be a script (not a module)
3. **First Parameter**: Must be `&signer` (injected by runtime)
4. **Type Arguments**: Must satisfy all type constraints
5. **Argument Count**: Must match script parameters (excluding signer)
6. **Argument Types**: Must match expected parameter types

## Script Signer Parameter

The first parameter of a Move script must be `&signer`. This is automatically provided by the runtime and represents the transaction sender.

```move
script {
    fun main(
        account: &signer,    // <-- Automatically injected
        amount: u64          // <-- Provided via TransactionArgument
    ) {
        // account is the transaction sender
    }
}
```

**Important**: Do NOT include the signer in `TransactionArgument` list.

## When to Use Scripts

### Advantages

1. **No deployment needed**: Code is in the transaction
2. **One-time logic**: Perfect for migration or admin tasks
3. **Atomic composition**: Combine multiple operations
4. **Prototyping**: Test logic before module deployment

### Disadvantages

1. **Higher gas cost**: Bytecode transmitted each time
2. **No code reuse**: Each transaction carries full code
3. **Limited arguments**: TransactionArgument is simpler than BCS
4. **Compilation required**: Need Move compiler toolchain

## Test Vector

### Input
- Script: Simple hello world (placeholder bytecode)
- Type args: none
- Arguments:
  - U64: `42`
  - Address: `0x1`

### Script Structure

```
00                              // Script variant
<bytecode_length>               // ULEB128 length
<bytecode>                      // Script bytecode
00                              // 0 type arguments
02                              // 2 arguments
01                              // U64 variant
2a00000000000000                // 42 as u64 LE
03                              // Address variant
0000000000000000000000000000000000000000000000000000000000000001
```

## Error Codes

| Error | Description |
|-------|-------------|
| `INVALID_SCRIPT` | Bytecode is not valid |
| `SCRIPT_TYPE_ERROR` | Not a script (is a module) |
| `TYPE_MISMATCH` | Argument type mismatch |
| `MISSING_DEPENDENCY` | Script depends on missing module |

## Related Documents

- [Payload Overview](01-payload-overview.md) - All payload types
- [Entry Function](02-entry-function.md) - Entry function specification
- [Move Types](04-move-types.md) - TypeTag encoding
