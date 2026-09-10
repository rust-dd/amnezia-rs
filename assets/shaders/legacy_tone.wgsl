fn apply_tone(source: vec3<f32>, channels: vec4<f32>) -> vec3<f32> {
    var rgb = vec3<i32>(round(clamp(source, vec3(0.0), vec3(1.0)) * 255.0));
    let tone = vec4<i32>(channels);
    if tone.w != 128 {
        let lum = (19595 * rgb.r + 38470 * rgb.g + 7471 * rgb.b) >> 16;
        let saturation = select(tone.w * 8, 1024 + (tone.w - 128) * 16, tone.w > 128);
        rgb = clamp((vec3(lum * 1024) + (rgb - vec3(lum)) * saturation) >> vec3(10u), vec3(0), vec3(255));
    }
    let dark = 2 * tone.rgb * rgb / vec3(255);
    let light = vec3(255) - 2 * (vec3(255) - tone.rgb) * (vec3(255) - rgb) / vec3(255);
    return vec3<f32>(clamp(select(dark, light, tone.rgb > vec3(128)), vec3(0), vec3(255))) / 255.0;
}
