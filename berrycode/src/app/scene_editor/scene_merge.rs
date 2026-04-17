//! Three-way scene merge for .bscene files.

use super::model::*;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct MergeConflict {
    pub entity_id: u64,
    pub entity_name: String,
    pub ours: SceneEntity,
    pub theirs: SceneEntity,
}

#[derive(Debug, Clone)]
pub struct MergeResult {
    pub merged: SceneModel,
    pub conflicts: Vec<MergeConflict>,
}

/// Three-way merge: given base, ours, theirs, produce merged result.
pub fn three_way_merge(base: &SceneModel, ours: &SceneModel, theirs: &SceneModel) -> MergeResult {
    let mut merged = base.clone();
    let mut conflicts = Vec::new();

    let base_ids: HashSet<u64> = base.entities.keys().copied().collect();
    let ours_ids: HashSet<u64> = ours.entities.keys().copied().collect();
    let theirs_ids: HashSet<u64> = theirs.entities.keys().copied().collect();

    // Entities added by ours (not in base)
    for &id in ours_ids.difference(&base_ids) {
        if let Some(entity) = ours.entities.get(&id) {
            merged.entities.insert(id, entity.clone());
            if !merged.root_entities.contains(&id) && entity.parent.is_none() {
                merged.root_entities.push(id);
            }
        }
    }

    // Entities added by theirs (not in base)
    for &id in theirs_ids.difference(&base_ids) {
        if !ours_ids.contains(&id) { // avoid duplicate if both added same id
            if let Some(entity) = theirs.entities.get(&id) {
                merged.entities.insert(id, entity.clone());
                if !merged.root_entities.contains(&id) && entity.parent.is_none() {
                    merged.root_entities.push(id);
                }
            }
        }
    }

    // Entities removed by ours
    for &id in base_ids.difference(&ours_ids) {
        if theirs_ids.contains(&id) {
            // ours removed, theirs kept — check if theirs modified
            let base_entity = base.entities.get(&id);
            let theirs_entity = theirs.entities.get(&id);
            if entity_eq(base_entity, theirs_entity) {
                // theirs didn't modify, safe to remove
                merged.entities.remove(&id);
                merged.root_entities.retain(|&eid| eid != id);
            } else {
                // conflict: ours removed, theirs modified
                if let (Some(ours_e), Some(theirs_e)) = (base_entity, theirs_entity) {
                    conflicts.push(MergeConflict {
                        entity_id: id,
                        entity_name: theirs_e.name.clone(),
                        ours: ours_e.clone(), // base version (was removed)
                        theirs: theirs_e.clone(),
                    });
                }
            }
        } else {
            // both removed — no conflict
            merged.entities.remove(&id);
            merged.root_entities.retain(|&eid| eid != id);
        }
    }

    // Entities removed by theirs
    for &id in base_ids.difference(&theirs_ids) {
        if ours_ids.contains(&id) {
            let base_entity = base.entities.get(&id);
            let ours_entity = ours.entities.get(&id);
            if entity_eq(base_entity, ours_entity) {
                merged.entities.remove(&id);
                merged.root_entities.retain(|&eid| eid != id);
            }
            // else: theirs removed, ours modified — keep ours (already in merged from base)
        }
    }

    // Entities modified by both
    for &id in &base_ids {
        if !ours_ids.contains(&id) || !theirs_ids.contains(&id) { continue; }
        let base_e = base.entities.get(&id);
        let ours_e = ours.entities.get(&id);
        let theirs_e = theirs.entities.get(&id);

        let ours_changed = !entity_eq(base_e, ours_e);
        let theirs_changed = !entity_eq(base_e, theirs_e);

        if ours_changed && theirs_changed {
            // Both modified — conflict
            if let (Some(o), Some(t)) = (ours_e, theirs_e) {
                conflicts.push(MergeConflict {
                    entity_id: id,
                    entity_name: o.name.clone(),
                    ours: o.clone(),
                    theirs: t.clone(),
                });
            }
        } else if ours_changed {
            if let Some(o) = ours_e { merged.entities.insert(id, o.clone()); }
        } else if theirs_changed {
            if let Some(t) = theirs_e { merged.entities.insert(id, t.clone()); }
        }
    }

    merged.next_id = merged.entities.keys().copied().max().unwrap_or(0) + 1;
    MergeResult { merged, conflicts }
}

fn entity_eq(a: Option<&SceneEntity>, b: Option<&SceneEntity>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            // Compare by serialized RON (simple but correct)
            let a_str = ron::ser::to_string(a).unwrap_or_default();
            let b_str = ron::ser::to_string(b).unwrap_or_default();
            a_str == b_str
        }
        (None, None) => true,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Merge UI
// ---------------------------------------------------------------------------

use crate::app::BerryCodeApp;

