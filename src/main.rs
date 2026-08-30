use rust_shopping_app::run;

fn main() {
    #[cfg(not(target_os = "android"))]
    run().unwrap();
}