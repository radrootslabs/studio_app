use std::io;
use std::path::{Component, Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

use radroots_studio_app_models::{PackDayExportArtifactKind, PackDayExportBundle, PackDayHostHandoffKind};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackDayHostHandoffCommandPlan {
    pub kind: PackDayHostHandoffKind,
    pub target_path: PathBuf,
    pub command_program: &'static str,
    pub command_args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackDayHostHandoffCommandResult {
    success: bool,
    exit_code: Option<i32>,
    stderr: String,
}

impl PackDayHostHandoffCommandResult {
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
pub enum PackDayHostHandoffError {
    #[error("pack day export bundle directory does not exist: {path}")]
    MissingBundleDirectory { path: PathBuf },
    #[error("pack day export bundle is missing required artifact {artifact_kind:?} for {kind:?}")]
    MissingArtifactReference {
        kind: PackDayHostHandoffKind,
        artifact_kind: PackDayExportArtifactKind,
    },
    #[error("pack day export artifact path is invalid for {kind:?}: {relative_path}")]
    InvalidArtifactRelativePath {
        kind: PackDayHostHandoffKind,
        relative_path: String,
    },
    #[error("pack day host handoff target does not exist for {kind:?}: {path}")]
    MissingTargetPath {
        kind: PackDayHostHandoffKind,
        path: PathBuf,
    },
    #[error("pack day host handoff target must be a file for {kind:?}: {path}")]
    InvalidTargetFile {
        kind: PackDayHostHandoffKind,
        path: PathBuf,
    },
    #[error("pack day host handoff is only supported on macos")]
    UnsupportedPlatform,
    #[error("failed to launch macos host command {program} for {kind:?}: {source}")]
    CommandLaunch {
        kind: PackDayHostHandoffKind,
        program: String,
        source: io::Error,
    },
    #[error("macos host command {program} for {kind:?} exited with code {exit_code:?}: {stderr}")]
    CommandFailed {
        kind: PackDayHostHandoffKind,
        program: String,
        exit_code: Option<i32>,
        stderr: String,
    },
}

impl PartialEq for PackDayHostHandoffError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::MissingBundleDirectory { path: left },
                Self::MissingBundleDirectory { path: right },
            ) => left == right,
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

impl Eq for PackDayHostHandoffError {}

pub fn plan_pack_day_host_handoff(
    bundle: &PackDayExportBundle,
    kind: PackDayHostHandoffKind,
) -> Result<PackDayHostHandoffCommandPlan, PackDayHostHandoffError> {
    let bundle_directory = PathBuf::from(&bundle.bundle_directory);
    if !bundle_directory.is_dir() {
        return Err(PackDayHostHandoffError::MissingBundleDirectory {
            path: bundle_directory,
        });
    }

    let target_path = match kind {
        PackDayHostHandoffKind::RevealBundle => bundle_directory.clone(),
        PackDayHostHandoffKind::OpenPackSheet => {
            resolve_bundle_artifact_path(bundle, PackDayExportArtifactKind::PackSheet, kind)?
        }
    };

    let command_args = match kind {
        PackDayHostHandoffKind::RevealBundle => {
            vec!["-R".to_owned(), target_path.to_string_lossy().into_owned()]
        }
        PackDayHostHandoffKind::OpenPackSheet => {
            vec![target_path.to_string_lossy().into_owned()]
        }
    };

    Ok(PackDayHostHandoffCommandPlan {
        kind,
        target_path,
        command_program: "open",
        command_args,
    })
}

pub fn execute_pack_day_host_handoff_plan(
    plan: &PackDayHostHandoffCommandPlan,
) -> Result<(), PackDayHostHandoffError> {
    #[cfg(target_os = "macos")]
    {
        execute_pack_day_host_handoff_plan_with(plan, run_macos_host_command)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = plan;
        Err(PackDayHostHandoffError::UnsupportedPlatform)
    }
}

fn resolve_bundle_artifact_path(
    bundle: &PackDayExportBundle,
    artifact_kind: PackDayExportArtifactKind,
    kind: PackDayHostHandoffKind,
) -> Result<PathBuf, PackDayHostHandoffError> {
    let Some(artifact) = bundle
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == artifact_kind)
    else {
        return Err(PackDayHostHandoffError::MissingArtifactReference {
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
        return Err(PackDayHostHandoffError::InvalidArtifactRelativePath {
            kind,
            relative_path: artifact.relative_path.clone(),
        });
    }

    let path = PathBuf::from(&bundle.bundle_directory).join(relative_path);
    if !path.exists() {
        return Err(PackDayHostHandoffError::MissingTargetPath { kind, path });
    }
    if !path.is_file() {
        return Err(PackDayHostHandoffError::InvalidTargetFile { kind, path });
    }

    Ok(path)
}

fn execute_pack_day_host_handoff_plan_with(
    plan: &PackDayHostHandoffCommandPlan,
    run_command: impl FnOnce(
        &PackDayHostHandoffCommandPlan,
    ) -> Result<PackDayHostHandoffCommandResult, io::Error>,
) -> Result<(), PackDayHostHandoffError> {
    let result = run_command(plan).map_err(|source| PackDayHostHandoffError::CommandLaunch {
        kind: plan.kind,
        program: plan.command_program.to_owned(),
        source,
    })?;

    if result.success {
        return Ok(());
    }

    Err(PackDayHostHandoffError::CommandFailed {
        kind: plan.kind,
        program: plan.command_program.to_owned(),
        exit_code: result.exit_code,
        stderr: result.stderr,
    })
}

#[cfg(target_os = "macos")]
fn run_macos_host_command(
    plan: &PackDayHostHandoffCommandPlan,
) -> Result<PackDayHostHandoffCommandResult, io::Error> {
    let output = Command::new(plan.command_program)
        .args(&plan.command_args)
        .output()?;

    Ok(PackDayHostHandoffCommandResult {
        success: output.status.success(),
        exit_code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        execute_pack_day_host_handoff_plan_with, plan_pack_day_host_handoff,
        PackDayHostHandoffCommandResult, PackDayHostHandoffError,
    };
    use radroots_studio_app_models::{
        PackDayExportArtifact, PackDayExportArtifactKind, PackDayExportBundle,
        PackDayHostHandoffKind,
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
            let path = std::env::temp_dir().join(format!(
                "radroots_studio_app_pack_day_host_handoff_{}",
                Uuid::new_v4()
            ));
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

    #[test]
    fn reveal_bundle_plan_uses_open_reveal_for_the_bundle_directory() {
        let temp_dir = TestDirectory::new();
        let bundle = sample_bundle(temp_dir.path());

        let plan = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::RevealBundle)
            .expect("reveal plan should build");

        assert_eq!(plan.kind, PackDayHostHandoffKind::RevealBundle);
        assert_eq!(plan.target_path, temp_dir.path().clone());
        assert_eq!(plan.command_program, "open");
        assert_eq!(
            plan.command_args,
            vec![
                "-R".to_owned(),
                temp_dir.path().to_string_lossy().into_owned(),
            ]
        );
    }

    #[test]
    fn open_pack_sheet_plan_targets_the_exported_pack_sheet() {
        let temp_dir = TestDirectory::new();
        let pack_sheet_path = temp_dir.path().join("pack_sheet.txt");
        fs::write(&pack_sheet_path, "pack day").expect("pack sheet should write");
        let bundle = sample_bundle(temp_dir.path());

        let plan = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::OpenPackSheet)
            .expect("open plan should build");

        assert_eq!(plan.kind, PackDayHostHandoffKind::OpenPackSheet);
        assert_eq!(plan.target_path, pack_sheet_path.clone());
        assert_eq!(plan.command_program, "open");
        assert_eq!(
            plan.command_args,
            vec![pack_sheet_path.to_string_lossy().into_owned()]
        );
    }

    #[test]
    fn planning_fails_when_the_bundle_directory_is_missing() {
        let bundle_directory = std::env::temp_dir().join(format!(
            "radroots_studio_app_pack_day_host_handoff_missing_{}",
            Uuid::new_v4()
        ));
        let bundle = sample_bundle(&bundle_directory);

        let error = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::RevealBundle)
            .expect_err("missing bundle directory should fail");

        assert_eq!(
            error,
            PackDayHostHandoffError::MissingBundleDirectory {
                path: bundle_directory,
            }
        );
    }

    #[test]
    fn planning_fails_when_pack_sheet_reference_is_missing() {
        let temp_dir = TestDirectory::new();
        let mut bundle = sample_bundle(temp_dir.path());
        bundle
            .artifacts
            .retain(|artifact| artifact.kind != PackDayExportArtifactKind::PackSheet);

        let error = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::OpenPackSheet)
            .expect_err("missing pack sheet artifact should fail");

        assert_eq!(
            error,
            PackDayHostHandoffError::MissingArtifactReference {
                kind: PackDayHostHandoffKind::OpenPackSheet,
                artifact_kind: PackDayExportArtifactKind::PackSheet,
            }
        );
    }

    #[test]
    fn planning_fails_when_pack_sheet_relative_path_is_invalid() {
        let temp_dir = TestDirectory::new();
        let mut bundle = sample_bundle(temp_dir.path());
        bundle.artifacts[0].relative_path = "../pack_sheet.txt".to_owned();

        let error = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::OpenPackSheet)
            .expect_err("invalid relative path should fail");

        assert_eq!(
            error,
            PackDayHostHandoffError::InvalidArtifactRelativePath {
                kind: PackDayHostHandoffKind::OpenPackSheet,
                relative_path: "../pack_sheet.txt".to_owned(),
            }
        );
    }

    #[test]
    fn execution_classifies_command_launch_failures() {
        let temp_dir = TestDirectory::new();
        let bundle = sample_bundle(temp_dir.path());
        let plan = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::RevealBundle)
            .expect("reveal plan should build");

        let error = execute_pack_day_host_handoff_plan_with(&plan, |_| {
            Err(io::Error::new(io::ErrorKind::NotFound, "open missing"))
        })
        .expect_err("launch failure should classify");

        assert!(matches!(
            error,
            PackDayHostHandoffError::CommandLaunch {
                kind: PackDayHostHandoffKind::RevealBundle,
                ..
            }
        ));
    }

    #[test]
    fn execution_classifies_nonzero_exit_failures() {
        let temp_dir = TestDirectory::new();
        let bundle = sample_bundle(temp_dir.path());
        let plan = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::RevealBundle)
            .expect("reveal plan should build");

        let error = execute_pack_day_host_handoff_plan_with(&plan, |_| {
            Ok(PackDayHostHandoffCommandResult::failed(
                Some(1),
                "finder unavailable",
            ))
        })
        .expect_err("nonzero exit should classify");

        assert_eq!(
            error,
            PackDayHostHandoffError::CommandFailed {
                kind: PackDayHostHandoffKind::RevealBundle,
                program: "open".to_owned(),
                exit_code: Some(1),
                stderr: "finder unavailable".to_owned(),
            }
        );
    }

    #[test]
    fn execution_accepts_successful_runs() {
        let temp_dir = TestDirectory::new();
        let bundle = sample_bundle(temp_dir.path());
        let plan = plan_pack_day_host_handoff(&bundle, PackDayHostHandoffKind::RevealBundle)
            .expect("reveal plan should build");

        let result = execute_pack_day_host_handoff_plan_with(&plan, |_| {
            Ok(PackDayHostHandoffCommandResult::succeeded())
        });

        assert_eq!(result, Ok(()));
    }
}
