export type MetaDataJsonCategory = 'image' | 'video' | 'audio' | 'vr' | 'html';

export type MetadataJsonAttribute = {
  trait_type: string;
  value: string;
};

export type MetadataJsonCollection = {
  name: string;
  family: string;
};

export type MetadataJsonFile = {
  uri: string;
  type: string;
  cdn?: boolean;
};

export type MetadataJsonCreator = {
  address: string;
  verified: boolean;
  share: number;
};

export type MetadataJsonProperties = {
  files: MetadataJsonFile[];
  category: MetaDataJsonCategory;
  creators: MetadataJsonCreator[];
};

export type MetadataJson = {
  name: string;
  symbol: string;
  description: string;
  seller_fee_basis_points: number;
  image: string;
  animation_url?: string;
  external_url?: string;
  attributes?: MetadataJsonAttribute[];
  collection?: MetadataJsonCollection;
  properties: MetadataJsonProperties;
};
