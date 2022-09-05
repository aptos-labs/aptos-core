import React from 'react';
import {
  Divider, HStack, StackProps, Flex,
} from '@chakra-ui/react';
import StepCircle from './StepCircle';

interface StepProps extends StackProps {
  isActive: boolean,
  isCompleted: boolean,
  isLastStep: boolean
}

function Step(props: StepProps) {
  const {
    isActive, isCompleted, isLastStep, ...stackProps
  } = props;

  const incompleteStyle = isLastStep ? 'transparent' : 'inherit';

  return (
    <HStack flex={isLastStep ? 0 : 1} spacing={0} {...stackProps}>
      <StepCircle isActive={isActive} isCompleted={isCompleted} />
      <Flex width="100%" justifyContent="center">
        <Divider
          width="70%"
          orientation="horizontal"
          borderWidth="1px"
          borderColor={isCompleted ? 'teal' : incompleteStyle}
        />
      </Flex>
    </HStack>
  );
}
export default Step;