impl BerryCodeApp {
    /// Render the Scene Merge floating panel.
    pub(crate) fn render_merge_panel(&mut self, ctx: &egui::Context) {
        if !self.merge_panel_open {
            return;
        }

        let mut open = self.merge_panel_open;
        egui::Window::new("Scene Merge")
            .open(&mut open)
            .default_size([500.0, 400.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Three-Way Merge");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Base file:");
                    ui.text_edit_singleline(&mut self.merge_base_path);
                });
                ui.horizontal(|ui| {
                    ui.label("Ours file:");
                    ui.text_edit_singleline(&mut self.merge_ours_path);
                });
                ui.horizontal(|ui| {
                    ui.label("Theirs file:");
                    ui.text_edit_singleline(&mut self.merge_theirs_path);
                });

                ui.separator();
                if ui.button("Merge").clicked() {
                    let base_res = super::serialization::load_scene_from_ron(&self.merge_base_path);
                    let ours_res = super::serialization::load_scene_from_ron(&self.merge_ours_path);
                    let theirs_res = super::serialization::load_scene_from_ron(&self.merge_theirs_path);

                    match (base_res, ours_res, theirs_res) {
                        (Ok(base), Ok(ours), Ok(theirs)) => {
                            let result = three_way_merge(&base, &ours, &theirs);
                            self.merge_result = Some(result);
                            self.status_message = "Merge completed".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                        _ => {
                            self.status_message = "Failed to load one or more scene files".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    }
                }

                // Display merge result
                if let Some(ref result) = self.merge_result.clone() {
                    ui.separator();
                    ui.label(format!(
                        "Merged: {} entities, {} conflicts",
                        result.merged.entities.len(),
                        result.conflicts.len()
                    ));

                    if result.conflicts.is_empty() {
                        ui.colored_label(egui::Color32::GREEN, "No conflicts - clean merge!");
                        if ui.button("Apply merged scene").clicked() {
                            self.scene_snapshot();
                            self.scene_model = result.merged.clone();
                            self.scene_needs_sync = true;
                            self.merge_result = None;
                            self.status_message = "Merged scene applied".to_string();
                            self.status_message_timestamp = Some(std::time::Instant::now());
                        }
                    } else {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            format!("{} conflict(s):", result.conflicts.len()),
                        );

                        let mut resolve_action: Option<(u64, bool)> = None;
                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            for conflict in &result.conflicts {
                                ui.group(|ui| {
                                    ui.label(format!(
                                        "Entity #{}: \"{}\"",
                                        conflict.entity_id, conflict.entity_name
                                    ));
                                    ui.horizontal(|ui| {
                                        if ui.button("Use Ours").clicked() {
                                            resolve_action = Some((conflict.entity_id, true));
                                        }
                                        if ui.button("Use Theirs").clicked() {
                                            resolve_action = Some((conflict.entity_id, false));
                                        }
                                    });
                                });
                            }
                        });

                        // Apply conflict resolution
                        if let Some((entity_id, use_ours)) = resolve_action {
                            if let Some(ref mut result) = self.merge_result {
                                if let Some(idx) = result.conflicts.iter().position(|c| c.entity_id == entity_id) {
                                    let conflict = result.conflicts.remove(idx);
                                    let entity = if use_ours { conflict.ours } else { conflict.theirs };
                                    result.merged.entities.insert(entity_id, entity);
                                }
                            }
                        }
                    }
                }
            });
        self.merge_panel_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_no_conflicts() {
        let base = SceneModel::new();
        let result = three_way_merge(&base, &base, &base);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn ours_adds_entity() {
        let base = SceneModel::new();
        let mut ours = base.clone();
        ours.add_entity("New".into(), vec![]);
        let result = three_way_merge(&base, &ours, &base);
        assert_eq!(result.merged.entities.len(), 1);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn both_modify_same_entity_creates_conflict() {
        let mut base = SceneModel::new();
        let id = base.add_entity("Shared".into(), vec![]);

        let mut ours = base.clone();
        if let Some(e) = ours.entities.get_mut(&id) { e.name = "OursName".into(); }

        let mut theirs = base.clone();
        if let Some(e) = theirs.entities.get_mut(&id) { e.name = "TheirsName".into(); }

        let result = three_way_merge(&base, &ours, &theirs);
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].entity_id, id);
    }

    #[test]
    fn non_conflicting_changes_auto_merge() {
        let mut base = SceneModel::new();
        let id_a = base.add_entity("A".into(), vec![]);
        let id_b = base.add_entity("B".into(), vec![]);

        let mut ours = base.clone();
        if let Some(e) = ours.entities.get_mut(&id_a) { e.name = "A_modified".into(); }

        let mut theirs = base.clone();
        if let Some(e) = theirs.entities.get_mut(&id_b) { e.name = "B_modified".into(); }

        let result = three_way_merge(&base, &ours, &theirs);
        assert!(result.conflicts.is_empty());
        assert_eq!(result.merged.entities.get(&id_a).map(|e| e.name.as_str()), Some("A_modified"));
        assert_eq!(result.merged.entities.get(&id_b).map(|e| e.name.as_str()), Some("B_modified"));
    }
}
