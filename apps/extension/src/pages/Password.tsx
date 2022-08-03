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
import { AptosBlackLogo, AptosWhiteLogo } from 'core/components/AptosLogo';
import { secondaryBgColor } from 'core/colors';
import React, { useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { unlockAccounts } from 'core/utils/account';
import { useNavigate } from 'react-router-dom';
import Routes from 'core/routes';

interface FormValues {
  password: string;
}

interface Props {
  isBackground: boolean
  onUnlock: () => void
}

function Password({ isBackground, onUnlock }: Props) {
  const { colorMode } = useColorMode();
  const [error, setError] = useState<string | undefined>(undefined);
  const { handleSubmit, register } = useForm<FormValues>({
    defaultValues: {
      password: '',
    },
  });

  const onSubmit: SubmitHandler<Record<string, any>> = async (data, event) => {
    event?.preventDefault();
    const accounts = await unlockAccounts(data.password, isBackground ?? false);
    if (accounts) {
      if (onUnlock) {
        onUnlock();
      }
    } else {
      setError('Incorrect password');
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
      <form onChange={() => setError(undefined)} onSubmit={handleSubmit(onSubmit)}>
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
            {error
              ? (
                <Text fontSize="sm" color="red.400">
                  {error}
                </Text>
              )
              : null}
          </FormControl>

          <Button width="100%" type="submit" colorScheme="teal">
            Login
          </Button>
        </VStack>
      </form>
    </VStack>
  );
}

// Used for password in main navigation stack
export function NavigationPassword() {
  const navigate = useNavigate();
  return (
    <Password isBackground={false} onUnlock={() => { navigate(Routes.wallet.routePath); }} />
  );
}

export default Password;
