terraform {
  required_version = "~> 1.3.6"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 4.54.0"
    }
    google-beta = {
      source  = "hashicorp/google-beta"
      version = "~> 4.54.0"
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
