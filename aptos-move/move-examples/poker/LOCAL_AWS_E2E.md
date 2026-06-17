# Localnet + AWS Nitro E2E

This is the repeatable smoke path for running the poker example with:

- an Aptos localnet on your laptop,
- the AWS Nitro root initialized on-chain,
- the poker table client running on an AWS EC2 parent instance,
- a Nitro Enclave producing the table attestation document,
- the player client running locally.

The flow validated on 2026-06-17 was:

```text
localnet -> initialize Nitro root store -> publish poker
AWS Nitro Enclave -> attestation doc bound to table address
AWS table client -> register_table over SSH reverse tunnel
local player client -> enter / request_leave
table signer -> settle_leaving_players
```

## Prerequisites

- AWS CLI configured for an account that can launch EC2 instances with Nitro Enclaves enabled.
- Docker available on the AWS parent instance. The commands below install it on Amazon Linux 2.
- Built local Aptos binaries:

```bash
cargo build -p aptos --bin aptos
cargo build -p aptos-framework --bin aptos-framework
cargo build -p aptos-node --bin aptos-node
cargo build -p aptos-faucet-service --bin aptos-faucet-service
```

## 1. Start Localnet

From the repo root:

```bash
export REPO="$PWD"
export WORK=/tmp/aptos-poker-e2e
rm -rf "$WORK"
mkdir -p "$WORK/framework"

(
  cd "$WORK/framework"
  "$REPO/target/debug/aptos-framework" release --target head
)

"$REPO/target/debug/aptos-node" \
  --test \
  --test-dir "$WORK/localnet" \
  --genesis-framework "$WORK/framework/head.mrb"
```

In another terminal:

```bash
"$REPO/target/debug/aptos-faucet-service" run-simple \
  --node-url http://127.0.0.1:8080 \
  --chain-id 4 \
  --key-file-path "$WORK/localnet/mint.key" \
  --listen-address 0.0.0.0 \
  --listen-port 8081
```

## 2. Initialize Nitro Roots

Download the AWS Nitro Enclaves root certificate and convert it to DER:

```bash
mkdir -p "$WORK/aws-roots"
curl -fsSL \
  https://aws-nitro-enclaves.amazonaws.com/AWS_NitroEnclaves_Root-G1.zip \
  -o "$WORK/aws-roots/AWS_NitroEnclaves_Root-G1.zip"
unzip -o "$WORK/aws-roots/AWS_NitroEnclaves_Root-G1.zip" -d "$WORK/aws-roots"
openssl x509 -in "$WORK/aws-roots/root.pem" -outform DER -out "$WORK/aws-roots/root.der"
openssl x509 -in "$WORK/aws-roots/root.pem" -noout -subject -issuer -fingerprint -sha256 -dates
```

The DER file should be 533 bytes. The root used in this smoke test had SHA-256 fingerprint:

```text
64:1A:03:21:A3:E2:44:EF:E4:56:46:31:95:D6:06:31:7E:D7:CD:CC:3C:17:56:E0:98:93:F3:C6:8F:79:BB:5B
```

Initialize the on-chain root store with the localnet core resources key. The sender is `0xa550c18`, not `0x1`; the entry function obtains the `0x1` signer through `aptos_governance::get_signer_testnet_only`.

```bash
ROOT_HEX="$(xxd -p -c 0 "$WORK/aws-roots/root.der")"
"$REPO/target/debug/aptos" move run \
  --url http://127.0.0.1:8080 \
  --sender-account 0xa550c18 \
  --private-key-file "$WORK/localnet/mint.key" \
  --encoding bcs \
  --function-id 0x1::aws_nitro_utils::initialize_testnet_only \
  --args "hex:[\"0x${ROOT_HEX}\"]" \
  --max-gas 100000 \
  --assume-yes

"$REPO/target/debug/aptos" move view \
  --url http://127.0.0.1:8080 \
  --function-id 0x1::aws_nitro_utils::trusted_root_count
```

Expected result:

```json
{ "Result": [ "1" ] }
```

## 3. Create Accounts And Publish Poker

Generate table and player keys:

```bash
mkdir -p "$WORK/keys"
"$REPO/target/debug/aptos" key generate --output-file "$WORK/keys/table.key" --encoding hex --assume-yes
"$REPO/target/debug/aptos" key generate --output-file "$WORK/keys/player.key" --encoding hex --assume-yes
```

Derive addresses:

