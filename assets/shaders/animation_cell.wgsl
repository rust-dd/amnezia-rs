#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_render::color_operations::{linear_to_srgb, srgb_to_linear}
#import "shaders/legacy_tone.wgsl" as legacy

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> channels: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> source: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> sampling: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var sheet: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var<uniform> flash: vec4<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let destination = floor(mesh.uv * sampling.xy) + 0.5;
    let inverse_zoom = trunc(source.zw / sampling.xy * 65536.0) / 65536.0;
    // Sprite effects crop before scaling; neutral cells sample the full sheet.
    let offset = select(trunc(source.xy * sampling.xy / source.zw), vec2<f32>(0.0), sampling.w != 0.0);
    let origin = select(vec2<f32>(0.0), source.xy, sampling.w != 0.0);
    let coordinate = origin + ceil((offset + destination) * inverse_zoom) - 1.0;
    let texel = textureLoad(sheet, vec2<i32>(coordinate), 0);
    var rgb = legacy::apply_tone(linear_to_srgb(texel.rgb), channels);
    if flash.a > 0.0 {
        let original = floor((round(rgb * 255.0) * (255.0 - flash.a) + 127.0) / 255.0);
        let overlay = floor(flash.rgb * flash.a / 256.0);
        rgb = min(original + overlay, vec3<f32>(255.0)) / 255.0;
    }
#ifndef SRGB_OUTPUT
    rgb = srgb_to_linear(rgb);
#endif
    return vec4<f32>(rgb, texel.a * sampling.z);
}
