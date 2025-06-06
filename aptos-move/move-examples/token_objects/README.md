# Token Object Examples

The Aptos Digital Asset (DA) standard provides a modern approach to NFTs built on Move Objects. This directory contains examples demonstrating various token object implementations and use cases, showcasing the flexibility and composability of Aptos token objects.

Token objects in Aptos have their own addresses and can own resources, including other token objects. This enables powerful patterns like composable NFTs, soulbound tokens, dynamic tokens, and more advanced token behaviors not easily achievable in other blockchain ecosystems.

## Examples

* **Hero**: Demonstrates composable NFTs where Hero tokens can equip Weapon and Gem tokens. Shows object ownership hierarchies and transfer between objects.

* **Ambassador**: Implements soulbound tokens that cannot be transferred once minted. Features level progression with automatic rank updates and dynamic URI changes based on token state.

* **Token Lockup**: Shows how to implement time-based transfer restrictions, preventing tokens from being transferred for 7 days after acquisition.

* **Guild**: Demonstrates hierarchical token structures with guild and member tokens, authorization patterns, and whitelist-based token creation.

* **Knight**: Shows interaction between NFTs (Knight tokens) and fungible assets (Food tokens), with state changes and property updates.

## Key Concepts Demonstrated

These examples showcase several important token object patterns:

- **Object Ownership**: Tokens owning other tokens (Hero, Guild)
- **Custom Transfer Logic**: Implementing specific transfer rules (Token Lockup, Ambassador)
- **Soulbound Tokens**: Non-transferable tokens (Ambassador)
- **Dynamic Metadata**: Tokens that change properties and appearance (Ambassador, Knight)
- **Ref-based Capabilities**: Using different refs to control token behaviors
- **Fungible & Non-Fungible Interaction**: How fungible and non-fungible tokens can interact (Knight)

## Getting Started

To run these examples locally:

1. Clone the Aptos repository: `git clone https://github.com/aptos-labs/aptos-core.git`
2. Navigate to an example directory: `cd aptos-core/aptos-move/move-examples/token_objects/hero`
3. Compile and test: `aptos move compile --named-addresses hero=0x1`
4. Run tests: `aptos move test --named-addresses hero=0x1`

To deploy to testnet, follow the [Aptos developer documentation for deployment steps of your first move module](https://aptos.dev/en/build/guides/first-move-module).

## Suggested Learning Path

If you're new to Aptos token objects, we recommend reviewing the examples in this order:

1. **Hero**: Basic token structure and object ownership
2. **Token Lockup**: Simple token transfer restrictions
3. **Ambassador**: Soulbound tokens with dynamic properties
4. **Knight**: Interaction between different token types
5. **Guild**: Complex hierarchical token systems

## Additional Resources

- [Aptos Digital Asset Documentation](https://aptos.dev/en/build/smart-contracts/digital-asset)
- [Aptos Standards](https://aptos.dev/en/build/smart-contracts/aptos-standards)
- [Move Objects Documentation](https://aptos.dev/en/build/smart-contracts/objects)