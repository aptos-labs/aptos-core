import React from "react";
import {
  FormControl,
  InputLabel,
  OutlinedInput,
  FormHelperText,
} from "@mui/material";

interface UrlTextFieldProps {
  label: string;
  url: string;
  urlIsValid: boolean;
  errorMessage: string;
  onUrlChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
}

export default function UrlTextField({
  label,
  url,
  urlIsValid,
  errorMessage,
  onUrlChange,
}: UrlTextFieldProps): JSX.Element {
  return urlIsValid ? (
    <FormControl fullWidth>
      <InputLabel htmlFor="outlined-adornment-url">{label}</InputLabel>
      <OutlinedInput notched label={label} value={url} onChange={onUrlChange} />
    </FormControl>
  ) : (
    <FormControl fullWidth>
      <InputLabel error htmlFor="outlined-adornment-url">
        {label}
      </InputLabel>
      <OutlinedInput
        error
        notched
        label={label}
        value={url}
        onChange={onUrlChange}
      />
      <FormHelperText error>{errorMessage}</FormHelperText>
    </FormControl>
  );
}
