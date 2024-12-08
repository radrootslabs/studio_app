<script lang="ts">
    import { dialog } from "$lib/client";
    import { catch_err, Glyph, ls } from "@radroots/svelte-lib";

    export let photo_path: string;

    const handle_photo_add = async (): Promise<void> => {
        try {
            const photo_paths_open = await dialog.open_photos();
            if (!photo_paths_open) return;
            photo_path = photo_paths_open.results[0];
        } catch (e) {
            await catch_err(e, `handle_photo_add`);
        }
    };
</script>

<div class={`relative flex flex-row w-full justify-center items-center`}>
    <button
        class={`flex flex-row h-[5rem] w-[5rem] justify-center items-center bg-layer-1-surface/60 rounded-full`}
        on:click={async () => {
            await handle_photo_add();
        }}
    >
        <Glyph
            basis={{
                classes: `text-[40px] text-layer-2-glyph`,
                dim: `sm`,
                key: `camera`,
            }}
        />
        <div
            class={`absolute -bottom-[1.8rem] flex flex-row justify-start items-center`}
        >
            <p
                class={`font-arch font-[600] text-sm text-layer-0-glyph capitalize`}
            >
                {`${$ls(`icu.add_*`, { value: `${$ls(`common.photo`)}` })}`}
            </p>
        </div>
    </button>
</div>
