<script lang="ts">
    import { ls } from "$lib/utils/i18n";
</script>

<div class={`flex flex-col w-full justify-start items-start`}>
    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
        {`${$ls(`icu.*_information`, { value: `${$ls(`common.farm`)}` })}`}
    </p>
</div>
