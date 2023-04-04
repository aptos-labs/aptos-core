resource "azuread_application" "aptos" {
  display_name            = "aptos-${terraform.workspace}/cluster"
  prevent_duplicate_names = true
}

resource "azuread_service_principal" "aptos" {
  application_id = azuread_application.aptos.application_id
}

//  Per https://registry.terraform.io/providers/hashicorp/azuread/latest/docs/resources/application_password,
//  SP I am authenticated with  must have permissions to both Read and Write all applications and Sign in and Read user profile within the Windows Azure Active Directory API
resource "azuread_application_password" "aptos" {
  application_object_id = azuread_application.aptos.object_id
  end_date_relative     = "8760h"
}

resource "azurerm_role_assignment" "subnet" {
  principal_id         = azuread_service_principal.aptos.id
  role_definition_name = "Network Contributor"
  scope                = azurerm_subnet.nodes.id
}

resource "azurerm_user_assigned_identity" "vault" {
  name                = "aptos-${terraform.workspace}-vault"
  resource_group_name = azurerm_resource_group.aptos.name
  location            = azurerm_resource_group.aptos.location
}
