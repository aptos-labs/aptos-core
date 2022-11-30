import React from "react";
import {Typography, TableCell, useTheme, Stack} from "@mui/material";
import {SxProps} from "@mui/system";
import {Theme} from "@mui/material/styles";

interface GeneralTableHeaderCellProps {
  header: React.ReactNode;
  textAlignRight?: boolean;
  sx?: SxProps<Theme>;
  tooltip?: React.ReactNode;
}

export default function GeneralTableHeaderCell({
  header,
  textAlignRight,
  sx = [],
  tooltip,
}: GeneralTableHeaderCellProps) {
  const theme = useTheme();
  const tableCellBackgroundColor = "transparent";
  const tableCellTextColor = theme.palette.text.secondary;

  return (
    <TableCell
      sx={[
        {
          textAlign: textAlignRight ? "right" : "left",
          background: `${tableCellBackgroundColor}`,
          color: `${tableCellTextColor}`,
        },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
    >
      {tooltip ? (
        <Stack
          direction="row"
          spacing={1}
          justifyContent={textAlignRight ? "flex-end" : "flex-start"}
        >
          <Typography variant="subtitle1">{header}</Typography>
          {tooltip}
        </Stack>
      ) : (
        <Typography variant="subtitle1">{header}</Typography>
      )}
    </TableCell>
  );
}
