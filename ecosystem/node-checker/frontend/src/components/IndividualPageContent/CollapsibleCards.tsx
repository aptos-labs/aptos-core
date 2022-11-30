import React from "react";
import {Stack, Typography, useTheme, Box, Button} from "@mui/material";
import {grey} from "../../themes/colors/aptosColorPalette";

// return true if there is at least 1 card is not expanded
function getNotAllExpanded(expandedList: boolean[]): boolean {
  return expandedList.find((expanded) => expanded === false) !== undefined;
}

// return true if there is at least 1 card is expanded
function getNotAllCollapse(expandedList: boolean[]): boolean {
  return expandedList.find((expanded) => expanded === true) !== undefined;
}

type ExpandAllCollapseAllButtonsProps = {
  expandedList: boolean[];
  expandAll: () => void;
  collapseAll: () => void;
};

function ExpandAllCollapseAllButtons({
  expandedList,
  expandAll,
  collapseAll,
}: ExpandAllCollapseAllButtonsProps) {
  const theme = useTheme();

  const heavyTextColor = grey[450];
  const lightTextColor = theme.palette.mode === "dark" ? grey[500] : grey[400];

  const enableExpandAllButton = getNotAllExpanded(expandedList);
  const enableCollapseAllButton = getNotAllCollapse(expandedList);

  return (
    <Stack
      direction="row"
      justifyContent="flex-end"
      spacing={1}
      marginY={0.5}
      height={16}
    >
      <Button
        variant="text"
        disabled={!enableExpandAllButton}
        onClick={expandAll}
        sx={{
          fontSize: 12,
          fontWeight: 600,
          color: enableExpandAllButton ? heavyTextColor : lightTextColor,
          padding: 0,
          "&:hover": {
            background: "transparent",
          },
          "&:disabled": {
            color: enableExpandAllButton ? heavyTextColor : lightTextColor,
          },
        }}
      >
        Expand All
      </Button>
      <Typography
        variant="subtitle1"
        sx={{
          fontSize: 11,
          fontWeight: 600,
          color: heavyTextColor,
        }}
      >
        |
      </Typography>
      <Button
        variant="text"
        disabled={!enableCollapseAllButton}
        onClick={collapseAll}
        sx={{
          fontSize: 12,
          fontWeight: 600,
          color: enableCollapseAllButton ? heavyTextColor : lightTextColor,
          padding: 0,
          "&:hover": {
            background: "transparent",
          },
          "&:disabled": {
            color: enableCollapseAllButton ? heavyTextColor : lightTextColor,
          },
        }}
      >
        Collapse All
      </Button>
    </Stack>
  );
}

type CollapsibleCardsProps = {
  expandedList: boolean[];
  expandAll: () => void;
  collapseAll: () => void;
  children: React.ReactNode;
};

export default function CollapsibleCards({
  expandedList,
  expandAll,
  collapseAll,
  children,
}: CollapsibleCardsProps) {
  const hideButtons = expandedList.length <= 1;

  return (
    <Box>
      {!hideButtons && (
        <ExpandAllCollapseAllButtons
          expandedList={expandedList}
          expandAll={expandAll}
          collapseAll={collapseAll}
        />
      )}
      <Stack direction="column" spacing={1} marginTop={hideButtons ? 3 : 0}>
        {children}
      </Stack>
    </Box>
  );
}
