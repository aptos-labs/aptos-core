import { AptosClient } from "../aptos_client";
import * as Gen from "../generated/index";

const ans_contracts: Record<string, string> = {
  "https://testnet.aptoslabs.com/v1": "0x5f8fd2347449685cf41d4db97926ec3a096eaf381332be4f1318ad4d16a8497c",
  "https://mainnet.aptoslabs.com/v1": "0x867ed1f6bf916171b1de3ee92849b8978b7d1b9e0a8cc982a3d19d535dfd9c0c",
};

export class AnsClient {
  aptosClient: AptosClient;
  contractAddress: string;

  /**
   * Creates new AnsClient instance
   * @param aptosClient AptosClient instance
   * @param optional contract address. If there is no contract address matching to the provided node url
   * then the AnsClient class expects a contract address - this is to support both live networks and local development.
   */
  constructor(aptosClient: AptosClient, contractAddress?: string) {
    this.aptosClient = aptosClient;

    if (!ans_contracts[this.aptosClient.nodeUrl] && !contractAddress) {
      throw new Error("Please provide a valid contract address");
    }
    this.contractAddress = ans_contracts[this.aptosClient.nodeUrl] ?? contractAddress;
  }

  async getNameFromAddress(address: string) {}

  async getAddressByName(name: string): Promise<string> {
    const ansResource: { type: Gen.MoveStructTag; data: any } = await this.aptosClient.getAccountResource(
      this.contractAddress,
      `${this.contractAddress}::domains::NameRegistryV1`,
    );

    const handle = (ansResource as any).data.registry.handle;
    const domainsTableItemRequest = {
      key_type: `${this.contractAddress}::domains::NameRecordKeyV1`,
      value_type: `${this.contractAddress}::domains::NameRecordV1`,
      key: {
        subdomain_name: { vec: [] },
        domain_name: name,
      },
    };

    const item = await this.aptosClient.getTableItem(handle, domainsTableItemRequest);
    return item.target_address.vec[0];
  }
}
