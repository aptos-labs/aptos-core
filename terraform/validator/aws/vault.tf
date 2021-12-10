variable "ssh_pub_key" {
  description = "SSH public key to configure for bastion and vault access"
}

variable "ssh_sources_ipv4" {
  description = "List of CIDR subnets which can SSH to the bastion host"
  default     = ["0.0.0.0/0"]
}

variable "bastion_enable" {
  default     = false
  description = "Enable the bastion host for access to Vault"
}

variable "vault_num" {
  default     = 1
  description = "Number of Vault servers"
}

resource "tls_private_key" "ca-key" {
  algorithm   = "ECDSA"
  ecdsa_curve = "P256"
}

resource "tls_self_signed_cert" "ca" {
  key_algorithm         = "ECDSA"
  private_key_pem       = tls_private_key.ca-key.private_key_pem
  validity_period_hours = 10 * 365 * 24
  early_renewal_hours   = 1 * 365 * 24
  is_ca_certificate     = true
  allowed_uses          = ["cert_signing"]

  subject {
    common_name  = "Vault CA"
    organization = "diem-${local.workspace_name}"
  }
}

resource "local_file" "ca" {
  filename        = "${local.workspace_name}-vault.ca"
  content         = tls_self_signed_cert.ca.cert_pem
  file_permission = "0644"
}

resource "tls_private_key" "vault-key" {
  algorithm   = "ECDSA"
  ecdsa_curve = "P256"
}

resource "tls_cert_request" "vault" {
  key_algorithm   = tls_private_key.vault-key.algorithm
  private_key_pem = tls_private_key.vault-key.private_key_pem
  dns_names       = [aws_lb.vault.dns_name, "localhost"]
  ip_addresses    = ["127.0.0.1"]

  subject {
    common_name  = aws_lb.vault.dns_name
    organization = "diem-${local.workspace_name}"
  }
}

resource "tls_locally_signed_cert" "vault" {
  cert_request_pem      = tls_cert_request.vault.cert_request_pem
  ca_key_algorithm      = tls_private_key.ca-key.algorithm
  ca_private_key_pem    = tls_private_key.ca-key.private_key_pem
  ca_cert_pem           = tls_self_signed_cert.ca.cert_pem
  validity_period_hours = tls_self_signed_cert.ca.validity_period_hours
  early_renewal_hours   = tls_self_signed_cert.ca.early_renewal_hours
  allowed_uses          = ["server_auth"]
}

resource "aws_secretsmanager_secret" "vault-tls" {
  name                    = "diem-${local.workspace_name}/vault-tls"
  recovery_window_in_days = 0
  tags                    = local.default_tags
}

resource "aws_secretsmanager_secret_version" "vault-tls" {
  secret_id     = aws_secretsmanager_secret.vault-tls.id
  secret_string = tls_private_key.vault-key.private_key_pem
}

resource "aws_key_pair" "diem" {
  key_name   = "diem-${local.workspace_name}"
  public_key = var.ssh_pub_key
}

resource "aws_dynamodb_table" "vault" {
  name         = "diem-${local.workspace_name}-vault"
  billing_mode = "PAY_PER_REQUEST"
  tags         = local.default_tags

  hash_key  = "Path"
  range_key = "Key"

  attribute {
    name = "Path"
    type = "S"
  }
  attribute {
    name = "Key"
    type = "S"
  }

  point_in_time_recovery {
    enabled = true
  }

  lifecycle {
    # prevent_destroy = true
  }
}

resource "aws_kms_key" "vault" {
  description             = "diem-${local.workspace_name}/vault"
  deletion_window_in_days = 7

  tags = merge(local.default_tags, {
    Name = "diem-${local.workspace_name}/vault"
  })

  lifecycle {
    prevent_destroy = true
  }
}

