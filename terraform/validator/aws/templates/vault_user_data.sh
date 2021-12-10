# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

#!/bin/sh
set -ex

cd /root

cat > "vault.sha256" <<EOF
${vault_sha256}  vault_${vault_version}_linux_amd64.zip
EOF

curl -O "https://releases.hashicorp.com/vault/${vault_version}/vault_${vault_version}_linux_amd64.zip"
sha256sum -c vault.sha256
unzip "vault_${vault_version}_linux_amd64.zip"
mv vault /usr/local/bin/

LOCAL_IPV4="$(curl -s http://169.254.169.254/latest/meta-data/local-ipv4)"

mkdir /etc/vault
cat > "/etc/vault/vault.json" <<EOF
${vault_config}
EOF

cat > "/etc/vault/vault.ca" <<EOF
${vault_ca}
EOF

cat > "/etc/vault/vault.crt" <<EOF
${vault_cert}
EOF

aws --region "${region}" secretsmanager get-secret-value --secret-id "${vault_key_secret}" --query 'SecretString' --output text > /etc/vault/vault.key

adduser --system --home /etc/vault.d --shell /bin/false vault

cat > "/etc/systemd/system/vault.service" <<EOF
[Unit]
Description="HashiCorp Vault - A tool for managing secrets"
Documentation=https://www.vaultproject.io/docs/
Requires=network-online.target
After=network-online.target
ConditionFileNotEmpty=/etc/vault/vault.json

[Service]
User=vault
Group=vault
ProtectSystem=full
ProtectHome=read-only
PrivateTmp=yes
PrivateDevices=yes
SecureBits=keep-caps
AmbientCapabilities=CAP_IPC_LOCK
Capabilities=CAP_IPC_LOCK+ep
CapabilityBoundingSet=CAP_SYSLOG CAP_IPC_LOCK
NoNewPrivileges=yes
ExecStart=/usr/local/bin/vault server -config=/etc/vault/vault.json
ExecReload=/bin/kill --signal HUP $MAINPID
KillMode=process
KillSignal=SIGINT
Restart=on-failure
RestartSec=5
TimeoutStopSec=30
StartLimitInterval=60
StartLimitBurst=3
LimitNOFILE=65536
LimitMEMLOCK=infinity

[Install]
WantedBy=multi-user.target
EOF

swapoff -a
sysctl 'kernel.core_pattern=|/bin/false'

systemctl enable vault
systemctl start vault
