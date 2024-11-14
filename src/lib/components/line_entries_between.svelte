<script lang="ts">
    import { fmt_cl, type IClOpt } from "@radroots/svelte-lib";
    import LineEntryData from "./line_entry_data.svelte";
    import LineEntryLabel from "./line_entry_label.svelte";

    export let basis: IClOpt & {
        label: IClOpt & {
            value: string;
            value_f?: string;
        };
        data: IClOpt & {
            value: string;
            value_f?: string;
        };
    };
</script>

<div
    class={`${fmt_cl(basis.classes)} flex flex-row h-6 w-full justify-between items-center`}
>
    <LineEntryLabel basis={basis.label} />
    <LineEntryData basis={basis.data} />
</div>
