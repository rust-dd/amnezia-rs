// RM2000 picture tone: an RGB multiply plus a saturation blend toward
// luminance (so `saturation = 0` renders the picture grayscale), then the
// picture's opacity. Mirrors EasyRPG's `Sprite_Picture` tone, applied per
// pixel so a colour source can be desaturated at runtime (a flat sprite tint
// cannot). Bound as a `Material2d` on the picture quad.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// xyz: per-channel RGB multiplier (1.0 = neutral). w: saturation (1.0 = neutral,
// 0.0 = full grayscale, >1.0 oversaturates).
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> rgb_sat: vec4<f32>;
// x: opacity 0..1 (1 - RM2000 transparency). yzw: unused padding.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> extra: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var picture_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var picture_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(picture_texture, picture_sampler, mesh.uv);
    var rgb = texel.rgb * rgb_sat.xyz;
    let luma = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(vec3<f32>(luma, luma, luma), rgb, rgb_sat.w);
    return vec4<f32>(rgb, texel.a * extra.x);
}
