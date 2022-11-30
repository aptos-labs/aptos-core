import * as React from "react";
import Typography from "@mui/material/Typography";

interface ChildrenProps {
  children?: React.ReactNode;
}

export default function HeadingSub(props: ChildrenProps) {
  return (
    <Typography
      color="secondary"
      variant="subtitle2"
      component="span"
      sx={{mb: 1}}
    >
      {props.children}
    </Typography>
  );
}