data "aws_iam_policy_document" "vault" {
  statement {
    actions   = ["kms:Encrypt", "kms:Decrypt", "kms:DescribeKey"]
    resources = [aws_kms_key.vault.arn]
  }
  statement {
    actions   = ["secretsmanager:GetSecretValue"]
    resources = [aws_secretsmanager_secret.vault-tls.id]
  }
  statement {
    actions   = ["dynamodb:DescribeLimits", "dynamodb:DescribeTimeToLive", "dynamodb:ListTagsOfResource", "dynamodb:DescribeReservedCapacityOfferings", "dynamodb:DescribeReservedCapacity", "dynamodb:ListTables", "dynamodb:BatchGetItem", "dynamodb:BatchWriteItem", "dynamodb:DeleteItem", "dynamodb:GetItem", "dynamodb:GetRecords", "dynamodb:PutItem", "dynamodb:Query", "dynamodb:UpdateItem", "dynamodb:Scan", "dynamodb:DescribeTable"]
    resources = [aws_dynamodb_table.vault.arn]
  }
}

resource "aws_iam_role" "vault" {
  name                 = "diem-${local.workspace_name}-vault"
  path                 = var.iam_path
  assume_role_policy   = data.aws_iam_policy_document.ec2-assume-role.json
  permissions_boundary = var.permissions_boundary_policy
  tags                 = local.default_tags
}

resource "aws_iam_role_policy" "vault" {
  name   = "vault"
  role   = aws_iam_role.vault.name
  policy = data.aws_iam_policy_document.vault.json
}

resource "aws_iam_instance_profile" "vault" {
  name = "diem-${local.workspace_name}-vault"
  path = var.iam_path
  role = aws_iam_role.vault.name
}

resource "aws_security_group" "bastion" {
  name        = "diem-${local.workspace_name}/bastion"
  description = "SSH bastion server"
  vpc_id      = aws_vpc.vpc.id
  tags        = local.default_tags

  ingress {
    protocol    = "tcp"
    from_port   = 22
    to_port     = 22
    cidr_blocks = var.ssh_sources_ipv4
    description = "Allow SSH from whitelisted sources"
  }

  egress {
    protocol    = "tcp"
    from_port   = 22
    to_port     = 22
    cidr_blocks = [aws_vpc.vpc.cidr_block]
    description = "Allow SSH to other instances in this VPC"
  }

  egress {
    protocol    = "tcp"
    from_port   = 8200
    to_port     = 8200
    cidr_blocks = [aws_vpc.vpc.cidr_block]
    description = "Allow Vault protocol to other instances in this VPC"
  }
}

resource "aws_security_group" "vault" {
  name        = "diem-${local.workspace_name}/vault"
  description = "Vault servers"
  vpc_id      = aws_vpc.vpc.id
  tags        = local.default_tags

  ingress {
    protocol    = "tcp"
    from_port   = 8200
    to_port     = 8201
    self        = true
    description = "Allow Vault replication between Vault servers"
  }

  ingress {
    protocol    = "tcp"
    from_port   = 8200
    to_port     = 8200
    cidr_blocks = concat([aws_vpc.vpc.cidr_block, "${aws_eip.nat.public_ip}/32"], var.vault_sources_ipv4)
    description = "Allow Vault protocol from other instances in this VPC"
  }

  ingress {
    protocol        = "tcp"
    from_port       = 22
    to_port         = 22
    security_groups = [aws_security_group.bastion.id]
    description     = "Allow SSH from bastion server"
  }

  egress {
    protocol    = "tcp"
    from_port   = 8200
    to_port     = 8201
    self        = true
    description = "Allow Vault replication between Vault servers"
  }

  egress {
    protocol    = "tcp"
    from_port   = 443
    to_port     = 443
    cidr_blocks = ["0.0.0.0/0"]
    description = "Allow outgoing HTTPS traffic"
  }
}

resource "aws_security_group_rule" "cluster-vault" {
  security_group_id        = aws_security_group.cluster.id
  type                     = "ingress"
  protocol                 = "tcp"
  from_port                = 443
  to_port                  = 443
  source_security_group_id = aws_security_group.vault.id
  description              = "Allow API traffic from Vault servers"
}

data "aws_ami" "amazon" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["amzn2-ami-hvm-2.0.*-x86_64-gp2"]
  }
}

