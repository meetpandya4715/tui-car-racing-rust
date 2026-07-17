# ASCII Apex Visual Polish Plan

## Status and intent

This plan defines the visual rewrite for a `0.2.0`-level release of ASCII Apex.
It is intentionally ambitious: the goal is not to decorate the existing ASCII
scene, but to replace its icon-like art language with coherent terminal pixel
art while preserving the game's responsiveness, portability, and playability.

Implementation should begin only after the current `0.1.2` speed/nitro tuning
changes are committed or otherwise isolated. Visual work must not silently mix
with gameplay balancing changes.

## North star

ASCII Apex should look like a grounded, orthographic top-down night-highway
diorama:

- Cars read as machines with body mass, glass, lights, tires, and shadows.
- Buildings are shown as rooftops and lots, not front-facing house icons.
- Trees read as top-down canopies with trunks and cast shadows, not triangles.
- Asphalt, lane paint, shoulders, sidewalks, foliage, roofs, and glass have
  distinct material treatments.
- Amber streetlights, red taillights, cool headlights, wet reflections, and
  controlled darkness create depth without reducing gameplay clarity.
- Motion feels fast through world-anchored texture, light streaks, and restrained
  effects rather than camera shake or full-screen noise.

The target is "terminal pixel art with believable materials," not photorealism
and not decorative ASCII clip art.

## Non-negotiable constraints

1. **Terminal native:** continue to ship one Rust executable using Crossterm.
   Do not require images, browser rendering, Kitty graphics, or external assets.
2. **No zoom masquerading as resolution:** cars, lanes, and world geometry retain
   a fixed terminal-cell footprint. Larger terminals reveal more world, more
   environmental layers, and more detail density.
3. **Minimum playfield:** `60x24` remains fully playable and readable.
4. **Reference layouts:** `80x30` is the primary art-review size; `120x40` is the
   showcase size.
5. **Stable world:** visual placement is deterministic in world space. Nothing
   shimmers, re-rolls, or jumps when a frame is redrawn.
6. **Gameplay clarity first:** the player car, traffic, road edges, gaps, and
   hazards remain immediately legible during boost.
7. **No steady-frame clears:** resize/startup may clear; ordinary frames must use
   differential output wrapped in synchronized terminal updates.
8. **Portable fallback:** enhanced Unicode/truecolor art has an ASCII-safe visual
   profile. Unsupported terminals must remain playable.
9. **Collision honesty:** a visible solid vehicle cell must not sit outside the
   gameplay footprint in a way that makes collisions feel incorrect.

## Current-state diagnosis

The current renderer has several structural limits that should be addressed
before adding more art:

- Vehicles are four fixed `7x5` ASCII silhouettes.
- Roadside objects are fixed `6x4` ASCII icons selected from short repeating
  sequences.
- Cars use a top-down view while buildings and trees are mostly front-facing.
- Materials are represented by one semantic foreground color rather than shape,
  lighting, surface texture, and shadow.
- Distant scenery is primarily the same sprite with a dimmer color.
- The framebuffer is rebuilt and every row is serialized every render, even when
  most cells are unchanged.
- Rendering, palettes, sprites, scenery selection, effects, overlays, and output
  serialization live in one large module, making visual iteration risky.

Adding more tiny icons to this system would increase clutter and repetition
without producing realism.

## Target rendering architecture

### 1. Layered scene composition

Use explicit layers with deterministic overwrite rules:

1. canvas and terrain base
2. asphalt and roadside ground materials
3. road paint, shoulders, curbs, reflectors, and puddles
4. environmental shadows
5. distant scenery
6. near scenery and roadside props
7. traffic shadows
8. traffic vehicles
9. player shadow and player vehicle
10. transient effects
11. telemetry rails
12. pause, crash, and game-over overlays

Each cell should carry a glyph, foreground, background, attributes, and layer
priority. Layer ownership makes occlusion testable and prevents scenery/effects
from accidentally overwriting road, traffic, or HUD regions.

### 2. Sub-cell terminal pixel art

Keep the existing gameplay footprint but increase effective art resolution with
a controlled width-one Unicode glyph set:

- Use half-block and quadrant glyphs to represent two vertical or `2x2` logical
  pixels inside one terminal cell.
- Limit each cell to a foreground/background pair so output remains compatible
  with ordinary terminal color commands.
- Prefer half blocks for large material areas and quadrant blocks for silhouettes
  and edges.
- Use Braille only for sparse one-color details, never for primary filled forms.
- Maintain an explicit ASCII fallback sprite for every gameplay-critical object.
- Keep a tested glyph whitelist; do not accept arbitrary Unicode whose terminal
  width may vary.

This can make a `7x5` car read closer to a `14x10` pixel sprite without enlarging
its collision footprint or zooming the game.

### 3. Double buffering and differential output

