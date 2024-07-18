struct Config {
    width: u32,
    height: u32,
    size: u32,
    timestep: f32,
    dA: f32,
    dB: f32,
    f: f32,
    k: f32,
};

@group(0) @binding(0) var<uniform> config: Config;
@group(0) @binding(1) var<storage, read> A: array<f32>;
@group(0) @binding(2) var<storage, read> B: array<f32>;

// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) vertex: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(vertex, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let width = config.width;
    let height = config.height;

    let i = u32(in.clip_position.x) + u32(in.clip_position.y) * width;
    let diff = A[i] - B[i];

    if diff > 0.7 {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else if diff < 0.3 {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else {
        let c = (diff - 0.3) / 0.4;
        return vec4<f32>(c, c, c, 1.0);
    }
}
