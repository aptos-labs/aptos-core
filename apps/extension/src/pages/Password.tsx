import React from 'react';
import {
  Box,
  Button,
  Center,
  FormControl,
  FormLabel,
  Input,
  useColorMode,
  VStack,
  Text,
} from '@chakra-ui/react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { secondaryBgColor } from 'core/colors';
import { AptosBlackLogo, AptosWhiteLogo } from 'core/components/AptosLogo';
import useGlobalStateContext from 'core/hooks/useGlobalState';

interface FormValues {
  password: string;
}

function Password() {
  const { colorMode } = useColorMode();
  const { unlockAccounts } = useGlobalStateContext();

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

  return (
    <VStack
      bgColor={secondaryBgColor[colorMode]}
      justifyContent="center"
      spacing={4}
      width="100%"
      height="100%"
    >
      <Center>
        <Box width="75px" pb={4}>
          {
            (colorMode === 'dark')
              ? <AptosWhiteLogo />
              : <AptosBlackLogo />
          }
        </Box>
      </Center>
      <form onSubmit={handleSubmit(onSubmit)}>
        <VStack gap={4}>
          <FormControl display="flex" flexDir="column" isRequired alignItems="center">
            <FormLabel requiredIndicator={<span />} fontSize="lg" fontWeight={500} pb={2}>
              Enter your password
            </FormLabel>

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
      </form>
    </VStack>
  );
}

export default Password;
