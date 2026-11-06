//! 64-byte frame codec for the mechanical-keyboard family.
//!
//! Request layout (all offsets in the 64-byte report, no report id):
//!
//! | off | meaning                                   |
//! |-----|-------------------------------------------|
//! | 0   | `0x55` (write/request flag)               |
//! | 1   | command                                   |
//! | 2   | 0                                         |
//! | 3   | checksum = sum(bytes 4..64) & 0xFF        |
//! | 4   | payload length        ^ key               |
//! | 5,6 | address, little endian ^ key              |
//! | 7   | handle                ^ key               |
//! | 8.. | data                  ^ key (zero padded) |
//!
//! Responses use `0xAA` as byte 0 and the same layout.

pub const REPORT_LEN: usize = 64;
pub const HEADER_LEN: usize = 8;
pub const MAX_PAYLOAD: usize = REPORT_LEN - HEADER_LEN;

pub const FLAG_REQUEST: u8 = 0x55;
pub const FLAG_RESPONSE: u8 = 0xAA;

pub type Frame = [u8; REPORT_LEN];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// Byte 0 is not `0xAA` (probably an unsolicited event report).
    NotAResponse(u8),
    /// Payload longer than 56 bytes.
    PayloadTooLong(usize),
    /// Declared length exceeds what fits in a report.
    BadLength(u8),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAResponse(b) => write!(f, "not a response frame (first byte {b:#04x})"),
            Self::PayloadTooLong(n) => write!(f, "payload of {n} bytes exceeds {MAX_PAYLOAD}"),
            Self::BadLength(n) => write!(f, "declared length {n} exceeds {MAX_PAYLOAD}"),
        }
    }
}
impl std::error::Error for FrameError {}

/// Sum of bytes `4..64`, modulo 256.
pub fn checksum(frame: &Frame) -> u8 {
    frame[4..].iter().fold(0u8, |a, b| a.wrapping_add(*b))
}

/// A command to send. For reads `len` is how many bytes are wanted and `data`
/// is empty; for writes `len == data.len()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub cmd: u8,
    pub addr: u16,
    pub handle: u8,
    pub len: u8,
    pub data: Vec<u8>,
}

impl Request {
    pub fn read(cmd: u8, addr: u16, handle: u8, len: u8) -> Self {
        Self { cmd, addr, handle, len, data: Vec::new() }
    }

    pub fn write(cmd: u8, addr: u16, handle: u8, data: &[u8]) -> Result<Self, FrameError> {
        if data.len() > MAX_PAYLOAD {
            return Err(FrameError::PayloadTooLong(data.len()));
        }
        Ok(Self { cmd, addr, handle, len: data.len() as u8, data: data.to_vec() })
    }

    /// Encode with the session `key` (`0` before the handshake completes).
    pub fn encode(&self, key: u8) -> Result<Frame, FrameError> {
        if self.data.len() > MAX_PAYLOAD || self.len as usize > MAX_PAYLOAD {
            return Err(FrameError::PayloadTooLong(self.data.len().max(self.len as usize)));
        }
        let mut f = [0u8; REPORT_LEN];
        f[0] = FLAG_REQUEST;
        f[1] = self.cmd;
        f[4] = self.len ^ key;
        f[5] = (self.addr as u8) ^ key;
        f[6] = ((self.addr >> 8) as u8) ^ key;
        f[7] = self.handle ^ key;
        for (i, b) in self.data.iter().enumerate() {
            f[HEADER_LEN + i] = b ^ key;
        }
        f[3] = checksum(&f);
        Ok(f)
    }
}

/// A decoded response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub cmd: u8,
    pub len: u8,
    pub addr: u16,
    pub handle: u8,
    pub data: Vec<u8>,
    /// Whether byte 3 matched the computed checksum. Reported rather than
    /// enforced: response checksums have only been observed in the web app's
    /// simulator, not on hardware.
    pub checksum_ok: bool,
}

