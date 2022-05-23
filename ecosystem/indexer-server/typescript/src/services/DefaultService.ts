/* eslint-disable @typescript-eslint/naming-convention */
/* eslint-disable no-async-promise-executor */
/* eslint-disable no-unused-vars */
import {
  collections as PrismaCollections,
  metadatas as PrismaMetadatas,
  ownerships as PrismaOwnerships,
  tokens as PrismaTokens,
} from '@prisma/client';
import Service, { SuccessResponseType } from './Service';
import prisma from './Prisma';

interface GetAllCollectionsParams {
  offset?: number | string;
  size?: number | string;
}

/**
* Returns all collections
*
* offset Integer offset of the collections to be returned from the first collection. (optional)
* size Integer The number of items to return (optional)
* returns Collections
* */
const getAllCollections = ({
  offset,
  size,
}: GetAllCollectionsParams) => new Promise<SuccessResponseType<PrismaCollections[]>>(
  async (resolve, reject) => {
    const data = await prisma.collections.findMany({
      skip: offset ? Number(offset) : undefined,
      take: size ? Number(size) : undefined,
    });
    try {
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

type GetAllOwnershipsParams = GetAllCollectionsParams;

/**
* Returns all ownerships
*
* offset Integer offset of the ownership to be returned from the first ownership. (optional)
* size Integer The number of items to return (optional)
* returns Ownerships
* */
const getAllOwnerships = ({
  offset,
  size,
}: GetAllOwnershipsParams) => new Promise<SuccessResponseType<PrismaOwnerships[]>>(
  async (resolve, reject) => {
    try {
      const data = prisma.ownerships.findMany({
        skip: offset ? Number(offset) : undefined,
        take: size ? Number(size) : undefined,
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

type GetAllTokensParams = GetAllCollectionsParams;

/**
* Returns all tokens
*
* offset Integer offset of the tokens to be returned from the first token. (optional)
* size Integer The number of items to return (optional)
* returns Tokens
* */
const getAllTokens = ({
  offset,
  size,
}: GetAllTokensParams) => new Promise<SuccessResponseType<PrismaTokens[]>>(
  async (resolve, reject) => {
    try {
      const data = await prisma.tokens.findMany({
        skip: offset ? Number(offset) : undefined,
        take: size ? Number(size) : undefined,
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

interface GetCollectionByIdParams {
  collectionId: string;
}

/**
* Returns collection by collection id
*
* collectionId String Id of the collection
* returns Collection
* */
const getCollectionById = ({
  collectionId,
}: GetCollectionByIdParams) => new Promise<SuccessResponseType<PrismaCollections | null>>(
  async (resolve, reject) => {
    try {
      const [creator, name] = collectionId.split('::');
      const data = await prisma.collections.findUnique({
        where: {
          creator_name: {
            creator,
            name,
          },
        },
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

interface GetOwnershipByIdParams {
  ownershipId: string;
}

/**
* Returns ownership by Id
*
* ownershipId String Ownership Id has the format of \"${tokenID}::${ownerAddress}\"
* returns Ownership
* */
const getOwnershipById = ({
  ownershipId,
}: GetOwnershipByIdParams) => new Promise<SuccessResponseType<PrismaOwnerships | null>>(
  async (resolve, reject) => {
    try {
      const [token_id, owner] = ownershipId.split('::');
      const data = await prisma.ownerships.findUnique({
        where: {
          token_id_owner: {
            owner,
            token_id,
          },
        },
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

interface GetOwnershipsByIdsParams {
  ownerships: string[]
}

/**
* Returns Ownerships by Ids
*
* ownerships Ownerships
* returns Ownership
* */
const getOwnershipsByIds = ({
  ownerships,
}: GetOwnershipsByIdsParams) => new Promise<SuccessResponseType<PrismaOwnerships []>>(
  async (resolve, reject) => {
    const owners: string[] = [];
    const tokenIds: string[] = [];
    ownerships.forEach((ownership) => {
      const [owner, tokenId] = ownership.split('::');
      owners.push(owner);
      tokenIds.push(tokenId);
    });
    const data = await prisma.ownerships.findMany({
      where: {
        owner: {
          in: owners,
        },
        token_id: {
          in: tokenIds,
        },
      },
    });
    try {
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

interface GetOwnershipsByTokenParams {
  offset: number | string;
  size: number | string;
  tokenId: string;
}

/**
* Returns ownerships by token
*
* tokenId String TokenId, has format `${creator_address}::${collection_name}::${token_name}`
* offset Integer offset from the first ownership to be returned (optional)
* size Integer The number of tokens to return (optional)
* returns Ownerships
* */
const getOwnershipsByToken = ({
  offset,
  size,
  tokenId,
}: GetOwnershipsByTokenParams) => new Promise<SuccessResponseType<PrismaOwnerships []>>(
  async (resolve, reject) => {
    try {
      const data = await prisma.ownerships.findMany({
        skip: offset ? Number(offset) : undefined,
        take: size ? Number(size) : undefined,
        where: {
          token_id: tokenId,
        },
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

interface GetTokenByIdParams {
  tokenId: string;
}

/**
* Returns Token by Id
*
* tokenId String Token Id has format `${creator_address}::${collection_name}::${token_name}`
* returns Token
* */
const getTokenById = ({
  tokenId,
}: GetTokenByIdParams) => new Promise<SuccessResponseType<PrismaTokens | null>>(
  async (resolve, reject) => {
    try {
      const data = await prisma.tokens.findUnique({
        where: {
          token_id: tokenId,
        },
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

interface GetTokenByIdsParams {
  tokenIds: string[];
}

/**
* Returns Tokens by Ids
*
* tokenIds TokenIds
* returns Tokens
* */
const getTokenByIds = ({
  tokenIds,
}: GetTokenByIdsParams) => new Promise<SuccessResponseType<PrismaTokens[]>>(
  async (resolve, reject) => {
    const data = await prisma.tokens.findMany({
      where: {
        token_id: {
          in: tokenIds,
        },
      },
    });
    try {
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

type GetTokenMetaDataByIdParams = GetTokenByIdParams;

/**
* Returns token metadata by Id
*
* tokenId TokenMetaData Token Id has format `${creator_address}::${collection_name}::${token_name}`
* returns Token
* */
const getTokenMetaDataById = ({
  tokenId,
}: GetTokenMetaDataByIdParams) => new Promise<SuccessResponseType<PrismaMetadatas | null>>(
  async (resolve, reject) => {
    try {
      const data = await prisma.metadatas.findUnique({
        where: {
          token_id: tokenId,
        },
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

type GetTokenMetaDataByIdsParams = GetTokenByIdsParams;

/**
* Returns Token MetaData by Ids
*
* tokenIds TokenIds  (optional)
* returns TokenMetaData
* */
const getTokenMetaDataByIds = ({
  tokenIds,
}: GetTokenMetaDataByIdsParams) => new Promise<SuccessResponseType<PrismaMetadatas []>>(
  async (resolve, reject) => {
    try {
      const data = await prisma.metadatas.findMany({
        where: {
          token_id: {
            in: tokenIds,
          },
        },
      });
      resolve(Service.successResponse({
        data,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

type GetTokenRoyaltiesByIdParams = GetTokenByIdParams;

/**
* Returns token royalty by tokenId
*
* @param tokenId String
* @returns Royalties
* */
const getTokenRoyaltiesById = ({
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  tokenId,
}: GetTokenRoyaltiesByIdParams) => new Promise<SuccessResponseType<any[]>>(
  async (resolve, reject) => {
    try {
      resolve(Service.successResponse({ data: [] }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

export const DefaultServiceFunctions = {
  getAllCollections,
  getAllOwnerships,
  getAllTokens,
  getCollectionById,
  getOwnershipById,
  getOwnershipsByIds,
  getOwnershipsByToken,
  getTokenById,
  getTokenByIds,
  getTokenMetaDataById,
  getTokenMetaDataByIds,
  getTokenRoyaltiesById,
};

export default {
  getAllCollections,
  getAllOwnerships,
  getAllTokens,
  getCollectionById,
  getOwnershipById,
  getOwnershipsByIds,
  getOwnershipsByToken,
  getTokenById,
  getTokenByIds,
  getTokenMetaDataById,
  getTokenMetaDataByIds,
  getTokenRoyaltiesById,
};
