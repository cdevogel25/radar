var<private> VERTICES: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
    vec2<f32>(0.0, 2.0),
    vec2<f32>(1.7321, -1.0), //sqrt(3)
    vec2<f32>(-1.7321, -1.0),
);

struct VertexInput {
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, ) -> VertexOutput {
    var out: VertexOutput;
    let local_space = VERTICES[vertex.index];
    out.clip_position = vec4<f32>(local_space, 0.0, 1.0);
    out.uv = local_space;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let aspect_ratio = 800.0 / 600.0;
    var centered_uv = in.uv - 0.5;
    centered_uv.x *= aspect_ratio;
    let dist = length(centered_uv);
    let radius = 0.3;
    if (dist < radius) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
}