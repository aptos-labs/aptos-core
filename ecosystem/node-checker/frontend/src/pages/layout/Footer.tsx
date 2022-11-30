import React from "react";
import Typography from "@mui/material/Typography";
import Link from "@mui/material/Link";
import Container from "@mui/material/Container";
import Grid from "@mui/material/Grid";

import {ReactComponent as GithubLogo} from "../../assets/github.svg";
import {ReactComponent as DiscordLogo} from "../../assets/discord.svg";
import {ReactComponent as TwitterLogo} from "../../assets/twitter.svg";
import {ReactComponent as MediumLogo} from "../../assets/medium.svg";
import {ReactComponent as LinkedInLogo} from "../../assets/linkedin.svg";
import Box from "@mui/material/Box";
import {useTheme} from "@mui/material";
import {grey} from "../../themes/colors/aptosColorPalette";
import SvgIcon, {SvgIconProps} from "@mui/material/SvgIcon";

import {ReactComponent as LogoFull} from "../../assets/svg/aptos_logo_full.svg";

const socialLinks = [
  {title: "Git", url: "https://github.com/aptos-labs", icon: GithubLogo},
  {title: "Discord", url: "https://discord.gg/zTDYBEud7U", icon: DiscordLogo},
  {title: "Twitter", url: "https://twitter.com/aptoslabs/", icon: TwitterLogo},
  {title: "Medium", url: "https://aptoslabs.medium.com/", icon: MediumLogo},
  {
    title: "LinkedIn",
    url: "https://www.linkedin.com/company/aptoslabs/",
    icon: LinkedInLogo,
  },
];

export default function Footer() {
  const theme = useTheme();

  return (
    <Box
      sx={{
        background: theme.palette.mode === "dark" ? grey[900] : "white",
        color: theme.palette.mode === "dark" ? grey[100] : "rgba(18,22,21,1)",
        mt: 8,
      }}
    >
      <Container maxWidth="xl" sx={{paddingTop: "2rem", paddingBottom: "2rem"}}>
        <Grid
          container
          spacing={{xs: 4, md: 1}}
          alignContent="center"
          alignItems="center"
          direction={{xs: "column", md: "row"}}
        >
          <Grid item xs="auto" sx={{mr: 2}} container justifyContent="start">
            <Link
              color="inherit"
              href="https://aptoslabs.com/"
              target="_blank"
              sx={{width: "8rem"}}
            >
              <LogoFull />
            </Link>
          </Grid>
          <Grid item xs="auto" container justifyContent="start">
            <Typography
              sx={{textAlign: {xs: "center", md: "left"}}}
              fontSize="0.8rem"
            >
              © {new Date().getFullYear()}{" "}
              <Box component="span" sx={{whiteSpace: "nowrap"}}>
                Matonee Inc. (dba Aptos Labs)
              </Box>
              <br />
              <Link
                color="inherit"
                href="mailto:info@aptoslabs.com"
                target="_blank"
              >
                info@aptoslabs.com
              </Link>
              <Box component="span" sx={{px: 1, display: "inline-block"}}>
                or
              </Box>
              <Link
                color="inherit"
                href="mailto:press@aptoslabs.com"
                target="_blank"
              >
                press@aptoslabs.com
              </Link>
            </Typography>
          </Grid>
          <Grid
            item
            xs="auto"
            sx={{marginLeft: {xs: "0", md: "auto"}}}
            container
            justifyContent="end"
          >
            <Grid
              container
              justifyContent={{xs: "center", md: "end"}}
              spacing={3}
              direction="row"
            >
              {socialLinks.map((link) => (
                <Grid item key={link.title}>
                  <Link
                    color="inherit"
                    href={link.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    title={link.title}
                    width="26px"
                    sx={{display: "block"}}
                  >
                    <SvgIcon component={link.icon} inheritViewBox />
                  </Link>
                </Grid>
              ))}
            </Grid>
          </Grid>
        </Grid>
      </Container>
    </Box>
  );
}
