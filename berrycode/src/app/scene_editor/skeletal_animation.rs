//! GLB skeletal animation playback for the Scene Editor.
//!
//! Distinct from `super::animation`, which animates editor-authored transform
//! keyframes. This module drives the `AnimationPlayer` Bevy auto-inserts on
//! GLB scene roots, so clips baked into a `.glb` (Run, Walk, Idle, …) actually
//! play in the Scene View.
//!
//! Pipeline:
//! 1. `bevy_sync` inserts a [`SceneAnimRoot`] on every Bevy entity that hosts
//!    a GLB scene, carrying the editor id and the `Gltf` handle.
//! 2. [`setup_scene_animation_graphs`] builds a single `AnimationGraph` of all
//!    named clips once the GLTF asset is loaded.
//! 3. [`attach_scene_animation_players`] catches the `Added<AnimationPlayer>`
//!    Bevy emits on the scene root and attaches our graph + transitions,
//!    starting playback on the requested (or first) clip on a loop.
//! 4. [`apply_scene_clip_changes`] cross-fades to a new clip whenever the
//!    inspector writes a different name into `requested_clip`.

use bevy::animation::graph::{AnimationGraph, AnimationGraphHandle, AnimationNodeIndex};
use bevy::animation::{AnimationClip, AnimationPlayer};
use bevy::gltf::Gltf;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component, Clone)]
pub struct SceneAnimRoot {
    pub scene_entity_id: u64,
    pub gltf: Handle<Gltf>,
    /// Clip name that should auto-play once the graph finishes loading.
    /// `None` falls back to the alphabetically-first clip.
    pub initial_clip: Option<String>,
}

#[derive(Default)]
pub struct SceneAnimEntry {
    pub graph: Option<Handle<AnimationGraph>>,
    /// Available clips, sorted alphabetically for deterministic UI order.
    pub clips: Vec<(String, AnimationNodeIndex)>,
    /// Raw clip handles keyed by name. `apply_scene_clip_changes` rebuilds a
    /// fresh single-clip graph from these on every switch — calling
    /// `player.play()` on a multi-clip graph was not actually replacing the
    /// active animation in Bevy 0.18 for our setup.
    pub clip_handles: Vec<(String, Handle<AnimationClip>)>,
    pub current_clip: Option<String>,
    pub requested_clip: Option<String>,
    pub player_entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct SceneAnimationState {
    pub entries: HashMap<u64, SceneAnimEntry>,
}

pub fn setup_scene_animation_graphs(
    roots: Query<&SceneAnimRoot>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut state: ResMut<SceneAnimationState>,
) {
    for root in roots.iter() {
        let entry = state.entries.entry(root.scene_entity_id).or_default();
        if entry.graph.is_some() {
            continue;
        }
        let Some(gltf) = gltfs.get(&root.gltf) else {
            continue;
        };
        if gltf.named_animations.is_empty() {
            continue;
        }

        let mut named: Vec<(String, Handle<AnimationClip>)> = gltf
            .named_animations
            .iter()
            .map(|(n, h)| (n.to_string(), h.clone()))
            .collect();
        named.sort_by(|a, b| a.0.cmp(&b.0));

        let clips: Vec<Handle<AnimationClip>> = named.iter().map(|(_, h)| h.clone()).collect();
        let (graph, indices) = AnimationGraph::from_clips(clips);
        let graph_handle = graphs.add(graph);

        entry.clip_handles = named.iter().map(|(n, h)| (n.clone(), h.clone())).collect();
        entry.clips = named
            .into_iter()
            .zip(indices)
            .map(|((n, _), i)| (n, i))
            .collect();
        if entry.requested_clip.is_none() {
            if let Some(clip) = root.initial_clip.clone() {
                entry.requested_clip = Some(clip);
            } else if let Some((first, _)) = entry.clips.first() {
                entry.requested_clip = Some(first.clone());
            }
        }
        tracing::info!(
            "Scene anim: built graph for entity {} with {} clips: {:?}",
            root.scene_entity_id,
            entry.clips.len(),
            entry
                .clips
                .iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
        );
        entry.graph = Some(graph_handle);
    }
}

pub fn attach_scene_animation_players(
    mut commands: Commands,
    mut state: ResMut<SceneAnimationState>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<AnimationGraphHandle>>,
    roots: Query<&SceneAnimRoot>,
    ancestors: Query<&ChildOf>,
) {
    for (entity, mut player) in &mut players {
        let mut current = entity;
        let mut found_id: Option<u64> = None;
        for _ in 0..50 {
            if let Ok(root) = roots.get(current) {
                found_id = Some(root.scene_entity_id);
                break;
            }
            match ancestors.get(current) {
                Ok(p) => current = p.parent(),
                Err(_) => break,
            }
        }
        let Some(id) = found_id else {
            continue;
        };
        let Some(entry) = state.entries.get_mut(&id) else {
            continue;
        };
        let Some(graph_handle) = entry.graph.clone() else {
            continue;
        };
        if entry.clips.is_empty() {
            continue;
        }
        let target = entry
            .requested_clip
            .clone()
            .or_else(|| entry.clips.first().map(|(n, _)| n.clone()));
        let Some(target) = target else {
            continue;
        };
        let Some(node) = entry
            .clips
            .iter()
            .find(|(n, _)| n == &target)
            .map(|(_, i)| *i)
        else {
            continue;
        };

        // Hard-set the player to the requested clip. Bypasses
        // `AnimationTransitions` because the editor doesn't need crossfading
        // and that layer was making subsequent `apply_scene_clip_changes`
        // updates a no-op.
        player.stop_all();
        let active = player.play(node);
        active.repeat();
        active.set_seek_time(0.0);
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(graph_handle));
        entry.player_entity = Some(entity);
        entry.current_clip = Some(target.clone());
        tracing::info!(
            "Scene anim: attached player on bevy entity {:?} for scene entity {} (clip={})",
            entity,
            id,
            target
        );
    }
}

pub fn apply_scene_clip_changes(
    mut commands: Commands,
    mut state: ResMut<SceneAnimationState>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for entry in state.entries.values_mut() {
        let Some(entity) = entry.player_entity else {
            continue;
        };
        let Some(requested) = entry.requested_clip.clone() else {
            continue;
        };
        if entry.current_clip.as_ref() == Some(&requested) {
            continue;
        }
        let Some(clip_handle) = entry
            .clip_handles
            .iter()
            .find(|(n, _)| n == &requested)
            .map(|(_, h)| h.clone())
        else {
            continue;
        };
        let Ok(mut player) = players.get_mut(entity) else {
            continue;
        };
        // Build a brand-new single-clip graph and swap it in. Calling
        // `player.play()` on the original 3-clip graph wasn't actually
        // changing the active animation in Bevy 0.18 — the player kept
        // running the originally-attached clip. Replacing the
        // `AnimationGraphHandle` forces the player to re-bind to the new
        // graph whose only node IS the requested clip.
        let (new_graph, new_root) = AnimationGraph::from_clip(clip_handle);
        let new_graph_handle = graphs.add(new_graph);
        player.stop_all();
        let active = player.play(new_root);
        active.repeat();
        active.set_seek_time(0.0);
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(new_graph_handle.clone()));
        entry.graph = Some(new_graph_handle);
        // Track the new root in `clips` so a subsequent identical request
        // would still short-circuit cleanly via `current_clip`.
        entry.clips = vec![(requested.clone(), new_root)];
        tracing::info!(
            "Scene anim: switching to clip '{}' on bevy entity {:?}",
            requested,
            entity
        );
        entry.current_clip = Some(requested);
    }
}
