import React from "react";
import {Tabs, TabsProps} from "@mui/material";

interface StyledTabsProps extends TabsProps {
  children: React.ReactNode;
}

export default function StyledTabs({
  children,
  ...props
}: StyledTabsProps): JSX.Element {
  return (
    <Tabs
      variant="scrollable"
      scrollButtons="auto"
      sx={{
        "& .MuiTabs-indicator": {
          display: "none",
        },
      }}
      {...props}
    >
      {children}
    </Tabs>
  );
}
