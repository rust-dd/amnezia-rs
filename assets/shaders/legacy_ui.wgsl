#import bevy_ui::ui_node as ui
#import bevy_render::color_operations::linear_to_srgb

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) flags: u32,
    @location(4) radius: vec4<f32>,
    @location(5) border: vec4<f32>,
    @location(6) size: vec2<f32>,
    @location(7) point: vec2<f32>,
) -> ui::VertexOutput {
    return ui::vertex(position, uv, color, flags, radius, border, size, point);
}

@fragment
fn fragment(in: ui::VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(ui::sprite_texture, ui::sprite_sampler, in.uv);
    let tint = vec4<f32>(linear_to_srgb(in.color.rgb), in.color.a);
    let texture_color = vec4<f32>(linear_to_srgb(sampled.rgb), sampled.a);
    let color = select(tint, tint * texture_color, ui::enabled(in.flags, ui::TEXTURED));
    if ui::enabled(in.flags, ui::BORDER_ANY) {
        return ui::draw_uinode_border(color, in.point, in.size, in.radius, in.border, in.flags);
    }
    return ui::draw_uinode_background(color, in.point, in.size, in.radius, in.border, in.flags);
}
