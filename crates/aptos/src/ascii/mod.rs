// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::CliResult;
use clap::Subcommand;
use indoc::indoc;

/// Aptos Ascii Art commands
///
/// These commands is used to celebrate Aptos in spectacular ascii
/// fashion!
#[derive(Debug, Subcommand)]
pub enum AsciiTool {
    /// Print the Aptos logo in ascii
    Logo,
}

impl AsciiTool {
    pub async fn execute(self) -> CliResult {
        match self {
            AsciiTool::Logo => self.logo(),
        }
    }

    pub fn logo(self) -> CliResult {
        Ok((indoc! {r"
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMWNNXKK0000KKXXNWMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMWNKkdl:;'............';:ldkKNWMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMWXOo:'.                        .':oOXWMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMWKd:.                                  .:dKWMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMXx:.                                        .:xXWMMMMMMMMMMMMMMM
MMMMMMMMMMMMMWKo'                                      ...     'oKWMMMMMMMMMMMMM
MMMMMMMMMMMMXo.                                      ,xXXO:.     .oKMMMMMMMMMMMM
MMMMMMMMMMW0:......................................;xNMMMMNk;......:OWMMMMMMMMMM
MMMMMMMMMMWXKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKKK0KNMMMMMMMMWXKKKKKKXWMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMW0xkXWMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMW0c.  'dXMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMN0OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOxc.      ,dOOOOOOOOOOOOOOOOOOOO0NMMMMM
MMMMNl.                                                                  .lNMMMM
MMMMO.                                     .                              .kMMMM
MMMNl                                  ..:k0k:.                            lNMMM
MMMK;                                .:OXWMMMWk;.                          ;KMMM
MMMN0kkkkkkkkkkkkkkkkkkkkkkkkkkkkxkkk0NMMMMMMMMN0kkkkkkkkkkkkkkkkkkkkkkkkkk0NMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMN00XWMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMNk;..'dNMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMWXXXXXXXXXXXXXXXXXXXXXXXKk:.     ,xKXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXNMMMM
MMMMk,.......................          ...................................,kMMMM
MMMMK,                                                                    ,KMMMM
MMMMWx.                    .cdo,                                         .xWMMMM
MMMMMNl                  .lKWMMXd'                                       lNMMMMM
MMMMMMNxlllllllllllllllldKWMMMMMMXxlllllllllllllllllllllllllllllllllllllxXMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMWXkxKWMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMXd'  .lKWMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMNOo,      .ok000000000000000000000000000000000000000KNMMMMMMMMMMMMM
MMMMMMMMMMMMMNd'                                               .,xNMMMMMMMMMMMMM
MMMMMMMMMMMMMMWXd,.                                           ,dXWMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMNOl'.                                    .'lONMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMN0d:.                              .:oONMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMWX0dl;'.                  .';ldOXWMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMWNK0kxdollccccllodxk0KNWMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
                        ___  ______ _____ _____ _____
                       / _ \ | ___ \_   _|  _  /  ___|
                      / /_\ \| |_/ / | | | | | \ `--.
                      |  _  ||  __/  | | | | | |`--. \
                      | | | || |     | | \ \_/ /\__/ /
                      \_| |_/\_|     \_/  \___/\____/

        "})
        .to_string())
    }
}
