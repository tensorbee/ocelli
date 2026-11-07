//! Local page on 127.0.0.1. A wasm build of this package has no listener.

#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = ocelli_folder_viewer::serve() {
        eprintln!("the local page could not start: {error}");
        std::process::exit(1);
    }
}
