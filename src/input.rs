use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::game::{ControlInput, GameAction};

// The first lease bridges the operating system's initial key-repeat delay.
// Once another key-down arrives, the shorter repeat lease gives legacy
// terminals a prompt stop without making held movement pulse.
const INITIAL_MOTION_LEASE: Duration = Duration::from_millis(650);
const REPEAT_MOTION_LEASE: Duration = Duration::from_millis(180);
const EDGE_DEBOUNCE: Duration = Duration::from_millis(700);
const MOTION_BINDING_COUNT: usize = 9;

/// Converts terminal key events into stable, frame-rate-independent controls.
///
/// Motion keys use short leases. Press and repeat events refresh a lease and a
/// release event clears it immediately. The lease is the fallback for terminals
/// that do not report key releases, preventing a movement key from sticking.
pub struct InputController {
    reliable_release_events: bool,
    motion_pressed: [bool; MOTION_BINDING_COUNT],
    motion_deadlines: [Option<Instant>; MOTION_BINDING_COUNT],
    edge_pressed: [bool; EDGE_BINDING_COUNT],
    edge_last_seen: [Option<Instant>; EDGE_BINDING_COUNT],
}

impl InputController {
    pub fn new() -> Self {
        // Crossterm's Windows backend reports explicit key-up records. Unix
        // terminals without keyboard enhancement use the lease fallback.
        Self::with_release_events(cfg!(windows))
    }

    pub fn new_with_release_events(reliable_release_events: bool) -> Self {
        Self::with_release_events(reliable_release_events)
    }

    /// Processes one keyboard event and returns an edge-triggered game action,
    /// if the event represents one.
    pub fn handle_key(&mut self, event: KeyEvent) -> Option<GameAction> {
        self.handle_key_at(event, Instant::now())
    }

    /// Returns the currently active continuous controls.
    pub fn snapshot(&self) -> ControlInput {
        self.snapshot_at(Instant::now())
    }

    /// Clears every continuous control, for example when pausing or restarting.
    pub fn clear_motion(&mut self) {
        self.motion_pressed.fill(false);
        self.motion_deadlines.fill(None);
    }

    /// Clears both continuous and edge-triggered key state after focus loss.
    pub fn clear_all(&mut self) {
        self.clear_motion();
        self.edge_pressed.fill(false);
        self.edge_last_seen.fill(None);
    }

    fn with_release_events(reliable_release_events: bool) -> Self {
        Self {
            reliable_release_events,
            motion_pressed: [false; MOTION_BINDING_COUNT],
            motion_deadlines: [None; MOTION_BINDING_COUNT],
            edge_pressed: [false; EDGE_BINDING_COUNT],
            edge_last_seen: [None; EDGE_BINDING_COUNT],
        }
    }

    fn handle_key_at(&mut self, event: KeyEvent, now: Instant) -> Option<GameAction> {
        if event.kind == KeyEventKind::Release && !self.reliable_release_events {
            for (pressed, deadline) in self
                .motion_pressed
                .iter_mut()
                .zip(self.motion_deadlines.iter())
            {
                *pressed = deadline.is_some_and(|value| value > now);
            }
            self.reliable_release_events = true;
        }

        if is_control_c(event) {
            return self.handle_edge(EdgeBinding::ControlC, GameAction::Quit, event.kind, now);
        }

        if let Some(binding) = MotionBinding::from_code(event.code) {
            self.handle_motion(binding, event.kind, now);
            return None;
        }

        let (binding, action) = match event.code {
            KeyCode::Enter => (EdgeBinding::Enter, GameAction::Start),
            KeyCode::Esc => (EdgeBinding::Escape, GameAction::Quit),
            KeyCode::Char(character) => match character.to_ascii_lowercase() {
                'p' => (EdgeBinding::Pause, GameAction::PauseToggle),
                'r' => (EdgeBinding::Restart, GameAction::Restart),
                'q' => (EdgeBinding::Quit, GameAction::Quit),
                _ => return None,
            },
            _ => return None,
        };

        self.handle_edge(binding, action, event.kind, now)
    }

