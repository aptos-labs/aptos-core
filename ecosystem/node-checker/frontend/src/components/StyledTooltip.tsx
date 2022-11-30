import {Box, Tooltip, TooltipProps} from "@mui/material";
import React from "react";

interface StyledTooltipProps extends TooltipProps {
  title: NonNullable<React.ReactNode>;
}

export default function StyledTooltip({
  children,
  title,
  ...props
}: StyledTooltipProps) {
  return (
    <Tooltip
      title={<Box sx={{fontSize: 13, fontFamily: "sans-serif"}}>{title}</Box>}
      {...props}
    >
      {children}
    </Tooltip>
  );
}
