use std::sync::RwLock;

use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};

use crate::{AptosDbError, DbReader, Result};

/// A wrapper over [`DbReader`], representing ledger at the end of a block
/// as the latest ledger and its version as the latest transaction version.
pub struct FinalityView<Db> {
    reader: Db,
    finalized_ledger_info: RwLock<Option<LedgerInfoWithSignatures>>,
}

impl<Db> FinalityView<Db> {
    pub fn new(reader: Db) -> Self {
        Self {
            reader,
            finalized_ledger_info: RwLock::new(None),
        }
    }
}

impl<Db: DbReader> FinalityView<Db> {
    /// Updates the view for the latest finalized block at the specified height.
    pub fn set_finalized_block_height(&self, height: u64) -> Result<()> {
        let (start_ver, _, _) = self.reader.get_block_info_by_height(height)?;
        let ledger_info = match self.reader.get_epoch_ending_ledger_info(start_ver) {
            Ok(li) => li,
            Err(AptosDbError::NotFound(_)) => self.reader.get_latest_ledger_info()?,
            Err(e) => return Err(e),
        };

        let mut fin_legder_info = self.finalized_ledger_info.write().unwrap();
        *fin_legder_info = Some(ledger_info);
        Ok(())
    }
}

impl<Db: DbReader> DbReader for FinalityView<Db> {
    fn get_read_delegatee(&self) -> &dyn DbReader {
        &self.reader
    }

    fn get_latest_ledger_info_option(&self) -> Result<Option<LedgerInfoWithSignatures>> {
        let fin_ledger_info = self.finalized_ledger_info.read().unwrap();
        Ok(fin_ledger_info.clone())
    }

    fn get_latest_version(&self) -> Result<Version> {
        let fin_ledger_info = self.finalized_ledger_info.read().unwrap();
        fin_ledger_info
            .as_ref()
            .map(|li| li.ledger_info().version())
            .ok_or_else(|| AptosDbError::NotFound("finalized version".into()))
    }

    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        let fin_ledger_info = self.finalized_ledger_info.read().unwrap();
        let version = fin_ledger_info
            .as_ref()
            .map(|li| li.ledger_info().version());
        Ok(version)
    }

    // TODO: override any other methods needed to maintain the illusion.
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aptos_crypto::HashValue;
    use aptos_types::state_store::{state_key::StateKey, TStateView as _};

    use super::*;
    use crate::{mock::MockDbReaderWriter, state_view::LatestDbStateCheckpointView as _};

    #[test]
    fn test_get_latest_ledger_info() -> anyhow::Result<()> {
        let view = FinalityView::new(MockDbReaderWriter);

        let ledger_info = view.get_latest_ledger_info_option()?;
        assert_eq!(ledger_info, None);
        let blockheight = 1;

        // Set the finalized ledger info
        view.set_finalized_block_height(blockheight)?;

        // Get the latest ledger info after setting it
        let ledger_info = view.get_latest_ledger_info_option()?.unwrap();

        assert_eq!(
            ledger_info.ledger_info().commit_info().id(),
            HashValue::new([1; HashValue::LENGTH]),
        );

        Ok(())
    }

    #[test]
    fn test_get_latest_version() -> anyhow::Result<()> {
        let view = FinalityView::new(MockDbReaderWriter);
        let res = view.get_latest_version();
        assert!(res.is_err());
        let blockheight = 1;
        view.set_finalized_block_height(blockheight)?;
        let version = view.get_latest_version()?;
        assert_eq!(version, 1);
        Ok(())
    }

    #[test]
    fn test_get_latest_state_checkpoint_version() -> Result<()> {
        let view = FinalityView::new(MockDbReaderWriter);
        let version = view.get_latest_state_checkpoint_version()?;
        assert_eq!(version, None);
        view.set_finalized_block_height(1)?;
        let version = view.get_latest_state_checkpoint_version()?;
        assert_eq!(version, Some(1));
        Ok(())
    }

    #[test]
    fn test_latest_state_checkpoint_view() -> anyhow::Result<()> {
        let view = Arc::new(FinalityView::new(MockDbReaderWriter));
        let reader: Arc<dyn DbReader> = view.clone();
        view.set_finalized_block_height(1)?;
        let latest_state_view = reader.latest_state_checkpoint_view()?;
        // TODO: modify mock so we get different states for different versions
        let key = StateKey::raw(vec![1]);
        let value = latest_state_view.get_state_value(&key)?.unwrap();
        assert_eq!(value.bytes(), &vec![1]);
        Ok(())
    }
}
