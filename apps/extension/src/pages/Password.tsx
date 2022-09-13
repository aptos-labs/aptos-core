import React, { useState } from 'react';
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
  Icon,
  InputRightElement,
} from '@chakra-ui/react';

import { SubmitHandler, useForm } from 'react-hook-form';
import { secondaryBgColor, textColor } from 'core/colors';
import { AptosBlackLogo, AptosWhiteLogo } from 'core/components/AptosLogo';
import { AiFillEyeInvisible } from '@react-icons/all-files/ai/AiFillEyeInvisible';
import { AiFillEye } from '@react-icons/all-files/ai/AiFillEye';
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
        <Heading fontSize="3xl" fontWeight="600" color={textColor[colorMode]}>
          Welcome back
        </Heading>
      </Center>
      <chakra.form onSubmit={handleSubmit(onSubmit)} width="100%" p={6}>
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
                type={showPassword ? 'text' : 'password'}
                placeholder="Password..."
                maxLength={64}
                {...register('password')}
              />
              <InputRightElement width="3rem">
                <Button
                  tabIndex={-3}
                  borderRadius="100%"
                  variant="ghost"
                  h="1.75rem"
                  size="sm"
                  onClick={handleClickShowPassword}
                >
                  {showPassword
                    ? <Icon as={AiFillEyeInvisible} />
                    : <Icon as={AiFillEye} />}
                </Button>
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
              Unlock
            </Button>
          </Box>
        </VStack>
      </chakra.form>
    </VStack>
  );
}

export default Password;
