import { AptosBlackLogo, AptosWhiteLogo } from '@aptos-wallet/web-ui';
import {
  Box,
  Button,
  Center,
  Flex,
  Heading,
  Input,
  InputGroup,
  InputRightAddon,
  Text,
  useColorMode,
  VStack,
} from '@chakra-ui/react';

export const secondaryBgColor = {
  dark: 'gray.900',
  light: 'white',
};

export const secondaryTextColor = {
  dark: 'gray.400',
  light: 'gray.500',
};

const imageBoxShadow = 'rgba(100, 100, 111, 0.2) 0px 7px 29px 0px';

export default function LoginDemo() {
  const { colorMode } = useColorMode();

  return (
    <Box height="600px" width="100%" maxW="375px" border="1px solid" borderColor="blackAlpha.300" borderRadius=".5rem" boxShadow={imageBoxShadow}>
      <VStack
        bgColor={secondaryBgColor[colorMode]}
        justifyContent="center"
        spacing={4}
        width="100%"
        height="100%"
        borderRadius=".5rem"
        px={4}
      >
        <Flex w="100%" flexDir="column">
          <Center>
            <Box width="75px" pb={4}>
              {
              (colorMode === 'dark')
                ? <AptosWhiteLogo />
                : <AptosBlackLogo />
            }
            </Box>
          </Center>
          <Heading textAlign="center">Wallet</Heading>
          <Text
            textAlign="center"
            pb={8}
            color={secondaryTextColor[colorMode]}
            fontSize="lg"
          >
            An Aptos crypto wallet
          </Text>
          <form>
            <VStack spacing={4}>
              <Center minW="100%" px={4}>
                <Box>
                  <InputGroup>
                    <Input
                      maxW="350px"
                      variant="filled"
                      required
                      placeholder="Private key..."
                      autoComplete="off"
                    />
                    <InputRightAddon>
                      <Button type="button" variant="unstyled">
                        Submit
                      </Button>
                    </InputRightAddon>
                  </InputGroup>
                  <Center />
                </Box>
              </Center>
              <Button colorScheme="teal" variant="ghost">
                Create a new wallet
              </Button>
            </VStack>
          </form>
        </Flex>
        {/* TODO: Fill this in later */}
        {/* <HStack spacing={2} color={secondaryTextColor[colorMode]}>
        <QuestionIcon />
        <ChakraLink to="/help" fontSize="xs">
          Help
        </ChakraLink>
      </HStack> */}
      </VStack>
    </Box>

  );
}
