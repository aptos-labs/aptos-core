import React from "react";
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined";
import {Box, Link, Typography, useTheme} from "@mui/material";
import {grey} from "../../themes/colors/aptosColorPalette";
import {Stack} from "@mui/system";
import StyledTooltip from "../StyledTooltip";

function TooltipBox({children}: {children?: React.ReactNode}) {
  return <Box sx={{width: 25}}>{children}</Box>;
}

type LearnMoreTooltipProps = {
  text: string;
  link?: string;
  linkToText?: boolean;
};

export function LearnMoreTooltip({
  text,
  link,
  linkToText,
}: LearnMoreTooltipProps): JSX.Element {
  // TODO: unify colors for the new transaction page
  const theme = useTheme();
  const color = theme.palette.mode === "dark" ? grey[600] : grey[200];

  return (
    <TooltipBox>
      <StyledTooltip
        title={
          <Stack alignItems="flex-end">
            {linkToText ? (
              <Link alignSelf="flex-end" href={link} color="inherit">
                {text}
              </Link>
            ) : (
              <>
                <Typography variant="inherit">{text}</Typography>
                {link && (
                  <Link alignSelf="flex-end" href={link} color="inherit">
                    Learn More
                  </Link>
                )}
              </>
            )}
          </Stack>
        }
        arrow
      >
        <InfoOutlinedIcon fontSize="inherit" htmlColor={color} />
      </StyledTooltip>
    </TooltipBox>
  );
}

export function LearnMoreTooltipPlaceholder(): JSX.Element {
  return <TooltipBox></TooltipBox>;
}
