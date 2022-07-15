import { Alert, AlertIcon } from '@chakra-ui/react';

interface Props {
  children: JSX.Element,
}

export default function Info({ children }: Props) {
  return (
    <Alert status="info">
      <AlertIcon />
      {children}
    </Alert>
  );
}
