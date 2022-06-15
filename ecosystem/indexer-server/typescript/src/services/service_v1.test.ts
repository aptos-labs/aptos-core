/* eslint-disable @typescript-eslint/naming-convention */
import {
  collections, metadatas, ownerships, tokens,
} from '@prisma/client';
import axios from 'axios';
import config from '../config';
import prisma from './Prisma';

const creator_address = '0xAptosAddress';
const collection_name = 'Lazer Ape';
const description = 'A collection of apes';
const image = 'https://miro.medium.com/max/1400/1*iE905qEuvUhSkXkWdSl8lA.jpeg';
const uri = 'https://aptoslabs.com';
const { URL_PORT } = config;
const version = 'v1';

const createRandomToken = async () => {
  const token_name = `Ape${Math.random() * 10000000}`;
  const token = await prisma.tokens.create({
    data: {
      collection: collection_name,
      creator: creator_address,
      description,
      last_minted_at: new Date(),
      max_amount: 1,
      minted_at: new Date(),
      name: token_name,
      supply: 1,
      token_id: `${creator_address}::${collection_name}::${token_name}`,
      uri,
    },
  });
  return token;
};

const cleanupToken = async (tokenId: string) => {
  await prisma.tokens.delete({
    where: {
      token_id: tokenId,
    },
  });
};

const createRandomMetadata = async () => {
  const token_name = `Ape${Math.random() * 10000000}`;
  const metadata = await prisma.metadatas.create({
    data: {
      attributes: [],
      description,
      image,
      last_updated_at: new Date(),
      name: token_name,
      properties: {},
      seller_fee_basis_points: 420,
      symbol: collection_name,
      token_id: `${creator_address}::${collection_name}::${token_name}`,
    },
  });
  return metadata;
};

const cleanupMetadata = async (tokenId: string) => {
  await prisma.metadatas.delete({
    where: {
      token_id: tokenId,
    },
  });
};

const createRandomOwnership = async () => {
  const token_name = `Ape${Math.random() * 10000000}`;
  const ownership = await prisma.ownerships.create({
    data: {
      amount: 1,
      inserted_at: new Date(),
      owner: creator_address,
      token_id: `${creator_address}::${collection_name}::${token_name}`,
      updated_at: new Date(),
    },
  });
  return ownership;
};

const cleanupOwnership = async (owner: string, token_id: string) => {
  await prisma.ownerships.delete({
    where: {
      token_id_owner: {
        owner,
        token_id,
      },
    },
  });
};

const createRandomCollection = async () => {
  const collection = await prisma.collections.create({
    data: {
      created_at: new Date(),
      creator: creator_address,
      description,
      inserted_at: new Date(),
      name: collection_name + (Math.random() * 10000000),
      uri,
    },
  });
  return collection;
};

const cleanupCollection = async (
  creatorAddress: string,
  collectionName: string,
) => {
  await prisma.collections.delete({
    where: {
      creator_name: {
        creator: creatorAddress,
        name: collectionName,
      },
    },
  });
};

test('/tokens/all', async () => {
  const token = await createRandomToken();
  const { token_id } = token;
  const fetchedToken = await axios.get<tokens[]>(`http://localhost:${URL_PORT}/${version}/tokens/all`);
  const tokenData = fetchedToken.data.map((value) => value.token_id);
  expect(tokenData).toContain(token_id);
  await cleanupToken(token_id);
});

test('/tokens/{tokenId}', async () => {
  const token = await createRandomToken();
  const {
    token_id,
  } = token;
  const fetchedToken = await axios.get<tokens>(`http://localhost:${URL_PORT}/${version}/tokens/${token_id}`);
  const tokenData = fetchedToken.data;
  expect(tokenData.token_id).toBe(token.token_id);
  await cleanupToken(token_id);
});

test('/tokens/{tokenId}/meta', async () => {
  const metadata = await createRandomMetadata();
  const {
    token_id,
  } = metadata;
  const fetchedMetadata = await axios.get<metadatas>(`http://localhost:${URL_PORT}/${version}/tokens/${token_id}/meta`);
  const metadataData = fetchedMetadata.data;
  expect(metadataData.token_id).toBe(metadata.token_id);
  await cleanupMetadata(token_id);
});

test('/tokens/{tokenId}/royalties', async () => {
  // TODO: royalties don't exist at the moment
});

