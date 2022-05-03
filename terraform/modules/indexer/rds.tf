resource "aws_db_subnet_group" "indexer" {
  name       = "indexer-${local.workspace_name}"
  subnet_ids = var.subnet_ids

  tags = local.default_tags
}

resource "aws_security_group" "indexer" {
  name   = "indexer-${local.workspace_name}"
  vpc_id = var.vpc_id
  tags   = local.default_tags

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = var.db_sources_ipv4
  }
}

resource "aws_db_parameter_group" "indexer" {
  name = "indexer-${local.workspace_name}"
  # family parameter must correspond with the engine version of aws_db_instance.indexer
  # aws rds describe-db-engine-versions --query "DBEngineVersions[].DBParameterGroupFamily"
  family = var.db_parameter_group_family

  parameter {
    name  = "log_connections"
    value = "1"
  }
}


resource "aws_db_instance" "indexer" {
  identifier = "indexer-${local.workspace_name}"

  instance_class        = var.db_instance_class
  allocated_storage     = var.db_allocated_storage
  max_allocated_storage = var.db_max_allocated_storage

  engine                 = var.db_engine
  engine_version         = var.db_engine_version
  username               = "indexer"
  password               = var.db_password
  db_subnet_group_name   = aws_db_subnet_group.indexer.name
  vpc_security_group_ids = [aws_security_group.indexer.id]
  parameter_group_name   = aws_db_parameter_group.indexer.name
  publicly_accessible    = var.db_publicly_accessible
  skip_final_snapshot    = true
}

resource "kubernetes_secret" "indexer_credentials" {
  metadata {
    name      = "indexer-credentials"
    namespace = "default"
  }

  # TODO(rustielin): replace assumptions on db name
  data = {
    pg_db_uri = "postgresql://${aws_db_instance.indexer.address}:5432/postgres?user=${aws_db_instance.indexer.username}&password=${var.db_password}"
  }
}
