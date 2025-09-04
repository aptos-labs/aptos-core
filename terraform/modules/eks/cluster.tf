resource "aws_cloudwatch_log_group" "eks" {
  name              = "/aws/eks/velor-${local.workspace_name}/cluster"
  retention_in_days = 7
  tags              = local.default_tags
}

resource "aws_eks_cluster" "velor" {
  name                      = var.eks_cluster_name
  role_arn                  = aws_iam_role.cluster.arn
  version                   = var.kubernetes_version
  enabled_cluster_log_types = ["api", "audit", "authenticator", "controllerManager", "scheduler"]
  tags                      = local.default_tags

  vpc_config {
    subnet_ids              = concat(aws_subnet.public[*].id, aws_subnet.private[*].id)
    public_access_cidrs     = var.k8s_api_sources
    endpoint_private_access = true
    security_group_ids      = [aws_security_group.cluster.id]
  }

  lifecycle {
    ignore_changes = [
      # ignore autoupgrade version
      version,
    ]
  }

  depends_on = [
    aws_iam_role_policy_attachment.cluster-cluster,
    aws_iam_role_policy_attachment.cluster-service,
    aws_cloudwatch_log_group.eks,
  ]
}

data "aws_eks_cluster_auth" "velor" {
  name = aws_eks_cluster.velor.name
}

locals {
  pools = {
    utilities = {
      instance_type = var.utility_instance_type
      size          = 1
      taint         = false
    }
    fullnode = {
      instance_type = var.fullnode_instance_type
      size          = var.num_fullnodes + var.num_extra_instance
      taint         = false
    }
  }
}

resource "aws_launch_template" "nodes" {
  for_each      = local.pools
  name          = "velor-${local.workspace_name}/${each.key}"
  instance_type = each.value.instance_type

  block_device_mappings {
    device_name = "/dev/xvda"

    ebs {
      delete_on_termination = true
      volume_size           = 100
      volume_type           = "gp3"
    }
  }

  tag_specifications {
    resource_type = "instance"
    tags = merge(local.default_tags, {
      Name = "velor-${local.workspace_name}/${each.key}",
    })
  }
}

resource "aws_eks_node_group" "nodes" {
  for_each        = local.pools
  cluster_name    = aws_eks_cluster.velor.name
  node_group_name = each.key
  version         = aws_eks_cluster.velor.version
  node_role_arn   = aws_iam_role.nodes.arn
  subnet_ids      = [aws_subnet.private[0].id]
  tags            = local.default_tags

  lifecycle {
    ignore_changes = [
      # ignore autoupgrade version
      version,
      # ignore changes to the desired size that may occur due to cluster autoscaler
      scaling_config[0].desired_size,
      # ignore changes to max size, especially when it decreases to < desired_size, which fails
      scaling_config[0].max_size,
    ]
  }

  launch_template {
    id      = aws_launch_template.nodes[each.key].id
    version = aws_launch_template.nodes[each.key].latest_version
  }

  scaling_config {
    desired_size = lookup(var.node_pool_sizes, each.key, each.value.size)
    min_size     = 1
    max_size     = lookup(var.node_pool_sizes, each.key, each.value.size) * 2 # surge to twice the size if necessary
  }

  update_config {
    max_unavailable_percentage = 50
  }

  depends_on = [
    aws_iam_role_policy_attachment.nodes-node,
    aws_iam_role_policy_attachment.nodes-cni,
    aws_iam_role_policy_attachment.nodes-ecr,
    kubernetes_config_map.aws-auth,
  ]
}

resource "aws_iam_openid_connect_provider" "cluster" {
  client_id_list  = ["sts.amazonaws.com"]
  thumbprint_list = ["9e99a48a9960b14926bb7f3b02e22da2b0ab7280"] # Thumbprint of Root CA for EKS OIDC, Valid until 2037
  url             = aws_eks_cluster.velor.identity[0].oidc[0].issuer
}

locals {
  oidc_provider = replace(aws_iam_openid_connect_provider.cluster.url, "https://", "")
}

# EBS CSI ADDON

