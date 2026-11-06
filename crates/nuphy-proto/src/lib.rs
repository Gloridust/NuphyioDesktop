//! NuPhy keyboard configuration protocol.
//!
//! Pure logic only: frame encoding/decoding, command ids, device table and a
//! transport-agnostic [`Session`]. Reverse-engineered from the NuPhyIO web app
//! (WebHID); see `docs/PROTOCOL.md` for the full write-up and its caveats.

pub mod cmd;
pub mod device;
pub mod frame;
pub mod keymap;
pub mod session;

mod devices_table;

pub use device::{lookup, DeviceInfo, Family, Mode, NUPHY_VID};
pub use frame::{Frame, FrameError, Request, Response};
pub use session::{Session, SessionError, Transport};
