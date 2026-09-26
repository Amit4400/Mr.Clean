//! Mr.Clean core: every scan, clean and safety decision lives here so it can be
//! unit-tested without the UI. The Tauri app in `src-tauri` is a thin wrapper.

pub mod bigfiles;
pub mod cleaner;
pub mod env;
pub mod fsutil;
pub mod memory;
pub mod safety;
pub mod security;
pub mod system;

pub use env::{Env, Os};
pub use fsutil::Cancel;
