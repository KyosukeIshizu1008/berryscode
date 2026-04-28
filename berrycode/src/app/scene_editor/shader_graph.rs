//! Shader graph: node-based material parameter graph.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderGraph {
    pub name: String,
    pub nodes: Vec<ShaderNode>,
    pub edges: Vec<ShaderEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderNode {
    pub id: u64,
    pub node_type: ShaderNodeType,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShaderNodeType {
    OutputPBR, // Final output: color, metallic, roughness, emissive, normal
    TextureSample { path: String },
    ColorConstant { value: [f32; 4] },
    FloatConstant { value: f32 },
    Multiply,
    Add,
    Lerp,
    UVCoord,
    Time,
    Fresnel { power: f32 },
}

impl ShaderNodeType {
    pub fn label(&self) -> &'static str {
        match self {
            ShaderNodeType::OutputPBR => "PBR Output",
            ShaderNodeType::TextureSample { .. } => "Texture",
            ShaderNodeType::ColorConstant { .. } => "Color",
            ShaderNodeType::FloatConstant { .. } => "Float",
            ShaderNodeType::Multiply => "Multiply",
            ShaderNodeType::Add => "Add",
            ShaderNodeType::Lerp => "Lerp",
            ShaderNodeType::UVCoord => "UV",
            ShaderNodeType::Time => "Time",
            ShaderNodeType::Fresnel { .. } => "Fresnel",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderEdge {
    pub from_node: u64,
    pub from_pin: usize,
    pub to_node: u64,
    pub to_pin: usize,
}

impl Default for ShaderGraph {
    fn default() -> Self {
        Self {
            name: "New Shader".into(),
            nodes: vec![
                ShaderNode {
                    id: 1,
                    node_type: ShaderNodeType::ColorConstant {
                        value: [0.5, 0.5, 1.0, 1.0],
                    },
                    position: [100.0, 100.0],
                },
                ShaderNode {
                    id: 2,
                    node_type: ShaderNodeType::OutputPBR,
                    position: [400.0, 100.0],
                },
            ],
            edges: vec![ShaderEdge {
                from_node: 1,
                from_pin: 0,
                to_node: 2,
                to_pin: 0,
            }],
        }
    }
}

/// Evaluate the shader graph to produce PBR material parameters.
pub fn evaluate_graph(graph: &ShaderGraph) -> PbrParams {
    let mut params = PbrParams::default();
    // Simple topological evaluation: find OutputPBR node, trace inputs
    let output = graph
        .nodes
        .iter()
        .find(|n| matches!(n.node_type, ShaderNodeType::OutputPBR));
    let output_id = match output {
        Some(n) => n.id,
        None => return params,
    };

    // Pin 0 = base_color, Pin 1 = metallic, Pin 2 = roughness, Pin 3 = emissive
    for pin in 0..4 {
        if let Some(edge) = graph
            .edges
            .iter()
            .find(|e| e.to_node == output_id && e.to_pin == pin)
        {
            if let Some(source) = graph.nodes.iter().find(|n| n.id == edge.from_node) {
                match &source.node_type {
                    ShaderNodeType::ColorConstant { value } => match pin {
                        0 => params.base_color = [value[0], value[1], value[2]],
                        3 => params.emissive = [value[0], value[1], value[2]],
                        _ => {}
                    },
                    ShaderNodeType::FloatConstant { value } => match pin {
                        1 => params.metallic = *value,
                        2 => params.roughness = *value,
                        _ => {}
                    },
                    _ => {} // Complex nodes not evaluated in MVP
                }
            }
        }
    }
    params
}

#[derive(Debug, Clone)]
pub struct PbrParams {
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
}

impl Default for PbrParams {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
        }
    }
}

/// Generate a Bevy-compatible WGSL fragment shader from the evaluated
/// PBR parameters. v0.5 / Shader Graph live-recompile preview.
///
/// The graph already drives the in-process material preview every
/// frame via [`evaluate_graph`]; this function lets the user export
/// the same parameters as a real `.wgsl` source file. Saving to
/// `assets/shaders/<name>.wgsl` then triggers the v0.5 asset hot
/// reload watcher, so the shader can be picked up by Bevy's runtime
/// without restarting the editor.
///
/// The output uses Bevy 0.18's pbr_fragment / mesh_view_bindings
/// conventions; advanced graph nodes (texture sampling, math nodes)
/// will gradually replace the literal-constant emit with real
/// expression trees in v0.6+.
pub fn generate_wgsl(graph: &ShaderGraph) -> String {
    let params = evaluate_graph(graph);
    let name_safe = graph
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    format!(
        r#"// Generated by BerryCode Shader Graph editor (v0.5).
// Source graph: {name}
//
// This shader is regenerated from `{name}.bshader` every time the
// user clicks "Compile" in the editor. Hand edits here will be
// overwritten on the next compile — keep changes in the .bshader
// instead.

#import bevy_pbr::{{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    forward_io::{{VertexOutput, FragmentOutput}},
    pbr_functions,
    pbr_types::pbr_input_new,
}}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {{
    var pbr_input = pbr_input_new();
    pbr_input.material.base_color = vec4<f32>({r:.4}, {g:.4}, {b:.4}, 1.0);
    pbr_input.material.metallic = {metallic:.4};
    pbr_input.material.perceptual_roughness = {roughness:.4};
    pbr_input.material.emissive = vec4<f32>({er:.4}, {eg:.4}, {eb:.4}, 1.0);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = pbr_functions::prepare_world_normal(
        in.world_normal,
        (pbr_input.material.flags & 0u) != 0u,
        is_front,
    );
    pbr_input.is_orthographic = false;
    pbr_input.N = pbr_input.world_normal;
    pbr_input.V = pbr_functions::calculate_view(in.world_position, false);
    var out: FragmentOutput;
    out.color = pbr_functions::apply_pbr_lighting(pbr_input);
    return out;
}}

// _generator_id: {name_safe}
"#,
        name = graph.name,
        name_safe = name_safe,
        r = params.base_color[0],
        g = params.base_color[1],
        b = params.base_color[2],
        metallic = params.metallic,
        roughness = params.roughness,
        er = params.emissive[0],
        eg = params.emissive[1],
        eb = params.emissive[2],
    )
}

