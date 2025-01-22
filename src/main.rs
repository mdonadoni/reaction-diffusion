use clap::Parser;
use reaction_diffusion::{App, Config};

fn main() {
    env_logger::init();
    let config = Config::parse();
    let app = App::new(config);
    pollster::block_on(app.run());
}
