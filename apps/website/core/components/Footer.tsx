import {
  Box,
  Center, Divider, Flex, Grid, HStack, Text, useColorMode, VStack,
} from '@chakra-ui/react';
import { COMPANY_NAME } from 'core/constants';
import ChakraLink from './ChakraLink';
import { secondaryTextColor } from './LoginDemo';

const secondaryBgColor = {
  dark: 'whiteAlpha.100',
  light: 'blackAlpha.50',
};

export default function Footer() {
  const { colorMode } = useColorMode();
  const year = new Date().getFullYear();
  return (
    <Flex width="100%" bgColor={secondaryBgColor[colorMode]} justifyContent="center" py={8}>
      <VStack as="footer" maxW="800px" width="100%" divider={<Divider />} spacing={4} px={4}>
        <Grid templateColumns="107px 1fr" width="100%">
          <Center>
            <ChakraLink href="/" fontSize="lg" fontWeight={600} verticalAlign="middle">
              {COMPANY_NAME}
            </ChakraLink>
          </Center>
          <HStack justifyContent="flex-end" spacing={[4, 4, 8]}>
            <ChakraLink color={secondaryTextColor[colorMode]} href="/about">
              About
            </ChakraLink>
            <ChakraLink color={secondaryTextColor[colorMode]} href="/legal">
              Legal
            </ChakraLink>
            <ChakraLink color={secondaryTextColor[colorMode]} href="/privacy-policy">
              Privacy policy
            </ChakraLink>
            <ChakraLink color={secondaryTextColor[colorMode]} href="press-kit">
              Press kit
            </ChakraLink>
          </HStack>
        </Grid>
        <Box width="100%">
          <Text color={secondaryTextColor[colorMode]}>
            ©
            {' '}
            {year}
            {' '}
            <a href="https://aptoslabs.com/" className="hover:underline">{COMPANY_NAME}</a>
            . All Rights Reserved.
          </Text>
        </Box>
      </VStack>
    </Flex>

  );
}
