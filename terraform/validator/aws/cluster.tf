resource "aws_cloudwatch_log_group" "eks" {
  name              = "/aws/eks/diem-${local.workspace_name}/cluster"
  retention_in_days = 7
  tags              = local.default_tags
}

resource "aws_eks_cluster" "diem" {
  name                      = "diem-${local.workspace_name}"
  role_arn                  = aws_iam_role.cluster.arn
  version                   = "1.21"
  enabled_cluster_log_types = ["api", "audit", "authenticator", "controllerManager", "scheduler"]
  tags                      = local.default_tags

  vpc_config {
    subnet_ids              = concat(aws_subnet.public.*.id, aws_subnet.private.*.id)
    public_access_cidrs     = var.k8s_api_sources
    endpoint_private_access = true
    security_group_ids      = [aws_security_group.cluster.id]
  }

  depends_on = [
    aws_iam_role_policy_attachment.cluster-cluster,
    aws_iam_role_policy_attachment.cluster-service,
    aws_cloudwatch_log_group.eks,
  ]

  lifecycle {
    prevent_destroy = true
  }
}

data "aws_eks_cluster_auth" "diem" {
  name = aws_eks_cluster.diem.name
}

locals {
  pools = {
    utilities = {
      instance_type = var.utility_instance_type
      size          = 3
      taint         = false
    }
    validators = {
      instance_type = var.validator_instance_type
      size          = 3
      taint         = true
    }
    trusted = {
      instance_type = var.trusted_instance_type
      size          = 1
      taint         = true
    }
  }
}

data "template_file" "user_data" {
  for_each = local.pools
  template = file("${path.module}/templates/eks_user_data.sh")

  vars = {
    taints = each.value.taint ? "diem.org/nodepool=${each.key}:NoExecute" : ""
  }
}

resource "aws_launch_template" "nodes" {
  for_each      = local.pools
  name          = "diem-${local.workspace_name}/${each.key}"
  instance_type = each.value.instance_type
  user_data     = base64encode(data.template_file.user_data[each.key].rendered)

  tag_specifications {
    resource_type = "instance"
    tags = merge(local.default_tags, {
      Name = "diem-${local.workspace_name}/${each.key}",
    })
  }
}

resource "aws_eks_node_group" "nodes" {
  for_each        = local.pools
  cluster_name    = aws_eks_cluster.diem.name
  node_group_name = each.key
  version         = aws_eks_cluster.diem.version
  node_role_arn   = aws_iam_role.nodes.arn
  subnet_ids      = [aws_subnet.private[0].id]
  tags            = local.default_tags

  launch_template {
    id      = aws_launch_template.nodes[each.key].id
    version = aws_launch_template.nodes[each.key].latest_version
  }

  scaling_config {
    desired_size = lookup(var.node_pool_sizes, each.key, each.value.size)
    min_size     = lookup(var.node_pool_sizes, each.key, each.value.size)
    max_size     = lookup(var.node_pool_sizes, each.key, each.value.size) * var.max_node_pool_surge
  }

  update_config {
    max_unavailable = var.max_node_pool_surge > 1 ? lookup(var.node_pool_sizes, each.key, each.value.size) * (var.max_node_pool_surge - 1) : 1
  }

  depends_on = [
    aws_iam_role_policy_attachment.nodes-node,
    aws_iam_role_policy_attachment.nodes-cni,
    aws_iam_role_policy_attachment.nodes-ecr,
    kubernetes_config_map.aws-auth,
  ]
}
