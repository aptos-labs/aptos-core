
# access control
data "aws_iam_policy_document" "indexer-assume-role" {
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]

    principals {
      type = "Federated"
      identifiers = [
        "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/${var.oidc_provider}"
      ]
    }

    condition {
      test     = "StringEquals"
      variable = "${var.oidc_provider}:sub"
      # the name of the default indexer service account
      values = ["system:serviceaccount:default:indexer"]
    }

    condition {
      test     = "StringEquals"
      variable = "${var.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "indexer" {
  statement {
    sid = "RDSWrite"
    actions = [
      "rds:*",
    ]
    resources = [
      aws_db_instance.indexer.arn
    ]
  }
}

resource "aws_iam_role" "indexer" {
  name                 = "aptos-testnet-${terraform.workspace}-indexer"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.indexer-assume-role.json
}

resource "aws_iam_role_policy" "indexer" {
  name   = "Helm"
  role   = aws_iam_role.indexer.name
  policy = data.aws_iam_policy_document.indexer.json
}
