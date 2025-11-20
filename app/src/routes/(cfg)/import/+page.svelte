<script lang="ts">
    import { goto } from "$app/navigation";
    import { notif } from "$lib/utils/app";
    import { route } from "$lib/utils/app/app";
    import {
        import_app_state_from_file,
        validate_import_file,
    } from "$lib/utils/backup/import";
    import { Fade, Glyph } from "@radroots/apps-lib";
    import { LogoCircle, SelectMenu } from "@radroots/apps-lib-pwa";
    import type { ExportedAppState } from "@radroots/apps-lib-pwa/types/app";
    import { handle_err } from "@radroots/utils";

    let loading = $state(false);
    let current_file: File | null = $state(null);
    let current_file_validated: ExportedAppState | undefined =
        $state(undefined);

    $effect(() => {
        if (current_file) {
            (async () => {
                try {
                    const validated = await validate_import_file(current_file);
                    if (validated) current_file_validated = validated;
                    else current_file_validated = undefined;
                } catch {
                    current_file_validated = undefined;
                }
            })();
        }
    });

    const handle_back = async (): Promise<void> => {
        await route("/setup");
    };

    const on_change_file = (event: Event): void => {
        const target = event.currentTarget as HTMLInputElement | null;
        current_file = target?.files?.[0] ?? null;
    };

    const submit = async (): Promise<void> => {
        try {
            if (!current_file)
                return void notif.alert(
                    "error.configuration.import.no_file_chosen",
                );
            loading = true;
            const import_result =
                await import_app_state_from_file(current_file);
            console.log(`import_result `, import_result);
            if ("err" in import_result)
                return void (await notif.alert(import_result.err));
            else if (import_result.pass === true)
                return void (await goto(`/?ref=backup_imported`));
            await notif.alert(
                import_result.message || "error.configuration.import.failure",
            );
        } catch (e) {
            handle_err(e, `submit`);
        } finally {
            loading = false;
        }
    };
</script>

<button
    class={`z-10 absolute top-8 right-6 flex flex-row justify-center items-center`}
>
    <SelectMenu
        basis={{
            layer: 0,
            options: [
                {
                    entries: [
                        {
                            value: "",
                            label: "Choose Options",
                            disabled: true,
                        },
                        {
                            value: "back",
                            label: "Go Back",
                        },
                    ],
                },
            ],
            callback: async ({ value }) => {
                if (value === "back") return void (await handle_back());
            },
        }}
    >
        <Glyph
            basis={{
                classes: `text-base text-ly0-gl group-active:text-ly0-gl-a`,
                key: "gear",
            }}
        />
    </SelectMenu>
</button>

<div class={`flex flex-col h-screen w-full px-4 justify-center items-start`}>
    <div class={`flex flex-col w-full gap-4 justify-center items-start `}>
        <LogoCircle />
        <div
            class={`flex flex-col w-full px-1 gap-1 justify-start items-start`}
        >
            <p class={`font-sans font-[600] text-lg text-ly0-gl`}>
                {`Import app state`}
            </p>
            <p class={`font-sans font-[400] text-sm text-ly0-gl/80 max-w-xl`}>
                {`Choose a JSON export created by this app to restore configuration, nostr keys, and database information.`}
            </p>
        </div>
        <div class={`flex flex-row w-full justify-center items-center`}>
            <input
                type={`file`}
                accept={`application/json,.json`}
                class={`w-full py-2 pl-2 bg-ly0-gl/5 text-ly0-gl text-sm border border-ly0-gl/30 rounded-xl`}
                onchange={on_change_file}
            />
        </div>
        <div class={`flex flex-row h-8 w-full justify-start items-start`}>
            {#if current_file}
                <Fade
                    basis={{
                        classes: `flex-row w-full gap-1 justify-center items-center`,
                    }}
                >
                    {#if current_file_validated}
                        <Glyph
                            basis={{
                                classes: `text-base text-lime-600`,
                                key: "check-circle",
                                weight: "fill",
                            }}
                        />
                        <p class={`font-sans font-[500] text-sm text-lime-600`}>
                            {`The backup file is valid (version: ${current_file_validated.backup_version})`}
                        </p>
                    {:else}
                        <Glyph
                            basis={{
                                classes: `text-base text-red-400`,
                                key: "x-circle",
                                weight: "fill",
                            }}
                        />
                        <p class={`font-sans font-[500] text-sm text-red-400`}>
                            {`Not a valid backup file`}
                        </p>
                    {/if}
                </Fade>
            {/if}
        </div>
        <div class={`flex flex-row h-8 w-full justify-center items-center`}>
            {#if current_file && current_file_validated}
                <Fade basis={{ in: { duration: 400 } }}>
                    <button
                        class={`relative flex flex-row h-8 px-5 justify-center items-center bg-ly1 active:bg-ly1-a rounded-lg text-ly1-gl disabled:opacity-40`}
                        disabled={loading}
                        onclick={submit}
                    >
                        {`Import`}
                        {#if loading}
                            <Glyph
                                basis={{
                                    classes: `absolute -right-6 text-[1px] text-ly0-gl animate-spin-slow`,
                                    key: "circle-notch",
                                }}
                            />
                        {/if}
                    </button>
                </Fade>
            {/if}
        </div>
    </div>
</div>
