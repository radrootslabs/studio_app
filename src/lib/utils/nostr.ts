import { db, dialog } from "$lib/client";
import { nostr_client, root_symbol } from "$lib/conf";
import { NDKKind } from "@nostr-dev-kit/ndk";
import { app_nostr_key, ndk, ndk_user, nostr_sync_prevent, t } from "@radroots/svelte-lib";
import { fmt_tags_basis_nip99, ndk_event, nevent_encode, num_str } from "@radroots/utils";
import { get as get_store } from "svelte/store";

export const nostr_sync = async (): Promise<void> => {
    try {
        const $t = get_store(t);
        const $nostr_sync_prevent = get_store(nostr_sync_prevent);
        const $app_nostr_key = get_store(app_nostr_key);

        if ($nostr_sync_prevent) {
            const confirm = await dialog.confirm({
                message: `${$t(`error.client.nostr_sync_disabled`)}`,
                cancel_label: `${$t(`common.cancel`)}`,
                ok_label: `${$t(`common.ok`)}`
            });
            if (confirm) {
                nostr_sync_prevent.set(false);
                await nostr_sync();
            }
            return;
        }

        const $ndk = get_store(ndk);
        const $ndk_user = get_store(ndk_user);

        const nostr_relays_active = await db.nostr_relay_get({
            list: [`on_profile`, { public_key: $app_nostr_key }],
        });
        if (`err` in nostr_relays_active) return; //@todo
        if (!nostr_relays_active.results.length) return; //@todo
        const trade_products_all = await db.trade_product_get({
            list: [`all`],
        });
        if (`err` in trade_products_all) return; //@todo
        for (const trade_product of trade_products_all.results) {
            const trade_product_location_res = await db.location_gcs_get({
                list: [`on_trade_product`, { id: trade_product.id }],
            });
            if (`err` in trade_product_location_res) continue; //@todo
            const trade_product_location = trade_product_location_res.results[0];

            const media_upload_res = await db.media_upload_get({
                list: [`on_trade_product`, { id: trade_product.id }],
            });
            if (`err` in media_upload_res) continue; //@todo

            const ev = await ndk_event({
                $ndk,
                $ndk_user,
                basis: {
                    kind: NDKKind.Classified,
                    content: ``,
                    tags: await fmt_tags_basis_nip99({
                        d_tag: trade_product.id,
                        client: nostr_client,
                        listing: {
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
                    relays: nostr_relays_active.results.map(i => i.url),
                    kind: NDKKind.Classified,
                })}]`
                await ev.publish();
            }
        }
    } catch (e) {
        console.log(`(error) nostr_sync `, e);
    }
};

export const nostr_tags_basis = (): string[][] => {
    const tags: string[][] = [];
    for (const tag of [`app/0.0.0`]) tags.push([root_symbol, tag])
    return tags;
};
