#!/bin/bash

# Script usage
usage() {
    echo "Usage: $0 -k <private_key_file> -u <url> -e <initial_epoch> -t <initial_timestamp>"
    echo ""
    echo "Required arguments:"
    echo "  -k    Path to the private key file"
    echo "  -u    URL of the network (e.g., http://127.0.0.1:8080)"
    echo "  -e    Initial epoch (e.g., 2111770). Get this from the node /v1 url. If you already set the value, DO NOT set it again."
    echo "  -t    Initial timestamp (e.g., 1762293093318227). Get this from the node /v1 url. If you already set the value, DO NOT set it again"
    echo ""
    echo "Example:"
    echo "  $0 -k ../private_key_testnet.key -u http://127.0.0.1:8080 -e 2111770 -t 1762293093318227"
    exit 1
}

# Parse command-line arguments
PRIVATE_KEY_FILE=""
URL=""
INITIAL_EPOCH=""
INITIAL_TIMESTAMP=""

while getopts "k:u:e:t:h" opt; do
    case ${opt} in
        k )
            PRIVATE_KEY_FILE="$OPTARG"
            ;;
        u )
            URL="$OPTARG"
            ;;
        e )
            INITIAL_EPOCH="$OPTARG"
            ;;
        t )
            INITIAL_TIMESTAMP="$OPTARG"
            ;;
        h )
            usage
            ;;
        \? )
            echo "Invalid option: -$OPTARG" >&2
            usage
            ;;
    esac
done

# Check if required arguments are provided
if [ -z "$PRIVATE_KEY_FILE" ] || [ -z "$URL" ] || [ -z "$INITIAL_EPOCH" ] || [ -z "$INITIAL_TIMESTAMP" ]; then
    echo "Error: All arguments (private key file, URL, initial epoch, and initial timestamp) are required."
    usage
fi

# Validate that the initial epoch and timestamp are numeric
if ! [[ "$INITIAL_EPOCH" =~ ^[0-9]+$ ]]; then
    echo "Error: Initial epoch must be a numeric value"
    exit 1
fi

if ! [[ "$INITIAL_TIMESTAMP" =~ ^[0-9]+$ ]]; then
    echo "Error: Initial timestamp must be a numeric value"
    exit 1
fi

# Verify that the private key file exists
if [ ! -f "$PRIVATE_KEY_FILE" ]; then
    echo "Error: Private key file not found: $PRIVATE_KEY_FILE"
    exit 1
fi

# Enable strict error handling - exit on any command failure
set -e

echo "Starting framework upgrade..."
echo "Private key file: $PRIVATE_KEY_FILE"
echo "URL: $URL"
echo "Initial epoch: $INITIAL_EPOCH"
echo "Initial timestamp: $INITIAL_TIMESTAMP"
echo ""

# update the framework for epoch duration update
cargo run -p movement move run-script --script-path  ./movement-migration/update_reconfiguration/clone_proposals/sources/vX.YY.Z/proposal_1_upgrade_framework/0-move-stdlib.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE" --url "$URL" --assume-yes
cargo run -p movement move run-script --script-path  ./movement-migration/update_reconfiguration/clone_proposals/sources/vX.YY.Z/proposal_1_upgrade_framework/1-aptos-stdlib.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE" --url "$URL" --assume-yes
cargo run -p movement move run-script --script-path  ./movement-migration/update_reconfiguration/clone_proposals/sources/vX.YY.Z/proposal_1_upgrade_framework/2-aptos-framework.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL" --assume-yes
cargo run -p movement move run-script --script-path  ./movement-migration/update_reconfiguration/clone_proposals/sources/vX.YY.Z/proposal_1_upgrade_framework/3-aptos-token.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL" --assume-yes
cargo run -p movement move run-script --script-path  ./movement-migration/update_reconfiguration/clone_proposals/sources/vX.YY.Z/proposal_1_upgrade_framework/4-aptos-token-objects.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL" --assume-yes
# update the framework to enable epoch duration change
cargo run -p movement move run-script --script-path movement-migration/update_reconfiguration/sources/script.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes --args u64:$INITIAL_EPOCH u64:$INITIAL_TIMESTAMP
cargo run -p movement move run-script --script-path movement-migration/update_reconfiguration/sources/change_epoch_duration.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
# enable feature flag 
cargo run -p movement move run-script --script-path movement-migration/framework-upgrades/scripts/feature-flag-reconfig.move  --bytecode-version 6 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes

# move2 and staking upgrade
cargo run -p movement move run-script --script-path movement-migration/move2/sources/v0.1/proposal_1_upgrade_framework/0-move-stdlib.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
cargo run -p movement move run-script --script-path movement-migration/move2/sources/v0.1/proposal_1_upgrade_framework/1-aptos-stdlib.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
cargo run -p movement move run-script --script-path movement-migration/move2/sources/v0.1/proposal_1_upgrade_framework/2-aptos-framework.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
cargo run -p movement move run-script --script-path movement-migration/move2/sources/v0.1/proposal_1_upgrade_framework/3-aptos-token.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
cargo run -p movement move run-script --script-path movement-migration/move2/sources/v0.1/proposal_1_upgrade_framework/4-aptos-token-objects.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes

cargo run -p movement move run-script --script-path movement-migration/post-move2-upgrade/scripts/force-epoch-change.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
# initialize the ggp extension this has to be done within the same epoch 
cargo run -p movement move run-script --script-path movement-migration/post-move2-upgrade/scripts/after-move2-upgrade.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
# Update staking lock period with the same value as Aptos
cargo run -p movement move run-script --script-path movement-migration/update_recurring_lockup_duration/scripts/update_recurring_lockup_duration.move --args u64:1209600 --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
