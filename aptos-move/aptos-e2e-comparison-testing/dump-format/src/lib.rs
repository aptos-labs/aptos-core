// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The on-disk dump format written by `aptos-comparison-testing dump`. It lives in its own crate so
//! that tools reading the dumps share the definition without depending on the comparison tool
//! (which pulls in the Move compiler stack).
//!
//! A dump directory holds:
//!
//! ```text
//! rocks_txn_idx_db/        # RocksDB: BCS(version) -> BCS(TxnIndex)
//! state_data/<v>_state     # BCS(BTreeMap<StateKey, StateValue>): the reads that found a value
//! write_set_data/<v>_write_set
//! version_index.txt        # versions, one per line (flushed in batches)
//! err_log.txt
//! aptos-commons/           # framework sources cloned from git at dump time
//! <package>.<addr>.<n>/    # sources of the called packages
//! ```

use anyhow::{Context, Result};
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, Version},
    write_set::WriteSet,
};
use rocksdb::{DBWithThreadMode, IteratorMode, Options, SingleThreaded, DB};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

pub const STATE_DATA: &str = "state_data";
const WRITE_SET_DATA: &str = "write_set_data";
pub const INDEX_FILE: &str = "version_index.txt";
const ERR_LOG: &str = "err_log.txt";
const ROCKS_INDEX_DB: &str = "rocks_txn_idx_db";
pub const APTOS_COMMONS: &str = "aptos-commons";
const MAX_TO_FLUSH: usize = 50000;

pub struct IndexWriter {
    index_writer: BufWriter<File>,
    err_logger: BufWriter<File>,
    version_vec: Vec<u64>,
    counter: usize,
}

impl IndexWriter {
    pub fn new(root: &Path) -> Self {
        let create_file = |file_name: &str| -> File {
            let path = root.to_path_buf().join(file_name);
            if !path.exists() {
                File::create(path).expect("Error encountered while creating file!")
            } else {
                OpenOptions::new().append(true).open(path).unwrap()
            }
        };
        let index_file = create_file(INDEX_FILE);
        let err_log = create_file(ERR_LOG);
        Self {
            index_writer: BufWriter::with_capacity(4096 * 1024 /* 4096KB */, index_file),
            err_logger: BufWriter::with_capacity(4096 * 1024 /* 4096KB */, err_log),
            version_vec: vec![],
            counter: 0,
        }
    }

    pub fn reset_vec(&mut self) {
        self.version_vec = vec![];
    }

    pub fn add_version(&mut self, version: u64) {
        self.version_vec.push(version);
    }

    pub fn dump_version(&mut self) {
        self.version_vec.sort();
        self.version_vec.iter().for_each(|&version| {
            self.index_writer
                .write_fmt(format_args!("{}\n", version))
                .unwrap()
        });
        self.counter += self.version_vec.len();
        self.reset_vec();
        if self.counter > MAX_TO_FLUSH {
            self.flush_writer();
        }
    }

    pub fn write_err(&mut self, err_msg: &str) {
        self.err_logger
            .write_fmt(format_args!("{}\n", err_msg))
            .unwrap();
        self.err_logger.flush().unwrap();
    }

    pub fn flush_writer(&mut self) {
        self.index_writer.flush().unwrap();
        self.counter = 0;
    }
}

pub struct IndexReader {
    index_reader: BufReader<File>,
}

impl IndexReader {
    pub fn check_availability(root: &Path) -> bool {
        root.to_path_buf().join(INDEX_FILE).exists()
    }

    pub fn new(root: &Path) -> Self {
        let index_path = root.to_path_buf().join(INDEX_FILE);
        let index_file = File::open(index_path).unwrap();
        let index_reader = BufReader::new(index_file);
        Self { index_reader }
    }