    fn snapshot_at(&self, now: Instant) -> ControlInput {
        ControlInput {
            left: self.any_active([MotionBinding::LeftArrow, MotionBinding::A], now),
            right: self.any_active([MotionBinding::RightArrow, MotionBinding::D], now),
            accelerate: self.any_active([MotionBinding::UpArrow, MotionBinding::W], now),
            brake: self.any_active([MotionBinding::DownArrow, MotionBinding::S], now),
            boost: self.any_active([MotionBinding::BoostSpace], now),
        }
    }

    fn handle_motion(&mut self, binding: MotionBinding, kind: KeyEventKind, now: Instant) {
        if self.reliable_release_events {
            self.motion_pressed[binding.index()] = kind != KeyEventKind::Release;
            return;
        }

        match kind {
            KeyEventKind::Press => {
                let deadline = &mut self.motion_deadlines[binding.index()];
                let lease = if deadline.is_some_and(|value| value > now) {
                    REPEAT_MOTION_LEASE
                } else {
                    INITIAL_MOTION_LEASE
                };
                *deadline = now.checked_add(lease);
            }
            KeyEventKind::Repeat => {
                self.motion_deadlines[binding.index()] = now.checked_add(REPEAT_MOTION_LEASE);
            }
            KeyEventKind::Release => {
                self.motion_deadlines[binding.index()] = None;
            }
        }
    }

    fn handle_edge(
        &mut self,
        binding: EdgeBinding,
        action: GameAction,
        kind: KeyEventKind,
        now: Instant,
    ) -> Option<GameAction> {
        if self.reliable_release_events {
            let pressed = &mut self.edge_pressed[binding.index()];
            return match kind {
                KeyEventKind::Press if !*pressed => {
                    *pressed = true;
                    Some(action)
                }
                KeyEventKind::Release => {
                    *pressed = false;
                    None
                }
                KeyEventKind::Press | KeyEventKind::Repeat => None,
            };
        }

        let last_seen = &mut self.edge_last_seen[binding.index()];
        match kind {
            KeyEventKind::Release => {
                *last_seen = None;
                None
            }
            KeyEventKind::Repeat => {
                if last_seen.is_some() {
                    *last_seen = Some(now);
                }
                None
            }
            KeyEventKind::Press => {
                if last_seen.is_some_and(|seen| now.saturating_duration_since(seen) < EDGE_DEBOUNCE)
                {
                    // Some terminals report auto-repeat as additional Press
                    // events. Refreshing this timestamp suppresses the whole
                    // repeat stream until the key has been quiet long enough.
                    *last_seen = Some(now);
                    return None;
                }

                *last_seen = Some(now);
                Some(action)
            }
        }
    }

    fn any_active<const N: usize>(&self, bindings: [MotionBinding; N], now: Instant) -> bool {
        bindings.into_iter().any(|binding| {
            if self.reliable_release_events {
                self.motion_pressed[binding.index()]
            } else {
                self.motion_deadlines[binding.index()].is_some_and(|deadline| deadline > now)
            }
        })
    }
}

impl Default for InputController {
    fn default() -> Self {
        Self::new()
    }
}

fn is_control_c(event: KeyEvent) -> bool {
    matches!(event.code, KeyCode::Char(character) if character.eq_ignore_ascii_case(&'c'))
        && event.modifiers.contains(KeyModifiers::CONTROL)
}

#[derive(Clone, Copy)]
enum MotionBinding {
    LeftArrow,
    A,
    RightArrow,
    D,
    UpArrow,
    W,
    DownArrow,
    S,
    BoostSpace,
}