Before visual density increases:

- Maintain current and previous cell buffers.
- Compare frames and serialize only changed horizontal runs.
- Skip all output for an identical frame.
- Reset the previous buffer on resize, profile change, or alternate-screen entry.
- Wrap writes in synchronized-update begin/end commands when supported.
- Preserve full-frame serialization as a tested fallback/debug mode.
- Add counters for composed cells, changed cells, output bytes, and render time.

This phase is a prerequisite for weather, lighting, and denser scenery.

### 4. Visual profiles and level of detail

Define two visual profiles and three layout tiers:

**Enhanced profile**

- truecolor
- half/quadrant block glyphs
- material shading
- layered shadows and light pools
- optional atmospheric effects

**ASCII profile**

- existing width-one ASCII character discipline
- reduced palette assumptions
- simplified but perspective-consistent silhouettes
- identical gameplay visibility

**Compact `60x24`**

- one near scenery layer per side
- simplified shadows and road texture
- limited atmosphere
- no loss of HUD or reaction space

**Standard `80x30`**

- full vehicle art
- near and middle environmental detail
- streetlight pools and richer ground materials

**Wide `120x40+`**

- additional world columns and distant environmental layers
- more building lots, vegetation clusters, and parked props
- never larger cars or wider lane geometry merely because space is available

Expose an explicit `--visual-mode auto|enhanced|ascii` option so art problems can
be diagnosed without relying on fragile terminal probing.

## Art direction specifications

### Vehicles

- Preserve the player's unmistakable red identity.
- Redesign sedan, sports car, and truck in one consistent top-down projection.
- Define body, roof/glass, tires, headlight, taillight, trim, and shadow palette
  roles rather than coloring characters by ad hoc symbol rules.
- Give every traffic class at least four deterministic visual variants: body
  color, roof shape, lamp arrangement, and minor trim.
- Ensure classes remain identifiable in monochrome silhouette tests.
- Add a one-cell soft shadow offset consistently from the scene light direction.
- At high speed, emphasize wheel/road contact and light streaks; do not blur the
  collision silhouette.
- Nitro should add a layered cool exhaust plume and subtle reflected light on
  nearby asphalt, not only three punctuation marks.

### Road and shoulders

- Replace uniform grain with low-contrast, world-anchored asphalt variation.
- Add repaired seams, occasional cracks, darker tire lanes, and sparse wet areas.
- Give lane paint thickness, chipped edges, and raised reflector highlights.
- Add a readable shoulder transition: paint, rumble strip, drainage edge, curb or
  guardrail depending on the environmental zone.
- Keep texture below traffic contrast; no road detail may resemble a vehicle.
- Use speed-dependent motion cues sparingly and anchor them to world coordinates.

### Scenery and environmental zones

Replace short repeating arrays with deterministic world-generated zones. Initial
zone set:

1. **Forest edge:** layered canopies, trunks, undergrowth, rocks, drainage ditch.
2. **Residential outskirts:** top-down roofs, yards, fences, driveways, parked cars.
3. **Commercial strip:** flat roofs, loading areas, signs, parking-lot markings.
4. **Industrial corridor:** warehouses, tanks, pipes, service roads, utility poles.
5. **Roadside service:** fuel stop, diner/rest area, illuminated sign, parked trucks.
6. **Open highway:** guardrails, fields, sparse trees, power lines, distant lights.

Each zone should provide:

- at least six major footprints
- at least four variants per common prop
- near, middle, and distant representations
- deterministic placement and palette variation from world coordinates
- spacing/footprint rules preventing overlap and obvious grids
- a transition band so zones blend rather than switch on one row

All architecture and vegetation must use the same top-down projection as the
vehicles. Houses should communicate through roofs, chimneys, paths, yards, and
shadows—not facade windows and triangular front elevations.

### Lighting and atmosphere

- Establish one scene-wide moon/key-light direction for all shadows.
- Add streetlight pools with soft stepped falloff and restrained amber reflection.
- Add player headlights as a subtle forward visibility cone and traffic taillight
  reflections that never hide lane edges.
- Support a wet-road treatment using sparse mirrored highlights and darker
  materials before adding active rain.
- Add optional rain streaks, mist, and roadside haze only after differential
  rendering meets performance targets.
- Change atmosphere by zone and progression, not through per-frame randomness.
- Keep weather intensity bounded and offer an effects reduction option if needed.

### HUD, title, and feedback

- Keep telemetry outside the road and preserve the current information hierarchy.
- Replace long uniform rules with a compact instrument-panel language using
  restrained separators and status colors.
- Reserve cyan for active telemetry/boost, red for player/danger, amber for lamps
  and warnings, and neutral tones for world materials.
- Rework the title screen into a composed roadside/night-highway scene using the
  same world renderer rather than a separate decorative language.
