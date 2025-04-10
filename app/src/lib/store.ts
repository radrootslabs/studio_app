import { writable } from "svelte/store";

export const cfg_role = writable<string>();
export const cfg_setup = writable<boolean | undefined>(undefined);
