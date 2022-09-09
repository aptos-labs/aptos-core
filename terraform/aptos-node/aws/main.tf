provider "aws" {
  region = var.region
}

data "aws_availability_zones" "available" {
  state = "available"
}

locals {
  aws_availability_zones = slice(sort(data.aws_availability_zones.available.names), 0, min(3, length(data.aws_availability_zones.available.names)))
  default_tags = {
    Terraform = "validator"
    Workspace = local.workspace_name
  }
  workspace_name = var.workspace_name_override == "" ? terraform.workspace : var.workspace_name_override
}

data "aws_caller_identity" "current" {}
