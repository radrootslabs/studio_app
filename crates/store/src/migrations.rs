struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../migrations/0002_activity_journal.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("../migrations/0003_account_surface_activation.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("../migrations/0004_account_farm_setup.sql"),
    },
    Migration {
        version: 5,
        sql: include_str!("../migrations/0005_products_workflow.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("../migrations/0006_farm_rules_workspace.sql"),
    },
    Migration {
        version: 7,
        sql: include_str!("../migrations/0007_activity_farm_settings_section.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("../migrations/0008_orders_and_pack_day.sql"),
    },
    Migration {
        version: 9,
        sql: include_str!("../migrations/0009_buyer_marketplace.sql"),
    },
    Migration {
        version: 10,
        sql: include_str!("../migrations/0010_sync_contract_alignment.sql"),
    },
    Migration {
        version: 11,
        sql: include_str!("../migrations/0011_reminders_and_recovery.sql"),
    },
    Migration {
        version: 12,
        sql: include_str!("../migrations/0012_local_interop_imports.sql"),
    },
    Migration {
        version: 13,
        sql: include_str!("../migrations/0013_local_interop_projection_cursor.sql"),
    },
    Migration {
        version: 14,
        sql: include_str!("../migrations/0014_buyer_order_coordination.sql"),
    },
    Migration {
        version: 15,
        sql: include_str!("../migrations/0015_buyer_order_listing_identity.sql"),
    },
    Migration {
        version: 16,
        sql: include_str!("../migrations/0016_deterministic_outbox.sql"),
    },
    Migration {
        version: 17,
        sql: include_str!("../migrations/0017_product_category.sql"),
    },
    Migration {
        version: 18,
        sql: include_str!("../migrations/0018_listing_relay_provenance.sql"),
    },
    Migration {
        version: 19,
        sql: include_str!("../migrations/0019_relay_ingest_freshness.sql"),
    },
    Migration {
        version: 20,
        sql: include_str!("../migrations/0020_declined_order_status.sql"),
    },
    Migration {
        version: 21,
        sql: include_str!("../migrations/0021_local_interop_signed_event_evidence.sql"),
    },
];

pub fn latest_schema_version() -> u32 {
    MIGRATIONS.last().map_or(0, |migration| migration.version)
}

pub fn pending_migrations(current_version: u32) -> impl Iterator<Item = (u32, &'static str)> {
    MIGRATIONS
        .iter()
        .filter(move |migration| migration.version > current_version)
        .map(|migration| (migration.version, migration.sql))
}
