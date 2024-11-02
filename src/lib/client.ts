import { ClientNostr, TauriClientDatabase, TauriClientDevice, TauriClientDialog, TauriClientFs, TauriClientGeolocation, TauriClientHaptics, TauriClientHttp, TauriClientKeying, TauriClientKeystore, TauriClientLogger, TauriClientMap, TauriClientNotification, TauriClientOs, TauriClientWindow } from "@radroots/client";
import { Geocoder } from "@radroots/geocoder";

export const geoc = new Geocoder(`/geonames/geonames.db`);
export const db = new TauriClientDatabase();
export const device = new TauriClientDevice();
export const dialog = new TauriClientDialog();
export const fs = new TauriClientFs();
export const geol = new TauriClientGeolocation();
export const haptics = new TauriClientHaptics();
export const os = new TauriClientOs();
export const http = new TauriClientHttp();
export const map = new TauriClientMap();
export const keystore = new TauriClientKeystore();
export const keyring = new TauriClientKeying();
export const nostr = new ClientNostr();
export const notification = new TauriClientNotification();
export const win = new TauriClientWindow();
export const logger = new TauriClientLogger();