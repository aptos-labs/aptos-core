resource "aws_cloudwatch_log_group" "eks" {
  name              = "/aws/eks/velor-${local.workspace_name}/cluster"
  retention_in_days = 7
  tags              = local.default_tags
}

resource "aws_eks_cluster" "velor" {
  name                      = "velor-${local.workspace_name}"
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
      min_size      = var.utility_instance_min_num
      desired_size  = var.utility_instance_num
      max_size      = var.utility_instance_max_num > 0 ? var.utility_instance_max_num : 2 * var.utility_instance_num
      taint         = var.utility_instance_enable_taint
    }
    validators = {
      instance_type = var.validator_instance_type
      min_size      = var.validator_instance_min_num
      desired_size  = var.validator_instance_num
      max_size      = var.validator_instance_max_num > 0 ? var.validator_instance_max_num : 2 * var.validator_instance_num
      taint         = var.validator_instance_enable_taint
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

  # if the NodeGroup should be tainted, then create the below dynamic block
  dynamic "taint" {
    for_each = each.value.taint ? [local.pools[each.key]] : []
    content {
      key    = "velor.org/nodepool"
      value  = each.key
      effect = "NO_EXECUTE"
    }
  }

  launch_template {
    id      = aws_launch_template.nodes[each.key].id
    version = aws_launch_template.nodes[each.key].latest_version
  }

  scaling_config {
    desired_size = each.value.desired_size
    min_size     = each.value.min_size
    max_size     = each.value.max_size
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
