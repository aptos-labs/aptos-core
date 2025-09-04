# Velor Rosetta Implementation

This implementation is built for running a local proxy against
a local fullnode.  However, for testing purposes, this can be used
against an external REST endpoint.

## Architecture

[Rosetta](https://en.wikipedia.org/wiki/Rosetta_(software)) works as a sidecar to an Velor fullnode.  Rosetta then proxies the Rosetta standard
API calls to underlying Velor REST API calls and builds the appropriate data.  


## Running Rosetta

The `velor-rosetta` binary can run in three modes:
1. `online` -> This runs a local fullnode and blocks the Velor REST API from outside access, using it only as a local proxy for Rosetta APIs.
2. `offline` -> This runs a Rosetta server that is not connected to the blockchain.  Only commands listed as `offline` work with this mode.
3. `online-remote` -> This runs a Rosetta instance that connects to a remote fullnode e.g. a public fullnode.  Please keep in mind that since this proxies APIs, it can fail due to throttling and network errors between the servers.


## Features supported

### Balances
* Only the native `APT` is supported.
* Staking balances are also supported, with the sub-account with the name of `stake`, and only with `0x1::staking_contract` stake pools.
* Balances are loaded from the live API `get_account_resources`; and if the `block` has been pruned, it will error out.
* All balances are provided the balance at the end of a `block`.


### Blocks

Blocks support reading the following operations:

 * `create_account` -> When an account is created.
 * `withdraw` -> When a balance is withdrawn from an account.
 * `deposit` -> When a balance is deposited to an account.
 * `fee` -> The gas fee associated with running a transaction.
 * `set_operator` -> Switching a `0x1::staking_contract` operator to a new operator.
 * `set_voter` -> Switching a `0x1::staking_contract` voter to a new voter.

Here are some exceptions:

 * Not all operators can be parsed from `failed transactions`.
 * Set operator will have the stake balance in its metadata.

All transactions are parsed from the events provided by the VelorFramework.  There are a few exceptions to this that use the transaction payload, but only for errors.

Block hash is `<chain_id>:<block_height>` and not actually a hash.

### Constructing transactions

More specifics can be found here: https://www.rosetta-api.org/docs/flow.html#construction-api

All inputs to the API must be done in the `ConstructionPreprocessRequest`.  This allows you to set
the sequence number, expiry time, gas parameters, and the public keys to sign the transaction.

Note: Currently only single signer is supported at this time.

The general flow is that you provide these inputs and follow the flow of APIs.  The Metadata call
will do a simulation of the transaction and tell you the estimated gas fee for that transaction. It
will also fail the transaction before paying gas if the transaction cannot work.  Once all the payloads
are built and combined, you must sign the transaction with the Ed25519 key that matches the PublicKey
provided in the `ConstructinoPreProcessRequest`.

#### Create Account
* Accounts can be created with just the `create_account` operation alone.

#### Transfers
* Transfers occur as a combination of a `withdraw` and a `deposit`.  This has the side effect of creating the receiver if it doesn't exist.
* Transfers support only APT at this moment.

#### Set Operator
* A staking contract stake pool can change its operator.
* If no operator is provided, it will attempt to find the first operator in the stake pool.

#### Set Voter
* A staking contract stake pool can chage its voter.
* If no operator is provided, it will attempt to find the first operator in the stake pool.

## Data types
All data types must hide `null` values from the output JSON.  Additionally, u64s must be
encoded as strings in any metadata fields.

## Errors

All errors are 500s and have error codes that are static and must not change.  To add more errors,
add new codes and associated data.  The error details must not show in the network options call and
are all provided as Option<String> for that reason.

## Time

All timestamps are valid except for the first two timestamps.  These are generally 0, so they are set to
January 1st 2000 if they're older than that.

## Mempool APIs

Mempool APIs are currently not supported.

## CLI testing

The [Rosetta CLI](https://www.rosetta-api.org/docs/rosetta_cli.html) can be run with the [rosetta_cli.json](./rosetta_cli.json)
file to run the automated checks.  Additionally, the [velor.ros](./velor.ros)
file uses the Rosetta CLI DSL to describe the possible operations that
can be run.

Additionally, we have our `velor-rosetta-cli` crate for local testing.
