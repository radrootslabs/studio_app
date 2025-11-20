import app_pkg from "../../../../package.json" assert { type: "json" };

export const APP_STATE_BACKUP_VERSION = "1.0.0";

export const app_cfg = {
    version: app_pkg.version ?? "0.0.0",
    backup: {
        version: APP_STATE_BACKUP_VERSION
    }
} as const;