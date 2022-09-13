resource "random_id" "backup-bucket" {
  byte_length = 4
}

resource "aws_s3_bucket" "backup" {
  bucket = "aptos-${local.workspace_name}-backup-${random_id.backup-bucket.hex}"
}

resource "aws_s3_bucket_public_access_block" "backup" {
  bucket                  = aws_s3_bucket.backup.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
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
      values   = ["system:serviceaccount:default:pfn0-aptos-fullnode"]
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
  name                 = "aptos-${local.workspace_name}-backup"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.backup-assume-role.json
}

resource "aws_iam_role_policy" "backup" {
  name   = "Backup"
  role   = aws_iam_role.backup.name
  policy = data.aws_iam_policy_document.backup.json
}
