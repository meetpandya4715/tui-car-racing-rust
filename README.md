# ASCII Apex

ASCII Apex is an endless, top-down night-highway racer rendered directly in
your terminal. Thread through traffic, build scoring streaks, skim past cars
for near-miss bonuses, and burn nitro as the highway accelerates from a gentle
opening into full arcade mayhem.

## Play instantly

```sh
npx ascii-apex@latest
```

No Rust toolchain is required. The npm launcher downloads the matching release
artifact from this repository on first run, verifies its SHA-256 hash, and
caches it locally. Node.js 18 or newer is required.

Prebuilt releases support:

- Windows x64
- Linux x64 (statically linked for broad distribution compatibility)
- macOS x64
- macOS Apple silicon

Your terminal should be at least 60 columns by 24 rows. ANSI color support is
recommended.

## Gameplay demo

<video src="https://raw.githubusercontent.com/meetpandya4715/tui-car-racing-rust/main/Terminal%20Rush%20-%20minimal.mp4" controls width="800"></video>

[Open the minimal gameplay demo](https://github.com/meetpandya4715/tui-car-racing-rust/blob/main/Terminal%20Rush%20-%20minimal.mp4)

```text
 ASCII APEX  180 KM/H   SCORE 0000108               P PAUSE
 N2O 068 [|||||||---]  STREAK x00  L01 ROOKIE  D 00108m
------------------------------------------------------------
         |          |         |  +-----+|         |
        o|                  .    |o___o|          |o
        ||           .           O|---|O         .||
 .2KM .  |          |         |  O|===|O|         |
 '----'  |          |       . |  |v___v||         |
   ||    |          |.        |         |        .|
         |    .     |         |         | .       |   . * .
         |                         .              |   .|. .
  /\     |    .     |         |         | .       |
 /  \    |          |     /--^--\  .    |         |
  ||    o|          |     /o_|_o\       |         |o
  ..    ||          |.    |/---\|       |        .||
        ||    .           O\_|_/O         .       ||
       . |                '\v*v/'  .              |
------------------------------------------------------------
 T 00:02.9  D 00108m  HI 0000108  P 000  N 00  B x00  L 01
```

The red player is always visually distinct from restrained white/gray traffic.
Cyan is reserved for live telemetry and glass, amber for lamps and escalation,
and the roadside uses sparse warm city and garden tones. Transient clean-pass,
near-miss, and level-up callouts appear in the road rather than crowding the
dashboard.

## Controls

| Action | Keys |
| --- | --- |
| Steer left | Left Arrow or `A` |
| Steer right | Right Arrow or `D` |
| Accelerate | Up Arrow or `W` |
| Brake | Down Arrow or `S` |
| Nitro boost | Hold Space |
| Pause / resume | `P` |
| Start / restart | Enter |
| Restart after a crash | `R` |
| Quit | `Q`, Escape, or Ctrl+C |

Movement is time-based rather than key-repeat-based. Terminals that report key
release events get true held-key input; a short key lease prevents stuck
movement on terminals that only report presses and repeats.

## Scoring and progression

- The first 2.5 seconds are traffic-free, level 1 permits only two cars, and
  early traffic moves in a narrow, readable speed range.
- Difficulty rises continuously across ten levels, grouped into five named
  tiers from `ROOKIE` through `MAYHEM`.
- Every safe pass starts or extends a 4.5-second streak, multiplying pass
  points up to `x5`.
- Passing with only one or two cells of clearance earns a near-miss bonus and
  refills nitro.
- Nitro starts half full. Hold Space for speeds above the normal limit; release
  Space to recharge it.

## Build from source

Install a stable Rust toolchain, then run:

```sh
cargo run --release
```

To validate a local checkout:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run --release -- --smoke-test
npm test
npm pack --dry-run
```

## Distribution

The `ascii-apex` npm package is a dependency-free Node.js launcher. For each
tagged version, GitHub Actions builds the native Rust executable for every
supported platform and publishes the executables plus `SHA256SUMS` in a GitHub
release. The launcher downloads only the asset matching the current operating
system and CPU architecture.

## Architecture

- `main.rs` coordinates the fixed-step simulation, event polling, resizing, and
  render cadence.
- `terminal.rs` owns raw mode, alternate-screen entry, cursor visibility, and
  best-effort restoration on errors and panics.
- `input.rs` converts Crossterm key events into continuous controls and
  edge-triggered game actions.
- `game.rs` contains the terminal-independent state machine, scoring,
  difficulty, traffic generation, and simulation.
- `entities.rs` defines the responsive road, cars, and predictable 7-by-5 ASCII
  sprites.
- `collision.rs` provides half-open rectangular hitboxes.
- `renderer.rs` composes each frame in memory and flushes it to the terminal in
  one buffered write.

The game uses a 60 Hz fixed simulation step with clamped catch-up time and a
30 FPS render target. Resizing below the minimum freezes simulation and shows a
warning; restoring a valid size safely rebuilds the road layout.
