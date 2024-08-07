// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::{
    fs,
    fs::File,
    io::{BufWriter, Error},
    path::PathBuf,
};
use tar::{Archive, Builder};

pub fn rename_db_folders_and_cleanup(
    original_db_path: &PathBuf,
    temp_old_db_path: &PathBuf,
    restored_db_path: &PathBuf,
) -> Result<(), Error> {
    // Rename the original DB path to a temporary old DB path
    fs::rename(original_db_path, temp_old_db_path).map_err(|e| {
        Error::new(
            e.kind(),
            format!(
                "Failed to rename original DB folder from {:?} to {:?}: {}",
                original_db_path, temp_old_db_path, e
            ),
        )
    })?;

    // Rename the restored DB path to the original DB path
    fs::rename(restored_db_path, original_db_path).map_err(|e| {
        Error::new(
            e.kind(),
            format!(
                "Failed to rename restored DB folder from {:?} to {:?}: {}",
                restored_db_path, original_db_path, e
            ),
        )
    })?;

    // Remove the temporary old DB folder
    fs::remove_dir_all(temp_old_db_path).map_err(|e| {
        Error::new(
            e.kind(),
            format!(
                "Failed to remove old DB folder {:?}: {}",
                temp_old_db_path, e
            ),
        )
    })?;

    Ok(())
}

/// Creates a tar.gz archive from the db snapshot directory
pub fn create_tar_gz(dir_path: PathBuf, backup_file_name: &str) -> Result<PathBuf, anyhow::Error> {
    let tar_file_name = format!("{}.tar.gz", backup_file_name);
    let tar_file_path = dir_path.join(&tar_file_name);
    let temp_tar_file_path = dir_path.join(format!("{}.tmp", tar_file_name));

    let tar_file = File::create(&temp_tar_file_path)?;
    let gz_encoder = GzEncoder::new(tar_file, Compression::default());
    let tar_data = BufWriter::new(gz_encoder);
    let mut tar_builder = Builder::new(tar_data);

    tar_builder.append_dir_all(".", &dir_path)?;
    tar_builder.into_inner()?;

    std::fs::rename(&temp_tar_file_path, &tar_file_path)?;

    Ok(tar_file_path)
}

