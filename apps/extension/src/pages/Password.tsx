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
} from '@chakra-ui/react';

import { SubmitHandler, useForm } from 'react-hook-form';
import { secondaryBgColor, textColor } from 'core/colors';
import { AptosBlackLogo, AptosWhiteLogo } from 'core/components/AptosLogo';
import useGlobalStateContext from 'core/hooks/useGlobalState';
import ResetPasswordConfirmationModal from '../core/components/ResetPasswordConfirmationModal';

interface FormValues {
  password: string;
}

function Password() {
  const { colorMode } = useColorMode();
  const { resetAccount, unlockAccounts } = useGlobalStateContext();
  const { isOpen, onClose, onOpen } = useDisclosure();

  const {
    formState: { errors },
    handleSubmit,
    register,
    setError,
  } = useForm<FormValues>({
    defaultValues: { password: '' },
    reValidateMode: 'onSubmit',
  });

  const onSubmit: SubmitHandler<FormValues> = async ({ password }, event) => {
    event?.preventDefault();
    try {
      await unlockAccounts!(password);
      // Note: redirection occurs automatically, see routing
    } catch (error: any) {
      setError('password', { message: 'Incorrect password', type: 'validate' });
    }
  };

  const handleClickResetPassword = (e: React.MouseEvent<HTMLParagraphElement>) => {
    e.preventDefault();
    onOpen();
  };

  const handleConfirmResetPassword = async () => {
    await resetAccount();
    onClose();
  };

  return (
    <VStack
      bgColor={secondaryBgColor[colorMode]}
      justifyContent="center"
      spacing={4}
      width="100%"
      height="100%"
    >
      <Center h="100%" display="flex" flexDir="column">
        <Box width="100%" pb={4} px={20}>
          {
            (colorMode === 'dark')
              ? <AptosWhiteLogo />
              : <AptosBlackLogo />
          }
        </Box>
        <Text fontSize="4xl" fontWeight="700" color={textColor[colorMode]}>
          Welcome back
        </Text>
      </Center>
      <chakra.form onSubmit={handleSubmit(onSubmit)} width="100%" p={6}>
        <VStack gap={4}>
          <FormControl display="flex" flexDir="column" isRequired>
            <Flex flexDirection="row">
              <FormLabel
                requiredIndicator={<span />}
                fontSize="sm"
                fontWeight={500}
                flex={1}
              >
                Password
              </FormLabel>
              <FormLabel
                requiredIndicator={<span />}
                fontSize="sm"
                fontWeight={500}
              >
                <Text cursor="pointer" as="u" onClick={handleClickResetPassword} fontWeight={500} fontSize="sm">Reset password</Text>
              </FormLabel>
            </Flex>
            <ResetPasswordConfirmationModal
              onConfirm={handleConfirmResetPassword}
              isOpen={isOpen}
              onClose={onClose}
            />
            <Input
              autoComplete="false"
              variant="filled"
              placeholder="Password..."
              type="password"
              {...register('password')}
            />
            {
              errors.password && (
                <Text fontSize="sm" color="red.400">
                  {errors.password.message}
                </Text>
              )
            }
          </FormControl>

          <Button width="100%" type="submit" colorScheme="teal">
            Unlock
          </Button>
        </VStack>
      </chakra.form>
    </VStack>
  );
}

export default Password;
