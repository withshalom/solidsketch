# SolidSketch plan

The goal in one line: **a 3D modeler anyone can pick up in a minute, with a real CAD engine underneath.**

## Principles

1. **The first minute is drawing.** No planes to pick, no constraint dialogs, no timeline on screen.
2. **Precision is optional and always one keystroke away.** Type a number while drawing or pulling.
3. **No new geometry code.** SolidSketch is a front end. Modelling is SolidCraft's engine and its kernel;
   SolidSketch adds tools, feel and look. If a tool needs geometry the engine lacks, the fix goes to SolidCraft.
4. **Tools are plain logic.** Pointer events in, engine commands out ([`crates/tools`](crates/tools)), so
   behaviour is unit-tested and runs on desktop and in the browser.
5. **Never crash on input.** Coordinates, typed text and picks are untrusted: errors are values.
6. **Independent.** No other product's code, icons or text.

## Architecture

```
App (egui; desktop + wasm)          -- window, viewport, toolbar, panels, theme
  └─ solidsketch-tools              -- Event -> Preview + Command (this repo, no dependencies)
       └─ SolidCraft engine         -- Session::execute(id, params): everything is a command
            └─ solidcore            -- B-rep kernel, display tessellation
```

SolidCraft comes in as a git submodule, pinned. The SolidCraft architecture notes say its *shared* crates
(`viewport`, `viewport-gpu`, `viewport-egui`, `theme`, `ui-kit`, `look`, `control`) are built for other
front ends to depend on, so the app uses those for the 3D view and chrome and writes its own toolbar,
status bar and tray.

## Phases

### Phase 0: foundation (done)
- Repository, licences, CI, SolidCraft submodule pinned at a known commit.
- Interface design as a clickable prototype; README pictures generated from it.
- Tool logic for Line, Rectangle, Circle and Push/Pull with the value box, 50 unit tests.
- `check-command-ids.sh`: every command the tools emit is registered in the engine.
- `crates/engine-check`: the same commands run on a real `Session` (runs in CI).

### Phase 1: the window
Exit: launch the app, see a sample solid lit and shaded, orbit, pan and zoom, and see the toolbar,
status bar and material tray from the design.
- App crate hosting `solidcraft_engine::Session` in an eframe window.
- 3D view from `viewport-egui` / `viewport-gpu`; camera, navigation, theme tokens.
- Toolbar and status bar driven by `Tool::ALL`, `label`, `key`, `hint`.
- Web build (wasm32) from day one.

### Phase 2: draw and pull
Exit: draw a rectangle on the ground, pull it into a block, push a face in, undo, redo, all with the pointer
and with typed values.
- Pointer to plane point with inference (endpoints, midpoints, axes).
- Line, Rectangle, Circle through `Drawing`; Push/Pull through `PushPull` and `solid.offset_face`.
- Face picking returns a `FaceRef` (a point on the face, or its stable name).
- Live previews from `Preview`; value box from `Output.readout`.

### Phase 3: direct-modelling feel
Exit: draw a rectangle *on the side of a block* and pull it out as a window recess, without the user
thinking about sketches.
- Drawing on a face splits it, so the new shape is pullable. SolidCraft already has face splitting (see its `split_face` tests);
  this is the largest piece of work in the project.
- Group the engine commands of one gesture into one undo step; collapse repeated push/pulls of the same
  face so the hidden history does not grow without bound.
- Move, Rotate, Scale (`solid.move`, `solid.scale`); components for repeated objects.

### Phase 4: the art layer
Exit: a finished model looks good enough to share as a picture.
- Paint and materials (`appearance.paint`, `appearance.pick_up`); the material tray.
- Edge styles (clean, pencil), soft shadows, plan view, scenes and image export.

### Phase 5: curves and organic shapes
- Arcs, offset, Follow Me (`solid.sweep`), and the surfacing tools SolidCraft is still finishing.

### Phase 6: share and ship
- STL, 3MF and glTF export, templates and examples, a hosted web build, installers.

## Hard problems and the current thinking

| Problem | Why it is hard | Current thinking |
|---|---|---|
| Drawing on a face splits it | The kernel must split a face by an open or closed curve and keep names stable | Use SolidCraft's face-split and persistent naming; prove it with a headless test before building the gesture |
| History growth | Each push/pull is a timeline feature, and casual use makes hundreds | Fold consecutive edits of the same face into one feature; keep the timeline hidden but exportable |
| Face identity across edits | A pick must still mean the same face after a rebuild | Prefer stable face names over points once a face has one |
| Organic shapes | Strict solids are less forgiving than loose geometry | Start with curve tools that map to real operations; consider a mesh body mode later |
| Feel | A tool that lags is a tool nobody uses | Measure push/pull latency in Phase 2 before polishing anything else |
| Kernel change | SolidCraft's kernel has moved before | Stay behind the engine's command interface; pin the submodule and update on purpose |

## How we check our work

- `cargo test`: tool behaviour, including hostile input, with no dependencies.
- `scripts/check-command-ids.sh`: no tool emits a command the engine does not have.
- `crates/engine-check`: the engine accepts what the tools emit (CI).
- From Phase 1: scripted UI scenarios and screenshots, so a change to the look shows up as a picture.

## Open questions

- Which first audience: makers and 3D printing, game and set designers, or architecture sketching?
  It decides which tools come after Phase 3.
- Desktop first, or web first? The code is the same; the first release is not.
- Do models save in SolidCraft's own format, or does SolidSketch need a simpler file of its own?
