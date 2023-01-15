# Working with PropertyMap Off-Chain

Token module uses property map [module](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/sources/property_map.move) to store on-chain properties of token. PropertyMap maps a string key to a property value on-chain, which stores the value in BCS serialized format and its type. Currently, only primitive types (bool, u8, u64, u128, address and String) are supported in property map. 

With both the value and typing, property map can be used to read and write values of different types in a map data structure on-chain. 

### Read and write property map using TS SDK

Our SDK supports read and write property map from typescript directly. It saves you from knowing the details for BCS serialization. 

To generate the BCS serialized data for creating token on-chain, please use `getPropertyValueRaw`

This method handle the serialization of typescript data into BCS format.

To read property map returned by API, please use `deserializePropertyMap` , it deserializes the data from API and create TS class property map

### Reference:

- TS property map serde for more details [link](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/utils/property_map_serde.ts)
- Examples using property map serde [link](https://github.com/aptos-labs/aptos-core/blob/main/ecosystem/typescript/sdk/src/utils/property_map_serde.test.ts)

###