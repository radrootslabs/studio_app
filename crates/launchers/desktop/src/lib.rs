#![forbid(unsafe_code)]

mod app;
mod menus;
mod runtime;
#[cfg(test)]
mod source_guards;
mod window;

pub fn run() {
    app::launch();
}
