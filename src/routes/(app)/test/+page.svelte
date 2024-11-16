<script lang="ts">
    import { LayoutView, Nav, t } from "@radroots/svelte-lib";

    const test1 = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) test1 `, e);
        }
    };
</script>

<LayoutView>
    <div class={`flex flex-col w-full justify-center items-center`}>
        <button
            class={`flex flex-row justify-center items-center`}
            on:click={async () => {
                await test1();
            }}
        >
            test1
        </button>
    </div>
</LayoutView>
<Nav
    basis={{
        prev: {
            label: `${$t(`common.home`)}`,
            route: `/`,
        },
        title: {
            label: {
                value: `Test`,
            },
        },
    }}
/>
