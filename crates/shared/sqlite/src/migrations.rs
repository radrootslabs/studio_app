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
