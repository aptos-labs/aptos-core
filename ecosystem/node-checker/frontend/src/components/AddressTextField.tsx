import React from "react";
import {TextField} from "@mui/material";

interface AddressTextFieldProps {
  label: string;
  addr: string;
  addrIsValid: boolean;
  onAddrChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}

export default function AddressTextField({
  label,
  addr,
  addrIsValid,
  onAddrChange,
}: AddressTextFieldProps): JSX.Element {
  return addrIsValid ? (
    <TextField
      fullWidth
      variant="outlined"
      label={label}
      value={addr}
      onChange={onAddrChange}
    />
  ) : (
    <TextField
      error
      fullWidth
      variant="outlined"
      label={label}
      value={addr}
      onChange={onAddrChange}
      helperText="Incorrect address"
    />
  );
}
