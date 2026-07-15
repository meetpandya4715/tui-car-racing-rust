# Terminal Rush

Terminal Rush is an endless, top-down night-highway racer written in Rust. Its
near-black endurance-racing interface, responsive telemetry rails, realistic
7-by-5 line-art cars, and animated city shoulders are rendered directly in the
terminal with Crossterm. The opening is a forgiving warm-up; traffic density,
speed variation, and reaction pressure then build smoothly toward full highway
mayhem.

## Gameplay demo

<video src="./Terminal%20Rush%20-%20minimal.mp4" controls width="800"></video>

[Open the minimal gameplay demo](./Terminal%20Rush%20-%20minimal.mp4)

```text
 TERMINAL RUSH  180 KM/H   SCORE 0000108            P PAUSE
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
  tiers from `ROOKIE` through `MAYHEM`. Spawn pressure, traffic capacity, speed
  variation, and reaction requirements all scale gradually.
- Every safe pass starts or extends a 4.5-second streak, multiplying pass
  points up to `x5`.
- Passing with only one or two cells of clearance earns a near-miss bonus and
  refills nitro.
- Nitro starts half full. Hold Space for speeds above the normal limit; release
  Space to recharge it.

## Prerequisites

- A stable Rust toolchain (`rustup` is the usual installer)
- A terminal at least 60 columns by 24 rows
- A terminal with ANSI color support is recommended; the red player, cyan live
  data, amber warnings, and muted city remain readable from the ASCII shapes
  alone

## Build and run

```sh
cargo run --release
```

To build without starting the game:

```sh
cargo build --release
```

## Tests and linting

```sh
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

A non-interactive startup/render check is also available:

```sh
cargo run --release -- --smoke-test
```

## Architecture

- `main.rs` coordinates the fixed-step simulation, event polling, resizing, and
  render cadence.
- `terminal.rs` owns raw mode, alternate-screen entry, cursor visibility, and
  best-effort restoration on errors and panics.
- `input.rs` converts Crossterm key events into continuous controls and
  edge-triggered game actions.
- `game.rs` contains the terminal-independent state machine, scoring,
  difficulty, traffic generation, and simulation.
- `entities.rs` defines the responsive 40/48/56-cell road, cars, and predictable
  7-by-5 ASCII sprites.
- `collision.rs` provides half-open rectangular hitboxes.
- `renderer.rs` composes each frame in memory and flushes it to the terminal in
  one buffered write. It also builds seamless, parallax night scenery from
  houses, shops, gardens, spectators, trees, parks, signs, lamps, and sidewalks.

The game uses a 60 Hz fixed simulation step with clamped catch-up time and a
30 FPS render target. Resizing below the minimum freezes simulation and shows a
warning; restoring a valid size safely rebuilds the road layout.
