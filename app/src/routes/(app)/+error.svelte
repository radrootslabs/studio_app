<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
</script>

<div class={`flex flex-col w-full pt-20 justify-center items-center`}>
    <p class={`font-sans font-[400] text-layer-0-glyph`}>
        {`error: ${page.error?.message || `no message`}`}
    </p>
    <button
        class={`flex flex-row justify-center items-center`}
        onclick={async () => {
            await goto(`/`);
        }}
    >
        <p class={`font-sans font-[400] text-layer-0-glyph`}>
            {`reset`}
        </p>
    </button>
</div>
