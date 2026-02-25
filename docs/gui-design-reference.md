# GUI Design Reference — tropa-relay

Research and best practices for overhauling the GUI into something that actually looks good.

---

## Current Problems

1. **Add/edit proxy is a modal inside the main window** — feels cramped, not a real dialog
2. **Inconsistent button heights** — buttons across the UI don't share a baseline size
3. **Bad text placement and spacing** — elements feel randomly placed, no visual rhythm
4. **The iOS pill toggle is out of place** — mobile metaphor jammed into a desktop app
5. **Everything looks flat and dead** — no hover feedback, no depth, no sense of interaction

---

## 1. Separate Window for Add/Edit Proxy

egui supports native multi-window via the viewport system. Use `ctx.show_viewport_immediate()` to open add/edit as an actual OS-level window.

### How it works

- Call `ctx.show_viewport_immediate()` every frame while the dialog should be visible
- Gate it behind a `bool` flag (e.g. `show_edit_viewport`)
- When the viewport's close is requested, set the flag to `false`
- Use `ViewportBuilder` to set title, size, and position

### Pattern

```rust
if self.show_edit_viewport {
    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("edit_proxy"),
        egui::ViewportBuilder::default()
            .with_title("Edit Proxy")
            .with_inner_size([420.0, 380.0]),
        |ctx, class| {
            assert!(class == egui::ViewportClass::Immediate);
            egui::CentralPanel::default().show(ctx, |ui| {
                // form fields here
            });
            if ctx.input(|i| i.viewport().close_requested()) {
                self.show_edit_viewport = false;
            }
        },
    );
}
```

### Important

- Must call `ctx.set_embed_viewports(false)` once at startup, otherwise viewports get embedded as panels instead of real windows
- Immediate viewports share the parent's repaint cycle — fine for a simple form dialog
- No need for `Arc`/`Mutex` since immediate viewports run synchronously

### Sources

