# Move Type Encoding Specification

> **Version:** 1.0.0  
> **Last Updated:** January 28, 2026

## Overview

This document specifies the BCS encoding of Move types used in transaction payloads, including `TypeTag`, `StructTag`, `ModuleId`, and `Identifier`.

## TypeTag

`TypeTag` represents Move type parameters. It's used for generic type arguments in entry functions and scripts.

```rust
pub enum TypeTag {
    Bool,                  // Variant 0
    U8,                    // Variant 1
    U64,                   // Variant 2
    U128,                  // Variant 3
    Address,               // Variant 4
    Signer,                // Variant 5
    Vector(Box<TypeTag>),  // Variant 6
    Struct(Box<StructTag>),// Variant 7
    U16,                   // Variant 8
    U32,                   // Variant 9
    U256,                  // Variant 10
}
```

### BCS Layout

| Variant | Index | Payload |
|---------|-------|---------|
| Bool | 0x00 | none |
| U8 | 0x01 | none |
| U64 | 0x02 | none |
| U128 | 0x03 | none |
| Address | 0x04 | none |
| Signer | 0x05 | none |
| Vector | 0x06 | TypeTag (recursive) |
| Struct | 0x07 | StructTag |
| U16 | 0x08 | none |
| U32 | 0x09 | none |
| U256 | 0x0a | none |

### Examples

```
// TypeTag::Bool
00

// TypeTag::U64
02

// TypeTag::Address
04

// TypeTag::Vector(TypeTag::U8)  -- vector<u8>
06 01

// TypeTag::Vector(TypeTag::Address)  -- vector<address>
06 04

// TypeTag::Struct(StructTag for 0x1::aptos_coin::AptosCoin)
07 <struct_tag_bytes>
```

## StructTag

`StructTag` identifies a specific Move struct type.

```rust
pub struct StructTag {
    pub address: AccountAddress,   // 32 bytes
    pub module: Identifier,        // Module name
    pub name: Identifier,          // Struct name
    pub type_args: Vec<TypeTag>,   // Generic type parameters
}
```

### BCS Layout

```
+------------------------------------------+
|              StructTag                   |
+------------------------------------------+
| address: [u8; 32]                        |
+------------------------------------------+
| module: Identifier                       |
|   +-- length: ULEB128                    |
|   +-- bytes: [u8; length]                |
+------------------------------------------+
| name: Identifier                         |
|   +-- length: ULEB128                    |
|   +-- bytes: [u8; length]                |
+------------------------------------------+
| type_args: Vec<TypeTag>                  |
|   +-- count: ULEB128                     |
|   +-- [TypeTag; count]                   |
+------------------------------------------+
```

### Examples

#### Simple Struct: `0x1::aptos_coin::AptosCoin`

```
// address (0x1, 32 bytes)
0000000000000000000000000000000000000000000000000000000000000001
// module name length (10 = "aptos_coin")
0a
// module name
6170746f735f636f696e
// struct name length (9 = "AptosCoin")
09
// struct name
4170746f73436f696e
// type_args count (0)
00
```

#### Generic Struct: `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>`

```
// address (0x1)
0000000000000000000000000000000000000000000000000000000000000001
// module name length (4 = "coin")
04
// module name
636f696e
// struct name length (9 = "CoinStore")
09
// struct name
436f696e53746f7265
// type_args count (1)
01
// type_arg[0]: TypeTag::Struct(AptosCoin)
07 <AptosCoin StructTag bytes>
```

## ModuleId

`ModuleId` identifies a Move module.

```rust
pub struct ModuleId {
    pub address: AccountAddress,  // 32 bytes
    pub name: Identifier,         // Module name
}
```

### BCS Layout

```
+------------------------------------------+
|               ModuleId                   |
+------------------------------------------+
| address: [u8; 32]                        |
+------------------------------------------+
| name: Identifier                         |
|   +-- length: ULEB128                    |
|   +-- bytes: [u8; length]                |
+------------------------------------------+
```

### Example: `0x1::aptos_account`

```
// address (0x1, 32 bytes)
0000000000000000000000000000000000000000000000000000000000000001
// name length (13 = "aptos_account")
0d
// name bytes
6170746f735f6163636f756e74
```

