// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {
  Center,
  Flex,
  Grid,
  Heading,
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
import { useGalleryItems } from 'core/queries/collectibles';
import AuthLayout from 'core/layouts/AuthLayout';
import { Routes as PageRoutes } from 'core/routes';
import { secondaryBorderColor } from 'core/colors';

function Gallery() {
  const { colorMode } = useColorMode();
  const {
    data: galleryItems,
    isLoading,
  } = useGalleryItems();

  return (
    <AuthLayout routePath={PageRoutes.gallery.path}>
      <WalletLayout title="Collectibles">
        <VStack width="100%" paddingTop={8} px={4}>
          {
            isLoading
              ? <Center pt={4}><Spinner size="lg" /></Center>
              : (
                <>
                  <Grid pb={4} templateColumns="1fr 72px" width="100%">
                    <Heading fontWeight={500} fontSize="md">Created by you</Heading>
                    <Flex justifyContent="right">
                      <CreateNFTDrawer />
                    </Flex>
                  </Grid>
                  <SimpleGrid w="100%" columns={2} spacing={2}>
                    {
                      (galleryItems && galleryItems!.length > 0)
                        ? (
                          galleryItems?.map((item) => (
                            <GalleryItem
                              id={item.id}
                              key={`${item.name}`}
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
                </>
              )
          }
        </VStack>
      </WalletLayout>
    </AuthLayout>
  );
}

export default Gallery;
