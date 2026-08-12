# Introduction
This module is a reference implementation of a managed USDK stablecoin with:

1. Explicit entry-function initialization (this example intentionally does not use `init_module`).
2. Per-minter mint allowances stored in an address-keyed table.
3. Self-burn for minters and separately authorized confiscation for compliance recovery.
4. Denylisting with direct-owner and root-owner enforcement for nested stores.
5. Pausing of mint, burn, and transfer operations.
6. Two-step rotation for the master-minter, pauser, denylister, and confiscator roles.

USDK uses 6 decimals, matching the convention used by USDC/USDT. See `DESIGN.md`
for the security model and pause policy.

## Pause policy

While paused, value movement is blocked. Recovery operations remain available:
minter revocation, allowance decreases, denylisting, undenylisting, confiscation,
and role rotation. Adding minters and increasing allowances are blocked until the
stablecoin is unpaused.

## Deployment

Currently only available in devnet due to the requirement of AIP 73:
<https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-73.md>.

1. Create a devnet profile with `aptos init --profile devnet`.
2. Publish the package. Replace each `devnet` named address with the intended role address:

   ```bash
   aptos move publish \
     --named-addresses stablecoin=devnet,master_minter=devnet,pauser=devnet,denylister=devnet,confiscator=devnet \
     --profile devnet
   ```

3. Initialize the asset explicitly from the publishing account:

   ```bash
   aptos move run \
     --function-id 'devnet::usdk::initialize' \
     --profile devnet
   ```

4. Grant a minter allowance. The master-minter account must sign this transaction:

   ```bash
   aptos move run \
     --function-id 'devnet::usdk::add_minter' \
     --args address:0xMINTER u64:100000000 \
     --profile devnet
   ```

5. Mint within that allowance. With 6 decimals, `u64:100000000` mints 100 USDK:

   ```bash
   aptos move run \
     --function-id 'devnet::usdk::mint' \
     --args address:0xRECIPIENT u64:100000000 \
     --profile devnet
   ```

## Running tests

```bash
aptos move test --dev
aptos move test --coverage --dev
aptos move coverage summary --dev
```

The package targets at least 90% Move executable-code coverage.
