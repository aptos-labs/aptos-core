data "aws_iam_policy_document" "eks-assume-role" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["eks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "cluster" {
  name                 = "aptos-${local.workspace_name}-cluster"
  path                 = var.iam_path
  assume_role_policy   = data.aws_iam_policy_document.eks-assume-role.json
  permissions_boundary = var.permissions_boundary_policy
  tags                 = local.default_tags
}

resource "aws_iam_role_policy_attachment" "cluster-cluster" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
  role       = aws_iam_role.cluster.name
}

resource "aws_iam_role_policy_attachment" "cluster-service" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSServicePolicy"
  role       = aws_iam_role.cluster.name
}

data "aws_iam_policy_document" "ec2-assume-role" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ec2.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "nodes" {
  name                 = "aptos-${local.workspace_name}-nodes"
  path                 = var.iam_path
  assume_role_policy   = data.aws_iam_policy_document.ec2-assume-role.json
  permissions_boundary = var.permissions_boundary_policy
  tags                 = local.default_tags
}

resource "aws_iam_instance_profile" "nodes" {
  name = "aptos-${local.workspace_name}-nodes"
  role = aws_iam_role.nodes.name
  path = var.iam_path
}

resource "aws_iam_role_policy_attachment" "nodes-node" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
  role       = aws_iam_role.nodes.name
}

resource "aws_iam_role_policy_attachment" "nodes-cni" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  role       = aws_iam_role.nodes.name
}

resource "aws_iam_role_policy_attachment" "nodes-ecr" {
  policy_arn = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
  role       = aws_iam_role.nodes.name
}
