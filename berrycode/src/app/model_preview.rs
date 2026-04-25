//! 3D model preview for GLTF/GLB, OBJ, STL, and PLY files
//! Shows metadata and a wireframe projection
//! Supports Gaussian Splatting PLY files with per-splat color and opacity

use super::BerryCodeApp;

/// Data for a single Gaussian Splat point
#[derive(Clone)]
pub struct GaussianSplat {
    pub position: [f32; 3],
    pub color: [u8; 3],     // RGB
    pub opacity: f32,       // 0.0-1.0
    pub scale: [f32; 3],    // Full 3D scale
    pub rotation: [f32; 4], // Quaternion (w, x, y, z)
}

/// A triangle face with vertex indices and a base color.
#[derive(Clone)]
pub struct TriFace {
    pub idx: [usize; 3],
    pub color: [u8; 4], // RGBA
}

/// Skinning data per vertex (up to 4 joints).
#[derive(Clone, Default)]
pub struct SkinVertex {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

/// One animation clip with per-channel keyframes.
#[derive(Clone)]
pub struct AnimClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimChannel>,
}

/// A channel targets one node with one property (translation/rotation/scale).
#[derive(Clone)]
pub struct AnimChannel {
    pub node_index: usize,
    pub property: AnimProperty,
    pub times: Vec<f32>,
    pub values: Vec<[f32; 4]>, // xyz(w=0) for translation/scale, xyzw for rotation quat
}

#[derive(Clone, Copy, PartialEq)]
pub enum AnimProperty {
    Translation,
    Rotation,
    Scale,
}

/// Parsed 3D model data for preview
#[derive(Clone)]
pub struct ModelPreviewData {
    pub meshes: Vec<MeshInfo>,
    pub materials_count: usize,
    pub animations_count: usize,
    pub nodes_count: usize,
    pub vertices: Vec<[f32; 3]>,
    pub edges: Vec<(usize, usize)>,
    pub triangles: Vec<TriFace>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub splats: Vec<GaussianSplat>,
    // Skeletal animation data
    pub skin_vertices: Vec<SkinVertex>,
    pub joint_node_indices: Vec<usize>,
    pub inverse_bind_matrices: Vec<[[f32; 4]; 4]>,
    pub node_parents: Vec<Option<usize>>,
    pub node_transforms: Vec<[[f32; 4]; 4]>,
    pub anim_clips: Vec<AnimClip>,
}

#[derive(Clone)]
pub struct MeshInfo {
    pub name: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
}

/// PLY property metadata for byte offset calculation
struct PlyPropertyInfo {
    name: String,
    #[allow(dead_code)]
    type_name: String,
    byte_offset: usize,
    byte_size: usize,
}

/// Calculate byte offsets for each property in a PLY vertex
fn calculate_property_offsets(vertex_props: &[(String, String)]) -> Vec<PlyPropertyInfo> {
    let mut offset = 0;
    vertex_props
        .iter()
        .map(|(name, type_name)| {
            let size = match type_name.as_str() {
                "float" | "float32" | "int" | "int32" | "uint" | "uint32" => 4,
                "double" | "float64" | "int64" | "uint64" => 8,
                "short" | "int16" | "uint16" | "ushort" => 2,
                "char" | "int8" | "uint8" | "uchar" => 1,
                _ => 4,
            };
            let info = PlyPropertyInfo {
                name: name.clone(),
                type_name: type_name.clone(),
                byte_offset: offset,
                byte_size: size,
            };
            offset += size;
            info
        })
        .collect()
}

/// Find the byte offset of a named property
fn find_property_offset(props: &[PlyPropertyInfo], name: &str) -> Option<usize> {
    props.iter().find(|p| p.name == name).map(|p| p.byte_offset)
}

