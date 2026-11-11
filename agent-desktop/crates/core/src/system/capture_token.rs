//! Opaque capture metadata that binds an image-space point to the window it
//! came from.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{AdapterError, DeliverySemantics, ErrorCode, Point, Rect, WindowInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CaptureToken {
    pub(crate) window_id: String,
    pub(crate) process_instance: String,
    pub(crate) window_bounds: Rect,
    pub(crate) capture_bounds: Rect,
    pub(crate) image_width: u32,
    pub(crate) image_height: u32,
    pub(crate) captured_at_ms: u64,
}

impl CaptureToken {
    pub(crate) fn new(
        window: &WindowInfo,
        capture_bounds: Rect,
        image_width: u32,
        image_height: u32,
    ) -> Result<Self, AdapterError> {
        let window_bounds = window
            .bounds
            .ok_or_else(|| invalid_capture("The captured window has no bounds"))?;
        let token = Self {
            window_id: window.id.clone(),
            process_instance: window.process_instance.clone().unwrap_or_default(),
            window_bounds,
            capture_bounds,
            image_width,
            image_height,
            captured_at_ms: now_ms(),
        };
        token.validate()?;
        Ok(token)
    }

    pub(crate) fn encode(&self) -> Result<String, AdapterError> {
        let bytes = serde_json::to_vec(self).map_err(|error| {
            AdapterError::internal(format!("capture metadata could not be encoded: {error}"))
        })?;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn decode(encoded: &str) -> Result<Self, AdapterError> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| invalid_capture("The capture id is not valid base64url"))?;
        let token = serde_json::from_slice::<Self>(&bytes)
            .map_err(|_| invalid_capture("The capture id does not contain capture metadata"))?;
        token.validate()?;
        Ok(token)
    }

    pub(crate) fn age_ms(&self) -> u64 {
        now_ms().saturating_sub(self.captured_at_ms)
    }

    pub(crate) fn image_to_screen(&self, x: f64, y: f64) -> Result<Point, AdapterError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(invalid_capture("Image coordinates must be finite"));
        }
        if !(0.0..f64::from(self.image_width)).contains(&x)
            || !(0.0..f64::from(self.image_height)).contains(&y)
        {
            return Err(invalid_capture(
                "Image coordinates fall outside the captured image",
            ));
        }
        let point = Point {
            x: self.capture_bounds.x + x * self.capture_bounds.width / f64::from(self.image_width),
            y: self.capture_bounds.y
                + y * self.capture_bounds.height / f64::from(self.image_height),
        };
        point.validate()?;
        Ok(point)
    }

    fn validate(&self) -> Result<(), AdapterError> {
        if self.window_id.is_empty() {
            return Err(invalid_capture("The capture has no window id"));
        }
        if self.process_instance.is_empty() {
            return Err(invalid_capture("The capture has no process instance"));
        }
        if self.image_width == 0 || self.image_height == 0 {
            return Err(invalid_capture("The capture image has no area"));
        }
        for bounds in [self.window_bounds, self.capture_bounds] {
            bounds.validate()?;
            if bounds.width <= 0.0 || bounds.height <= 0.0 {
                return Err(invalid_capture("The capture has zero-area bounds"));
            }
        }
        Ok(())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn invalid_capture(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::InvalidArgs, message)
        .with_suggestion("Take a fresh screenshot and pass its capture_id unchanged")
        .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "capture_token_tests.rs"]
mod tests;