test('/metadata/byIds', async () => {
  const metadata1 = await createRandomMetadata();
  const {
    token_id: token_id1,
  } = metadata1;
  const metadata2 = await createRandomMetadata();
  const {
    token_id: token_id2,
  } = metadata2;
  const fetchedMetadatas = await axios.post<metadatas[]>(`http://localhost:${URL_PORT}/${version}/metadata/byIds`, {
    tokenIds: [token_id1, token_id2],
  });
  const metadataDatas = fetchedMetadatas.data;
  const metadataDatasTokenIds = metadataDatas.map((value) => value.token_id);
  expect(metadataDatasTokenIds).toEqual([token_id1, token_id2]);
  await cleanupMetadata(token_id1);
  await cleanupMetadata(token_id2);
});

test('/tokens/byIds', async () => {
  const token1 = await createRandomToken();
  const {
    token_id: token_id1,
  } = token1;
  const token2 = await createRandomToken();
  const {
    token_id: token_id2,
  } = token2;
  const fetchTokens = await axios.post<tokens[]>(`http://localhost:${URL_PORT}/${version}/tokens/byIds`, {
    tokenIds: [token_id1, token_id2],
  });
  const fetchTokensData = fetchTokens.data.map((value) => value.token_id);
  expect(fetchTokensData).toEqual([token_id1, token_id2]);
  await cleanupToken(token_id1);
  await cleanupToken(token_id2);
});

test('/ownerships/all', async () => {
  const ownership1 = await createRandomOwnership();
  const ownership2 = await createRandomOwnership();
  const {
    owner: owner1,
    token_id: token_id1,
  } = ownership1;
  const {
    owner: owner2,
    token_id: token_id2,
  } = ownership2;
  const fetchedOwnerships = await axios.get<ownerships[]>(`http://localhost:${URL_PORT}/${version}/ownerships/all`);
  expect(fetchedOwnerships.data.length).toBeGreaterThan(1);
  await cleanupOwnership(owner1, token_id1);
  await cleanupOwnership(owner2, token_id2);
});

test('/ownerships/{ownershipId}', async () => {
  const ownership = await createRandomOwnership();
  const {
    owner,
    token_id,
  } = ownership;
  const fetchedOwnership = await axios.get<ownerships>(`http://localhost:${URL_PORT}/${version}/ownerships/${token_id}::${owner}`);
  expect(fetchedOwnership.data.owner).toBe(owner);
  await cleanupOwnership(owner, token_id);
});

test('/ownerships/byIds', async () => {
  const ownership1 = await createRandomOwnership();
  const ownership2 = await createRandomOwnership();
  const {
    owner: owner1,
    token_id: token_id1,
  } = ownership1;
  const {
    owner: owner2,
    token_id: token_id2,
  } = ownership2;
  // ownership id is ${tokenID}::${ownerAddress},
  // or ${creator_address}::${collection_name}::${token_name}::${ownerAddress}
  const ownershipIds = [`${token_id1}::${owner1}`, `${token_id2}::${owner2}`];
  const fetchedOwnerships = await axios.post<ownerships[]>(`http://localhost:${URL_PORT}/${version}/ownerships/byIds`, {
    ownershipIds,
  });
  const fetchedOwnershipsData = fetchedOwnerships.data.map((value) => `${value.token_id}::${value.owner}`);
  expect(fetchedOwnershipsData).toEqual(ownershipIds);
  await cleanupOwnership(owner1, token_id1);
  await cleanupOwnership(owner2, token_id2);
});

test('/collections/{collectionId}', async () => {
  const collection = await createRandomCollection();
  const {
    creator, name,
  } = collection;
  const fetchedCollections = await axios.get<collections>(`http://localhost:${URL_PORT}/${version}/collections/${creator}::${name}`);
  expect(fetchedCollections.data.creator).toBe(creator);
  await cleanupCollection(creator, name);
});

test('/collections/all', async () => {
  const collection1 = await createRandomCollection();
  const collection2 = await createRandomCollection();
  const {
    creator: creator1, name: name1,
  } = collection1;
  const {
    creator: creator2, name: name2,
  } = collection2;
  const fetchedCollections = await axios.get<collections[]>(`http://localhost:${URL_PORT}/${version}/collections/all`);
  expect(fetchedCollections.data.length).toBeGreaterThan(1);
  await cleanupCollection(creator1, name1);
  await cleanupCollection(creator2, name2);
});

// test('/activities/byUser', async () => {});
