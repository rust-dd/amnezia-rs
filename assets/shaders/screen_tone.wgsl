#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import "shaders/legacy_tone.wgsl" as legacy

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

// Original RGB/saturation channels: 0..255, neutral 128.
struct ScreenTone {
    channels: vec4<f32>,
};
@group(0) @binding(2) var<uniform> tone: ScreenTone;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    // The sRGB-compositing camera stores encoded values in its intermediate texture.
    let src = textureSample(screen_texture, texture_sampler, in.uv);
    let rgb = legacy::apply_tone(src.rgb, tone.channels);
    return vec4<f32>(rgb, src.a);
}
