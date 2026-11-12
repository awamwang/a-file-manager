// SPDX-License-Identifier: GPL-3.0-or-later
//! ExtensionHost vNext (S8).
//! - API break OK — legacy marketplace packages are not migrated
//! - Empty marketplace catalog (no GitHub / Sigma market fetch)
//! - JS/UI command channel with crash isolation
//! - FS provider registration hooks (contribution metadata; ProviderRegistry is FS authority)

mod commands;
mod host;
mod pack_fs;
mod permissions;
pub mod types;

pub use commands::*;
