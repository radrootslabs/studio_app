const DEFAULT_RELAYS = import.meta.env.VITE_PUBLIC_DEFAULT_RELAYS;
if (!DEFAULT_RELAYS || typeof DEFAULT_RELAYS !== 'string') throw new Error('Missing env var: VITE_PUBLIC_DEFAULT_RELAYS');

const RADROOTS_API = import.meta.env.VITE_PUBLIC_RADROOTS_API;
if (!RADROOTS_API || typeof RADROOTS_API !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_API');

const RADROOTS_MEDIA = import.meta.env.VITE_PUBLIC_RADROOTS_MEDIA;
if (!RADROOTS_MEDIA || typeof RADROOTS_MEDIA !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_MEDIA');

const KEYVAL_NAME = import.meta.env.VITE_PUBLIC_KEYVAL_NAME;
if (!KEYVAL_NAME || typeof KEYVAL_NAME !== 'string') throw new Error('Missing env var: VITE_PUBLIC_KEYVAL_NAME');

const SQL_WASM_URL = import.meta.env.VITE_PUBLIC_SQL_WASM_URL;
if (!SQL_WASM_URL || typeof SQL_WASM_URL !== 'string') throw new Error('Missing env var: VITE_PUBLIC_SQL_WASM_URL');

const GEOCODER_DB_URL = import.meta.env.VITE_PUBLIC_GEOCODER_DB_URL;
if (!GEOCODER_DB_URL || typeof GEOCODER_DB_URL !== 'string') throw new Error('Missing env var: VITE_PUBLIC_GEOCODER_DB_URL');

const NOSTR_CLIENT = import.meta.env.VITE_PUBLIC_NOSTR_CLIENT;
if (!NOSTR_CLIENT || typeof NOSTR_CLIENT !== 'string') throw new Error('Missing env var: VITE_PUBLIC_NOSTR_CLIENT');

const RADROOTS_RELAY = import.meta.env.VITE_PUBLIC_RADROOTS_RELAY;
if (!RADROOTS_RELAY || typeof RADROOTS_RELAY !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_RELAY');

const PLATFORM_NAME = import.meta.env.VITE_PLATFORM_NAME;
if (!PLATFORM_NAME || typeof PLATFORM_NAME !== 'string') throw new Error('Missing env var: VITE_PLATFORM_NAME');

const PLATFORM_ACCENT = import.meta.env.VITE_PLATFORM_ACCENT;
if (!PLATFORM_ACCENT || typeof PLATFORM_ACCENT !== 'string') throw new Error('Missing env var: VITE_PLATFORM_ACCENT');

const PLATFORM_DESCRIPTION = import.meta.env.VITE_PLATFORM_DESCRIPTION;
if (!PLATFORM_DESCRIPTION || typeof PLATFORM_DESCRIPTION !== 'string') throw new Error('Missing env var: VITE_PLATFORM_DESCRIPTION');

const PROD = import.meta.env.MODE === 'production';

export const _env = {
  PROD,
  DEFAULT_RELAYS,
  GEOCODER_DB_URL,
  KEYVAL_NAME,
  NOSTR_CLIENT,
  PLATFORM_ACCENT,
  PLATFORM_DESCRIPTION,
  PLATFORM_NAME,
  RADROOTS_API,
  RADROOTS_MEDIA,
  RADROOTS_RELAY,
  SQL_WASM_URL,
} as const;
