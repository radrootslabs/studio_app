#![forbid(unsafe_code)]

mod accounts;
mod app;
mod menus;
mod remote_signer;
mod runtime;
#[cfg(test)]
mod source_guards;
mod window;

pub use app::AppLaunchError;

pub fn run() -> Result<(), AppLaunchError> {
    app::launch()
}
