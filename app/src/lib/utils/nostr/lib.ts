import { PUBLIC_NOSTR_RELAY_DEFAULTS } from "$env/static/public";

export const get_default_nostr_relays = (): string[] => {
    return Array.from(
        new Set(
            PUBLIC_NOSTR_RELAY_DEFAULTS.split(",")
                .map((url) => url.trim())
                .filter((url) => url.length > 0),
        ),
    );
};
