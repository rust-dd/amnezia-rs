#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_render::color_operations::{linear_to_srgb, srgb_to_linear}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> control: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> crop: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> mosaic: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var live: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var before: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var after: texture_2d<f32>;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let native = vec2<f32>(320.0, 240.0);
    var pixel = clamp(floor(mesh.uv * native), vec2<f32>(0.0), native - 1.0);
    let erase = control.y != 0.0;
    var rgb = vec3<f32>(0.0);
    if control.x == 20.0 {
        rgb = linear_to_srgb(textureLoad(live, vec2<i32>(pixel), 0).rgb);
    } else if control.x == 0.0 {
        var first = vec3<f32>(0.0);
        var second = vec3<f32>(0.0);
        if control.z == 0.0 {
            first = linear_to_srgb(textureLoad(before, vec2<i32>(pixel), 0).rgb);
        }
        if !erase {
            second = linear_to_srgb(textureLoad(after, vec2<i32>(pixel), 0).rgb);
        }
        rgb = mix(first, second, control.w);
    } else if control.x != 21.0 {
        if control.x == 17.0 {
            let half = floor(mosaic.x / 2.0);
            pixel = clamp(floor((pixel + mosaic.y + half) / mosaic.x) * mosaic.x - half,
                vec2<f32>(0.0), native - 1.0);
        } else if control.x == 16.0 {
            let scale = trunc(crop.zw / native * 65536.0) / 65536.0;
            let origin = trunc(crop.xy * native / crop.zw);
            pixel = clamp(ceil((origin + pixel + 0.5) * scale) - 1.0,
                vec2<f32>(0.0), native - 1.0);
        }
        if erase {
            rgb = linear_to_srgb(textureLoad(before, vec2<i32>(pixel), 0).rgb);
        } else {
            rgb = linear_to_srgb(textureLoad(after, vec2<i32>(pixel), 0).rgb);
        }
    }
#ifndef SRGB_OUTPUT
    rgb = srgb_to_linear(rgb);
#endif
    return vec4<f32>(rgb, 1.0);
}
