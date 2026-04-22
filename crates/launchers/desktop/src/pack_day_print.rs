use std::io;
use std::path::{Component, Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

use radroots_studio_app_models::{PackDayExportArtifactKind, PackDayExportBundle, PackDayPrintKind};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackDayPrintCommandPlan {
    pub kind: PackDayPrintKind,
    pub target_path: PathBuf,
    pub command_program: &'static str,
    pub command_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackDayPrintCommandResult {
    success: bool,
    exit_code: Option<i32>,
    stderr: String,
}

impl PackDayPrintCommandResult {
    #[cfg(test)]
    fn succeeded() -> Self {
        Self {
            success: true,
            exit_code: Some(0),
            stderr: String::new(),
        }
    }

    #[cfg(test)]
    fn failed(exit_code: Option<i32>, stderr: impl Into<String>) -> Self {
        Self {
            success: false,
            exit_code,
            stderr: stderr.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum PackDayPrintError {
    #[error("pack day export bundle directory does not exist: {path}")]
    MissingBundleDirectory { path: PathBuf },
    #[error("pack day print kind is not supported by this launcher path yet: {kind:?}")]
    UnsupportedKind { kind: PackDayPrintKind },
    #[error("pack day export bundle is missing required artifact {artifact_kind:?} for {kind:?}")]
    MissingArtifactReference {
        kind: PackDayPrintKind,
        artifact_kind: PackDayExportArtifactKind,
    },
    #[error("pack day export artifact path is invalid for {kind:?}: {relative_path}")]
    InvalidArtifactRelativePath {
        kind: PackDayPrintKind,
        relative_path: String,
    },
    #[error("pack day print target does not exist for {kind:?}: {path}")]
    MissingTargetPath {
        kind: PackDayPrintKind,
        path: PathBuf,
    },
    #[error("pack day print target must be a file for {kind:?}: {path}")]
    InvalidTargetFile {
        kind: PackDayPrintKind,
        path: PathBuf,
    },
    #[error("pack day print is only supported on macos")]
    UnsupportedPlatform,
    #[error("failed to launch macos print command {program} for {kind:?}: {source}")]
    CommandLaunch {
        kind: PackDayPrintKind,
        program: String,
        source: io::Error,
    },
    #[error("macos print command {program} for {kind:?} exited with code {exit_code:?}: {stderr}")]
    CommandFailed {
        kind: PackDayPrintKind,
        program: String,
        exit_code: Option<i32>,
        stderr: String,
    },
}

impl PartialEq for PackDayPrintError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::MissingBundleDirectory { path: left },
                Self::MissingBundleDirectory { path: right },
            ) => left == right,
            (Self::UnsupportedKind { kind: left }, Self::UnsupportedKind { kind: right }) => {
                left == right
            }
            (
                Self::MissingArtifactReference {
                    kind: left_kind,
                    artifact_kind: left_artifact,
                },
                Self::MissingArtifactReference {
                    kind: right_kind,
                    artifact_kind: right_artifact,
                },
            ) => left_kind == right_kind && left_artifact == right_artifact,
            (
                Self::InvalidArtifactRelativePath {
                    kind: left_kind,
                    relative_path: left_path,
                },
                Self::InvalidArtifactRelativePath {
                    kind: right_kind,
                    relative_path: right_path,
                },
            ) => left_kind == right_kind && left_path == right_path,
            (
                Self::MissingTargetPath {
                    kind: left_kind,
                    path: left_path,
                },
                Self::MissingTargetPath {
                    kind: right_kind,
                    path: right_path,
                },
            ) => left_kind == right_kind && left_path == right_path,
            (
                Self::InvalidTargetFile {
                    kind: left_kind,
                    path: left_path,
                },
                Self::InvalidTargetFile {
                    kind: right_kind,
                    path: right_path,
                },
            ) => left_kind == right_kind && left_path == right_path,
            (Self::UnsupportedPlatform, Self::UnsupportedPlatform) => true,
            (
                Self::CommandLaunch {
                    kind: left_kind,
                    program: left_program,
                    source: left_source,
                },
                Self::CommandLaunch {
                    kind: right_kind,
                    program: right_program,
                    source: right_source,
                },
            ) => {
                left_kind == right_kind
                    && left_program == right_program
                    && left_source.kind() == right_source.kind()
                    && left_source.to_string() == right_source.to_string()
            }
            (
                Self::CommandFailed {
                    kind: left_kind,
                    program: left_program,
                    exit_code: left_code,
                    stderr: left_stderr,
                },
                Self::CommandFailed {
                    kind: right_kind,
                    program: right_program,
                    exit_code: right_code,
                    stderr: right_stderr,
                },
            ) => {
                left_kind == right_kind
                    && left_program == right_program
                    && left_code == right_code
                    && left_stderr == right_stderr
            }
            _ => false,
        }
    }
}

impl Eq for PackDayPrintError {}

pub fn plan_pack_day_print(
    bundle: &PackDayExportBundle,
    kind: PackDayPrintKind,
) -> Result<PackDayPrintCommandPlan, PackDayPrintError> {
    let bundle_directory = PathBuf::from(&bundle.bundle_directory);
    if !bundle_directory.is_dir() {
        return Err(PackDayPrintError::MissingBundleDirectory {
            path: bundle_directory,
        });
    }

    let artifact_kind = match kind {
        PackDayPrintKind::PrintPackSheet => PackDayExportArtifactKind::PackSheet,
        PackDayPrintKind::PrintPickupRoster => PackDayExportArtifactKind::PickupRoster,
        PackDayPrintKind::PrintCustomerLabels => {
            return Err(PackDayPrintError::UnsupportedKind { kind });
        }
    };
    let target_path = resolve_bundle_artifact_path(bundle, artifact_kind, kind)?;

    Ok(PackDayPrintCommandPlan {
        kind,
        target_path: target_path.clone(),
        command_program: "lp",
        command_args: vec![target_path.to_string_lossy().into_owned()],
    })
}

pub fn execute_pack_day_print_plan(
    plan: &PackDayPrintCommandPlan,
) -> Result<(), PackDayPrintError> {
    #[cfg(target_os = "macos")]
    {
        execute_pack_day_print_plan_with(plan, run_macos_print_command)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = plan;
        Err(PackDayPrintError::UnsupportedPlatform)
    }
}

fn resolve_bundle_artifact_path(
    bundle: &PackDayExportBundle,
    artifact_kind: PackDayExportArtifactKind,
    kind: PackDayPrintKind,
) -> Result<PathBuf, PackDayPrintError> {
    let Some(artifact) = bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == artifact_kind)
    else {
        return Err(PackDayPrintError::MissingArtifactReference {
            kind,
            artifact_kind,
        });
    };

    let relative_path = Path::new(&artifact.relative_path);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(PackDayPrintError::InvalidArtifactRelativePath {
            kind,
            relative_path: artifact.relative_path.clone(),
        });
    }

    let path = PathBuf::from(&bundle.bundle_directory).join(relative_path);
    if !path.exists() {
        return Err(PackDayPrintError::MissingTargetPath { kind, path });
    }
    if !path.is_file() {
        return Err(PackDayPrintError::InvalidTargetFile { kind, path });
    }

    Ok(path)
}