impl Response {
    pub fn decode(frame: &Frame, key: u8) -> Result<Self, FrameError> {
        if frame[0] != FLAG_RESPONSE {
            return Err(FrameError::NotAResponse(frame[0]));
        }
        let len = frame[4] ^ key;
        if len as usize > MAX_PAYLOAD {
            return Err(FrameError::BadLength(len));
        }
        let data = frame[HEADER_LEN..HEADER_LEN + len as usize].iter().map(|b| b ^ key).collect();
        Ok(Self {
            cmd: frame[1],
            len,
            addr: u16::from(frame[5] ^ key) | u16::from(frame[6] ^ key) << 8,
            handle: frame[7] ^ key,
            data,
            checksum_ok: frame[3] == checksum(frame),
        })
    }
}

/// Build the `SetSecretKey` frame and return it with the key it establishes.
///
/// The frame is *not* XOR-encrypted. Bytes 8..64 carry `random`; the key is
/// `random[20]` (frame byte 28), or `0xAA` if that byte is zero.
pub fn handshake(random: &[u8; MAX_PAYLOAD]) -> (Frame, u8) {
    let mut f = [0u8; REPORT_LEN];
    f[0] = FLAG_REQUEST;
    f[1] = crate::cmd::mech::SET_SECRET_KEY;
    f[HEADER_LEN..].copy_from_slice(random);
    f[3] = checksum(&f);
    let key = if random[20] == 0 { 0xAA } else { random[20] };
    (f, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::mech;

    #[test]
    fn read_request_without_key() {
        let f = Request::read(mech::GET_FIRMWARE_INFO, 0, 0, 8).encode(0).unwrap();
        assert_eq!(&f[..8], &[0x55, 0xA1, 0, 8, 8, 0, 0, 0]);
        assert!(f[8..].iter().all(|b| *b == 0));
    }

    #[test]
    fn keyed_request_xors_fields_not_padding() {
        let k = 0x5A;
        let f = Request::read(mech::GET_USE_KEYS, 0x0138, 2, 56).encode(k).unwrap();
        assert_eq!(f[4], 56 ^ k);
        assert_eq!(f[5], 0x38 ^ k);
        assert_eq!(f[6], 0x01 ^ k);
        assert_eq!(f[7], 2 ^ k);
        assert!(f[8..].iter().all(|b| *b == 0), "padding stays zero");
        assert_eq!(f[3], checksum(&f));
    }

    #[test]
    fn write_request_xors_data() {
        let k = 0x11;
        let f = Request::write(mech::SET_USE_KEYS, 4, 0, &[1, 2, 3]).unwrap().encode(k).unwrap();
        assert_eq!(&f[8..11], &[1 ^ k, 2 ^ k, 3 ^ k]);
        assert_eq!(f[4], 3 ^ k);
    }

    #[test]
    fn too_long_payload_rejected() {
        assert!(Request::write(0xB3, 0, 0, &[0; 57]).is_err());
    }

    #[test]
    fn response_roundtrip() {
        // Build a response the way the web app's simulator does.
        let k = 0x33;
        let payload = [18u8, 0, 1, 170, 7, 0, 0, 0];
        let mut f = [0u8; REPORT_LEN];
        f[0] = FLAG_RESPONSE;
        f[1] = mech::GET_FIRMWARE_INFO;
        f[4] = 8 ^ k;
        f[5] = k;
        f[6] = k;
        f[7] = k;
        for (i, b) in payload.iter().enumerate() {
            f[8 + i] = b ^ k;
        }
        f[3] = checksum(&f);
        let r = Response::decode(&f, k).unwrap();
        assert_eq!(r.cmd, mech::GET_FIRMWARE_INFO);
        assert_eq!(r.data, payload);
        assert!(r.checksum_ok);
    }

    #[test]
    fn events_are_not_responses() {
        let mut f = [0u8; REPORT_LEN];
        f[0] = crate::cmd::mech::event::KEY_UPLOAD;
        assert_eq!(Response::decode(&f, 0), Err(FrameError::NotAResponse(0xC1)));
    }

    #[test]
    fn handshake_key_and_checksum() {
        let mut rnd = [7u8; MAX_PAYLOAD];
        rnd[20] = 0x9C;
        let (f, key) = handshake(&rnd);
        assert_eq!(key, 0x9C);
        assert_eq!(f[28], 0x9C);
        assert_eq!(f[1], 0xEE);
        assert_eq!(f[4], 0);
        assert_eq!(f[3], checksum(&f));
        rnd[20] = 0;
        assert_eq!(handshake(&rnd).1, 0xAA);
    }
}