- Improve pass, near-miss, level-up, and crash feedback with spatial effects and
  short-lived light/material responses instead of more HUD text.

## Proposed module layout

Split visual responsibilities while keeping simulation independent:

```text
src/
  render/
    mod.rs          scene orchestration and public Renderer API
    frame.rs        cells, layers, buffers, clipping, dirty-run generation
    glyphs.rs       sub-cell glyph encoder and ASCII fallback mapping
    palette.rs      semantic palette roles and visual profiles
    road.rs         asphalt, paint, shoulders, reflectors, puddles
    vehicles.rs     vehicle silhouettes, variants, shadows, lights
    scenery.rs      zones, footprints, props, deterministic placement
    lighting.rs     shadows, lamps, headlights, reflections
    effects.rs      nitro, rain, sparks, collision, event feedback
    hud.rs          title, telemetry, pause, and game-over presentation
```

`renderer.rs` should migrate into this structure behind the same public
`Renderer::render` boundary. `game.rs` must remain unaware of glyphs and colors;
it should expose only immutable state needed by visuals. Gameplay geometry stays
in `entities.rs`, while visual footprints and transparent margins live in the
render modules.

No new runtime dependency is required for the first implementation. Use a tested
width-one glyph whitelist instead of adding a Unicode-width dependency unless a
later compatibility test demonstrates a real need.

## Phased implementation plan

### Phase 0 - Baseline, art bible, and review gates

Estimated focused effort: 1-2 days.

- Commit/isolate the current `0.1.2` tuning work.
- Capture deterministic baseline frames at `60x24`, `80x30`, and `120x40` for
  title, early race, traffic-heavy race, nitro, pause, and crash states.
- Record full-frame output bytes and render/serialization time for those scenes.
- Generate two or three raster art-direction concepts with `$imagegen` as
  reference-only mood boards; do not ship them or convert them directly to assets.
- Select one concept and produce a short art bible covering projection, palette,
  materials, lighting direction, density, and forbidden motifs.
- Approve a representative player car, traffic car, tree canopy, house roof, and
  road material study before production art begins.

**Gate A:** approve the art bible and reference frame. No large sprite production
before this gate.

### Phase 1 - Renderer foundation

Estimated focused effort: 2-3 days.

- Split renderer modules behind the existing API.
- Add explicit layer priorities and clipping regions.
- Add previous/current framebuffers and dirty-run serialization.
- Add synchronized terminal updates and fallback behavior.
- Add enhanced/ASCII visual profiles and the CLI selector.
- Add render statistics and deterministic snapshot helpers.

**Gate B:** an unchanged frame produces negligible output; resize/full repaint and
terminal restoration remain correct; all current visuals still render.

### Phase 2 - Sub-cell art and vehicle production

Estimated focused effort: 2-3 days.

- Implement the half/quadrant glyph encoder and glyph-width tests.
- Produce the player, sedan, sports, and truck master silhouettes.
- Add deterministic color/trim variants.
- Add vehicle shadows, glass, lights, and improved nitro exhaust.
- Confirm visual solids and collision footprints remain honest.

**Gate C:** every vehicle class is identifiable in color and monochrome at
`60x24`, and the player remains instantly distinct during boost and dense traffic.

### Phase 3 - Road material pass

Estimated focused effort: 1-2 days.

- Build world-anchored asphalt, wear, repair, paint, reflector, shoulder, curb,
  drainage, and guardrail systems.
- Add wet-road material response and vehicle/light reflections.
- Tune contrast at normal and nitro speeds.
- Verify that no texture creates false hazards or lane ambiguity.

**Gate D:** the road reads as layered material rather than a flat field while
traffic remains the highest-contrast moving content.

### Phase 4 - Environmental zone system

Estimated focused effort: 3-5 days.

- Implement deterministic zone selection and blended transitions.
- Implement footprint-aware placement with near/middle/distant layers.
- Produce forest, residential, commercial, industrial, service, and open-highway
  asset families.
- Add lots, paths, fences, parked vehicles, utilities, vegetation clusters, and
  zone-specific shoulders.
- Add variation constraints that prevent obvious same-screen repetition.

**Gate E:** screenshots from three seeds and three sizes show coherent but visibly
different worlds, with no front-facing scenery or obvious seven-row icon loops.

### Phase 5 - Lighting, weather, and motion polish

Estimated focused effort: 2-3 days.

- Add moon/key-light shadows, streetlight pools, headlights, taillights, and
  restrained reflections.
- Add optional mist/rain after performance validation.
- Redesign nitro, speed, near-miss, level-up, and collision feedback.
- Ensure effects are deterministic or time-driven and do not shimmer.

**Gate F:** atmosphere is materially richer, but lane edges, traffic silhouettes,
and HUD remain readable in the busiest supported scene.

### Phase 6 - HUD and presentation polish

