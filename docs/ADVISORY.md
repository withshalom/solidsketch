# Advisory: SolidSketch (the SketchUp analog) and a BIM sibling (the Revit analog)

Written 2026-11-11. It condenses a working session that read the `solidsketch` repo and the pinned
SolidCraft engine (`eda725dc`, 0.40.0-rc.4). Section 8 lists what was checked and what was not.

> **Scope of this repo:** this repository builds SolidSketch only. The BIM sibling (sections 4 and
> 5) is background on how the two products relate, not work planned here.

## 1. Summary

- **SolidSketch is viable on SolidCraft.** The tools crate passes its 50 tests, and all 5 engine
  commands it emits exist. The remaining work is the app window and the "feels like drawing" layer.
  Rough estimate for one person: 6 to 10 weeks to a usable draw-and-pull alpha. This is a guess
  and rests on two early tests (section 3).
- **First audience: makers and 3D printing.** SketchUp's centre of gravity is architecture,
  engineering and construction, but SolidCraft backs the maker promise today and does not yet
  back an architecture promise.
- **The BIM sibling should be built on SolidCore, not SolidCraft.** SolidCraft's layers are shaped
  for mechanical parts; BIM is a database of building elements and relationships.
- **Share the viewport layer, not plugins.** SolidCraft's plugin system is only a draft RFC, and
  it sits on SolidCraft's command layer, which a SolidCore-based app would not have.
- **Keep the Revit work legally separate** from SolidCraft (see section 4).
- **Name the BIM tool distinctively.** My pick is SolidStory. The "Solid-" prefix is crowded.

## 2. The two products

| | SolidSketch (SketchUp analog) | BIM sibling (Revit analog) |
|---|---|---|
| Base | SolidCraft engine (pinned submodule) | SolidCore (the B-rep kernel) |
| Promise | Draw a shape, pull it into a solid | Coordinated building model: walls, floors, doors, schedules |
| First audience | Makers, 3D printing | Architecture, engineering, construction |
| v1 scope | Flat-faced forms: draw, pull, push, move, paint | To be set by the BIM developer |
| Name | SolidSketch | Candidates in section 4 |

## 3. SolidSketch

### Audience (SketchUp)

