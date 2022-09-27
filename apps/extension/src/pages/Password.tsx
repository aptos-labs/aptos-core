import React, { useState } from 'react';
import {
  Box,
  Button,
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
import { mainBgColor, passwordBgColor } from 'core/colors';
import { PetraLogo } from 'core/components/PetraLogo';
import { AiOutlineEye } from '@react-icons/all-files/ai/AiOutlineEye';
import { AiOutlineEyeInvisible } from '@react-icons/all-files/ai/AiOutlineEyeInvisible';
import { useInitializedAccounts } from 'core/hooks/useAccounts';
import ResetPasswordConfirmationModal from '../core/components/ResetPasswordConfirmationModal';

interface FormValues {
  password: string;
}

function Password() {
  const { colorMode } = useColorMode();
  const { clearAccounts, unlockAccounts } = useInitializedAccounts();
  const { isOpen, onClose, onOpen } = useDisclosure();
  const [showPassword, setShowPassword] = useState<boolean>(false);

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
      await unlockAccounts(password);
      // Note: redirection occurs automatically, see routing
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

  const handleClickShowPassword = () => {
    setShowPassword(!showPassword);
  };

  return (
    <VStack
      bgColor={mainBgColor[colorMode]}
      justifyContent="center"
      spacing={4}
      width="100%"
      height="100%"
    >
      <Flex
        w="100%"
        pt={8}
        px={4}
        h="100%"
        justifyContent="center"
        flexDir="column"
      >
        <Box width="84px" pb={4}>
          <PetraLogo />
        </Box>
        <Heading fontSize="3xl" fontWeight="600" color="white">
          Welcome back
        </Heading>
      </Flex>
      <chakra.form onSubmit={handleSubmit(onSubmit)} width="100%" p={6}>
        <VStack gap={4}>
          <FormControl display="flex" flexDir="column" isRequired>
            <Flex flexDirection="row">
              <FormLabel
                requiredIndicator={<span />}
                fontSize="md"
                fontWeight={500}
                flex={1}
                color="white"
              >
                Password
              </FormLabel>
              <Button
                cursor="pointer"
                onClick={handleClickResetPassword}
                fontWeight={500}
                fontSize="md"
                color="navy.600"
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
                bgColor={passwordBgColor[colorMode]}
                height="48px"
                type={showPassword ? 'text' : 'password'}
                placeholder="Password..."
                maxLength={64}
                _hover={{
                  bgColor: 'navy.700',
                }}
                color="white"
                {...register('password')}
              />
              <InputRightElement width="3rem">

                {showPassword
                  ? <AiOutlineEyeInvisible size={24} onClick={handleClickShowPassword} color="white" />
                  : <AiOutlineEye size={24} onClick={handleClickShowPassword} color="white" />}
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
            <Button py={6} width="100%" type="submit" colorScheme="salmon">
              Unlock
            </Button>
          </Box>
        </VStack>
      </chakra.form>
    </VStack>
  );
}

export default Password;
