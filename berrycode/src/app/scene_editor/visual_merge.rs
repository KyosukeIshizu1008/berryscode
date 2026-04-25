//! Visual merge conflict resolver with side-by-side diff.

use crate::app::BerryCodeApp;

pub struct VisualMergeState {
    pub open: bool,
    pub conflicts: Vec<MergeConflict>,
    pub selected_conflict: Option<usize>,
}

pub struct MergeConflict {
    pub file_path: String,
    pub ours: String,
    pub theirs: String,
    pub resolved: Option<String>,
}

impl Default for VisualMergeState {
    fn default() -> Self {
        Self {
            open: false,
            conflicts: Vec::new(),
            selected_conflict: None,
        }
    }
}

impl VisualMergeState {
    pub fn add_conflict(&mut self, file_path: String, ours: String, theirs: String) {
        self.conflicts.push(MergeConflict {
            file_path,
            ours,
            theirs,
            resolved: None,
        });
    }

    pub fn resolve_ours(&mut self, idx: usize) {
        if let Some(c) = self.conflicts.get_mut(idx) {
            c.resolved = Some(c.ours.clone());
        }
    }

    pub fn resolve_theirs(&mut self, idx: usize) {
        if let Some(c) = self.conflicts.get_mut(idx) {
            c.resolved = Some(c.theirs.clone());
        }
    }

    pub fn resolve_manual(&mut self, idx: usize, text: String) {
        if let Some(c) = self.conflicts.get_mut(idx) {
            c.resolved = Some(text);
        }
    }

    pub fn unresolved_count(&self) -> usize {
        self.conflicts
            .iter()
            .filter(|c| c.resolved.is_none())
            .count()
    }

    pub fn all_resolved(&self) -> bool {
        !self.conflicts.is_empty() && self.conflicts.iter().all(|c| c.resolved.is_some())
    }
}

impl BerryCodeApp {
    pub(crate) fn render_visual_merge(&mut self, ctx: &egui::Context) {
        if !self.visual_merge.open {
            return;
        }
        let mut open = self.visual_merge.open;
        egui::Window::new("Visual Merge")
            .open(&mut open)
            .default_size([700.0, 450.0])
            .show(ctx, |ui| {
                let unresolved = self.visual_merge.unresolved_count();
                let total = self.visual_merge.conflicts.len();
                ui.label(format!("{} conflicts ({} unresolved)", total, unresolved));
                if self.visual_merge.all_resolved() {
                    ui.colored_label(egui::Color32::from_rgb(80, 200, 80), "All resolved!");
                }
                ui.separator();
                // Conflict list
                let mut select = self.visual_merge.selected_conflict;
                for (i, c) in self.visual_merge.conflicts.iter().enumerate() {
                    let resolved_marker = if c.resolved.is_some() { " [ok]" } else { "" };
                    let label = format!("{}{}", c.file_path, resolved_marker);
                    if ui.selectable_label(select == Some(i), &label).clicked() {
                        select = Some(i);
                    }
                }
                self.visual_merge.selected_conflict = select;
                ui.separator();
                // Side-by-side diff for selected conflict
                if let Some(idx) = self.visual_merge.selected_conflict {
                    if idx < self.visual_merge.conflicts.len() {
                        let ours = self.visual_merge.conflicts[idx].ours.clone();
                        let theirs = self.visual_merge.conflicts[idx].theirs.clone();
                        ui.columns(2, |cols| {
                            cols[0].heading("Ours");
                            cols[0].label(egui::RichText::new(&ours).monospace().size(11.0));
                            cols[1].heading("Theirs");
                            cols[1].label(egui::RichText::new(&theirs).monospace().size(11.0));
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Accept Ours").clicked() {
                                self.visual_merge.resolve_ours(idx);
                            }
                            if ui.button("Accept Theirs").clicked() {
                                self.visual_merge.resolve_theirs(idx);
                            }
                            if ui.button("Manual").clicked() {
                                let combined = format!("{}\n---\n{}", ours, theirs);
                                self.visual_merge.resolve_manual(idx, combined);
                            }
                        });
                    }
                }
            });
        self.visual_merge.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_resolve_ours() {
        let mut s = VisualMergeState::default();
        s.add_conflict("file.rs".into(), "our code".into(), "their code".into());
        assert_eq!(s.unresolved_count(), 1);
        s.resolve_ours(0);
        assert_eq!(s.conflicts[0].resolved.as_deref(), Some("our code"));
        assert!(s.all_resolved());
    }

    #[test]
    fn resolve_theirs() {
        let mut s = VisualMergeState::default();
        s.add_conflict("a.rs".into(), "A".into(), "B".into());
        s.resolve_theirs(0);
        assert_eq!(s.conflicts[0].resolved.as_deref(), Some("B"));
    }

    #[test]
    fn resolve_manual() {
        let mut s = VisualMergeState::default();
        s.add_conflict("a.rs".into(), "A".into(), "B".into());
        s.resolve_manual(0, "custom".into());
        assert_eq!(s.conflicts[0].resolved.as_deref(), Some("custom"));
    }

    #[test]
    fn all_resolved_empty_is_false() {
        let s = VisualMergeState::default();
        assert!(!s.all_resolved());
    }

    #[test]
    fn unresolved_count_mixed() {
        let mut s = VisualMergeState::default();
        s.add_conflict("a.rs".into(), "A".into(), "B".into());
        s.add_conflict("b.rs".into(), "C".into(), "D".into());
        s.resolve_ours(0);
        assert_eq!(s.unresolved_count(), 1);
        assert!(!s.all_resolved());
    }
}
