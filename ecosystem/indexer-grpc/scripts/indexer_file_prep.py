# This script is used to prepare the indexer files for the indexer.
# It will download the files from the bucket and prepare them for the indexer.

import os
import argparse
import requests
import json
from hashbase import RIPEMD128
from tqdm import tqdm
import asyncio
import aiohttp
from itertools import islice

MAINNET_BUCKET_NAME = "aptos-indexer-grpc-mainnet2"
TESTNET_BUCKET_NAME = "aptos-indexer-grpc-testnet2"
METADATA_FILE_NAME = "metadata.json"
COMPRESSED_FILES_DIR = "compressed_files"
TRANSACTION_COUNT_PER_FILE = 1000
COMPRESSION_ALGORITHM = "lz4"

def main():
    # Get the network from the command line.
    parser = argparse.ArgumentParser()
    parser.add_argument("--network", type=str, required=True)
    # Target folder for the files.
    parser.add_argument("--target_folder", type=str, required=True)
    args = parser.parse_args()

    if args.network == "mainnet":
        bucket_name = MAINNET_BUCKET_NAME
    elif args.network == "testnet":
        bucket_name = TESTNET_BUCKET_NAME
    else:
        raise ValueError(f"Invalid network: {args.network}")

    # Download the metadata file.
    metadata_url = f"https://storage.googleapis.com/{bucket_name}/{METADATA_FILE_NAME}"
    metadata_response = requests.get(metadata_url)
    metadata_response.raise_for_status()
    metadata_json = metadata_response.json()
    
    # Save the metadata file to the target folder.
    metadata_file_path = os.path.join(args.target_folder, METADATA_FILE_NAME)
    # Create the target folder if it doesn't exist.
    os.makedirs(args.target_folder, exist_ok=True)
    with open(metadata_file_path, "w") as f:
        json.dump(metadata_json, f)

    target_version = metadata_json["version"] -  metadata_json["version"] % 1000

    # Download the files for the target version.
    download_files(bucket_name, target_version, args.target_folder)

def file_name_to_version(version):
    # use Ripemd128 to hash the version.
    hash = RIPEMD128().generate_hash(str(version))
    return f"{hash}_{version}.bin"

async def download_file_async(session, bucket_name, file_name, file_path):
    url = f"https://storage.googleapis.com/{bucket_name}/{COMPRESSED_FILES_DIR}/{COMPRESSION_ALGORITHM}/{file_name}"
    async with session.get(url) as response:
        response.raise_for_status()
        content = await response.read()
        with open(file_path, "wb") as f:
            f.write(content)

async def download_batch_async(session, bucket_name, versions, compression_algorithm_dir):
    tasks = []
    for version in versions:
        file_name = file_name_to_version(version)
        file_path = os.path.join(compression_algorithm_dir, file_name)
        if not os.path.exists(file_path):
            task = download_file_async(session, bucket_name, file_name, file_path)
            tasks.append(task)
    if tasks:
        await asyncio.gather(*tasks)

async def download_files_async(bucket_name, target_version, target_folder):
    # If compressed files directory doesn't exist, create it.
    compressed_files_dir = os.path.join(target_folder, COMPRESSED_FILES_DIR)
    if not os.path.exists(compressed_files_dir):
        os.makedirs(compressed_files_dir)
    compression_algorithm_dir = os.path.join(compressed_files_dir, COMPRESSION_ALGORITHM)
    # If the compression algorithm folder doesn't exist, create it.
    if not os.path.exists(compression_algorithm_dir):
        os.makedirs(compression_algorithm_dir)

    # Create version ranges in batches of 20
    all_versions = range(0, target_version, TRANSACTION_COUNT_PER_FILE)
    # Split the versions into batches of 20.
    batch_size = 20
    batches = [all_versions[i:i + batch_size] for i in range(0, len(all_versions), batch_size)]
    
    async with aiohttp.ClientSession() as session:
        with tqdm(total=len(range(0, target_version, TRANSACTION_COUNT_PER_FILE))) as pbar:
            for batch in batches:
                await download_batch_async(session, bucket_name, batch, compression_algorithm_dir)
                pbar.update(len(batch))

    print("Successfully downloaded all the files.")

def download_files(bucket_name, target_version, target_folder):
    asyncio.run(download_files_async(bucket_name, target_version, target_folder))

if __name__ == "__main__":
    main()