# DeFi Examples for Move Prover

This package includes simplified versions of decentralized finance (DeFi) contracts written in Move, designed for specification and verification using the Move Prover.

1. **reserve.move**: A straightforward reserve-backed currency system. This contract defines a currency backed by a reserve of another currency, allowing users to mint and burn the currency as well as deposit and withdraw the reserve currency. It ensures that the total supply of the currency is always fully backed by the reserve. Additionally, it includes intentionally incorrect implementations of the mint and burn functions to demonstrate the Move Prover's capabilities.

2. **uniswap.move**: A basic implementation of the Uniswap v1 automated market maker. This contract defines a pair of tokens, enabling users to swap one token for the other and add or remove liquidity from the pool. It specifies the [constant product invariant](https://docs.uniswap.org/contracts/v2/concepts/protocol-overview/glossary#constant-product-formula) of the Uniswap protocol, which ensures that the product of the token balances remains constant (more precisely, non-decreasing). The contract also contains the flawed algorithm of the swap function (found in the Uniswap v1's whitepaper) to showcase the Move Prover's capabilities.
