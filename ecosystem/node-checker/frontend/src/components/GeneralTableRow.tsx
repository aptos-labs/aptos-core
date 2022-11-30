import React from "react";
import {TableRow, useTheme} from "@mui/material";
import {grey} from "../themes/colors/aptosColorPalette";

interface TableRowProps {
  children: React.ReactNode;
  onClick?: () => void;
}

export default function GeneralTableRow({children, onClick}: TableRowProps) {
  const theme = useTheme();
  const clickDisabled = !onClick;

  return (
    <TableRow
      onClick={onClick}
      sx={{
        cursor: clickDisabled ? undefined : "pointer",
        userSelect: "none",
        backgroundColor: `${
          theme.palette.mode === "dark" ? grey[800] : grey[50]
        }`,
        "&:hover:not(:active)": clickDisabled
          ? undefined
          : {
              filter: `${
                theme.palette.mode === "dark"
                  ? "brightness(0.9)"
                  : "brightness(0.99)"
              }`,
            },
        "&:active": clickDisabled
          ? undefined
          : {
              background: theme.palette.neutralShade.main,
              transform: "translate(0,0.1rem)",
            },
      }}
    >
      {children}
    </TableRow>
  );
}
