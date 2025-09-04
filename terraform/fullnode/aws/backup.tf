resource "random_id" "backup-bucket" {
  byte_length = 4
}

resource "aws_s3_bucket" "backup" {
  bucket = "velor-${local.workspace_name}-backup-${random_id.backup-bucket.hex}"
}

resource "aws_s3_bucket_public_access_block" "backup" {
  bucket                  = aws_s3_bucket.backup.id
  block_public_acls       = !var.enable_public_backup
  block_public_policy     = !var.enable_public_backup
  ignore_public_acls      = !var.enable_public_backup
  restrict_public_buckets = !var.enable_public_backup
}

resource "aws_s3_bucket_acl" "public-backup" {
  count  = var.enable_public_backup ? 1 : 0
  bucket = aws_s3_bucket.backup.id
  acl    = "public-read"
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
      # NOTE: assumes the deployment defaults: that namespace is default and helm release is pfn*
      values = [for i in range(var.num_fullnodes) : "system:serviceaccount:default:pfn${i}-velor-fullnode"]
    }

    condition {
      test     = "StringEquals"
      variable = "${local.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
  # Allow the AWS Backup service to assume this role
  statement {
    actions = ["sts:AssumeRole"]
    effect  = "Allow"

    principals {
      type        = "Service"
      identifiers = ["backup.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "backup" {
  statement {
    actions = [
      "s3:ListBucket",
      "s3:PutBucketAcl",
      "s3:PutObject",
      "s3:GetObject",
      "s3:GetObjectTagging",
      "s3:DeleteObject",
      "s3:DeleteObjectVersion",
      "s3:GetObjectVersion",
      "s3:GetObjectVersionTagging",
      "s3:GetObjectACL",
      "s3:PutObjectACL"
    ]
    resources = [
      "arn:aws:s3:::${aws_s3_bucket.backup.id}",
      "arn:aws:s3:::${aws_s3_bucket.backup.id}/*"
    ]
  }
}

resource "aws_iam_role" "backup" {
  name               = "velor-${local.workspace_name}-backup"
  path               = var.iam_path
  assume_role_policy = data.aws_iam_policy_document.backup-assume-role.json
}

resource "aws_iam_role_policy" "backup" {
  name   = "Backup"
  role   = aws_iam_role.backup.name
  policy = data.aws_iam_policy_document.backup.json
}
