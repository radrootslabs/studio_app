use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

use radroots_studio_app_models::{
    PackDayExportArtifactKind, PackDayExportBundle, PackDayExportInstanceId, PackDayPrintKind,
    PackDayPrintLabelStock,
};
use thiserror::Error;

const CUSTOMER_LABEL_PREPARED_ASSET_ROOT: &str = "radroots_studio_app_pack_day_print";
const LETTER_MEDIA_OPTION: &str = "media=Letter";
const LETTER_PAGE_WIDTH_POINTS: u16 = 612;
const LETTER_PAGE_HEIGHT_POINTS: u16 = 792;
const AVERY_5160_LABELS_PER_ROW: usize = 3;
const AVERY_5160_LABEL_ROWS_PER_PAGE: usize = 10;
const AVERY_5160_LABELS_PER_PAGE: usize =
    AVERY_5160_LABELS_PER_ROW * AVERY_5160_LABEL_ROWS_PER_PAGE;
const AVERY_5160_COLUMN_PITCH_POINTS: f32 = 198.0;
const AVERY_5160_ROW_PITCH_POINTS: f32 = 72.0;
const AVERY_5160_LEFT_MARGIN_POINTS: f32 = 13.5;
const AVERY_5160_TOP_MARGIN_POINTS: f32 = 36.0;
const AVERY_5160_PAGE_HEIGHT_POINTS: f32 = 792.0;
const AVERY_5160_TEXT_LEFT_PADDING_POINTS: f32 = 9.0;
const AVERY_5160_TEXT_TOP_PADDING_POINTS: f32 = 11.0;
const AVERY_5160_TEXT_LEADING_POINTS: f32 = 10.0;
const AVERY_5160_TEXT_FONT_SIZE_POINTS: f32 = 9.0;
const AVERY_5160_MAX_CHARS_PER_LINE: usize = 32;
const AVERY_5160_MAX_LINES_PER_LABEL: usize = 6;

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
    #[error("failed to read pack day print source artifact {path} for {kind:?}: {source}")]
    ReadSourceArtifact {
        kind: PackDayPrintKind,
        path: PathBuf,
        source: io::Error,
    },
    #[error("failed to create prepared print asset directory {path} for {kind:?}: {source}")]
    CreatePreparedAssetDirectory {
        kind: PackDayPrintKind,
        path: PathBuf,
        source: io::Error,
    },
    #[error("failed to write prepared print asset {path} for {kind:?}: {source}")]
    WritePreparedAsset {
        kind: PackDayPrintKind,
        path: PathBuf,
        source: io::Error,
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
            (
                Self::ReadSourceArtifact {
                    kind: left_kind,
                    path: left_path,
                    source: left_source,
                },
                Self::ReadSourceArtifact {
                    kind: right_kind,
                    path: right_path,
                    source: right_source,
                },
            ) => {
                left_kind == right_kind
                    && left_path == right_path
                    && io_errors_match(left_source, right_source)
            }
            (
                Self::CreatePreparedAssetDirectory {
                    kind: left_kind,
                    path: left_path,
                    source: left_source,
                },
                Self::CreatePreparedAssetDirectory {
                    kind: right_kind,
                    path: right_path,
                    source: right_source,
                },
            ) => {
                left_kind == right_kind
                    && left_path == right_path
                    && io_errors_match(left_source, right_source)
            }
            (
                Self::WritePreparedAsset {
                    kind: left_kind,
                    path: left_path,
                    source: left_source,
                },
                Self::WritePreparedAsset {
                    kind: right_kind,
                    path: right_path,
                    source: right_source,
                },
            ) => {
                left_kind == right_kind
                    && left_path == right_path
                    && io_errors_match(left_source, right_source)
            }
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

fn io_errors_match(left: &io::Error, right: &io::Error) -> bool {
    left.kind() == right.kind() && left.to_string() == right.to_string()
}

pub(crate) fn cleanup_prepared_customer_label_asset_root() -> io::Result<()> {
    cleanup_prepared_customer_label_assets_at_path(prepared_customer_label_asset_root())
}

pub(crate) fn cleanup_prepared_customer_label_assets_for_export_instance(
    export_instance_id: PackDayExportInstanceId,
) -> io::Result<()> {
    cleanup_prepared_customer_label_assets_at_path(
        prepared_customer_label_asset_directory_for_export_instance(export_instance_id),
    )
}

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

    let (target_path, command_args) = match kind {
        PackDayPrintKind::PrintPackSheet => {
            let target_path =
                resolve_bundle_artifact_path(bundle, PackDayExportArtifactKind::PackSheet, kind)?;
            let command_args = vec![target_path.to_string_lossy().into_owned()];
            (target_path, command_args)
        }
        PackDayPrintKind::PrintPickupRoster => {
            let target_path = resolve_bundle_artifact_path(
                bundle,
                PackDayExportArtifactKind::PickupRoster,
                kind,
            )?;
            let command_args = vec![target_path.to_string_lossy().into_owned()];
            (target_path, command_args)
        }
        PackDayPrintKind::PrintCustomerLabels => {
            let target_path = prepare_customer_label_stock_asset(
                bundle,
                PackDayPrintLabelStock::Avery5160Letter30Up,
            )?;
            let command_args = vec![
                "-o".to_owned(),
                LETTER_MEDIA_OPTION.to_owned(),
                target_path.to_string_lossy().into_owned(),
            ];
            (target_path, command_args)
        }
    };

    Ok(PackDayPrintCommandPlan {
        kind,
        target_path,
        command_program: "lp",
        command_args,
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

fn prepare_customer_label_stock_asset(
    bundle: &PackDayExportBundle,
    stock: PackDayPrintLabelStock,
) -> Result<PathBuf, PackDayPrintError> {
    let kind = PackDayPrintKind::PrintCustomerLabels;
    let source_path =
        resolve_bundle_artifact_path(bundle, PackDayExportArtifactKind::CustomerLabels, kind)?;
    let source_contents = fs::read_to_string(&source_path).map_err(|source| {
        PackDayPrintError::ReadSourceArtifact {
            kind,
            path: source_path.clone(),
            source,
        }
    })?;
    let target_directory = prepared_customer_label_asset_directory(bundle);
    fs::create_dir_all(&target_directory).map_err(|source| {
        PackDayPrintError::CreatePreparedAssetDirectory {
            kind,
            path: target_directory.clone(),
            source,
        }
    })?;
    let target_path = prepared_customer_label_asset_path(bundle, stock);
    let prepared_asset = render_customer_label_stock_asset(&source_contents, stock);
    fs::write(&target_path, prepared_asset).map_err(|source| {
        let _ =
            cleanup_prepared_customer_label_assets_for_export_instance(bundle.export_instance_id);
        PackDayPrintError::WritePreparedAsset {
            kind,
            path: target_path.clone(),
            source,
        }
    })?;

    Ok(target_path)
}

pub(crate) fn prepared_customer_label_asset_root() -> PathBuf {
    let root = std::env::temp_dir().join(CUSTOMER_LABEL_PREPARED_ASSET_ROOT);

    #[cfg(test)]
    {
        root.join(format!("{:?}", std::thread::current().id()))
    }

    #[cfg(not(test))]
    {
        root
    }
}

fn cleanup_prepared_customer_label_assets_at_path(path: PathBuf) -> io::Result<()> {
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn prepared_customer_label_asset_directory_for_export_instance(
    export_instance_id: PackDayExportInstanceId,
) -> PathBuf {
    prepared_customer_label_asset_root().join(export_instance_id.to_string())
}

fn prepared_customer_label_asset_directory(bundle: &PackDayExportBundle) -> PathBuf {
    prepared_customer_label_asset_directory_for_export_instance(bundle.export_instance_id)
}

fn prepared_customer_label_asset_path(
    bundle: &PackDayExportBundle,
    stock: PackDayPrintLabelStock,
) -> PathBuf {
    prepared_customer_label_asset_directory(bundle)
        .join(format!("customer_labels_{}.ps", stock.storage_key()))
}

fn render_customer_label_stock_asset(
    source_contents: &str,
    stock: PackDayPrintLabelStock,
) -> String {
    match stock {
        PackDayPrintLabelStock::Avery5160Letter30Up => {
            render_avery_5160_customer_labels_postscript(parse_customer_label_blocks(
                source_contents,
            ))
        }
    }
}

fn parse_customer_label_blocks(source_contents: &str) -> Vec<Vec<String>> {
    let blocks = source_contents
        .trim()
        .split("\n\n---\n\n")
        .filter_map(|block| {
            let lines = block
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            (!lines.is_empty()).then_some(lines)
        })
        .collect::<Vec<_>>();

    if blocks.is_empty() {
        vec![vec!["No customer labels".to_owned()]]
    } else {
        blocks
    }
}

fn render_avery_5160_customer_labels_postscript(blocks: Vec<Vec<String>>) -> String {
    let page_count = blocks.len().div_ceil(AVERY_5160_LABELS_PER_PAGE);
    let mut rendered = String::new();

    let _ = writeln!(&mut rendered, "%!PS-Adobe-3.0");
    let _ = writeln!(&mut rendered, "%%Creator: radroots_studio_app");
    let _ = writeln!(&mut rendered, "%%Pages: {page_count}");
    let _ = writeln!(
        &mut rendered,
        "%%BoundingBox: 0 0 {LETTER_PAGE_WIDTH_POINTS} {LETTER_PAGE_HEIGHT_POINTS}"
    );
    let _ = writeln!(
        &mut rendered,
        "%%DocumentMedia: Letter {LETTER_PAGE_WIDTH_POINTS} {LETTER_PAGE_HEIGHT_POINTS} 0 () ()"
    );
    let _ = writeln!(&mut rendered, "%%EndComments");

    for (page_index, page_blocks) in blocks.chunks(AVERY_5160_LABELS_PER_PAGE).enumerate() {
        let page_number = page_index + 1;
        let _ = writeln!(&mut rendered, "%%Page: {page_number} {page_number}");
        let _ = writeln!(
            &mut rendered,
            "<< /PageSize [{LETTER_PAGE_WIDTH_POINTS} {LETTER_PAGE_HEIGHT_POINTS}] >> setpagedevice"
        );
        let _ = writeln!(
            &mut rendered,
            "/Courier findfont {} scalefont setfont",
            AVERY_5160_TEXT_FONT_SIZE_POINTS
        );

        for (slot_index, block) in page_blocks.iter().enumerate() {
            let row = slot_index / AVERY_5160_LABELS_PER_ROW;
            let column = slot_index % AVERY_5160_LABELS_PER_ROW;
            let left = AVERY_5160_LEFT_MARGIN_POINTS
                + (column as f32 * AVERY_5160_COLUMN_PITCH_POINTS)
                + AVERY_5160_TEXT_LEFT_PADDING_POINTS;
            let top = AVERY_5160_PAGE_HEIGHT_POINTS
                - AVERY_5160_TOP_MARGIN_POINTS
                - (row as f32 * AVERY_5160_ROW_PITCH_POINTS)
                - AVERY_5160_TEXT_TOP_PADDING_POINTS;

            for (line_index, line) in wrap_customer_label_block(block).into_iter().enumerate() {
                let baseline = top - (line_index as f32 * AVERY_5160_TEXT_LEADING_POINTS);
                let escaped = escape_postscript_text(&line);
                let _ = writeln!(
                    &mut rendered,
                    "{left:.2} {baseline:.2} moveto ({escaped}) show"
                );
            }
        }

        let _ = writeln!(&mut rendered, "showpage");
    }

    rendered
}

fn wrap_customer_label_block(lines: &[String]) -> Vec<String> {
    let mut wrapped = Vec::new();

    for line in lines {
        for segment in wrap_customer_label_line(line) {
            if wrapped.len() == AVERY_5160_MAX_LINES_PER_LABEL {
                return wrapped;
            }
            wrapped.push(segment);
        }
    }

    wrapped
}

fn wrap_customer_label_line(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut wrapped = Vec::new();
    let mut current = String::new();

    for word in trimmed.split_whitespace() {
        let word_len = word.chars().count();
        if word_len > AVERY_5160_MAX_CHARS_PER_LINE {
            if !current.is_empty() {
                wrapped.push(std::mem::take(&mut current));
            }
            push_chunked_word(word, &mut wrapped);
            continue;
        }

        if current.is_empty() {
            current.push_str(word);
            continue;
        }

        if current.chars().count() + 1 + word_len <= AVERY_5160_MAX_CHARS_PER_LINE {
            current.push(' ');
            current.push_str(word);
            continue;
        }

        wrapped.push(std::mem::take(&mut current));
        current.push_str(word);
    }

    if !current.is_empty() {
        wrapped.push(current);
    }

    wrapped
}

fn push_chunked_word(word: &str, wrapped: &mut Vec<String>) {
    let mut chunk = String::new();

    for character in word.chars() {
        if chunk.chars().count() == AVERY_5160_MAX_CHARS_PER_LINE {
            wrapped.push(std::mem::take(&mut chunk));
        }
        chunk.push(character);
    }

    if !chunk.is_empty() {
        wrapped.push(chunk);
    }
}

fn escape_postscript_text(line: &str) -> String {
    let mut escaped = String::with_capacity(line.len());

    for character in line.chars() {
        match character {
            '(' | ')' | '\\' => {
                escaped.push('\\');
                escaped.push(character);
            }
            '\n' | '\r' | '\t' => escaped.push(' '),
            _ => escaped.push(character),
        }
    }

    escaped
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
        cleanup_prepared_customer_label_asset_root, execute_pack_day_print_plan_with,
        plan_pack_day_print, prepared_customer_label_asset_directory,
        prepared_customer_label_asset_path, prepared_customer_label_asset_root,
        PackDayPrintCommandResult, PackDayPrintError, LETTER_MEDIA_OPTION,
    };
    use radroots_studio_app_models::{
        PackDayExportArtifact, PackDayExportArtifactKind, PackDayExportBundle,
        PackDayExportInstanceId, PackDayPrintKind, PackDayPrintLabelStock,
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
    fn customer_labels_plan_derives_a_stock_aware_asset_outside_the_export_bundle() {
        let temp_dir = TestDirectory::new();
        let source_path = temp_dir.path().join("customer_labels.txt");
        fs::write(
            &source_path,
            "Willow farm\nCasey\nOrder: R-1001\nPickup: North barn\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\n\n---\n\nWillow farm\nTaylor\nOrder: R-1002\nPickup: North barn\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\n",
        )
        .expect("customer labels should write");
        let bundle = sample_bundle(temp_dir.path());
        let prepared_path = prepared_customer_label_asset_path(
            &bundle,
            PackDayPrintLabelStock::Avery5160Letter30Up,
        );

        let plan = plan_pack_day_print(&bundle, PackDayPrintKind::PrintCustomerLabels)
            .expect("customer labels plan should build");

        assert_eq!(plan.kind, PackDayPrintKind::PrintCustomerLabels);
        assert_eq!(plan.target_path, prepared_path.clone());
        assert_eq!(plan.command_program, "lp");
        assert_eq!(
            plan.command_args,
            vec![
                "-o".to_owned(),
                LETTER_MEDIA_OPTION.to_owned(),
                prepared_path.to_string_lossy().into_owned()
            ]
        );
        assert!(plan.target_path.is_file());
        assert!(!plan.target_path.starts_with(temp_dir.path()));
        assert!(plan
            .target_path
            .to_string_lossy()
            .contains(bundle.export_instance_id.to_string().as_str()));
        assert_eq!(
            fs::read_to_string(&source_path).expect("source labels should stay untouched"),
            "Willow farm\nCasey\nOrder: R-1001\nPickup: North barn\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\n\n---\n\nWillow farm\nTaylor\nOrder: R-1002\nPickup: North barn\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\n"
        );

        let prepared_contents =
            fs::read_to_string(&prepared_path).expect("prepared labels should render");
        assert!(prepared_contents.contains("%!PS-Adobe-3.0"));
        assert!(prepared_contents.contains("%%Pages: 1"));
        assert!(prepared_contents.contains("%%DocumentMedia: Letter 612 792 0 () ()"));
        assert!(prepared_contents.contains("<< /PageSize [612 792] >> setpagedevice"));
        assert!(prepared_contents.contains("(Casey) show"));
        assert!(prepared_contents.contains("(Taylor) show"));
        assert!(
            prepared_contents.contains("(Order: R-1001) show")
                || prepared_contents.contains("(Order: R-1002) show")
        );

        let _ = fs::remove_dir_all(prepared_customer_label_asset_directory(&bundle));
    }

    #[test]
    fn customer_label_stock_assets_are_scoped_by_export_instance_id() {
        let temp_dir = TestDirectory::new();
        fs::write(
            temp_dir.path().join("customer_labels.txt"),
            "Willow farm\nCasey\nOrder: R-1001\n",
        )
        .expect("customer labels should write");
        let bundle = sample_bundle(temp_dir.path());
        let other_bundle = PackDayExportBundle {
            export_instance_id: PackDayExportInstanceId::from(Uuid::new_v4()),
            ..bundle.clone()
        };

        let plan = plan_pack_day_print(&bundle, PackDayPrintKind::PrintCustomerLabels)
            .expect("first customer labels plan should build");
        let other_plan = plan_pack_day_print(&other_bundle, PackDayPrintKind::PrintCustomerLabels)
            .expect("second customer labels plan should build");

        assert_ne!(plan.target_path, other_plan.target_path);
        assert!(plan.target_path.is_file());
        assert!(other_plan.target_path.is_file());

        let _ = fs::remove_dir_all(prepared_customer_label_asset_directory(&bundle));
        let _ = fs::remove_dir_all(prepared_customer_label_asset_directory(&other_bundle));
    }

    #[test]
    fn customer_label_stock_preparation_classifies_directory_creation_failures() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_bundle(temp_dir.path());
        let prepared_directory = prepared_customer_label_asset_directory(&bundle);
        if let Some(parent) = prepared_directory.parent() {
            fs::create_dir_all(parent).expect("prepared asset parent should create");
        }
        fs::write(&prepared_directory, "blocked").expect("blocking file should write");

        let error = plan_pack_day_print(&bundle, PackDayPrintKind::PrintCustomerLabels)
            .expect_err("prepared directory failure should surface");

        match error {
            PackDayPrintError::CreatePreparedAssetDirectory { kind, path, source } => {
                assert_eq!(kind, PackDayPrintKind::PrintCustomerLabels);
                assert_eq!(path, prepared_directory);
                assert_eq!(source.kind(), io::ErrorKind::AlreadyExists);
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let _ = fs::remove_file(prepared_directory);
    }

    #[test]
    fn customer_label_stock_preparation_classifies_write_failures() {
        let temp_dir = TestDirectory::new();
        write_artifact(temp_dir.path(), "customer_labels.txt");
        let bundle = sample_bundle(temp_dir.path());
        let prepared_directory = prepared_customer_label_asset_directory(&bundle);
        fs::create_dir_all(&prepared_directory).expect("prepared directory should create");
        let prepared_path = prepared_customer_label_asset_path(
            &bundle,
            PackDayPrintLabelStock::Avery5160Letter30Up,
        );
        fs::create_dir_all(&prepared_path).expect("prepared asset directory should block writes");

        let error = plan_pack_day_print(&bundle, PackDayPrintKind::PrintCustomerLabels)
            .expect_err("prepared asset write failure should surface");

        match error {
            PackDayPrintError::WritePreparedAsset { kind, path, source } => {
                assert_eq!(kind, PackDayPrintKind::PrintCustomerLabels);
                assert_eq!(path, prepared_path);
                assert_eq!(source.kind(), io::ErrorKind::IsADirectory);
            }
            other => panic!("unexpected error: {other:?}"),
        }

        assert!(!prepared_directory.exists());
    }

    #[test]
    fn cleanup_prepared_customer_label_asset_root_removes_existing_directories() {
        let root = prepared_customer_label_asset_root();
        let stale_directory = root.join(PackDayExportInstanceId::new().to_string());
        fs::create_dir_all(&stale_directory).expect("stale prepared directory should create");
        fs::write(stale_directory.join("stale.ps"), "stale").expect("stale asset should write");

        cleanup_prepared_customer_label_asset_root()
            .expect("prepared customer label asset root should clean");

        assert!(!root.exists());
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
        assert!(execute_pack_day_print_plan_with(&plan, |_| {
            Ok(PackDayPrintCommandResult::succeeded())
        })
        .is_ok());
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
