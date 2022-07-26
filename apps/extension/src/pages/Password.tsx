import {
  Box,
  Button,
  Center,
  FormControl,
  FormLabel,
  Input,
  useColorMode,
  VStack,
} from '@chakra-ui/react';
import { AptosBlackLogo, AptosWhiteLogo } from 'core/components/AptosLogo';
import { secondaryBgColor } from 'core/colors';
import React from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';

interface FormValues {
  password: string;
}

function Password() {
  const { colorMode } = useColorMode();
  const { handleSubmit, register } = useForm<FormValues>({
    defaultValues: {
      password: '',
    },
  });

  const onSubmit: SubmitHandler<Record<string, any>> = async (_data, event) => {
    event?.preventDefault();
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
          </FormControl>
          <Button width="100%" type="submit" colorScheme="teal">
            Login
          </Button>
        </VStack>
      </form>
    </VStack>
  );
}

export default Password;
