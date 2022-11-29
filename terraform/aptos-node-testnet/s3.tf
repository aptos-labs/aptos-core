# Creates an s3 bucket to use internally for loading testnet data
resource "random_id" "testnet-bucket" {
  byte_length = 4
}

resource "aws_s3_bucket" "testnet-bucket" {
  bucket = "aptos-${local.workspace_name}-testnet-${random_id.testnet-bucket.hex}"
}

resource "aws_s3_bucket_public_access_block" "testnet-bucket" {
  bucket                  = aws_s3_bucket.testnet-bucket.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_lifecycle_configuration" "testnet-bucket-genesis" {
  bucket = aws_s3_bucket.testnet-bucket.id

  rule {
    id = "expire-genesis-data"

    filter {
      prefix = "genesis/"
    }
    expiration {
      days = var.genesis_s3_retention_days == "" ? 1 : tonumber(var.genesis_s3_retention_days)
    }
    status = var.genesis_s3_retention_days == "" ? "Disabled" : "Enabled"
  }
}


data "aws_iam_policy_document" "testnet-bucket-assume-role" {
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]

    principals {
      type = "Federated"
      identifiers = [
        "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/${module.validator.oidc_provider}"
      ]
    }

    condition {
      test     = "StringLike"
      variable = "${module.validator.oidc_provider}:sub"
      # Genesis serviceaccounts need access to publish the genesis data to S3
      # Validator and fullnode serviceaccounts need access to pull genesis data from S3
      values = [
        "system:serviceaccount:*:genesis-aptos-genesis",
        "system:serviceaccount:*:${local.aptos_node_helm_prefix}-validator",
        "system:serviceaccount:*:${local.aptos_node_helm_prefix}-fullnode",
      ]
    }

    condition {
      test     = "StringEquals"
      variable = "${module.validator.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "testnet-bucket" {
  statement {
    sid = "AllowS3"
    actions = [
      "s3:*",
    ]
    resources = [
      "arn:aws:s3:::${aws_s3_bucket.testnet-bucket.id}",
      "arn:aws:s3:::${aws_s3_bucket.testnet-bucket.id}/*"
    ]
  }
}

resource "aws_iam_role" "testnet-bucket" {
  name                 = "aptos-node-testnet-${local.workspace_name}-bucket"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.testnet-bucket-assume-role.json
}

resource "aws_iam_role_policy" "testnet-bucket" {
  name   = "Helm"
  role   = aws_iam_role.testnet-bucket.name
  policy = data.aws_iam_policy_document.testnet-bucket.json
}

