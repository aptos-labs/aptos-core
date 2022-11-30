import {Box, Typography} from "@mui/material";
import React from "react";
import StartRoundedIcon from "@mui/icons-material/StartRounded";
import OutlinedFlagIcon from "@mui/icons-material/OutlinedFlag";
import SubtitlesOutlinedIcon from "@mui/icons-material/SubtitlesOutlined";
import MultipleStopRoundedIcon from "@mui/icons-material/MultipleStopRounded";
import UpdateRoundedIcon from "@mui/icons-material/UpdateRounded";
import {grey} from "../themes/colors/aptosColorPalette";

type Color = "inherit" | "primary" | undefined;

function getTypeLabel(type: string): string {
  switch (type) {
    case "block_metadata_transaction":
      return "BlockMetadata";
    case "genesis_transaction":
      return "GenesisTransaction";
    case "user_transaction":
      return "UserTransaction";
    case "pending_transaction":
      return "PendingTransaction";
    case "state_checkpoint_transaction":
      return "StateCheckpoint";
    default:
      throw `Unknown TransactionType:${type}`;
  }
}

function getTypeIcon(type: string, color?: Color) {
  switch (type) {
    case "block_metadata_transaction":
      return <SubtitlesOutlinedIcon fontSize="small" color={color} />;
    case "genesis_transaction":
      return <StartRoundedIcon fontSize="small" color={color} />;
    case "user_transaction":
      return <MultipleStopRoundedIcon fontSize="small" color={color} />;
    case "pending_transaction":
      return <UpdateRoundedIcon fontSize="small" color={color} />;
    case "state_checkpoint_transaction":
      return <OutlinedFlagIcon fontSize="small" color={color} />;
    default:
      throw `Unknown TransactionType:${type}`;
  }
}

type TransactionTypeProps = {
  type: string;
};

export function TransactionType({type}: TransactionTypeProps) {
  return (
    <Box sx={{display: "flex", alignItems: "center", gap: 1, color: grey[450]}}>
      {getTypeIcon(type, "inherit")}
      <Typography variant="body2">{getTypeLabel(type)}</Typography>
    </Box>
  );
}

export function TableTransactionType({type}: TransactionTypeProps) {
  return (
    <Box sx={{display: "flex", alignItems: "center", gap: 1.5}}>
      {getTypeIcon(type, "primary")}
      <Typography fontSize="inherit">{getTypeLabel(type)}</Typography>
    </Box>
  );
}
