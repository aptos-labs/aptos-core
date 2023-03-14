import { Provider } from "../providers/";
import * as Gen from "../generated/index";

const ans_contracts: Record<string, string> = {
  testnet: "0x5f8fd2347449685cf41d4db97926ec3a096eaf381332be4f1318ad4d16a8497c",
  mainnet: "0x867ed1f6bf916171b1de3ee92849b8978b7d1b9e0a8cc982a3d19d535dfd9c0c",
};

// Each name component can only have lowercase letters, number or hyphens, and cannot start or end with a hyphen.
const nameComponentPattern = /^[a-z\d][a-z\d-]{1,61}[a-z\d]$/;

const namePattern = new RegExp(
  "^" +
    // Optional subdomain (cannot be followed by .apt)
    "(?:(?<subdomain>[^.]+)\\.(?!apt$))?" +
    // Domain
    "(?<domain>[^.]+)" +
    // Optional .apt suffix
    "(?:\\.apt)?" +
    "$",
);

export class AnsClient {
  contractAddress: string;
  provider: Provider;

  /**
   * Creates new AnsClient instance
   * @param provider Provider instance
   * @param contractAddress An optional contract address. If there is no contract address matching to the provided network
   * then the AnsClient class expects a contract address - this is to support both mainnet/testnet networks and local development.
   */
  constructor(provider: Provider, contractAddress?: string) {
    this.provider = provider;
    if (!ans_contracts[this.provider.network] && !contractAddress) {
      throw new Error("Please provide a valid contract address");
    }
    this.contractAddress = ans_contracts[this.provider.network] ?? contractAddress;
  }

  /**
   * Returns the primary name for the given account address
   * @param address An account address
   * @returns Account's primary name | null
   */
  async getNamebyAddress(address: string): Promise<string | null> {
    const ansResource: { type: Gen.MoveStructTag; data: any } = await this.provider.getAccountResource(
      this.contractAddress,
      `${this.contractAddress}::domains::ReverseLookupRegistryV1`,
    );
    const handle = (ansResource as any).data.registry.handle;
    const domainsTableItemRequest = {
      key_type: "address",
      value_type: `${this.contractAddress}::domains::NameRecordKeyV1`,
      key: address,
    };
    try {
      const item = await this.provider.getTableItem(handle, domainsTableItemRequest);
      return item.subdomain_name.vec[0] ? `${item.ubdomain_name.vec[0]}.${item.domain_name}` : item.domain_name;
    } catch (error: any) {
      // if item not found, response is 404 error - meaning item not found
      return null;
    }
  }

  /**
   * Returns the target account address for the given name
   * @param name ANS name
   * @returns Account address | null
   */
  async getAddressByName(name: string): Promise<string | null> {
    const { domain, subdomain } = name.match(namePattern)?.groups ?? {};
    if (!domain) return null;
    if (subdomain) return this.getAddressBySubdomainName(domain, subdomain);
    return this.getAddressByDomainName(domain);
  }

  /**
   * Returns the account address for the given domain name
   * @param domain domain name
   * @example
   * if name is `aptos.apt`
   * domain = aptos
   *
   * @returns account address | null
   */
  private async getAddressByDomainName(domain: string) {
    if (domain.match(nameComponentPattern) === null) return null;
    const ansResource: { type: Gen.MoveStructTag; data: any } = await this.provider.getAccountResource(
      this.contractAddress,
      `${this.contractAddress}::domains::NameRegistryV1`,
    );

    const handle = (ansResource as any).data.registry.handle;
    const domainsTableItemRequest = {
      key_type: `${this.contractAddress}::domains::NameRecordKeyV1`,
      value_type: `${this.contractAddress}::domains::NameRecordV1`,
      key: {
        subdomain_name: { vec: [] },
        domain_name: domain,
      },
    };

    try {
      const item = await this.provider.getTableItem(handle, domainsTableItemRequest);
      return item.target_address.vec[0];
    } catch (error: any) {
      // if item not found, response is 404 error - meaning item not found
      return null;
    }
  }

  /**
   * Returns the account address for the given subdomain_name
   * @param domain domain name
   * @param subdomain subdomain name
   * @example
   * if name is `dev.aptos.apt`
   * domain = aptos
   * subdomain = dev
   *
   * @returns account address | null
   */
  private async getAddressBySubdomainName(domain: string, subdomain: string): Promise<string | null> {
    if (domain.match(nameComponentPattern) === null) return null;
    if (subdomain.match(nameComponentPattern) === null) return null;
    const ansResource: { type: Gen.MoveStructTag; data: any } = await this.provider.getAccountResource(
      this.contractAddress,
      `${this.contractAddress}::domains::NameRegistryV1`,
    );
    const handle = (ansResource as any).data.registry.handle;
    const domainsTableItemRequest = {
      key_type: `${this.contractAddress}::domains::NameRecordKeyV1`,
      value_type: `${this.contractAddress}::domains::NameRecordV1`,
      key: {
        subdomain_name: { vec: [subdomain] },
        domain_name: domain,
      },
    };

    try {
      const item = await this.provider.getTableItem(handle, domainsTableItemRequest);
      return item.target_address.vec[0];
    } catch (error: any) {
      // if item not found, response is 404 error - meaning item not found
      return null;
    }
  }
}