- Trimble announced [over one million active subscribers](https://s202.q4cdn.com/922416715/files/doc_news/Trimble-SketchUp-Surpasses-One-Million-Active-Subscribers-2024.pdf)
  in November 2024. Its usage figures include free and trial users, so they are not subscriber
  counts. Its main market is AEC.
- Users named in [one industry summary](https://www.nobledesktop.com/learn/sketchup/sketchup-industries-and-professions):
  architects, interior designers, builders, set and exhibit designers, engineers, some game
  designers. Woodworkers and makers are a large hobbyist group in the community, but I found no
  sourced share for them. I found no breakdown of subscribers by profession.
- SketchUp's win has always been speed and approachability, not precision.

### Recommended goal changes

1. **Decide the first audience: makers and 3D printing.** Export to STEP, STL, 3MF and OBJ exists.
   Game and set designers would want glTF export, which does not. Architecture would want
   dimensioned plans, and SolidCraft's drawings work is still in progress. Revisit after Phase 3.
2. **Narrow v1 to flat-faced forms.** Move Follow Me and the curved tools out of the v1 promise.
   SolidCraft's notes say some coincident curved configurations can still fail in booleans, and
   casual drawing creates coincident faces constantly.
3. **Pull Rotate and Tape Measure forward.** Rotate is `solid.move` with `axis` and `angle`; Tape
   can use `inspect.measure`. Both need only tool logic.
4. **Bring a thin version of the art look into Phase 1.** Paper background, pencil-style edges and
   the material palette are what separate it from a re-skinned SolidCraft. Keep a plain-edge toggle.
5. **wasm32 compiles in CI from day one; ship the web build after Phase 3.**
6. **Set a fork policy.** SolidCraft's contribution rules are strict, so land fixes in your own
   copy first and upstream later; move the pin on purpose. The project names two homes
   (`tangency-labs` and `bherbruck`), so confirm which is canonical.
7. **Add measurable success criteria.** A new person draws a rectangle, pulls it and paints a face
   in under a minute, unaided. Set a push/pull latency budget (starting guess: a drag updates in
   under about 100 ms; adjust after measuring).
8. **Reconsider the "Follow Me" name.** It is a tool name in the best-known product this
   resembles. Not legal advice; worth a look by someone legally minded.

### Work, riskiest first

1. **Draw on a face, then pull it.** Sketch on the face, split the face by the sketch, offset the
   new piece. SolidCraft's tests prove the split; nothing proves that offsetting the patch grows
   side walls. Write that headless test first (days, not weeks). It decides whether the vision
   works on this kernel, which is mid-migration to SMLib, so rerun it when the pin moves.
2. **The window.** `viewport-egui` gives the GPU viewport, but the code that turns a session into
   a viewport frame (about 1,800 lines in `ui-egui/src/gpu.rs`) is not a shared crate. Spike for
   2 to 4 days: one sample solid with orbit, pan and zoom, native first.
3. **Picking and inference.** The picker (about 560 lines in `ui-egui/src/view_pick.rs`) is also
   not shared. Port it, then build endpoint, midpoint and axis snapping. This is most of what
   makes it feel like drawing.
4. **Profile-pull in the tools crate.** `PushPull` only emits `solid.offset_face`. A drawn
   rectangle is a sketch, not a body, so pulling it into a block needs `sketch.finish` then
   `solid.extrude`. `Command::extrude_new` exists but nothing calls it.
5. **The art layer.** Paint and materials exist in the engine; pencil edges and soft shadows are
   yours.
6. **History growth.** Group a gesture into one undo step, fold repeated push/pulls, and measure
   rebuild time on a few hundred features.

### Smaller points

- The README promises glTF export. SolidCraft's `io` crate imports glTF but exports STEP, STL, 3MF
  and OBJ. Drop the claim (natural under makers-first) or add export.
- Save in SolidCraft's own design format. Export comes free, but files are tied to the pinned
  version.
- Desktop first.

## 4. The BIM sibling (Revit analog)

### Why SolidCore, not SolidCraft

- SolidCore is the B-rep kernel only: exact booleans, offset and shell, sweeps, fillets, NURBS,
  mass properties and STEP, in safe Rust that builds for wasm32. Wall openings, roofs and room
  areas come from those.
- SolidCraft adds a feature timeline, persistent face and edge names, a sketch solver, and
  components and joints. These are mechanical-part and mechanical-assembly concepts. BIM needs an
  element and relationship model with geometry as the output; neither project provides it.
- The shared UI crates (viewport, theme, UI kit and others) depend only on each other and on
  SolidCore's public crates, so a BIM app can reuse the viewport without SolidCraft's engine.

### Costs and risks

- SolidCore is 0.0.x with an experimental API. It can panic on geometry it does not handle;
  SolidCraft wraps every kernel call in a `guard` that catches the panic. The BIM app needs the
  same wrapper. Stress wall-opening booleans and roof joins early.
- Undo, the file format and the command surface all have to be rebuilt.
- SolidCore is developed inside the SolidCraft repo (`solidcore/`) and mirrored read-only to
  `tangency-labs/solidcore`. Depend on the mirror at a pinned commit.

### Rules to keep

- SolidCraft's ground rules forbid driving or extracting data from commercial CAD, using another
  product's command ids, or generating test data with it. Revit is commercial. Keep the Revit work
  a separate project that talks to SolidCraft through public formats, its CLI or MCP, and let
  nothing derived from Revit flow back into SolidCraft or its tests.
- Use public specifications for file formats. I did not check Autodesk's SDK terms, or whether
  SolidCraft supports IFC.
- Never put "Revit" or "for Revit" in a product name; it is Autodesk's mark.

### Name candidates

1. **SolidStory.** A story is a building floor, so it reads naturally for BIM, and it matches the
   friendly tone of Sketch and Craft. It does not say "building" by itself, so the tagline must.
2. **SolidFrame.** Clear and structural, but may read as structural-only. Altair sells S-FRAME,
   a structural analysis tool.
3. **SolidPlumb.** Distinctive (true vertical, with a nod to MEP), but sits close to plumbing
   software.
4. **SolidSite.** Simple, but hard to search for.

Avoid SolidBuild: a [SolidBuilder](https://www.capterra.com/p/70088/SolidBuilder/) listing exists.
Others in the space include SolidFace Pro, SolidStructural and SolidPlant 3D. The prefix is also
close to SolidWorks and Solid Edge. I found no exact matches for the four candidates, but web
search is not a trademark check; have someone with legal knowledge look before committing. If the
BIM tool is the successor to KONSTRUCT, a K-name like the rest of KADsuite may fit better than
Solid-.

## 5. What the two projects can share

- **Do share:** the viewport crates, the picking code, the `guard` wrapper, and (possibly) the
  model-to-viewport adapter built once against SolidCore's display plans. That adapter is the
  biggest port in section 3, so building it once pays twice.
- **Do not count on plugins.** `docs/rfcs/0002-plugins.md` is a draft: no host, no WIT file, no
  example plugin, and nothing runs. It is written for SolidCraft only, and its web build is an
  open question.
- **Do agree on:** the SolidCore pin, the file formats you exchange (STEP, 3MF, OBJ exist), and
  who owns the repo you both depend on.
- **Optionally contribute** the BIM developer's needs to the plugin RFC (discussion #1221 and
  draft PR #4980, both under `tangency-labs/solidcraft`).
- The BIM tool also strengthens the architecture path for SolidSketch. As far as I know,
  SketchUp-to-Revit is a common handoff, so clean export into the BIM tool is a real pitch.

## 6. Building with Claude

- **Add a `CLAUDE.md`** to each repo. Claude Code reads `CLAUDE.md`, not `AGENTS.md`; write
  `AGENTS.md` for a tool-neutral version and have `CLAUDE.md` contain `@AGENTS.md`. Keep it under
  about 200 lines of concrete, checkable rules: no new geometry code, no unwrap or panic outside
  tests, every emitted command id registered and listed, licence rules, the checks to run, and
  "look, names and audience are the owner's calls." `BUILD.md` has a draft for SolidSketch.
- **Let Claude drive the engine before writing Rust.** SolidCraft ships `solidcraft-cli` and an
  MCP server (`claude mcp add solidcraft -- solidcraft-cli mcp`). Prove the face-pull chain
  interactively, then freeze it as a test. Building the CLI needs the full engine build, so do it
  on your machine or in CI.
- **Work in small test-first slices**, one per session or PR, and update the status tables after
  each so the next session starts from the docs. Pure logic with tight tests is where Claude is
  strongest: the tools crate, the command-id check, the picker port.
- **Parallel versus sequential.** Independent slices (picker port, art overlay, profile-pull, the
  README fix) can run in parallel in separate worktrees. The chain of window, picking, gestures
  and undo grouping stays sequential.
- **What needs you:** how it feels and looks (Claude can screenshot but cannot judge feel),
  audience and naming decisions, and the one-minute first-use test.
- **Do not shorten the estimate for Claude** until the face-pull test and window spike are done.

## 7. Decisions for you

1. Confirm makers-first as the SolidSketch v1 audience.
2. Is the BIM tool the successor to KONSTRUCT or a separate product? That decides the family
   (Solid- or K-names) and the final name.
3. Which repo is canonical for SolidCraft and SolidCore?
4. Which files do the two projects exchange, and does the BIM developer need IFC?

## 8. What was and was not verified

Checked directly:
- `crates/tools` tests (50 pass) and the command-id script (all 5 ids exist).
- That the toolbar's engine commands are registered in the pinned SolidCraft source.
- The SolidCore README and kernel docs, the layering rules, the plugin RFC, and the ground rules.

Not checked:
- `crates/engine-check` (needs the full engine build; package installs are off here). CI is where
  to confirm it.
- The face-pull chain, the window spike, and the SolidCraft CLI and MCP setup.
- SolidCore's internals beyond its README and docs, IFC support, Autodesk SDK terms, and any
  trademark clearance.
- The 6 to 10 week estimate and the 100 ms latency budget are judgement, not measurements.

## Sources

- [Trimble: SketchUp surpasses one million active subscribers](https://s202.q4cdn.com/922416715/files/doc_news/Trimble-SketchUp-Surpasses-One-Million-Active-Subscribers-2024.pdf)
- [Noble Desktop: SketchUp industries and professions](https://www.nobledesktop.com/learn/sketchup/sketchup-industries-and-professions)
- [Capterra: SolidBuilder](https://www.capterra.com/p/70088/SolidBuilder/)
- [Claude Code memory and CLAUDE.md](https://code.claude.com/docs/en/memory)
- [Claude Code subagents](https://code.claude.com/docs/en/sub-agents)
- SolidCraft repo at the pinned commit: `README.md`, `AGENTS.md`, `docs/rfcs/0002-plugins.md`,
  `docs/layers.md`, `solidcore/README.md`, `skills/solidcraft-ground-rules/SKILL.md`
- `BUILD.md` in the `solidsketch` repo (the detailed SolidSketch plan)
