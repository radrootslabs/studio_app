#![forbid(unsafe_code)]

mod accounts;
mod app;
mod menus;
pub mod pack_day_host_handoff;
mod remote_signer;
mod runtime;
#[cfg(test)]
mod source_guards;
mod window;

pub use app::AppLaunchError;

pub fn run() -> Result<(), AppLaunchError> {
    app::launch()
}
