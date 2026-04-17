//! Bevy API documentation reference lookup
//!
//! Maps well-known Bevy types to their docs.rs URLs and provides
//! a list of type names for autocomplete suggestions.

/// Mapping of Bevy type names to their documentation paths on docs.rs.
const BEVY_DOCS: &[(&str, &str)] = &[
    ("Component", "bevy/ecs/component/trait.Component.html"),
    ("Resource", "bevy/ecs/system/trait.Resource.html"),
    ("Plugin", "bevy/app/trait.Plugin.html"),
    ("Query", "bevy/ecs/system/struct.Query.html"),
    ("Commands", "bevy/ecs/system/struct.Commands.html"),
    ("Res", "bevy/ecs/system/struct.Res.html"),
    ("ResMut", "bevy/ecs/system/struct.ResMut.html"),
    ("Entity", "bevy/ecs/entity/struct.Entity.html"),
    ("Bundle", "bevy/ecs/bundle/trait.Bundle.html"),
    ("Event", "bevy/ecs/event/trait.Event.html"),
    ("EventReader", "bevy/ecs/event/struct.EventReader.html"),
    ("EventWriter", "bevy/ecs/event/struct.EventWriter.html"),
    ("App", "bevy/app/struct.App.html"),
    (
        "Transform",
        "bevy/transform/components/struct.Transform.html",
    ),
    (
        "GlobalTransform",
        "bevy/transform/components/struct.GlobalTransform.html",
    ),
    ("Vec3", "bevy/math/struct.Vec3.html"),
    ("Vec2", "bevy/math/struct.Vec2.html"),
    ("Quat", "bevy/math/struct.Quat.html"),
    ("Color", "bevy/render/color/enum.Color.html"),
    ("Camera", "bevy/render/camera/struct.Camera.html"),
    (
        "Camera2d",
        "bevy/core_pipeline/core_2d/struct.Camera2d.html",
    ),
    (
        "Camera3d",
        "bevy/core_pipeline/core_3d/struct.Camera3d.html",
    ),
    ("Mesh", "bevy/render/mesh/struct.Mesh.html"),
    ("Handle", "bevy/asset/struct.Handle.html"),
    ("AssetServer", "bevy/asset/struct.AssetServer.html"),
    ("Assets", "bevy/asset/struct.Assets.html"),
    ("Sprite", "bevy/sprite/struct.Sprite.html"),
    ("Text", "bevy/text/struct.Text.html"),
    ("Name", "bevy/core/struct.Name.html"),
    ("Timer", "bevy/time/struct.Timer.html"),
    ("Time", "bevy/time/struct.Time.html"),
    ("Input", "bevy/input/struct.ButtonInput.html"),
    ("KeyCode", "bevy/input/keyboard/enum.KeyCode.html"),
    ("MouseButton", "bevy/input/mouse/enum.MouseButton.html"),
    ("Window", "bevy/window/struct.Window.html"),
    ("States", "bevy/ecs/schedule/trait.States.html"),
    ("NextState", "bevy/ecs/schedule/struct.NextState.html"),
    ("State", "bevy/ecs/schedule/struct.State.html"),
    ("With", "bevy/ecs/query/struct.With.html"),
    ("Without", "bevy/ecs/query/struct.Without.html"),
    ("Added", "bevy/ecs/query/struct.Added.html"),
    ("Changed", "bevy/ecs/query/struct.Changed.html"),
    (
        "DespawnRecursiveExt",
        "bevy/hierarchy/trait.DespawnRecursiveExt.html",
    ),
    ("Children", "bevy/hierarchy/struct.Children.html"),
    ("Parent", "bevy/hierarchy/struct.Parent.html"),
    ("Visibility", "bevy/render/view/enum.Visibility.html"),
    ("AudioPlayer", "bevy/audio/struct.AudioPlayer.html"),
];

/// Get the docs.rs URL for a Bevy type.
///
/// Returns `None` if the type name is not in the known list.
pub fn get_bevy_docs_url(type_name: &str) -> Option<String> {
    BEVY_DOCS
        .iter()
        .find(|(name, _)| *name == type_name)
        .map(|(_, path)| format!("https://docs.rs/bevy/latest/{}", path))
}

/// Get a list of all known Bevy type names for autocomplete suggestions.
pub fn bevy_type_names() -> Vec<&'static str> {
    BEVY_DOCS.iter().map(|(name, _)| *name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_type_returns_url() {
        let url = get_bevy_docs_url("Query").unwrap();
        assert_eq!(
            url,
            "https://docs.rs/bevy/latest/bevy/ecs/system/struct.Query.html"
        );
    }

    #[test]
    fn test_unknown_type_returns_none() {
        assert!(get_bevy_docs_url("NonExistentType").is_none());
    }

    #[test]
    fn test_type_names_not_empty() {
        let names = bevy_type_names();
        assert!(!names.is_empty());
        assert!(names.contains(&"Component"));
        assert!(names.contains(&"App"));
    }
}