impl MotionBinding {
    fn from_code(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Left => Some(Self::LeftArrow),
            KeyCode::Right => Some(Self::RightArrow),
            KeyCode::Up => Some(Self::UpArrow),
            KeyCode::Down => Some(Self::DownArrow),
            KeyCode::Char(' ') => Some(Self::BoostSpace),
            KeyCode::Char(character) => match character.to_ascii_lowercase() {
                'a' => Some(Self::A),
                'd' => Some(Self::D),
                'w' => Some(Self::W),
                's' => Some(Self::S),
                _ => None,
            },
            _ => None,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeBinding {
    Enter,
    Escape,
    Pause,
    Restart,
    Quit,
    ControlC,
}

const EDGE_BINDING_COUNT: usize = 6;

impl EdgeBinding {
    const fn index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
        KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind)
    }

    fn legacy_input() -> InputController {
        InputController::with_release_events(false)
    }

    #[test]
    fn motion_mapping_is_case_insensitive_and_expires() {
        let start = Instant::now();
        let mut input = legacy_input();

        assert!(
            input
                .handle_key_at(key(KeyCode::Char('A'), KeyEventKind::Press), start)
                .is_none()
        );
        assert!(input.snapshot_at(start + Duration::from_millis(500)).left);
        assert!(!input.snapshot_at(start + Duration::from_millis(651)).left);
    }

    #[test]
    fn repeat_refreshes_a_motion_lease() {
        let start = Instant::now();
        let mut input = legacy_input();
        input.handle_key_at(key(KeyCode::Up, KeyEventKind::Press), start);
        input.handle_key_at(
            key(KeyCode::Up, KeyEventKind::Repeat),
            start + Duration::from_millis(500),
        );

        assert!(
            input
                .snapshot_at(start + Duration::from_millis(650))
                .accelerate
        );
        assert!(
            !input
                .snapshot_at(start + Duration::from_millis(681))
                .accelerate
        );
    }

    #[test]
    fn releases_are_immediate_and_track_physical_bindings() {
        let start = Instant::now();
        let mut input = InputController::with_release_events(true);
        input.handle_key_at(key(KeyCode::Left, KeyEventKind::Press), start);
        input.handle_key_at(key(KeyCode::Char('a'), KeyEventKind::Press), start);
        input.handle_key_at(
            key(KeyCode::Char('a'), KeyEventKind::Release),
            start + Duration::from_millis(10),
        );

        assert!(input.snapshot_at(start + Duration::from_millis(20)).left);

        input.handle_key_at(
            key(KeyCode::Left, KeyEventKind::Release),
            start + Duration::from_millis(30),
        );
        assert!(!input.snapshot_at(start + Duration::from_millis(30)).left);
    }

    #[test]
    fn reliable_release_mode_keeps_motion_held_without_repeat() {
        let start = Instant::now();
        let mut input = InputController::with_release_events(true);
        input.handle_key_at(key(KeyCode::Right, KeyEventKind::Press), start);

        assert!(input.snapshot_at(start + Duration::from_secs(30)).right);
        input.handle_key_at(
            key(KeyCode::Right, KeyEventKind::Release),
            start + Duration::from_secs(31),
        );
        assert!(!input.snapshot_at(start + Duration::from_secs(31)).right);
    }

    #[test]
    fn space_holds_and_releases_nitro_boost() {
        let start = Instant::now();
        let mut input = InputController::with_release_events(true);
        input.handle_key_at(key(KeyCode::Char(' '), KeyEventKind::Press), start);
        assert!(input.snapshot_at(start).boost);

        input.handle_key_at(
            key(KeyCode::Char(' '), KeyEventKind::Release),
            start + Duration::from_millis(10),
        );
        assert!(!input.snapshot_at(start + Duration::from_millis(10)).boost);
    }

