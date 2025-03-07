<script lang="ts">
    import { goto } from "$app/navigation";
</script>

<div class={`flex flex-col w-full pt-32 justify-center items-center`}>
    <button
        class={`flex flex-row justify-center items-center`}
        onclick={async () => {
            await goto(`/`);
        }}
    >
        profile edit
    </button>
</div>
