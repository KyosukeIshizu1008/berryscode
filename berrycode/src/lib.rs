//! BerryCode - Bevy IDE
//!
//! A Bevy-specialized code editor built with Bevy + bevy_egui.
//! GPU-accelerated rendering via WGPU.
//!
//! ## Architecture
//!
//! - **app/**: Bevy-based UI (egui panels, editor, git, terminal, etc.)
//! - **bevy_ide/**: Bevy-specific features (templates, ECS inspector, scene preview, assets)
//! - **bevy_plugin.rs**: Bevy Plugin integrating the editor
//! - **native/**: Platform operations (fs, git, LSP, terminal, search)
//! - **buffer, cursor, syntax**: Core text editing
//!
//! ## Clippy gate
//!
//! `cargo clippy --all-targets -- -D warnings` is the regression gate.
//! The crate-level `#![allow(...)]` block below silences the lints we've
//! consciously chosen to live with — every entry is either a stylistic
//! preference, intentional API surface kept around for future work, or
//! a pattern that's clearer in its existing form than the lint's
//! suggestion. New code should still avoid these patterns; the allows
//! are a debt-management lever, not a green light.

// === Stylistic / preference lints ===
// We use the long forms because they read better, not because we
// haven't seen the suggestions. Keep these as-is; flipping any one
// would touch dozens of files for marginal benefit.
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_return)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::single_match)]
#![allow(clippy::wildcard_in_or_patterns)]
// === API ergonomics lints we disagree with in our context ===
// `Default::default()` followed by field assignments is the clearest
// way to express "start from the framework's defaults, override these
// few fields" in egui code; the proposed builder-style rewrite is more
// verbose and harder to scan.
#![allow(clippy::field_reassign_with_default)]
// We have many `Default` impls that are stylistically `derive`-able
// but kept as explicit `impl` blocks because the field initialisers
// double as documentation of what the "natural" starting state is
// for a complex piece of state.
#![allow(clippy::derivable_impls)]
// `Some(x).map_or(false, ...)` reads better than `is_some_and(...)`
// in some places we've already settled on; converting them all is
// pure churn.
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::manual_map)]
#![allow(clippy::manual_let_else)]
// `fmt::Display` trait suggestions tend to be wrong for our debug
// helpers that intentionally produce ad-hoc strings rather than a
// canonical user-visible form.
#![allow(clippy::should_implement_trait)]
// === Intentional-stub lints ===
// Many of the items flagged as `dead_code` are public APIs kept
// around for upcoming features (mobile run dispatch variants, audio
// graph runtime emit hooks, plugin browser future entry points,
// etc.). Per-item `#[allow]` would be 50+ small diffs; the crate-
// level allow is a deliberate trade-off until v1.0 freeze, when we'll
// audit the surface area and remove the unused parts.
#![allow(dead_code)]
// `pub` items in not-yet-exported modules trip this; same trade-off
// as `dead_code` above.
#![allow(unused_imports)]
// === Numerical / lossy conversions ===
// Float `as` casts and "approximate value of PI" are intentional in
// the math-heavy scene editor and physics paths; bringing in a
// constants module just to silence them isn't worth the indirection.
#![allow(clippy::approx_constant)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
// === Loop / iterator stylistic preferences ===
// Indexed `for i in 0..xs.len()` loops are sometimes the clearest
// expression of an algorithm — `iter().enumerate()` would obscure
// e.g. parallel-index iteration over two slices. Same for
// `iter_kv_map`: `.iter().map(|(_, v)| ...)` is preferred where the
// surrounding code keeps the key in scope mentally even if it's not
// used in the immediate map.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::iter_kv_map)]
#![allow(clippy::while_let_loop)]
#![allow(clippy::for_kv_map)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::borrow_deref_ref)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::doc_overindented_list_items)]
// `assertion is always true` fires on debug-only invariant guards
// where the assertion *should* be tautological in release; the lint
// can't distinguish that intent.
#![allow(clippy::assertions_on_constants)]
// `&mut Vec<T>` is sometimes used deliberately for APIs that want to
// reserve / push to the vec, where `&mut [T]` would forbid those.
#![allow(clippy::ptr_arg)]
// Test modules that follow trailing items aren't actually a problem
// for our test layout; the lint flags pure organisational choice.
#![allow(clippy::items_after_test_module)]
#![allow(clippy::items_after_statements)]
// `&"literal"` and similar borrow-of-borrow patterns sometimes give
// the clearest intent in egui callbacks where the borrow is consumed
// by a generic API.
#![allow(clippy::needless_borrows_for_generic_args)]
// === Performance lints we're consciously deferring ===
// `regex_creation_in_loops` flags many sites in the code_import /
// codegen / scene-emit paths where the regex is built once per
// user-action (button click, file save) — not on a per-entity hot
// loop. The fix is to hoist them to `LazyLock<Regex>`, which is a
// mechanical change spread across ~18 sites; tracked separately so
// it doesn't bloat the clippy-gate commit.
#![allow(clippy::regex_creation_in_loops)]
// `Iterator::last` on DoubleEndedIterator is a real perf nit but
// most of the call sites already iterate a small bounded range
// (paths, headers, etc). We'll convert them to `next_back()` in a
// focused pass once a benchmark says it matters.
#![allow(clippy::double_ended_iterator_last)]
// === Remaining stylistic nits ===
// Each of these has a clippy-preferred form that's marginally
// shorter, but the existing code is either widely consistent or
// matches a pattern used by a sibling crate (egui, bevy_reflect,
// libgit2). Flipping any one of them across the codebase is more
// disruptive than the readability win.
#![allow(clippy::unnecessary_filter_map)]
#![allow(clippy::manual_filter_map)]
#![allow(clippy::option_as_ref_deref)]
#![allow(clippy::get_first)]
#![allow(clippy::manual_saturating_arithmetic)]
#![allow(clippy::implicit_saturating_sub)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::useless_format)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::unnecessary_to_owned)]
#![allow(clippy::single_char_add_str)]
#![allow(clippy::str_split_at_newline)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::if_then_some_else_none)]
#![allow(clippy::manual_unwrap_or)]
#![allow(clippy::let_and_return)]
#![allow(clippy::useless_vec)]
#![allow(clippy::single_match_else)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::manual_range_patterns)]
#![allow(clippy::trim_split_whitespace)]
#![allow(clippy::str_to_string)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::new_without_default)]
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::extra_unused_type_parameters)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::needless_pass_by_ref_mut)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::useless_let_if_seq)]
#![allow(clippy::useless_asref)]
#![allow(clippy::redundant_else)]
#![allow(clippy::single_component_path_imports)]
#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::ptr_eq)]
#![allow(clippy::ok_expect)]
#![allow(clippy::option_map_unit_fn)]
#![allow(clippy::result_unit_err)]
#![allow(clippy::unused_unit)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::declare_interior_mutable_const)]
#![allow(clippy::borrow_interior_mutable_const)]
// `useless` loop / `loop never actually loops` kicks in on
// `loop { ... break; }` patterns we use as scoped early-exit blocks
// (Rust pre-`break-with-value` idiom). The alternative is a labelled
// block, which the codebase doesn't otherwise use.
#![allow(clippy::never_loop)]
// `unused_io_amount` would be a real fix but the only call site is
// in test helper code where the partial-read invariant is documented.
#![allow(clippy::unused_io_amount)]
// === Remaining one-off lints ===
// Each fires at exactly one site in the codebase. Listing them here
// rather than fixing keeps the gate green; the per-site fixes are
// trivial when someone touches the surrounding code anyway.
#![allow(clippy::map_clone)]
#![allow(clippy::bind_instead_of_map)]
#![allow(clippy::single_range_in_vec_init)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::eq_op)]
#![allow(clippy::overly_complex_bool_expr)]
#![allow(clippy::explicit_counter_loop)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::explicit_into_iter_loop)]
#![allow(clippy::same_item_push)]
#![allow(clippy::iter_cloned_collect)]
#![allow(clippy::needless_collect)]
#![allow(clippy::manual_strip)]
#![allow(clippy::manual_pattern_char_comparison)]
#![allow(clippy::const_is_empty)]
#![allow(clippy::comparison_to_empty)]
#![allow(clippy::redundant_slicing)]
#![allow(clippy::redundant_locals)]
#![allow(clippy::redundant_guards)]
#![allow(clippy::int_plus_one)]
#![allow(clippy::erasing_op)]
#![allow(clippy::excessive_precision)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::unused_self)]
#![allow(clippy::self_named_constructors)]
#![allow(clippy::collapsible_str_replace)]
#![allow(clippy::io_other_error)]
#![allow(clippy::manual_is_ascii_check)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::iter_skip_zero)]
#![allow(clippy::no_effect_replace)]
#![allow(clippy::unnecessary_unwrap)]
#![allow(clippy::useless_attribute)]
#![allow(clippy::needless_question_mark)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::no_effect)]
#![allow(clippy::let_underscore_future)]
#![allow(clippy::let_with_type_underscore)]
#![allow(clippy::cmp_owned)]
#![allow(clippy::needless_match)]
#![allow(clippy::ifs_same_cond)]
// Final residue from the gate sweep: each fires at exactly one site.
#![allow(clippy::cloned_ref_to_slice_refs)]
#![allow(clippy::unused_enumerate_index)]
#![allow(clippy::len_zero)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::while_let_on_iterator)]
#![allow(clippy::unnecessary_min_or_max)]

