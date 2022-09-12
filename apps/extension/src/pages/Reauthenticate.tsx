import React from 'react';
import {
  Box,
  Button,
  Center,
  Flex,
  FormControl,
  FormLabel,
  Input,
  Text,
  useColorMode,
  useDisclosure,
  VStack,
  chakra,
  Heading,
  InputGroup,
  InputRightElement,
} from '@chakra-ui/react';

import { SubmitHandler, useForm } from 'react-hook-form';
import { AptosBlackLogo, AptosWhiteLogo } from 'core/components/AptosLogo';
import WalletLayout from 'core/layouts/WalletLayout';
import { secondaryBgColor, textColor } from 'core/colors';
import { AiOutlineEye } from '@react-icons/all-files/ai/AiOutlineEye';
import { AiOutlineEyeInvisible } from '@react-icons/all-files/ai/AiOutlineEyeInvisible';
import { useInitializedAccounts } from 'core/hooks/useAccounts';
import ResetPasswordConfirmationModal from '../core/components/ResetPasswordConfirmationModal';

interface FormValues {
  hasReauthenticated: boolean;
  password: string;
  show: boolean;
}

type ReauthenticateProps = {
  children: JSX.Element;
};

function Reauthenticate({ children }: ReauthenticateProps) {
  const { colorMode } = useColorMode();
  const { clearAccounts, unlockAccounts } = useInitializedAccounts();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const {
    formState: { errors },
    getValues,
    handleSubmit,
    register,
    setError,
    setValue,
    watch,
  } = useForm<FormValues>({
    defaultValues: { hasReauthenticated: false, password: '', show: false },
    reValidateMode: 'onSubmit',
  });

  const hasReauthenticated = watch('hasReauthenticated');
  const show = watch('show');

  const onSubmit: SubmitHandler<FormValues> = async ({ password }, event) => {
    event?.preventDefault();
    try {
      await unlockAccounts!(password);
      setValue('hasReauthenticated', true);
    } catch (error: any) {
      setError('password', { message: 'Incorrect password', type: 'validate' });
    }
  };

  const handleClickResetPassword = () => {
    onOpen();
  };

  const handleConfirmResetPassword = async () => {
    await clearAccounts();
    onClose();
  };

  const handleToggleShowPassword = () => {
    setValue('show', !getValues('show'));
  };

  // prompt user for password if they have not yet re-authenticated
  if (!hasReauthenticated) {
    return (
      <WalletLayout title="Reauthentication" showBackButton>
        <VStack
          bgColor={secondaryBgColor[colorMode]}
          justifyContent="center"
          spacing={4}
          width="100%"
          height="100%"
        >
          <Center
            w="100%"
            pt={8}
            h="100%"
            display="flex"
            flexDir="column"
          >
            <Box width="72px" pb={4}>
              {
            (colorMode === 'dark')
              ? <AptosWhiteLogo />
              : <AptosBlackLogo />
          }
            </Box>
            <Heading fontSize="xl" fontWeight="600" color={textColor[colorMode]}>
              Please re-enter your password
            </Heading>
          </Center>
          <chakra.form onSubmit={handleSubmit(onSubmit)} width="100%" px={6}>
            <VStack gap={4}>
              <FormControl display="flex" flexDir="column" isRequired>
                <Flex flexDirection="row">
                  <FormLabel
                    requiredIndicator={<span />}
                    fontSize="md"
                    fontWeight={500}
                    flex={1}
                  >
                    Password
                  </FormLabel>
                  <Button
                    cursor="pointer"
                    onClick={handleClickResetPassword}
                    fontWeight={500}
                    fontSize="md"
                    colorScheme="teal"
                    variant="link"
                  >
                    Reset password
                  </Button>
                </Flex>
                <ResetPasswordConfirmationModal
                  onConfirm={handleConfirmResetPassword}
                  isOpen={isOpen}
                  onClose={onClose}
                />
                <InputGroup>
                  <Input
                    autoComplete="false"
                    variant="filled"
                    type={show ? 'text' : 'password'}
                    placeholder="Password..."
                    maxLength={64}
                    {...register('password')}
                  />
                  <InputRightElement width="4.5rem">
                    {show
                      ? <AiOutlineEyeInvisible size={28} onClick={handleToggleShowPassword} />
                      : <AiOutlineEye size={28} onClick={handleToggleShowPassword} />}
                  </InputRightElement>
                </InputGroup>
                {
                    errors.password && (
                      <Text fontSize="sm" color="red.400">
                        {errors.password.message}
                      </Text>
                    )
                  }
              </FormControl>
              <Box w="100%" pb={4}>
                <Button py={6} width="100%" type="submit" colorScheme="teal">
                  Re-authenticate
                </Button>
              </Box>
            </VStack>
          </chakra.form>
        </VStack>
      </WalletLayout>
    );
  }

  // show the wrapped children after correct password has been entered
  return children;
}

export default Reauthenticate;
