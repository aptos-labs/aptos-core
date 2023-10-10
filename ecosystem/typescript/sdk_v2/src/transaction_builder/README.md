

This is a different problem than anything you've tried to tackle before.

Let's build things up from the foundation and then work our way up.

The foundational patterns we'll use will be the BCS serialization/deserialization. We will build off of that to create more complex developer capabilities.

1. BCS serialization/deserialization (ser/de) of primitives with the Serializer class
2. ser/de of entry function args, script args, and structs
3. ser/de of AccountAuthenticators (ed25519 and multied25519), payloads, and raw transactions
4. 




I think you should use BCS for *everything*.

We'll tag everything we serialize with an enum value. Maybe fit it into the hierarchical values:

1. Transactions
2. 

We use serialization for:



Let's break down the 