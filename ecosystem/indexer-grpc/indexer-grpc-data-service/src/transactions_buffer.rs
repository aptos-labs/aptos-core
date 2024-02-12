use aptos_protos::transaction::v1::Transaction;
use std::sync::Arc;
use tokio::sync::Mutex;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

const DEFAULT_BUFFER_SIZE: usize = 20_000;

pub enum BufferGetStatus {
    AheadOfBuffer,
    InBuffer(Vec<Arc<Transaction>>),
    BehindBuffer,
}

/// TransactionsBuffer is a circular buffer for storing the latest transactions in cache.
#[derive(Clone)]
pub struct TransactionsBuffer {
    data: Arc<Mutex<TransactionsBufferData>>,
}

struct TransactionsBufferData {
    internal_transactions: ConstGenericRingBuffer<Arc<Transaction>, DEFAULT_BUFFER_SIZE>,
    last_transaction_version: u64,
}

impl TransactionsBuffer {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(TransactionsBufferData {
                internal_transactions:  ConstGenericRingBuffer::new(),
                last_transaction_version: 0,
            })),
        }
    }

    // Push the transactions to the buffer.
    pub async fn push_transactions(&self, transactions: &[Arc<Transaction>]) -> anyhow::Result<()> {
        // Verify the transactions are consecutive.
        if !transactions.is_empty() {
            let mut last_version = transactions[0].version;
            for transaction in transactions.iter().skip(1) {
                if transaction.version != last_version + 1 {
                    anyhow::bail!("The transaction version is not consecutive. No update is made.");
                }
                last_version = transaction.version;
            }
        }

        let mut data = self.data.lock().await;
        for transaction in transactions {
            if data.internal_transactions.is_empty() {
                data.last_transaction_version = transaction.version;
                data.internal_transactions.push(transaction.clone());
            } else {
                // The transactions are already in the buffer.
                if transaction.version <= data.last_transaction_version {
                    continue;
                }

                // The transactions should be added consecutively.
                // If it happens, it's a serious error and the buffer is cleared.
                if transaction.version != data.last_transaction_version + 1 {
                    anyhow::bail!(
                        "The transaction version is not consecutive. Partial update is made."
                    );
                }

                data.internal_transactions.push(transaction.clone());
                data.last_transaction_version = transaction.version;
            }
        }
        Ok(())
    }

    // Get the transactions from the buffer.
    // The buffer covers the range [latest_transaction_version + 1 - BUFFER_SIZE, latest_transaction_version].
    pub async fn get_transactions(&self, starting_version: u64) -> BufferGetStatus {
        let data = self.data.lock().await;
        // Buffer is not filled yet.
        if data.internal_transactions.is_empty() {
            return BufferGetStatus::AheadOfBuffer;
        }
        if data.last_transaction_version < starting_version {
            return BufferGetStatus::AheadOfBuffer;
        }
        if data.last_transaction_version + 1 > starting_version + data.internal_transactions.len() as u64 {
            return BufferGetStatus::BehindBuffer;
        }

        let num_of_transactions = (data.last_transaction_version + 1 - starting_version) as usize;
        let end_index = data.internal_transactions.len();
        let start_index = end_index.saturating_sub(num_of_transactions);
        let result: Vec<Arc<Transaction>> = (start_index..end_index)
            .map(|i| data.internal_transactions[i].clone())
            .collect();
        BufferGetStatus::InBuffer(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_push_and_get() {
        let buffer = TransactionsBuffer::new();
        let transactions = vec![
            Arc::new(Transaction {
                version: 1,
                ..Default::default()
            }),
            Arc::new(Transaction {
                version: 2,
                ..Default::default()
            }),
            Arc::new(Transaction {
                version: 3,
                ..Default::default()
            }),
        ];
        buffer.push_transactions(&transactions).await.unwrap();
        let result = buffer.get_transactions(1).await;
        match result {
            BufferGetStatus::InBuffer(transactions) => {
                assert_eq!(transactions.len(), 3);
                assert_eq!(transactions[0].version, 1);
                assert_eq!(transactions[1].version, 2);
                assert_eq!(transactions[2].version, 3);
            }
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    async fn test_get_an_empty_buffer() {
        let buffer = TransactionsBuffer::new();
        let result = buffer.get_transactions(1).await;
        match result {
            BufferGetStatus::AheadOfBuffer => {}
            _ => panic!("Unexpected result"),
        }
        buffer.push_transactions(&[Arc::new(Transaction {
            version: 1,
            ..Default::default()
        })]).await.unwrap();
        let result = buffer.get_transactions(1).await;
        match result {
            BufferGetStatus::InBuffer(transactions) => {
                assert_eq!(transactions.len(), 1);
                assert_eq!(transactions[0].version, 1);
            },
            _ => panic!("Unexpected result"),
        }
    }
    #[tokio::test]
    async fn test_buffer_eviction() {
        let buffer = TransactionsBuffer::new();
        for i in 0..DEFAULT_BUFFER_SIZE {
            buffer
                .push_transactions(&[Arc::new(
                    Transaction {
                        version: i as u64,
                        ..Default::default()
                    }
                )])
            .await
            .unwrap();
        }
        let result = buffer.get_transactions(0).await;
        match result {
            BufferGetStatus::InBuffer(transactions) => {
                assert_eq!(transactions.len(), DEFAULT_BUFFER_SIZE);
                assert_eq!(transactions[0].version, 0);
                assert_eq!(transactions[DEFAULT_BUFFER_SIZE - 1].version, DEFAULT_BUFFER_SIZE as u64 - 1);
            },
            _ => panic!("Unexpected result"),
        }
        // Current buffer contains 0 to DEFAULT_BUFFER_SIZE - 1 versions.
        // Push DEFAULT_BUFFER_SIZE - 2, DEFAULT_BUFFER_SIZE - 1, DEFAULT_BUFFER_SIZE.
        // The buffer should evict 0 only.
        buffer
            .push_transactions(&[
                Arc::new(Transaction {
                    version: DEFAULT_BUFFER_SIZE as u64 - 2,
                    ..Default::default()
                }),
                Arc::new(Transaction {
                    version: DEFAULT_BUFFER_SIZE as u64 - 1,
                    ..Default::default()
                }),
                Arc::new(Transaction {
                    version: DEFAULT_BUFFER_SIZE as u64,
                    ..Default::default()
                }),
            ])
            .await
            .unwrap();
        let result = buffer.get_transactions(0).await;
        match result {
            BufferGetStatus::BehindBuffer => {}
            _ => panic!("Unexpected result"),
        }
        let result = buffer.get_transactions(1).await;
        match result {
            BufferGetStatus::InBuffer(transactions) => {
                assert_eq!(transactions.len(), DEFAULT_BUFFER_SIZE);
                // Verify they're all consecutive.
                for (index, t) in transactions.iter().enumerate() {
                    assert_eq!(t.version, index as u64 + 1);
                }
            },
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    async fn test_unordered_transaction_inserted() {
        let buffer = TransactionsBuffer::new();
        let transactions = vec![
            Arc::new(Transaction {
                version: 1,
                ..Default::default()
            }),
            Arc::new(Transaction {
                version: 3,
                ..Default::default()
            }),
            Arc::new(Transaction {
                version: 2,
                ..Default::default()
            }),
        ];
        let result = buffer.push_transactions(&transactions).await;
        assert!(result.is_err());
        let result = buffer.get_transactions(1).await;
        match result {
            BufferGetStatus::AheadOfBuffer => {}
            _ => panic!("Unexpected result"),
        }
    }

    #[tokio::test]
    async fn test_insertion_gap() {
        let buffer = TransactionsBuffer::new();
        let transactions = vec![
            Arc::new(Transaction {
                version: 1,
                ..Default::default()
            }),
        ];
        buffer.push_transactions(&transactions).await.unwrap();
        let result = buffer.get_transactions(1).await;
        match result {
            BufferGetStatus::InBuffer(transactions) => {
                assert_eq!(transactions.len(), 1);
                assert_eq!(transactions[0].version, 1);
            }
            _ => panic!("Unexpected result"),
        }

        let transactions = vec![
            Arc::new(Transaction {
                version: 3,
                ..Default::default()
            }),
        ];
        let result = buffer.push_transactions(&transactions).await;
        assert!(result.is_err());
    }
}