## Identifier

An `Identifier` is a valid Move identifier string.

### Validation Rules

1. First character: letter (`a-z`, `A-Z`) or underscore (`_`)
2. Subsequent characters: letters, digits (`0-9`), or underscores
3. Maximum length: 128 characters
4. Not a reserved keyword

### BCS Layout

```
| length (ULEB128) | bytes (UTF-8) |
```

### Examples

```
// "transfer" (8 characters)
08 7472616e73666572

// "aptos_account" (13 characters)
0d 6170746f735f6163636f756e74

// "CoinStore" (9 characters)
09 436f696e53746f7265
```

## Code Examples

### Rust

```rust
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};

/// Create TypeTag for AptosCoin
fn apt_type_tag() -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("aptos_coin").unwrap(),
        name: Identifier::new("AptosCoin").unwrap(),
        type_args: vec![],
    }))
}

/// Create TypeTag for CoinStore<T>
fn coin_store_type_tag(coin_type: TypeTag) -> TypeTag {
    TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("coin").unwrap(),
        name: Identifier::new("CoinStore").unwrap(),
        type_args: vec![coin_type],
    }))
}

/// Create TypeTag for vector<u8>
fn vector_u8_type_tag() -> TypeTag {
    TypeTag::Vector(Box::new(TypeTag::U8))
}

/// Parse a type tag from string
fn parse_type_tag(s: &str) -> Result<TypeTag, anyhow::Error> {
    // This would use a parser - simplified example
    match s {
        "bool" => Ok(TypeTag::Bool),
        "u8" => Ok(TypeTag::U8),
        "u64" => Ok(TypeTag::U64),
        "u128" => Ok(TypeTag::U128),
        "address" => Ok(TypeTag::Address),
        "u16" => Ok(TypeTag::U16),
        "u32" => Ok(TypeTag::U32),
        "u256" => Ok(TypeTag::U256),
        s if s.starts_with("vector<") => {
            // Parse inner type
            let inner = &s[7..s.len()-1];
            Ok(TypeTag::Vector(Box::new(parse_type_tag(inner)?)))
        }
        _ => {
            // Parse as struct type
            parse_struct_tag(s).map(|st| TypeTag::Struct(Box::new(st)))
        }
    }
}
```

### Python

