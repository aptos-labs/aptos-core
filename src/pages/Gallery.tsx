// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Center,
  Flex,
  Grid,
  Heading,
  SimpleGrid,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import React from 'react';
import SquareBox from 'core/components/SquareBox';
import CreateNFTModal from 'core/components/CreateNFTModal';
import GalleryItem from 'core/components/GalleryItem';
import withSimulatedExtensionContainer from 'core/components/WithSimulatedExtensionContainer';
import WalletLayout from 'core/layouts/WalletLayout';
import { useGalleryItems } from 'core/queries/collectibles';

const secondaryBorderColor = {
  dark: 'gray.600',
  light: 'gray.200',
};

function Gallery() {
  const { colorMode } = useColorMode();
  const {
    data: galleryItems,
  } = useGalleryItems();

  return (
    <WalletLayout>
      <VStack width="100%" paddingTop={8} px={4}>
        <Grid pb={4} templateColumns="1fr 72px" width="100%">
          <Heading fontSize="xl">Collectibles</Heading>
          <Flex justifyContent="right">
            <CreateNFTModal />
          </Flex>
        </Grid>
        <SimpleGrid w="100%" columns={2} spacing={2}>
          {
            (galleryItems && galleryItems.length > 0)
              ? (
                galleryItems?.map((item) => (
                  <GalleryItem
                    key={`${item.name}`}
                    imageSrc={item.metadata?.image || 'https://www.publicdomainpictures.net/pictures/280000/nahled/not-found-image-15383864787lu.jpg'}
                  />
                ))
              )
              : (
                <SquareBox borderWidth="1px" borderRadius=".5rem" borderColor={secondaryBorderColor[colorMode]}>
                  <Center height="100%" p={4}>
                    <Text textAlign="center">No collectibles yet!</Text>
                  </Center>
                </SquareBox>
              )
          }
        </SimpleGrid>
      </VStack>
    </WalletLayout>
  );
}

export default withSimulatedExtensionContainer(Gallery);
