// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Warning: This could be attacked and cause the server to OOM because we
//! don't throw out captchas info if it has been sitting there for too long /
//! the map grows too large.

use super::{CheckerData, CheckerTrait};
use crate::endpoints::{
    VelorTapError, VelorTapErrorCode, RejectionReason, RejectionReasonCode, CAPTCHA_KEY,
    CAPTCHA_VALUE,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use captcha::{
    filters::{Dots, Grid, Noise, Wave},
    Captcha,
};
use futures::lock::Mutex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TapCaptchaCheckerConfig {}

pub struct TapCaptchaChecker {
    #[allow(dead_code)]
    config: TapCaptchaCheckerConfig,

    /// Reference to the one captcha manager. This must be pased in because we
    /// need to be able to use it from the challenge endpoint too.
    captcha_manager: Arc<Mutex<CaptchaManager>>,
}

impl TapCaptchaChecker {
    pub fn new(
        config: TapCaptchaCheckerConfig,
        captcha_manager: Arc<Mutex<CaptchaManager>>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            captcha_manager,
        })
    }
}

#[async_trait]
impl CheckerTrait for TapCaptchaChecker {
    async fn check(
        &self,
        data: CheckerData,
        _dry_run: bool,
    ) -> Result<Vec<RejectionReason>, VelorTapError> {
        let captcha_key = match data.headers.get(CAPTCHA_KEY) {
            Some(header_value) => match header_value
                .to_str()
                .map_err(|e| {
                    VelorTapError::new_with_error_code(e, VelorTapErrorCode::InvalidRequest)
                })?
                .parse::<u32>()
            {
                Ok(value) => value,
                Err(e) => {
                    return Ok(vec![RejectionReason::new(
                        format!("Captcha value not a number: {:#}", e),
                        RejectionReasonCode::CaptchaInvalid,
                    )])
                },
            },
            None => {
                return Ok(vec![RejectionReason::new(
                    format!("Captcha header {} not found", CAPTCHA_KEY),
                    RejectionReasonCode::CaptchaInvalid,
                )])
            },
        };

        let captcha_value = match data.headers.get(CAPTCHA_VALUE) {
            Some(header_value) => header_value.to_str().map_err(|e| {
                VelorTapError::new_with_error_code(e, VelorTapErrorCode::InvalidRequest)
            })?,
            None => {
                return Ok(vec![RejectionReason::new(
                    format!("Captcha header {} not found", CAPTCHA_VALUE),
                    RejectionReasonCode::CaptchaInvalid,
                )])
            },
        };

        let captcha_correct = match self
            .captcha_manager
            .lock()
            .await
            .check_challenge(captcha_key, captcha_value)
        {
            Ok(correct) => correct,
            Err(e) => {
                return Ok(vec![RejectionReason::new(
                    format!("Captcha key unknown: {}", e),
                    RejectionReasonCode::CaptchaInvalid,
                )])
            },
        };

        if !captcha_correct {
            return Ok(vec![RejectionReason::new(
                format!("Captcha value {} incorrect", captcha_value),
                RejectionReasonCode::CaptchaInvalid,
            )]);
        }

        Ok(vec![])
    }

    fn cost(&self) -> u8 {
        3
    }
}

/// CaptchaManager is responsible for creating captcha challenges and later
/// checking them. We do this in memory for now (meaning clients should use
/// cookies to benefit from cookie based sticky routing), but we could make
/// a trait and implement a storage backed version down the line.
#[derive(Debug, Default)]
pub struct CaptchaManager {
    /// When a challenge is created, we return to the client the captcha itself
    /// and a random key they must make the second request with. This is a map
    /// from that random key to the value of the captcha.
    challenges: HashMap<u32, String>,
}

impl CaptchaManager {
    pub fn new() -> Self {
        Self {
            challenges: HashMap::new(),
        }
    }

    /// Create a new captcha challenge. Returns the captcha and the key
    /// the client must include when submitting the results of the captcha.
    pub fn create_challenge(&mut self) -> Result<(u32, Vec<u8>)> {
        // Generate a random key.
        let key = rand::thread_rng().gen_range(0, u32::MAX - 1);

        // Generate a captcha.
        let (name, image) = Captcha::new()
            .add_chars(5)
            .apply_filter(Noise::new(0.4))
            .apply_filter(Wave::new(4.0, 6.0).vertical())
            .apply_filter(Wave::new(3.0, 2.0).horizontal())
            .apply_filter(Grid::new(10, 6))
            .apply_filter(Dots::new(8))
            .as_tuple()
            .context("Failed to generate captcha")?;

        // Store the captcha information.
        self.challenges.insert(key, name);

        // Return (key, <captcha as base64>).
        Ok((key, image))
    }

    /// Check a captcha challenge. Returns true if the captcha is correct.
    pub fn check_challenge(&mut self, key: u32, value: &str) -> Result<bool> {
        match self.challenges.get(&key) {
            Some(captcha) => {
                if captcha == value {
                    self.challenges.remove(&key);
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            None => bail!("Captcha key unknown: {}", key),
        }
    }
}
