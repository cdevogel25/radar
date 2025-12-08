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

@group(0) @binding(0)
var<uniform> u: vec4<f32>;

const PI: f32 = 3.141592653589793;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var centered_uv = in.uv;
    centered_uv.x *= u.y; // aspect

    let dist = length(centered_uv);
    let radius = u.z;

    // base background (dark)
    var out_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    // filled circle area
    if (dist < radius) {
        out_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    } else {
        return out_color;
    }

    // thin outline ring
    if (dist > radius - 0.02 && dist < radius) {
        out_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    // rotating radius line
    let angle = atan2(centered_uv.y, centered_uv.x);
    var a = angle;
    if (a < 0.0) {
        a = a + 2.0 * PI;
    }
    var diff = abs(a - u.x);
    if (diff > 2.0 * PI - diff) {
        diff = 2.0 * PI - diff;
    }

    // line thickness threshold (radians)
    let line_thickness = 0.02;
    if (dist < radius && diff < line_thickness) {
        // bright sweep color
        out_color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    return out_color;
}