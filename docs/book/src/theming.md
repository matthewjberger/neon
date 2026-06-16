# Theming

A `data-theme` attribute on the document root selects a theme. CSS variable
blocks under `[data-theme="..."]` in `public/styles.css` define each one.

## The themes

Five ship: VS Code Dark (the default, for familiarity), midnight, ember, forest,
and paper. The choice persists in local storage (`theme.rs`).

## Switching

- The theme picker in the top bar previews a theme on hover and applies it on
  click.
- `SPC T` cycles to the next theme.
- The palette has a "Theme: ..." entry per theme.

When you set `state.theme`, an effect applies the `data-theme` attribute and
saves the choice.

## Adding a theme

Add a `[data-theme="yourid"]` block to `public/styles.css` defining the CSS
variables, then add the id and label to the `THEMES` table in `theme.rs`. The
first entry in `THEMES` is the default.