    /// `Err` marks a line that is not a version; callers skip it.
    #[allow(clippy::result_unit_err)]
    pub fn get_next_version(&mut self) -> Result<Option<u64>, ()> {
        let mut cur_idx = String::new();
        let num_bytes = self.index_reader.read_line(&mut cur_idx).unwrap();
        if num_bytes == 0 {
            return Ok(None);
        }
        let indx = cur_idx.trim().parse();
        if indx.is_ok() {
            Ok(indx.ok())
        } else {
            Err(())
        }
    }

    pub fn get_next_version_ge(&mut self, version: u64) -> Option<u64> {
        loop {
            let next_val = self.get_next_version();
            if next_val.is_err() {
                continue;
            }
            if let Some(val) = next_val.unwrap() {
                if val >= version {
                    return Some(val);
                }
            } else {
                break;
            }
        }
        None
    }
}

pub struct DataManager {
    state_data_dir_path: PathBuf,
    write_set_dir_path: PathBuf,
    db: DBWithThreadMode<SingleThreaded>,
}

impl DataManager {
    pub fn new_with_dir_creation(root: &Path) -> Self {
        let dm = Self::new(root);
        if !dm.state_data_dir_path.exists() {
            std::fs::create_dir_all(dm.state_data_dir_path.as_path()).unwrap();
        }
        if !dm.write_set_dir_path.exists() {
            std::fs::create_dir_all(dm.write_set_dir_path.as_path()).unwrap();
        }
        dm
    }

    pub fn new(root: &Path) -> Self {
        let db = DB::open_default(root.to_path_buf().join(ROCKS_INDEX_DB)).unwrap();
        Self::with_db(root, db)
    }

    /// Opens an existing dump without writing to it.
    pub fn open_read_only(root: &Path) -> Result<Self> {
        let db = DB::open_for_read_only(&Options::default(), root.join(ROCKS_INDEX_DB), false)
            .with_context(|| format!("failed to open the txn index in {:?}", root))?;
        Ok(Self::with_db(root, db))
    }

    fn with_db(root: &Path, db: DBWithThreadMode<SingleThreaded>) -> Self {
        Self {
            state_data_dir_path: root.join(STATE_DATA),
            write_set_dir_path: root.join(WRITE_SET_DATA),
            db,
        }
    }

    pub fn check_dir_availability(&self) -> bool {
        if !(self.state_data_dir_path.exists() && self.write_set_dir_path.exists()) {
            return false;
        }
        true
    }

    pub fn dump_state_data(&self, version: u64, state: &HashMap<StateKey, StateValue>) {
        let state_path = self.state_path(version);
        if !state_path.exists() {
            let mut data_state_file = File::create(state_path).unwrap();
            let state: BTreeMap<_, _> = state.iter().collect();
            data_state_file
                .write_all(&bcs::to_bytes(&state).unwrap())
                .unwrap();
        }
    }

    pub fn dump_write_set(&self, version: u64, write_set: &WriteSet) {
        let write_set_path = self
            .write_set_dir_path
            .join(format!("{}_write_set", version));
        if !write_set_path.exists() {
            let mut write_set_file = File::create(write_set_path).unwrap();
            write_set_file
                .write_all(&bcs::to_bytes(&write_set).unwrap())
                .unwrap();
        }
    }

    pub fn dump_txn_index(&self, version: u64, version_idx: &TxnIndex) {
        self.db
            .put(
                bcs::to_bytes(&version).unwrap(),
                bcs::to_bytes(&version_idx).unwrap(),
            )
            .unwrap();
    }

    /// The legacy tool's lookup: an entry that cannot be read counts as missing; one that cannot be
    /// decoded panics. See [`Self::try_get_txn_index`] for a lookup that reports both.
    pub fn get_txn_index(&self, version: u64) -> Option<TxnIndex> {
        let key = bcs::to_bytes(&version).expect("a version encodes");
        match self.db.get(key) {
            Ok(Some(bytes)) => {
                Some(bcs::from_bytes(&bytes).expect("failed to decode the txn index entry"))
            },
            Ok(None) | Err(_) => None,
        }
    }