/// Unpack a tar.gz archive to a specified directory
pub fn unpack_tar_gz(temp_file_path: &PathBuf, target_db_path: &PathBuf) -> anyhow::Result<()> {
    let temp_dir_path = target_db_path.with_extension("tmp");
    fs::create_dir(&temp_dir_path)?;

    let file = File::open(temp_file_path)?;
    let gz_decoder = GzDecoder::new(file);
    let mut archive = Archive::new(gz_decoder);
    archive.unpack(&temp_dir_path)?;

    fs::remove_dir_all(target_db_path).unwrap_or(());
    fs::rename(&temp_dir_path, target_db_path)?; // Atomically replace the directory
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocksdb::{
        ColumnFamilyDescriptor, DBWithThreadMode, IteratorMode, Options, SingleThreaded,
    };
    use std::{
        fs::File,
        io::{Read, Write},
    };
    use tempfile::tempdir;

    type DB = DBWithThreadMode<SingleThreaded>;

    #[test]
    fn test_rename_db_folders_and_cleanup() {
        // Create temporary directories to simulate the original, temp old, and restored DB paths
        let original_db_dir = tempdir().unwrap();
        let temp_old_db_dir = tempdir().unwrap();
        let restored_db_dir = tempdir().unwrap();

        // Create a mock file in each directory to simulate DB contents
        File::create(original_db_dir.path().join("original_db_file")).unwrap();
        File::create(restored_db_dir.path().join("restored_db_file")).unwrap();

        // Call the function with the paths
        let result = rename_db_folders_and_cleanup(
            &original_db_dir.path().to_path_buf(),
            &temp_old_db_dir.path().to_path_buf(),
            &restored_db_dir.path().to_path_buf(),
        );

        // Check if the function executed successfully
        assert!(result.is_ok());

        // Check if the original DB directory now contains the restored DB file
        assert!(original_db_dir.path().join("restored_db_file").exists());

        // Check if the temp old DB directory has been removed
        assert!(!temp_old_db_dir.path().exists());
    }

    #[test]
    fn test_create_unpack_tar_gz_and_preserves_content() -> anyhow::Result<()> {
        // Create a temporary directory and a file within it
        let dir_to_compress = tempdir()?;
        let file_path = dir_to_compress.path().join("testfile.txt");
        let test_content = "Sample content";
        let mut file = File::create(file_path)?;
        writeln!(file, "{}", test_content)?;

        // Create a tar.gz file from the directory
        let tar_gz_path = create_tar_gz(dir_to_compress.path().to_path_buf(), "testbackup")?;
        assert!(tar_gz_path.exists());

        // Create a new temporary directory to unpack the tar.gz file
        let unpack_dir = tempdir()?;
        unpack_tar_gz(&tar_gz_path, &unpack_dir.path().to_path_buf())?;

        // Verify the file is correctly unpacked
        let unpacked_file_path = unpack_dir.path().join("testfile.txt");
        assert!(unpacked_file_path.exists());

        // Read content from the unpacked file
        let mut unpacked_file = File::open(unpacked_file_path)?;
        let mut unpacked_content = String::new();
        unpacked_file.read_to_string(&mut unpacked_content)?;

        // Assert that the original content is equal to the unpacked content
        assert_eq!(unpacked_content.trim_end(), test_content);

        Ok(())
    }

    #[tokio::test]
    async fn test_pack_unpack_compare_rocksdb() -> anyhow::Result<()> {
        // Create a temporary directory for the original RocksDB
        let original_db_dir = tempdir()?;
        let original_db_path = original_db_dir.path();

        // Initialize RocksDB with some data
        {
            let db = DB::open_default(original_db_path)?;
            db.put(b"key1", b"value1")?;
            db.put(b"key2", b"value2")?;
            db.flush()?;
        }

        // Pack the original RocksDB into a tar.gz file
        let tar_gz_path = create_tar_gz(original_db_path.to_path_buf(), "testbackup")?;
        assert!(tar_gz_path.exists(), "Tar.gz file was not created.");

        // Create a temporary directory for the unpacked RocksDB
        let unpacked_db_dir = tempdir()?;
        let unpacked_db_path = unpacked_db_dir.path();

        // Unpack the tar.gz file to the new directory
        unpack_tar_gz(&tar_gz_path, &unpacked_db_path.to_path_buf())?;

        // Compare the original and unpacked databases
        let comparison_result = compare_rocksdb(
            original_db_path.to_str().unwrap(),
            unpacked_db_path.to_str().unwrap(),
        )?;
        assert!(
            comparison_result,
            "Databases are not the same after packing and unpacking."
        );

        Ok(())
    }

    fn compare_rocksdb(db1_path: &str, db2_path: &str) -> Result<bool, anyhow::Error> {
        let db1 = open_db_with_column_families(db1_path)?;
        let db2 = open_db_with_column_families(db2_path)?;

        let iter1 = db1.iterator(IteratorMode::Start); // Iterate from the start of db1
        let mut iter2 = db2.iterator(IteratorMode::Start); // Iterate from the start of db2

        for result1 in iter1 {
            let (key1, value1) = result1?;

            match iter2.next() {
                Some(result2) => {
                    let (key2, value2) = result2?;
                    if key1 != key2 || value1 != value2 {
                        // If keys or values differ, the databases are not identical
                        return Ok(false);
                    }
                },
                None => {
                    // db2 has fewer elements than db1
                    return Ok(false);
                },
            }
        }

        // Check if db2 has more elements than db1
        if iter2.next().is_some() {
            return Ok(false);
        }

        Ok(true) // Databases are identical
    }

    fn open_db_with_column_families(
        db_path: &str,
    ) -> anyhow::Result<DBWithThreadMode<SingleThreaded>> {
        let mut db_opts = Options::default();
        db_opts.create_if_missing(false);

        let cfs = DB::list_cf(&db_opts, db_path).map_err(anyhow::Error::new)?; // Convert rocksdb::Error to anyhow::Error
        let cf_descriptors = cfs
            .into_iter()
            .map(|cf| ColumnFamilyDescriptor::new(cf, Options::default()))
            .collect::<Vec<_>>();

        DB::open_cf_descriptors(&db_opts, db_path, cf_descriptors).map_err(anyhow::Error::new)
    }
}
