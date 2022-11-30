import React, {useState} from "react";
import {Box, Button, Tooltip, Typography, useTheme} from "@mui/material";
import {grey} from "../themes/colors/aptosColorPalette";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import useMediaQuery from "@mui/material/useMediaQuery";
import {truncateAddress, truncateAddressMiddle} from "../pages/utils";

const TOOLTIP_TIME = 2000; // 2s

interface HashButtonCopyableProps {
  hash: string;
}

export default function HashButtonCopyable({hash}: HashButtonCopyableProps) {
  const theme = useTheme();
  const isOnMobile = !useMediaQuery(theme.breakpoints.up("md"));
  const isOnSmallerScreen = !useMediaQuery(theme.breakpoints.up("lg"));

  const [tooltipOpen, setTooltipOpen] = useState<boolean>(false);

  const copyAddress = async (event: React.MouseEvent<HTMLButtonElement>) => {
    await navigator.clipboard.writeText(hash);

    setTooltipOpen(true);

    setTimeout(() => {
      setTooltipOpen(false);
    }, TOOLTIP_TIME);
  };

  const buttonComponent = (
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
      }}
      size="small"
      onClick={copyAddress}
      variant="contained"
      endIcon={
        <ContentCopyIcon
          fontSize="small"
          sx={{opacity: "0.75", my: 0.7, mr: 1}}
        />
      }
    >
      <Typography variant="body2">
        {isOnMobile
          ? truncateAddress(hash)
          : isOnSmallerScreen
          ? truncateAddressMiddle(hash)
          : hash}
      </Typography>
    </Button>
  );

  return (
    <Box>
      <Tooltip
        title="Copied"
        placement="bottom-end"
        open={tooltipOpen}
        disableFocusListener
        disableHoverListener
        disableTouchListener
      >
        {buttonComponent}
      </Tooltip>
    </Box>
  );
}
