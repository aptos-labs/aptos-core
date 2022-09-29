// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { coinStoreStructTag } from 'core/constants';
import { CoinStoreResourceData, Resource } from 'shared/types';

/**
 * Get coin store resources from a set of resources, grouped by coin type
 */
export function getCoinStoresByCoinType(resources: Resource[]) {
  const coinStoreTypePattern = new RegExp(`^${coinStoreStructTag}<(.+)>$`);
  const coinStores: Record<string, CoinStoreResourceData> = {};
  for (const resource of resources) {
    const match = resource.type.match(coinStoreTypePattern);
    if (match !== null) {
      const coinType = match[1];
      coinStores[coinType] = resource.data;
    }
  }
  return coinStores;
}

export default getCoinStoresByCoinType;