data "aws_iam_policy_document" "aws-ebs-csi-driver-trust-policy" {
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
      values   = ["system:serviceaccount:kube-system:ebs-csi-controller-sa"]
    }

    condition {
      test     = "StringEquals"
      variable = "${local.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "aws-ebs-csi-driver" {
  name                 = "velor-${local.workspace_name}-ebs-csi-controller"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.aws-ebs-csi-driver-trust-policy.json
}

resource "aws_iam_role_policy_attachment" "caws-ebs-csi-driver" {
  role = aws_iam_role.aws-ebs-csi-driver.name
  # From this reference: https://docs.aws.amazon.com/eks/latest/userguide/csi-iam-role.html
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEBSCSIDriverPolicy"
}

resource "aws_eks_addon" "aws-ebs-csi-driver" {
  cluster_name             = aws_eks_cluster.velor.name
  addon_name               = "aws-ebs-csi-driver"
  service_account_role_arn = aws_iam_role.aws-ebs-csi-driver.arn
}

# access control
data "aws_iam_policy_document" "cluster-autoscaler-assume-role" {
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
      # the name of the kube-system cluster-autoscaler service account
      values = ["system:serviceaccount:kube-system:cluster-autoscaler"]
    }

    condition {
      test     = "StringEquals"
      variable = "${local.oidc_provider}:aud"
      values   = ["sts.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "cluster-autoscaler" {
  statement {
    sid = "Autoscaling"
    actions = [
      "autoscaling:SetDesiredCapacity",
      "autoscaling:TerminateInstanceInAutoScalingGroup"
    ]
    resources = ["*"]
    condition {
      test     = "StringEquals"
      variable = "aws:ResourceTag/k8s.io/cluster-autoscaler/${aws_eks_cluster.velor.name}"
      values   = ["owned"]
    }
  }

  statement {
    sid = "DescribeAutoscaling"
    actions = [
      "autoscaling:DescribeLaunchConfigurations",
      "autoscaling:DescribeAutoScalingGroups",
      "autoscaling:DescribeAutoScalingInstances",
      "autoscaling:DescribeLaunchConfigurations",
      "autoscaling:DescribeScalingActivities",
      "autoscaling:DescribeTags",
      "ec2:DescribeInstanceTypes",
      "ec2:DescribeLaunchTemplateVersions",
      "ec2:DescribeImages",
      "ec2:GetInstanceTypesFromInstanceRequirements",
      "eks:DescribeNodegroup"
    ]
    resources = ["*"]
  }
}

resource "aws_iam_role" "cluster-autoscaler" {
  name                 = "velor-fullnode-${local.workspace_name}-cluster-autoscaler"
  path                 = var.iam_path
  permissions_boundary = var.permissions_boundary_policy
  assume_role_policy   = data.aws_iam_policy_document.cluster-autoscaler-assume-role.json
}

resource "aws_iam_role_policy" "cluster-autoscaler" {
  name   = "Helm"
  role   = aws_iam_role.cluster-autoscaler.name
  policy = data.aws_iam_policy_document.cluster-autoscaler.json
}

locals {
  autoscaling_helm_chart_path = "${path.module}/../../helm/autoscaling"
}

resource "helm_release" "autoscaling" {
  name        = "autoscaling"
  namespace   = "kube-system"
  chart       = local.autoscaling_helm_chart_path
  max_history = 5
  wait        = false

  values = [
    jsonencode({
      autoscaler = {
        enabled     = true
        clusterName = aws_eks_cluster.velor.name
        image = {
          # EKS does not report patch version
          tag = "v${aws_eks_cluster.velor.version}.0"
        }
        serviceAccount = {
          annotations = {
            "eks.amazonaws.com/role-arn" = aws_iam_role.cluster-autoscaler.arn
          }
        }
      }
    })
  ]

  # inspired by https://stackoverflow.com/a/66501021 to trigger redeployment whenever any of the charts file contents change.
  set {
    name  = "chart_sha1"
    value = sha1(join("", [for f in fileset(local.autoscaling_helm_chart_path, "**") : filesha1("${local.autoscaling_helm_chart_path}/${f}")]))
  }
}
