import React, {useState} from "react";
import {parseTimestamp, timestampDisplay} from "../../../pages/utils";
import EmptyValue from "./EmptyValue";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import {IconButton, Stack, Typography, useTheme} from "@mui/material";
import {grey} from "../../../themes/colors/aptosColorPalette";
import StyledTooltip from "../../StyledTooltip";

const TOOLTIP_TIME = 2000; // 2s

type TimestampValueProps = {
  timestamp: string;
};

export default function TimestampValue({timestamp}: TimestampValueProps) {
  const [tooltipOpen, setTooltipOpen] = useState<boolean>(false);

  // TODO: unify colors for the new transaction page
  const theme = useTheme();
  const color = theme.palette.mode === "dark" ? grey[600] : grey[200];

  if (timestamp === "0") {
    return <EmptyValue />;
  }

  const moment = parseTimestamp(timestamp);
  const timestamp_display = timestampDisplay(moment);

  const copyTimestamp = async (event: React.MouseEvent<HTMLButtonElement>) => {
    await navigator.clipboard.writeText(timestamp);

    setTooltipOpen(true);

    setTimeout(() => {
      setTooltipOpen(false);
    }, TOOLTIP_TIME);
  };

  return (
    <Stack direction="row" spacing={1} alignItems="center">
      <Typography fontSize="inherit">
        {timestamp_display.local_formatted}
      </Typography>
      <StyledTooltip
        title="Timestamp copied"
        placement="right"
        open={tooltipOpen}
        disableFocusListener
        disableHoverListener
        disableTouchListener
      >
        <IconButton onClick={copyTimestamp} sx={{padding: 0.6}}>
          <ContentCopyIcon sx={{color: color, fontSize: 15}} />
        </IconButton>
      </StyledTooltip>
    </Stack>
  );
}
