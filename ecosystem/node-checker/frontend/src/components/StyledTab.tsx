import * as React from "react";
import {Tab, TabProps, useTheme} from "@mui/material";
import {grey} from "../themes/colors/aptosColorPalette";

interface StyledTabProps extends TabProps {
  isFirst: boolean;
  isLast: boolean;
}

export default function StyledTab({
  isFirst,
  isLast,
  ...props
}: StyledTabProps): JSX.Element {
  const theme = useTheme();
  // TODO: unify colors for the new transaction page
  const backgroundColor = theme.palette.mode === "dark" ? grey[800] : grey[50];

  return (
    <Tab
      sx={{
        minHeight: 60,
        textTransform: "none",
        fontSize: {xs: "small", md: "medium"},
        paddingX: 2,
        color: grey[450],
        minWidth: {xs: 0, md: "200px"},
        "&.Mui-selected": {
          color: "inherit",
          backgroundColor: backgroundColor,
        },
        borderTopLeftRadius: isFirst ? "15px 15px" : "",
        borderBottomLeftRadius: isFirst ? "15px 15px" : "",
        borderTopRightRadius: isLast ? "15px 15px" : "",
        borderBottomRightRadius: isLast ? "15px 15px" : "",
        border: 1,
        borderColor: backgroundColor,
      }}
      iconPosition="start"
      disableRipple
      {...props}
    />
  );
}
