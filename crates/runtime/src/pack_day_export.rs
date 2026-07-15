use std::{
    fs, io,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use radroots_studio_app_view::{
    PackDayExportArtifact, PackDayExportArtifactKind, PackDayExportBundle, PackDayExportInstanceId,
    PackDayOutputSource,
};
use thiserror::Error;

use crate::AppRuntimeRoots;

pub const APP_EXPORTS_DIR_NAME: &str = "exports";
pub const PACK_DAY_EXPORTS_DIR_NAME: &str = "pack_day";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackDayExportDocument {
    pub kind: PackDayExportArtifactKind,
    pub absolute_path: PathBuf,
    pub contents: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedPackDayExportBundle {
    pub bundle: PackDayExportBundle,
    pub documents: Vec<PackDayExportDocument>,
}

impl PreparedPackDayExportBundle {
    pub fn artifact_path(&self, kind: PackDayExportArtifactKind) -> Option<&Path> {
        self.documents
            .iter()
            .find(|document| document.kind == kind)
            .map(|document| document.absolute_path.as_path())
    }

    pub fn artifact_contents(&self, kind: PackDayExportArtifactKind) -> Option<&str> {
        self.documents
            .iter()
            .find(|document| document.kind == kind)
            .map(|document| document.contents.as_str())
    }
}

#[derive(Debug, Error)]
pub enum PackDayExportWriteError {
    #[error("failed to create export directory {path}: {source}")]
    CreateDirectory { path: PathBuf, source: io::Error },
    #[error("failed to write export file {path}: {source}")]
    WriteFile { path: PathBuf, source: io::Error },
}

pub fn app_exports_root(roots: &AppRuntimeRoots) -> PathBuf {
    app_exports_root_from_data_root(roots.data.as_path())
}

pub fn app_exports_root_from_data_root(data_root: &Path) -> PathBuf {
    data_root.join(APP_EXPORTS_DIR_NAME)
}

pub fn prepare_pack_day_export_bundle(
    roots: &AppRuntimeRoots,
    source: &PackDayOutputSource,
    generated_at: DateTime<Utc>,
) -> PreparedPackDayExportBundle {
    prepare_pack_day_export_bundle_at_data_root(roots.data.as_path(), source, generated_at)
}

pub fn prepare_pack_day_export_bundle_at_data_root(
    data_root: &Path,
    source: &PackDayOutputSource,
    generated_at: DateTime<Utc>,
) -> PreparedPackDayExportBundle {
    let timestamp = format_bundle_timestamp(generated_at);
    let bundle_directory = app_exports_root_from_data_root(data_root)
        .join(PACK_DAY_EXPORTS_DIR_NAME)
        .join(source.fulfillment_window.fulfillment_window_id.to_string())
        .join(timestamp);
    let artifacts = Vec::from(PackDayExportArtifactKind::all_v1())
        .into_iter()
        .map(|kind| PackDayExportArtifact {
            kind,
            relative_path: kind.file_name().to_owned(),
        })
        .collect::<Vec<_>>();
    let bundle = PackDayExportBundle {
        fulfillment_window_id: source.fulfillment_window.fulfillment_window_id,
        export_instance_id: PackDayExportInstanceId::generate(),
        generated_at_utc: generated_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        bundle_directory: bundle_directory.to_string_lossy().into_owned(),
        artifacts,
    };
    let documents = bundle
        .artifacts
        .iter()
        .map(|artifact| PackDayExportDocument {
            kind: artifact.kind,
            absolute_path: bundle_directory.join(&artifact.relative_path),
            contents: match artifact.kind {
                PackDayExportArtifactKind::PackSheet => render_pack_sheet(source),
                PackDayExportArtifactKind::PickupRoster => render_pickup_roster(source),
                PackDayExportArtifactKind::CustomerLabels => render_customer_labels(source),
            },
        })
        .collect();

    PreparedPackDayExportBundle { bundle, documents }
}

pub fn write_prepared_pack_day_export_bundle(
    prepared: &PreparedPackDayExportBundle,
) -> Result<(), PackDayExportWriteError> {
    let bundle_directory = PathBuf::from(&prepared.bundle.bundle_directory);
    fs::create_dir_all(&bundle_directory).map_err(|source| {
        PackDayExportWriteError::CreateDirectory {
            path: bundle_directory,
            source,
        }
    })?;

    for document in &prepared.documents {
        fs::write(&document.absolute_path, &document.contents).map_err(|source| {
            PackDayExportWriteError::WriteFile {
                path: document.absolute_path.clone(),
                source,
            }
        })?;
    }

    Ok(())
}

fn format_bundle_timestamp(generated_at: DateTime<Utc>) -> String {
    generated_at.format("%Y%m%dT%H%M%SZ").to_string()
}

fn render_pack_sheet(source: &PackDayOutputSource) -> String {
    let mut lines = render_export_header("Pack day", source);
    lines.push(String::new());
    lines.push("Totals by product".to_owned());
    if source.totals_by_product.is_empty() {
        lines.push("- none".to_owned());
    } else {
        lines.extend(
            source
                .totals_by_product
                .iter()
                .map(|row| format!("- {} | {}", row.title, format_quantity(&row.quantity))),
        );
    }
    lines.push(String::new());
    lines.push("Pack list".to_owned());
    if source.pack_list.is_empty() {
        lines.push("- none".to_owned());
    } else {
        lines.extend(source.pack_list.iter().map(|row| {
            format!(
                "- {} | {} | {} | {} | {}",
                row.customer_display_name,
                row.order_number,
                row.order_state.storage_key(),
                row.title,
                format_quantity(&row.quantity)
            )
        }));
    }

    finalize_export_lines(lines)
}

fn render_pickup_roster(source: &PackDayOutputSource) -> String {
    let mut lines = render_export_header("Pickup roster", source);
    lines.push(String::new());
    lines.push("Orders".to_owned());
    if source.pickup_roster.is_empty() {
        lines.push("- none".to_owned());
    } else {
        lines.extend(source.pickup_roster.iter().map(|row| {
            format!(
                "- {} | {} | {}",
                row.customer_display_name,
                row.order_number,
                row.order_state.storage_key()
            )
        }));
    }

    finalize_export_lines(lines)
}

fn render_customer_labels(source: &PackDayOutputSource) -> String {
    let mut blocks = Vec::new();

    for row in &source.pickup_roster {
        let mut lines = vec![
            source.fulfillment_window.farm_display_name.clone(),
            row.customer_display_name.clone(),
            format!("Order: {}", row.order_number),
        ];
        if let Some(pickup_location_label) =
            source.fulfillment_window.pickup_location_label.as_ref()
        {
            lines.push(format!("Pickup: {pickup_location_label}"));
        }
        lines.push(format!(
            "Window: {} to {}",
            source.fulfillment_window.starts_at, source.fulfillment_window.ends_at
        ));
        blocks.push(lines.join("\n"));
    }

    if blocks.is_empty() {
        blocks.push(
            [
                source.fulfillment_window.farm_display_name.clone(),
                "No customer labels".to_owned(),
                format!(
                    "Window: {} to {}",
                    source.fulfillment_window.starts_at, source.fulfillment_window.ends_at
                ),
            ]
            .join("\n"),
        );
    }

    format!("{}\n", blocks.join("\n\n---\n\n"))
}

fn render_export_header(title: &str, source: &PackDayOutputSource) -> Vec<String> {
    let mut lines = vec![
        format!("Radroots {title}"),
        format!("Farm: {}", source.fulfillment_window.farm_display_name),
        format!(
            "Window: {} to {}",
            source.fulfillment_window.starts_at, source.fulfillment_window.ends_at
        ),
    ];
    if let Some(pickup_location_label) = source.fulfillment_window.pickup_location_label.as_ref() {
        lines.push(format!("Pickup location: {pickup_location_label}"));
    }
    lines
}

fn finalize_export_lines(lines: Vec<String>) -> String {
    format!("{}\n", lines.join("\n"))
}

fn format_quantity(quantity: &radroots_studio_app_view::PackDayOutputQuantity) -> String {
    let unit_label = quantity.unit_label.trim();
    if unit_label.is_empty() {
        quantity.value.to_string()
    } else {
        format!("{} {}", quantity.value, unit_label)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};
    use radroots_studio_app_view::{
        FarmId, FulfillmentWindowId, OrderId, PackDayExportArtifactKind,
        PackDayOutputCustomerOrder, PackDayOutputOrderState, PackDayOutputPackListEntry,
        PackDayOutputProductTotal, PackDayOutputQuantity, PackDayOutputSource, PackDayOutputWindow,
    };

    use super::{
        APP_EXPORTS_DIR_NAME, PACK_DAY_EXPORTS_DIR_NAME, app_exports_root,
        app_exports_root_from_data_root, prepare_pack_day_export_bundle,
        prepare_pack_day_export_bundle_at_data_root, write_prepared_pack_day_export_bundle,
    };
    use crate::AppRuntimeRoots;

    #[test]
    fn export_root_uses_existing_app_data_namespace() {
        let roots = AppRuntimeRoots::from_base_root("/Users/treesap/.radroots").namespaced_app();

        assert_eq!(
            app_exports_root(&roots),
            PathBuf::from("/Users/treesap/.radroots/data/apps/app").join(APP_EXPORTS_DIR_NAME)
        );
        assert_eq!(
            app_exports_root_from_data_root(roots.data.as_path()),
            PathBuf::from("/Users/treesap/.radroots/data/apps/app").join(APP_EXPORTS_DIR_NAME)
        );
    }

    #[test]
    fn prepared_bundle_freezes_path_shape_and_file_names() {
        let roots = AppRuntimeRoots::from_base_root("/Users/treesap/.radroots").namespaced_app();
        let source = sample_source();
        let generated_at = Utc
            .with_ymd_and_hms(2026, 4, 23, 15, 0, 0)
            .single()
            .expect("timestamp should build");

        let prepared = prepare_pack_day_export_bundle(&roots, &source, generated_at);

        assert_eq!(
            prepared.bundle.bundle_directory,
            roots
                .data
                .join(APP_EXPORTS_DIR_NAME)
                .join(PACK_DAY_EXPORTS_DIR_NAME)
                .join(source.fulfillment_window.fulfillment_window_id.to_string())
                .join("20260423T150000Z")
                .to_string_lossy()
                .into_owned()
        );
        assert_eq!(prepared.bundle.artifact_count(), 3);
        assert_eq!(
            prepared.bundle.artifacts[0].relative_path,
            PackDayExportArtifactKind::PackSheet.file_name()
        );
        assert_eq!(
            prepared.bundle.artifacts[1].relative_path,
            PackDayExportArtifactKind::PickupRoster.file_name()
        );
        assert_eq!(
            prepared.bundle.artifacts[2].relative_path,
            PackDayExportArtifactKind::CustomerLabels.file_name()
        );
        assert_eq!(
            prepared
                .artifact_path(PackDayExportArtifactKind::CustomerLabels)
                .expect("customer labels path should exist"),
            Path::new(&prepared.bundle.bundle_directory).join("customer_labels.txt")
        );
    }

    #[test]
    fn prepared_bundle_renders_text_first_artifacts_from_output_source() {
        let roots = AppRuntimeRoots::from_base_root("/Users/treesap/.radroots").namespaced_app();
        let source = sample_source();
        let generated_at = Utc
            .with_ymd_and_hms(2026, 4, 23, 15, 0, 0)
            .single()
            .expect("timestamp should build");

        let prepared = prepare_pack_day_export_bundle(&roots, &source, generated_at);

        assert_eq!(
            prepared
                .artifact_contents(PackDayExportArtifactKind::PackSheet)
                .expect("pack sheet should render"),
            "Radroots Pack day\nFarm: Willow farm\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\nPickup location: North barn\n\nTotals by product\n- Carrots | 3 bunches\n- Salad mix | 2 bags\n\nPack list\n- Casey | R-1001 | scheduled | Salad mix | 2 bags\n- Taylor | R-1002 | packed | Carrots | 3 bunches\n"
        );
        assert_eq!(
            prepared
                .artifact_contents(PackDayExportArtifactKind::PickupRoster)
                .expect("pickup roster should render"),
            "Radroots Pickup roster\nFarm: Willow farm\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\nPickup location: North barn\n\nOrders\n- Casey | R-1001 | scheduled\n- Taylor | R-1002 | packed\n"
        );
        assert_eq!(
            prepared
                .artifact_contents(PackDayExportArtifactKind::CustomerLabels)
                .expect("customer labels should render"),
            "Willow farm\nCasey\nOrder: R-1001\nPickup: North barn\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\n\n---\n\nWillow farm\nTaylor\nOrder: R-1002\nPickup: North barn\nWindow: 2026-04-23T16:00:00Z to 2026-04-23T19:00:00Z\n"
        );
    }

    #[test]
    fn prepared_bundle_can_use_the_runtime_data_root_directly() {
        let data_root = PathBuf::from("/Users/treesap/.radroots/data/apps/app");
        let source = sample_source();
        let generated_at = Utc
            .with_ymd_and_hms(2026, 4, 23, 15, 0, 0)
            .single()
            .expect("timestamp should build");

        let prepared =
            prepare_pack_day_export_bundle_at_data_root(data_root.as_path(), &source, generated_at);

        assert_eq!(
            prepared.bundle.bundle_directory,
            data_root
                .join(APP_EXPORTS_DIR_NAME)
                .join(PACK_DAY_EXPORTS_DIR_NAME)
                .join(source.fulfillment_window.fulfillment_window_id.to_string())
                .join("20260423T150000Z")
                .to_string_lossy()
                .into_owned()
        );
    }

    #[test]
    fn prepared_bundle_writes_files_to_disk() {
        let roots = AppRuntimeRoots::from_base_root(temp_root("write_bundle")).namespaced_app();
        let source = sample_source();
        let generated_at = Utc
            .with_ymd_and_hms(2026, 4, 23, 15, 0, 0)
            .single()
            .expect("timestamp should build");
        let prepared = prepare_pack_day_export_bundle(&roots, &source, generated_at);

        write_prepared_pack_day_export_bundle(&prepared).expect("bundle should write");

        for document in &prepared.documents {
            assert_eq!(
                fs::read_to_string(&document.absolute_path).expect("artifact should write"),
                document.contents
            );
        }

        cleanup_temp_root(&roots);
    }

    fn sample_source() -> PackDayOutputSource {
        let farm_id = FarmId::generate();
        let fulfillment_window_id = FulfillmentWindowId::generate();
        PackDayOutputSource {
            fulfillment_window: PackDayOutputWindow {
                fulfillment_window_id,
                farm_id,
                farm_display_name: "Willow farm".to_owned(),
                pickup_location_label: Some("North barn".to_owned()),
                starts_at: "2026-04-23T16:00:00Z".to_owned(),
                ends_at: "2026-04-23T19:00:00Z".to_owned(),
            },
            totals_by_product: vec![
                PackDayOutputProductTotal {
                    title: "Carrots".to_owned(),
                    quantity: PackDayOutputQuantity::new(3, "bunches"),
                },
                PackDayOutputProductTotal {
                    title: "Salad mix".to_owned(),
                    quantity: PackDayOutputQuantity::new(2, "bags"),
                },
            ],
            pack_list: vec![
                PackDayOutputPackListEntry {
                    order_id: OrderId::generate(),
                    order_number: "R-1001".to_owned(),
                    customer_display_name: "Casey".to_owned(),
                    order_state: PackDayOutputOrderState::Scheduled,
                    title: "Salad mix".to_owned(),
                    quantity: PackDayOutputQuantity::new(2, "bags"),
                },
                PackDayOutputPackListEntry {
                    order_id: OrderId::generate(),
                    order_number: "R-1002".to_owned(),
                    customer_display_name: "Taylor".to_owned(),
                    order_state: PackDayOutputOrderState::Packed,
                    title: "Carrots".to_owned(),
                    quantity: PackDayOutputQuantity::new(3, "bunches"),
                },
            ],
            pickup_roster: vec![
                PackDayOutputCustomerOrder {
                    order_id: OrderId::generate(),
                    order_number: "R-1001".to_owned(),
                    customer_display_name: "Casey".to_owned(),
                    order_state: PackDayOutputOrderState::Scheduled,
                },
                PackDayOutputCustomerOrder {
                    order_id: OrderId::generate(),
                    order_number: "R-1002".to_owned(),
                    customer_display_name: "Taylor".to_owned(),
                    order_state: PackDayOutputOrderState::Packed,
                },
            ],
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "radroots_studio_app_pack_day_export_{label}_{unique}"
        ))
    }

    fn cleanup_temp_root(roots: &AppRuntimeRoots) {
        if let Some(base) = roots.data.ancestors().nth(3) {
            let _ = fs::remove_dir_all(base);
        }
    }
}
