import React from 'react';
import { Circle, SquareProps, Icon } from '@chakra-ui/react';
import { HiCheck } from '@react-icons/all-files/hi/HiCheck';

interface RadioCircleProps extends SquareProps {
  isActive: boolean,
  isCompleted: boolean
}

function StepCircle(props: RadioCircleProps) {
  const { isActive, isCompleted, ...rest } = props;
  return (
    <Circle
      size="5"
      bg={isCompleted ? 'teal' : 'inherit'}
      borderWidth={isCompleted ? '0' : '2px'}
      borderColor={isActive ? 'accent' : 'inherit'}
      {...rest}
    >
      {isCompleted ? (
        <Icon as={HiCheck} color="white" boxSize="3" />
      ) : (
        <Circle bg={isActive ? 'teal' : 'gray.400'} size="2" />
      )}
    </Circle>
  );
}

export default StepCircle;
