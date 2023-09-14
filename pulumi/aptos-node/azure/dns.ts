import * as pulumi from '@pulumi/pulumi';
import * as azure from "@pulumi/azure-native";
import * as k8s from '@pulumi/kubernetes';
import * as random from '@pulumi/random';
import { AptosLoadBalancerIngress } from './utils';

export interface DNSConfig {
  workspaceName: string;
  recordName: string;
  zoneName: string;
  zoneResourceGroup: string;
  validatorLoadBalancerIngress: pulumi.Output<AptosLoadBalancerIngress>;
  fullnodeLoadBalancerIngress: pulumi.Output<AptosLoadBalancerIngress>;
}

export class DNS extends pulumi.ComponentResource {
  public readonly validatorEndpoint: pulumi.Output<string>;
  public readonly fullnodeEndpoint: pulumi.Output<string>;

  constructor(name: string, args: DNSConfig, opts?: pulumi.ComponentResourceOptions) {
    super("aptos-node:azure:dns", name, {}, opts);

    // Generate random string for DNS
    const validatorDns = new random.RandomString(`${name}-validator-dns`, {
      length: 16,
      upper: false,
      special: false,
    });

    // Azure DNS A Record for validator
    const validatorDnsRecord = new azure.network.RecordSet(`${name}-validator`, {
      relativeRecordSetName: pulumi.interpolate`${validatorDns.result}.${args.recordName}`,
      recordType: "A",
      resourceGroupName: args.zoneResourceGroup,
      zoneName: args.zoneName,
      ttl: 3600,
      aRecords: [{
        ipv4Address: args.validatorLoadBalancerIngress.ip,
      }]
    });

    // Azure DNS A Record for fullnode
    const fullnodeDnsRecord = new azure.network.RecordSet(`${name}-fullnode`, {
      relativeRecordSetName: args.recordName,
      recordType: "A",
      resourceGroupName: args.zoneResourceGroup,
      zoneName: args.zoneName,
      ttl: 3600,
      aRecords: [{
        ipv4Address: args.fullnodeLoadBalancerIngress.ip,
      }]
    });

    // Register outputs
    this.validatorEndpoint = pulumi.interpolate`/dns4/${validatorDnsRecord.fqdn}/tcp/${args.validatorLoadBalancerIngress.port}`;
    this.fullnodeEndpoint = pulumi.interpolate`/dns4/${fullnodeDnsRecord.fqdn}/tcp/${args.fullnodeLoadBalancerIngress.port}`;
  }
}

