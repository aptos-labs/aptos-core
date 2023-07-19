// Copyright © Aptos Foundation

use image::ImageFormat;

pub struct ImageOptimizer;

impl ImageOptimizer {
    /**
     * Resizes and optimizes image from input URI.
     * Returns new image as a byte array and its format.
     */
    pub async fn optimize(_uri: Option<String>) -> Option<(Vec<u8>, ImageFormat)> {
        todo!();
    }
}
