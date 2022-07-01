import {
  Box,
  Center,
  Divider,
  Flex,
  Grid,
  HStack,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { COMPANY_NAME, CURRENT_YEAR } from 'core/constants';
import ChakraLink from './ChakraLink';
import { secondaryTextColor } from './LoginDemo';

const secondaryBgColor = {
  dark: 'whiteAlpha.100',
  light: 'blackAlpha.50',
};

export default function Footer() {
  const { colorMode } = useColorMode();
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
            <ChakraLink color={secondaryTextColor[colorMode]} href="/about" target="_blank">
              About
            </ChakraLink>
            <ChakraLink color={secondaryTextColor[colorMode]} href="/legal" target="_blank">
              Legal
            </ChakraLink>
            <ChakraLink color={secondaryTextColor[colorMode]} href="/privacy-policy" target="_blank">
              Privacy policy
            </ChakraLink>
            <ChakraLink color={secondaryTextColor[colorMode]} href="/press-kit" target="_blank">
              Press kit
            </ChakraLink>
          </HStack>
        </Grid>
        <Box width="100%">
          <Text color={secondaryTextColor[colorMode]}>
            ©
            {' '}
            {CURRENT_YEAR}
            {' '}
            <ChakraLink href="https://aptoslabs.com/" target="_blank">{COMPANY_NAME}</ChakraLink>
            . All Rights Reserved.
          </Text>
        </Box>
      </VStack>
    </Flex>
  );
}
