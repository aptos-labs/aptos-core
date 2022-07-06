// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { useMemo } from 'react';
import {
  Box,
  Button,
  Center,
  Divider,
  Flex,
  Grid,
  Heading,
  Icon,
  SimpleGrid,
  Tag,
  TagLabel,
  Text,
  Tooltip,
  useClipboard,
  useColorMode,
  VStack,
  Wrap,
} from '@chakra-ui/react';
import { useParams } from 'react-router-dom';
import { useTokenData } from 'core/queries/collectibles';
import { GrStorage } from '@react-icons/all-files/gr/GrStorage';
import { ExternalLinkIcon } from '@chakra-ui/icons';
import SquareImage from './SquareImage';
import ArweaveLogo from './ArweaveLogo';
import IPFSlogo from './IPFSLogo';
import CollectionIcon from './CollectionIcon';

const imageNotFound = 'https://www.flexx.co/assets/camaleon_cms/image-not-found-4a963b95bf081c3ea02923dceaeb3f8085e1a654fc54840aac61a57a60903fef.png';
const imageBoxShadow = 'rgba(0, 0, 0, 0.25) 0px 54px 55px, rgba(0, 0, 0, 0.12) 0px -12px 30px, rgba(0, 0, 0, 0.12) 0px 4px 6px, rgba(0, 0, 0, 0.17) 0px 12px 13px, rgba(0, 0, 0, 0.09) 0px -3px 5px';

const secondaryAttributeColor = {
  dark: 'gray.200',
  light: 'gray.600',
};

const secondaryBgColor = {
  dark: 'gray.700',
  light: 'gray.100',
};

interface MetadataStorageProviderProps {
  uri?: string;
}

export const getMetadataStorageProviderFromURI = ({
  uri,
}: MetadataStorageProviderProps) => {
  if (!uri) {
    return undefined;
  }
  if (uri.includes('arweave.net')) {
    return 'Arweave';
  } if (uri.includes('ipfs.io')) {
    return 'IPFS';
  }
  return 'Storage Provider';
};

export type MetadataStorageProviderType = ReturnType<typeof getMetadataStorageProviderFromURI>;

const getMetadataStorageProviderIcon = (
  storageProvider: MetadataStorageProviderType,
) => {
  switch (storageProvider) {
    case 'Arweave': {
      return <ArweaveLogo />;
    }
    case 'IPFS': {
      return <IPFSlogo />;
    }
    case 'Storage Provider': {
      return <Icon as={GrStorage} />;
    }
    default: {
      return <Icon as={GrStorage} />;
    }
  }
};

interface CreatorTagProps {
  address: string;
}

function CreatorTag({
  address,
}: CreatorTagProps) {
  const { hasCopied, onCopy } = useClipboard(address || '');

  return (
    <Tooltip label={hasCopied ? 'Copied!' : 'Copy'} closeDelay={300}>
      <Tag
        cursor="pointer"
        onClick={onCopy}
        key={address}
        size="md"
        colorScheme="gray"
        borderRadius="full"
      >
        <TagLabel maxW="85px">{address}</TagLabel>
      </Tag>
    </Tooltip>
  );
}

function TokenBody() {
  const { id } = useParams();
  const { data } = useTokenData({ tokenId: id || '' });
  const { colorMode } = useColorMode();

  const metadataStorageProvider = useMemo(
    () => getMetadataStorageProviderFromURI({ uri: data?.uri }),
    [data],
  );

  const metadataStorageProviderIcon = useMemo(
    () => getMetadataStorageProviderIcon(metadataStorageProvider),
    [metadataStorageProvider],
  );

  return (
    <VStack>
      <Center px={8} py={12} bgColor={secondaryBgColor[colorMode]} width="100%">
        <a
          target="_blank"
          href={data?.metadata?.animation_url || data?.metadata?.image || data?.uri || 'https://aptos.dev'}
          rel="noreferrer"
        >
          <SquareImage
            src={data?.metadata?.image || imageNotFound}
            boxShadow={imageBoxShadow}
            borderRadius=".5rem"
          />
        </a>
      </Center>
      <VStack
        alignItems="flex-start"
        width="100%"
        px={4}
        divider={<Divider />}
        spacing={6}
        pt={4}
      >
        <VStack alignItems="flex-start">
          <Heading>
            {data?.name}
          </Heading>
          <Text>
            {data?.metadata?.description}
          </Text>
        </VStack>
        <VStack alignItems="flex-start">
          <Heading size="md">
            Details
          </Heading>
          <VStack spacing={4} pt={4} width="100%">
            <Grid templateColumns="28px 1fr" gap={6} width="100%">
              <Box pt={1}>
                <Icon fontSize={28} as={CollectionIcon} />
              </Box>
              <Box>
                <Text fontSize="md">{data?.collection}</Text>
                <Text fontSize="xs" fontWeight={600} color={secondaryAttributeColor[colorMode]}>Collection</Text>
              </Box>
            </Grid>
            <Grid templateColumns="28px 1fr" gap={6} width="100%">
              <Box pt={1}>
                {metadataStorageProviderIcon}
              </Box>
              <Box>
                <Button
                  fontSize="md"
                  fontWeight={400}
                  height="24px"
                  as="a"
                  target="_blank"
                  rightIcon={<ExternalLinkIcon />}
                  variant="unstyled"
                  cursor="pointer"
                  href={data?.uri}
                >
                  View on
                  {' '}
                  {metadataStorageProvider}
                </Button>
                <Text fontSize="xs" fontWeight={600} color={secondaryAttributeColor[colorMode]}>Metadata storage</Text>
              </Box>
            </Grid>
          </VStack>
        </VStack>
        <VStack alignItems="flex-start">
          <Heading size="md">
            Created by
          </Heading>
          <Wrap pt={4}>
            {
              data?.metadata?.properties.creators.map((creator) => (
                <CreatorTag key={creator.address} address={creator.address} />
              ))
            }
          </Wrap>
        </VStack>
        <VStack alignItems="flex-start" width="100%">
          <Heading size="md">
            Attributes
          </Heading>
          <SimpleGrid columns={2} width="100%" gap={0} pt={4}>
            {
              data?.metadata?.attributes?.map((attribute) => (
                <React.Fragment key={attribute.trait_type}>
                  <Flex justifyContent="left" key={attribute.trait_type}>
                    <Text
                      fontWeight={300}
                      color={secondaryAttributeColor[colorMode]}
                    >
                      {attribute.trait_type}
                    </Text>
                  </Flex>
                  <Flex justifyContent="right" key={attribute.value}>
                    <Text
                      fontWeight={500}
                    >
                      {attribute.value}
                    </Text>
                  </Flex>
                </React.Fragment>
              ))
            }
          </SimpleGrid>
        </VStack>
      </VStack>
    </VStack>
  );
}

export default TokenBody;
