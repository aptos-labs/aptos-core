## Limitations

1. Transactions must have a sequence number larger than the current account sequence number for them to be accepted by the mempool.
2. Transactions with the same sequence number but different payloads cannot be resubmitted.
3. Transactions get removed from mempool if their expire timestamps reached. Mempool also has a system wide TTL (defaults to 10 minutes) that are used to purge transactions.
4. Each account can have a maximum 100 (default value) transactions in mempool.
5. Only transactions with consecutive increasing sequence numbers from the current account sequence number are ready for execution.
6. `transactions/batch` has a `api.max_submit_transaction_batch_size` config , by default set to 10. (`max_submit_transaction_batch_size` can be configured by different node holders)

## Example

To submit 1000 transactions in a batch, we need to create 100 (`total_transactions` = 1000 / `max_batch_size` = 10) transaction buffers.
For each buffer we create a signed-ready-to-be-submitted transaction.
To create each transaction, we first need to fetch the current sender sequence number and to `maintain a local sequence number` that would be increased for every new transaction creation.
We then submit each buffer to the `/transactions/batch` endpoint.
Then we need to handle any possible error (see errors at the bottom)

## implementation

For initial implementation, we start with implementing transactions for the same account