Estimated focused effort: 1-2 days.

- Align title, gameplay HUD, pause, and game-over screens with the art bible.
- Reduce decorative rules and improve instrument grouping.
- Add compact/wide layout snapshots.
- Keep all controls and critical telemetry visible at the minimum size.

### Phase 7 - Optimization, compatibility, and release hardening

Estimated focused effort: 2-3 days.

- Profile composition, diffing, serialization, and output bytes.
- Test enhanced and ASCII profiles across representative terminals.
- Run resize, focus-loss, panic-restoration, and long-session tests.
- Run deterministic snapshot, unit, release, musl, npm, and PTY verification.
- Publish as a `0.2.0` visual release only after all acceptance criteria pass.

Total estimated focused effort: approximately 13-20 engineering days, depending
on the number of approved scenery variants and weather scope.

## Acceptance criteria

### Visual coherence

- Cars, buildings, trees, props, and road materials share one orthographic
  top-down projection.
- Each vehicle class is recognizable from silhouette alone.
- The player is identifiable within 100 ms in every tested scene.
- No front-facing triangular house, facade-window building, or triangular tree
  icon remains in the enhanced profile.
- No identical major near-scenery footprint repeats within 40 visible world rows
  on the same side of the road.
- Distant scenery has a distinct representation, not only a dimmer near sprite.
- Three deterministic seeds produce clearly different but art-directed scenes.

### Gameplay and layout

- `60x24`, `80x30`, and `120x40` layouts preserve road bounds, HUD, traffic entry,
  steering gaps, and collision fairness.
- Larger terminals reveal more world/detail without scaling car or lane geometry.
- Visual solids remain aligned with collision footprints.
- Road texture, weather, shadows, and reflections never resemble hazards.
- Player, traffic, and lane boundaries remain readable during full nitro.

### Stability and performance

- Same seed, state, size, and visual profile produce identical cells.
- Re-rendering an unchanged frame emits no cell payload.
- Steady gameplay never uses a full-screen clear.
- At `120x40`, p95 scene composition plus diff generation stays below 8 ms on
  the reference machine.
- Typical active-frame output stays below 40% of equivalent full-frame bytes;
  heavy nitro/weather scenes stay below 65%.
- The game sustains its 30 FPS render target without simulation catch-up caused
  by rendering on the reference machine.
- Resize performs one clean full repaint and returns to differential output.
- Terminal state is restored after normal quit, error, and panic paths.

### Compatibility

- Every enhanced glyph is verified as width one in the supported test terminals.
- ASCII mode contains only width-one ASCII and preserves gameplay readability.
- Truecolor, reduced-color, and no-bold behavior degrade intentionally.
- Linux musl, Windows x64, macOS x64, and macOS arm64 release builds remain valid.

## Test and review matrix

For every visual gate, review deterministic captures across:

| Dimension | Required cases |
| --- | --- |
| Size | `60x24`, `80x30`, `120x40` |
| Profile | enhanced, ASCII |
| State | title, early race, dense race, nitro, pause, crash/game over |
| Zone | all six zone families |
| Motion | stopped snapshot, normal speed, maximum speed, full nitro |
| Terminal | Ghostty plus at least one mainstream Linux terminal; Windows and macOS release smoke tests |

Automated coverage should include:

- glyph whitelist and width assumptions
- layer/occlusion priority
- deterministic scenery placement and zone transitions
- no scenery inside road/HUD clipping regions
- sprite footprint versus collision bounds
- unchanged-frame zero-payload behavior
- dirty-run merging and last-column repaint
- full repaint after resize/profile switch
- compact/standard/wide snapshot anchors
- stable output at animation wrap boundaries
- ASCII fallback parity for critical gameplay information

## Delivery strategy

- Develop on a dedicated visual branch after the `0.1.2` tuning work is isolated.
- Keep the legacy renderer available behind a temporary debug switch until Phase
  3 passes; do not maintain it after `0.2.0` ships.
- Commit by subsystem and gate: foundation, vehicles, road, each zone family,
  lighting/effects, HUD, hardening.
- Require a screenshot/capture set and performance delta in every visual PR.
- Avoid one giant renderer rewrite with no intermediate playable state.
- Do not publish `0.2.0` from concept approval alone; all acceptance criteria and
  release-platform checks must pass.

## Recommended first execution slice

The first implementation slice should stop after a vertical proof of quality:

1. baseline captures and render metrics
2. approved image-generated art-direction reference
3. differential renderer plus synchronized updates
4. enhanced glyph encoder with ASCII fallback
5. one finished player car, one traffic car, one top-down tree canopy, one house
   roof/yard footprint, and one finished road-material strip
6. deterministic `80x30` reference scene and `60x24` readability check

If that slice does not create a clear visual leap, revise the art bible and
renderer assumptions before producing the full environment library.
