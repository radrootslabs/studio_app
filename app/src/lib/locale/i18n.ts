import { i18n_conf } from '@radroots/util';
import locales_keys from './locales.json';

export type Locale = keyof typeof locales_keys;

const translations: Record<Locale, any> = {
    en: { locales_keys },
};

const i18n = i18n_conf<Locale>({
    default_locale: `en`,
    translations,
    loaders: [
        ...Object.keys(translations).map((locale) => [`common`, `error`, `eula`, `icu`, `notify`].map(key => ({
            locale,
            key,
            loader: async () => (await import(`./${locale}/${key}.json`)).default
        }))),
    ].flat()
});

const {
    t: ls,
    loading: translations_loading,
    locales,
    locale,
    loadTranslations: load_translations
} = i18n;

export { load_translations, locale, locales, ls, translations, translations_loading };