    #[test]
    fn edge_actions_ignore_repeat_and_debounce_legacy_press_streams() {
        let start = Instant::now();
        let mut input = legacy_input();

        assert!(matches!(
            input.handle_key_at(key(KeyCode::Char('p'), KeyEventKind::Press), start),
            Some(GameAction::PauseToggle)
        ));
        assert!(
            input
                .handle_key_at(
                    key(KeyCode::Char('p'), KeyEventKind::Repeat),
                    start + Duration::from_millis(20),
                )
                .is_none()
        );
        assert!(
            input
                .handle_key_at(
                    key(KeyCode::Char('p'), KeyEventKind::Press),
                    start + Duration::from_millis(30),
                )
                .is_none()
        );
        assert!(matches!(
            input.handle_key_at(
                key(KeyCode::Char('p'), KeyEventKind::Press),
                start + Duration::from_millis(800),
            ),
            Some(GameAction::PauseToggle)
        ));
    }

    #[test]
    fn legacy_edge_debounce_is_independent_for_each_binding() {
        let start = Instant::now();
        let mut input = legacy_input();

        assert_eq!(
            input.handle_key_at(key(KeyCode::Char('p'), KeyEventKind::Press), start),
            Some(GameAction::PauseToggle)
        );
        assert_eq!(
            input.handle_key_at(
                key(KeyCode::Enter, KeyEventKind::Press),
                start + Duration::from_millis(20),
            ),
            Some(GameAction::Start)
        );

        // A second P press in the debounce window is still the original
        // terminal auto-repeat stream, even though Enter arrived between it.
        assert_eq!(
            input.handle_key_at(
                key(KeyCode::Char('p'), KeyEventKind::Press),
                start + Duration::from_millis(30),
            ),
            None
        );
    }

    #[test]
    fn release_rearms_an_edge_action_without_waiting() {
        let start = Instant::now();
        let mut input = InputController::with_release_events(true);
        assert!(matches!(
            input.handle_key_at(key(KeyCode::Enter, KeyEventKind::Press), start),
            Some(GameAction::Start)
        ));
        input.handle_key_at(
            key(KeyCode::Enter, KeyEventKind::Release),
            start + Duration::from_millis(10),
        );
        assert!(matches!(
            input.handle_key_at(
                key(KeyCode::Enter, KeyEventKind::Press),
                start + Duration::from_millis(20),
            ),
            Some(GameAction::Start)
        ));
    }

    #[test]
    fn repeated_press_cannot_retrigger_an_edge_until_release() {
        let start = Instant::now();
        let mut input = InputController::with_release_events(true);
        let pause = key(KeyCode::Char('p'), KeyEventKind::Press);

        assert_eq!(
            input.handle_key_at(pause, start),
            Some(GameAction::PauseToggle)
        );
        assert_eq!(
            input.handle_key_at(pause, start + Duration::from_secs(5)),
            None
        );
        input.handle_key_at(
            key(KeyCode::Char('p'), KeyEventKind::Release),
            start + Duration::from_secs(6),
        );
        assert_eq!(
            input.handle_key_at(pause, start + Duration::from_secs(6)),
            Some(GameAction::PauseToggle)
        );
    }

    #[test]
    fn control_c_quits_and_clear_motion_releases_everything() {
        let start = Instant::now();
        let mut input = InputController::with_release_events(true);
        input.handle_key_at(key(KeyCode::Right, KeyEventKind::Press), start);
        input.handle_key_at(key(KeyCode::Down, KeyEventKind::Press), start);

        let control_c = KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert!(matches!(
            input.handle_key_at(control_c, start),
            Some(GameAction::Quit)
        ));

        input.clear_motion();
        let controls = input.snapshot_at(start);
        assert!(!controls.left);
        assert!(!controls.right);
        assert!(!controls.accelerate);
        assert!(!controls.brake);
    }

    #[test]
    fn clear_all_rearms_edge_keys_after_focus_loss() {
        let start = Instant::now();
        let mut input = InputController::with_release_events(true);
        let pause = key(KeyCode::Char('p'), KeyEventKind::Press);
        assert_eq!(
            input.handle_key_at(pause, start),
            Some(GameAction::PauseToggle)
        );

        input.clear_all();
        assert_eq!(
            input.handle_key_at(pause, start),
            Some(GameAction::PauseToggle)
        );
    }
}
