// RM2000 screen tone as a fullscreen post-process on the main camera: an RGB
// multiply (channel / 100, so < 100 darkens and > 100 brightens) followed by a
// saturation blend toward luminance (saturation 0 = grayscale, > 1 oversaturates).
// Mirrors picture_tone.wgsl but samples the whole rendered scene instead of one
// quad. Bound by Bevy's FullscreenMaterial: binding 0 the scene colour texture,
// 1 its sampler, 2 the tone uniform.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

// xyz: per-channel RGB multiplier (1.0 = neutral). w: saturation (1.0 = neutral,
// 0.0 = full grayscale, > 1.0 oversaturates).
struct ScreenTone {
    rgb_sat: vec4<f32>,
};
@group(0) @binding(2) var<uniform> tone: ScreenTone;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let src = textureSample(screen_texture, texture_sampler, in.uv);
    var rgb = src.rgb * tone.rgb_sat.xyz;
    let luma = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(vec3<f32>(luma, luma, luma), rgb, tone.rgb_sat.w);
    return vec4<f32>(rgb, src.a);
}
