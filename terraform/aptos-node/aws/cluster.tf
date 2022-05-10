resource "aws_cloudwatch_log_group" "eks" {
  name              = "/aws/eks/aptos-${local.workspace_name}/cluster"
  retention_in_days = 7
  tags              = local.default_tags
}

resource "aws_eks_cluster" "aptos" {
  name                      = "aptos-${local.workspace_name}"
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
}

data "aws_eks_cluster_auth" "aptos" {
  name = aws_eks_cluster.aptos.name
}

locals {
  pools = {
    utilities = {
      instance_type = var.utility_instance_type
      size          = var.utility_instance_num
      taint         = false
    }
    validators = {
      instance_type = var.validator_instance_type
      size          = var.validator_instance_num
      taint         = true
    }
  }
}

resource "aws_launch_template" "nodes" {
  for_each      = local.pools
  name          = "aptos-${local.workspace_name}/${each.key}"
  instance_type = each.value.instance_type
  user_data     = base64encode(
    templatefile("${path.module}/templates/eks_user_data.sh", {
      taints = each.value.taint ? "aptos.org/nodepool=${each.key}:NoExecute" : ""
    })
  )

  tag_specifications {
    resource_type = "instance"
    tags = merge(local.default_tags, {
      Name = "aptos-${local.workspace_name}/${each.key}",
    })
  }
}

resource "aws_eks_node_group" "nodes" {
  for_each        = local.pools
  cluster_name    = aws_eks_cluster.aptos.name
  node_group_name = each.key
  version         = aws_eks_cluster.aptos.version
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
    max_size     = lookup(var.node_pool_sizes, each.key, each.value.size)
  }

  update_config {
    max_unavailable = 1
  }

  depends_on = [
    aws_iam_role_policy_attachment.nodes-node,
    aws_iam_role_policy_attachment.nodes-cni,
    aws_iam_role_policy_attachment.nodes-ecr,
    kubernetes_config_map.aws-auth,
  ]
}
