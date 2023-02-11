struct Vert {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertFrag {
    @builtin(position) out_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct Global {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> global: Global;
var<push_constant> model: mat4x4<f32>;

@vertex
fn vs_main(vert: Vert) -> VertFrag {
    var vert_frag: VertFrag;
    vert_frag.out_position = global.projection * global.view * model * vec4<f32>(vert.position, 1.0);
    vert_frag.color = vert.color;
    vert_frag.uv = vert.uv;
    return vert_frag;
}

struct Frag {
    @location(0) out_color: vec4<f32>,
}

@group(1) @binding(0) var texture: texture_2d<f32>;
@group(1) @binding(1) var texture_sampler: sampler;

@fragment
fn fs_main(vert_frag: VertFrag) -> Frag {
    var frag: Frag;
    let texture_color = textureSample(texture, texture_sampler, vert_frag.uv);
    frag.out_color = vert_frag.color * texture_color;
    return frag;
}
