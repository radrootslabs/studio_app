<script lang="ts">
    import { page } from "$app/stores";
    import { Glyph, LayoutArea, Nav, t } from "@radroots/svelte-lib";
</script>

<LayoutArea>
    <div class={`flex flex-col h-full w-full justify-center items-center`}>
        <div class={`flex flex-col justify-start items-center`}>
            <Glyph
                basis={{
                    classes: `text-layer-1-glyph-shade`,
                    key: `subtitles-slash`,
                    dim: `xl`,
                }}
            />
            <p class={`font-sans font-[400] text-layer-0-glyph`}>
                {`${$t(`error.client.page.status.${$page.status}`)}`}
            </p>
        </div>
    </div>
</LayoutArea>
<Nav
    basis={{
        prev: {
            label: `Home`,
            route: `/`,
        },
    }}
/>
