---
title: "Move Types"
slug: "typescript-sdk-move-types"
---

When developing on Aptos, and specifically working with the SDK, developers often need to handle Move types serialization and deserialization. Whether is to construct a transaction payload, build a raw transaction or read BCS data.

The SDK provides a convenient Move sub-classes to easily interact with move types to perform serialization or deserialization operations.
Each class has a `serialize`, `serializeForEntryFunction` and `serializeForScriptFunction` methods and a `deserialize` static class.

In addition, for complex types like `Vector` the SDK supports nested serialization and deserialization.

### Move primitive types

Classes to handle Move primitive types:

- U8
- U16
- U32
- U64
- U128
- U256
- Bool

```ts
const serializer = new Serializer();

const u8 = new U8(1);
u8.serialize(serializer);
u8.serializeForEntryFunction(serializer);
u8.serializeForScriptFunction(serializer);

const deserializer = new Deserializer();
U8.deserialize(deserializer);
```

### Move struct types

- MoveVector
- MoveString
- MoveOption

```ts
const serializer = new Serializer();

const moveString = new MoveString("hello world");
moveString.serialize(serializer);
moveString.serializeForEntryFunction(serializer);
moveString.serializeForScriptFunction(serializer);

const deserializer = new Deserializer();
MoveString.deserialize(deserializer);
```
