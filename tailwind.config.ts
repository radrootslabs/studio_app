import { theme_colors, themes } from "@radroots/theme";
import { wind } from "@radroots/utils";
import aspect_ratio from "@tailwindcss/aspect-ratio";
import daisyui from "daisyui";
import type { Config } from "tailwindcss";
import heropatterns from "tailwindcss-hero-patterns";
import tailwind_default from "tailwindcss/defaultTheme";
const { fontFamily: tw_font } = tailwind_default;

const heights_form = {
    line: `46px`,
};

const heights_responsive = {
    tabs_mobile_base: `64px`,
    tabs_mobile_y: `86px`,
    nav_mobile_base: `71px`,
    nav_mobile_y: `100px`,
    view_mobile_base: `31rem`,
    view_mobile_y: `42rem`,
    view_offset_mobile_base: `1rem`,
    view_offset_mobile_y: `2rem`,
    trellis_centered_mobile_base: `32rem`,
    trellis_centered_mobile_y: `35rem`
};

const heights = {
    ...heights_responsive,
    entry_bold: `3.75rem`,
    entry_guide: `3.6rem`,
    entry_line: `46px`,
    touch_bold: `4.25rem`,
    touch_guide: `3.4rem`,
    line: `46px`,
    envelope_top: `56px`,
    toast_min: `56px`,
    envelope_button: `50px`,
    line_label: `1.75rem`,
};


const widths_responsive = {
    mobile_base: `335px`,
    mobile_y: `345px`
};

const widths = {
    ...widths_responsive,
    line: `320px`,
    trellis_line: `349px`,
    trellis_value: `180px`,
    trellis_display: `286px`,
};

const dimensions_responsive = {
    map_offset_top_mobile_base: `32px`,
    map_offset_top_mobile_y: `64px`,
};

const dimensions = {
    ...dimensions_responsive,
    glyph_btn_sm: `28px`,
    map_circle: `16px`,
    map_circle_inner: `10px`,
};

const config: Config = {
    content: [
        `src/**/*.{ts,svelte}`,
        `../../packages/svelte-lib/src/**/*.{ts,svelte}`,
    ],
    theme: {
        screens: {
            mobile_y: { raw: `(orientation: portrait) and (min-height: ${wind.app.layout.mobile_y}px)` },
            mobile_base: { raw: `(orientation: portrait) and (max-height: ${wind.app.layout.mobile_base}px)` },
        },
        aspectRatio: {
            auto: `auto`,
            square: `1 / 1`,
            video: `16 / 9`,
            1: `1`,
            2: `2`,
            3: `3`,
            4: `4`,
            5: `5`,
            6: `6`,
            7: `7`,
            8: `8`,
            9: `9`,
            10: `10`,
            11: `11`,
            12: `12`,
            13: `13`,
            14: `14`,
            15: `15`,
            16: `16`,
        },
        extend: {
            colors: {
                ...theme_colors,
                'chart-green': `var(--chart-color-green)`,
                'chart-red': `var(--chart-color-red)`,
            },
            fontFamily: {
                sans: [`SF Pro Rounded`, ...tw_font.sans],
                sansd: [`SF Pro Display`],
                serif: [...tw_font.serif],
                mono: [...tw_font.mono],
                apercu: [`Apercu Mono Pro`],
                magda: [`Magda Text`],
                lust: [`Lust`],
                circ: [`Circular`],
                arch: [`Archivo`],
                sg: [`Space Grotesk`],
                sp: [`Spartan`],
                of: [`Outfit`]
            },
            fontSize: {
                guide: [`1.2rem`, { lineHeight: `1.2rem` }],
                line_label: [`1.3rem`, { lineHeight: `1.3rem` }],
                trellisTitle: [`0.8rem`, { lineHeight: `1rem`, fontWeight: 300 }],
                trellisTitleNote: [`0.76rem`, { lineHeight: `1rem`, fontWeight: 200 }],
                line_display: [`1.05rem`, { lineHeight: `1.33rem`, fontWeight: 400 }],
                line_display_e: [`1.05rem`, { lineHeight: `1.33rem`, fontWeight: 500 }],
                trellisLabel: [`0.8rem`, { lineHeight: `1rem`, fontWeight: 200 }],
                navPrevious: [`1.09rem`, { lineHeight: `1.33rem`, fontWeight: 400 }],
                navCurrent: [`1.09rem`, { lineHeight: `1.33rem`, fontWeight: 500 }],
                envelopeTitle: [`1.05rem`, { lineHeight: `1.75rem`, fontWeight: 600 }],
                envelopeTitlePrevious: [`1.02rem`, { lineHeight: `1.75rem`, fontWeight: 400 }],
                envelopeTitleAction: [`1.02rem`, { lineHeight: `1.75rem`, fontWeight: 500 }],
                envelopeButtonCancel: [`1.1rem`, { lineHeight: `1.75rem`, fontWeight: 600 }],
                envelopeButtonLabel: [`1.1rem`, { lineHeight: `1.75rem`, fontWeight: 500 }],
            },
            gridTemplateColumns: {
                '16': `repeat(16, minmax(0, 1fr))`,
                '24': `repeat(24, minmax(0, 1fr))`,
            },
            height: {
                ...heights,
                ...dimensions,
                ...Object.fromEntries(Object.entries(heights_form).map(([k, v]) => [`form_${k}`, v])),
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
        }
    },
    plugins: [
        daisyui,
        aspect_ratio,
        heropatterns
    ],
    daisyui: {
        themes: [
            themes.theme_os_dark,
            themes.theme_os_light,
            themes.theme_garden_light,
            themes.theme_garden_dark
        ],
    },
};

export default config;