resource "aws_instance" "bastion" {
  count                       = var.bastion_enable ? 1 : 0
  ami                         = data.aws_ami.amazon.id
  instance_type               = "t3.nano"
  subnet_id                   = aws_subnet.public[0].id
  vpc_security_group_ids      = [aws_security_group.bastion.id]
  associate_public_ip_address = true
  key_name                    = aws_key_pair.diem.key_name
  user_data                   = file("${path.module}/templates/bastion_user_data.cloud")

  tags = merge(local.default_tags, {
    Name = "diem-${local.workspace_name}/bastion"
  })
}

data "template_file" "vault_user_data" {
  template = file("${path.module}/templates/vault_user_data.sh")

  vars = {
    region           = var.region
    vault_version    = "1.8.1"
    vault_sha256     = "bb411f2bbad79c2e4f0640f1d3d5ef50e2bda7d4f40875a56917c95ff783c2db"
    vault_ca         = tls_self_signed_cert.ca.cert_pem
    vault_cert       = tls_locally_signed_cert.vault.cert_pem
    vault_key_secret = aws_secretsmanager_secret.vault-tls.id
    vault_config = jsonencode({
      cluster_addr = "https://$LOCAL_IPV4:8201"
      api_addr     = "http://${aws_lb.vault.dns_name}:8200"
      storage = {
        dynamodb = {
          ha_enabled = "true"
          region     = var.region
          table      = aws_dynamodb_table.vault.name
        }
      }
      listener = {
        tcp = {
          address                 = "[::]:8200"
          tls_cert_file           = "/etc/vault/vault.crt"
          tls_key_file            = "/etc/vault/vault.key"
          proxy_protocol_behavior = "use_always"
          telemetry = {
            unauthenticated_metrics_access = true
          }
        }
      }
      seal = {
        awskms = { kms_key_id = aws_kms_key.vault.id }
      }
      telemetry = {
        disable_hostname = true
      }
    })
  }
}

resource "aws_launch_template" "vault" {
  name                   = "diem-${local.workspace_name}/vault"
  image_id               = data.aws_ami.amazon.id
  instance_type          = "c5.large"
  key_name               = aws_key_pair.diem.key_name
  vpc_security_group_ids = [aws_security_group.vault.id]
  user_data              = base64encode(data.template_file.vault_user_data.rendered)

  iam_instance_profile {
    arn = aws_iam_instance_profile.vault.arn
  }

  tags = merge(local.default_tags, {
    Name = "diem-${local.workspace_name}/vault"
  })
}

resource "aws_autoscaling_group" "vault" {
  name             = "diem-${local.workspace_name}/vault"
  desired_capacity = var.vault_num
  min_size         = var.vault_num
  max_size         = var.vault_num + 1

  launch_template {
    id      = aws_launch_template.vault.id
    version = "$Latest"
  }

  vpc_zone_identifier = aws_subnet.private.*.id
  target_group_arns   = [aws_lb_target_group.vault.arn]

  tag {
    key                 = "Name"
    value               = "diem-${local.workspace_name}/vault"
    propagate_at_launch = true
  }

  tag {
    key                 = "Terraform"
    value               = "validator"
    propagate_at_launch = true
  }

  tag {
    key                 = "Workspace"
    value               = local.workspace_name
    propagate_at_launch = true
  }
}

resource "aws_lb" "vault" {
  name                             = "diem-${local.workspace_name}-vault"
  internal                         = var.vault_lb_internal
  load_balancer_type               = "network"
  subnets                          = var.vault_lb_internal ? aws_subnet.private.*.id : aws_subnet.public.*.id
  enable_cross_zone_load_balancing = true
  tags                             = local.default_tags
}

resource "aws_lb_target_group" "vault" {
  name     = "diem-${local.workspace_name}-vault"
  port     = 8200
  protocol = "TCP"
  vpc_id   = aws_vpc.vpc.id
  tags     = local.default_tags

  health_check {
    path                = "/v1/sys/health"
    protocol            = "HTTPS"
    interval            = 10
    healthy_threshold   = 2
    unhealthy_threshold = 2
  }
}

resource "aws_lb_listener" "vault" {
  load_balancer_arn = aws_lb.vault.arn
  port              = 8200
  protocol          = "TCP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.vault.arn
  }
}

output "vault" {
  value     = local.vault
  sensitive = true
}
