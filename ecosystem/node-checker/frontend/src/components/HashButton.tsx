import React, {useState} from "react";
import {
  Box,
  BoxProps,
  Button,
  Typography,
  Popover,
  IconButton,
  useTheme,
  Link,
} from "@mui/material";
import * as RRD from "react-router-dom";
import {grey} from "../themes/colors/aptosColorPalette";
import ChevronRightRoundedIcon from "@mui/icons-material/ChevronRightRounded";
import ChevronLeftRoundedIcon from "@mui/icons-material/ChevronLeftRounded";
import {truncateAddress} from "../pages/utils";
import {assertNever} from "../utils";

export enum HashType {
  ACCOUNT = "account",
  TRANSACTION = "transaction",
  OTHERS = "others",
}

function getHashLinkStr(hash: string, type: HashType): string {
  switch (type) {
    case HashType.ACCOUNT:
      return `/account/${hash}`;
    case HashType.TRANSACTION:
      return `/txn/${hash}`;
    case HashType.OTHERS:
      return "";
    default:
      return assertNever(type);
  }
}

function HashLink(hash: string, type: HashType): JSX.Element {
  switch (type) {
    case HashType.ACCOUNT:
    case HashType.TRANSACTION:
      return (
        <Link
          component={RRD.Link}
          to={getHashLinkStr(hash, type)}
          color="inherit"
        >
          {hash}
        </Link>
      );
    case HashType.OTHERS:
      return <>{hash}</>;
    default:
      return assertNever(type);
  }
}

interface HashButtonProps extends BoxProps {
  hash: string;
  type: HashType;
}

export default function HashButton({hash, type, ...props}: HashButtonProps) {
  const theme = useTheme();
  const [anchorEl, setAnchorEl] = useState<HTMLButtonElement | null>(null);

  const hashExpand = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    setAnchorEl(event.currentTarget);
  };

  const hashCollapse = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    setAnchorEl(null);
  };

  const open = Boolean(anchorEl);
  const id = open ? "simple-popover" : undefined;

  return (
    <Box {...props}>
      <Button
        sx={{
          textTransform: "none",
          backgroundColor: `${
            theme.palette.mode === "dark" ? grey[600] : grey[200]
          }`,
          display: "flex",
          borderRadius: 1,
          color: "inherit",
          padding: "0.15rem 0.5rem 0.15rem 1rem",
          "&:hover": {
            backgroundColor: `${
              theme.palette.mode === "dark" ? grey[500] : grey[300]
            }`,
          },
          minWidth: 141,
        }}
        aria-describedby={id}
        onClick={hashExpand}
        variant="contained"
        endIcon={<ChevronRightRoundedIcon sx={{opacity: "0.75", m: 0}} />}
      >
        {truncateAddress(hash)}
      </Button>

      <Popover
        onClick={(e) => {
          e.stopPropagation();
        }}
        sx={{
          overflow: "scroll",
          ".MuiPaper-root": {boxShadow: "none"},
          "&.MuiModal-root .MuiBackdrop-root": {
            transition: "none!important",
            backgroundColor: `${
              theme.palette.mode === "dark"
                ? "rgba(18,22,21,0.5)"
                : "rgba(255,255,255,0.5)"
            }`,
          },
        }}
        id={id}
        open={open}
        anchorEl={anchorEl}
        onClose={hashCollapse}
        anchorOrigin={{
          vertical: "center",
          horizontal: "left",
        }}
        transformOrigin={{
          vertical: "center",
          horizontal: "left",
        }}
      >
        <Typography
          sx={{
            display: "flex",
            alignItems: "center",
            backgroundColor: `${
              theme.palette.mode === "dark" ? grey[600] : grey[200]
            }`,
            px: 2,
            py: "0.15rem",
            fontSize: "14px",
            overflow: "scroll",
            scrollbarWidth: "none",
            "&::-webkit-scrollbar": {
              display: "none",
            },
          }}
        >
          {HashLink(hash, type)}
          <IconButton
            aria-label="collapse hash"
            onClick={hashCollapse}
            sx={{
              ml: 1,
              mr: 0,
              p: 0,
              "&.MuiButtonBase-root:hover": {
                bgcolor: "transparent",
              },
            }}
          >
            <ChevronLeftRoundedIcon sx={{opacity: "0.5"}} />
          </IconButton>
        </Typography>
      </Popover>
    </Box>
  );
}
