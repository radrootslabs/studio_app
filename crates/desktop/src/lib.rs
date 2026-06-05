#![forbid(unsafe_code)]

mod accounts;
mod app;
mod menus;
pub mod pack_day_host_handoff;
pub mod pack_day_print;
mod remote_signer;
mod runtime;
#[cfg(test)]
mod source_guards;
mod window;

pub use accounts::DesktopLocalIdentityImportRequest;
pub use app::AppLaunchError;
pub use runtime::DesktopAppRuntime;

pub fn run() -> Result<(), AppLaunchError> {
    app::launch()
}
