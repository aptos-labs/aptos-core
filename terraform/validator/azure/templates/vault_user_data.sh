# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

#!/bin/sh
set -ex

gpg --dearmor > /etc/apt/trusted.gpg.d/microsoft.gpg <<EOF
-----BEGIN PGP PUBLIC KEY BLOCK-----
Version: GnuPG v1.4.7 (GNU/Linux)

mQENBFYxWIwBCADAKoZhZlJxGNGWzqV+1OG1xiQeoowKhssGAKvd+buXCGISZJwT
LXZqIcIiLP7pqdcZWtE9bSc7yBY2MalDp9Liu0KekywQ6VVX1T72NPf5Ev6x6DLV
7aVWsCzUAF+eb7DC9fPuFLEdxmOEYoPjzrQ7cCnSV4JQxAqhU4T6OjbvRazGl3ag
OeizPXmRljMtUUttHQZnRhtlzkmwIrUivbfFPD+fEoHJ1+uIdfOzZX8/oKHKLe2j
H632kvsNzJFlROVvGLYAk2WRcLu+RjjggixhwiB+Mu/A8Tf4V6b+YppS44q8EvVr
M+QvY7LNSOffSO6Slsy9oisGTdfE39nC7pVRABEBAAG0N01pY3Jvc29mdCAoUmVs
ZWFzZSBzaWduaW5nKSA8Z3Bnc2VjdXJpdHlAbWljcm9zb2Z0LmNvbT6JATUEEwEC
AB8FAlYxWIwCGwMGCwkIBwMCBBUCCAMDFgIBAh4BAheAAAoJEOs+lK2+EinPGpsH
/32vKy29Hg51H9dfFJMx0/a/F+5vKeCeVqimvyTM04C+XENNuSbYZ3eRPHGHFLqe
MNGxsfb7C7ZxEeW7J/vSzRgHxm7ZvESisUYRFq2sgkJ+HFERNrqfci45bdhmrUsy
7SWw9ybxdFOkuQoyKD3tBmiGfONQMlBaOMWdAsic965rvJsd5zYaZZFI1UwTkFXV
KJt3bp3Ngn1vEYXwijGTa+FXz6GLHueJwF0I7ug34DgUkAFvAs8Hacr2DRYxL5RJ
XdNgj4Jd2/g6T9InmWT0hASljur+dJnzNiNCkbn9KbX7J/qK1IbR8y560yRmFsU+
NdCFTW7wY0Fb1fWJ+/KTsC4=
=J6gs
-----END PGP PUBLIC KEY BLOCK-----
EOF

cat > /etc/apt/sources.list.d/azure-cli.list <<EOF
deb [arch=amd64] https://packages.microsoft.com/repos/azure-cli/ bionic main
EOF

apt-get update
apt-get -y install unzip azure-cli

cd /root

cat > "vault.sha256" <<EOF
${vault_sha256}  vault_${vault_version}_linux_amd64.zip
EOF

curl -O "https://releases.hashicorp.com/vault/${vault_version}/vault_${vault_version}_linux_amd64.zip"
sha256sum -c vault.sha256
unzip "vault_${vault_version}_linux_amd64.zip"
mv vault /usr/local/bin/

LOCAL_IPV4="$(ip -o -4 addr show scope global type inet | awk '{print $4}' | cut -d/ -f1)"

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

az login --identity --allow-no-subscriptions
az keyvault secret show --vault-name "${vault_key_vault}" --name "${vault_key_secret}" | python -c 'import sys, json; print(json.loads(sys.stdin.read())["value"])' > /etc/vault/vault.key

adduser --system --group --home /etc/vault --shell /bin/false vault

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
