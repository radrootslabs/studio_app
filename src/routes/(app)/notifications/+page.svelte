<script lang="ts">
    import {
        LayoutView,
        ls,
        NavToolbar,
        PageHeader,
        TabsFloat,
    } from "@radroots/svelte-lib";

    let notifications: any[] = [];
</script>

<LayoutView>
    <NavToolbar />
    <PageHeader
        basis={{
            label: `${$ls(`common.notifications`)} (${notifications.length})`,
        }}
    ></PageHeader>
    <div class={`flex flex-col w-full px-4 gap-6 justify-center items-center`}>
        {#if notifications.length}
            {#each notifications as li}
                <div class={`flex flex-row w-full justify-center items-center`}>
                    {li}
                </div>
            {/each}
        {:else}
            <div class={`flex flex-row w-full justify-center items-center`}>
                <p class={`font-sans font-[500] text-layer-0-glyph capitalize`}>
                    {`${$ls(`icu.no_*`, { value: `${$ls(`common.notifications`)}` })}`}
                </p>
            </div>
        {/if}
    </div>
</LayoutView>
<TabsFloat />
