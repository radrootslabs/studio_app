import { load_translations, locales, translations_loading } from '$lib/utils/i18n';
import { get_locale, handle_err } from '@radroots/apps-lib';
import type { LayoutLoad, LayoutLoadEvent } from './$types';

export const prerender = true;
export const ssr = false;
export const trailingSlash = 'always';

export const load: LayoutLoad = async ({ url }: LayoutLoadEvent) => {
    try {
        const loc = get_locale(locales.get());
        await load_translations(loc, url.pathname);
        await translations_loading.toPromise();
    } catch (e) {
        handle_err(e, `(root)load`)
    } finally {
        return {};
    };
};
