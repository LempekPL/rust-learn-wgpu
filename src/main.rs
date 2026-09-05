fn main() {
    #[cfg(not(target_os = "android"))]
    rust_learn_wgpu::app::App::run().unwrap();
}