```python
from dataclasses import dataclass
from typing import List, Optional
from enum import IntEnum

class TypeTagVariant(IntEnum):
    BOOL = 0
    U8 = 1
    U64 = 2
    U128 = 3
    ADDRESS = 4
    SIGNER = 5
    VECTOR = 6
    STRUCT = 7
    U16 = 8
    U32 = 9
    U256 = 10


@dataclass
class StructTag:
    address: str
    module: str
    name: str
    type_args: List['TypeTag']
    
    def encode(self) -> bytes:
        result = encode_address(self.address)
        result += encode_identifier(self.module)
        result += encode_identifier(self.name)
        
        # Type arguments
        result += encode_uleb128(len(self.type_args))
        for ty_arg in self.type_args:
            result += ty_arg.encode()
        
        return result


@dataclass
class TypeTag:
    variant: TypeTagVariant
    inner: Optional['TypeTag'] = None      # For Vector
    struct_tag: Optional[StructTag] = None # For Struct
    
    def encode(self) -> bytes:
        result = bytes([self.variant])
        
        if self.variant == TypeTagVariant.VECTOR:
            result += self.inner.encode()
        elif self.variant == TypeTagVariant.STRUCT:
            result += self.struct_tag.encode()
        
        return result
    
    @staticmethod
    def bool() -> 'TypeTag':
        return TypeTag(TypeTagVariant.BOOL)
    
    @staticmethod
    def u8() -> 'TypeTag':
        return TypeTag(TypeTagVariant.U8)
    
    @staticmethod
    def u64() -> 'TypeTag':
        return TypeTag(TypeTagVariant.U64)
    
    @staticmethod
    def u128() -> 'TypeTag':
        return TypeTag(TypeTagVariant.U128)
    
    @staticmethod
    def address() -> 'TypeTag':
        return TypeTag(TypeTagVariant.ADDRESS)
    
    @staticmethod
    def u16() -> 'TypeTag':
        return TypeTag(TypeTagVariant.U16)
    
    @staticmethod
    def u32() -> 'TypeTag':
        return TypeTag(TypeTagVariant.U32)
    
    @staticmethod
    def u256() -> 'TypeTag':
        return TypeTag(TypeTagVariant.U256)
    
    @staticmethod
    def vector(inner: 'TypeTag') -> 'TypeTag':
        return TypeTag(TypeTagVariant.VECTOR, inner=inner)
    
    @staticmethod
    def struct(struct_tag: StructTag) -> 'TypeTag':
        return TypeTag(TypeTagVariant.STRUCT, struct_tag=struct_tag)


# Common type tags
def aptos_coin_type() -> TypeTag:
    """Create TypeTag for 0x1::aptos_coin::AptosCoin"""
    return TypeTag.struct(StructTag(
        address="0x1",
        module="aptos_coin",
        name="AptosCoin",
        type_args=[],
    ))


def coin_store_type(coin_type: TypeTag) -> TypeTag:
    """Create TypeTag for 0x1::coin::CoinStore<T>"""
    return TypeTag.struct(StructTag(
        address="0x1",
        module="coin",
        name="CoinStore",
        type_args=[coin_type],
    ))


def parse_type_tag(s: str) -> TypeTag:
    """Parse a type tag from string representation."""
    s = s.strip()
    
    # Primitive types
    primitives = {
        "bool": TypeTag.bool,
        "u8": TypeTag.u8,
        "u64": TypeTag.u64,
        "u128": TypeTag.u128,
        "address": TypeTag.address,
        "u16": TypeTag.u16,
        "u32": TypeTag.u32,
        "u256": TypeTag.u256,
    }
    
    if s in primitives:
        return primitives[s]()
    
    # Vector type
    if s.startswith("vector<") and s.endswith(">"):
        inner_str = s[7:-1]
        return TypeTag.vector(parse_type_tag(inner_str))
    
    # Struct type: address::module::name or address::module::name<type_args>
    return parse_struct_type_tag(s)


def parse_struct_type_tag(s: str) -> TypeTag:
    """Parse a struct type tag from string."""
    # Handle generic types
    type_args = []
    if '<' in s:
        base, args_str = s.split('<', 1)
        args_str = args_str[:-1]  # Remove trailing >
        type_args = parse_type_args(args_str)
    else:
        base = s
    
    # Parse address::module::name
    parts = base.split("::")
    if len(parts) != 3:
        raise ValueError(f"Invalid struct type: {s}")
    
    return TypeTag.struct(StructTag(
        address=parts[0],
        module=parts[1],
        name=parts[2],
        type_args=type_args,
    ))


def parse_type_args(s: str) -> List[TypeTag]:
    """Parse comma-separated type arguments, handling nested generics."""
    args = []
    depth = 0
    current = ""
    
    for char in s:
        if char == '<':
            depth += 1
            current += char
        elif char == '>':
            depth -= 1
            current += char
        elif char == ',' and depth == 0:
            args.append(parse_type_tag(current.strip()))
            current = ""
        else:
            current += char
    
    if current.strip():
        args.append(parse_type_tag(current.strip()))
    
    return args


# Example usage
if __name__ == "__main__":
    # Create APT type tag
    apt = aptos_coin_type()
    print(f"APT type tag: {apt.encode().hex()}")
    
    # Create CoinStore<APT>
    coin_store = coin_store_type(apt)
    print(f"CoinStore<APT>: {coin_store.encode().hex()}")
    
    # Parse from string
    parsed = parse_type_tag("0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>")
    print(f"Parsed: {parsed.encode().hex()}")
    
    # Vector types
    vector_u8 = TypeTag.vector(TypeTag.u8())
    print(f"vector<u8>: {vector_u8.encode().hex()}")
```

### TypeScript

