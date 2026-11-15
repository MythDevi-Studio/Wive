mod app;
mod bootstrap;
mod lifecycle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    bootstrap::run()
}
