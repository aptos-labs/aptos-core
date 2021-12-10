# Copyright (c) The Diem Core Contributors
# SPDX-License-Identifier: Apache-2.0

MIME-Version: 1.0
Content-Type: multipart/mixed; boundary="==eef105b1-a7ca-4eb3-9db2-64bad3373176=="

--==eef105b1-a7ca-4eb3-9db2-64bad3373176==
Content-Type: text/x-shellscript; charset="us-ascii"

#!/bin/sh
set -ex

# Block access from pods to EC2 instance metadata
# https://docs.aws.amazon.com/eks/latest/userguide/restrict-ec2-credential-access.html
yum install -y iptables-services
# Dropping in the mangle table is a workaround for conflicts with calico rules
cat > /etc/sysconfig/iptables <<EOF
*mangle
-A FORWARD -d 169.254.169.254/32 -i eni+ -j DROP
COMMIT
EOF
systemctl enable --now iptables

# Edit the EKS inserted script to set --register-with-taints
sed -i 's,--node-labels=,--register-with-taints=${taints} --node-labels=,' "$(dirname "$0")/part-002"

--==eef105b1-a7ca-4eb3-9db2-64bad3373176==--
