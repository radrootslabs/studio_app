import { theme_colors, themes } from "@radroots/theme";
import { type AppHeightsResponsiveIOS, type AppWidthsResponsiveIOS, cfg_app } from "@radroots/util";
import daisyui from "daisyui";
import type { Config } from "tailwindcss";
import tailwind_default from "tailwindcss/defaultTheme";
const { fontFamily: tw_font, screens: tw_screens } = tailwind_default;

const heights_responsive: Record<AppHeightsResponsiveIOS, string> = {
    nav_tabs_ios0: `80px`,
    nav_tabs_ios1: `120px`,
    nav_page_toolbar_ios0: `72px`,
    nav_page_toolbar_ios1: `120px`,
    nav_page_header_ios0: `62px`,
    nav_page_header_ios1: `62px`,
    lo_bottom_button_ios0: `90px`,
    lo_bottom_button_ios1: `112px`,
    lo_view_main_ios0: `22rem`,
    lo_view_main_ios1: `28rem`,
};

const heights: Record<string, string> = {
    ...heights_responsive,
    line: `46px`,
    line_button: `3.25rem`,
    touch_guide: `3.4rem`,
    entry_line: `48px`,
    bold_button: `4.25rem`,
};

const widths_responsive: Record<AppWidthsResponsiveIOS, string> = {
    lo_ios0: `340px`,
    lo_ios1: `345px`,
    lo_textdesc_ios0: `312px`,
    lo_textdesc_ios1: `312px`,
};

const widths: Record<string, string> = {
    ...widths_responsive,
    trellis_line: `349px`,
    trellis_value: `180px`,
    trellis_display: `286px`,
};

const dimensions_responsive: Record<string, string> = {
    ios0: `340px`,
    ios1: `345px`
};

const spacing: Record<string, string> = {
    line: `1px`,
    edge: `2px`
};

const dimensions: Record<string, string> = {
    ...dimensions_responsive
};

const config: Config = {
    content: [
        `src/**/*.{ts,svelte}`,
        `../packages/lib-app/src/**/*.{ts,svelte}`,
    ],
    theme: {
        screens: {
            ios1: { raw: `(orientation: portrait) and (min-height: ${cfg_app.layout.ios1.h}px)` },
            ios0: { raw: `(orientation: portrait) and (max-height: ${cfg_app.layout.ios0.h}px)` },
            ...tw_screens
        },
        extend: {
            colors: {
                ...theme_colors,
            },
            fontFamily: {
                sans: [`SF Pro Rounded`, ...tw_font.sans],
                sansd: [`SF Pro Display`],
                serif: [...tw_font.serif],
                mono: [...tw_font.mono],
            },
            fontSize: {
                guide: [`1.25rem`, { lineHeight: `1.25rem` }],
                form_base: `1.08rem`,
                line_label: [`1.3rem`, { lineHeight: `1.3rem` }],
                trellis_ti: [`0.8rem`, { lineHeight: `1rem`, fontWeight: 300 }],
                line_d: [`1.05rem`, { lineHeight: `1.33rem`, fontWeight: 400 }],
                nav_prev: [`1.09rem`, { lineHeight: `1.33rem`, fontWeight: 400 }],
                nav_curr: [`1.09rem`, { lineHeight: `1.33rem`, fontWeight: 500 }],
                env_ti: [`1.05rem`, { lineHeight: `1.75rem`, fontWeight: 600 }],
                env_btnc: [`1.1rem`, { lineHeight: `1.75rem`, fontWeight: 600 }],
                env_btnl: [`1.1rem`, { lineHeight: `1.75rem`, fontWeight: 500 }],
            },
            gridTemplateColumns: {
                '16': `repeat(16, minmax(0, 1fr))`,
                '24': `repeat(24, minmax(0, 1fr))`,
            },
            height: {
                ...heights,
                ...dimensions,
            },
            width: {
                ...widths,
                ...dimensions,
            },
            minHeight: {
                ...heights
            },
            minWidth: {
                ...widths
            },
            maxHeight: {
                ...heights
            },
            maxWidth: {
                ...widths
            },
            padding: {
                ...Object.fromEntries(Object.entries(heights).map(([k, v]) => [`h_${k}`, v])),
                ...Object.fromEntries(Object.entries(widths).map(([k, v]) => [`w_${k}`, v])),
                ...Object.fromEntries(Object.entries(dimensions).map(([k, v]) => [`dim_${k}`, v])),
            },
            translate: {
                ...Object.fromEntries(Object.entries(heights).map(([k, v]) => [`h_${k}`, v])),
                ...Object.fromEntries(Object.entries(widths).map(([k, v]) => [`w_${k}`, v])),
            },
            spacing: {
                ...spacing,
                ...Object.fromEntries(Object.entries(dimensions).map(([k, v]) => [`dim_${k}`, v])),
            },
            borderWidth: {
                line: `1px`,
                edge: `2px`
            },
            borderRadius: {
                input_form: `8px`,
                entry: `1.05rem`,
                touch: `1.25rem`
            },
            animation: {
                'spin-slow': 'spin 3s linear infinite',
            },
            keyframes: {
                spin: {
                    '0%': { transform: 'rotate(0deg)' },
                    '100%': { transform: 'rotate(360deg)' },
                },
            },
        },
    },
    plugins: [
        daisyui,
    ],
    daisyui: {
        themes: [
            themes.theme_os_light,
            themes.theme_os_dark,
        ],
    },
};

export default config;
