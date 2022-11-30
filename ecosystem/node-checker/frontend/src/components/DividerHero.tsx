import * as React from "react";
import Box from "@mui/material/Box";
import {useTheme} from "@mui/material";

export default function DividerHero() {
  const theme = useTheme();

  return (
    <Box
      sx={{
        width: 1,
        mt: 2,
        mb: 4,
        display: "flex",
        textAlign: "center",
        color: `${theme.palette.lineShade.main}`,
      }}
    >
      <Box
        sx={{
          borderColor: "currentColor",
          borderWidth: "1px 0 1px 0",
          borderStyle: "solid",
          flex: "1 1 0%",
          marginTop: "14px",
        }}
      />
      <svg width="52" height="34">
        <path
          fill="none"
          stroke="currentColor"
          d="M0,14.5h0.6h17.6c7.1,0,14-2.8,19-8c3.6-3.7,8.3-6,13.4-6H52"
        />
        <path
          fill="none"
          stroke="currentColor"
          d="M52,19.5h-0.6H33.8c-7.1,0-14,2.8-19,8c-3.6,3.7-8.3,6-13.4,6H0"
        />
      </svg>
      <Box
        sx={{
          borderColor: "currentColor",
          borderWidth: "1px 0 1px 0",
          borderStyle: "solid",
          flex: "1 1 0%",
          marginBottom: "14px",
        }}
      />
    </Box>
  );
}
