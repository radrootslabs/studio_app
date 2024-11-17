import type { NDKEvent } from "@nostr-dev-kit/ndk";
import type { ExtendedBaseType, NDKEventStore } from "@nostr-dev-kit/ndk-svelte";
import type { LocationGcs, MediaUpload, TradeProduct } from "@radroots/models";

export type TradeProductBundle = {
    trade_product: TradeProduct;
    location_gcs: LocationGcs;
    media_uploads?: MediaUpload[];
};

export type NostrEventPageStore = NDKEventStore<ExtendedBaseType<NDKEvent>>;