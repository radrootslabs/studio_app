<script lang="ts">
    import { fmt_cl, type IClOpt, type IClWrapOpt } from "@radroots/svelte-lib";

    export let basis: IClOpt &
        IClWrapOpt & {
            value: string;
            value_f?: string;
        };
</script>

<div class={`${fmt_cl(basis.classes_wrap)} flex flex-row h-6`}>
    <p
        class={`${fmt_cl(basis.classes)} font-sans font-[500] text-[1.05rem] text-justify truncate text-layer-1-glyph-shade`}
    >
        {basis.value || basis.value_f || ``}
    </p>
</div>
