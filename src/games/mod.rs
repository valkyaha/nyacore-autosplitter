//! Game-specific autosplitter implementations
//!
//! Each game module provides pattern scanning and flag reading for a specific FromSoftware game.

pub mod armored_core_6;
pub mod dark_souls_1;
pub mod dark_souls_2;
pub mod dark_souls_3;
pub mod elden_ring;
pub mod event_flags;
pub mod sekiro;

pub use armored_core_6::ArmoredCore6;
pub use dark_souls_1::DarkSouls1;
pub use dark_souls_2::DarkSouls2;
pub use dark_souls_3::DarkSouls3;
pub use elden_ring::EldenRing;
pub use event_flags::{BinaryTree, CategoryDecomposition, KillCounter, OffsetTable};
pub use sekiro::Sekiro;
