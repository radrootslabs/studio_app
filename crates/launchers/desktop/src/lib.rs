#![forbid(unsafe_code)]

mod app;
mod menus;
mod runtime;
#[cfg(test)]
mod source_guards;
mod window;

pub use app::AppLaunchError;

pub fn run() -> Result<(), AppLaunchError> {
    app::launch()
}