fn read_u32_le(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

fn read_f32_le(data: &[u8]) -> f32 {
    f32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

/// Compute 2D screen-space ellipse parameters from 3D scale and rotation
#[allow(dead_code)]
fn compute_2d_ellipse(
    scale: &[f32; 3],
    rotation: &[f32; 4], // w, x, y, z
    camera_rot_y: f32,
    _camera_rot_x: f32,
    view_scale: f32,
) -> (f32, f32, f32) {
    // (semi_axis_a, semi_axis_b, angle_radians)
    let (w, x, y, z) = (rotation[0], rotation[1], rotation[2], rotation[3]);

    let s0 = scale[0];
    let s1 = scale[1];

    let a = (s0 * view_scale).max(0.5);
    let b = (s1 * view_scale).max(0.5);

    // Simplified rotation angle from quaternion projected onto screen
    let angle = (2.0 * (w * z + x * y)).atan2(1.0 - 2.0 * (y * y + z * z));

    (a.min(30.0), b.min(30.0), angle + camera_rot_y)
}

/// Draw a rotated ellipse using a convex polygon approximation
#[allow(dead_code)]
fn draw_ellipse(
    painter: &egui::Painter,
    center: egui::Pos2,
    semi_a: f32,
    semi_b: f32,
    angle: f32,
    color: egui::Color32,
) {
    // Skip very small ellipses
    if semi_a < 0.3 && semi_b < 0.3 {
        return;
    }

    // For tiny splats, use circle instead (faster)
    if semi_a < 2.0 && semi_b < 2.0 {
        painter.circle_filled(center, (semi_a + semi_b) * 0.5, color);
        return;
    }

    // Generate ellipse polygon points
    let segments = if semi_a.max(semi_b) > 10.0 { 16 } else { 8 };
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let points: Vec<egui::Pos2> = (0..segments)
        .map(|i| {
            let t = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let ex = semi_a * t.cos();
            let ey = semi_b * t.sin();
            // Rotate
            let rx = ex * cos_a - ey * sin_a;
            let ry = ex * sin_a + ey * cos_a;
            egui::pos2(center.x + rx, center.y + ry)
        })
        .collect();

    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

impl BerryCodeApp {
    /// Load and parse a 3D model file (GLTF/GLB, OBJ, STL, PLY)
    pub(crate) fn load_model_data(file_path: &str) -> Option<ModelPreviewData> {
        let ext = file_path.rsplit('.').next()?.to_lowercase();
        match ext.as_str() {
            "gltf" | "glb" => Self::load_gltf(file_path),
            "obj" => Self::load_obj(file_path),
            "stl" => Self::load_stl(file_path),
            "ply" => Self::load_ply(file_path),
            _ => None,
        }
    }

    /// Load and parse a GLTF/GLB file
    fn load_gltf(file_path: &str) -> Option<ModelPreviewData> {
        let (document, buffers, images) = gltf::import(file_path).ok()?;

        // Pre-decode texture images for sampling
        let decoded_images: Vec<Option<(Vec<[u8; 4]>, u32, u32)>> = images
            .iter()
            .map(|img| {
                let w = img.width;
                let h = img.height;
                let pixels: Vec<[u8; 4]> = match img.format {
                    gltf::image::Format::R8G8B8A8 => img
                        .pixels
                        .chunks(4)
                        .map(|c| [c[0], c[1], c[2], c[3]])
                        .collect(),
                    gltf::image::Format::R8G8B8 => img
                        .pixels
                        .chunks(3)
                        .map(|c| [c[0], c[1], c[2], 255])
                        .collect(),
                    _ => return None,
                };
                Some((pixels, w, h))
            })
            .collect();

        let mut all_vertices: Vec<[f32; 3]> = Vec::new();
        let mut all_edges: Vec<(usize, usize)> = Vec::new();
        let mut all_triangles: Vec<TriFace> = Vec::new();
        let mut meshes_info: Vec<MeshInfo> = Vec::new();
        let mut bounds_min = [f32::MAX; 3];
        let mut bounds_max = [f32::MIN; 3];

        for mesh in document.meshes() {
            let mesh_name = mesh.name().unwrap_or("unnamed").to_string();
            let mut mesh_vertex_count = 0;
            let mut mesh_triangle_count = 0;

            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // Extract material base color + texture
                let mat = primitive.material();
                let pbr = mat.pbr_metallic_roughness();
                let base = pbr.base_color_factor();
                let mat_color = [
                    (base[0] * 255.0) as u8,
                    (base[1] * 255.0) as u8,
                    (base[2] * 255.0) as u8,
                    (base[3] * 255.0) as u8,
                ];

                // Get texture image if available
                let tex_image: Option<&(Vec<[u8; 4]>, u32, u32)> =
                    pbr.base_color_texture().and_then(|t| {
                        let idx = t.texture().source().index();
                        decoded_images.get(idx).and_then(|o| o.as_ref())
                    });

                // Read UVs
                let uvs: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|tc| tc.into_f32().collect())
                    .unwrap_or_default();

                if let Some(positions) = reader.read_positions() {
                    let base_idx = all_vertices.len();
                    let verts: Vec<[f32; 3]> = positions.collect();
                    mesh_vertex_count += verts.len();

                    for v in &verts {
                        for i in 0..3 {
                            bounds_min[i] = bounds_min[i].min(v[i]);
                            bounds_max[i] = bounds_max[i].max(v[i]);
                        }
                    }

                    all_vertices.extend_from_slice(&verts);

                    // Build triangle indices
                    let tri_indices: Vec<[usize; 3]> = if let Some(indices) = reader.read_indices()
                    {
                        let indices: Vec<u32> = indices.into_u32().collect();
                        mesh_triangle_count += indices.len() / 3;
                        indices
                            .chunks(3)
                            .filter(|t| t.len() == 3)
                            .map(|t| {
                                [
                                    base_idx + t[0] as usize,
                                    base_idx + t[1] as usize,
                                    base_idx + t[2] as usize,
                                ]
                            })
                            .collect()
                    } else {
                        let tri_count = verts.len() / 3;
                        mesh_triangle_count += tri_count;
                        (0..tri_count)
                            .map(|t| [base_idx + t * 3, base_idx + t * 3 + 1, base_idx + t * 3 + 2])
                            .collect()
                    };

                    for idx in &tri_indices {
                        // Sample texture at triangle centroid UV
                        let color = if let Some((pixels, w, h)) = tex_image {
                            let uv_avg = |vi: usize| -> [f32; 2] {
                                let local_idx = vi.checked_sub(base_idx).unwrap_or(0);
                                if local_idx < uvs.len() {
                                    uvs[local_idx]
                                } else {
                                    [0.5, 0.5]
                                }
                            };
                            let uv0 = uv_avg(idx[0]);
                            let uv1 = uv_avg(idx[1]);
                            let uv2 = uv_avg(idx[2]);
                            let u = ((uv0[0] + uv1[0] + uv2[0]) / 3.0).fract().abs();
                            let v = ((uv0[1] + uv1[1] + uv2[1]) / 3.0).fract().abs();
                            let px = (u * (*w as f32 - 1.0)) as usize;
                            let py = (v * (*h as f32 - 1.0)) as usize;
                            let pi = py * *w as usize + px;
                            if pi < pixels.len() {
                                pixels[pi]
                            } else {
                                mat_color
                            }
                        } else {
                            mat_color
                        };

                        all_edges.push((idx[0], idx[1]));
                        all_edges.push((idx[1], idx[2]));
                        all_edges.push((idx[2], idx[0]));
                        all_triangles.push(TriFace { idx: *idx, color });
                    }
                }
            }

            meshes_info.push(MeshInfo {
                name: mesh_name,
                vertex_count: mesh_vertex_count,
                triangle_count: mesh_triangle_count,
            });
        }

        // Extract skeleton data
        let mut skin_vertices: Vec<SkinVertex> = vec![SkinVertex::default(); all_vertices.len()];
        let mut joint_node_indices: Vec<usize> = Vec::new();
        let mut inverse_bind_matrices: Vec<[[f32; 4]; 4]> = Vec::new();

        // Re-read skinning attributes (joints + weights per vertex)
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                // We need base_idx for this primitive - recompute from positions
                // Since vertices were added in order, count up
                if let Some(joints) = reader.read_joints(0) {
                    let weights: Vec<[f32; 4]> = reader
                        .read_weights(0)
                        .map(|w| w.into_f32().collect())
                        .unwrap_or_default();
                    for (i, j) in joints.into_u16().enumerate() {
                        if i < skin_vertices.len() {
                            skin_vertices[i].joints = j;
                            if i < weights.len() {
                                skin_vertices[i].weights = weights[i];
                            }
                        }
                    }
                }
            }
        }

        // Extract skin joint indices and inverse bind matrices
        if let Some(skin) = document.skins().next() {
            joint_node_indices = skin.joints().map(|j| j.index()).collect();
            if let Some(accessor) = skin.inverse_bind_matrices() {
                let reader_ibm =
                    gltf::accessor::util::Iter::<[[f32; 4]; 4]>::new(accessor, |buffer| {
                        Some(&buffers[buffer.index()])
                    });
                if let Some(reader_ibm) = reader_ibm {
                    inverse_bind_matrices = reader_ibm.collect();
                }
            }
        }

        // Build node hierarchy
        let node_count = document.nodes().count();
        let mut node_parents: Vec<Option<usize>> = vec![None; node_count];
        let mut node_transforms: Vec<[[f32; 4]; 4]> = Vec::with_capacity(node_count);
        for node in document.nodes() {
            let mat = node.transform().matrix();
            node_transforms.push(mat);
            for child in node.children() {
                if child.index() < node_count {
                    node_parents[child.index()] = Some(node.index());
                }
            }
        }

        // Extract animation clips
        let mut anim_clips: Vec<AnimClip> = Vec::new();
        for anim in document.animations() {
            let name = anim.name().unwrap_or("unnamed").to_string();
            let mut duration: f32 = 0.0;
            let mut channels: Vec<AnimChannel> = Vec::new();

            for channel in anim.channels() {
                let target = channel.target();
                let node_index = target.node().index();
                let property = match target.property() {
                    gltf::animation::Property::Translation => AnimProperty::Translation,
                    gltf::animation::Property::Rotation => AnimProperty::Rotation,
                    gltf::animation::Property::Scale => AnimProperty::Scale,
                    _ => continue,
                };

                let sampler = channel.sampler();
                let input_accessor = sampler.input();
                let output_accessor = sampler.output();

                // Read keyframe times
                let times: Vec<f32> =
                    gltf::accessor::util::Iter::<f32>::new(input_accessor, |buffer| {
                        Some(&buffers[buffer.index()])
                    })
                    .map(|iter| iter.collect())
                    .unwrap_or_default();

                if let Some(&last) = times.last() {
                    duration = duration.max(last);
                }

                // Read keyframe values
                let values: Vec<[f32; 4]> = match property {
                    AnimProperty::Translation | AnimProperty::Scale => {
                        gltf::accessor::util::Iter::<[f32; 3]>::new(output_accessor, |buffer| {
                            Some(&buffers[buffer.index()])
                        })
                        .map(|iter| iter.map(|v| [v[0], v[1], v[2], 0.0]).collect())
                        .unwrap_or_default()
                    }
                    AnimProperty::Rotation => {
                        gltf::accessor::util::Iter::<[f32; 4]>::new(output_accessor, |buffer| {
                            Some(&buffers[buffer.index()])
                        })
                        .map(|iter| iter.collect())
                        .unwrap_or_default()
                    }
                };

                channels.push(AnimChannel {
                    node_index,
                    property,
                    times,
                    values,
                });
            }

            anim_clips.push(AnimClip {
                name,
                duration,
                channels,
            });
        }

        Some(ModelPreviewData {
            meshes: meshes_info,
            materials_count: document.materials().count(),
            animations_count: document.animations().count(),
            nodes_count: document.nodes().count(),
            vertices: all_vertices,
            edges: all_edges,
            triangles: all_triangles,
            bounds_min,
            bounds_max,
            splats: Vec::new(),
            skin_vertices,
            joint_node_indices,
            inverse_bind_matrices,
            node_parents,
            node_transforms,
            anim_clips,
        })
    }

    /// Load and parse an OBJ file using tobj
    fn load_obj(file_path: &str) -> Option<ModelPreviewData> {
        let (models, materials_result) = tobj::load_obj(file_path, &tobj::GPU_LOAD_OPTIONS).ok()?;

        let mut all_vertices: Vec<[f32; 3]> = Vec::new();
        let mut all_edges: Vec<(usize, usize)> = Vec::new();
        let mut meshes_info: Vec<MeshInfo> = Vec::new();
        let mut bounds_min = [f32::MAX; 3];
        let mut bounds_max = [f32::MIN; 3];

        for model in &models {
            let mesh = &model.mesh;
            let base_idx = all_vertices.len();
            let vertex_count = mesh.positions.len() / 3;

            for i in 0..vertex_count {
                let v = [
                    mesh.positions[i * 3],
                    mesh.positions[i * 3 + 1],
                    mesh.positions[i * 3 + 2],
                ];
                for j in 0..3 {
                    bounds_min[j] = bounds_min[j].min(v[j]);
                    bounds_max[j] = bounds_max[j].max(v[j]);
                }
                all_vertices.push(v);
            }

            let triangle_count = mesh.indices.len() / 3;
            for tri in mesh.indices.chunks(3) {
                if tri.len() == 3 {
                    let i0 = base_idx + tri[0] as usize;
                    let i1 = base_idx + tri[1] as usize;
                    let i2 = base_idx + tri[2] as usize;
                    all_edges.push((i0, i1));
                    all_edges.push((i1, i2));
                    all_edges.push((i2, i0));
                }
            }

            meshes_info.push(MeshInfo {
                name: model.name.clone(),
                vertex_count,
                triangle_count,
            });
        }

        let materials_count = materials_result.map(|m| m.len()).unwrap_or(0);

        Some(ModelPreviewData {
            meshes: meshes_info,
            materials_count,
            animations_count: 0,
            nodes_count: models.len(),
            vertices: all_vertices,
            edges: all_edges,
            triangles: vec![],
            bounds_min,
            bounds_max,
            splats: Vec::new(),
            skin_vertices: vec![],
            joint_node_indices: vec![],
            inverse_bind_matrices: vec![],
            node_parents: vec![],
            node_transforms: vec![],
            anim_clips: vec![],
        })
    }

    /// Load and parse an STL file (binary + ASCII)
    fn load_stl(file_path: &str) -> Option<ModelPreviewData> {
        let data = std::fs::read(file_path).ok()?;

        let mut all_vertices: Vec<[f32; 3]> = Vec::new();
        let mut all_edges: Vec<(usize, usize)> = Vec::new();
        let mut bounds_min = [f32::MAX; 3];
        let mut bounds_max = [f32::MIN; 3];

        // Check if binary STL (starts with 80-byte header, then u32 triangle count)
        let is_binary = data.len() > 84 && {
            let text_start = String::from_utf8_lossy(&data[..5]);
            !text_start.starts_with("solid")
                || data.len() == 84 + read_u32_le(&data[80..84]) as usize * 50
        };

        if is_binary && data.len() > 84 {
            let num_triangles = read_u32_le(&data[80..84]) as usize;
            let mut offset = 84;

            for _ in 0..num_triangles {
                if offset + 50 > data.len() {
                    break;
                }
                // Skip normal (12 bytes)
                offset += 12;

                let base = all_vertices.len();
                for _ in 0..3 {
                    let x = read_f32_le(&data[offset..]);
                    let y = read_f32_le(&data[offset + 4..]);
                    let z = read_f32_le(&data[offset + 8..]);
                    let v = [x, y, z];
                    for j in 0..3 {
                        bounds_min[j] = bounds_min[j].min(v[j]);
                        bounds_max[j] = bounds_max[j].max(v[j]);
                    }
                    all_vertices.push(v);
                    offset += 12;
                }
                offset += 2; // attribute byte count

                all_edges.push((base, base + 1));
                all_edges.push((base + 1, base + 2));
                all_edges.push((base + 2, base));
            }
        } else {
            // ASCII STL
            let text = String::from_utf8_lossy(&data);
            let mut current_tri_verts: Vec<[f32; 3]> = Vec::new();

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("vertex ") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse().unwrap_or(0.0);
                        let y: f32 = parts[2].parse().unwrap_or(0.0);
                        let z: f32 = parts[3].parse().unwrap_or(0.0);
                        let v = [x, y, z];
                        for j in 0..3 {
                            bounds_min[j] = bounds_min[j].min(v[j]);
                            bounds_max[j] = bounds_max[j].max(v[j]);
                        }
                        current_tri_verts.push(v);
                    }
                } else if trimmed == "endfacet" && current_tri_verts.len() == 3 {
                    let base = all_vertices.len();
                    all_vertices.extend_from_slice(&current_tri_verts);
                    all_edges.push((base, base + 1));
                    all_edges.push((base + 1, base + 2));
                    all_edges.push((base + 2, base));
                    current_tri_verts.clear();
                }
            }
        }

        let triangle_count = all_edges.len() / 3;
        Some(ModelPreviewData {
            meshes: vec![MeshInfo {
                name: file_path.rsplit('/').next().unwrap_or("model").to_string(),
                vertex_count: all_vertices.len(),
                triangle_count,
            }],
            materials_count: 0,
            animations_count: 0,
            nodes_count: 1,
            vertices: all_vertices,
            edges: all_edges,
            triangles: vec![],
            bounds_min,
            bounds_max,
            splats: Vec::new(),
            skin_vertices: vec![],
            joint_node_indices: vec![],
            inverse_bind_matrices: vec![],
            node_parents: vec![],
            node_transforms: vec![],
            anim_clips: vec![],
        })
    }

    /// Load and parse a PLY file (ASCII + binary little-endian)
    /// Detects Gaussian Splatting PLY files and parses per-splat color/opacity/scale
    fn load_ply(file_path: &str) -> Option<ModelPreviewData> {
        let data = std::fs::read(file_path).ok()?;

        // Find end_header by scanning raw bytes (works for both ASCII and binary)
        let header_end_marker = b"end_header\n";
        let header_end_marker_r = b"end_header\r\n";
        let header_end_offset = if let Some(pos) = data
            .windows(header_end_marker.len())
            .position(|w| w == header_end_marker)
        {
            pos + header_end_marker.len()
        } else if let Some(pos) = data
            .windows(header_end_marker_r.len())
            .position(|w| w == header_end_marker_r)
        {
            pos + header_end_marker_r.len()
        } else {
            return None;
        };

        let header_text = String::from_utf8_lossy(&data[..header_end_offset]);

        // Parse header
        let mut vertex_count: usize = 0;
        let mut face_count: usize = 0;
        let mut is_binary_le = false;
        let mut is_binary_be = false;
        let mut vertex_props: Vec<(String, String)> = Vec::new(); // (name, type)
        let mut in_vertex_element = false;
        #[allow(unused_assignments)]
        let mut _in_face_element = false;

        for line in header_text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("element vertex ") {
                vertex_count = trimmed
                    .split_whitespace()
                    .nth(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                in_vertex_element = true;
                _in_face_element = false;
            } else if trimmed.starts_with("element face ") {
                face_count = trimmed
                    .split_whitespace()
                    .nth(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                in_vertex_element = false;
                _in_face_element = true;
            } else if trimmed.starts_with("element ") {
                in_vertex_element = false;
                _in_face_element = false;
            } else if trimmed.starts_with("property ")
                && in_vertex_element
                && !trimmed.contains("list")
            {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    vertex_props.push((parts[2].to_string(), parts[1].to_string()));
                }
            } else if trimmed.starts_with("format binary_little_endian") {
                is_binary_le = true;
            } else if trimmed.starts_with("format binary_big_endian") {
                is_binary_be = true;
            }
        }

        let is_binary = is_binary_le || is_binary_be;

        // Detect Gaussian Splatting PLY by checking for characteristic properties
        let prop_names: Vec<&str> = vertex_props.iter().map(|(n, _)| n.as_str()).collect();
        let is_gaussian_splatting = prop_names.contains(&"f_dc_0")
            || (prop_names.contains(&"opacity") && prop_names.contains(&"rot_0"));

        // Calculate property byte offsets for binary mode
        let prop_infos: Vec<PlyPropertyInfo> = calculate_property_offsets(&vertex_props);
        let vertex_stride: usize = if is_binary {
            prop_infos
                .last()
                .map(|p| p.byte_offset + p.byte_size)
                .unwrap_or(0)
        } else {
            0
        };

        let mut all_vertices: Vec<[f32; 3]> = Vec::new();
        let mut all_edges: Vec<(usize, usize)> = Vec::new();
        let mut splats: Vec<GaussianSplat> = Vec::new();
        let mut bounds_min = [f32::MAX; 3];
        let mut bounds_max = [f32::MIN; 3];

        if is_gaussian_splatting {
            // Parse as Gaussian Splatting PLY
            let off_x = find_property_offset(&prop_infos, "x");
            let off_y = find_property_offset(&prop_infos, "y");
            let off_z = find_property_offset(&prop_infos, "z");
            let off_fdc0 = find_property_offset(&prop_infos, "f_dc_0");
            let off_fdc1 = find_property_offset(&prop_infos, "f_dc_1");
            let off_fdc2 = find_property_offset(&prop_infos, "f_dc_2");
            let off_opacity = find_property_offset(&prop_infos, "opacity");
            let off_scale0 = find_property_offset(&prop_infos, "scale_0");
            let off_scale1 = find_property_offset(&prop_infos, "scale_1");
            let off_scale2 = find_property_offset(&prop_infos, "scale_2");
            let off_rot0 = find_property_offset(&prop_infos, "rot_0");
            let off_rot1 = find_property_offset(&prop_infos, "rot_1");
            let off_rot2 = find_property_offset(&prop_infos, "rot_2");
            let off_rot3 = find_property_offset(&prop_infos, "rot_3");

            // Find column indices for ASCII mode
            let col_x = vertex_props.iter().position(|(n, _)| n == "x");
            let col_y = vertex_props.iter().position(|(n, _)| n == "y");
            let col_z = vertex_props.iter().position(|(n, _)| n == "z");
            let col_fdc0 = vertex_props.iter().position(|(n, _)| n == "f_dc_0");
            let col_fdc1 = vertex_props.iter().position(|(n, _)| n == "f_dc_1");
            let col_fdc2 = vertex_props.iter().position(|(n, _)| n == "f_dc_2");
            let col_opacity = vertex_props.iter().position(|(n, _)| n == "opacity");
            let col_scale0 = vertex_props.iter().position(|(n, _)| n == "scale_0");
            let col_scale1 = vertex_props.iter().position(|(n, _)| n == "scale_1");
            let col_scale2 = vertex_props.iter().position(|(n, _)| n == "scale_2");
            let col_rot0 = vertex_props.iter().position(|(n, _)| n == "rot_0");
            let col_rot1 = vertex_props.iter().position(|(n, _)| n == "rot_1");
            let col_rot2 = vertex_props.iter().position(|(n, _)| n == "rot_2");
            let col_rot3 = vertex_props.iter().position(|(n, _)| n == "rot_3");

            /// SH DC component to RGB conversion constant (C0)
            const C0: f32 = 0.28209479177387814;

            /// Sigmoid function for opacity decoding
            #[inline]
            fn sigmoid(x: f32) -> f32 {
                1.0 / (1.0 + (-x).exp())
            }

            /// Convert SH DC component to 0-255 color channel
            #[inline]
            fn sh_to_u8(f_dc: f32) -> u8 {
                ((0.5 + C0 * f_dc) * 255.0).clamp(0.0, 255.0) as u8
            }

            splats.reserve(vertex_count);

            if is_binary {
                let mut offset = header_end_offset;
                for _ in 0..vertex_count {
                    if offset + vertex_stride > data.len() {
                        break;
                    }

                    let x = off_x
                        .map(|o| read_f32_le(&data[offset + o..]))
                        .unwrap_or(0.0);
                    let y = off_y
                        .map(|o| read_f32_le(&data[offset + o..]))
                        .unwrap_or(0.0);
                    let z = off_z
                        .map(|o| read_f32_le(&data[offset + o..]))
                        .unwrap_or(0.0);

                    let fdc0 = off_fdc0
                        .map(|o| read_f32_le(&data[offset + o..]))
                        .unwrap_or(0.0);
                    let fdc1 = off_fdc1
                        .map(|o| read_f32_le(&data[offset + o..]))
                        .unwrap_or(0.0);
                    let fdc2 = off_fdc2
                        .map(|o| read_f32_le(&data[offset + o..]))
                        .unwrap_or(0.0);

                    let raw_opacity = off_opacity
                        .map(|o| read_f32_le(&data[offset + o..]))
                        .unwrap_or(0.0);

                    let position = [x, y, z];
                    for j in 0..3 {
                        bounds_min[j] = bounds_min[j].min(position[j]);
                        bounds_max[j] = bounds_max[j].max(position[j]);
                    }

                    let scale = [
                        off_scale0
                            .map(|o| read_f32_le(&data[offset + o..]).exp())
                            .unwrap_or(0.01),
                        off_scale1
                            .map(|o| read_f32_le(&data[offset + o..]).exp())
                            .unwrap_or(0.01),
                        off_scale2
                            .map(|o| read_f32_le(&data[offset + o..]).exp())
                            .unwrap_or(0.01),
                    ];
                    let rotation = [
                        off_rot0
                            .map(|o| read_f32_le(&data[offset + o..]))
                            .unwrap_or(1.0), // w
                        off_rot1
                            .map(|o| read_f32_le(&data[offset + o..]))
                            .unwrap_or(0.0), // x
                        off_rot2
                            .map(|o| read_f32_le(&data[offset + o..]))
                            .unwrap_or(0.0), // y
                        off_rot3
                            .map(|o| read_f32_le(&data[offset + o..]))
                            .unwrap_or(0.0), // z
                    ];

                    let color = [sh_to_u8(fdc0), sh_to_u8(fdc1), sh_to_u8(fdc2)];

                    // Debug: log first splat's color values
                    if splats.is_empty() {
                        tracing::info!(
                            "First splat: f_dc=({:.3},{:.3},{:.3}) -> RGB=({},{},{}) opacity_raw={:.3} -> {:.3} scale=({:.4},{:.4},{:.4})",
                            fdc0, fdc1, fdc2, color[0], color[1], color[2],
                            raw_opacity, sigmoid(raw_opacity),
                            scale[0], scale[1], scale[2]
                        );
                        tracing::info!(
                            "Property offsets: fdc0={:?} fdc1={:?} fdc2={:?} opacity={:?} x={:?} y={:?} z={:?}",
                            off_fdc0, off_fdc1, off_fdc2, off_opacity, off_x, off_y, off_z
                        );
                    }

                    splats.push(GaussianSplat {
                        position,
                        color,
                        opacity: sigmoid(raw_opacity),
                        scale,
                        rotation,
                    });

                    all_vertices.push(position);
                    offset += vertex_stride;
                }
            } else {
                // ASCII Gaussian Splatting PLY
                let body_text = String::from_utf8_lossy(&data[header_end_offset..]);
                let mut body_lines = body_text.lines();

                for _ in 0..vertex_count {
                    if let Some(line) = body_lines.next() {
                        let parts: Vec<&str> = line.trim().split_whitespace().collect();

                        let parse_col = |col: Option<usize>| -> f32 {
                            col.and_then(|c| parts.get(c))
                                .and_then(|s| s.parse::<f32>().ok())
                                .unwrap_or(0.0)
                        };

                        let x = parse_col(col_x);
                        let y = parse_col(col_y);
                        let z = parse_col(col_z);
                        let fdc0 = parse_col(col_fdc0);
                        let fdc1 = parse_col(col_fdc1);
                        let fdc2 = parse_col(col_fdc2);
                        let raw_opacity = parse_col(col_opacity);
                        let s0 = parse_col(col_scale0);
                        let s1 = parse_col(col_scale1);
                        let s2 = parse_col(col_scale2);
                        let r0 = parse_col(col_rot0);
                        let r1 = parse_col(col_rot1);
                        let r2 = parse_col(col_rot2);
                        let r3 = parse_col(col_rot3);

                        let position = [x, y, z];
                        for j in 0..3 {
                            bounds_min[j] = bounds_min[j].min(position[j]);
                            bounds_max[j] = bounds_max[j].max(position[j]);
                        }

                        let scale = [s0.exp(), s1.exp(), s2.exp()];
                        let rotation = [
                            if col_rot0.is_some() { r0 } else { 1.0 },
                            if col_rot1.is_some() { r1 } else { 0.0 },
                            if col_rot2.is_some() { r2 } else { 0.0 },
                            if col_rot3.is_some() { r3 } else { 0.0 },
                        ];

                        splats.push(GaussianSplat {
                            position,
                            color: [sh_to_u8(fdc0), sh_to_u8(fdc1), sh_to_u8(fdc2)],
                            opacity: sigmoid(raw_opacity),
                            scale,
                            rotation,
                        });

                        all_vertices.push(position);
                    }
                }
            }

            Some(ModelPreviewData {
                meshes: vec![MeshInfo {
                    name: file_path.rsplit('/').next().unwrap_or("model").to_string(),
                    vertex_count: all_vertices.len(),
                    triangle_count: 0,
                }],
                materials_count: 0,
                animations_count: 0,
                nodes_count: 1,
                vertices: all_vertices,
                edges: Vec::new(),
                triangles: vec![],
                bounds_min,
                bounds_max,
                splats,
                skin_vertices: vec![],
                joint_node_indices: vec![],
                inverse_bind_matrices: vec![],
                node_parents: vec![],
                node_transforms: vec![],
                anim_clips: vec![],
            })
        } else {
            // Standard PLY (non-Gaussian Splatting)
            if is_binary {
                let mut offset = header_end_offset;
                for _ in 0..vertex_count {
                    if offset + vertex_stride > data.len() {
                        break;
                    }
                    let x = read_f32_le(&data[offset..]);
                    let y = read_f32_le(&data[offset + 4..]);
                    let z = read_f32_le(&data[offset + 8..]);
                    let v = [x, y, z];
                    for j in 0..3 {
                        bounds_min[j] = bounds_min[j].min(v[j]);
                        bounds_max[j] = bounds_max[j].max(v[j]);
                    }
                    all_vertices.push(v);
                    offset += vertex_stride;
                }

                for _ in 0..face_count {
                    if offset >= data.len() {
                        break;
                    }
                    let count = data[offset] as usize;
                    offset += 1;
                    if offset + count * 4 > data.len() {
                        break;
                    }
                    let mut indices = Vec::with_capacity(count);
                    for _ in 0..count {
                        indices.push(read_u32_le(&data[offset..]) as usize);
                        offset += 4;
                    }
                    for i in 1..indices.len().saturating_sub(1) {
                        all_edges.push((indices[0], indices[i]));
                        all_edges.push((indices[i], indices[i + 1]));
                        all_edges.push((indices[i + 1], indices[0]));
                    }
                }
            } else {
                let body_text = String::from_utf8_lossy(&data[header_end_offset..]);
                let mut body_lines = body_text.lines();

                for _ in 0..vertex_count {
                    if let Some(line) = body_lines.next() {
                        let parts: Vec<&str> = line.trim().split_whitespace().collect();
                        if parts.len() >= 3 {
                            let x: f32 = parts[0].parse().unwrap_or(0.0);
                            let y: f32 = parts[1].parse().unwrap_or(0.0);
                            let z: f32 = parts[2].parse().unwrap_or(0.0);
                            let v = [x, y, z];
                            for j in 0..3 {
                                bounds_min[j] = bounds_min[j].min(v[j]);
                                bounds_max[j] = bounds_max[j].max(v[j]);
                            }
                            all_vertices.push(v);
                        }
                    }
                }

                for _ in 0..face_count {
                    if let Some(line) = body_lines.next() {
                        let parts: Vec<&str> = line.trim().split_whitespace().collect();
                        if !parts.is_empty() {
                            let count: usize = parts[0].parse().unwrap_or(0);
                            if count >= 3 && parts.len() >= count + 1 {
                                let indices: Vec<usize> = parts[1..=count]
                                    .iter()
                                    .filter_map(|s| s.parse().ok())
                                    .collect();
                                for i in 1..indices.len().saturating_sub(1) {
                                    all_edges.push((indices[0], indices[i]));
                                    all_edges.push((indices[i], indices[i + 1]));
                                    all_edges.push((indices[i + 1], indices[0]));
                                }
                            }
                        }
                    }
                }
            }

            let triangle_count = all_edges.len() / 3;
            Some(ModelPreviewData {
                meshes: vec![MeshInfo {
                    name: file_path.rsplit('/').next().unwrap_or("model").to_string(),
                    vertex_count: all_vertices.len(),
                    triangle_count,
                }],
                materials_count: 0,
                animations_count: 0,
                nodes_count: 1,
                vertices: all_vertices,
                edges: all_edges,
                triangles: vec![],
                bounds_min,
                bounds_max,
                splats: Vec::new(),
                skin_vertices: vec![],
                joint_node_indices: vec![],
                inverse_bind_matrices: vec![],
                node_parents: vec![],
                node_transforms: vec![],
                anim_clips: vec![],
            })
        }
    }

    /// Render 3D model preview
    pub(crate) fn render_model_preview(&mut self, ui: &mut egui::Ui) {
        let tab = &mut self.editor_tabs[self.active_tab_idx];

        // GPU-accelerated preview for GLB/GLTF via Bevy's PBR renderer
        let ext = tab
            .file_path
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if false && (ext == "glb" || ext == "gltf") && tab.gpu_preview_texture_id.is_some() {
            let texture_id = tab.gpu_preview_texture_id.unwrap();

            // Metadata header
            ui.horizontal(|ui| {
                ui.heading("3D Model Preview (GPU)");
                ui.separator();
                let file_size = std::fs::metadata(&tab.file_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                if file_size < 1024 * 1024 {
                    ui.label(format!("{:.1} KB", file_size as f64 / 1024.0));
                } else {
                    ui.label(format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0)));
                }
            });
            ui.separator();
            ui.label("Bevy PBR renderer -- drag to orbit, scroll to zoom, double-click to reset");
            ui.separator();

            // Allocate the preview area with interaction
            let available = ui.available_size();
            let preview_size = available.x.min(available.y - 20.0).max(200.0);
            let (response, _painter) = ui.allocate_painter(
                egui::vec2(available.x, preview_size),
                egui::Sense::click_and_drag(),
            );
            let rect = response.rect;

            // Handle mouse interaction for orbit camera
            // We store orbit params on the tab's existing rotation fields
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                tab.model_rot_y += delta.x * 0.01;
                tab.model_rot_x += delta.y * 0.01;
                tab.model_rot_x = tab.model_rot_x.clamp(
                    -std::f32::consts::FRAC_PI_2 + 0.1,
                    std::f32::consts::FRAC_PI_2 - 0.1,
                );
                ui.ctx().request_repaint();
            }

            // Scroll to zoom
            let scroll_delta = ui.input(|i| {
                if let Some(pos) = i.pointer.hover_pos() {
                    if rect.contains(pos) {
                        i.smooth_scroll_delta.y
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            });
            if scroll_delta != 0.0 {
                tab.model_zoom *= 1.0 + scroll_delta * 0.002;
                tab.model_zoom = tab.model_zoom.clamp(0.1, 20.0);
                ui.ctx().request_repaint();
            }

            // Double-click to reset view
            if response.double_clicked() {
                tab.model_rot_y = std::f32::consts::FRAC_PI_4;
                tab.model_rot_x = 0.3;
                tab.model_zoom = 1.0;
            }

            // Display the GPU-rendered texture
            let display_size = egui::vec2(rect.width(), rect.height());
            let image_rect = egui::Rect::from_min_size(rect.min, display_size);
            ui.painter().image(
                texture_id,
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

            return;
        }

        // Load model data on first render
        if tab.model_data.is_none() {
            tab.model_data = Self::load_model_data(&tab.file_path);
        }

        match &tab.model_data {
            Some(data) => {
                // Metadata header
                ui.horizontal(|ui| {
                    ui.heading("3D Model Preview");
                    ui.separator();
                    let file_size = std::fs::metadata(&tab.file_path)
                        .map(|m| m.len())
                        .unwrap_or(0);
                    if file_size < 1024 * 1024 {
                        ui.label(format!("{:.1} KB", file_size as f64 / 1024.0));
                    } else {
                        ui.label(format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0)));
                    }
                });
                ui.separator();

                // Stats
                ui.horizontal(|ui| {
                    ui.label(format!("Meshes: {}", data.meshes.len()));
                    ui.separator();
                    ui.label(format!("Materials: {}", data.materials_count));
                    ui.separator();
                    ui.label(format!("Animations: {}", data.animations_count));
                    ui.separator();
                    ui.label(format!("Nodes: {}", data.nodes_count));
                    ui.separator();
                    let total_verts: usize = data.meshes.iter().map(|m| m.vertex_count).sum();
                    let total_tris: usize = data.meshes.iter().map(|m| m.triangle_count).sum();
                    ui.label(format!("Vertices: {}", total_verts));
                    ui.separator();
                    ui.label(format!("Triangles: {}", total_tris));
                });

                // Mesh list
                ui.separator();
                ui.label("Meshes:");
                for mesh in &data.meshes {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "  {} -- {} verts, {} tris",
                            mesh.name, mesh.vertex_count, mesh.triangle_count
                        ));
                    });
                }

                ui.separator();

                // Wireframe preview with interactive camera
                let available = ui.available_size();
                let preview_size = available.x.min(available.y - 20.0).max(200.0);
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(available.x, preview_size),
                    egui::Sense::click_and_drag(),
                );
                let rect = response.rect;

                // Handle mouse interaction for orbit camera
                if response.dragged_by(egui::PointerButton::Primary) {
                    let delta = response.drag_delta();
                    tab.model_rot_y += delta.x * 0.01;
                    tab.model_rot_x += delta.y * 0.01;
                    // Clamp pitch to avoid flipping
                    tab.model_rot_x = tab.model_rot_x.clamp(
                        -std::f32::consts::FRAC_PI_2 + 0.1,
                        std::f32::consts::FRAC_PI_2 - 0.1,
                    );
                }

                // Scroll to zoom
                let scroll_delta = ui.input(|i| {
                    if let Some(pos) = i.pointer.hover_pos() {
                        if rect.contains(pos) {
                            i.smooth_scroll_delta.y
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    }
                });
                if scroll_delta != 0.0 {
                    tab.model_zoom *= 1.0 + scroll_delta * 0.002;
                    tab.model_zoom = tab.model_zoom.clamp(0.1, 20.0);
                }

                // Double-click to reset view
                if response.double_clicked() {
                    tab.model_rot_y = std::f32::consts::PI * 0.25;
                    tab.model_rot_x = std::f32::consts::PI * 0.15;
                    tab.model_zoom = 1.0;
                }

                // Dark background
                painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 25));

                // Grid
                let grid_color = egui::Color32::from_rgba_premultiplied(255, 255, 255, 15);
                let grid_count = 8;
                for i in 0..=grid_count {
                    let t = i as f32 / grid_count as f32;
                    let x = rect.min.x + t * rect.width();
                    let y = rect.min.y + t * rect.height();
                    painter.line_segment(
                        [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                        egui::Stroke::new(0.5, grid_color),
                    );
                    painter.line_segment(
                        [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                        egui::Stroke::new(0.5, grid_color),
                    );
                }

                // Controls hint
                painter.text(
                    egui::pos2(rect.max.x - 8.0, rect.min.y + 12.0),
                    egui::Align2::RIGHT_TOP,
                    "Drag: rotate | Scroll: zoom | Double-click: reset",
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_rgb(80, 80, 80),
                );

                if !data.vertices.is_empty() {
                    // Interactive camera rotation
                    let angle_y = tab.model_rot_y;
                    let angle_x = tab.model_rot_x;
                    let zoom = tab.model_zoom;

                    // Calculate model center and scale
                    let center = [
                        (data.bounds_min[0] + data.bounds_max[0]) / 2.0,
                        (data.bounds_min[1] + data.bounds_max[1]) / 2.0,
                        (data.bounds_min[2] + data.bounds_max[2]) / 2.0,
                    ];
                    let extent = [
                        data.bounds_max[0] - data.bounds_min[0],
                        data.bounds_max[1] - data.bounds_min[1],
                        data.bounds_max[2] - data.bounds_min[2],
                    ];
                    let max_extent = extent[0].max(extent[1]).max(extent[2]).max(0.001);
                    let scale = (preview_size * 0.35) / max_extent * zoom;

                    let cos_y = angle_y.cos();
                    let sin_y = angle_y.sin();
                    let cos_x = angle_x.cos();
                    let sin_x = angle_x.sin();

                    // Project 3D -> 2D with depth
                    let project = |v: &[f32; 3]| -> egui::Pos2 {
                        let x = v[0] - center[0];
                        let y = v[1] - center[1];
                        let z = v[2] - center[2];

                        // Y-axis rotation (yaw)
                        let rx = x * cos_y - z * sin_y;
                        let rz = x * sin_y + z * cos_y;
                        let ry = y;

                        // X-axis rotation (pitch)
                        let fy = ry * cos_x - rz * sin_x;

                        let screen_x = rect.center().x + rx * scale;
                        let screen_y = rect.center().y - fy * scale;

                        egui::pos2(screen_x, screen_y)
                    };

                    // Request repaint while dragging for smooth rotation
                    if response.dragged() {
                        ui.ctx().request_repaint();
                    }

                    // Render Gaussian Splats if present
                    if !data.splats.is_empty() {
                        // Camera direction for depth sorting
                        let cam_fwd = [
                            angle_y.sin() * angle_x.cos(),
                            -angle_x.sin(),
                            angle_y.cos() * angle_x.cos(),
                        ];

                        // Calculate depth and sort back-to-front
                        let mut sorted: Vec<(usize, f32)> = data
                            .splats
                            .iter()
                            .enumerate()
                            .map(|(i, s)| {
                                let d = s.position[0] * cam_fwd[0]
                                    + s.position[1] * cam_fwd[1]
                                    + s.position[2] * cam_fwd[2];
                                (i, d)
                            })
                            .collect();
                        sorted.sort_by(|a, b| {
                            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        // Subsample for performance
                        let max_splats = 300_000;
                        let step = if sorted.len() > max_splats {
                            sorted.len() / max_splats
                        } else {
                            1
                        };

                        // Show splat count in stats area
                        painter.text(
                            egui::pos2(rect.min.x + 8.0, rect.min.y + 12.0),
                            egui::Align2::LEFT_TOP,
                            format!(
                                "Gaussian Splats: {} (showing: {})",
                                data.splats.len(),
                                data.splats.len() / step
                            ),
                            egui::FontId::proportional(10.0),
                            egui::Color32::from_rgb(180, 180, 80),
                        );

                        // Calculate scale normalization: auto-adjust so splats are visible
                        let avg_max_scale: f32 = if data.splats.len() > 100 {
                            let sample: f32 = data
                                .splats
                                .iter()
                                .take(1000)
                                .map(|s| s.scale[0].max(s.scale[1]).max(s.scale[2]))
                                .sum();
                            sample / data.splats.len().min(1000) as f32
                        } else {
                            data.splats
                                .iter()
                                .map(|s| s.scale[0].max(s.scale[1]).max(s.scale[2]))
                                .sum::<f32>()
                                / data.splats.len().max(1) as f32
                        };
                        // Target: average splat should be ~2px on screen
                        let scale_multiplier = if avg_max_scale > 0.0001 {
                            2.0 / (avg_max_scale * scale)
                        } else {
                            1.0
                        };

                        for (draw_idx, &(idx, _depth)) in sorted.iter().enumerate() {
                            if draw_idx % step != 0 {
                                continue;
                            }
                            let splat = &data.splats[idx];

                            let p = project(&splat.position);
                            if !rect.contains(p) {
                                continue;
                            }

                            // Alpha from opacity
                            let alpha = (splat.opacity * 255.0).min(255.0) as u8;
                            if alpha < 3 {
                                continue;
                            }

                            // Boost dark colors: lift shadows so structure is visible
                            let boost = |c: u8| -> u8 {
                                let f = c as f32 / 255.0;
                                // Gamma correction + brightness boost
                                let boosted = (f.powf(0.6) * 1.3).min(1.0);
                                (boosted * 255.0) as u8
                            };

                            let color = egui::Color32::from_rgba_unmultiplied(
                                boost(splat.color[0]),
                                boost(splat.color[1]),
                                boost(splat.color[2]),
                                alpha,
                            );

                            // Point size auto-scaled for visibility
                            let max_s = splat.scale[0].max(splat.scale[1]).max(splat.scale[2]);
                            let point_size = (max_s * scale * scale_multiplier).clamp(0.8, 10.0);

                            painter.circle_filled(p, point_size, color);
                        }

                        // Splat count label
                        painter.text(
                            egui::pos2(rect.min.x + 8.0, rect.max.y - 16.0),
                            egui::Align2::LEFT_BOTTOM,
                            format!(
                                "{} splats (showing {})",
                                data.splats.len(),
                                data.splats.len() / step
                            ),
                            egui::FontId::proportional(11.0),
                            egui::Color32::from_rgb(150, 150, 150),
                        );
                    }

                    // Draw edges (limit for performance)
                    let edge_color = egui::Color32::from_rgb(100, 180, 255);
                    let max_edges = 50000;
                    let step = if data.edges.len() > max_edges {
                        data.edges.len() / max_edges
                    } else {
                        1
                    };

                    for (idx, &(i0, i1)) in data.edges.iter().enumerate() {
                        if idx % step != 0 {
                            continue;
                        }
                        if i0 < data.vertices.len() && i1 < data.vertices.len() {
                            let p0 = project(&data.vertices[i0]);
                            let p1 = project(&data.vertices[i1]);

                            // Clip to rect
                            if rect.contains(p0) || rect.contains(p1) {
                                painter.line_segment([p0, p1], egui::Stroke::new(0.5, edge_color));
                            }
                        }
                    }

                    // If no edges and no splats (plain point cloud), render individual points
                    if data.edges.is_empty() && data.splats.is_empty() && !data.vertices.is_empty()
                    {
                        let point_color = egui::Color32::from_rgb(100, 200, 255);
                        let max_points = 100000;
                        let step = if data.vertices.len() > max_points {
                            data.vertices.len() / max_points
                        } else {
                            1
                        };

                        for (idx, vert) in data.vertices.iter().enumerate() {
                            if idx % step != 0 {
                                continue;
                            }
                            let p = project(vert);
                            if rect.contains(p) {
                                painter.circle_filled(p, 1.0, point_color);
                            }
                        }
                    }

                    // Axes indicator (bottom-left corner)
                    let axes_origin = egui::pos2(rect.min.x + 40.0, rect.max.y - 40.0);
                    let axis_len = 25.0;
                    // X axis (red)
                    let x_end = egui::pos2(axes_origin.x + axis_len * cos_y, axes_origin.y);
                    painter.line_segment(
                        [axes_origin, x_end],
                        egui::Stroke::new(2.0, egui::Color32::RED),
                    );
                    painter.text(
                        x_end,
                        egui::Align2::LEFT_CENTER,
                        "X",
                        egui::FontId::proportional(10.0),
                        egui::Color32::RED,
                    );
                    // Y axis (green)
                    let y_end = egui::pos2(axes_origin.x, axes_origin.y - axis_len);
                    painter.line_segment(
                        [axes_origin, y_end],
                        egui::Stroke::new(2.0, egui::Color32::GREEN),
                    );
                    painter.text(
                        y_end,
                        egui::Align2::CENTER_BOTTOM,
                        "Y",
                        egui::FontId::proportional(10.0),
                        egui::Color32::GREEN,
                    );
                    // Z axis (blue)
                    let z_end = egui::pos2(
                        axes_origin.x + axis_len * sin_y,
                        axes_origin.y + axis_len * sin_x,
                    );
                    painter.line_segment(
                        [axes_origin, z_end],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 255)),
                    );
                    painter.text(
                        z_end,
                        egui::Align2::LEFT_TOP,
                        "Z",
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_rgb(80, 80, 255),
                    );
                }
            }
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("Cannot load 3D model");
                    ui.label(&tab.file_path);
                    ui.label("Supported formats: GLTF, GLB, OBJ, STL, PLY");
                });
            }
        }
    }
}
