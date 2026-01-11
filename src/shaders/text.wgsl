// WebGPU Text Rendering Shader
// Optimized for monospace fonts (JetBrains Mono)

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) glyph_index: u32,
    @location(2) color: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct Uniforms {
    canvas_size: vec2<f32>,
    atlas_size: vec2<f32>,
    glyph_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var font_atlas: texture_2d<f32>;
@group(0) @binding(2) var font_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Convert pixel coordinates to NDC
    let ndc_x = (input.position.x / uniforms.canvas_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (input.position.y / uniforms.canvas_size.y) * 2.0;

    output.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);

    // Calculate texture coordinates from glyph index
    let glyphs_per_row = u32(uniforms.atlas_size.x / uniforms.glyph_size.x);
    let glyph_row = input.glyph_index / glyphs_per_row;
    let glyph_col = input.glyph_index % glyphs_per_row;

    let u = f32(glyph_col) * uniforms.glyph_size.x / uniforms.atlas_size.x;
    let v = f32(glyph_row) * uniforms.glyph_size.y / uniforms.atlas_size.y;

    output.tex_coords = vec2<f32>(u, v);

    // Unpack color from u32 (RGBA)
    let r = f32((input.color >> 24u) & 0xFFu) / 255.0;
    let g = f32((input.color >> 16u) & 0xFFu) / 255.0;
    let b = f32((input.color >> 8u) & 0xFFu) / 255.0;
    let a = f32(input.color & 0xFFu) / 255.0;

    output.color = vec4<f32>(r, g, b, a);

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample font atlas
    let glyph_alpha = textureSample(font_atlas, font_sampler, input.tex_coords).r;

    // Apply color with alpha from glyph texture
    return vec4<f32>(input.color.rgb, input.color.a * glyph_alpha);
}
