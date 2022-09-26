import React from 'react';
import {
  Circle, SquareProps, Icon,
} from '@chakra-ui/react';
import { HiCheck } from '@react-icons/all-files/hi/HiCheck';
import { customColors } from 'core/colors';

interface RadioCircleProps extends SquareProps {
  isActive: boolean,
  isCompleted: boolean
}

function StepCircle(props: RadioCircleProps) {
  const { isActive, isCompleted, ...rest } = props;
  return (
    <Circle
      size="5"
      background={isCompleted ? customColors.navy[900] : 'inherit'}
      borderWidth={isCompleted ? '0' : '5px'}
      borderColor={isActive ? 'gray.200' : 'inherit'}
      {...rest}
    >
      {isCompleted ? (
        <Icon as={HiCheck} color="white" boxSize="3" />
      ) : (
        <Circle bg={isActive ? customColors.navy[900] : 'gray.400'} size="2" />
      )}
    </Circle>
  );
}

export default StepCircle;
