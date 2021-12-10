resource "random_id" "backup-bucket" {
  byte_length = 4
}

resource "aws_s3_bucket" "backup" {
  bucket = "diem-${local.workspace_name}-backup-${random_id.backup-bucket.hex}"
  tags   = local.default_tags
}

resource "aws_s3_bucket_public_access_block" "backup" {
  bucket                  = aws_s3_bucket.backup.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_iam_openid_connect_provider" "cluster" {
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = ["9e99a48a9960b14926bb7f3b02e22da2b0ab7280"] # Thumbprint of Root CA for EKS OIDC, Valid until 2037
  url             = aws_eks_cluster.diem.identity[0].oidc[0].issuer
}

locals {
  oidc_provider = replace(aws_iam_openid_connect_provider.cluster.url, "https://", "")
}

data "aws_iam_policy_document" "backup-assume-role" {
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]

    principals {
      type = "Federated"
      identifiers = [
        "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/${local.oidc_provider}"
      ]
    }

    condition {
      test     = "StringEquals"
      variable = "${local.oidc_provider}:sub"
      values   = ["system:serviceaccount:default:${var.helm_release_name != "" ? var.helm_release_name : local.workspace_name}-diem-validator-backup"]
    }

    condition {
      test     = "StringEquals"
      variable = "${local.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "backup" {
  statement {
    actions = [
      "s3:GetObject",
      "s3:ListBucket",
      "s3:PutObject",
    ]
    resources = [
      "arn:aws:s3:::${aws_s3_bucket.backup.id}",
      "arn:aws:s3:::${aws_s3_bucket.backup.id}/*"
    ]
  }
}

resource "aws_iam_role" "backup" {
  name                 = "diem-${local.workspace_name}-backup"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.backup-assume-role.json
  tags                 = local.default_tags
}

resource "aws_iam_role_policy" "backup" {
  name   = "Backup"
  role   = aws_iam_role.backup.name
  policy = data.aws_iam_policy_document.backup.json
}

output "oidc_provider" {
  value     = local.oidc_provider
  sensitive = true
}
