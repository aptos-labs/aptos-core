// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import { Box, Heading, SimpleGrid, VStack } from '@chakra-ui/react'
import React from 'react'
import GalleryItem from '../components/GalleryItem'
import withSimulatedExtensionContainer from '../components/WithSimulatedExtensionContainer'
import WalletLayout from '../Layouts/WalletLayout'

const Gallery = () => {
  return (
    <WalletLayout>
      <VStack width="100%" paddingTop={8} px={4}>
        <Box px={4} pb={4}>
          <Heading fontSize="xl" >Collectibles</Heading>
        </Box>
        <SimpleGrid w="100%" columns={2} spacing={2}>
          {
            [1, 2, 3, 4, 5, 6].map((item) => (
              <GalleryItem
                key={`${item}`}
                title={`${item}`}
                imageSrc="https://d15shllkswkct0.cloudfront.net/wp-content/blogs.dir/1/files/2021/07/phantom.jpg"
              />
            ))
          }
        </SimpleGrid>
      </VStack>
    </WalletLayout>
  )
}

export default withSimulatedExtensionContainer(Gallery)
