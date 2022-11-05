# Speed sending transactions for a single account

## Summary

The simplest way to send transactions for a single account is sequential sending. Submitting a transaction and waiting for it to be executed before submitting the next one is very inefficient. Since the Aptos blockchain executes transactions in blocks, transactions that are in ready state in mempool might get executed in the same block. Therefore, maximizing the number of transactions standby in mempool is the key to high throughput.

## Constraints for single account txn submission

Several mempool constraints need to be considered when designing the transaction submission software. Not only these constraints decide if transactions can be ready for execution in the mempool parking lot, they also affect the error handling logic. See below details for the major constraints:

1. Transactions must have a sequence number larger than the current account's sequence number for the transactions to be accepted by the mempool.
2. Transactions with the same sequence number but different payloads cannot be resubmitted.
3. Transactions get removed from mempool if their expire timestamps reached. Mempool also pruge transactions once a system TTL (defaults to 10 minutes) is reached.
4. Each account can have a maximum 100 (default value) transactions in mempool.

## Design

- Local state management - To avoid fetching sequence number from the chain for every single request, clients need to maintain a local sequence number. With such local sequence numbers, clients do not have to wait for the chain to update the chain state. Even further, clients do not need to wait for mempool to respond to the requests.
- Edge case handling
  - If local sequence number is lagging behind, clients need to refetch the sequence number from the chain. This is to solve the problem described by constraint 1.
  - If local sequence number is identical as the account sequence number on chain, clients need to advance their local sequence numbers. This is to solve the problem described by constraint 2.
  - If the chain is congested, transactions might hit TTLs and get removed from mempool. Clients need to poll the outstanding transactions. If transactions are not found, we know the transactions are TTLed. This is to deal with the problem described in constraint 3.
- Back pressure and requests rate control - Because of constraint 4, each account has a finite parking lot space for pending transactions. Once the mempool's parking lot is full, clients should sleep some time before continue sending more requests.

## Code

- fast_transaction_client.ts - A client that handles local state management and edge case handling. fast_transaction_client is only good for single account transaction submission. Extending it to support multiple accounts is not hard.
- batch_submission.ts - Demonstrates how to send requests as fast as possible. Clients break down the input transactions into smaller batches. Each batch submits 50 transactions. This is to avoid hitting the mempool capacity limit too quickly. Every submission request doesn’t wait for mempool to respond to maximize the concurrency. The "mempool full" exception is used to slow down the sending speed.
- multiple_client_submission.ts - Demonstrates how multiple clients can send transactions for the same account in parallel. Since multiple clients race for updating the account’s on-chain sequence number, each client should be able to make local sequence number progress correctly under the conflicting, lagging and TTL cases.

## Acknowledgment

- Clients need to decide how to process the TTLed or failed transactions.
- Special care needs to be given for submitting transactions that have a dependency order.