```typescript
enum TypeTagVariant {
  Bool = 0,
  U8 = 1,
  U64 = 2,
  U128 = 3,
  Address = 4,
  Signer = 5,
  Vector = 6,
  Struct = 7,
  U16 = 8,
  U32 = 9,
  U256 = 10,
}

interface StructTag {
  address: string;
  module: string;
  name: string;
  typeArgs: TypeTag[];
}

interface TypeTag {
  variant: TypeTagVariant;
  inner?: TypeTag;      // For Vector
  structTag?: StructTag; // For Struct
}

function encodeStructTag(structTag: StructTag): Uint8Array {
  const parts: Uint8Array[] = [];
  
  // Address
  parts.push(encodeAddress(structTag.address));
  
  // Module name
  parts.push(encodeIdentifier(structTag.module));
  
  // Struct name
  parts.push(encodeIdentifier(structTag.name));
  
  // Type arguments
  parts.push(encodeULEB128(structTag.typeArgs.length));
  for (const typeArg of structTag.typeArgs) {
    parts.push(encodeTypeTag(typeArg));
  }
  
  return concatBytes(parts);
}

function encodeTypeTag(typeTag: TypeTag): Uint8Array {
  const parts: Uint8Array[] = [new Uint8Array([typeTag.variant])];
  
  if (typeTag.variant === TypeTagVariant.Vector && typeTag.inner) {
    parts.push(encodeTypeTag(typeTag.inner));
  } else if (typeTag.variant === TypeTagVariant.Struct && typeTag.structTag) {
    parts.push(encodeStructTag(typeTag.structTag));
  }
  
  return concatBytes(parts);
}

// Type tag constructors
const TypeTags = {
  bool: (): TypeTag => ({ variant: TypeTagVariant.Bool }),
  u8: (): TypeTag => ({ variant: TypeTagVariant.U8 }),
  u64: (): TypeTag => ({ variant: TypeTagVariant.U64 }),
  u128: (): TypeTag => ({ variant: TypeTagVariant.U128 }),
  address: (): TypeTag => ({ variant: TypeTagVariant.Address }),
  u16: (): TypeTag => ({ variant: TypeTagVariant.U16 }),
  u32: (): TypeTag => ({ variant: TypeTagVariant.U32 }),
  u256: (): TypeTag => ({ variant: TypeTagVariant.U256 }),
  
  vector: (inner: TypeTag): TypeTag => ({
    variant: TypeTagVariant.Vector,
    inner,
  }),
  
  struct: (structTag: StructTag): TypeTag => ({
    variant: TypeTagVariant.Struct,
    structTag,
  }),
};

// Common type tags
function aptosCoinType(): TypeTag {
  return TypeTags.struct({
    address: '0x1',
    module: 'aptos_coin',
    name: 'AptosCoin',
    typeArgs: [],
  });
}

function coinStoreType(coinType: TypeTag): TypeTag {
  return TypeTags.struct({
    address: '0x1',
    module: 'coin',
    name: 'CoinStore',
    typeArgs: [coinType],
  });
}

// Parse type tag from string
function parseTypeTag(s: string): TypeTag {
  s = s.trim();
  
  // Primitive types
  const primitives: Record<string, () => TypeTag> = {
    'bool': TypeTags.bool,
    'u8': TypeTags.u8,
    'u64': TypeTags.u64,
    'u128': TypeTags.u128,
    'address': TypeTags.address,
    'u16': TypeTags.u16,
    'u32': TypeTags.u32,
    'u256': TypeTags.u256,
  };
  
  if (s in primitives) {
    return primitives[s]();
  }
  
  // Vector type
  if (s.startsWith('vector<') && s.endsWith('>')) {
    const inner = s.slice(7, -1);
    return TypeTags.vector(parseTypeTag(inner));
  }
  
  // Struct type
  return parseStructTypeTag(s);
}

function parseStructTypeTag(s: string): TypeTag {
  let typeArgs: TypeTag[] = [];
  let base = s;
  
  // Handle generic types
  const genericStart = s.indexOf('<');
  if (genericStart !== -1) {
    base = s.slice(0, genericStart);
    const argsStr = s.slice(genericStart + 1, -1);
    typeArgs = parseTypeArgs(argsStr);
  }
  
  // Parse address::module::name
  const parts = base.split('::');
  if (parts.length !== 3) {
    throw new Error(`Invalid struct type: ${s}`);
  }
  
  return TypeTags.struct({
    address: parts[0],
    module: parts[1],
    name: parts[2],
    typeArgs,
  });
}

function parseTypeArgs(s: string): TypeTag[] {
  const args: TypeTag[] = [];
  let depth = 0;
  let current = '';
  
  for (const char of s) {
    if (char === '<') {
      depth++;
      current += char;
    } else if (char === '>') {
      depth--;
      current += char;
    } else if (char === ',' && depth === 0) {
      args.push(parseTypeTag(current.trim()));
      current = '';
    } else {
      current += char;
    }
  }
  
  if (current.trim()) {
    args.push(parseTypeTag(current.trim()));
  }
  
  return args;
}

// Example usage
const apt = aptosCoinType();
console.log('APT type tag:', Buffer.from(encodeTypeTag(apt)).toString('hex'));

const coinStore = coinStoreType(apt);
console.log('CoinStore<APT>:', Buffer.from(encodeTypeTag(coinStore)).toString('hex'));

const parsed = parseTypeTag('0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>');
console.log('Parsed:', Buffer.from(encodeTypeTag(parsed)).toString('hex'));

export {
  TypeTagVariant,
  TypeTags,
  encodeTypeTag,
  encodeStructTag,
  parseTypeTag,
  aptosCoinType,
  coinStoreType,
};
```

