terraform {
  required_version = "~> 1.0.0"
  required_providers {
    google = {
      source  = "hashicorp/google"
    }
    google-beta = {
      source  = "hashicorp/google-beta"
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
