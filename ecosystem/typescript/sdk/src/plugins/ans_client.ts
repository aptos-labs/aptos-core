import { AptosClient, Provider, OptionalTransactionArgs, ApiError } from "../providers";
import * as Gen from "../generated/index";
import { AptosAccount } from "../account";
import { AccountAddress } from "../aptos_types";
import { TransactionBuilderRemoteABI } from "../transaction_builder";

export const ansContractsMap: Record<string, string> = {
  testnet: "0x5f8fd2347449685cf41d4db97926ec3a096eaf381332be4f1318ad4d16a8497c",
  mainnet: "0x867ed1f6bf916171b1de3ee92849b8978b7d1b9e0a8cc982a3d19d535dfd9c0c",
};

// Each name component can only have lowercase letters, number or hyphens, and cannot start or end with a hyphen.
export const nameComponentPattern = /^[a-z\d][a-z\d-]{1,61}[a-z\d]$/;

export const namePattern = new RegExp(
  "^" +
    // Optional subdomain (cannot be followed by .apt)
    "(?:(?<subdomain>[^.]+)\\.(?!apt$))?" +
    // Domain
    "(?<domain>[^.]+)" +
    // Optional .apt suffix
    "(?:\\.apt)?" +
    "$",
);

export type ReverseLookupRegistryV1 = {
  registry: {
    handle: string;
  };
};

export type NameRegistryV1 = {
  registry: {
    handle: string;
  };
};

export type AnsRegistry = {
  expirationTimestampSeconds: number;
  target: string | null;
};

export class AnsClient {
  contractAddress: string;

  provider: Provider;

  /**
   * Creates new AnsClient instance
   * @param provider Provider instance
   * @param contractAddress An optional contract address.
   * If there is no contract address matching to the provided network
   * then the AnsClient class expects a contract address -
   * this is to support both mainnet/testnet networks and local development.
   */
  constructor(provider: Provider, contractAddress?: string) {
    this.provider = provider;
    if (!ansContractsMap[this.provider.network] && !contractAddress) {
      throw new Error("Error: For custom providers, you must pass in a contract address");
    }
    this.contractAddress = ansContractsMap[this.provider.network] ?? contractAddress;
  }

