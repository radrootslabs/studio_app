<script lang="ts">
    import { restart } from "$lib/util/client";
    import { LayoutView } from "@radroots/svelte-lib";
</script>

<LayoutView>
    <div class={`flex flex-col w-full gap-2 justify-start items-center`}>
        <p class={`font-sans font-[400] text-layer-0-glyph`}>
            The device is improperly configured.
        </p>
        <button
            class={`flex flex-row justify-center items-center`}
            on:click={async () => {
                await restart({
                    route: `/`,
                    notify_message: `The device was restarted`,
                });
            }}
        >
            <p class={`font-sans font-[400] text-layer-0-glyph-hl`}>
                Click to restart
            </p>
        </button>
    </div>
</LayoutView>
