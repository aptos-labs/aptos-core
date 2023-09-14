import * as pulumi from '@pulumi/pulumi';
import * as azure from '@pulumi/azure-native';
import * as azuread from '@pulumi/azuread';

export interface AuthConfig {
  location: string;
  resourceGroupName: pulumi.Output<string>;
  subnetId: pulumi.Output<string>;
  workspaceName: string;
}

export class Auth extends pulumi.ComponentResource {

  public readonly applicationlId: pulumi.Output<string>;
  public readonly servicePrincipalPassword: pulumi.Output<string>;

  constructor(name: string, args: AuthConfig, opts?: pulumi.ComponentResourceOptions) {
    super("aptos-node:azure:Auth", name, {}, opts);

    const options = {
      parent: this,
      deleteBeforeReplace: true,
    };

    // Create Azure AD Application
    const aptosApplication = new azuread.Application(`${name}-aptos`, {
      displayName: `aptos-${args.workspaceName}/cluster`,
      preventDuplicateNames: true,
    }, options);

    // Create Azure AD Service Principal
    const aptosServicePrincipal = new azuread.ServicePrincipal(`${name}-aptos`, {
      applicationId: aptosApplication.applicationId,
    }, options);

    // Create Azure AD Application Password
    const aptosServicePrincipalPassword = new azuread.ApplicationPassword(`${name}-aptos`, {
      applicationObjectId: aptosApplication.objectId,
      endDateRelative: "8760h",
    }, options);

    // Create Azure Role Assignment
    new azure.authorization.RoleAssignment(`${name}-subnet`, {
      principalId: aptosServicePrincipal.id,
      principalType: "ServicePrincipal",
      roleDefinitionId: "/providers/Microsoft.Authorization/roleDefinitions/4d97b98b-1d4f-4787-a291-c67834d212e7",
      scope: args.subnetId,
    }, options);

    // Create Azure User Assigned Identity
    new azure.managedidentity.UserAssignedIdentity(`${name}-vault`, {
      resourceName: `aptos-${args.workspaceName}-vault`,
      // Replace with your actual resource group name and location
      resourceGroupName: args.resourceGroupName,
      location: args.location,
    }, options);

    this.applicationlId = aptosApplication.applicationId;
    this.servicePrincipalPassword = aptosServicePrincipalPassword.value;
  }
}
