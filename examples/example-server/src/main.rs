//! Loopback helper. A wasm build of this package has no listener.

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = ocelli_example_server::serve() {
        eprintln!("the example helper could not start: {error}");
        std::process::exit(1);
    }
}
