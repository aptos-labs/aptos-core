resource "aws_vpc" "vpc" {
  cidr_block           = var.vpc_cidr_block
  enable_dns_hostnames = true

  tags = merge(local.default_tags, {
    Name                                                  = "velor-${local.workspace_name}"
    "kubernetes.io/cluster/velor-${local.workspace_name}" = "shared"
  })
}

resource "aws_subnet" "public" {
  count                   = length(local.aws_availability_zones)
  vpc_id                  = aws_vpc.vpc.id
  cidr_block              = cidrsubnet(cidrsubnet(aws_vpc.vpc.cidr_block, 1, 0), 2, count.index)
  availability_zone       = local.aws_availability_zones[count.index]
  map_public_ip_on_launch = true

  tags = merge(local.default_tags, {
    Name                                                  = "velor-${local.workspace_name}/public-${local.aws_availability_zones[count.index]}"
    "kubernetes.io/cluster/velor-${local.workspace_name}" = "shared"
    "kubernetes.io/role/elb"                              = "1"
  })
}

resource "aws_internet_gateway" "public" {
  vpc_id = aws_vpc.vpc.id

  tags = merge(local.default_tags, {
    Name = "velor-${local.workspace_name}"
  })
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.public.id
  }

  tags = merge(local.default_tags, {
    Name = "velor-${local.workspace_name}/public"
  })
}

resource "aws_route_table_association" "public" {
  count          = length(local.aws_availability_zones)
  subnet_id      = element(aws_subnet.public[*].id, count.index)
  route_table_id = aws_route_table.public.id
}

resource "aws_subnet" "private" {
  count             = length(local.aws_availability_zones)
  vpc_id            = aws_vpc.vpc.id
  cidr_block        = cidrsubnet(cidrsubnet(aws_vpc.vpc.cidr_block, 1, 1), 2, count.index)
  availability_zone = local.aws_availability_zones[count.index]

  tags = merge(local.default_tags, {
    Name                                                  = "velor-${local.workspace_name}/private-${local.aws_availability_zones[count.index]}"
    "kubernetes.io/cluster/velor-${local.workspace_name}" = "shared"
    "kubernetes.io/role/internal-elb"                     = "1"
  })
}

resource "aws_eip" "nat" {
  vpc = true

  tags = merge(local.default_tags, {
    Name = "velor-${local.workspace_name}-nat"
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
    Name = "velor-${local.workspace_name}/private"
  })
}

resource "aws_route_table_association" "private" {
  count          = length(local.aws_availability_zones)
  subnet_id      = element(aws_subnet.private[*].id, count.index)
  route_table_id = aws_route_table.private.id
}

resource "aws_security_group" "cluster" {
  name        = "velor-${local.workspace_name}/cluster"
  description = "k8s masters"
  vpc_id      = aws_vpc.vpc.id

  tags = merge(local.default_tags, {
    "kubernetes.io/cluster/velor-${local.workspace_name}" = "owned"
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
  name        = "velor-${local.workspace_name}/nodes"
  description = "k8s nodes"
  vpc_id      = aws_vpc.vpc.id

  tags = merge(local.default_tags, {
    "kubernetes.io/cluster/velor-${local.workspace_name}" = "owned"
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

output "vpc_id" {
  value     = aws_vpc.vpc.id
  sensitive = true
}

output "aws_subnet_public" {
  value = aws_subnet.public
}

output "aws_subnet_private" {
  value = aws_subnet.private
}

output "aws_vpc_cidr_block" {
  value = aws_vpc.vpc.cidr_block
}

output "aws_eip_nat_public_ip" {
  value = aws_eip.nat.public_ip
}

output "cluster_security_group_id" {
  value = aws_security_group.cluster.id
}
