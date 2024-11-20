<script lang="ts">
    import { nostr_tags_basis } from "$lib/utils/nostr";
    import { NDKKind } from "@nostr-dev-kit/ndk";
    import { LayoutView, Nav, ndk, ndk_user, t } from "@radroots/svelte-lib";
    import { ndk_event } from "@radroots/utils";

    const post = async (): Promise<void> => {
        try {
            const tags = nostr_tags_basis();

            const ev = await ndk_event({
                $ndk,
                $ndk_user,
                basis: {
                    kind: NDKKind.Text,
                    content: `testing radroots at ${new Date().toISOString()}`,
                    tags,
                },
            });
            if (ev) await ev.publish();
            console.log(JSON.stringify(ev, null, 4), `ev`);
        } catch (e) {
            console.log(`(error) post `, e);
        }
    };
</script>

<LayoutView>
    <div class={`flex flex-col w-full px-4 gap-4 justify-center items-center`}>
        <button
            class={`flex flex-row justify-center items-center`}
            on:click={async () => {
                await post();
            }}
        >
            post
        </button>
    </div>
</LayoutView>

<Nav
    basis={{
        prev: {
            label: `${$t("common.home")}`,
            route: `/`,
        },
        title: {
            label: { value: `Image Upload` },
        },
    }}
/>
