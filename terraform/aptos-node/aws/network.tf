locals {
  num_azs = length(local.aws_availability_zones)

  # Maximize the capacity of the nodegroup in a single AZ. Otherwise, the CIDR ranges are divided equally.
  # This gives us a max of /17 for the first AZ, which supports 32,768 hosts. The number of pods this can support
  # varies, but with c5.4xlarge gets us ~600 validators. See https://github.com/awslabs/amazon-eks-ami/blob/master/files/eni-max-pods.txt
  # The other way to increase the cluster capacity is to allocate a new CIDR block to the VPC and associate
  # it via configuring CNI, which is more complex: https://aws.amazon.com/premiumsupport/knowledge-center/eks-multiple-cidr-ranges/
  num_other_subnets      = local.num_azs * 2 - 1
  max_subnet_cidr_ranges = cidrsubnets(var.vpc_cidr_block, 1, [for x in range(local.num_other_subnets) : 1 + ceil(pow(local.num_other_subnets, 0.5))]...)

  # The subnet CIDR ranges in the case we want a maximally large one
  max_private_subnet_cidr_ranges = slice(local.max_subnet_cidr_ranges, 0, local.num_azs)
  max_public_subnet_cidr_ranges  = slice(local.max_subnet_cidr_ranges, local.num_azs, local.num_azs * 2)

  # The subnet CIDR ranges in the case all are equally sized
  default_public_subnet_cidr_ranges  = [for x in range(local.num_azs) : cidrsubnet(cidrsubnet(aws_vpc.vpc.cidr_block, 1, 0), 2, x)]
  default_private_subnet_cidr_ranges = [for x in range(local.num_azs) : cidrsubnet(cidrsubnet(aws_vpc.vpc.cidr_block, 1, 1), 2, x)]

  public_subnet_cidr_ranges  = var.maximize_single_az_capacity ? local.max_public_subnet_cidr_ranges : local.default_public_subnet_cidr_ranges
  private_subnet_cidr_ranges = var.maximize_single_az_capacity ? local.max_private_subnet_cidr_ranges : local.default_private_subnet_cidr_ranges
}

resource "aws_vpc" "vpc" {
  cidr_block           = var.vpc_cidr_block
  enable_dns_hostnames = true

  tags = merge(local.default_tags, {
    Name                                                  = "aptos-${local.workspace_name}"
    "kubernetes.io/cluster/aptos-${local.workspace_name}" = "shared"
  })
}

resource "aws_subnet" "public" {
  count                   = local.num_azs
  vpc_id                  = aws_vpc.vpc.id
  cidr_block              = local.public_subnet_cidr_ranges[count.index]
  availability_zone       = local.aws_availability_zones[count.index]
  map_public_ip_on_launch = true

  tags = merge(local.default_tags, {
    Name                                                  = "aptos-${local.workspace_name}/public-${local.aws_availability_zones[count.index]}"
    "kubernetes.io/cluster/aptos-${local.workspace_name}" = "shared"
    "kubernetes.io/role/elb"                              = "1"
  })
}

resource "aws_internet_gateway" "public" {
  vpc_id = aws_vpc.vpc.id

  tags = merge(local.default_tags, {
    Name = "aptos-${local.workspace_name}"
  })
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.public.id
  }

  tags = merge(local.default_tags, {
    Name = "aptos-${local.workspace_name}/public"
  })
}

resource "aws_route_table_association" "public" {
  count          = local.num_azs
  subnet_id      = element(aws_subnet.public.*.id, count.index)
  route_table_id = aws_route_table.public.id
}

resource "aws_subnet" "private" {
  count             = local.num_azs
  vpc_id            = aws_vpc.vpc.id
  cidr_block        = local.private_subnet_cidr_ranges[count.index]
  availability_zone = local.aws_availability_zones[count.index]

  tags = merge(local.default_tags, {
    Name                                                  = "aptos-${local.workspace_name}/private-${local.aws_availability_zones[count.index]}"
    "kubernetes.io/cluster/aptos-${local.workspace_name}" = "shared"
    "kubernetes.io/role/internal-elb"                     = "1"
  })
}

