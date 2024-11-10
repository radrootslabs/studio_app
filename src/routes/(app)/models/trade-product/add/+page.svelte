<script lang="ts">
    import ImageUploadMulti from "$lib/components/image_upload_multi.svelte";
    import { tradeproduct_init_kv } from "$lib/utils/trade_product";
    import {
        carousel_index,
        carousel_index_max,
        carousel_num,
        fmt_id,
        layout_view_cover,
        LayoutTrellis,
        LayoutView,
        Nav,
        t,
        view_effect,
    } from "@radroots/svelte-lib";
    import { type TradeKey } from "@radroots/utils";
    import { onMount } from "svelte";
    import { fade } from "svelte/transition";

    const CAROUSEL_INDEX_MAP = 2;

    type CarouselParam = {
        label_prev?: string;
        label_next: string;
    };
    const page_param: {
        carousel: Record<View, Map<number, CarouselParam>>;
        trade_product: {
            key_default: TradeKey;
        };
    } = {
        carousel: {
            fl_1: new Map<number, CarouselParam>([
                [
                    0,
                    {
                        label_next: `${$t(`common.add`)}`,
                    },
                ],
                [
                    1,
                    {
                        label_next: `${$t(`common.location`)}`,
                    },
                ],
                [
                    2,
                    {
                        label_next: `*`,
                    },
                ],
                [
                    3,
                    {
                        label_next: `Post`,
                    },
                ],
            ]),
        },
        trade_product: {
            key_default: `coffee`,
        },
    };

    let view_init: View = `fl_1`;
    type View = `fl_1`;
    let view: View = view_init;
    $: {
        view_effect<View>(view);
    }

    let load_submit = false;

    let tradeproduct_key_sel_toggle = false;
    let tradeproduct_key_sel = ``;

    let photo_add_paths: string[] = [];

    onMount(async () => {
        try {
            await handle_view(view_init);
            layout_view_cover.set(false);
            carousel_index.set(0);
            carousel_index_max.set(page_param.carousel[view].size - 1);
            await setup_tests();
        } catch (e) {
        } finally {
        }
    });

    const setup_tests = async (): Promise<void> => {
        try {
            //const photo1 = `file:///Users/treesap/Library/Developer/CoreSimulator/Devices/04252089-B59A-4955-B1E0-84ECBAC1C28D/data/Containers/Data/Application/73E35A31-5CCD-4C1A-97C3-B5FA617DDB80/Library/Caches/IMG_0111.jpeg`;
            //photo_add_list_paths = [photo1];
        } catch (e) {
            console.log(`(error) setup_tests `, e);
        }
    };

    const handle_view = async (view_new: View): Promise<void> => {
        try {
            /*
            const index_max_new = page_param.carousel[view_new].size - 1;
            carousel_index_max.set(index_max_new);
            if (view === `fl_2` && view_new === `fl_1`)
                carousel_index.set(index_max_new);
            else carousel_index.set(0);
            */
            carousel_index.set(0);
            view = view_new;
        } catch (e) {
            console.log(`(error) handle_view `, e);
        }
    };

    const handle_back = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) handle_back `, e);
        }
    };

    const handle_continue = async (): Promise<void> => {
        try {
            switch (view) {
                case `fl_1`:
                    {
                        switch ($carousel_index) {
                            case 0:
                                {
                                    console.log(
                                        JSON.stringify(
                                            photo_add_paths,
                                            null,
                                            4,
                                        ),
                                        `photo_add_paths`,
                                    );
                                }
                                break;
                        }
                    }
                    break;
            }
        } catch (e) {
            console.log(`(error) handle_continue `, e);
        }
    };

    const submit = async (): Promise<void> => {
        try {
        } catch (e) {
            console.log(`(error) submit `, e);
        }
    };
</script>

<LayoutView>
    <div
        data-view={`fl_1`}
        class={`flex flex-col h-full w-full justify-start items-center`}
    >
        <div
            data-carousel-container={`fl_1`}
            class={`carousel-container flex h-full w-full`}
        >
            <div
                data-carousel-item={`fl_1`}
                class={`carousel-item flex flex-col w-full justify-start items-center`}
            >
                <LayoutTrellis>
                    <ImageUploadMulti bind:photo_paths={photo_add_paths} />
                </LayoutTrellis>
            </div>
        </div>
    </div>
</LayoutView>
{#if $carousel_index !== CAROUSEL_INDEX_MAP}
    <div
        in:fade={{ delay: 0, duration: 50 }}
        out:fade={{ delay: 50, duration: 200 }}
        class={`flex flex-col w-full justify-start items-center fade-in`}
    >
        <Nav
            basis={{
                prev: {
                    label: `${$t(`common.back`)}`,
                    route: `/models/trade-product`,
                    prevent_route:
                        view === `fl_1` && $carousel_index === 0
                            ? undefined
                            : {
                                  callback: async () => {
                                      await handle_back();
                                  },
                              },
                    callback: async () => {
                        await tradeproduct_init_kv(fmt_id());
                    },
                },
                title:
                    $carousel_index === CAROUSEL_INDEX_MAP
                        ? undefined
                        : {
                              label: {
                                  value: `${$t(`icu.new_*`, { value: `${$t(`common.product`)}` })}`,
                              },
                              callback: async () => {},
                          },
                option: {
                    loading: load_submit,
                    label:
                        $carousel_index === CAROUSEL_INDEX_MAP
                            ? undefined
                            : {
                                  value:
                                      $carousel_num > 1
                                          ? `${$t(`common.return`)}`
                                          : page_param.carousel[view].get(
                                                $carousel_index,
                                            )?.label_next || ``,
                                  glyph:
                                      $carousel_index > 0
                                          ? {
                                                key: `caret-right`,
                                                classes: `text-layer-1-glyph-hl`,
                                            }
                                          : undefined,
                              },
                    callback: async () => {
                        if ($carousel_index === $carousel_index_max)
                            await submit();
                        else await handle_continue();
                    },
                },
            }}
        />
    </div>
{/if}