- [egui viewport docs](https://docs.rs/egui/latest/egui/viewport/index.html)
- [egui multiple viewports example](https://github.com/emilk/egui/blob/main/examples/multiple_viewports/src/main.rs)
- [ViewportBuilder docs](https://docs.rs/egui/latest/egui/viewport/struct.ViewportBuilder.html)

---

## 2. Consistent Sizing via 8px Grid

All spacing, padding, margins, and component sizes should snap to multiples of **8px**. Use 4px only for tight inner gaps.

### The system

| Token    | Value | Use case                              |
|----------|-------|---------------------------------------|
| `xs`     | 4px   | Icon-to-label gap, tight inner space  |
| `sm`     | 8px   | Inner padding, related element gap    |
| `md`     | 16px  | Card padding, section gap             |
| `lg`     | 24px  | Between card groups, panel margins    |
| `xl`     | 32px  | Major section breaks                  |

### Button sizes

Pick ONE height for all standard buttons and stick to it:

- **Standard button height: 32px** (4 grid units)
- **Small/inline buttons: 24px** (3 grid units)
- Minimum touch/click width: 64px for labeled buttons
- Horizontal padding inside buttons: 16px

All buttons — Save, Cancel, Delete, Edit, Add Proxy — use the same height. No exceptions. The "chunky vs tiny" inconsistency dies here.

### In egui terms

```rust
// Define once, use everywhere
const BTN_H: f32 = 32.0;
const BTN_PAD: egui::Vec2 = egui::vec2(16.0, 0.0);

// Usage
ui.add(egui::Button::new("Save").min_size(egui::vec2(64.0, BTN_H)))
```

### Sources

- [The 8pt Grid System](https://www.rejuvenate.digital/news/designing-rhythm-power-8pt-grid-ui-design)
- [Spacing best practices (8pt grid, internal <= external)](https://cieden.com/book/sub-atomic/spacing/spacing-best-practices)
- [Space, grids, and layouts](https://www.designsystems.com/space-grids-and-layouts/)

---

## 3. Text Alignment and Visual Hierarchy

### Principles (from Apple HIG, adapted for desktop)

- **Hierarchy through size, weight, position** — not decoration
- **Primary info** gets larger/bolder text, sits at the top/left
- **Secondary info** is smaller, weaker (dimmer) color
- **Actions** are visually distinct from content and pushed to consistent positions (e.g. always right-aligned)
- **Group related things** with proximity; separate unrelated things with space

### Concrete rules for our proxy cards

```
┌─ card (16px padding, 8px corner radius) ──────────────┐
│                                                        │
│  proxy-name (16px, bold)              [ON/OFF toggle]  │
│  ↕ 4px                                                 │
│  host:port → local port (13px, weak/dim)               │
│  ↕ 8px                                                 │
│                                    [Edit]  [Delete]    │
│                                                        │
└────────────────────────────────────────────────────────┘
 ↕ 8px gap between cards
```

- Name and toggle on the same horizontal line, vertically centered
- Subtitle directly below with minimal gap (4px)
- Action buttons right-aligned, consistent height
- Card internal padding: 16px all sides (2 grid units)
- Card-to-card gap: 8px

### In egui terms

Use `ui.add_space()` explicitly between elements instead of relying on default `item_spacing` which applies everywhere equally. Control spacing surgically:

```rust
ui.spacing_mut().item_spacing.y = 4.0; // tight within card
// or use ui.add_space(4.0) between specific elements
```

### Sources

- [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/)
- [Liquid Glass: Hierarchy, Harmony and Consistency](https://www.createwithswift.com/liquid-glass-redefining-design-through-hierarchy-harmony-and-consistency/)

---

## 4. Replace the Pill Toggle

The iOS-style pill switch looks alien on desktop. Two better alternatives:

### Option A: Styled checkbox (recommended)

Just use egui's built-in checkbox but with better visuals:
- Increase the checkbox size via `Spacing::icon_width` and `Spacing::icon_width_inner`
- Use a colored fill (green/accent) when checked instead of just a checkmark
- Round the checkbox corners slightly

This is the least surprising approach — desktop users expect checkboxes, not phone sliders.

### Option B: Segmented text toggle

A small two-state button like `[ON | OFF]` where the active side is filled:

```rust
fn on_off_toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let text = if *on { "ON" } else { "OFF" };
    let color = if *on { accent_color } else { inactive_color };
    // Paint as a small rounded button with the appropriate fill
}
```

This reads as a desktop control, not a mobile widget.

### Option C: Filled checkbox with custom paint

Override the checkbox painting to show a filled rounded square when on, empty when off. No sliding knob, no pill shape — just a clear on/off state indicator.

### Sources

- [Toggle UX: Tips on Getting it Right](https://www.eleken.co/blog-posts/toggle-ux)
- [Checkbox vs Toggle Switch: When to Use Which](https://blog.uxtweak.com/checkbox-vs-toggle-switch/)
- [Checkbox UI design best practices](https://blog.logrocket.com/ux-design/checkbox-ui-design-best-practices-examples/)

---

## 5. Depth, Hover Feedback, and Responsiveness

The UI needs to feel alive. Every interactive element must visually respond to the user.

### Depth via Soft UI / subtle shadows

Not full neumorphism — just enough depth so cards don't look like flat colored rectangles:

- Cards get a subtle `Shadow` (small offset, soft blur, low opacity)
- Active/running proxies could have a slightly different card background or a subtle left-border accent
- Windows already get `window_shadow` from egui — make sure it's visible

### egui shadow API

```rust
// Card shadow
egui::Frame::new()
    .fill(card_color)
    .shadow(egui::Shadow {
        offset: egui::vec2(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(25),
    })
    .corner_radius(8)
    .inner_margin(16)
```

### Hover feedback via widget visuals

egui already supports per-state widget styling. Make the differences VISIBLE:

```rust
let mut style = (*ctx.style()).clone();

// Inactive (default, not hovered)
style.visuals.widgets.inactive.bg_fill = subtle_bg;
style.visuals.widgets.inactive.weak_bg_fill = subtle_bg;

// Hovered — make it obviously different
style.visuals.widgets.hovered.bg_fill = hover_bg;        // lighter/brighter
style.visuals.widgets.hovered.bg_stroke = hover_stroke;   // visible border
style.visuals.widgets.hovered.weak_bg_fill = hover_bg;

// Active (being clicked)
style.visuals.widgets.active.bg_fill = active_bg;         // even more distinct
```

### Card hover effect

For the proxy cards specifically, detect hover on the `Frame` response and change the fill:

```rust
let response = egui::Frame::new()
    .fill(if hovered { hover_fill } else { normal_fill })
    .shadow(shadow)
    .show(ui, |ui| { /* card content */ });

let hovered = response.response.hovered();
```

Note: since egui is immediate mode, you paint with last frame's hover state — this creates a natural one-frame-delayed response that actually feels fine in practice.

### Interactive cursor

egui changes the cursor to a pointer on interactive elements by default. Make sure frameless buttons (`Button::new("Edit").frame(false)`) still show pointer cursor — they should, since they're still buttons.

### Sources

- [egui `Visuals` struct](https://docs.rs/egui/latest/egui/style/struct.Visuals.html)
- [Shadows in UI design](https://blog.logrocket.com/ux-design/shadows-ui-design-tips-best-practices/)
- [egui-desktop hover effects](https://github.com/PxlSyl/egui-desktop)
- [Neumorphism guide](https://clay.global/blog/neumorphism-ui)

---

## 6. Global Theme Refinements

### Font

The default egui font is fine but thin. Consider loading **Inter Medium** (what Rerun uses) via `Context::set_fonts()` for a more substantial feel. If not worth the binary size, at least bump the default sizes.

### Color palette

Pick 4-5 colors max:

| Role        | Example                  | Usage                           |
|-------------|--------------------------|---------------------------------|
| Background  | `#1a1a2e` / `#f5f5f5`   | Panel/window fill               |
| Card        | `#252540` / `#ffffff`    | Card fill, slightly off-bg      |
| Accent      | `#3b82f6` (blue)         | Primary actions (Save, Add)     |
| Danger      | `#dc3545` (red)          | Destructive actions (Delete)    |
| Text        | `#e0e0e0` / `#1a1a1a`   | Primary text                    |
| Text weak   | `#888888`                | Secondary/subtitle text         |

### Corner radius

Be consistent:
- Cards: 8px
- Buttons: 6px
- Input fields: 6px
- Windows: 10px
- Checkboxes/toggles: 4px

### Sources

- [re_ui crate (Rerun's theme library)](https://crates.io/crates/re_ui)
- [egui_colors (community color toolkit)](https://github.com/frankvgompel/egui_colors)
- [egui Style docs](https://docs.rs/egui/latest/egui/style/struct.Style.html)

---

## 7. General Design Principles to Follow

Distilled from Apple HIG, modern UI trends, and common sense:

1. **Internal spacing <= external spacing** — space inside a card is tighter than space between cards
2. **Fewer competing elements per screen** — if everything is bold, nothing is
3. **Consistent alignment** — pick left-align for labels, right-align for actions, and never deviate
4. **Motion as feedback, not decoration** — hover transitions and toggle animations confirm the user did something
5. **Progressive disclosure** — the main screen shows the minimum (name, status, toggle). Details live in the edit window
6. **Respect dark mode** — test both. Card backgrounds need to differ from panel backgrounds in both themes

### Sources

- [Apple HIG](https://developer.apple.com/design/human-interface-guidelines/)
- [10 UI Design Principles for 2026](https://www.lyssna.com/blog/ui-design-principles/)
- [UI Design Trends 2026](https://www.index.dev/blog/ui-ux-design-trends)
- [Liquid Glass design principles (WWDC 2025)](https://developer.apple.com/videos/play/wwdc2025/219/)

---

## Implementation Priority

1. **Separate viewport for add/edit** — biggest UX win, removes the cramped modal
2. **8px grid + consistent button heights** — fixes the sizing mess in one pass
3. **Card depth (shadows) + hover feedback** — makes the UI feel alive
4. **Replace pill toggle** — swap for styled checkbox or segmented toggle
5. **Text hierarchy cleanup** — surgical spacing adjustments within cards
6. **Color/theme polish** — last pass, fine-tune the palette
