import React from "react";
import {
  FormControl,
  InputLabel,
  OutlinedInput,
  FormHelperText,
} from "@mui/material";

interface PortTextFieldProps {
  label: string;
  port: string;
  portIsValid: boolean;
  errorMessage: string;
  onPortChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}

export default function PortTextField({
  label,
  port,
  portIsValid,
  errorMessage,
  onPortChange,
}: PortTextFieldProps): JSX.Element {
  return portIsValid ? (
    <FormControl fullWidth>
      <InputLabel htmlFor="outlined-adornment-amount">{label}</InputLabel>
      <OutlinedInput
        notched
        label={label}
        value={port}
        onChange={onPortChange}
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
        value={port}
        onChange={onPortChange}
      />
      <FormHelperText error>{errorMessage}</FormHelperText>
    </FormControl>
  );
}
