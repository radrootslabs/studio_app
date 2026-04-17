#![forbid(unsafe_code)]

mod app;
mod menus;
#[cfg(test)]
mod source_guards;
mod substrate;
mod window;

pub fn run() {
    app::launch();
}
