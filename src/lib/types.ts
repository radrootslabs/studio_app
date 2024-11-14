import type { LocationGcs, TradeProduct } from "@radroots/models";

export type TradeProductBundle = {
    trade_product: TradeProduct;
    location_gcs?: LocationGcs;
};