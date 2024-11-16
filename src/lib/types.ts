import type { LocationGcs, MediaUpload, TradeProduct } from "@radroots/models";

export type TradeProductBundle = {
    trade_product: TradeProduct;
    location_gcs: LocationGcs;
    media_uploads?: MediaUpload[];
};