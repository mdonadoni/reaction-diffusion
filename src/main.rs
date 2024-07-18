use clap::Parser;
use reaction_diffusion::config::Config;
use reaction_diffusion::run;

fn main() {
    env_logger::init();
    let config = Config::parse();
    pollster::block_on(run(&config));
}
