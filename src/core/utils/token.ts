// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export interface TokenId {
  collection: string;
  creator: string;
  name: string;
}

export const getTokenIdStringFromDict = ({
  collection,
  creator,
  name,
}: TokenId) => `${creator}::${collection}::${name}`;

interface GetTokenIdStringFromDictProps {
  tokenId: string;
}

export const getTokenIdDictFromString = ({
  tokenId,
}: GetTokenIdStringFromDictProps): TokenId => {
  const [creator, collection, name] = tokenId.split('::');
  return {
    collection,
    creator,
    name,
  };
};
