// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import Browser from 'core/utils/browser';

const PERMISSIONS_STORAGE_KEY = 'aptosWalletPermissions';

export default class Permissions {
  static async addDomain(domain: string, address: string): Promise<void> {
    const domains = await this.getDomains(address);
    domains.add(domain);
    return this.saveDomains(domains, address);
  }

  public static async removeDomain(domain: string, address: string): Promise<void> {
    const domains = await this.getDomains(address);
    domains.delete(domain);
    return this.saveDomains(domains, address);
  }

  public static async isDomainAllowed(domain: string, address: string): Promise<boolean> {
    const domains = await this.getDomains(address);
    return domains.has(domain);
  }

  static async getAllDomains(): Promise<{ [address: string]: string[] }> {
    const result = await Browser.persistentStorage()?.get([PERMISSIONS_STORAGE_KEY]);
    return (result && result[PERMISSIONS_STORAGE_KEY])
      ? result[PERMISSIONS_STORAGE_KEY]
      : {};
  }

  public static async getDomains(address: string): Promise<Set<string>> {
    const allDomains = await this.getAllDomains();
    return new Set(allDomains[address]) ?? new Set();
  }

  static async saveDomains(domains: Set<string>, address: string): Promise<void> {
    const allDomains = await this.getAllDomains();
    allDomains[address] = Array.from(domains);
    return Browser.persistentStorage()!.set({ [PERMISSIONS_STORAGE_KEY]: allDomains });
  }
}