```bash
(
  cd "$REPO/aptos-move/move-examples/poker/clients"
  npm install --package-lock=false
  node - <<'NODE'
const fs = require("fs");
const { Account, Ed25519PrivateKey } = require("@aptos-labs/ts-sdk");
for (const name of ["table", "player"]) {
  const pk = fs.readFileSync(`/tmp/aptos-poker-e2e/keys/${name}.key`, "utf8").trim();
  const account = Account.fromPrivateKey({ privateKey: new Ed25519PrivateKey(pk) });
  console.log(`${name.toUpperCase()}_PRIVATE_KEY=0x${pk}`);
  console.log(`${name.toUpperCase()}_ADDRESS=${account.accountAddress.toStringLong()}`);
}
NODE
)
```

Export those values:

```bash
export TABLE_PRIVATE_KEY=0x...
export TABLE_ADDRESS=0x...
export PLAYER_PRIVATE_KEY=0x...
export PLAYER_ADDRESS=0x...
```

Fund both accounts:

```bash
"$REPO/target/debug/aptos" account fund-with-faucet \
  --url http://127.0.0.1:8080 \
  --faucet-url http://127.0.0.1:8081 \
  --account "$TABLE_ADDRESS" \
  --amount 100000000000

"$REPO/target/debug/aptos" account fund-with-faucet \
  --url http://127.0.0.1:8080 \
  --faucet-url http://127.0.0.1:8081 \
  --account "$PLAYER_ADDRESS" \
  --amount 100000000000
```

Publish from a temporary package copy that uses the local framework:

```bash
rm -rf "$WORK/poker"
rsync -a --exclude node_modules "$REPO/aptos-move/move-examples/poker/" "$WORK/poker/"
python3 - <<PY
from pathlib import Path
p = Path("$WORK/poker/Move.toml")
s = p.read_text()
s = s.replace(
    'AptosFramework = { git = "https://github.com/aptos-labs/aptos-framework.git", subdir = "aptos-framework", rev = "mainnet" }',
    'AptosFramework = { local = "' + "$REPO" + '/aptos-move/framework/aptos-framework" }',
)
p.write_text(s)
PY

"$REPO/target/debug/aptos" move publish \
  --url http://127.0.0.1:8080 \
  --package-dir "$WORK/poker" \
  --named-addresses poker="$TABLE_ADDRESS" \
  --sender-account "$TABLE_ADDRESS" \
  --private-key-file "$WORK/keys/table.key" \
  --encoding hex \
  --skip-fetch-latest-git-deps \
  --max-gas 200000 \
  --assume-yes
```

## 4. Launch AWS Parent Instance

These commands create temporary AWS resources. Clean them up at the end.

```bash
export AWS_REGION=us-west-2
export AWS_NAME=aptos-poker-e2e
export MY_IP="$(curl -fsSL https://checkip.amazonaws.com | tr -d '\n')"
export VPC_ID="$(aws ec2 describe-vpcs \
  --filters Name=is-default,Values=true \
  --query 'Vpcs[0].VpcId' \
  --output text)"
export SUBNET_ID="$(aws ec2 describe-subnets \
  --filters Name=default-for-az,Values=true \
  --query 'Subnets[0].SubnetId' \
  --output text)"
export AMI_ID="$(aws ec2 describe-images \
  --owners amazon \
  --filters 'Name=name,Values=amzn2-ami-hvm-2.0.*-x86_64-gp2' 'Name=state,Values=available' \
  --query 'sort_by(Images,&CreationDate)[-1].ImageId' \
  --output text)"

aws ec2 create-key-pair \
  --key-name "${AWS_NAME}-key" \
  --query KeyMaterial \
  --output text > "$WORK/aws-key.pem"
chmod 600 "$WORK/aws-key.pem"

export SG_ID="$(aws ec2 create-security-group \
  --group-name "${AWS_NAME}-sg" \
  --description "Temporary SSH access for Aptos poker Nitro E2E" \
  --vpc-id "$VPC_ID" \
  --query GroupId \
  --output text)"

aws ec2 authorize-security-group-ingress \
  --group-id "$SG_ID" \
  --protocol tcp \
  --port 22 \
  --cidr "${MY_IP}/32"

export INSTANCE_ID="$(aws ec2 run-instances \
  --image-id "$AMI_ID" \
  --instance-type m5.xlarge \
  --key-name "${AWS_NAME}-key" \
  --network-interfaces "DeviceIndex=0,SubnetId=${SUBNET_ID},Groups=${SG_ID},AssociatePublicIpAddress=true" \
  --enclave-options Enabled=true \
  --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=${AWS_NAME}},{Key=Purpose,Value=${AWS_NAME}}]" \
  --query 'Instances[0].InstanceId' \
  --output text)"

aws ec2 wait instance-running --instance-ids "$INSTANCE_ID"
export PUBLIC_IP="$(aws ec2 describe-instances \
  --instance-ids "$INSTANCE_ID" \
  --query 'Reservations[0].Instances[0].PublicIpAddress' \
  --output text)"
```

