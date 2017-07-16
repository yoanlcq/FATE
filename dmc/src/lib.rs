//! DMC - DirectMedia Crate
//! 
//! This is an attempt at an SDL2 rewrite in Rust. The end goal is to get
//! rid of the dependency on SDL2's DLL for Rust apps.

//#![feature(test)]
//#![warn(missing_docs)]
#![doc(test(attr(deny(warnings))))]
#![cfg_attr(feature="cargo-clippy", allow(doc_markdown))]

// TODO "log!" everything

pub mod semver;
pub use semver::Semver;
pub mod display;
pub use display::Display;
pub mod game_input_device;
pub use game_input_device::{GameInputDevice, Dpad, Minmax, SignedAxis, UnsignedAxis};
pub mod event;
pub use event::{EventQueue, Clipboard, TextInput};
pub mod battery;
pub use battery::{BatteryState, BatteryStatus};
pub mod timeout;
pub use timeout::Timeout;
pub mod option_alternatives;
pub use option_alternatives::Decision;
pub use option_alternatives::Knowledge;
pub use option_alternatives::Decision::*;
pub use option_alternatives::Knowledge::*;
pub mod vec;
pub use vec::*;
