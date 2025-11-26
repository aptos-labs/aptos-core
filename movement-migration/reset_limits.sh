#!/bin/bash
# RUN THIS AFTER L1 MIGRATION IS COMPLETE AND WE HAVE ACUALLY HAVE THE STAKE 

# Parse command line arguments
PRIVATE_KEY_FILE=""
URL=""

while getopts "k:u:" opt; do
  case $opt in
    k)
      PRIVATE_KEY_FILE="$OPTARG"
      ;;
    u)
      URL="$OPTARG"
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      exit 1
      ;;
    :)
      echo "Option -$OPTARG requires an argument." >&2
      exit 1
      ;;
  esac
done

# Check if required parameters are provided
if [ -z "$PRIVATE_KEY_FILE" ] || [ -z "$URL" ]; then
  echo "Error: Both -k (private key file) and -u (URL) parameters are required."
  echo "Usage: $0 -k <private_key_file> -u <url>"
  exit 1
fi

cargo run -p movement move run-script --script-path movement-migration/post-migration/scripts/reset.move --sender-account 0xa550c18 --private-key-file "$PRIVATE_KEY_FILE"  --url "$URL"  --assume-yes