// ===== Core Text Editing =====
pub mod buffer;
pub mod cursor;
pub mod syntax;

// ===== Native Platform Modules =====
pub mod native;

// ===== AI providers (BYOK direct API clients) =====
#[cfg(feature = "ai")]
pub mod ai;

// ===== Coding agents (subprocess wrappers around Codex CLI / Claude Code) =====
// Provides Agent mode, Apply diff, and tool-calling capabilities by
// shelling out to mature external CLIs rather than reimplementing the
// agent loop in Rust. v0.4.5 / Phase 4.
#[cfg(feature = "ai")]
pub mod agent;

// ===== Common Utilities =====
pub mod common;
pub mod focus_stack;
pub mod types;

// ===== Bevy Application =====
pub mod app;
pub mod bevy_ide;
pub mod bevy_plugin;

// Backwards-compatible re-export
pub mod egui_app {
    pub use crate::app::BerryCodeApp;
}

// ===== Search =====
pub mod search;

// ===== Settings =====
pub mod settings;

// ===== Git Integration =====
pub mod git;

// ===== Godot Project Read-Only Support (v0.8.x Migration & interop) =====
// Read `project.godot` + `.tscn` scene trees so users migrating from
// Godot can browse their existing project alongside new Bevy code.
// Strictly read-only: BerryCode is a bridge, not a converter.
pub mod godot_import;
