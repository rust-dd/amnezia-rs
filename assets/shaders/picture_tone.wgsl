// RM2000 picture tone: an RGB multiply plus a saturation blend toward
// luminance (so `saturation = 0` renders the picture grayscale), then the
// picture's opacity. Mirrors EasyRPG's `Sprite_Picture` tone, applied per
// pixel so a colour source can be desaturated at runtime (a flat sprite tint
// cannot). Bound as a `Material2d` on the picture quad.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// xyz: per-channel RGB multiplier (1.0 = neutral). w: saturation (1.0 = neutral,
// 0.0 = full grayscale, >1.0 oversaturates).
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> rgb_sat: vec4<f32>;
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
    var rgb = texel.rgb * rgb_sat.xyz;
    let luma = dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
    rgb = mix(vec3<f32>(luma, luma, luma), rgb, rgb_sat.w);
    let alpha = select(1.0, texel.a, extra.y != 0.0);
    return vec4<f32>(rgb, alpha * extra.x);
}
