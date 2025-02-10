import { load_translations, locales, translations_loading } from '$lib/locale/i18n';
import { handle_err } from '@radroots/lib-app';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const prerender = true;
export const ssr = false;
export const trailingSlash = 'always';

export const load: LayoutLoad = async ({ url }: LayoutLoadEvent) => {
    try {
        const { language: nav_locale } = navigator;
        let locale = `en`;
        const locales_avail = locales.get();
        if (locales_avail.some(i => i === nav_locale.toLowerCase())) locale = navigator.language;
        else if (locales_avail.some(i => i === nav_locale.slice(0, 2).toLowerCase())) locale = nav_locale.slice(0, 2);
        await load_translations(locale.toLowerCase(), url.pathname);
        await translations_loading.toPromise();
    } catch (e) {
        await handle_err(e, `(root)load`)
    } finally {
        return {};
    };
};
