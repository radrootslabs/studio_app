use leptos::prelude::*;

use radroots_studio_app_ui_components::{
    RadrootsAppUiSheetClose,
    RadrootsAppUiSheetContent,
    RadrootsAppUiSheetDescription,
    RadrootsAppUiSheetOverlay,
    RadrootsAppUiSheetPortal,
    RadrootsAppUiSheetRoot,
    RadrootsAppUiSheetTitle,
    RadrootsAppUiSheetTrigger,
};

#[component]
pub fn RadrootsAppUiDemoPage() -> impl IntoView {
    let sheet_open = RwSignal::new(false);
    let sheet_open_read = sheet_open.read_only();
    let sheet_open_set = Callback::new(move |value| sheet_open.set(value));
    view! {
        <main style="padding: 16px;">
            <div style="font: var(--type-title2); margin-bottom: 12px;">"UI Demo"</div>
            <div data-ui="list-group">
                <div data-ui="list-row">
                    <div data-ui="list-row-leading">"Wi-Fi"</div>
                    <div data-ui="list-row-trailing">"On"</div>
                </div>
                <div data-ui="list-row">
                    <div data-ui="list-row-leading">"Bluetooth"</div>
                    <div data-ui="list-row-trailing">"On"</div>
                </div>
                <div data-ui="list-row">
                    <div data-ui="list-row-leading">"Notifications"</div>
                    <div data-ui="list-row-trailing">"Enabled"</div>
                </div>
            </div>

            <RadrootsAppUiSheetRoot
                open=Some(sheet_open_read)
                default_open=false
                modal=None
                on_open_change=Some(sheet_open_set)
            >
                <RadrootsAppUiSheetTrigger
                    disabled=false
                    class=Some("ui-card".to_string())
                    id=None
                    style=Some("padding:12px 16px; width: 100%; text-align: left;".to_string())
                >
                    "Open Sheet"
                </RadrootsAppUiSheetTrigger>
                <RadrootsAppUiSheetPortal>
                    <RadrootsAppUiSheetOverlay
                        close_on_click=None
                        class=None
                        id=None
                        style=None
                    />
                    <RadrootsAppUiSheetContent
                        disable_outside_pointer_events=false
                        show_handle=true
                        class=None
                        id=None
                        style=None
                    >
                        <RadrootsAppUiSheetTitle
                            class=None
                            id=None
                            style=None
                        >
                            "Sheet Preview"
                        </RadrootsAppUiSheetTitle>
                        <RadrootsAppUiSheetDescription
                            class=None
                            id=None
                            style=Some("margin-top: 6px;".to_string())
                        >
                            "This is a placeholder sheet for iOS styling."
                        </RadrootsAppUiSheetDescription>
                        <RadrootsAppUiSheetClose
                            class=Some("ui-card".to_string())
                            id=None
                            style=Some("margin-top: 16px; padding: 10px 14px;".to_string())
                        >
                            "Close"
                        </RadrootsAppUiSheetClose>
                    </RadrootsAppUiSheetContent>
                </RadrootsAppUiSheetPortal>
            </RadrootsAppUiSheetRoot>
        </main>
    }
}
