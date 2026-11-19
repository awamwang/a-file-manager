// SPDX-License-Identifier: GPL-3.0-or-later
//! Feature Pack 宿主：本机 pack 的安装/启停/贡献聚合（ADR 0012 / 0022）。
//! 遗留 vNext 扩展宿主（marketplace / JS-UI command / fs provider 钩子）已移除，
//! 「能力扩展」唯一单元即 Feature Pack。

mod commands;
mod host;
pub(crate) mod pack_fs;
mod permissions;
pub mod types;

pub use commands::*;
