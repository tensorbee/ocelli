//! Local page on 127.0.0.1. A wasm build of this package has no listener.

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = ocelli_folder_viewer::serve() {
        eprintln!("the local page could not start: {error}");
        std::process::exit(1);
    }
}