resource "aws_eip" "nat" {
  vpc = true

  tags = merge(local.default_tags, {
    Name = "aptos-${local.workspace_name}-nat"
  })
}

resource "aws_nat_gateway" "private" {
  allocation_id = aws_eip.nat.id
  subnet_id     = aws_subnet.public[0].id
  tags          = local.default_tags
}

resource "aws_route_table" "private" {
  vpc_id = aws_vpc.vpc.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.private.id
  }

  tags = merge(local.default_tags, {
    Name = "aptos-${local.workspace_name}/private"
  })
}

resource "aws_route_table_association" "private" {
  count          = local.num_azs
  subnet_id      = element(aws_subnet.private.*.id, count.index)
  route_table_id = aws_route_table.private.id
}

resource "aws_security_group" "cluster" {
  name        = "aptos-${local.workspace_name}/cluster"
  description = "k8s masters"
  vpc_id      = aws_vpc.vpc.id

  tags = merge(local.default_tags, {
    "kubernetes.io/cluster/aptos-${local.workspace_name}" = "owned"
  })
}

resource "aws_security_group_rule" "cluster-api" {
  security_group_id        = aws_security_group.cluster.id
  type                     = "ingress"
  protocol                 = "tcp"
  from_port                = 443
  to_port                  = 443
  source_security_group_id = aws_security_group.nodes.id
  description              = "Allow API traffic from k8s nodes"
}

resource "aws_security_group_rule" "cluster-kubelet" {
  security_group_id        = aws_security_group.cluster.id
  type                     = "egress"
  protocol                 = "tcp"
  from_port                = 10250
  to_port                  = 10250
  source_security_group_id = aws_security_group.nodes.id
  description              = "Allow kubelet traffic to k8s nodes"
}

resource "aws_security_group" "nodes" {
  name        = "aptos-${local.workspace_name}/nodes"
  description = "k8s nodes"
  vpc_id      = aws_vpc.vpc.id

  tags = merge(local.default_tags, {
    "kubernetes.io/cluster/aptos-${local.workspace_name}" = "owned"
  })
}

resource "aws_security_group_rule" "nodes-tcp" {
  security_group_id        = aws_security_group.nodes.id
  type                     = "ingress"
  protocol                 = "tcp"
  from_port                = 1025
  to_port                  = 65535
  source_security_group_id = aws_security_group.nodes.id
  description              = "Allow TCP traffic between k8s nodes"
}

resource "aws_security_group_rule" "nodes-udp" {
  security_group_id        = aws_security_group.nodes.id
  type                     = "ingress"
  protocol                 = "udp"
  from_port                = 1025
  to_port                  = 65535
  source_security_group_id = aws_security_group.nodes.id
  description              = "Allow UDP traffic between k8s nodes"
}

resource "aws_security_group_rule" "nodes-icmp" {
  security_group_id        = aws_security_group.nodes.id
  type                     = "ingress"
  protocol                 = "icmp"
  from_port                = -1
  to_port                  = -1
  source_security_group_id = aws_security_group.nodes.id
  description              = "Allow ICMP traffic between k8s nodes"
}

resource "aws_security_group_rule" "nodes-dns" {
  security_group_id        = aws_security_group.nodes.id
  type                     = "ingress"
  protocol                 = "udp"
  from_port                = 53
  to_port                  = 53
  source_security_group_id = aws_security_group.nodes.id
  description              = "Allow DNS traffic between k8s nodes"
}

resource "aws_security_group_rule" "nodes-kubelet" {
  security_group_id        = aws_security_group.nodes.id
  type                     = "ingress"
  protocol                 = "tcp"
  from_port                = 10250
  to_port                  = 10250
  source_security_group_id = aws_security_group.cluster.id
  description              = "Allow kubelet traffic from k8s masters"
}

resource "aws_security_group_rule" "nodes-egress" {
  security_group_id = aws_security_group.nodes.id
  type              = "egress"
  protocol          = -1
  from_port         = 0
  to_port           = 0
  cidr_blocks       = ["0.0.0.0/0"]
  description       = "Allow all outgoing traffic"
}
