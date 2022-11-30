import React from "react";
import {Alert, IconButton, Snackbar} from "@mui/material";
import CloseIcon from "@mui/icons-material/Close";

interface ErrorSnackbarProps {
  errorMessage: string | undefined;
  updateErrorMessage: (key: string | undefined) => void;
}

export default function ErrorSnackbar({
  errorMessage,
  updateErrorMessage,
}: ErrorSnackbarProps) {
  const onCloseSnackbar = () => {
    updateErrorMessage(undefined);
  };

  if (errorMessage === undefined) {
    return null;
  }

  return (
    <Snackbar
      open={true}
      anchorOrigin={{
        vertical: "top",
        horizontal: "center",
      }}
    >
      <Alert
        variant="filled"
        severity="error"
        action={<CloseAction onCloseSnackbar={onCloseSnackbar} />}
      >
        {errorMessage}
      </Alert>
    </Snackbar>
  );
}

export function CloseAction({onCloseSnackbar}: {onCloseSnackbar: () => void}) {
  return (
    <IconButton
      size="small"
      aria-label="close"
      color="inherit"
      onClick={onCloseSnackbar}
    >
      <CloseIcon fontSize="small" />
    </IconButton>
  );
}
