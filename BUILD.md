# SolidSketch build-out

What it takes to turn SolidSketch from tool logic and a prototype into a working app. Written
2026-11-10 after reading the repository and the pinned SolidCraft engine
(`eda725dc`, SolidCraft 0.40.0-rc.4). Revised the same day to add recommended goal changes and
advice for building with Claude.

## Bottom line

The vision holds up. SolidCraft already does most of the hard geometry, so the remaining work is
the app window and the "feels like drawing" layer. Rough estimate for one person: **6 to 10 weeks
to a usable draw-and-pull alpha**. That figure is a guess and rests mostly on items 1 and 2 below,
so settle those first. Do not shorten it for Claude until both spikes are done (see
[Building with Claude](#building-with-claude)).

## Recommended goal changes

None of these abandon the vision. They narrow the first release so the promises in the README are
the ones the engine can keep.

1. **Decide the first audience: makers and 3D printing.** PLAN.md leaves this open. The one
   promise the engine fully backs today is "a true solid that measures, exports and prints":
   STEP, STL, 3MF and OBJ export exist. Game and set designers would want glTF export, which does
   not exist. Architecture would want dimensioned plans, and SolidCraft's drawings work is still
   in progress. Revisit after Phase 3.
2. **Narrow v1 to flat-faced forms.** Draw, pull, push, move, paint. Move Follow Me and the
   curved and organic tools (Phase 5) out of the v1 promise and leave them greyed as "planned",
   as the prototype already does. SolidCraft's own notes say some coincident curved
   configurations can still fail in booleans, and casual drawing produces coincident faces
   constantly. Flat faces are the best-supported path.
3. **Pull Rotate and Tape Measure forward.** They no longer need engine work: Rotate is
   `solid.move` with `axis` and `angle`, and Tape can use `inspect.measure` or be computed in the
   tools crate. Only tool logic remains, so they belong in Phase 2 or early Phase 3, not "planned".
4. **Bring a thin version of the art look into Phase 1.** Paper background, pencil-style edge
   overlay and the material palette are cheap, and they are what separates SolidSketch from a
   re-skinned SolidCraft. Early screenshots should already tell that story. Keep a plain-edge
   toggle.
5. **Change "web from day one" to "wasm32 compiles in CI from day one".** SolidCraft already
   gates every crate below its apps on a wasm32 build, so a compile check costs little and catches
   regressions. Debugging GPU paths on two targets during the window spike would slow it down, so
   ship the web build after Phase 3.
6. **Set a fork policy.** PLAN.md principle 3 says a missing geometry feature gets fixed in
   SolidCraft. SolidCraft's contribution rules are strict (claim with a draft PR, run
   `cargo xtask ci`, no `unwrap` or `panic` in non-test code, one topic per PR), so do not block
   on upstream merges. The submodule already points at `withshalom/solidcraft`: land fixes there
   first and upstream them later, and move the pin on purpose. SolidCraft's own README and
   `Cargo.toml` name other homes for the project (`tangency-labs`, `bherbruck`); confirm which is
   canonical before deciding what to track.
7. **Add measurable success criteria.** The README's "first minute" promise becomes a check: a
   person who has never seen it draws a rectangle, pulls it and paints a face in under a minute,
   unaided. Only a person can run that test. Also set a push/pull latency budget before Phase 2
   polish. As a starting point to adjust (not a measured number): a drag should update in under
   about 100 ms.
8. **Reconsider the "Follow Me" name.** It is the name of a tool in the best-known product this
   resembles. The README disclaims affiliation and TRADEMARK.md claims SolidSketch's own mark, so
   a distinctive name is cheap now (one string in `tool.rs`, the README and the prototype) and
   costly later. This is not legal advice; it is worth a legal-minded look.

Keep principle 3 (no new geometry code) and principle 5 (never crash on input). Item 1 below may
test principle 3.

## What is already true

Checked directly:

- `cargo test` in `crates/tools`: 50 tests pass.
- `scripts/check-command-ids.sh`: all 5 commands the tools emit exist in SolidCraft
  (`sketch.line`, `sketch.rectangle.two_point`, `sketch.circle.center`, `solid.extrude`,
  `solid.offset_face`).

Not checked:

- `crates/engine-check` (runs the commands on the real engine). It needs the full engine build, so
  it was not run. CI is the place to confirm it.

## Toolbar against the engine

Every toolbar tool maps to a command SolidCraft registers today. Only the ids marked "registered"
were looked up in the engine source.

| Tool | Engine command | State |
|---|---|---|
| Line, Rectangle, Circle | `sketch.line`, `sketch.rectangle.two_point`, `sketch.circle.center` | In the tools crate |
| Arc (Phase 5) | `sketch.arc.center_point`, `.tangent`, `.three_point` | Registered |
| Push/Pull | `solid.offset_face`, `solid.press_pull` | `offset_face` in tools; `press_pull` unused |
| Move | `solid.move` | Registered |
| Rotate | `solid.move` with `axis` and `angle` | Registered; tool logic only (goal change 3) |
| Scale | `solid.scale` | Registered |
| Follow Me | `solid.sweep` | Registered; out of v1 (goal change 2) |
| Paint | `appearance.paint`, `appearance.pick_up` | Registered |
| Tape Measure | `inspect.measure` (or compute client-side) | Registered; tool logic only (goal change 3) |
| Undo / redo | `edit.undo`, `edit.redo` | Registered |
| Plan view | `view.top` | Registered |
| Drawing on a face | `sketch.create` with `{face: [x,y,z]}`, then `solid.split_face` with `sketch` | Registered; see item 1 |

## Work, riskiest first

### 1. Draw on a face, then pull it (Phase 3)

The chain is: sketch on the face, split the face by that sketch, then offset the new piece.
SolidCraft's tests prove the split (`split_face_tests.rs`). Nothing proves that offsetting a
split-off patch grows side walls.

- Write a headless test in `crates/engine-check`: box, sketch a rectangle on its top face, finish,
  `solid.split_face` by the sketch, `solid.offset_face` on the new piece, then check volume and
  face count.
- Also try a rectangle on a side face, and a circle.
- This takes days, not weeks, and decides whether the vision works on this kernel. The kernel is
  mid-migration from truck to SMLib (`kernel-smlib` branch, not yet in releases), so run it again
  when the pin moves.
- Exit: the test passes, or the result tells you which command to ask SolidCraft to change.

### 2. The window (Phase 1)

`viewport-egui` gives the GPU viewport and `viewport` gives the camera. Turning a `Session` into
the viewport's `Frame` is done in SolidCraft's `ui-egui` crate (about 1,800 lines in `gpu.rs`),
which is not a shared crate.

- Spike: an eframe app that opens a `Session`, builds one sample solid and draws it with orbit,
  pan and zoom. Budget 2 to 4 days.
- The spike answers whether to port that code, rewrite it, or start from the CPU rasterizer in
  `crates/viewport/src/raster.rs` (not evaluated; it might suit the paper-and-pencil look).
- Include the thin art look from goal change 4.
- Exit: a lit solid in a window, native first, with wasm32 compiling in CI.

### 3. Picking and inference

- The mouse picker (`view.pick`, about 560 lines in `ui-egui/src/view_pick.rs`) is also not shared.
  It answers body, face index, world point and normal from a pixel. Port it.
- Then build inference: endpoints, midpoints, on-edge, axis lock. This is most of what makes the
  tools feel like drawing, and it feeds `P2` points into `Drawing`.
- Exit: hover highlights the face under the pointer; clicks snap to endpoints and axes.

### 4. A gap in the tools crate (done)

**Status: done.** `PushPull::begin_profile` pulls a drawn profile into a block; `commit` now
returns the commands in order (`sketch.finish`, `solid.extrude`) and `sketch.finish` is in
`ENGINE_COMMANDS`. The engine-check test for it is written but has not been run here; CI runs it.

`PushPull` only emitted `solid.offset_face`. A drawn rectangle on the ground is a sketch, not a
body, so pulling it into a block needs the extrude path. `Command::extrude_new` existed and the
engine test used it, but nothing called it.

- Add a profile-pull mode to `PushPull`: the host passes "profile" instead of a face, and commit
  emits `sketch.finish` then `solid.extrude`.
- Add unit tests alongside the existing 9 in `pushpull.rs`, including hostile input.
- Exit: draw a rectangle, press `P`, drag, and a block exists.

### 5. The art layer (Phase 4)

- Paint and materials: engine commands exist (`appearance.paint`, `appearance.library`); the tray
  is UI work.
- Pencil edges and soft shadows are SolidSketch's own. Drawing edges as overlay lines with the
  same jitter as the prototype is probably the cheap route.
- Plan view is `view.top`.

### 6. History growth

Each gesture becomes a timeline feature, and casual use makes hundreds.

- Group one gesture's commands into one undo step. Unproven; check how `sketch.finish` and
  `edit.checkpoint` behave together.
- Fold consecutive push/pulls of the same face into one feature.
- Measure rebuild time on a few hundred features before polishing anything else.

## README fix

The README says models export to STEP, STL, 3MF and glTF. In SolidCraft's `io` crate, glTF is an
import path only; export is STEP, STL, 3MF and OBJ. Either drop glTF from the promise or add
export. Check against the pinned commit before editing. Goal change 1 (makers first) makes
dropping it the natural choice.

## Building with Claude

### Set the repo up so a fresh session can work

- Add a `CLAUDE.md` at the repo root. Claude Code reads `CLAUDE.md`, not `AGENTS.md`. If you want
  a tool-neutral file, write `AGENTS.md` and make `CLAUDE.md` contain `@AGENTS.md` plus any
  Claude-specific rules. Keep it under about 200 lines, with concrete, checkable rules.
- SolidCraft is a good model. Its `AGENTS.md` is a short router: hard rules, then which skill to
  read for each kind of work. For engine work, point Claude at SolidCraft's `skills/` directory in
  the submodule rather than copying it.
- Draft to start from:

```markdown
# SolidSketch

A friendly 3D modeler on the SolidCraft engine (git submodule `solidcraft/`, pinned).
Read PLAN.md for phases and BUILD.md for the current build-out plan.

## Rules
- No new geometry code. Modelling is SolidCraft's; if the engine lacks something, say so.
- `crates/tools` has no UI toolkit and no kernel dependency: pointer events in, commands out.
- Input is untrusted. Errors are values. No unwrap, expect or panic outside tests.
- Every command id a tool emits must be listed in `ENGINE_COMMANDS` (command.rs, between the
  markers) and registered in the engine.
- Licence is MIT OR Apache-2.0. No GPL, LGPL or AGPL code. No other product's code, icons or text.
- `docs/prototype/index.html` copies tool hints from `tool.rs`; keep them in step.
- Look, feel, tool names and audience are the owner's calls. Ask; do not decide.

## Before finishing a change
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
scripts/check-command-ids.sh
cargo test --manifest-path crates/engine-check/Cargo.toml   # large build, needs the submodule

Toolchain is 1.98.0, as in CI. Update the status tables in README.md, PLAN.md and BUILD.md when
a slice lands.
```

- `CLAUDE.md` guidance is followed but not enforced. For steps that must always run, use a hook or
  let CI decide. CI already runs format, clippy, tests and the command-id check, so keep it as the
  arbiter and have Claude read its logs.

### Let Claude drive the engine before writing Rust

SolidCraft ships `solidcraft-cli` (`commands`, `run`, `eval`, `snapshot`) and an MCP server
(`solidcraft-cli mcp`). Its README gives the Claude Code setup line:
`claude mcp add solidcraft -- solidcraft-cli mcp`. Use it to:

- Prove item 1 interactively: `sketch.create` on a face, draw, `sketch.finish`,
  `solid.split_face`, `solid.offset_face`, then `inspect.measure` and a snapshot.
- Look up exact parameters with `solidcraft-cli commands` instead of guessing.
- Freeze the working command sequence as an `engine-check` test once it behaves.

SolidCraft's modelling skills are also exposed through the MCP server's `skill_reference` tool.
Building `solidcraft-cli` needs the full engine build, so do this on your machine or in CI.

### Work in small test-first slices

- Where Claude is strongest here: pure logic with tight tests. The tools crate, the command-id
  check, `engine-check` tests, and the picker port with its own tests all fit.
- One slice per session or PR: for example item 4, then the README fix, then the face-pull test.
  Update the status tables after each, so the next session starts from the docs, not from chat
  history.
- Ask for hostile-input tests on anything that takes pointer or typed input, as the existing
  tests do.

### Parallel and sequential work

- Independent slices can run in parallel: the picker port, the art overlay, profile-pull in
  `PushPull`, the README fix. In Claude Code, subagents (Markdown files in `.claude/agents/`) run
  in their own context, and `isolation: worktree` gives each its own copy of the repo so edits do
  not collide.
- The dependent chain stays sequential: window, then picking, then gestures, then undo grouping.
  SolidCraft's `AGENTS.md` states the same rule.

### What needs you

- **Feel and look.** Claude can build a scripted scenario, take a screenshot and compare it with
  `docs/hero.png`, but it cannot tell whether a drag feels live or the pencil edges look right.
  Use the prototype as the reference, and judge in person.
- **Decisions.** Audience, names, what to cut. Keep them out of Claude's hands by saying so in
  `CLAUDE.md`.
- **The first-minute test** from goal change 7.

### Licences and assets when porting from SolidCraft

SolidCraft's code is MIT OR Apache-2.0, the same as SolidSketch, so porting the picker or frame
code is fine; keep the notices. Check `ATTRIBUTION.md` before reusing any SolidCraft asset such as
icons or fonts, since each has its own row there. Tell Claude never to pull in GPL, LGPL or AGPL
code, which SolidCraft itself forbids.

## Decisions

- **Save format:** use SolidCraft's own design format (`file.save`, `file.save_as`). Save and
  export come free. The cost is that files are tied to the pinned version, and SolidCraft's older
  builds refuse designs saved by newer ones.
- **First audience:** makers and 3D printing (goal change 1).
- **Desktop or web first:** desktop first; wasm32 in CI from the start (goal change 5).

## Suggested order

1. Face-pull test (item 1).
2. Window spike with the thin art look (item 2).
3. `CLAUDE.md` and the engine MCP setup, so every later slice starts well.
4. Profile-pull in `PushPull` (item 4), plus Rotate and Tape logic (goal change 3).
5. Picking, then inference (item 3).
6. Phase 3 gestures, undo grouping (item 6).
7. Full art layer (item 5), then the rest of PLAN.md; curved tools only after v1.

## Checks to keep running

- `cargo test`
- `scripts/check-command-ids.sh` (add each new command id to `ENGINE_COMMANDS` first)
- `cargo test --manifest-path crates/engine-check/Cargo.toml`
- From Phase 1: scripted UI scenarios and screenshots, so a look change shows up as a picture.

## Sources

- SolidCraft `README.md`, `docs/mcp.md`, `AGENTS.md`, `ROADMAP.md`, `skills/` (pinned commit).
- [Claude Code memory and CLAUDE.md](https://code.claude.com/docs/en/memory)
- [Claude Code subagents](https://code.claude.com/docs/en/sub-agents)
