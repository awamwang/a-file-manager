// SPDX-License-Identifier: GPL-3.0-or-later

//! One-shot Sigma → A File Manager import for Navigator session / Work Groups / Projects.

mod apply;
pub mod discover;
mod map;
mod types;

pub use apply::{apply_import, target_has_meaningful_data, ApplyRequest, ApplyResult};
pub use discover::{default_afm_config_dir, discover_sigma_user_data, try_sigma_root, SigmaSourcePaths};
pub use map::{map_sigma_bundle, MappedImport, MappingReport};
pub use types::{ImportMode, ImportPlan, LossyField};
