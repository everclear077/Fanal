# ui

The panel is one page and has no bundler. A change is invisible until `cargo build` replaces `fanal.exe`.

Measure `#panel` at the width it is about to become, then clear that width. Measuring at the lip width wraps the rows and flashes a scrollbar.

Keep `#sheet` hidden until `fit_panel` resolves. The idle window is the lip and nothing under it.
