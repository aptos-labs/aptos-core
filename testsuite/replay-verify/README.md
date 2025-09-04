# Replay Verification Tools

This folder contains tools for managing and provisioning archive storage used in replay verifying for the Velor blockchain networks.

## Files

### main.py

The main script for executing replay verify tests. This script is responsible for:

- Running replay verification tests against specified networks (testnet/mainnet)
- Verifying transaction execution matches expected results
- Handling test orchestration and reporting
``` test with cli
cd testsuite/replay-verify
poetry shell
python main.py  --image_tag YOUR_IMAGE_TAG --network testnet 
```

### archive_disk_utils.py

A utility script for managing archive storage disks used in replay verification. This script:

- Provisions Google Cloud Storage disks for storing blockchain archive data
- Supports both testnet and mainnet networks
- Is called by GitHub Actions workflows to automatically manage storage resources
```test with cli
cd testsuite/replay-verify
poetry shell
python archive_disk_utils.py --network mainnet
```


