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
