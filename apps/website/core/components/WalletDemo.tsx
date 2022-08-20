import {
  Box,
  Button,
  Drawer,
  DrawerBody,
  DrawerContent,
  DrawerFooter,
  DrawerHeader,
  DrawerOverlay,
  Flex,
  Grid,
  Heading,
  HStack,
  Input,
  SimpleGrid,
  Text,
  useColorMode,
  useDisclosure,
  VStack,
  Wrap,
} from '@chakra-ui/react';
import { IoIosSend } from '@react-icons/all-files/io/IoIosSend';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faFaucet } from '@fortawesome/free-solid-svg-icons/faFaucet';
import { ChevronRightIcon, ExternalLinkIcon } from '@chakra-ui/icons';
import { useRef, useState } from 'react';
import numeral from 'numeral';
import { useForm } from 'react-hook-form';
import {
  secondaryAddressFontColor,
  secondaryDividerColor,
  secondaryErrorMessageColor,
  secondaryTextColorWalletDemo,
  secondaryWalletHomeCardBgColor,
  secondaryWalletNoteBgColor,
} from 'core/colors';
import dynamic from 'next/dynamic';
import WalletDemoFooter from './WalletDemoFooter';
import WalletDemoHeader from './WalletDemoHeader';
import ChakraLink from './ChakraLink';

const ReactConfetti = dynamic(() => import('react-confetti'), { ssr: false });

const imageBoxShadow = 'rgba(100, 100, 111, 0.2) 0px 7px 29px 0px';

/**
 * @see https://boringavatars.com/
 */
function GraceHopperBoringAvatar() {
  const beam = (
    <svg viewBox="0 0 36 36" fill="none" role="img" xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
      <title>Grace Hopper</title>
      <mask id="mask__beam" maskUnits="userSpaceOnUse" x="0" y="0" width="36" height="36"><rect width="36" height="36" rx="72" fill="#FFFFFF" /></mask>
      <g mask="url(#mask__beam)">
        <rect width="36" height="36" fill="#d1eaee" />
        <rect x="0" y="0" width="36" height="36" transform="translate(0 8) rotate(44 18 18) scale(1.2)" fill="#efb0a9" rx="36" />
        <g transform="translate(-4 4) rotate(-4 18 18)">
          <path d="M13,21 a1,0.75 0 0,0 10,0" fill="#000000" />
          <rect x="10" y="14" width="1.5" height="2" rx="1" stroke="none" fill="#000000" />
          <rect x="24" y="14" width="1.5" height="2" rx="1" stroke="none" fill="#000000" />
        </g>
      </g>
    </svg>
  );

  return beam;
}

interface WalletDemoFormValues {
  sendAmount: number;
}

