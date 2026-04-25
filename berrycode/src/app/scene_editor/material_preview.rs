//! Material preview for the Inspector panel.
//!
//! Bevy render-to-texture pipeline that renders a real PBR
//! sphere on `RenderLayers::layer(3)`. The original CPU approximation is
//! kept as a fallback (used when the GPU preview texture is not yet ready).

use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::image::{TextureFormatPixelInfo as _, ImageSampler}; use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::camera::visibility::RenderLayers;

// ---------------------------------------------------------------------------
// Bevy Resource: GPU material preview state
// ---------------------------------------------------------------------------

/// Bevy resource that holds the render-to-texture pipeline for the material
/// preview sphere shown in the inspector.
#[derive(Resource)]
pub struct MaterialPreviewRender {
    /// Handle to the off-screen image used as render target.
    pub render_target: Option<Handle<Image>>,
    /// Cached egui texture id (re-registered each frame in `berry_ui_system`).
    pub egui_texture_id: Option<egui::TextureId>,
    /// The Bevy entity of the preview sphere mesh.
    pub sphere_entity: Option<Entity>,
    /// Current base color to apply to the preview material.
    pub current_color: [f32; 3],
    /// Current metallic value.
    pub current_metallic: f32,
    /// Current roughness value.
    pub current_roughness: f32,
    /// Current emissive color.
    pub current_emissive: [f32; 3],
    /// When true, the material on the sphere needs updating.
    pub dirty: bool,
}

impl Default for MaterialPreviewRender {
    fn default() -> Self {
        Self {
            render_target: None,
            egui_texture_id: None,
            sphere_entity: None,
            current_color: [0.8, 0.8, 0.8],
            current_metallic: 0.0,
            current_roughness: 0.5,
            current_emissive: [0.0, 0.0, 0.0],
            dirty: false,
        }
    }
}

/// Marker component on the material preview sphere.
#[derive(Component)]
pub struct MaterialPreviewSphere;

/// Marker component on the material preview camera.
#[derive(Component)]
pub struct MaterialPreviewCamera;

/// Marker component on the material preview light.
#[derive(Component)]
pub struct MaterialPreviewLight;

// ---------------------------------------------------------------------------
// Bevy systems
// ---------------------------------------------------------------------------

/// Startup system: create a 128x128 render target, a camera, a directional
/// light, and a unit sphere -- all on `RenderLayers::layer(3)`.
pub fn setup_material_preview(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut preview: ResMut<MaterialPreviewRender>,
) {
    let size = Extent3d {
        width: 128,
        height: 128,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[30, 32, 36, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image_handle = images.add(image);

    // Camera looking at origin from a slight angle.
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: bevy::camera::RenderTarget::Image(image_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(Color::srgba(0.12, 0.13, 0.14, 1.0)),
            order: -3,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(3),
        MaterialPreviewCamera,
    ));

    // Key light.
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
        RenderLayers::layer(3),
        MaterialPreviewLight,
    ));

    // PBR sphere.
    let mesh = meshes.add(Sphere::new(0.8).mesh().uv(32, 16));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.8, 0.8),
        metallic: 0.0,
        perceptual_roughness: 0.5,
        ..default()
    });
    let sphere = commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::IDENTITY,
            RenderLayers::layer(3),
            MaterialPreviewSphere,
        ))
        .id();

    preview.render_target = Some(image_handle);
    preview.sphere_entity = Some(sphere);
}

/// Update system: when the inspector pushes new PBR values (dirty flag),
/// apply them to the sphere's `StandardMaterial`.
pub fn update_material_preview(
    mut preview: ResMut<MaterialPreviewRender>,
    sphere_q: Query<&MeshMaterial3d<StandardMaterial>, With<MaterialPreviewSphere>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !preview.dirty {
        return;
    }
    preview.dirty = false;

    if let Ok(mat_handle) = sphere_q.single() {
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgb(
                preview.current_color[0],
                preview.current_color[1],
                preview.current_color[2],
            );
            mat.metallic = preview.current_metallic;
            mat.perceptual_roughness = preview.current_roughness;
            mat.emissive = LinearRgba::rgb(
                preview.current_emissive[0],
                preview.current_emissive[1],
                preview.current_emissive[2],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// GPU preview helper for the Inspector UI
// ---------------------------------------------------------------------------

/// Draw the GPU-rendered material preview if the texture is available,
/// otherwise fall back to the CPU approximation.
pub fn draw_material_preview_gpu_or_fallback(
    ui: &mut egui::Ui,
    gpu_texture_id: Option<egui::TextureId>,
    base_color: [f32; 3],
    metallic: f32,
    roughness: f32,
    emissive: [f32; 3],
) {
    if let Some(tex_id) = gpu_texture_id {
        ui.image(egui::load::SizedTexture::new(
            tex_id,
            egui::vec2(64.0, 64.0),
        ));
    } else {
        // Fallback to the CPU approximation.
        draw_material_preview(ui, base_color, metallic, roughness, emissive);
    }
}

// ---------------------------------------------------------------------------
// CPU fallback (original implementation, kept for robustness)
// ---------------------------------------------------------------------------

/// Draw a material preview circle in the given UI area.
/// This is a CPU approximation, not a GPU render.
pub fn draw_material_preview(
    ui: &mut egui::Ui,
    base_color: [f32; 3],
    metallic: f32,
    roughness: f32,
    emissive: [f32; 3],
) {
    let size = 64.0;
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());

    let painter = ui.painter();
    let center = rect.center();
    let radius = size * 0.45;

    // Dark background behind the sphere.
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 22, 26));

    // Draw concentric rings to simulate a lit sphere.
    let steps: u32 = 16;
    for i in (0..=steps).rev() {
        let t = i as f32 / steps as f32; // 0 = center, 1 = edge
        let r = radius * t;

        // Simple Lambert + Phong shading approximation.
        let normal_z = (1.0 - t * t).max(0.0).sqrt(); // hemisphere normal Z
        let light_dir: [f32; 3] = [0.5, 0.7, 0.5];
        let light_len = (light_dir[0] * light_dir[0]
            + light_dir[1] * light_dir[1]
            + light_dir[2] * light_dir[2])
            .sqrt();
        let ndotl = (normal_z * light_dir[2] / light_len).max(0.0);

        // Diffuse contribution.
        let diffuse = ndotl * (1.0 - metallic);

        // Specular (Blinn-Phong approximation).
        let specular_power = (1.0 - roughness) * 64.0 + 2.0;
        let half_z = (light_dir[2] / light_len + 1.0) * 0.5; // simplified half vector
        let spec = half_z.powf(specular_power) * (1.0 - roughness) * 0.5;
        let spec_color = if metallic > 0.5 {
            base_color
        } else {
            [1.0, 1.0, 1.0]
        };

        let cr = ((base_color[0] * diffuse + spec_color[0] * spec + emissive[0]) * 255.0).min(255.0)
            as u8;
        let cg = ((base_color[1] * diffuse + spec_color[1] * spec + emissive[1]) * 255.0).min(255.0)
            as u8;
        let cb = ((base_color[2] * diffuse + spec_color[2] * spec + emissive[2]) * 255.0).min(255.0)
            as u8;

        painter.circle_filled(center, r, egui::Color32::from_rgb(cr, cg, cb));
    }

    // Edge highlight.
    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
    );
}
