struct Config {
    width: u32,
    height: u32,
    size: u32,
    timestep: f32,
    diffusion_a: f32,
    diffusion_b: f32,
    feed: f32,
    kill: f32,
};

@group(0) @binding(0) var<uniform> config: Config;
@group(0) @binding(1) var<storage, read> A: array<f32>;
@group(0) @binding(2) var<storage, read> B: array<f32>;
@group(0) @binding(3) var<storage, read_write> A_out: array<f32>;
@group(0) @binding(4) var<storage, read_write> B_out: array<f32>;

@compute @workgroup_size(64)
fn diffusion_step(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
    let width = config.width;
    let height = config.height;
    let timestep = config.timestep;
    let dA = config.diffusion_a;
    let dB = config.diffusion_b;
    let f = config.feed;
    let k = config.kill;

    let i = global_invocation_id.x;

    if i > config.size {
        return;
    }

    let conv_A = -A[i] + // current cell
        (A[i - 1] + A[i + 1] + A[i - width] + A[i + width]) * 0.2 + // neighbours
        (A[i - width - 1] + A[i - width + 1] + A[i + width - 1] + A[i + width + 1]) * 0.05; //corners

    let conv_B = -B[i] + // current cell
        (B[i - 1] + B[i + 1] + B[i - width] + B[i + width]) * 0.2 + // neighbours
        (B[i - width - 1] + B[i - width + 1] + B[i + width - 1] + B[i + width + 1]) * 0.05; // corners

    let a = A[i];
    let b = B[i];
    A_out[i] = a + ((dA * conv_A) - (a * b * b) + (f * (1.0 - a))) * timestep;
    B_out[i] = b + ((dB * conv_B) + (a * b * b) - (k * b)) * timestep;
}