// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Flex,
  Center,
  SimpleGrid,
  Spinner,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import SquareBox from 'core/components/SquareBox';
import CreateNFTDrawer from 'core/components/CreateNFTDrawer';
import GalleryItem from 'core/components/GalleryItem';
import WalletLayout from 'core/layouts/WalletLayout';
import { TokenAttributes, useDepositTokens } from 'core/queries/collectibles';
import { secondaryBorderColor } from 'core/colors';

interface SectionProps {
  galleryItems?: TokenAttributes[];
  isLoading: boolean;
}

function GallerySection({ galleryItems, isLoading }: SectionProps) {
  const { colorMode } = useColorMode();
  return (
    isLoading
      ? <Center pt={4}><Spinner size="lg" /></Center>
      : (
        <SimpleGrid w="100%" columns={2} spacing={2}>
          {
            (galleryItems && galleryItems.length > 0)
              ? (
                galleryItems?.map((item) => (
                  <GalleryItem
                    id={item.id}
                    key={item.name}
                    imageSrc={item.metadata?.image || 'https://www.publicdomainpictures.net/pictures/280000/nahled/not-found-image-15383864787lu.jpg'}
                  />
                ))
              )
              : (
                <SquareBox borderWidth="1px" borderRadius=".5rem" borderColor={secondaryBorderColor[colorMode]}>
                  <Center height="100%" p={4}>
                    <Text fontSize="md" textAlign="center">No collectibles yet!</Text>
                  </Center>
                </SquareBox>
              )
          }
        </SimpleGrid>
      )
  );
}

function Gallery() {
  const {
    data: depositItems,
    isLoading: isDepositLoading,
  } = useDepositTokens();

  return (
    <WalletLayout title="Collectibles">
      <VStack width="100%" paddingTop={4} px={4}>
        <Flex alignSelf="end">
          <CreateNFTDrawer />
        </Flex>
        <GallerySection galleryItems={depositItems} isLoading={isDepositLoading} />
      </VStack>
    </WalletLayout>
  );
}

export default Gallery;
