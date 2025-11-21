import { _env } from "$lib/_env";

export const get_default_nostr_relays = (): string[] => {
    return Array.from(
        new Set(
            _env.DEFAULT_RELAYS.split(",")
                .map((url) => url.trim())
                .filter((url) => url.length > 0),
        ),
    );
};
