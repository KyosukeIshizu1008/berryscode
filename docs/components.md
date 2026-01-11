# BerryEditor Design System

**Version**: 1.0
**Last Updated**: 2026-01-06
**Philosophy**: IntelliJ Platform Design Language

## Table of Contents

1. [Design Tokens](#design-tokens)
2. [Color System](#color-system)
3. [Typography](#typography)
4. [Spacing System](#spacing-system)
5. [Component Catalog](#component-catalog)
6. [Accessibility Guidelines](#accessibility-guidelines)
7. [Theme System](#theme-system)
8. [CSS Architecture](#css-architecture)

---

## Design Tokens

Design tokens are the visual design atoms of the design system — specifically, they are named entities that store visual design attributes. We use CSS custom properties (CSS variables) for runtime theme switching.

### Location
All design tokens are defined in `assets/themes/variables.css`.

### Token Categories

#### Typography Tokens
```css
--font-family-mono: 'JetBrains Mono', Menlo, Monaco, 'Courier New', monospace;
--font-size-xs: 11px;
--font-size-sm: 12px;
--font-size-base: 13px;
--font-size-md: 14px;
--font-size-lg: 16px;
--font-size-xl: 18px;

--line-height-editor: 20px;
--line-height-base: 1.5;
--line-height-compact: 1.2;
```

#### Spacing Tokens
```css
--spacing-xs: 2px;
--spacing-sm: 4px;
--spacing-md: 8px;
--spacing-lg: 12px;
--spacing-xl: 16px;
--spacing-2xl: 24px;
--spacing-3xl: 32px;
```

#### Border Radius Tokens
```css
--border-radius-none: 0;
--border-radius-sm: 2px;
--border-radius-md: 4px;
--border-radius-lg: 6px;
```

#### Animation Tokens
```css
--transition-base: 0.15s ease;
--transition-slow: 0.3s ease;
```

---

## Color System

### Darcula Theme (Default)
Based on IntelliJ IDEA Darcula theme with WCAG AA contrast compliance.

#### Background Colors
```css
--color-bg-primary: #1E1F22;      /* Main editor background */
--color-bg-secondary: #2B2D30;    /* Sidebar background */
--color-bg-tertiary: #313438;     /* Elevated surfaces */
--color-bg-elevated: #2d2d2d;     /* Popups, modals */
--color-bg-input: #3c3c3c;        /* Input fields */
--color-bg-hover: rgba(255,255,255,0.05);   /* Hover state */
--color-bg-active: rgba(255,255,255,0.1);   /* Active state */
```

#### Text Colors
```css
--color-text-primary: #FFFFFF;    /* Primary text (WCAG AAA: 16.23) */
--color-text-secondary: #d4d4d4;  /* Secondary text (WCAG AA: 12.06) */
--color-text-tertiary: #afb1b3;   /* Tertiary text (WCAG AA: 9.14) */
--color-text-disabled: #858585;   /* Disabled text */
```

#### Accent Colors
```css
--color-accent-primary: #007ACC;   /* Primary blue */
--color-accent-secondary: #0e639c; /* Secondary blue */
--color-accent-hover: #1177bb;     /* Hover blue */
```

#### Semantic Colors
```css
--color-success: #73C991;
--color-warning: #E5C07B;
--color-error: #F48771;
--color-info: #61AFEF;
```

### Light Theme
IntelliJ Light theme with WCAG AA contrast compliance.

```css
[data-theme="light"] {
    --color-bg-primary: #FFFFFF;
    --color-text-primary: #000000;  /* WCAG AAA: 21.00 */
    /* ... see assets/themes/light.css for full palette */
}
```

### High Contrast Theme
WCAG AAA compliant (7:1 minimum contrast) for maximum accessibility.

```css
[data-theme="high-contrast"] {
    --color-bg-primary: #000000;
    --color-text-primary: #FFFFFF;  /* WCAG AAA: 21.00 */
    /* Enhanced borders, bold text, stronger indicators */
    /* ... see assets/themes/high-contrast.css for full palette */
}
```

---

## Typography

### Font Stack
Primary font: **JetBrains Mono** (preloaded from Google Fonts)

```css
font-family: var(--font-family-mono);
```

Fallback chain: `Menlo → Monaco → Courier New → monospace`

### Type Scale

| Size | CSS Variable | Pixels | Usage |
|------|-------------|--------|-------|
| XS   | `--font-size-xs` | 11px | Status bar, metadata |
| SM   | `--font-size-sm` | 12px | UI labels, secondary text |
| Base | `--font-size-base` | 13px | Editor content, body text |
| MD   | `--font-size-md` | 14px | Headings, emphasis |
| LG   | `--font-size-lg` | 16px | Section titles |
| XL   | `--font-size-xl` | 18px | Page titles |

### Line Height

| Type | CSS Variable | Value | Usage |
|------|-------------|-------|-------|
| Editor | `--line-height-editor` | 20px | Code editor lines |
| Compact | `--line-height-compact` | 1.2 | Dense UI elements |
| Base | `--line-height-base` | 1.5 | Readable text blocks |

---

## Spacing System

The spacing system follows an 8px grid with half-step increments (2px, 4px).

### Spacing Scale

| Token | Size | Usage |
|-------|------|-------|
| `--spacing-xs` | 2px | Minimal padding, tight spacing |
| `--spacing-sm` | 4px | Compact UI elements |
| `--spacing-md` | 8px | Default padding/margin |
| `--spacing-lg` | 12px | Comfortable spacing |
| `--spacing-xl` | 16px | Section separation |
| `--spacing-2xl` | 24px | Large gaps |
| `--spacing-3xl` | 32px | Page-level spacing |

### Application

```css
/* Component padding */
padding: var(--spacing-md);

/* Flex gap */
gap: var(--spacing-lg);

/* Margin between sections */
margin-bottom: var(--spacing-xl);
```

---

## Component Catalog

### Buttons

#### Primary Button
```css
.berry-button {
    background: var(--color-accent-secondary);
    color: var(--color-text-primary);
    padding: var(--spacing-sm) var(--spacing-lg);
    border: none;
    border-radius: var(--border-radius-sm);
    transition: var(--transition-base);
}

.berry-button:hover {
    background: var(--color-accent-hover);
}
```

**Usage**:
```rust
use berry_editor::css_classes::BUTTON;

view! {
    <button class=BUTTON>"Save"</button>
}
```

#### Icon Button
```css
.berry-icon-button {
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-text-secondary);
    padding: var(--spacing-sm);
}

.berry-icon-button:hover:not(:disabled) {
    background: var(--color-bg-hover);
}
```

### File Tree

#### Structure
```
.berry-editor-sidebar
  ├── .berry-sidebar-panel-header
  └── .berry-editor-file-tree
        ├── .berry-project-root
        │     ├── .berry-project-name
        │     └── .berry-project-actions
        └── .berry-editor-file-item
              ├── .berry-editor-folder-icon
              └── (file/folder name text)
```

#### States
- Default: Gray text (`--color-text-tertiary`)
- Hover: Light background (`--color-bg-hover`)
- Selected: Blue background (`#2d333b` / `#CCE8FF`)

#### Accessibility
- Keyboard navigation: Arrow keys
- Screen reader support: ARIA labels on interactive elements
- Focus indicators: 2px outline

### Git UI

#### Staging Area
**File**: `assets/git-ui/staging.css`

Components:
- `.berry-git-panel` - Main container
- `.berry-git-file` - Individual file item
- `.berry-git-file-status` - Status icon (M, A, D, etc.)
- `.berry-git-stage-btn` - Stage/unstage button
- `.berry-git-commit-message` - Commit message textarea
- `.berry-git-commit-btn` - Commit button

#### Commit History
**File**: `assets/git-ui/commit-history.css`

Components:
- `.berry-commit-history` - Scrollable list
- `.berry-commit-item` - Individual commit
- `.berry-commit-hash` - Commit SHA (monospace font)
- `.berry-commit-message` - Commit message text
- `.berry-commit-author` - Author name

#### Branch Manager
**File**: `assets/git-ui/branch-manager.css`

Components:
- `.berry-branch-manager` - Main container
- `.berry-branch-item` - Individual branch
- `.berry-branch-name` - Branch name (monospace)
- `.berry-branch-checkout-btn` - Checkout button
- `.berry-branch-delete-btn` - Delete button (red)

### Command Palette

#### Overlay Pattern
```css
.berry-command-palette-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 9999;
}

.berry-command-palette {
    position: fixed;
    top: 20%;
    left: 50%;
    transform: translateX(-50%);
    width: 600px;
    max-height: 400px;
    background: var(--color-bg-elevated);
    border: 1px solid var(--color-border-strong);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
}
```

#### Keyboard Navigation
- `↑/↓`: Navigate items
- `Enter`: Execute selected command
- `Esc`: Close palette

### Completion Widget (LSP)

#### Structure
```
.berry-completion-widget
  └── .berry-completion-item
        ├── .berry-completion-item-icon
        ├── .berry-completion-item-text
        └── .berry-completion-item-type
```

#### Icon Types
- Function: `λ` or codicon `symbol-function`
- Variable: `x` or codicon `symbol-variable`
- Class: `C` or codicon `symbol-class`
- Module: `M` or codicon `symbol-module`

---

## Accessibility Guidelines

### WCAG 2.1 Compliance

#### Color Contrast Requirements

| Theme | Level | Ratio | Implementation |
|-------|-------|-------|---------------|
| Darcula | AA | 4.5:1 | Default theme |
| Light | AA | 4.5:1 | Light mode |
| High Contrast | AAA | 7:1 | Maximum accessibility |

#### Testing Contrast
All color combinations are documented with their contrast ratios:

```css
/* Example from themes/light.css */
.syntax-keyword {
    color: #000080;  /* Navy - WCAG AA contrast: 8.59 */
}
```

Use tools like [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/) to verify.

### Keyboard Navigation

All interactive elements must be keyboard accessible:

- **Tab**: Move focus forward
- **Shift+Tab**: Move focus backward
- **Enter/Space**: Activate buttons
- **Arrow Keys**: Navigate lists/trees
- **Escape**: Close dialogs/overlays

### Focus Indicators

```css
*:focus {
    outline: 2px solid var(--color-accent-primary);
    outline-offset: 2px;
}
```

High contrast theme uses stronger indicators:
```css
[data-theme="high-contrast"] *:focus {
    outline: 3px solid #00BFFF !important;
    outline-offset: 2px;
}
```

### Screen Reader Support

- Use semantic HTML (`<button>`, `<input>`, etc.)
- Add ARIA labels where needed
- Maintain logical DOM order
- Provide text alternatives for icons

---

## Theme System

### Architecture

Themes are implemented using CSS custom properties (CSS variables) with the `data-theme` attribute on the `<body>` element.

### Available Themes

1. **Darcula** (default) - `data-theme="darcula"`
2. **Light** - `data-theme="light"`
3. **High Contrast** - `data-theme="high-contrast"`

### Theme Switching

#### User Interface
Settings Panel → Theme → Color Theme dropdown

#### Programmatic API
```rust
use berry_editor::settings::EditorSettings;

let settings = EditorSettings::load();
settings.color_theme = "light".to_string();
settings.apply_theme();  // Sets data-theme attribute on body
settings.save();         // Persists to localStorage
```

#### CSS Implementation
```css
/* Base variables in :root */
:root {
    --color-bg-primary: #1E1F22;  /* Darcula default */
}

/* Override for light theme */
[data-theme="light"] {
    --color-bg-primary: #FFFFFF;
}

/* Override for high contrast */
[data-theme="high-contrast"] {
    --color-bg-primary: #000000;
}

/* Component uses the variable */
.berry-editor-pane {
    background: var(--color-bg-primary);
}
```

### Creating a New Theme

1. Create `assets/themes/yourtheme.css`
2. Override CSS variables using `[data-theme="yourtheme"]` selector
3. Add to `index.html` imports
4. Update `EditorSettings::available_themes()` in `src/settings.rs`

---

## CSS Architecture

### File Organization

```
assets/
├── editor.css           # Core editor layout
├── file-tree.css        # File explorer
├── git-ui/
│   ├── staging.css      # Git staging area
│   ├── commit-history.css  # Commit log
│   └── branch-manager.css  # Branch operations
├── diagnostics.css      # LSP diagnostics panel
├── command-palette.css  # Command palette (Cmd+P)
├── completion.css       # LSP autocomplete widget
├── scrollbar.css        # Custom scrollbar styling
└── themes/
    ├── variables.css    # Design tokens (loaded first)
    ├── darcula.css      # IntelliJ Darcula theme
    ├── light.css        # IntelliJ Light theme
    └── high-contrast.css # WCAG AAA theme
```

### Naming Convention

**BEM-like structure**: `.berry-{component}-{element}-{modifier}`

Examples:
- `.berry-editor-pane` (component)
- `.berry-file-item` (component + element)
- `.berry-file-item.selected` (modifier via class)
- `.berry-git-stage-btn` (component + action)

### Rust Constants

All CSS class names have corresponding Rust constants in `src/css_classes.rs`:

```rust
use berry_editor::css_classes::{FILE_ITEM, FILE_ITEM_SELECTED};

let class_name = if is_selected {
    format!("{} {}", FILE_ITEM, FILE_ITEM_SELECTED)
} else {
    FILE_ITEM.to_string()
};
```

Benefits:
- Compile-time checking (no typos)
- IDE autocomplete
- Refactoring support

### Build Process

**Trunk** handles CSS bundling:
1. Parses `<link data-trunk rel="css" href="...">` in `index.html`
2. Processes each CSS file
3. Generates hashed filenames (e.g., `darcula-fdce5178db62ecba.css`)
4. Minifies CSS in release builds (configured in `Trunk.toml`)
5. Injects `<link>` tags in output HTML

---

## Best Practices

### DO ✅

- Use CSS variables for all colors and spacing
- Test in all three themes (Darcula, Light, High Contrast)
- Verify WCAG AA contrast minimums (use browser DevTools or contrast checker)
- Use Rust constants from `css_classes.rs` instead of string literals
- Add `:hover` and `:focus` states for interactive elements
- Use semantic HTML elements (`<button>`, `<input>`, not `<div onclick>`)

### DON'T ❌

- Hardcode color values directly in component CSS
- Use `!important` unless absolutely necessary (theming edge cases only)
- Rely on browser default styles (reset or normalize)
- Use pixel values for font sizes without design token variables
- Skip keyboard accessibility testing
- Forget to test theme switching

---

## Contributing

### Adding a New Component

1. **Create CSS file** in appropriate directory
2. **Define design tokens** in `variables.css` if needed
3. **Add class constants** to `src/css_classes.rs`
4. **Implement Rust component** using constants
5. **Test in all themes**
6. **Document in this file** under Component Catalog
7. **Verify accessibility** (keyboard nav, contrast, focus)

### Updating Existing Component

1. **Update CSS file** with changes
2. **Update constants** in `css_classes.rs` if class names changed
3. **Test theme compatibility** (Darcula, Light, High Contrast)
4. **Update documentation** in this file
5. **Verify WCAG compliance** hasn't regressed

---

## Resources

### Tools
- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/)
- [WAVE Web Accessibility Evaluator](https://wave.webaim.org/)
- Chrome DevTools Accessibility Tab
- Firefox Accessibility Inspector

### References
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [IntelliJ Platform UI Guidelines](https://jetbrains.design/intellij/)
- [CSS Custom Properties Specification](https://www.w3.org/TR/css-variables/)
- [BEM Naming Convention](https://getbem.com/)

### Internal Documentation
- [CLAUDE.md](../CLAUDE.md) - 100% Canvas architecture philosophy
- [src/css_classes.rs](../src/css_classes.rs) - CSS class constants
- [src/settings.rs](../src/settings.rs) - Settings and theme management

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-01-06 | Initial design system documentation |

---

**Maintained by**: BerryEditor Team
**Last Review**: 2026-01-06
**Next Review**: 2026-03-06 (quarterly updates recommended)