    pub fn try_get_txn_index(&self, version: Version) -> Result<Option<TxnIndex>> {
        let key = bcs::to_bytes(&version).context("failed to encode version")?;
        let Some(bytes) = self.db.get(key).context("failed to read the txn index")? else {
            return Ok(None);
        };
        let txn_idx = bcs::from_bytes(&bytes)
            .with_context(|| format!("failed to decode the txn index entry of {}", version))?;
        Ok(Some(txn_idx))
    }

    pub fn get_state(&self, version: u64) -> BTreeMap<StateKey, StateValue> {
        self.try_get_state(version)
            .expect("failed to read or decode the state file")
    }

    pub fn try_get_state(&self, version: Version) -> Result<BTreeMap<StateKey, StateValue>> {
        let state_path = self.state_path(version);
        let bytes = std::fs::read(&state_path)
            .with_context(|| format!("failed to read {:?}", state_path))?;
        bcs::from_bytes(&bytes).with_context(|| format!("failed to decode {:?}", state_path))
    }

    pub fn state_path(&self, version: Version) -> PathBuf {
        self.state_data_dir_path.join(format!("{}_state", version))
    }

    /// The versions in the txn index, in increasing order. Unlike `version_index.txt`, which is
    /// flushed in batches, the index holds every dumped version.
    pub fn txn_index_versions(&self) -> Result<Vec<Version>> {
        let mut versions = vec![];
        for item in self.db.iterator(IteratorMode::Start) {
            let (key, _) = item.context("failed to iterate the txn index")?;
            versions.push(bcs::from_bytes::<Version>(&key).context("bad txn index key")?);
        }
        versions.sort_unstable();
        Ok(versions)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct PackageInfo {
    pub address: AccountAddress,
    pub package_name: String,
    pub upgrade_number: Option<u64>,
}

impl fmt::Display for PackageInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut name = format!("{}.{}", self.package_name, self.address);
        if let Some(upgrade_number) = self.upgrade_number {
            name = format!("{}.{}", name, upgrade_number);
        }
        write!(f, "{}", name)?;
        Ok(())
    }
}

impl PackageInfo {
    pub fn is_compilable(&self) -> bool {
        self.address != AccountAddress::ZERO
    }

    pub fn non_compilable_info() -> Self {
        Self {
            address: AccountAddress::ZERO,
            package_name: "".to_string(),
            upgrade_number: None,
        }
    }
}

/// The field order is the BCS layout of every dump written so far and must not change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnIndex {
    pub version: u64,
    pub package_info: PackageInfo,
    pub txn: Transaction,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_records_read_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        let txn_index = TxnIndex {
            version: 7,
            package_info: PackageInfo {
                address: AccountAddress::ONE,
                package_name: "AptosFramework".to_string(),
                upgrade_number: None,
            },
            txn: Transaction::StateCheckpoint(Default::default()),
        };
        let state: HashMap<_, _> = [
            (StateKey::raw(b"b"), StateValue::new_legacy(vec![2].into())),
            (StateKey::raw(b"a"), StateValue::new_legacy(vec![1].into())),
        ]
        .into_iter()
        .collect();
        {
            let dm = DataManager::new_with_dir_creation(dir.path());
            dm.dump_txn_index(7, &txn_index);
            dm.dump_state_data(7, &state);
        }

        let dm = DataManager::open_read_only(dir.path()).expect("open");
        assert_eq!(dm.txn_index_versions().expect("versions"), vec![7]);
        let read = dm.try_get_txn_index(7).expect("read").expect("present");
        assert_eq!(read.package_info, txn_index.package_info);
        assert_eq!(read.txn, txn_index.txn);
        assert!(dm.try_get_txn_index(8).expect("read").is_none());
        let read_state = dm.try_get_state(7).expect("state");
        assert_eq!(read_state, state.into_iter().collect::<BTreeMap<_, _>>());
    }
}
