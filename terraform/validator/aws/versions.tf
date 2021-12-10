terraform {
  required_version = "~> 1.0.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
    }
    helm = {
      source  = "hashicorp/helm"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
    }
    local = {
      source  = "hashicorp/local"
    }
    null = {
      source  = "hashicorp/null"
    }
    random = {
      source  = "hashicorp/random"
    }
    template = {
      source  = "hashicorp/template"
    }
    time = {
      source  = "hashicorp/time"
    }
    tls = {
      source  = "hashicorp/tls"
    }
  }
}
