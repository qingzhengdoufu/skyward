//! Wire-compatible with AERA protocol.hpp, pinned by AERA_COMMIT.

use anyhow::{Context, Result, ensure};

pub const MAGIC: u32 = 0x4152_5033;
pub const WIDTH: u32 = 1440;
pub const HEIGHT: u32 = 3200;
pub const VIEW_WIDTH: i32 = 360;
pub const VIEW_HEIGHT: i32 = 800;
pub const FRAME_BYTES: usize = WIDTH as usize * HEIGHT as usize * 4;
pub const SHARED_BYTES: usize = FRAME_BYTES * 2;
pub const MESSAGE_BYTES: usize = 2072;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub kind: u32,
    pub sequence: u32,
    pub x: i32,
    pub y: i32,
    pub value: u32,
    pub text: String,
}

impl Message {
    pub fn command(kind: u32) -> Self {
        Self {
            kind,
            sequence: 0,
            x: 0,
            y: 0,
            value: 0,
            text: String::new(),
        }
    }

    pub fn encode(&self) -> Result<[u8; MESSAGE_BYTES]> {
        ensure!(
            self.text.len() < 2048 && !self.text.contains('\0'),
            "browser text exceeds its wire bound"
        );
        let mut out = [0; MESSAGE_BYTES];
        for (k, value) in [MAGIC, self.kind, self.sequence, self.x as u32, self.y as u32, self.value]
            .into_iter()
            .enumerate()
        {
            out[k * 4..k * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        out[24..24 + self.text.len()].copy_from_slice(self.text.as_bytes());
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() == MESSAGE_BYTES, "browser packet length mismatch");
        let word = |k: usize| u32::from_le_bytes(bytes[k * 4..k * 4 + 4].try_into().expect("fixed header"));
        ensure!(word(0) == MAGIC, "browser protocol mismatch");
        let end = bytes[24..].iter().position(|b| *b == 0).context("unterminated browser text")?;
        Ok(Self {
            kind: word(1),
            sequence: word(2),
            x: word(3) as i32,
            y: word(4) as i32,
            value: word(5),
            text: std::str::from_utf8(&bytes[24..24 + end])?.into(),
        })
    }

    pub fn validate_worker(&self) -> Result<()> {
        let valid = match self.kind {
            32 => valid_viewport(self.x, self.y) && self.value == self.x as u32 * self.y as u32 * 4,
            33 => self.value <= 100 && (0..=1).contains(&self.x) && (0..=1).contains(&self.y),
            34 | 36 | 42 | 43 => true,
            35 => self.value <= 10,
            37..=41 => self.sequence > 0 && (0..=100).contains(&self.x) && self.y >= 0,
            _ => false,
        };
        ensure!(valid, "invalid browser event");
        Ok(())
    }

    pub fn frame_offset(&self) -> Result<usize> {
        self.validate_worker()?;
        ensure!(self.kind == 32, "not a browser frame");
        Ok(self.sequence as usize % 2 * FRAME_BYTES)
    }

    pub fn validate_command(&self) -> Result<()> {
        let valid = match self.kind {
            1 => address(&self.text).is_ok(),
            2..=5 | 10..=11 | 15 => true,
            9 => self.value <= 0x10ffff && !(0xd800..=0xdfff).contains(&self.value),
            16 => valid_viewport(self.x, self.y),
            17..=19 => self.value <= 100,
            6..=8 => (0..VIEW_WIDTH).contains(&self.x) && (0..VIEW_HEIGHT).contains(&self.y) && self.value < 2,
            12 => (50..=300).contains(&self.value),
            13 => self.sequence > 0,
            14 => self.value <= 2,
            _ => false,
        };
        ensure!(valid, "invalid browser command");
        Ok(())
    }
}

pub fn valid_viewport(width: i32, height: i32) -> bool {
    (320..=WIDTH as i32).contains(&width) && (400..=HEIGHT as i32).contains(&height) && width % 4 == 0 && height % 4 == 0
}

pub fn address(input: &str) -> Result<String> {
    let text = input.trim();
    ensure!(
        !text.is_empty() && text.len() <= 2040 && !text.chars().any(|c| c.is_control() || c.is_whitespace() || c == '\\'),
        "invalid browser address"
    );
    if matches!(text, "aera://start" | "aera://test") {
        return Ok(text.into());
    }
    let text = if text.contains("://") {
        text.to_string()
    } else {
        format!("https://{text}")
    };
    let url = url::Url::parse(&text)?;
    ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some() && url.username().is_empty() && url.password().is_none(),
        "only HTTP(S) browser addresses are allowed"
    );
    ensure!(url.as_str().len() < 2048, "browser address too long");
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_wire_layout_round_trips_and_rejects_truncation() {
        let m = Message {
            kind: 33,
            sequence: 7,
            x: 1,
            y: 0,
            value: 95,
            text: "网页".into(),
        };
        let bytes = m.encode().unwrap();
        assert_eq!(&bytes[..4], &MAGIC.to_le_bytes());
        assert_eq!(Message::decode(&bytes).unwrap(), m);
        for len in [0, 24, MESSAGE_BYTES - 1] {
            assert!(Message::decode(&bytes[..len]).is_err());
        }
        let mut invalid = bytes;
        invalid[24..].fill(b'x');
        assert!(Message::decode(&invalid).is_err());
    }

    #[test]
    fn only_bounded_worker_frames_select_one_of_two_fixed_slots() {
        let mut m = Message::command(32);
        m.x = WIDTH as i32;
        m.y = HEIGHT as i32;
        m.value = FRAME_BYTES as u32;
        m.sequence = 1;
        assert_eq!(m.frame_offset().unwrap(), FRAME_BYTES);
        m.sequence = 2;
        assert_eq!(m.frame_offset().unwrap(), 0);
        m.value -= 1;
        assert!(m.frame_offset().is_err());
        m.kind = 999;
        assert!(m.validate_worker().is_err());
    }

    #[test]
    fn native_viewport_and_smaller_frames_keep_the_maximum_slot_stride() {
        let mut frame = Message::command(32);
        frame.x = 1440;
        frame.y = 2492;
        frame.value = 1440 * 2492 * 4;
        frame.sequence = 1;
        assert_eq!(frame.frame_offset().unwrap(), FRAME_BYTES);
        for (width, height) in [(1441, 2492), (1440, 3204), (1440, 2493), (0, 800)] {
            let mut viewport = Message::command(16);
            viewport.x = width;
            viewport.y = height;
            assert!(viewport.validate_command().is_err());
        }
    }

    #[test]
    fn navigation_cannot_dispatch_a_recovery_operation_or_local_file() {
        assert_eq!(address("example.com").unwrap(), "https://example.com/");
        assert_eq!(address("aera://test").unwrap(), "aera://test");
        for bad in [
            "file:///data",
            "javascript:alert(1)",
            "data:text/html,x",
            "shell://reboot",
            "https://u:p@example.com",
            "https://a\\b",
            "https://a\nx",
            "",
        ] {
            assert!(address(bad).is_err(), "{bad}");
        }
    }
}
