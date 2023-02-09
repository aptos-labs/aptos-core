# Working with PropertyMap Off-Chain

[The Aptos token standard](https://aptos.dev/concepts/coin-and-token/aptos-token/) uses the [property map module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/property_map.move) to store on-chain properties of tokens. PropertyMap maps a string key to a property value on-chain, which stores the value in Binary Canonical Serialization (BCS) format and its type. Currently, only primitive types (bool, u8, u64, u128, address and String) are supported in property map.

With both the value and typing, property map can be used to read and write values of heterogeneous types in a map data structure on-chain. 

### Read and write property map using TS SDK

Our TypeScript SDK supports reading and written to property map from TypeScript directly. It saves you from knowing the details of BCS serialization. 

To generate the BCS data for creating tokens on-chain, use `getPropertyValueRaw`. This method handles the serialization of TypeScript data into BCS format.

To read property maps returned by API, use `deserializePropertyMap`; it deserializes the data from API and create the TypeScript class property map.

### Reference:
- [property_map_serde.ts](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/utils/property_map_serde.ts) - TypeScript property map.
- [property_map_serde.test.ts](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/utils/property_map_serde.test.ts) - Examples using property map serde.
