use rust_learn_wgpu::app::App;

fn main() {
    #[cfg(not(target_os = "android"))]
    App::run().unwrap();
}