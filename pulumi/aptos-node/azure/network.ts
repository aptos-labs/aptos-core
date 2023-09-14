import * as pulumi from '@pulumi/pulumi';
import * as azure from "@pulumi/azure-native";
import * as std from "@pulumi/std";

export interface NetworkConfig {
  location: string
  resourceGroupName: pulumi.Output<string>;
  vnetAddress: string;
  workspaceName: string;
}

export class Network extends pulumi.ComponentResource {
  public readonly vNetId: pulumi.Output<string>;
  public readonly vNetName: pulumi.Output<string>;
  public readonly nodesSubnetId: pulumi.Output<string>;
  public readonly nodesSubnetName: pulumi.Output<string>;
  public readonly nodesSubnetAddressPrefixes: pulumi.Output<string[]>;
  public readonly nodesSecurityGroupName: pulumi.Output<string>;
  public readonly natId: pulumi.Output<string>;
  public readonly natName: pulumi.Output<string>;
  public readonly natIpAddress: pulumi.Output<string>;
  // public readonly natGatewayId: pulumi.Output<string>;
  // public readonly natGatewayName: pulumi.Output<string>;

  constructor(name: string, args: NetworkConfig, opts?: pulumi.ComponentResourceOptions) {
    super("aptos-node:azure:Network", name, {}, opts);

    const options = {
      parent: this,
      deleteBeforeReplace: true,
    };

    const virtualNetwork = new azure.network.VirtualNetwork(`${name}-virtualNetwork`, {
      virtualNetworkName: `aptos-${args.workspaceName}`,
      resourceGroupName: args.resourceGroupName,
      location: args.location,
      addressSpace: {
        addressPrefixes: [args.vnetAddress],
      }
    }, { ...options, ignoreChanges: ["subnets"] });

    const addressPrefix = std.cidrsubnetOutput({
      input: args.vnetAddress,
      newbits: 4,
      netnum: 0,
    });

    const nodesSecurityGroup = new azure.network.NetworkSecurityGroup(`${name}-nodes-nsg`, {
      networkSecurityGroupName: `aptos-${args.workspaceName}-nodes`,
      resourceGroupName: args.resourceGroupName,
      location: args.location,
      securityRules: [
        {
          name: "allow-load-balancer-inbound",
          priority: 3000,
          direction: "Inbound",
          access: "Allow",
          protocol: "*",
          destinationAddressPrefix: "*",
          destinationPortRange: "*",
          sourceAddressPrefix: "AzureLoadBalancer",
          sourcePortRange: "*",
        },
        {
          name: "allow-internet-inbound",
          priority: 3010,
          direction: "Inbound",
          access: "Allow",
          protocol: "*",
          destinationAddressPrefix: "*",
          destinationPortRange: "*",
          sourceAddressPrefix: "Internet",
          sourcePortRange: "*",
        },
        {
          name: "deny-all-inbound",
          priority: 4000,
          direction: "Inbound",
          access: "Deny",
          protocol: "*",
          destinationAddressPrefix: "*",
          destinationPortRange: "*",
          sourceAddressPrefix: "*",
          sourcePortRange: "*",
        },
      ]
    }, options);

    const nodesSubnet = new azure.network.Subnet(`${name}-nodes-sub`, {
      name: "nodes",
      resourceGroupName: args.resourceGroupName,
      virtualNetworkName: virtualNetwork.name,
      addressPrefixes: [
        addressPrefix.result
      ],
      serviceEndpoints: [{
        service: "Microsoft.Storage"
      }],
      networkSecurityGroup: {
        id: nodesSecurityGroup.id,
      }
    }, { ...options, parent: virtualNetwork });

    const natPublicIp = new azure.network.PublicIPAddress("nat", {
      publicIpAddressName: `aptos-${args.workspaceName}-nat`,
      resourceGroupName: args.resourceGroupName,
      location: args.location,
      publicIPAllocationMethod: "Static",
      sku: {
        name: "Standard"
      }
    }, { parent: this });

    const natResource = new azure.network.NatGateway("nat", {
      natGatewayName: `aptos-${args.workspaceName}-nat`,
      resourceGroupName: args.resourceGroupName,
      publicIpAddresses: [{
        id: natPublicIp.id,
      }],
      location: args.location,
      sku: {
        name: "Standard"
      },
    }, { parent: this });

    // Register outputs
    this.vNetId = virtualNetwork.id;
    this.vNetName = virtualNetwork.name;
    this.nodesSubnetId = nodesSubnet.id;
    // coerce the null possibility away
    this.nodesSubnetName = nodesSubnet.name.apply(s => s!);
    this.nodesSubnetAddressPrefixes = nodesSubnet.addressPrefixes.apply(prefixes => prefixes!);

    this.natId = natPublicIp.id
    this.natName = natPublicIp.name
    this.natIpAddress = natPublicIp.ipAddress.apply(ip => ip!);

    this.nodesSecurityGroupName = nodesSecurityGroup.name;
    // this.natGatewayId = natResource.id
    // this.natGatewayName = natResource.name
  }
}

