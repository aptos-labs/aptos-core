import React from "react";
import {Box, Stack, Typography} from "@mui/material";
import CheckCircleIcon from "@mui/icons-material/CheckCircle";
import ErrorOutlinedIcon from "@mui/icons-material/ErrorOutlined";
import CheckCircleOutlinedIcon from "@mui/icons-material/CheckCircleOutlined";
import ErrorOutlineOutlinedIcon from "@mui/icons-material/ErrorOutlineOutlined";

// TODO: unify the colors
const SUCCESS_COLOR = "#00BFA5";
const SUCCESS_BACKGROUND_COLOR = "rgba(0,191,165,0.1)";
const ERROR_COLOR = "#F97373";
const ERROR_BACKGROUND_COLOR = "rgba(249,115,115,0.1)";

type TransactionStatusProps = {
  success: boolean;
};

export function TransactionStatus({success}: TransactionStatusProps) {
  return success ? (
    <Stack
      direction="row"
      spacing={1}
      padding={0.7}
      alignItems="center"
      justifyContent="center"
      sx={{
        backgroundColor: SUCCESS_BACKGROUND_COLOR,
        width: 100,
      }}
      borderRadius={1}
    >
      <CheckCircleIcon
        fontSize="small"
        titleAccess="Executed successfully"
        sx={{color: SUCCESS_COLOR}}
      />
      <Typography variant="body2" sx={{color: SUCCESS_COLOR}}>
        Success
      </Typography>
    </Stack>
  ) : (
    <Stack
      direction="row"
      spacing={1}
      padding={0.7}
      alignItems="center"
      justifyContent="center"
      sx={{
        backgroundColor: ERROR_BACKGROUND_COLOR,
        width: 80,
      }}
      borderRadius={1}
    >
      <ErrorOutlinedIcon
        fontSize="small"
        titleAccess="Failed to Execute"
        sx={{color: ERROR_COLOR}}
      />
      <Typography variant="body2" sx={{color: ERROR_COLOR}}>
        Fail
      </Typography>
    </Stack>
  );
}

export function TableTransactionStatus({success}: TransactionStatusProps) {
  return (
    <Box
      sx={{
        display: "flex",
        alignItems: "center",
        marginLeft: 2,
      }}
    >
      {success ? (
        <CheckCircleOutlinedIcon
          fontSize="small"
          color="success"
          titleAccess="Executed successfully"
        />
      ) : (
        <ErrorOutlineOutlinedIcon
          fontSize="small"
          color="error"
          titleAccess="Failed to Execute"
        />
      )}
    </Box>
  );
}

export function OldTransactionStatus({success}: TransactionStatusProps) {
  return success ? (
    <Box sx={{display: "flex", alignItems: "center", gap: 1}}>
      <CheckCircleOutlinedIcon
        fontSize="small"
        color="success"
        titleAccess="Executed successfully"
      />
      Success
    </Box>
  ) : (
    <Box sx={{display: "flex", alignItems: "center", gap: 1}}>
      <ErrorOutlineOutlinedIcon
        fontSize="small"
        color="error"
        titleAccess="Failed to Execute"
      />
      Fail
    </Box>
  );
}
