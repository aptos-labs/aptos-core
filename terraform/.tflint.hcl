plugin "aws" {
  enabled = true
  version = "0.16.1"
  source  = "github.com/terraform-linters/tflint-ruleset-aws"
}

plugin "azurerm" {
  enabled = true
  version = "0.17.1"
  source  = "github.com/terraform-linters/tflint-ruleset-azurerm"
}

plugin "google" {
  enabled = true
  version = "0.19.0"
  source  = "github.com/terraform-linters/tflint-ruleset-google"
}
