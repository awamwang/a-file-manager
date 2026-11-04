// SPDX-License-Identifier: GPL-3.0-or-later

//! One-shot CLI: import Sigma Navigator tabs / Work Groups / Projects into A File Manager.
//!
//! ```text
//! cargo run --manifest-path src-tauri/Cargo.toml --bin sigma-import -- --dry-run
//! cargo run --manifest-path src-tauri/Cargo.toml --bin sigma-import -- --mode replace
//! ```

use a_file_manager_lib::sigma_import::{
    apply_import, default_afm_config_dir, discover_sigma_user_data, map_sigma_bundle,
    target_has_meaningful_data, try_sigma_root, ApplyRequest, ImportMode,
};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }

    let dry_run = args.iter().any(|a| a == "--dry-run");
    let mode = parse_mode(&args)?;
    let source_override = flag_value(&args, "--source");
    let target_override = flag_value(&args, "--target");

    let source = if let Some(root) = source_override {
        try_sigma_root(PathBuf::from(root).as_path()).ok_or_else(|| {
            "No Sigma user-data under --source (expected user-data/user-work-groups.json)"
                .to_string()
        })?
    } else {
        discover_sigma_user_data()?
    };

    let target = if let Some(dir) = target_override {
        PathBuf::from(dir)
    } else {
        default_afm_config_dir()
            .ok_or_else(|| "APPDATA not set; pass --target <afm-config-dir>".to_string())?
    };

    println!("Sigma source: {}", source.user_data_dir.display());
    println!("  work-groups: {}", source.work_groups.display());
    println!("  projects:    {}", source.projects.display());
    println!("  workspaces:  {}", source.workspaces.display());
    println!("AFM target:    {}", target.display());

    let mapped = map_sigma_bundle(&source)?;
    println!();
    println!(
        "Mapped: {} work group(s), {} project(s), {} navigator tab group(s)",
        mapped.work_groups.work_groups.len(),
        mapped.projects.projects.len(),
        mapped.report.navigator_tab_group_count
    );
    if !mapped.report.lossy_fields.is_empty() {
        println!("Lossy fields:");
        for field in &mapped.report.lossy_fields {
            println!("  - {}.{} — {}", field.surface, field.field, field.reason);
        }
    }
    for note in &mapped.report.notes {
        println!("Note: {note}");
    }

    let meaningful = target_has_meaningful_data(&target)?;
    let mode = match mode {
        Some(m) => m,
        None => {
            if dry_run {
                ImportMode::Replace
            } else if meaningful {
                eprintln!();
                eprintln!("STOP: AFM target already has Work Groups / Projects data.");
                eprintln!("Choose one and re-run:");
                eprintln!("  --mode replace   overwrite work-groups.json + projects.json");
                eprintln!("  --mode merge     keep existing ids; append missing only");
                eprintln!("(Use --dry-run to preview without writing.)");
                return Ok(ExitCode::from(2));
            } else {
                println!();
                println!("Target looks empty (default WG / no projects); using --mode replace.");
                ImportMode::Replace
            }
        }
    };

    let result = apply_import(
        &target,
        &mapped,
        &ApplyRequest {
            mode,
            dry_run,
            // Explicit --mode or empty target already gated above.
            allow_destructive: true,
        },
    )?;

    println!();
    println!(
        "Plan mode={:?} wg={} projects={} navTabGroups={}",
        result.plan.mode,
        result.plan.work_group_count,
        result.plan.project_count,
        result.plan.navigator_tab_group_count
    );
    if let Some(reason) = result.blocked_reason {
        eprintln!("Blocked: {reason}");
        return Ok(ExitCode::from(2));
    }
    if dry_run {
        println!("Dry-run only — no files written.");
        println!("Would write:");
        println!("  {}", result.work_groups_path);
        println!("  {}", result.projects_path);
    } else if result.wrote {
        println!("Wrote:");
        println!("  {}", result.work_groups_path);
        println!("  {}", result.projects_path);
    }

    Ok(ExitCode::SUCCESS)
}

fn parse_mode(args: &[String]) -> Result<Option<ImportMode>, String> {
    match flag_value(args, "--mode") {
        None => Ok(None),
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "replace" => Ok(Some(ImportMode::Replace)),
            "merge" => Ok(Some(ImportMode::Merge)),
            other => Err(format!(
                "Unknown --mode {other:?}; expected replace|merge"
            )),
        },
    }
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().map(|s| s.as_str());
        }
        if let Some(rest) = arg.strip_prefix(&format!("{name}=")) {
            return Some(rest);
        }
    }
    None
}

fn print_help() {
    println!(
        "sigma-import — import Sigma Navigator tabs / Work Groups / Projects into A File Manager

USAGE:
  sigma-import [--dry-run] [--mode replace|merge] [--source <dir>] [--target <afm-config-dir>]

DEFAULTS:
  --source  %APPDATA%\\com.sigma-file-manager.app  (then other candidates)
  --target  %APPDATA%\\com.a-file-manager.app

SAFETY:
  If the AFM target already has meaningful Work Groups / Projects and --mode is omitted,
  the tool stops and asks you to choose replace vs merge.
"
    );
}