## Common Type Tags Reference

### Primitive Types

| Type | Variant | BCS |
|------|---------|-----|
| `bool` | 0 | `00` |
| `u8` | 1 | `01` |
| `u64` | 2 | `02` |
| `u128` | 3 | `03` |
| `address` | 4 | `04` |
| `signer` | 5 | `05` |
| `u16` | 8 | `08` |
| `u32` | 9 | `09` |
| `u256` | 10 | `0a` |

### Vector Types

| Type | BCS |
|------|-----|
| `vector<u8>` | `06 01` |
| `vector<u64>` | `06 02` |
| `vector<address>` | `06 04` |
| `vector<vector<u8>>` | `06 06 01` |

### Common Struct Types

| Type | Address | Module | Name |
|------|---------|--------|------|
| `AptosCoin` | `0x1` | `aptos_coin` | `AptosCoin` |
| `CoinStore<T>` | `0x1` | `coin` | `CoinStore` |
| `Account` | `0x1` | `account` | `Account` |
| `Object<T>` | `0x1` | `object` | `Object` |
| `String` | `0x1` | `string` | `String` |
| `Option<T>` | `0x1` | `option` | `Option` |

## String Type Encoding

The Move `String` type (`0x1::string::String`) is commonly used. In BCS, it's encoded as:

```
| length (ULEB128) | UTF-8 bytes |
```

This is the same as `vector<u8>` but with UTF-8 validation.

## Option Type Encoding

`Option<T>` is encoded as a vector with 0 or 1 elements:

```
// None
00

// Some(value)
01 <BCS(value)>
```

## Test Vectors

### AptosCoin TypeTag

```
// TypeTag::Struct
07
// StructTag
// - address (0x1)
0000000000000000000000000000000000000000000000000000000000000001
// - module (aptos_coin)
0a 6170746f735f636f696e
// - name (AptosCoin)
09 4170746f73436f696e
// - type_args (empty)
00
```

### vector<u8> TypeTag

```
// TypeTag::Vector
06
// Inner: TypeTag::U8
01
```

### CoinStore<AptosCoin> TypeTag

```
// TypeTag::Struct
07
// StructTag
// - address (0x1)
0000000000000000000000000000000000000000000000000000000000000001
// - module (coin)
04 636f696e
// - name (CoinStore)
09 436f696e53746f7265
// - type_args count (1)
01
// - type_args[0]: AptosCoin
07 <AptosCoin StructTag>
```

## Related Documents

- [Payload Overview](01-payload-overview.md) - All payload types
- [Entry Function](02-entry-function.md) - Using TypeTag in entry functions
- [Script Payload](03-script-payload.md) - Using TypeTag in scripts
