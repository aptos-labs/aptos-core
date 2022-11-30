import React from "react";
import {
  FormControl,
  InputLabel,
  OutlinedInput,
  InputAdornment,
  FormHelperText,
} from "@mui/material";

interface AmountTextFieldProps {
  label: string;
  amount: string;
  amountIsValid: boolean;
  errorMessage: string;
  onAmountChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}

export default function AmountTextField({
  label,
  amount,
  amountIsValid,
  errorMessage,
  onAmountChange,
}: AmountTextFieldProps): JSX.Element {
  return amountIsValid ? (
    <FormControl fullWidth>
      <InputLabel htmlFor="outlined-adornment-amount">{label}</InputLabel>
      <OutlinedInput
        notched
        label={label}
        value={amount}
        onChange={onAmountChange}
        startAdornment={<InputAdornment position="start">$</InputAdornment>}
      />
    </FormControl>
  ) : (
    <FormControl fullWidth>
      <InputLabel error htmlFor="outlined-adornment-amount">
        {label}
      </InputLabel>
      <OutlinedInput
        error
        notched
        label={label}
        value={amount}
        onChange={onAmountChange}
        startAdornment={<InputAdornment position="start">$</InputAdornment>}
      />
      <FormHelperText error>{errorMessage}</FormHelperText>
    </FormControl>
  );
}