  /**
   * Returns the primary name for the given account address
   * @param address An account address
   * @returns Account's primary name | null if there is no primary name defined
   */
  async getPrimaryNameByAddress(address: string): Promise<string | null> {
    const ansResource: Gen.MoveResource = await this.provider.getAccountResource(
      this.contractAddress,
      `${this.contractAddress}::domains::ReverseLookupRegistryV1`,
    );
    const data = ansResource.data as ReverseLookupRegistryV1;
    const { handle } = data.registry;
    const domainsTableItemRequest = {
      key_type: "address",
      value_type: `${this.contractAddress}::domains::NameRecordKeyV1`,
      key: address,
    };
    try {
      const item = await this.provider.getTableItem(handle, domainsTableItemRequest);
      return item.subdomain_name.vec[0] ? `${item.subdomain_name.vec[0]}.${item.domain_name}` : item.domain_name;
    } catch (error: any) {
      // if item not found, response is 404 error - meaning item not found
      if (error.status === 404) {
        return null;
      }
      throw new Error(error);
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
    const registration = subdomain
      ? await this.getRegistrationForSubdomainName(domain, subdomain)
      : await this.getRegistrationForDomainName(domain);
    return registration === null ? null : registration.target;
  }

  /**
   * Mint a new Aptos name
   *
   * @param account AptosAccount where collection will be created
   * @param domainName Aptos domain name to mint
   * @param years year duration of the domain name
   * @returns The hash of the pending transaction submitted to the API
   */
  async mintAptosName(
    account: AptosAccount,
    domainName: string,
    years: number = 1,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<Gen.HashValue> {
    // check if the name is valid
    if (domainName.match(nameComponentPattern) === null) {
      throw new ApiError(400, `Name ${domainName} is not valid`);
    }
    // check if the name is available
    const registration = await this.getRegistrationForDomainName(domainName);
    if (registration) {
      const now = Math.ceil(Date.now() / 1000);
      if (now < registration.expirationTimestampSeconds) {
        throw new ApiError(400, `Name ${domainName} is not available`);
      }
    }

    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(`${this.contractAddress}::domains::register_domain`, [], [domainName, years]);

    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.submitSignedBCSTransaction(bcsTxn);

    return pendingTransaction.hash;
  }

  /**
   * Mint a new Aptos Subdomain
   *
   * @param account AptosAccount the owner of the domain name
   * @param subdomainName subdomain name to mint
   * @param domainName Aptos domain name to mint under
   * @param expirationTimestampSeconds must be set between the domains expiration and the current time
   * @returns The hash of the pending transaction submitted to the API
   */
  async mintAptosSubdomain(
    account: AptosAccount,
    subdomainName: string,
    domainName: string,
    expirationTimestampSeconds?: number,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<Gen.HashValue> {
    // check if the name is valid
    if (domainName.match(nameComponentPattern) === null) {
      throw new ApiError(400, `Domain name ${domainName} is not valid`);
    }
    // check if the name is valid
    if (subdomainName.match(nameComponentPattern) === null) {
      throw new ApiError(400, `Subdomain name ${subdomainName} is not valid`);
    }
    // check if the name is available
    const subdomainRegistration = await this.getRegistrationForSubdomainName(domainName, subdomainName);
    if (subdomainRegistration) {
      const now = Math.ceil(Date.now() / 1000);
      if (now < subdomainRegistration.expirationTimestampSeconds) {
        throw new ApiError(400, `Name ${subdomainName}.${domainName} is not available`);
      }
    }

    const domainRegistration = await this.getRegistrationForDomainName(domainName);
    if (domainRegistration === null) {
      throw new ApiError(400, `Domain name ${domainName} does not exist`);
    }
    const now = Math.ceil(Date.now() / 1000);
    if (domainRegistration.expirationTimestampSeconds < now) {
      throw new ApiError(400, `Domain name ${domainName} expired`);
    }

    const actualExpirationTimestampSeconds =
      expirationTimestampSeconds || domainRegistration.expirationTimestampSeconds;
    if (actualExpirationTimestampSeconds < now) {
      throw new ApiError(400, `Expiration for ${subdomainName}.${domainName} is before now`);
    }

    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      `${this.contractAddress}::domains::register_subdomain`,
      [],
      [subdomainName, domainName, actualExpirationTimestampSeconds],
    );

    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.submitSignedBCSTransaction(bcsTxn);

    return pendingTransaction.hash;
  }

  /**
   * @param account AptosAccount the owner of the domain name
   * @param subdomainName subdomain name to mint
   * @param domainName Aptos domain name to mint
   * @param target the target address for the subdomain
   * @returns The hash of the pending transaction submitted to the API
   */
  async setSubdomainAddress(
    account: AptosAccount,
    subdomainName: string,
    domainName: string,
    target: string,
    extraArgs?: OptionalTransactionArgs,
  ): Promise<Gen.HashValue> {
    const standardizeAddress = AccountAddress.standardizeAddress(target);

    // check if the name is valid
    if (domainName.match(nameComponentPattern) === null) {
      throw new ApiError(400, `Name ${domainName} is not valid`);
    }
    // check if the name is valid
    if (subdomainName.match(nameComponentPattern) === null) {
      throw new ApiError(400, `Name ${subdomainName} is not valid`);
    }

    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: account.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(
      `${this.contractAddress}::domains::set_subdomain_address`,
      [],
      [subdomainName, domainName, standardizeAddress],
    );

    const bcsTxn = AptosClient.generateBCSTransaction(account, rawTxn);
    const pendingTransaction = await this.provider.submitSignedBCSTransaction(bcsTxn);

    return pendingTransaction.hash;
  }

  /**
   * Initialize reverse lookup for contract owner
   *
   * @param owner the `aptos_names` AptosAccount
   * @returns The hash of the pending transaction submitted to the API
   */
  async initReverseLookupRegistry(owner: AptosAccount, extraArgs?: OptionalTransactionArgs): Promise<Gen.HashValue> {
    const builder = new TransactionBuilderRemoteABI(this.provider.aptosClient, {
      sender: owner.address(),
      ...extraArgs,
    });
    const rawTxn = await builder.build(`${this.contractAddress}::domains::init_reverse_lookup_registry_v1`, [], []);

    const bcsTxn = AptosClient.generateBCSTransaction(owner, rawTxn);
    const pendingTransaction = await this.provider.submitSignedBCSTransaction(bcsTxn);

    return pendingTransaction.hash;
  }

  /**
   * Returns the AnsRegistry for the given domain name
   * @param domain domain name
   * @example
   * if name is `aptos.apt`
   * domain = aptos
   *
   * @returns AnsRegistry | null
   */
  private async getRegistrationForDomainName(domain: string): Promise<AnsRegistry | null> {
    if (domain.match(nameComponentPattern) === null) return null;
    const ansResource: { type: Gen.MoveStructTag; data: any } = await this.provider.getAccountResource(
      this.contractAddress,
      `${this.contractAddress}::domains::NameRegistryV1`,
    );
    const data = ansResource.data as NameRegistryV1;
    const { handle } = data.registry;
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
      return {
        target: item.target_address.vec.length === 1 ? item.target_address.vec[0] : null,
        expirationTimestampSeconds: item.expiration_time_sec,
      };
    } catch (error: any) {
      // if item not found, response is 404 error - meaning item not found
      if (error.status === 404) {
        return null;
      }
      throw new Error(error);
    }
  }

  /**
   * Returns the AnsRegistry for the given subdomain_name
   * @param domain domain name
   * @param subdomain subdomain name
   * @example
   * if name is `dev.aptos.apt`
   * domain = aptos
   * subdomain = dev
   *
   * @returns AnsRegistry | null
   */
  private async getRegistrationForSubdomainName(domain: string, subdomain: string): Promise<AnsRegistry | null> {
    if (domain.match(nameComponentPattern) === null) return null;
    if (subdomain.match(nameComponentPattern) === null) return null;
    const ansResource: { type: Gen.MoveStructTag; data: any } = await this.provider.getAccountResource(
      this.contractAddress,
      `${this.contractAddress}::domains::NameRegistryV1`,
    );
    const data = ansResource.data as NameRegistryV1;
    const { handle } = data.registry;
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
      return {
        target: item.target_address.vec.length === 1 ? item.target_address.vec[0] : null,
        expirationTimestampSeconds: item.expiration_time_sec,
      };
    } catch (error: any) {
      // if item not found, response is 404 error - meaning item not found
      if (error.status === 404) {
        return null;
      }
      throw new Error(error);
    }
  }
}