/// Write the graph's generated WGSL to `<root>/assets/shaders/<name>.wgsl`.
/// The hot-reload watcher (v0.5) is already monitoring `assets/`, so the
/// next frame after this returns the file change is surfaced in the
/// status bar — and Bevy's own asset reload picks the shader up at the
/// runtime layer.
pub fn write_wgsl_to_assets(graph: &ShaderGraph, project_root: &str) -> Result<String, String> {
    let dir = format!("{}/assets/shaders", project_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Create dir failed: {e}"))?;
    let safe = graph
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let path = format!(
        "{}/{}.wgsl",
        dir,
        if safe.is_empty() { "shader" } else { &safe }
    );
    std::fs::write(&path, generate_wgsl(graph)).map_err(|e| format!("Write failed: {e}"))?;
    Ok(path)
}

pub fn save_shader_graph(graph: &ShaderGraph, path: &str) -> Result<(), String> {
    let s = ron::ser::to_string_pretty(graph, ron::ser::PrettyConfig::default())
        .map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn load_shader_graph(path: &str) -> Result<ShaderGraph, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    ron::from_str(&s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_graph_has_output() {
        let g = ShaderGraph::default();
        assert!(g
            .nodes
            .iter()
            .any(|n| matches!(n.node_type, ShaderNodeType::OutputPBR)));
    }
    #[test]
    fn evaluate_default_gets_color() {
        let g = ShaderGraph::default();
        let p = evaluate_graph(&g);
        assert!((p.base_color[2] - 1.0).abs() < 0.01); // blue from default ColorConstant
    }
    #[test]
    fn evaluate_empty_returns_defaults() {
        let g = ShaderGraph {
            name: "empty".into(),
            nodes: vec![],
            edges: vec![],
        };
        let p = evaluate_graph(&g);
        assert!((p.metallic - 0.0).abs() < 0.01);
    }
    #[test]
    fn ron_roundtrip() {
        let g = ShaderGraph::default();
        let s = ron::ser::to_string(&g).unwrap();
        let loaded: ShaderGraph = ron::from_str(&s).unwrap();
        assert_eq!(loaded.nodes.len(), g.nodes.len());
    }
}