Install dependencies on the AWS parent:

```bash
ssh -o StrictHostKeyChecking=no -i "$WORK/aws-key.pem" ec2-user@"$PUBLIC_IP" '
set -euxo pipefail
sudo amazon-linux-extras install -y aws-nitro-enclaves-cli docker
sudo yum install -y aws-nitro-enclaves-cli-devel jq gcc make git
sudo systemctl enable --now docker
sudo usermod -aG docker,ne ec2-user || true
sudo sed -i "s/^memory_mib:.*/memory_mib: 2048/" /etc/nitro_enclaves/allocator.yaml
sudo sed -i "s/^cpu_count:.*/cpu_count: 2/" /etc/nitro_enclaves/allocator.yaml
sudo systemctl enable --now nitro-enclaves-allocator.service
'
```

Amazon Linux 2 cannot install Node 20 from NodeSource because its glibc is too old. Install Node 16 for this smoke client:

```bash
ssh -i "$WORK/aws-key.pem" ec2-user@"$PUBLIC_IP" '
set -euxo pipefail
sudo rm -f /etc/yum.repos.d/nodesource*.repo
curl -fsSL https://rpm.nodesource.com/setup_16.x | sudo bash -
sudo yum clean all
sudo yum install -y nodejs-16.20.2-1nodesource
node --version
npm --version
'
```

## 5. Build And Run The Nitro Attester

The table attestation must bind to:

```text
user_data = b"APTOS_POKER_TABLE_V1" || bcs(table_address)
```

For an Aptos address, `bcs(address)` is the 32-byte address.

```bash
export USER_DATA_HEX="$(python3 - <<PY
domain = b"APTOS_POKER_TABLE_V1"
addr_hex = "$TABLE_ADDRESS"
addr = bytes.fromhex(addr_hex[2:] if addr_hex.startswith("0x") else addr_hex)
print((domain + addr).hex())
PY
)"

rsync -az -e "ssh -i $WORK/aws-key.pem" \
  "$REPO/aptos-move/move-examples/poker/e2e/nitro-attester/" \
  ec2-user@"$PUBLIC_IP":/home/ec2-user/attester/

ssh -i "$WORK/aws-key.pem" ec2-user@"$PUBLIC_IP" "
set -euxo pipefail
cd ~/attester
sudo docker build --build-arg USER_DATA_HEX=$USER_DATA_HEX -t aptos-poker-attester:latest .
sudo nitro-cli build-enclave --docker-uri aptos-poker-attester:latest --output-file aptos-poker-attester.eif
sudo nitro-cli run-enclave \
  --eif-path /home/ec2-user/attester/aptos-poker-attester.eif \
  --cpu-count 2 \
  --memory 1024 \
  --enclave-cid 16 \
  --debug-mode
"
```

For this smoke test, `--debug-mode` is used only so the parent can read stdout through `nitro-cli console`. Production table runners should use a vsock channel and should not run debug enclaves.

Capture the attestation document:

```bash
ssh -i "$WORK/aws-key.pem" ec2-user@"$PUBLIC_IP"
ENCLAVE_ID="$(sudo nitro-cli describe-enclaves | jq -r '.[0].EnclaveID')"
sudo nitro-cli console --enclave-id "$ENCLAVE_ID" | tee ~/attestation_console.txt
```

After you see `ATTESTATION_DOC_BASE64=...`, press `Ctrl-C`, then decode it:

```bash
python3 - <<'PY'
import base64, re, pathlib
data = pathlib.Path("/home/ec2-user/attestation_console.txt").read_bytes()
matches = re.findall(rb"ATTESTATION_DOC_BASE64=([A-Za-z0-9+/=\r\n]+)", data)
assert matches, "missing ATTESTATION_DOC_BASE64"
s = re.sub(rb"[^A-Za-z0-9+/=]", b"", matches[-1])
doc = base64.b64decode(s, validate=False)
pathlib.Path("/home/ec2-user/attestation_doc.bin").write_bytes(doc)
print(len(doc), doc[:32].hex())
PY
```

