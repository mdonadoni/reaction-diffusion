use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(long, default_value_t = 512)]
    pub width: u32,
    #[arg(long, default_value_t = 512)]
    pub height: u32,
    #[arg(long, default_value_t = 20)]
    pub steps_per_frame: u32,
    #[arg(long, default_value_t = 1.0)]
    pub timestep: f32,
    #[arg(long, default_value_t = 1.0)]
    pub diffusion_a: f32,
    #[arg(long, default_value_t = 0.5)]
    pub diffusion_b: f32,
    #[arg(long, default_value_t = 0.03)]
    pub feed: f32,
    #[arg(long, default_value_t = 0.09)]
    pub kill: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            steps_per_frame: 20,
            timestep: 1.0,
            diffusion_a: 1.0,
            diffusion_b: 0.5,
            feed: 0.03,
            kill: 0.09,
        }
    }
}

impl Config {
    pub fn with_size(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }
}
