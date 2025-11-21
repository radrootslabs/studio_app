const DEFAULT_RELAYS = import.meta.env.VITE_PUBLIC_DEFAULT_RELAYS;
if (!DEFAULT_RELAYS || typeof DEFAULT_RELAYS !== 'string') throw new Error('Missing env var: VITE_PUBLIC_DEFAULT_RELAYS');

const RADROOTS_API = import.meta.env.VITE_PUBLIC_RADROOTS_API;
if (!RADROOTS_API || typeof RADROOTS_API !== 'string') throw new Error('Missing env var: VITE_PUBLIC_RADROOTS_API');

const KEYVAL_NAME = import.meta.env.VITE_PUBLIC_KEYVAL_NAME;
if (!KEYVAL_NAME || typeof KEYVAL_NAME !== 'string') throw new Error('Missing env var: VITE_PUBLIC_KEYVAL_NAME');

const NDK_CACHE = import.meta.env.VITE_PUBLIC_NDK_CACHE;
if (!NDK_CACHE || typeof NDK_CACHE !== 'string') throw new Error('Missing env var: VITE_PUBLIC_NDK_CACHE');

const NDK_CLIENT = import.meta.env.VITE_PUBLIC_NDK_CLIENT;
if (!NDK_CLIENT || typeof NDK_CLIENT !== 'string') throw new Error('Missing env var: VITE_PUBLIC_NDK_CLIENT');

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
  KEYVAL_NAME,
  NDK_CACHE,
  NDK_CLIENT,
  PLATFORM_ACCENT,
  PLATFORM_DESCRIPTION,
  PLATFORM_NAME,
  RADROOTS_API,
  RADROOTS_RELAY,
} as const;
