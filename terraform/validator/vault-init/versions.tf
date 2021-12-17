terraform {
  required_version = "~> 1.0.0"
  required_providers {
    null = {
      source = "hashicorp/null"
    }
    vault = {
      source = "hashicorp/vault"
    }
  }
}
