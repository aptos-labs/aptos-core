import React, {useState} from "react";

import Button from "@mui/material/Button";
import Menu from "@mui/material/Menu";
import MenuItem from "@mui/material/MenuItem";
import {ReactComponent as HamburgerIcon} from "../assets/svg/icon_hamburger.svg";
import {ReactComponent as CloseIcon} from "../assets/svg/icon_close.svg";
import {grey} from "../themes/colors/aptosColorPalette";
import Box from "@mui/material/Box";
import {useTheme} from "@mui/material";
import {useNavigate} from "react-router-dom";
import KeyboardArrowDownIcon from "@mui/icons-material/KeyboardArrowDown";
import KeyboardArrowUpIcon from "@mui/icons-material/KeyboardArrowUp";
import {useGetInGtmMode} from "../api/hooks/useGetInDevMode";

export default function NavMobile() {
  const inGtm = useGetInGtmMode();
  const [menuAnchorEl, setMenuAnchorEl] = useState<null | HTMLElement>(null);
  const [governanceMenuOpen, setGovernanceMenuOpen] = useState<boolean>(false);
  const theme = useTheme();
  const navigate = useNavigate();

  const menuOpen = Boolean(menuAnchorEl);

  const handleGovernanceClick = (event: React.MouseEvent<HTMLElement>) => {
    setGovernanceMenuOpen(!governanceMenuOpen);
  };

  const handleIconClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setMenuAnchorEl(event.currentTarget);
  };
  const handleMenuClose = () => {
    setMenuAnchorEl(null);
    setGovernanceMenuOpen(false);
  };

  const handleCloseAndNavigate = (to: string) => {
    setMenuAnchorEl(null);
    setGovernanceMenuOpen(false);
    navigate(to);
  };

  return (
    <Box sx={{display: {xs: "block", md: "none"}}}>
      <Button
        id="nav-mobile-button"
        aria-controls={menuOpen ? "nav-mobile-menu" : undefined}
        aria-haspopup="true"
        aria-expanded={menuOpen ? "true" : undefined}
        onClick={handleIconClick}
        sx={{
          minWidth: "0",
          width: "1.5rem",
          padding: "0",
          ml: 2,
          color: "inherit",
          "&:hover": {
            background: "transparent",
            color: `${theme.palette.mode === "dark" ? grey[100] : grey[400]}`,
          },
          "&[aria-expanded=true]": {opacity: "0.7"},
        }}
      >
        {menuOpen ? <CloseIcon /> : <HamburgerIcon />}
      </Button>
      {inGtm ? (
        <Menu
          anchorEl={menuAnchorEl}
          open={menuOpen}
          onClose={handleMenuClose}
          MenuListProps={{
            "aria-labelledby": "nav-mobile-button",
            sx: {
              minWidth: 240,
              padding: "1rem",
            },
          }}
          sx={{
            marginTop: "1rem",
            boxShadow: 0,
            minWidth: "400px",
            maxWidth: "none",
          }}
        >
          <MenuItem onClick={() => handleCloseAndNavigate("/node_checker")}>
            Node Checker
          </MenuItem>
        </Menu>
      ) : (
        <Menu
          anchorEl={menuAnchorEl}
          open={menuOpen}
          onClose={handleMenuClose}
          MenuListProps={{
            "aria-labelledby": "nav-mobile-button",
            sx: {
              minWidth: 240,
              padding: "1rem",
            },
          }}
          sx={{
            marginTop: "1rem",
            boxShadow: 0,
            minWidth: "400px",
            maxWidth: "none",
          }}
        >
          <MenuItem onClick={() => handleCloseAndNavigate("/transactions")}>
            Transactions
          </MenuItem>
          <MenuItem
            onClick={handleGovernanceClick}
            sx={{display: "flex", justifyContent: "space-between"}}
          >
            Governance{" "}
            {governanceMenuOpen ? (
              <KeyboardArrowUpIcon />
            ) : (
              <KeyboardArrowDownIcon />
            )}
          </MenuItem>
          {governanceMenuOpen && (
            <Box
              sx={{
                paddingLeft: "1.5rem",
              }}
              aria-controls={governanceMenuOpen ? "nav-mobile-menu" : undefined}
              aria-haspopup="true"
              aria-expanded={governanceMenuOpen ? "true" : undefined}
            >
              <MenuItem onClick={() => handleCloseAndNavigate("/proposals")}>
                Proposals
              </MenuItem>
              <MenuItem
                onClick={() => handleCloseAndNavigate("/proposals/staking")}
              >
                Staking
              </MenuItem>
            </Box>
          )}
        </Menu>
      )}
    </Box>
  );
}