fn execute_pack_day_print_plan_with(
    plan: &PackDayPrintCommandPlan,
    run_command: impl FnOnce(&PackDayPrintCommandPlan) -> Result<PackDayPrintCommandResult, io::Error>,
) -> Result<(), PackDayPrintError> {
    let result = run_command(plan).map_err(|source| PackDayPrintError::CommandLaunch {
        kind: plan.kind,
        program: plan.command_program.to_owned(),
        source,
    })?;

    if result.success {
        return Ok(());
    }

    Err(PackDayPrintError::CommandFailed {
        kind: plan.kind,
        program: plan.command_program.to_owned(),
        exit_code: result.exit_code,
        stderr: result.stderr,
    })
}

#[cfg(target_os = "macos")]
fn run_macos_print_command(
    plan: &PackDayPrintCommandPlan,
) -> Result<PackDayPrintCommandResult, io::Error> {
    let output = Command::new(plan.command_program)
        .args(&plan.command_args)
        .output()?;

    Ok(PackDayPrintCommandResult {
        success: output.status.success(),
        exit_code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PackDayPrintCommandResult, PackDayPrintError, execute_pack_day_print_plan_with,
        plan_pack_day_print,
    };
    use radroots_studio_app_models::{
        PackDayExportArtifact, PackDayExportArtifactKind, PackDayExportBundle,
        PackDayExportInstanceId, PackDayPrintKind,
    };
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("radroots_studio_app_pack_day_print_{}", Uuid::new_v4()));
            fs::create_dir_all(&path).expect("test directory should create");
            Self { path }
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_bundle(bundle_directory: &PathBuf) -> PackDayExportBundle {
        PackDayExportBundle {
            fulfillment_window_id: radroots_studio_app_models::FulfillmentWindowId::new(),
            export_instance_id: PackDayExportInstanceId::new(),
            generated_at_utc: "2026-04-23T15:00:00Z".to_owned(),
            bundle_directory: bundle_directory.to_string_lossy().into_owned(),
            artifacts: vec![
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PackSheet,
                    relative_path: "pack_sheet.txt".to_owned(),
                },
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::PickupRoster,
                    relative_path: "pickup_roster.txt".to_owned(),
                },
                PackDayExportArtifact {
                    kind: PackDayExportArtifactKind::CustomerLabels,
                    relative_path: "customer_labels.txt".to_owned(),
                },
            ],
        }
    }

    fn write_artifact(bundle_directory: &PathBuf, file_name: &str) -> PathBuf {
        let path = bundle_directory.join(file_name);
        fs::write(&path, file_name).expect("artifact should write");
        path
    }

    #[test]
    fn print_pack_sheet_plan_targets_the_exported_file_with_lp() {
        let temp_dir = TestDirectory::new();
        let pack_sheet_path = write_artifact(temp_dir.path(), "pack_sheet.txt");
        let bundle = sample_bundle(temp_dir.path());

        let plan = plan_pack_day_print(&bundle, PackDayPrintKind::PrintPackSheet)
            .expect("pack sheet print plan should build");

        assert_eq!(plan.kind, PackDayPrintKind::PrintPackSheet);
        assert_eq!(plan.target_path, pack_sheet_path.clone());
        assert_eq!(plan.command_program, "lp");
        assert_eq!(
            plan.command_args,
            vec![pack_sheet_path.to_string_lossy().into_owned()]
        );
    }

    #[test]
    fn print_pickup_roster_plan_targets_the_exported_file_with_lp() {
        let temp_dir = TestDirectory::new();
        let pickup_roster_path = write_artifact(temp_dir.path(), "pickup_roster.txt");
        let bundle = sample_bundle(temp_dir.path());

        let plan = plan_pack_day_print(&bundle, PackDayPrintKind::PrintPickupRoster)
            .expect("pickup roster print plan should build");

        assert_eq!(plan.kind, PackDayPrintKind::PrintPickupRoster);
        assert_eq!(plan.target_path, pickup_roster_path.clone());
        assert_eq!(plan.command_program, "lp");
        assert_eq!(
            plan.command_args,
            vec![pickup_roster_path.to_string_lossy().into_owned()]
        );
    }

    #[test]
    fn customer_labels_are_deferred_to_the_stock_preparation_slice() {
        let temp_dir = TestDirectory::new();
        let bundle = sample_bundle(temp_dir.path());

        let error = plan_pack_day_print(&bundle, PackDayPrintKind::PrintCustomerLabels)
            .expect_err("customer labels should remain deferred");

        assert_eq!(
            error,
            PackDayPrintError::UnsupportedKind {
                kind: PackDayPrintKind::PrintCustomerLabels,
            }
        );
    }

    #[test]
    fn planning_fails_when_pack_sheet_reference_is_missing_on_disk() {
        let temp_dir = TestDirectory::new();
        let bundle = sample_bundle(temp_dir.path());

        let error = plan_pack_day_print(&bundle, PackDayPrintKind::PrintPackSheet)
            .expect_err("missing pack sheet file should fail");

        assert_eq!(
            error,
            PackDayPrintError::MissingTargetPath {
                kind: PackDayPrintKind::PrintPackSheet,
                path: temp_dir.path().join("pack_sheet.txt"),
            }
        );
    }

    #[test]
    fn planning_fails_when_pickup_roster_relative_path_is_invalid() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        let mut bundle = sample_bundle(temp_dir.path());
        bundle.artifacts[1].relative_path = "../pickup_roster.txt".to_owned();

        let error = plan_pack_day_print(&bundle, PackDayPrintKind::PrintPickupRoster)
            .expect_err("invalid relative path should fail");

        assert_eq!(
            error,
            PackDayPrintError::InvalidArtifactRelativePath {
                kind: PackDayPrintKind::PrintPickupRoster,
                relative_path: "../pickup_roster.txt".to_owned(),
            }
        );
    }

    #[test]
    fn execution_accepts_successful_lp_runs() {
        let temp_dir = TestDirectory::new();
        let pack_sheet_path = write_artifact(temp_dir.path(), "pack_sheet.txt");
        let bundle = sample_bundle(temp_dir.path());
        let plan = plan_pack_day_print(&bundle, PackDayPrintKind::PrintPackSheet)
            .expect("pack sheet print plan should build");

        assert_eq!(plan.target_path, pack_sheet_path);
        assert!(
            execute_pack_day_print_plan_with(&plan, |_| {
                Ok(PackDayPrintCommandResult::succeeded())
            })
            .is_ok()
        );
    }

    #[test]
    fn execution_classifies_command_launch_failures() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pickup_roster.txt");
        let bundle = sample_bundle(temp_dir.path());
        let plan = plan_pack_day_print(&bundle, PackDayPrintKind::PrintPickupRoster)
            .expect("pickup roster print plan should build");

        let error = execute_pack_day_print_plan_with(&plan, |_| {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "lp unavailable",
            ))
        })
        .expect_err("launch failure should surface");

        assert_eq!(
            error,
            PackDayPrintError::CommandLaunch {
                kind: PackDayPrintKind::PrintPickupRoster,
                program: "lp".to_owned(),
                source: io::Error::new(io::ErrorKind::PermissionDenied, "lp unavailable"),
            }
        );
    }

    #[test]
    fn execution_classifies_nonzero_exit_failures() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "pack_sheet.txt");
        let bundle = sample_bundle(temp_dir.path());
        let plan = plan_pack_day_print(&bundle, PackDayPrintKind::PrintPackSheet)
            .expect("pack sheet print plan should build");

        let error = execute_pack_day_print_plan_with(&plan, |_| {
            Ok(PackDayPrintCommandResult::failed(
                Some(1),
                "lp: printer not found",
            ))
        })
        .expect_err("nonzero exit should surface");

        assert_eq!(
            error,
            PackDayPrintError::CommandFailed {
                kind: PackDayPrintKind::PrintPackSheet,
                program: "lp".to_owned(),
                exit_code: Some(1),
                stderr: "lp: printer not found".to_owned(),
            }
        );
    }
}
