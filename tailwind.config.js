/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.rs",
  ],
  theme: {
    extend: {
      // RustRover Darcula Theme Colors
      colors: {
        // Background Colors
        'berry-bg-main': '#1e1f22',
        'berry-bg-sidebar': '#313335',
        'berry-bg-activity-bar': '#2B2B2B',
        'berry-bg-gutter': '#1e1f22',
        'berry-bg-statusbar': '#2b2d30',
        'berry-bg-tab-bar': '#313335',
        'berry-bg-tab-inactive': '#313335',
        'berry-bg-tab-hover': '#3c3f41',
        'berry-bg-input': '#3c3f41',
        'berry-bg-elevated': '#2d2d30',
        'berry-bg-hover': 'rgba(255, 255, 255, 0.05)',
        'berry-bg-active': 'rgba(255, 255, 255, 0.1)',

        // Border Colors
        'berry-border': '#393b40',
        'berry-border-subtle': '#1e1f22',
        'berry-border-default': '#323232',
        'berry-border-strong': '#3e3e3e',
        'berry-border-accent': '#454545',
        'berry-border-error': '#be1100',

        // Text Colors (WCAG AA Compliant)
        'berry-text': '#bcbec4',
        'berry-text-primary': '#ffffff',
        'berry-text-secondary': '#d4d4d4',
        'berry-text-tertiary': '#bcbec4',
        'berry-text-muted': '#7a7e85',
        'berry-text-dim': '#606366',
        'berry-text-disabled': '#858585',
        'berry-text-active': '#ffffff',
        'berry-text-darcula': '#a9b7c6',

        // Selection & Hover
        'berry-selection-bg': '#4B6EAF',
        'berry-selection-fg': '#ffffff',
        'berry-hover-bg': '#4D5157',
        'berry-selection-active': '#094771',
        'berry-selection-border': '#007acc',

        // Icon Colors
        'berry-icon-muted': '#858585',

        // Accent Colors
        'berry-accent': '#007acc',
        'berry-accent-secondary': '#0e639c',
        'berry-accent-hover': '#1177bb',

        // Semantic Colors
        'berry-success': '#6a8759',
        'berry-warning': '#cca700',
        'berry-error': '#f48771',
        'berry-info': '#75beff',

        // Semantic Backgrounds
        'berry-bg-success': '#2d3d2a',
        'berry-bg-warning': '#3d3a2a',
        'berry-bg-error': '#5a1d1d',
        'berry-bg-info': '#1a3a4a',

        // Syntax Highlighting (IntelliJ Darcula)
        'berry-syntax-keyword': '#cc7832',
        'berry-syntax-function': '#ffc66d',
        'berry-syntax-type': '#a9b7c6',
        'berry-syntax-string': '#6a8759',
        'berry-syntax-number': '#6897bb',
        'berry-syntax-comment': '#629755',
        'berry-syntax-operator': '#a9b7c6',
        'berry-syntax-identifier': '#a9b7c6',
        'berry-syntax-attribute': '#bbb529',
        'berry-syntax-constant': '#9876aa',

        // Git/VCS Colors
        'berry-git-added': '#4ec9b0',
        'berry-git-modified': '#cca700',
        'berry-git-deleted': '#f48771',
        'berry-git-ignored': '#858585',

        // Scrollbar Colors
        'berry-scrollbar-track': '#2b2d30',
        'berry-scrollbar-thumb': '#4e5254',
        'berry-scrollbar-thumb-hover': '#5a5d60',
        'berry-scrollbar-thumb-active': '#6a6d70',
      },

      // Typography
      fontFamily: {
        'mono': ['JetBrains Mono', 'Menlo', 'Monaco', 'Courier New', 'monospace'],
        'sans': ['-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'system-ui', 'sans-serif'],
      },
      fontSize: {
        'editor': '13px',
        'tree': '13px',
      },
      lineHeight: {
        'editor': '20px',
        'tree': '22px',
        'tight': '1.3',
        'relaxed': '1.6',
      },
      letterSpacing: {
        'tight': '-0.01em',
      },

      // Spacing (4px baseline)
      spacing: {
        'gutter': '55px',
      },

      // Border Radius
      borderRadius: {
        'berry-sm': '2px',
        'berry-md': '3px',
        'berry-lg': '6px',
      },

      // Z-Index Layers
      zIndex: {
        'base': '1',
        'dropdown': '10',
        'sticky': '100',
        'overlay': '1000',
        'modal': '10000',
      },

      // Box Shadow
      boxShadow: {
        'berry-sm': '0 2px 8px rgba(0, 0, 0, 0.4)',
        'berry-md': '0 4px 16px rgba(0, 0, 0, 0.6)',
        'berry-lg': '0 4px 16px rgba(0, 0, 0, 0.8)',
      },

      // Opacity Levels
      opacity: {
        'disabled': '0.4',
        'muted': '0.5',
        'hover': '0.6',
      },

      // Transitions
      transitionDuration: {
        'fast': '0.1s',
        'slow': '0.2s',
      },
    },
  },
  plugins: [],
}
