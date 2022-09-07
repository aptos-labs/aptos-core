provider "azuread" {}

terraform {
  required_version = "~> 1.2.0"
  required_providers {
    azuread = {
      source  = "hashicorp/azuread"
      version = "~> 2.27.0"
    }
    azurerm = {
      source = "hashicorp/azurerm"
    }
    helm = {
      source = "hashicorp/helm"
    }
    kubernetes = {
      source = "hashicorp/kubernetes"
    }
    local = {
      source = "hashicorp/local"
    }
    null = {
      source = "hashicorp/null"
    }
    random = {
      source = "hashicorp/random"
    }
    time = {
      source = "hashicorp/time"
    }
    tls = {
      source = "hashicorp/tls"
    }
  }
}
