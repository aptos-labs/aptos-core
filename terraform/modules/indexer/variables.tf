variable "region" {
  description = "AWS region"
  type        = string
}

variable "image_tag" {
  default     = "devnet"
  description = "Image tag for indexer"
}

variable "iam_path" {
  default     = "/"
  description = "Path to use when naming IAM objects"
}

variable "permissions_boundary_policy" {
  default     = ""
  description = "ARN of IAM policy to set as permissions boundary on created roles"
}

variable "workspace_name_override" {
  description = "If specified, overrides the usage of Terraform workspace for naming purposes"
  default     = ""
}

variable "subnet_ids" {
  description = "Subnet IDs to create the DB subnet groups"
}

variable "vpc_id" {
  description = "VPC ID to create resources in"
}

variable "db_sources_ipv4" {
  description = "List of CIDR subnets which can access the DB"
  default     = ["0.0.0.0/0"]
}

variable "indexer_helm_values" {
  default = {}
}

variable "db_password" {
  description = "RDS root user password"
  type        = string
  sensitive   = true
}

variable "oidc_provider" {}

variable "node_url" {
  description = "REST API endpoint to pull blockchain data from"
}

variable "db_instance_class" {
  description = "Instance class of the RDS DB"
  default     = "db.t3.micro"
}

variable "db_engine_version" {
  description = "Engine version for the RDS DB"
  default     = "14.1"
}

variable "db_engine" {
  description = "Engine name for the RDS DB"
  default     = "postgres"
}

variable "db_allocated_storage" {
  description = "Allocated storage in GB for the RDS DB"
  default     = 100
}

variable "db_max_allocated_storage" {
  description = "Max allocated storage in GB for the RDS DB, enabling autoscaling"
  default     = 500
}

variable "db_parameter_group_family" {
  description = "Parameter group family name for the RDS DB. Must be compatible with the db_engine and db_engine_version. https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/USER_WorkingWithDBInstanceParamGroups.html"
  default     = "postgres14"
}

variable "db_publicly_accessible" {
  default     = false
  description = "Determines if RDS instance is publicly accessible"
}
