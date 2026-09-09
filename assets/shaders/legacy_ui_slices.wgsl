#import "embedded://bevy_ui_render/ui_texture_slice.wgsl" as sliced
#import bevy_render::color_operations::linear_to_srgb

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) texture_slices: vec4<f32>,
    @location(4) target_slices: vec4<f32>,
    @location(5) repeat: vec4<f32>,
    @location(6) atlas_rect: vec4<f32>,
) -> sliced::UiVertexOutput {
    return sliced::vertex(position, uv, color, texture_slices, target_slices, repeat, atlas_rect);
}

@fragment
fn fragment(in: sliced::UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = sliced::map_uvs_to_slice(in.uv, in.target_slices, in.texture_slices, in.repeat);
    let atlas_uv = in.atlas_rect.xy + uv * (in.atlas_rect.zw - in.atlas_rect.xy);
    let sampled = textureSample(sliced::sprite_texture, sliced::sprite_sampler, atlas_uv);
    return vec4<f32>(linear_to_srgb(in.color.rgb) * linear_to_srgb(sampled.rgb), in.color.a * sampled.a);
}