The run above produced a 4513-byte COSE attestation document.

## 6. Register Table From AWS

Open a reverse tunnel from your laptop to the AWS parent. Keep this terminal open:

```bash
ssh -N \
  -o ExitOnForwardFailure=yes \
  -o ServerAliveInterval=30 \
  -i "$WORK/aws-key.pem" \
  -R 8080:127.0.0.1:8080 \
  ec2-user@"$PUBLIC_IP"
```

On the AWS parent:

```bash
rsync -az --exclude node_modules -e "ssh -i $WORK/aws-key.pem" \
  "$REPO/aptos-move/move-examples/poker/clients/" \
  ec2-user@"$PUBLIC_IP":/home/ec2-user/poker-clients/

ssh -i "$WORK/aws-key.pem" ec2-user@"$PUBLIC_IP" '
set -euxo pipefail
cd ~/poker-clients
npm install --package-lock=false
NODE_URL=http://127.0.0.1:8080/v1 \
TABLE_PRIVATE_KEY='"$TABLE_PRIVATE_KEY"' \
POKER_MODULE_ADDRESS='"$TABLE_ADDRESS"' \
ATTESTATION_DOC_PATH=/home/ec2-user/attestation_doc.bin \
node table-client.js
'
```

Expected output includes:

```text
Table registered. Tx: 0x...
```

## 7. Run Player Flow Locally

Back on your laptop:

```bash
cd "$REPO/aptos-move/move-examples/poker/clients"
npm install --package-lock=false

NODE_URL=http://127.0.0.1:8080/v1 \
PLAYER_PRIVATE_KEY="$PLAYER_PRIVATE_KEY" \
POKER_MODULE_ADDRESS="$TABLE_ADDRESS" \
node player-client.js enter "$TABLE_ADDRESS" 1000

NODE_URL=http://127.0.0.1:8080/v1 \
PLAYER_PRIVATE_KEY="$PLAYER_PRIVATE_KEY" \
POKER_MODULE_ADDRESS="$TABLE_ADDRESS" \
node player-client.js balance "$TABLE_ADDRESS"

NODE_URL=http://127.0.0.1:8080/v1 \
PLAYER_PRIVATE_KEY="$PLAYER_PRIVATE_KEY" \
POKER_MODULE_ADDRESS="$TABLE_ADDRESS" \
node player-client.js leave "$TABLE_ADDRESS"
```

Settle the leaving player with the table signer:

```bash
"$REPO/target/debug/aptos" move run \
  --url http://127.0.0.1:8080 \
  --sender-account "$TABLE_ADDRESS" \
  --private-key-file "$WORK/keys/table.key" \
  --encoding hex \
  --function-id "${TABLE_ADDRESS}::poker::settle_leaving_players" \
  --args address:"$TABLE_ADDRESS" "address:[\"$PLAYER_ADDRESS\"]" \
  --max-gas 100000 \
  --assume-yes

NODE_URL=http://127.0.0.1:8080/v1 \
PLAYER_PRIVATE_KEY="$PLAYER_PRIVATE_KEY" \
POKER_MODULE_ADDRESS="$TABLE_ADDRESS" \
node player-client.js balance "$TABLE_ADDRESS"
```

Expected final chip balance:

```text
Chip balance: [ '0' ]
```

## Cleanup

Stop local long-running processes:

```bash
# Stop the localnet, faucet, and SSH tunnel terminals with Ctrl-C.
```

Terminate the enclave and EC2 resources:

```bash
ssh -i "$WORK/aws-key.pem" ec2-user@"$PUBLIC_IP" 'sudo nitro-cli terminate-enclave --all || true'
aws ec2 terminate-instances --instance-ids "$INSTANCE_ID"
aws ec2 wait instance-terminated --instance-ids "$INSTANCE_ID"
aws ec2 delete-key-pair --key-name "${AWS_NAME}-key"
aws ec2 delete-security-group --group-id "$SG_ID"
rm -f "$WORK/aws-key.pem"
```

## Notes

- `NODE_URL` is intentionally `http://.../v1` for these JS clients.
- `initialize_testnet_only` is for localnet/testnet-style development. Production root changes should go through governance.
- The smoke attester uses console output and debug mode for convenience. A production table runner should keep signing/table logic inside the enclave and send transactions through a parent vsock proxy.
