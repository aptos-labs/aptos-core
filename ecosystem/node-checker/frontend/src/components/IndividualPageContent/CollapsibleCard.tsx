import React from "react";
import {Stack, Typography, useTheme, Box, Grid} from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import ExpandLessIcon from "@mui/icons-material/ExpandLess";
import {grey} from "../../themes/colors/aptosColorPalette";

type CollapsibleCardProps = {
  titleKey: string;
  titleValue: string;
  children: React.ReactNode;
  expanded: boolean;
  toggleExpanded: () => void;
};

export default function CollapsibleCard({
  expanded,
  titleKey,
  titleValue,
  children,
  toggleExpanded,
  ...props
}: CollapsibleCardProps) {
  const theme = useTheme();
  // TODO: unify colors for the new transaction page
  const titleBackgroundColor =
    theme.palette.mode === "dark" ? grey[700] : grey[100];
  const contentBackgroundColor =
    theme.palette.mode === "dark" ? grey[800] : grey[50];

  return (
    <Box {...props}>
      <Box
        paddingX={4}
        paddingY={2}
        sx={{
          color: grey[450],
          backgroundColor: titleBackgroundColor,
          borderRadius: expanded ? "10px 10px 0px 0px" : "10px 10px 10px 10px",
        }}
        onClick={toggleExpanded}
      >
        <Grid
          container
          direction={{xs: "column", md: "row"}}
          rowSpacing={1}
          columnSpacing={4}
        >
          <Grid item md={3}>
            <Typography variant="body2" color={grey[450]}>
              {titleKey}
            </Typography>
          </Grid>
          <Grid
            item
            md={9}
            width={{xs: 1, md: 0.75}}
            sx={{
              fontSize: 13.5,
              overflow: "auto",
            }}
          >
            <Stack direction="row" justifyContent="space-between">
              {titleValue}
              {expanded ? (
                <ExpandLessIcon fontSize="small" />
              ) : (
                <ExpandMoreIcon fontSize="small" />
              )}
            </Stack>
          </Grid>
        </Grid>
      </Box>
      {expanded && (
        <Box
          padding={4}
          sx={{
            backgroundColor: contentBackgroundColor,
            borderRadius: "0px 0px 10px 10px",
          }}
        >
          <Stack direction="column" spacing={2}>
            {children}
          </Stack>
        </Box>
      )}
    </Box>
  );
}
