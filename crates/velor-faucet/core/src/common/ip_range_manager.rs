// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use ipnet::{Ipv4Net, Ipv6Net};
use iprange::IpRange;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufRead, net::IpAddr, path::PathBuf};

/// Generic list checker, for either an allowlist or blocklist.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IpRangeManagerConfig {
    /// Path to a file containing one IP range per line, where an IP range is
    /// something like 32.143.133.32/24.
    pub file: PathBuf,
}

pub struct IpRangeManager {
    pub ipv4_list: IpRange<Ipv4Net>,
    pub ipv6_list: IpRange<Ipv6Net>,
}

impl IpRangeManager {
    pub fn new(config: IpRangeManagerConfig) -> Result<Self> {
        let file = File::open(&config.file)
            .with_context(|| format!("Failed to open {}", config.file.to_string_lossy()))?;

        let mut ipv4_list = IpRange::<Ipv4Net>::new();
        let mut ipv6_list = IpRange::<Ipv6Net>::new();
        for line in std::io::BufReader::new(file).lines() {
            let line = line?;
            if line.starts_with('#') || line.starts_with("//") || line.is_empty() {
                continue;
            }
            match line.parse::<Ipv4Net>() {
                Ok(ipv4_net) => {
                    ipv4_list.add(ipv4_net);
                },
                Err(_) => match line.parse::<Ipv6Net>() {
                    Ok(ipv6_net) => {
                        ipv6_list.add(ipv6_net);
                    },
                    Err(_) => {
                        bail!("Failed to parse line as IPv4 or IPv6 range: {}", line);
                    },
                },
            }
        }
        Ok(Self {
            ipv4_list,
            ipv6_list,
        })
    }

    pub fn contains_ip(&self, ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => self.ipv4_list.contains(ipv4),
            IpAddr::V6(ipv6) => self.ipv6_list.contains(ipv6),
        }
    }
}
