#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_render::color_operations::{linear_to_srgb, srgb_to_linear}
#import "shaders/legacy_tone.wgsl" as legacy

// Original RGB/saturation channels: 0..255, neutral 128.
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> channels: vec4<f32>;
// x: opacity, y: use the palette-index-zero alpha mask.
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> extra: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var picture_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var picture_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> wave: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var texel: vec4<f32>;
    if wave.z > 0.0 {
        let scaled_size = vec2<f32>(textureDimensions(picture_texture)) * wave.z;
        let drawn = floor(scaled_size);
        let padding = ceil(abs(wave.y));
        let row = floor(mesh.uv.y * drawn.y);
        // RPG_RT restarts the row phase at the visible top when the image is clipped.
        let phase = wave.x + (row - wave.w) * 6.28318530718 / (32.0 * wave.z);
        let offset = trunc(wave.y * sin(phase));
        let x = floor(mesh.uv.x * (drawn.x + 2.0 * padding)) + 0.5 - padding - offset;
        if x < 0.0 || x >= drawn.x {
            discard;
        }
        // The original nearest sampler chooses the lower texel at exact ties.
        let inverse_zoom = trunc(65536.0 / wave.z) / 65536.0;
        let source = vec2<i32>(ceil(vec2<f32>(x, row + 0.5) * inverse_zoom) - 1.0);
        texel = textureLoad(picture_texture, source, 0);
    } else {
        texel = textureSampleLevel(picture_texture, picture_sampler, mesh.uv, 0.0);
    }
    var rgb = legacy::apply_tone(linear_to_srgb(texel.rgb), channels);
#ifndef SRGB_OUTPUT
    rgb = srgb_to_linear(rgb);
#endif
    let alpha = select(1.0, texel.a, extra.y != 0.0);
    return vec4<f32>(rgb, alpha * extra.x);
}