export default function WalletDemo() {
  const { colorMode } = useColorMode();
  const containerRef = useRef(null);

  const { isOpen, onClose, onOpen } = useDisclosure();
  const [coinBalance, setCoinBalance] = useState<number>(150000);
  const [faucetIsLoading, setFaucetIsLoading] = useState<boolean>(false);
  const [sendIsLoading, setSendIsLoading] = useState<boolean>(false);
  const [party, setParty] = useState<boolean>(false);
  const { register, watch } = useForm<WalletDemoFormValues>({
    defaultValues: {
      sendAmount: 0,
    },
  });

  const sendAmount = Number(watch('sendAmount'));

  const coinBalanceString = numeral(coinBalance).format('0,0');

  const isFundsSufficient = ((sendAmount + 2) <= coinBalance);

  const faucetOnClick = () => {
    setFaucetIsLoading(true);
    setTimeout(() => {
      setCoinBalance(coinBalance + 50000);
      setFaucetIsLoading(false);
    }, 1000);
  };

  const sendOnClick = () => {
    if (!isFundsSufficient) {
      return;
    }
    setSendIsLoading(true);
    setTimeout(() => {
      setParty(true);
      setSendIsLoading(false);
      setCoinBalance(coinBalance - (sendAmount + 2));
      onClose();
      setTimeout(() => {
        setParty(false);
      }, 2000);
    }, 1000);
  };

  return (
    <Box
      height="600px"
      width="375px"
      maxW="375px"
      border="1px solid"
      borderColor="blackAlpha.300"
      borderRadius=".5rem"
      boxShadow={imageBoxShadow}
      ref={containerRef}
    >
      <ReactConfetti
        style={{ pointerEvents: 'none' }}
        numberOfPieces={party ? 500 : 0}
        recycle={false}
      />
      <Grid
        height="100%"
        width="100%"
        maxW="100%"
        templateRows="64px 1fr 60px"
      >
        <WalletDemoHeader />
        <form>
          <VStack width="100%" p={4}>
            <Flex
              py={4}
              width="100%"
              flexDir="column"
              borderRadius=".5rem"
              bgColor={secondaryWalletHomeCardBgColor[colorMode]}
            >
              <HStack spacing={0} alignItems="flex-end">
                <VStack px={4} alignItems="left">
                  <Text fontSize="sm" color={secondaryAddressFontColor[colorMode]}>Account balance</Text>
                  <Wrap alignItems="baseline">
                    <span>
                      <Heading as="span" wordBreak="break-word" maxW="100%">{coinBalanceString}</Heading>
                      <Text pl={2} pb="2px" as="span" fontSize="xl" fontWeight={600}>
                        APT
                      </Text>
                    </span>
                  </Wrap>
                </VStack>
              </HStack>
              <Flex width="100%" flexDir="column" px={4}>
                <HStack spacing={4} pt={4}>
                  <Button
                    leftIcon={<FontAwesomeIcon icon={faFaucet} />}
                    colorScheme="teal"
                    variant="outline"
                    onClick={faucetOnClick}
                    isLoading={faucetIsLoading}
                  >
                    Faucet
                  </Button>
                  <>
                    <Button
                      leftIcon={<IoIosSend />}
                      onClick={onOpen}
                      colorScheme="teal"
                    >
                      Send
                    </Button>
                    <Drawer
                      portalProps={{
                        appendToParentPortal: true,
                        containerRef,
                      }}
                      size="xl"
                      isOpen={isOpen}
                      onClose={onClose}
                      placement="bottom"
                    >
                      <DrawerOverlay />
                      <DrawerContent>
                        <DrawerHeader borderBottomWidth="1px" px={4}>
                          <HStack spacing={4}>
                            <Box width="32px">
                              <GraceHopperBoringAvatar />
                            </Box>
                            <VStack boxSizing="border-box" spacing={0} alignItems="flex-start" flexGrow={1}>
                              <Input
                                pb={1}
                                variant="unstyled"
                                size="sm"
                                fontWeight={600}
                                autoComplete="off"
                                spellCheck="false"
                                placeholder="Please enter an address"
                                value="0xBob"
                                readOnly
                              />
                              <Button
                                color={secondaryTextColorWalletDemo[colorMode]}
                                fontSize="sm"
                                fontWeight={400}
                                height="24px"
                                as="a"
                                target="_blank"
                                rightIcon={<ExternalLinkIcon />}
                                variant="unstyled"
                                cursor="pointer"
                                href="https://explorer.devnet.aptos.dev/account/0x1"
                                tabIndex={-1}
                              >
                                View on explorer
                              </Button>
                            </VStack>
                          </HStack>
                        </DrawerHeader>
                        <DrawerBody px={0} py={0}>
                          <VStack spacing={0}>
                            <Input
                              autoComplete="off"
                              textAlign="center"
                              type="number"
                              variant="filled"
                              placeholder="0"
                              py={32}
                              fontSize={64}
                              borderRadius="0px"
                              size="lg"
                              _focusVisible={{
                                outline: 'none',
                              }}
                              min={0}
                              required
                              {...register('sendAmount')}
                            />
                            <VStack
                              borderTopWidth="1px"
                              borderTopColor={secondaryDividerColor[colorMode]}
                              p={4}
                              width="100%"
                              spacing={0}
                              mt={0}
                            >
                              <SimpleGrid width="100%" columns={2} gap={1}>
                                <Flex>
                                  <Text fontWeight={600} fontSize="md">
                                    Balance
                                  </Text>
                                </Flex>
                                <Flex justifyContent="right">
                                  <Text color={secondaryTextColorWalletDemo[colorMode]} fontSize="md">
                                    {coinBalanceString}
                                    {' '}
                                    coins
                                  </Text>
                                </Flex>
                                <Flex>
                                  <Text fontWeight={600} fontSize="md">
                                    Fee
                                  </Text>
                                </Flex>
                                <Flex justifyContent="right">
                                  <Text color={secondaryTextColorWalletDemo[colorMode]} fontSize="md" as="span">
                                    2
                                    { ' coins' }
                                  </Text>
                                </Flex>
                              </SimpleGrid>
                              <Flex overflowY="auto" maxH="100px">
                                <Text
                                  fontSize="xs"
                                  color={secondaryErrorMessageColor[colorMode]}
                                  wordBreak="break-word"
                                >
                                  {(isFundsSufficient) ? '' : 'Insufficient funds'}
                                </Text>
                              </Flex>
                            </VStack>
                          </VStack>
                        </DrawerBody>
                        <DrawerFooter borderTopColor={secondaryDividerColor[colorMode]} borderTopWidth="1px" px={4}>
                          <Grid gap={4} width="100%" templateColumns="2fr 1fr">
                            <Button
                              colorScheme="teal"
                              onClick={sendOnClick}
                              isLoading={sendIsLoading}
                              isDisabled={!isFundsSufficient}
                            >
                              Send
                              {' '}
                              {sendAmount}
                              {' '}
                              coins
                            </Button>
                            <Button onClick={onClose}>
                              Cancel
                            </Button>
                          </Grid>
                        </DrawerFooter>
                      </DrawerContent>
                    </Drawer>
                  </>
                </HStack>
              </Flex>
            </Flex>
            <Button
              py={6}
              width="100%"
              rightIcon={<ChevronRightIcon />}
              justifyContent="space-between"
              isDisabled
            >
              View your activity
            </Button>
            <Box
              bgColor={secondaryWalletNoteBgColor[colorMode]}
              borderRadius=".5rem"
              px={4}
              py={4}
              width="100%"
            >
              <b>Note:</b>
              {' '}
              <b>This is a demo</b>
              {' '}
              and has limited functionality
            </Box>
            <Box
              opacity={0.5}
              borderRadius=".5rem"
              px={4}
              py={4}
              width="100%"
              fontSize="xs"
              pt={24}
              textAlign="center"
            >
              Made with ❤️ by
              {' '}
              <ChakraLink target="_blank" href="https://aptoslabs.com/">Aptos Labs</ChakraLink>
            </Box>
          </VStack>
        </form>
        <WalletDemoFooter pathname="/wallet" />
      </Grid>
    </Box>
  );
}
