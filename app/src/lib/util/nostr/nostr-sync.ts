import { ls } from "$lib/locale/i18n";
import { db, gui, nostre } from "$lib/util";
import { cfg_nostr_client } from "$lib/util/conf";
import { NDKKind } from "@nostr-dev-kit/ndk";
import { get_store, handle_err, key_nostr, ndk, ndk_user, nostr_sync_prevent } from "@radroots/lib-app";
import type { NostrRelay } from "@radroots/models";
import { ndk_event, ndk_event_metadata, num_str, throw_err } from "@radroots/util";
import { err } from "../err";

export const nostr_sync_metadata = async (): Promise<void> => {
    try {
        const $ndk = get_store(ndk);
        const $ndk_user = get_store(ndk_user);
        const $key_nostr = get_store(key_nostr);
        const tbl_nostr_profile = await db.nostr_profile_read({
            public_key: $key_nostr
        });
        if (`err` in tbl_nostr_profile) return throw_err(tbl_nostr_profile);
        const ev_metadata = await ndk_event_metadata({
            $ndk,
            $ndk_user,
            metadata: tbl_nostr_profile.result
        });
        if (ev_metadata) await ev_metadata.publish();
    } catch (e) {
        await handle_err(e, `nostr_sync_metadata`);
    }
};

export const nostr_sync_classified = async (nostr_relays: NostrRelay[]): Promise<void> => {
    try {
        const $ndk = get_store(ndk);
        const $ndk_user = get_store(ndk_user);
        const tbl_trade_products = await db.trade_product_read_list();
        if (`err` in tbl_trade_products) return throw_err(tbl_trade_products);
        for (const trade_product of tbl_trade_products.results) {
            const tbl_location_gcss = await db.location_gcs_read_list({
                table: [`on_trade_product`, { id: trade_product.id }],
            });
            if (`err` in tbl_location_gcss) return throw_err(tbl_location_gcss);
            const trade_product_location = tbl_location_gcss.results[0];
            const tbl_media_uploads = await db.media_image_read_list({
                table: [`on_trade_product`, { id: trade_product.id }],
            });
            if (`err` in tbl_media_uploads) return throw_err(tbl_media_uploads);
            const ev = await ndk_event({
                $ndk,
                $ndk_user,
                basis: {
                    kind: NDKKind.Classified,
                    content: ``,
                    tags: nostre.fmt_tags_basis_nip99({
                        d_tag: trade_product.id,
                        client: cfg_nostr_client,
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
                        images: tbl_media_uploads.results.length ? tbl_media_uploads.results.map(i => ({ url: `${i.res_base}/${i.res_path}.${i.mime_type}` })) : undefined
                    }),
                },
            });
            if (ev) {
                ev.content = `radroots:[nostr:${nostre.nevent_encode({
                    id: ev.id,
                    author: ev.pubkey,
                    relays: nostr_relays.map(i => i.url),
                    kind: NDKKind.Classified,
                })}]`
                await ev.publish();
            }
        }
    } catch (e) {
        await handle_err(e, `nostr_sync_classified`);
    }
};

export const nostr_sync = async (): Promise<void> => {
    try {
        const $nostr_sync_prevent = get_store(nostr_sync_prevent);
        const $lls = get_store(ls);
        const $key_nostr = get_store(key_nostr);
        if ($nostr_sync_prevent) {
            const confirm = await gui.confirm({
                message: `${$lls(`error.client.nostr_sync_disabled`)}`,
            });
            if (confirm) {
                nostr_sync_prevent.set(false);
                await nostr_sync();
            }
            return;
        }
        const tbl_nostr_relays = await db.nostr_relay_read_list({
            table: [`on_profile`, { public_key: $key_nostr }],
        });
        if (`err` in tbl_nostr_relays) return throw_err(tbl_nostr_relays);
        if (!tbl_nostr_relays.results.length) return throw_err(err.nostr.no_relays);
        await nostr_sync_metadata();
        await nostr_sync_classified(tbl_nostr_relays.results);
    } catch (e) {
        await handle_err(e, `nostr_sync`);
    }
};
