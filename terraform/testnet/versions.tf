terraform {
  required_version = "~> 1.1.0"
  required_providers {
    aws = {
      source = "hashicorp/aws"
    }
    helm = {
      source = "hashicorp/helm"
    }
    null = {
      source = "hashicorp/null"
    }
    random = {
      source = "hashicorp/random"
    }
    vault = {
      source  = "hashicorp/vault"
    }
  }
}
