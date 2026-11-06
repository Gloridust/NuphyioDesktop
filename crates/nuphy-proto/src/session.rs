//! Transport-agnostic request/response session (mechanical family).

use std::collections::VecDeque;
use std::fmt;
use std::io;
use std::time::Duration;

use crate::cmd::mech;
use crate::frame::{self, Frame, FrameError, Request, Response, MAX_PAYLOAD};

/// A raw 64-byte HID report pipe (no report id on the wire).
pub trait Transport {
    fn write(&mut self, report: &Frame) -> io::Result<()>;
    /// Read one report; `Ok(None)` on timeout.
    fn read(&mut self, timeout: Duration) -> io::Result<Option<Frame>>;
}

#[derive(Debug)]
pub enum SessionError {
    Io(io::Error),
    Frame(FrameError),
    /// No matching response after all retries.
    Timeout,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Frame(e) => write!(f, "frame error: {e}"),
            Self::Timeout => write!(f, "timed out waiting for the keyboard"),
        }
    }
}
impl std::error::Error for SessionError {}
impl From<io::Error> for SessionError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<FrameError> for SessionError {
    fn from(e: FrameError) -> Self {
        Self::Frame(e)
    }
}

pub struct Session<T: Transport> {
    transport: T,
    key: u8,
    /// Unsolicited reports seen while waiting for a response.
    pub events: VecDeque<Frame>,
    pub timeout: Duration,
    pub retries: u32,
}

impl<T: Transport> Session<T> {
    /// Perform the secret-key handshake. `random` should come from a CSPRNG
    /// (the web app uses `crypto.getRandomValues`).
    pub fn open(transport: T, mut random: [u8; MAX_PAYLOAD]) -> Result<Self, SessionError> {
        if random[20] == 0 {
            random[20] = 1; // avoid the 0 → 0xAA special case; any non-zero works
        }
        let mut s = Self {
            transport,
            key: 0,
            events: VecDeque::new(),
            timeout: Duration::from_millis(500),
            retries: 3,
        };
        let (frame, key) = frame::handshake(&random);
        s.exchange(&frame, mech::SET_SECRET_KEY, 0)?;
        s.key = key;
        Ok(s)
    }

    pub fn key(&self) -> u8 {
        self.key
    }

