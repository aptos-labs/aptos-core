// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import Browser from 'core/utils/browser';

const PERMISSIONS_STORAGE_KEY = 'aptosWalletPermissions';

export default class Permissions {
  public static async addDomain(domain: string): Promise<void> {
    const domains = await this.getDomains();
    domains.add(domain);
    return this.saveDomains(domains);
  }

  public static async removeDomain(domain: string): Promise<void> {
    const domains = await this.getDomains();
    domains.delete(domain);
    return this.saveDomains(domains);
  }

  public static async isDomainAllowed(domain: string): Promise<boolean> {
    const domains = await this.getDomains();
    return domains.has(domain);
  }

  static async getDomains(): Promise<Set<string>> {
    const result = await Browser.storage()?.get([PERMISSIONS_STORAGE_KEY]);
    if (result && result[PERMISSIONS_STORAGE_KEY]) {
      return new Set(result[PERMISSIONS_STORAGE_KEY]);
    }
    return new Set();
  }

  static saveDomains(domains: Set<string>): Promise<void> {
    return Browser.storage()!.set({ [PERMISSIONS_STORAGE_KEY]: Array.from(domains) });
  }
}
