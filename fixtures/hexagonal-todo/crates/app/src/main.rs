//! Composition root: builds adapters, wires the interactor and mounts the UI.

mod wiring;

#[cfg(target_arch = "wasm32")]
fn main() {
    todo_ui::mount(wiring::build_service());
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let service = wiring::build_service();
    if let Err(error) = wiring::smoke::run(service.as_ref()) {
        eprintln!("smoke run failed: {error}");
        std::process::exit(1);
    }
}