    pub fn into_transport(self) -> T {
        self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Send one request and wait for the response with the same command id.
    pub fn request(&mut self, req: &Request) -> Result<Response, SessionError> {
        let frame = req.encode(self.key)?;
        let raw = self.exchange(&frame, req.cmd, self.key)?;
        Ok(Response::decode(&raw, self.key)?)
    }

    /// Read `total` bytes of a command's memory region, in ≤56-byte chunks.
    pub fn read_region(&mut self, cmd: u8, handle: u8, total: usize) -> Result<Vec<u8>, SessionError> {
        let mut out = Vec::with_capacity(total);
        while out.len() < total {
            let n = (total - out.len()).min(MAX_PAYLOAD);
            let r = self.request(&Request::read(cmd, out.len() as u16, handle, n as u8))?;
            if r.data.is_empty() {
                break;
            }
            out.extend_from_slice(&r.data);
        }
        out.truncate(total);
        Ok(out)
    }

    fn exchange(&mut self, frame: &Frame, cmd: u8, _key: u8) -> Result<Frame, SessionError> {
        for _ in 0..=self.retries {
            self.transport.write(frame)?;
            // Drain until a response for this command shows up or we time out.
            while let Some(raw) = self.transport.read(self.timeout)? {
                if raw[0] == frame::FLAG_RESPONSE && raw[1] == cmd {
                    return Ok(raw);
                }
                if raw[0] != frame::FLAG_RESPONSE {
                    if self.events.len() >= 64 {
                        self.events.pop_front();
                    }
                    self.events.push_back(raw);
                }
            }
        }
        Err(SessionError::Timeout)
    }
}

/// Decode `GetFirmwareInfo` data into a version string.
///
/// Layout: `[patch, minor, major, flag, build_lo, build_hi, ..]`; the version
/// is `major.minor.patch`, plus `.build` when `flag == 0xAA`.
pub fn format_firmware_version(d: &[u8]) -> Option<String> {
    if d.len() < 6 {
        return None;
    }
    let mut s = format!("{}.{}.{}", d[2], d[1], d[0]);
    if d[3] == 0xAA {
        s.push_str(&format!(".{}", u16::from(d[4]) | u16::from(d[5]) << 8));
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::REPORT_LEN;

    /// In-memory keyboard that mimics the web app's simulator.
    struct Fake {
        key: u8,
        regions: Vec<(u8, Vec<u8>)>,
        queue: VecDeque<Frame>,
        noise_first: bool,
        drop_next: u32,
    }

    impl Fake {
        fn new(regions: Vec<(u8, Vec<u8>)>) -> Self {
            Self { key: 0, regions, queue: VecDeque::new(), noise_first: false, drop_next: 0 }
        }
    }

    impl Transport for Fake {
        fn write(&mut self, f: &Frame) -> io::Result<()> {
            if self.drop_next > 0 {
                self.drop_next -= 1;
                return Ok(());
            }
            assert_eq!(f[0], frame::FLAG_REQUEST);
            assert_eq!(f[3], frame::checksum(f), "host checksum");
            if self.noise_first {
                let mut n = [0u8; REPORT_LEN];
                n[0] = mech::event::LIGHT_STATE_CHANGE;
                self.queue.push_back(n);
            }
            if f[1] == mech::SET_SECRET_KEY {
                self.key = f[28];
                let mut r = *f;
                r[0] = frame::FLAG_RESPONSE;
                self.queue.push_back(r);
                return Ok(());
            }
            let k = self.key;
            let len = (f[4] ^ k) as usize;
            let addr = (u16::from(f[5] ^ k) | u16::from(f[6] ^ k) << 8) as usize;
            let mem = &self.regions.iter().find(|(c, _)| *c == f[1]).expect("region").1;
            let mut r = [0u8; REPORT_LEN];
            r[0] = frame::FLAG_RESPONSE;
            r[1] = f[1];
            r[4] = (len as u8) ^ k;
            r[5] = f[5];
            r[6] = f[6];
            r[7] = f[7];
            for i in 0..len {
                r[8 + i] = mem[addr + i] ^ k;
            }
            r[3] = frame::checksum(&r);
            self.queue.push_back(r);
            Ok(())
        }

        fn read(&mut self, _t: Duration) -> io::Result<Option<Frame>> {
            Ok(self.queue.pop_front())
        }
    }

    fn rnd() -> [u8; MAX_PAYLOAD] {
        let mut r = [0u8; MAX_PAYLOAD];
        for (i, b) in r.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(37).wrapping_add(11);
        }
        r
    }

    #[test]
    fn handshake_then_read_firmware() {
        let fake = Fake::new(vec![(mech::GET_FIRMWARE_INFO, vec![18, 0, 1, 170, 7, 0, 0, 0])]);
        let mut s = Session::open(fake, rnd()).unwrap();
        assert_ne!(s.key(), 0);
        let d = s.read_region(mech::GET_FIRMWARE_INFO, 0, 8).unwrap();
        assert_eq!(format_firmware_version(&d).as_deref(), Some("1.0.18.7"));
    }

    #[test]
    fn chunked_read_reassembles_and_events_are_queued() {
        let big: Vec<u8> = (0..150u32).map(|i| (i * 7) as u8).collect();
        let mut fake = Fake::new(vec![(mech::GET_USE_KEYS, big.clone())]);
        fake.noise_first = true;
        let mut s = Session::open(fake, rnd()).unwrap();
        let d = s.read_region(mech::GET_USE_KEYS, 0, 150).unwrap();
        assert_eq!(d, big);
        assert!(!s.events.is_empty());
    }

    #[test]
    fn retries_after_lost_write() {
        let fake = Fake::new(vec![(mech::GET_BASE, vec![1, 2, 3, 4])]);
        let mut s = Session::open(fake, rnd()).unwrap();
        s.timeout = Duration::from_millis(1);
        s.transport_mut().drop_next = 1; // first attempt is lost
        let r = s.request(&Request::read(mech::GET_BASE, 0, 0, 4)).unwrap();
        assert_eq!(r.data, [1, 2, 3, 4]);
        assert!(r.checksum_ok);
    }

    #[test]
    fn gives_up_after_retries() {
        let fake = Fake::new(vec![(mech::GET_BASE, vec![1, 2, 3, 4])]);
        let mut s = Session::open(fake, rnd()).unwrap();
        s.transport_mut().drop_next = 100;
        assert!(matches!(
            s.request(&Request::read(mech::GET_BASE, 0, 0, 4)),
            Err(SessionError::Timeout)
        ));
    }

    #[test]
    fn firmware_version_without_build_flag() {
        assert_eq!(format_firmware_version(&[3, 2, 1, 0, 9, 9]).as_deref(), Some("1.2.3"));
        assert_eq!(format_firmware_version(&[1, 2]), None);
    }
}
