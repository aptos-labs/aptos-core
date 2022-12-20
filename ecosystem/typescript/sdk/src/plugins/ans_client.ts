import { AptosClient } from "../aptos_client";
import * as Gen from "../generated/index";

const CONTRACT_ADDRESS = "";

export class AnsClient {
  aptosClient: AptosClient;

  /**
   * Creates new AnsClient instance
   * @param aptosClient AptosClient instance
   */
  constructor(aptosClient: AptosClient) {
    this.aptosClient = aptosClient;
  }

  async getNameFromAddress(address: string) {}

  async getAddressFromName(name: string) {
    const ansResource: { type: Gen.MoveStructTag; data: any } = await this.aptosClient.getAccountResource(
      `${CONTRACT_ADDRESS}`,
      `${CONTRACT_ADDRESS}::domains::NameRegistryV1`,
    );
    const { handle }: { handle: string } = (ansResource as any).registry.handle;
    const domainsTableItemRequest = {
      key_type: `${CONTRACT_ADDRESS}::domains::NameRecordKeyV1`,
      value_type: `${CONTRACT_ADDRESS}::domains::NameRecordV1`,
      key: {
        subdomain_name: { vec: [] },
        domain_name: name,
      },
    };

    const address = await this.aptosClient.getTableItem(handle, domainsTableItemRequest);
    console.log(address);
  }
}
