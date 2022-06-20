# If Forge test framework is enabled on this testnet, also create and use
# an internal helm repository hosted on S3

resource "random_id" "helm-bucket" {
  count       = var.enable_forge ? 1 : 0
  byte_length = 4
}

resource "aws_s3_bucket" "aptos-testnet-helm" {
  count  = var.enable_forge ? 1 : 0
  bucket = "aptos-testnet-${local.workspace}-helm-${random_id.helm-bucket[0].hex}"
}

resource "aws_s3_bucket_public_access_block" "aptos-testnet-helm" {
  count                   = var.enable_forge ? 1 : 0
  bucket                  = aws_s3_bucket.aptos-testnet-helm[0].id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# install a helm repo called "testnet-${local.workspace}" at the s3 bucket
# this helm repo includes all the charts deployed onto a testnet
resource "null_resource" "helm-s3-init" {
  count = var.enable_forge ? 1 : 0
  depends_on = [
    aws_s3_bucket.aptos-testnet-helm
  ]

  triggers = {
    time = timestamp()
  }

  provisioner "local-exec" {
    command = <<-EOT
      helm plugin install https://github.com/C123R/helm-blob.git || true
      helm blob init s3://${aws_s3_bucket.aptos-testnet-helm[0].bucket}/charts
      helm repo add testnet-${local.workspace} s3://${aws_s3_bucket.aptos-testnet-helm[0].bucket}/charts
    EOT
  }
}

# package and push helm charts using a machine-controlled package directory
# NOTE: re-version the helm charts, as the helm s3 plugin does not like all SemVer
resource "null_resource" "helm-s3-package" {
  count = var.enable_forge ? 1 : 0
  depends_on = [
    null_resource.helm-s3-init
  ]

  # push the latest local changes
  triggers = {
    time = timestamp()
  }

  provisioner "local-exec" {
    command = <<-EOT
      set -e
      TEMPDIR="$(mktemp -d)"
      helm package ${path.module}/../helm/aptos-node -d "$TEMPDIR" --app-version 1.0.0 --version 1.0.0
      helm package ${path.module}/../helm/genesis -d "$TEMPDIR" --app-version 1.0.0 --version 1.0.0
      helm blob push --force "$TEMPDIR"/aptos-node-*.tgz testnet-${local.workspace}
      helm blob push --force "$TEMPDIR"/aptos-genesis-*.tgz testnet-${local.workspace}
      echo "pushed to internal helm repo"
    EOT
  }
}

# access control
data "aws_iam_policy_document" "forge-assume-role" {
  count = var.enable_forge ? 1 : 0
  statement {
    actions = ["sts:AssumeRoleWithWebIdentity"]

    principals {
      type = "Federated"
      identifiers = [
        "arn:aws:iam::${data.aws_caller_identity.current.account_id}:oidc-provider/${module.validator.oidc_provider}"
      ]
    }

    condition {
      test     = "StringEquals"
      variable = "${module.validator.oidc_provider}:sub"
      # the name of the default forge service account
      values = ["system:serviceaccount:default:forge"]
    }

    condition {
      test     = "StringEquals"
      variable = "${module.validator.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "forge" {
  count = var.enable_forge ? 1 : 0
  statement {
    sid = "HelmRead"
    actions = [
      "s3:GetObject",
      "s3:ListBucket",
    ]
    resources = [
      "arn:aws:s3:::${aws_s3_bucket.aptos-testnet-helm[0].id}",
      "arn:aws:s3:::${aws_s3_bucket.aptos-testnet-helm[0].id}/*"
    ]
  }

  statement {
    sid = "UpdateEksNodegroups"
    actions = [
      "eks:ListNodegroups",
      "eks:DescribeNodegroup",
      "eks:DescribeUpdate",
      "eks:UpdateNodegroupConfig",
      "eks:UpdateNodegroupVersion"
    ]
    resources = [
      module.validator.aws_eks_cluster.arn,
      "arn:aws:eks:${var.region}:${data.aws_caller_identity.current.account_id}:cluster/${module.validator.aws_eks_cluster.name}/*",
      "arn:aws:eks:${var.region}:${data.aws_caller_identity.current.account_id}:nodegroup/${module.validator.aws_eks_cluster.name}/*"
    ]
  }
}

resource "aws_iam_role" "forge" {
  count                = var.enable_forge ? 1 : 0
  name                 = "aptos-testnet-${local.workspace}-forge"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.forge-assume-role[0].json
}

resource "aws_iam_role_policy" "forge" {
  count  = var.enable_forge ? 1 : 0
  name   = "Helm"
  role   = aws_iam_role.forge[0].name
  policy = data.aws_iam_policy_document.forge[0].json
}

### Forge helm release


resource "helm_release" "forge" {
  count       = var.enable_forge ? 1 : 0
  name        = "forge"
  chart       = "${path.module}/../helm/forge"
  max_history = 2
  wait        = false

  depends_on = [
    null_resource.helm-s3-package
  ]

  values = [
    jsonencode({
      forge = {
        numValidators = var.num_validators
        helmBucket    = aws_s3_bucket.aptos-testnet-helm[0].bucket
        image = {
          tag = var.image_tag
        }
      }
      serviceAccount = {
        annotations = {
          "eks.amazonaws.com/role-arn" = aws_iam_role.forge[0].arn
        }
      }
    }),
    jsonencode(var.forge_helm_values),
  ]

  set {
    name  = "timestamp"
    value = timestamp()
  }
}

