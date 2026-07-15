mod collision;
mod entities;
mod game;
mod input;
mod renderer;
mod terminal;

use std::{
    env, io, panic,
    process::ExitCode,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event};

use crate::{
    game::{ControlInput, Game, GameAction, GameState},
    input::InputController,
    renderer::{MIN_HEIGHT, MIN_WIDTH, Renderer},
    terminal::TerminalSession,
};

const FIXED_STEP: Duration = Duration::from_nanos(16_666_667);
const RENDER_INTERVAL: Duration = Duration::from_nanos(33_333_333);
const MAX_FRAME_DELTA: Duration = Duration::from_millis(250);
const MAX_CATCH_UP_STEPS: usize = 8;
const MAX_EVENTS_PER_BATCH: usize = 128;

fn main() -> ExitCode {
    install_panic_cleanup();

    let result = if env::args().any(|argument| argument == "--smoke-test") {
        run_smoke_test()
    } else {
        run_interactive()
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("terminal-rush: {error}");
            ExitCode::FAILURE
        }
    }
}

fn install_panic_cleanup() {
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        terminal::restore_terminal_best_effort();
        previous_hook(panic_info);
    }));
}

fn run_interactive() -> io::Result<()> {
    let mut terminal = TerminalSession::enter()?;
    let game_result = run_game_loop(&mut terminal);
    let restore_result = terminal.restore();

    match game_result {
        Err(error) => Err(error),
        Ok(()) => restore_result,
    }
}

fn run_game_loop(terminal: &mut TerminalSession) -> io::Result<()> {
    let initial_size = crossterm::terminal::size()?;
    let initial_game_width = initial_size.0.max(MIN_WIDTH);
    let initial_game_height = initial_size.1.max(MIN_HEIGHT);

    let mut game = Game::new(initial_game_width, initial_game_height);
    let mut input = InputController::new_with_release_events(terminal.reliable_key_releases());
    let mut renderer = Renderer::new();
    let mut terminal_size = initial_size;
    let mut accumulator = Duration::ZERO;
    let mut last_update = Instant::now();
    let mut next_render = last_update;

    loop {
        let now = Instant::now();
        if now < next_render {
            let wait = next_render.saturating_duration_since(now);
            if event::poll(wait)? {
                if process_event_batch(&mut game, &mut input)? {
                    accumulator = Duration::ZERO;
                    last_update = Instant::now();
                }
                if game.state() == GameState::Quit {
                    break;
                }
                continue;
            }
        }

        let frame_time = Instant::now();
        let observed_size = crossterm::terminal::size()?;
        if observed_size != terminal_size {
            terminal_size = observed_size;
            terminal.clear()?;
            input.clear_motion();
            accumulator = Duration::ZERO;
            last_update = frame_time;

            if terminal_size.0 >= MIN_WIDTH && terminal_size.1 >= MIN_HEIGHT {
                game.resize(terminal_size.0, terminal_size.1);
            }
        }

        let large_enough = terminal_size.0 >= MIN_WIDTH && terminal_size.1 >= MIN_HEIGHT;
        let frame_delta = frame_time
            .saturating_duration_since(last_update)
            .min(MAX_FRAME_DELTA);
        last_update = frame_time;

        if large_enough && game.state() == GameState::Playing {
            accumulator = accumulator.saturating_add(frame_delta);
            let controls = input.snapshot();
            let mut steps = 0;

            while accumulator >= FIXED_STEP && steps < MAX_CATCH_UP_STEPS {
                game.update(FIXED_STEP.as_secs_f32(), controls);
                accumulator = accumulator.saturating_sub(FIXED_STEP);
                steps += 1;
            }

            if steps == MAX_CATCH_UP_STEPS && accumulator >= FIXED_STEP {
                accumulator = Duration::ZERO;
            }

            if game.state() != GameState::Playing {
                input.clear_motion();
                accumulator = Duration::ZERO;
            }
        } else {
            accumulator = Duration::ZERO;
        }

        let frame = renderer.render(&game, terminal_size.0, terminal_size.1)?;
        terminal.write_frame(frame)?;

        if game.state() == GameState::Quit {
            break;
        }

        next_render = frame_time + RENDER_INTERVAL;
    }

    Ok(())
}

fn process_event_batch(game: &mut Game, input: &mut InputController) -> io::Result<bool> {
    let first_event = event::read()?;
    let mut state_changed = process_event(first_event, game, input);

    for _ in 1..MAX_EVENTS_PER_BATCH {
        if !event::poll(Duration::ZERO)? {
            break;
        }
        state_changed |= process_event(event::read()?, game, input);
        if game.state() == GameState::Quit {
            break;
        }
    }

    Ok(state_changed)
}

fn process_event(event: Event, game: &mut Game, input: &mut InputController) -> bool {
    match event {
        Event::Key(key_event) => {
            if let Some(action) = input.handle_key(key_event) {
                let state_before = game.state();
                game.handle_action(action);
                if game.state() != state_before || game.state() != GameState::Playing {
                    input.clear_motion();
                }
                return game.state() != state_before;
            }
        }
        Event::FocusLost => input.clear_all(),
        Event::Resize(_, _) | Event::FocusGained | Event::Mouse(_) | Event::Paste(_) => {}
    }
    false
}

fn run_smoke_test() -> io::Result<()> {
    let mut game = Game::new_seeded(80, 30, 0x5eed);
    game.handle_action(GameAction::Start);

    let starting_nitro = game.nitro();
    let controls = ControlInput {
        accelerate: true,
        boost: true,
        ..ControlInput::default()
    };
    for _ in 0..60 {
        game.update(FIXED_STEP.as_secs_f32(), controls);
    }

    let mut renderer = Renderer::new();
    let frame = renderer.render(&game, 80, 30)?;
    if frame.is_empty()
        || game.distance() <= 0.0
        || !game.boosting()
        || game.nitro() >= starting_nitro
    {
        return Err(io::Error::other(
            "simulation, nitro, or buffered rendering did not advance",
        ));
    }

    println!(
        "smoke check passed: state={:?}, distance={:.1}m, score={}, nitro={:.0}%, frame={} bytes",
        game.state(),
        game.distance(),
        game.score(),
        game.nitro(),
        frame.len()
    );
    Ok(())
}
