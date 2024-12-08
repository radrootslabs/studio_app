import { db, device, dialog } from "$lib/client";
import { err, nostr_client, root_symbol } from "$lib/conf";
import { NDKKind } from "@nostr-dev-kit/ndk";
import type { NostrRelay } from "@radroots/models";
import {
    app_nostr_key, catch_err, ls, ndk, ndk_user, nostr_sync_prevent
} from "@radroots/svelte-lib";
import { fmt_tags_basis_nip99, ndk_event, ndk_event_metadata, nevent_encode, num_str } from "@radroots/utils";
import { get as get_store } from "svelte/store";
import { throw_err } from "./error";

export const nostr_sync_metadata = async (): Promise<void> => {
    try {
        const $ndk = get_store(ndk);
        const $ndk_user = get_store(ndk_user);
        const $app_nostr_key = get_store(app_nostr_key);
        const nostr_profile = await db.nostr_profile_get_one({
            public_key: $app_nostr_key
        });
        if (`err` in nostr_profile) return throw_err(nostr_profile);
        const ev_metadata = await ndk_event_metadata({
            $ndk,
            $ndk_user,
            metadata: nostr_profile.result
        });
        if (ev_metadata) await ev_metadata.publish();
    } catch (e) {
        await catch_err(e, `nostr_sync_metadata`);
    }
};

export const nostr_sync_classified = async (nostr_relays: NostrRelay[]): Promise<void> => {
    try {
        const $ndk = get_store(ndk);
        const $ndk_user = get_store(ndk_user);
        const trade_products_all = await db.trade_product_get({
            list: [`all`],
        });
        if (`err` in trade_products_all) return throw_err(trade_products_all);
        for (const trade_product of trade_products_all.results) {
            console.log(`sync trade_product.id `, trade_product.id)
            const trade_product_location_res = await db.location_gcs_get({
                list: [`on_trade_product`, { id: trade_product.id }],
            });
            if (`err` in trade_product_location_res) return throw_err(trade_product_location_res);
            const trade_product_location = trade_product_location_res.results[0];

            const media_upload_res = await db.media_upload_get({
                list: [`on_trade_product`, { id: trade_product.id }],
            });
            if (`err` in media_upload_res) return throw_err(media_upload_res);
            const ev = await ndk_event({
                $ndk,
                $ndk_user,
                basis: {
                    kind: NDKKind.Classified,
                    content: ``,
                    tags: fmt_tags_basis_nip99({
                        d_tag: trade_product.id,
                        client: nostr_client,
                        listing: {
                            key: trade_product.key,
                            category: trade_product.category,
                            title: trade_product.title,
                            summary: trade_product.summary,
                            process: trade_product.process,
                            lot: trade_product.lot,
                            profile: trade_product.profile,
                            year: num_str(trade_product.year),
                        },
                        quantity: {
                            amt: num_str(trade_product.qty_amt),
                            unit: trade_product.qty_unit,
                            label: trade_product.qty_label
                        },
                        price: {
                            amt: num_str(trade_product.price_amt),
                            currency: trade_product.price_currency,
                            qty_amt: num_str(trade_product.price_qty_amt),
                            qty_unit: trade_product.price_qty_unit,
                        },
                        location: {
                            city: trade_product_location.gc_name,
                            region: trade_product_location.gc_admin1_name,
                            region_code: trade_product_location.gc_admin1_id,
                            country: trade_product_location.gc_country_name,
                            country_code: trade_product_location.gc_country_id,
                            lat: trade_product_location.lat,
                            lng: trade_product_location.lng,
                            geohash: trade_product_location.geohash,
                        },
                        images: media_upload_res.results.length ? media_upload_res.results.map(i => ({ url: `${i.res_base}/${i.res_path}.${i.mime_type}` })) : undefined
                    }),
                },
            });
            if (ev) {
                ev.content = `radroots:[nostr:${nevent_encode({
                    id: ev.id,
                    author: ev.pubkey,
                    relays: nostr_relays.map(i => i.url),
                    kind: NDKKind.Classified,
                })}]`
                await ev.publish();
            }
        }
    } catch (e) {
        await catch_err(e, `nostr_sync_classified`);
    }
};

export const nostr_sync = async (): Promise<void> => {
    try {
        const $nostr_sync_prevent = get_store(nostr_sync_prevent);
        const $ls = get_store(ls);
        const $app_nostr_key = get_store(app_nostr_key);
        if ($nostr_sync_prevent) {
            const confirm = await dialog.confirm({
                message: `${$ls(`error.client.nostr_sync_disabled`)}`,
            });
            if (confirm) {
                nostr_sync_prevent.set(false);
                await nostr_sync();
            }
            return;
        }
        console.log(`nostr_sync start`)
        const nostr_relays = await db.nostr_relay_get({
            list: [`on_profile`, { public_key: $app_nostr_key }],
        });
        if (`err` in nostr_relays) return throw_err(nostr_relays);
        if (!nostr_relays.results.length) return throw_err(err.nostr.no_relays);
        //
        // sync
        await nostr_sync_metadata();
        await nostr_sync_classified(nostr_relays.results);
        console.log(`nostr_sync done`)
    } catch (e) {
        await catch_err(e, `nostr_sync`);
    }
};


export const nostr_tags_basis = (): string[][] => {
    const tags: string[][] = [];
    for (const tag of [`app${device.metadata?.version ? `/${device.metadata.version}` : ``}`]) tags.push([root_symbol, tag])
    return tags;
};
