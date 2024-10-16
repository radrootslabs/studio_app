import { ClientNostr, TauriClientDb, TauriClientDialog, TauriClientGeolocation, TauriClientHaptics, TauriClientHttp, TauriClientKeying, TauriClientKeystore, TauriClientMap, TauriClientNotification, TauriClientWindow } from "@radroots/client";
import { Geocoder } from "@radroots/geocoder";

export const geoc = new Geocoder(`/geonames/geonames.db`);
export const db = new TauriClientDb();
export const dialog = new TauriClientDialog();
export const geol = new TauriClientGeolocation();
export const haptics = new TauriClientHaptics();
export const http = new TauriClientHttp();
export const map = new TauriClientMap();
export const keystore = new TauriClientKeystore();
export const keyring = new TauriClientKeying();
export const nostr = new ClientNostr();
export const notification = new TauriClientNotification();
export const win = new TauriClientWindow();