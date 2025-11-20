<script lang="ts">
    import { notif, route } from "$lib/utils/app";
    import { ls } from "$lib/utils/i18n";
    import { handle_err, sleep } from "@radroots/apps-lib";
    import { Home } from "@radroots/apps-lib-pwa";
    import { qp_ref } from "@radroots/apps-lib-pwa/stores/app";
    import type { IViewHomeData } from "@radroots/apps-lib-pwa/types/views";

    qp_ref.subscribe(async (_qp_ref) => {
        if (_qp_ref?.toString() === "backup_imported") {
            await sleep(100);
            await notif.alert(
                `${$ls(`notification.backup.import_success`)}`,
            );
            qp_ref.set(null);
        }
    });

    let data: IViewHomeData | undefined = $state({});
</script>

<Home
    basis={{
        data,
        on_handle_farms: async () => {
            try {
                await route("/farms");
            } catch (e) {
                await handle_err(e, `on_handle_farms`);
            }
        },
        on_handle_products: async () => {
            try {
            } catch (e) {
                await handle_err(e, `on_handle_products`);
            }
        },
    }}
/>
