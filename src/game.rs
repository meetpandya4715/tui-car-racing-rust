//! Terminal-independent state machine and fixed-step racing simulation.

use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{
    collision::Rect,
    entities::{Player, Road, SPRITE_HEIGHT, SPRITE_WIDTH, Vehicle, VehicleKind},
};

const MIN_SPEED: f32 = 55.0;
const MAX_SPEED: f32 = 180.0;
const BOOST_MAX_SPEED: f32 = 220.0;
const START_SPEED: f32 = 68.0;
const ACCELERATION: f32 = 40.0;
const BOOST_ACCELERATION: f32 = 95.0;
const BRAKING: f32 = 80.0;
const COAST_DECELERATION: f32 = 8.0;
const OVERSPEED_DECELERATION: f32 = 35.0;
const STEERING_SPEED: f32 = 25.0;
const STEERING_RESPONSE: f32 = 105.0;
const PASS_BASE_SCORE: u64 = 200;
const NEAR_MISS_BASE_SCORE: u64 = 300;
const NEAR_MISS_GAP_CELLS: i32 = 2;
const STREAK_WINDOW: f32 = 4.5;
const MAX_STREAK: u8 = 5;
const NITRO_MAX: f32 = 100.0;
const NITRO_START: f32 = 50.0;
const NITRO_DRAIN_PER_SECOND: f32 = 35.0;
const NITRO_RECHARGE_PER_SECOND: f32 = 6.0;
const NITRO_NEAR_MISS_GAIN: f32 = 18.0;
const EVENT_DISPLAY_SECONDS: f32 = 0.9;
const LEVEL_EVENT_SECONDS: f32 = 1.2;
const DISTANCE_PER_LEVEL: f32 = 500.0;
const MAX_LEVEL: u8 = 10;
const INITIAL_SPAWN_DELAY: f32 = 2.5;
const RESIZE_SPAWN_GRACE: f32 = 1.2;
// Common wrap period for the 7-cell lane/paving patterns, 12-cell lamps,
// 28-cell texture grain, 84-cell scenery, and 3-cell boost streaks.
const ROAD_ANIMATION_PERIOD: f32 = 84.0;
const SPAWN_HEADWAY_CELLS: i32 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlInput {
    pub left: bool,
    pub right: bool,
    pub accelerate: bool,
    pub brake: bool,
    pub boost: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameAction {
    Start,
    PauseToggle,
    Restart,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Title,
    Playing,
    Paused,
    GameOver,
    Quit,
}

/// A short-lived scoring or progression event for renderer feedback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaceEvent {
    Pass { points: u64, streak: u8 },
    NearMiss { points: u64, streak: u8 },
    LevelUp { level: u8 },
}

pub struct Game {
    state: GameState,
    road: Road,
    player: Player,
    vehicles: Vec<Vehicle>,
    speed: f32,
    distance: f32,
    run_time: f32,
    score: u64,
    high_score: u64,
    traffic_score: u64,
    passed: u32,
    streak: u8,
    best_streak: u8,
    streak_timer: f32,
    near_misses: u32,
    nitro: f32,
    boosting: bool,
    last_event: Option<RaceEvent>,
    event_timer: f32,
    level: u8,
    road_phase: f32,
    spawn_timer: f32,
    rng: StdRng,
}

impl Game {
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        Self::new_seeded(width, height, rand::rng().random())
    }

    #[must_use]
    pub fn new_seeded(width: u16, height: u16, seed: u64) -> Self {
        let road = Road::new(width, height);
        let player = centered_player(&road);
        Self {
            state: GameState::Title,
            road,
            player,
            vehicles: Vec::new(),
            speed: START_SPEED,
            distance: 0.0,
            run_time: 0.0,
            score: 0,
            high_score: 0,
            traffic_score: 0,
            passed: 0,
            streak: 0,
            best_streak: 0,
            streak_timer: 0.0,
            near_misses: 0,
            nitro: NITRO_START,
            boosting: false,
            last_event: None,
            event_timer: 0.0,
            level: 1,
            road_phase: 0.0,
            spawn_timer: INITIAL_SPAWN_DELAY,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn handle_action(&mut self, action: GameAction) {
        match action {
            GameAction::Quit => self.state = GameState::Quit,
            GameAction::Start if matches!(self.state, GameState::Title | GameState::GameOver) => {
                self.restart_round();
            }
            GameAction::Restart if self.state == GameState::GameOver => self.restart_round(),
            GameAction::PauseToggle => {
                self.state = match self.state {
                    GameState::Playing => GameState::Paused,
                    GameState::Paused => GameState::Playing,
                    other => other,
                };
            }
            GameAction::Start | GameAction::Restart => {}
        }

        // Boost is an active driving effect, never a persistent menu/pause state.
        if self.state != GameState::Playing {
            self.boosting = false;
        }
    }

    pub fn update(&mut self, elapsed_seconds: f32, controls: ControlInput) {
        if self.state != GameState::Playing || !elapsed_seconds.is_finite() {
            return;
        }

        let dt = elapsed_seconds.clamp(0.0, 0.1);
        if dt == 0.0 {
            return;
        }

        self.run_time += dt;
        self.update_feedback_timers(dt);
        self.update_speed(dt, controls);
        self.update_player(dt, controls);

        self.distance += self.speed / 3.6 * dt;
        let next_level = difficulty_for_distance(self.distance);
        if next_level > self.level {
            self.level = next_level;
            self.set_event(
                RaceEvent::LevelUp { level: self.level },
                LEVEL_EVENT_SECONDS,
            );
        } else {
            self.level = next_level;
        }
        let scroll_speed = self.scroll_speed();
        self.road_phase = (self.road_phase + scroll_speed * dt).rem_euclid(ROAD_ANIMATION_PERIOD);

        for vehicle in &mut self.vehicles {
            vehicle.y += scroll_speed * vehicle.downward_speed * dt;
        }

        let player_rect = self.player.rect();
        let mut clean_passes = 0_u8;
        let mut near_miss_passes = 0_u8;
        for index in 0..self.vehicles.len() {
            let clearance = {
                let vehicle = &mut self.vehicles[index];
                if !vehicle.passed && vehicle.rect().top() >= player_rect.bottom() {
                    vehicle.passed = true;
                    Some(horizontal_clearance(player_rect, vehicle.rect()))
                } else {
                    None
                }
            };

            if let Some(clearance) = clearance {
                if (1..=NEAR_MISS_GAP_CELLS).contains(&clearance) {
                    near_miss_passes = near_miss_passes.saturating_add(1);
                } else {
                    clean_passes = clean_passes.saturating_add(1);
                }
            }
        }
        // Resolve same-tick passes in a stable order, independent of vehicle
        // storage order. Near misses go last so the riskier pass receives the
        // newest streak multiplier and remains the visible ticker event.
        for _ in 0..clean_passes {
            self.award_pass(false);
        }
        for _ in 0..near_miss_passes {
            self.award_pass(true);
        }
        self.refresh_score();

        if self
            .vehicles
            .iter()
            .any(|vehicle| self.player.rect().overlaps(vehicle.rect()))
        {
            self.state = GameState::GameOver;
            self.boosting = false;
            self.high_score = self.high_score.max(self.score);
            return;
        }

        let despawn_y = self.road.bottom.saturating_add(i32::from(SPRITE_HEIGHT));
        self.vehicles
            .retain(|vehicle| vehicle.rect().top() <= despawn_y);
        self.refresh_score();

        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            let spawned = if self.vehicles.len() < max_traffic_for_level(self.level) {
                self.try_spawn_vehicle(scroll_speed)
            } else {
                false
            };

            self.spawn_timer = if spawned {
                let jitter = self.rng.random_range(0.90..=1.10);
                spawn_interval(self.distance, self.speed) * jitter
            } else {
                0.2
            };
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        let old_min = self.road.min_x_for(SPRITE_WIDTH) as f32;
        let old_max = self.road.max_x_for(SPRITE_WIDTH) as f32;
        let old_span = (old_max - old_min).max(1.0);
        let relative_x = ((self.player.x - old_min) / old_span).clamp(0.0, 1.0);

        self.road = Road::new(width, height);
        let new_min = self.road.min_x_for(SPRITE_WIDTH) as f32;
        let new_max = self.road.max_x_for(SPRITE_WIDTH) as f32;
        self.player.x = new_min + relative_x * (new_max - new_min);
        self.player.x = self.road.clamp_x(self.player.x, SPRITE_WIDTH);
        self.player.y = self.road.player_y(SPRITE_HEIGHT) as f32;
        self.player.lateral_velocity = 0.0;

        // Existing traffic is tied to the old lane geometry. Clearing it gives
        // the player a short grace period and prevents resize-created crashes.
        self.vehicles.clear();
        self.spawn_timer = RESIZE_SPAWN_GRACE;
    }

    #[must_use]
    pub const fn state(&self) -> GameState {
        self.state
    }

    #[must_use]
    pub const fn road(&self) -> &Road {
        &self.road
    }

    #[must_use]
    pub const fn player(&self) -> &Player {
        &self.player
    }

    #[must_use]
    pub fn vehicles(&self) -> &[Vehicle] {
        &self.vehicles
    }

    #[must_use]
    pub const fn score(&self) -> u64 {
        self.score
    }

    #[must_use]
    pub const fn high_score(&self) -> u64 {
        self.high_score
    }

    #[must_use]
    pub const fn speed(&self) -> f32 {
        self.speed
    }

    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    #[must_use]
    pub const fn run_time(&self) -> f32 {
        self.run_time
    }

    #[must_use]
    pub const fn level(&self) -> u8 {
        self.level
    }

    #[must_use]
    pub const fn passed(&self) -> u32 {
        self.passed
    }

    #[must_use]
    pub const fn streak(&self) -> u8 {
        self.streak
    }

    #[must_use]
    pub const fn best_streak(&self) -> u8 {
        self.best_streak
    }

    #[must_use]
    pub const fn near_misses(&self) -> u32 {
        self.near_misses
    }

    #[must_use]
    pub const fn nitro(&self) -> f32 {
        self.nitro
    }

    #[must_use]
    pub const fn boosting(&self) -> bool {
        self.boosting
    }

    #[must_use]
    pub const fn last_event(&self) -> Option<RaceEvent> {
        self.last_event
    }

    #[must_use]
    pub const fn event_time_remaining(&self) -> f32 {
        self.event_timer
    }

    #[must_use]
    pub const fn road_phase(&self) -> f32 {
        self.road_phase
    }

    fn restart_round(&mut self) {
        self.state = GameState::Playing;
        self.player = centered_player(&self.road);
        self.vehicles.clear();
        self.speed = START_SPEED;
        self.distance = 0.0;
        self.run_time = 0.0;
        self.score = 0;
        self.traffic_score = 0;
        self.passed = 0;
        self.streak = 0;
        self.best_streak = 0;
        self.streak_timer = 0.0;
        self.near_misses = 0;
        self.nitro = NITRO_START;
        self.boosting = false;
        self.last_event = None;
        self.event_timer = 0.0;
        self.level = 1;
        self.road_phase = 0.0;
        self.spawn_timer = INITIAL_SPAWN_DELAY;
    }

    fn update_speed(&mut self, dt: f32, controls: ControlInput) {
        self.boosting = controls.boost && !controls.brake && self.nitro > 0.0;
        let boost_seconds = if self.boosting {
            dt.min(self.nitro / NITRO_DRAIN_PER_SECOND)
        } else {
            0.0
        };
        if self.boosting {
            self.nitro = (self.nitro - NITRO_DRAIN_PER_SECOND * boost_seconds).max(0.0);
        } else if !controls.boost {
            self.nitro = (self.nitro + NITRO_RECHARGE_PER_SECOND * dt).min(NITRO_MAX);
        }

        if controls.brake {
            self.speed -= BRAKING * dt;
        } else {
            self.speed += BOOST_ACCELERATION * boost_seconds;

            // If the tank runs dry between fixed ticks, finish the remainder of
            // this tick with ordinary driving physics instead of granting a
            // whole tick of boost for a trace amount of nitro.
            let normal_seconds = dt - boost_seconds;
            if self.speed > MAX_SPEED {
                self.speed = (self.speed - OVERSPEED_DECELERATION * normal_seconds).max(MAX_SPEED);
            } else if controls.accelerate {
                self.speed = (self.speed + ACCELERATION * normal_seconds).min(MAX_SPEED);
            } else {
                self.speed -= COAST_DECELERATION * normal_seconds;
            }
        }
        self.speed = self.speed.clamp(MIN_SPEED, BOOST_MAX_SPEED);
    }

    fn update_player(&mut self, dt: f32, controls: ControlInput) {
        let direction = f32::from(i8::from(controls.right) - i8::from(controls.left));
        let speed_scale = steering_speed_scale(self.speed);
        let target_velocity = direction * STEERING_SPEED * speed_scale;
        self.player.lateral_velocity = approach(
            self.player.lateral_velocity,
            target_velocity,
            STEERING_RESPONSE * dt,
        );
        self.player.x += self.player.lateral_velocity * dt;

        let unclamped_x = self.player.x;
        self.player.x = self.road.clamp_x(self.player.x, SPRITE_WIDTH);
        if self.player.x != unclamped_x
            && ((self.player.x <= self.road.min_x_for(SPRITE_WIDTH) as f32
                && self.player.lateral_velocity < 0.0)
                || (self.player.x >= self.road.max_x_for(SPRITE_WIDTH) as f32
                    && self.player.lateral_velocity > 0.0))
        {
            self.player.lateral_velocity = 0.0;
        }
    }

    fn update_feedback_timers(&mut self, dt: f32) {
        self.streak_timer = (self.streak_timer - dt).max(0.0);
        if self.streak_timer == 0.0 {
            self.streak = 0;
        }

        self.event_timer = (self.event_timer - dt).max(0.0);
        if self.event_timer == 0.0 {
            self.last_event = None;
        }
    }

    fn set_event(&mut self, event: RaceEvent, duration: f32) {
        if self.event_timer > 0.0
            && matches!(self.last_event, Some(RaceEvent::LevelUp { .. }))
            && !matches!(event, RaceEvent::LevelUp { .. })
        {
            return;
        }
        self.last_event = Some(event);
        self.event_timer = duration;
    }

    fn award_pass(&mut self, near_miss: bool) {
        self.passed = self.passed.saturating_add(1);
        self.streak = if self.streak_timer > 0.0 {
            self.streak.saturating_add(1).min(MAX_STREAK)
        } else {
            1
        };
        self.best_streak = self.best_streak.max(self.streak);
        self.streak_timer = STREAK_WINDOW;

        let multiplier = u64::from(self.streak);
        let mut points = PASS_BASE_SCORE.saturating_mul(multiplier);
        let event = if near_miss {
            points = points.saturating_add(NEAR_MISS_BASE_SCORE.saturating_mul(multiplier));
            self.near_misses = self.near_misses.saturating_add(1);
            self.nitro = (self.nitro + NITRO_NEAR_MISS_GAIN).min(NITRO_MAX);
            RaceEvent::NearMiss {
                points,
                streak: self.streak,
            }
        } else {
            RaceEvent::Pass {
                points,
                streak: self.streak,
            }
        };

        self.traffic_score = self.traffic_score.saturating_add(points);
        self.set_event(event, EVENT_DISPLAY_SECONDS);
    }

    fn scroll_speed(&self) -> f32 {
        5.0 + 8.2 * speed_ratio(self.speed)
            + 2.8 * difficulty_progress(self.distance)
            + 3.0 * boost_ratio(self.speed)
    }

    fn refresh_score(&mut self) {
        self.score = score_from_distance(self.distance).saturating_add(self.traffic_score);
        self.high_score = self.high_score.max(self.score);
    }

    fn try_spawn_vehicle(&mut self, scroll_speed: f32) -> bool {
        let lane_count = self.road.lane_count.max(1);
        let starting_lane = self.rng.random_range(0..lane_count);
        let attempts = usize::from(lane_count).saturating_mul(2);

        for attempt in 0..attempts {
            let lane = (starting_lane + u8::try_from(attempt).unwrap_or(0)) % lane_count;
            let jitter = self.rng.random_range(-1_i32..=1_i32);
            let x = self
                .road
                .lane_x_for(lane, SPRITE_WIDTH)
                .saturating_add(jitter)
                .clamp(
                    self.road.min_x_for(SPRITE_WIDTH),
                    self.road.max_x_for(SPRITE_WIDTH),
                );
            // Begin fully above the road so taller cars enter smoothly instead
            // of popping into view, while preserving a fair reaction window.
            let y = self.road.top.saturating_sub(i32::from(SPRITE_HEIGHT)) as f32;
            let (minimum_speed, maximum_speed) = enemy_speed_range(self.distance);
            let downward_speed = self.rng.random_range(minimum_speed..=maximum_speed);
            let kind = VehicleKind::ALL[self.rng.random_range(0..VehicleKind::ALL.len())];
            let candidate = Vehicle::new(x as f32, y, downward_speed, kind);

            if is_valid_enemy_spawn(&self.road, &self.vehicles, &candidate)
                && self.spawn_keeps_reachable_gap(&candidate, scroll_speed)
            {
                self.vehicles.push(candidate);
                return true;
            }
        }

        false
    }

    fn spawn_keeps_reachable_gap(&self, candidate: &Vehicle, scroll_speed: f32) -> bool {
        let candidate_speed = (scroll_speed * candidate.downward_speed).max(0.1);
        let player_top = self.player.render_y() as f32;
        let player_bottom = self.player.rect().bottom() as f32;
        let sprite_height = f32::from(SPRITE_HEIGHT);
        let collision_start = (player_top - sprite_height - candidate.y) / candidate_speed;
        let collision_end = (player_bottom - candidate.y) / candidate_speed;
        // Keep the warm-up readable without suppressing traffic entirely in a
        // minimum-height terminal at normal top speed. Nitro can briefly outrun
        // early spawn attempts, but ordinary acceleration cannot.
        let minimum_reaction = 0.85 - 0.30 * difficulty_progress(self.distance);
        if collision_start < minimum_reaction || collision_end <= collision_start {
            return false;
        }

        // Any car whose crossing window intersects the candidate's is treated
        // as a blocker for the whole short span. Requiring one fixed, full-car
        // gap is conservative and prevents staggered sprites from forming an
        // apparently open gap that is too narrow to use.
        let blockers: Vec<Rect> = self
            .vehicles
            .iter()
            .chain(std::iter::once(candidate))
            .filter_map(|vehicle| {
                let vehicle_speed = (scroll_speed * vehicle.downward_speed).max(0.1);
                let vehicle_start = (player_top - sprite_height - vehicle.y) / vehicle_speed;
                let vehicle_end = (player_bottom - vehicle.y) / vehicle_speed;
                (vehicle_end >= collision_start && vehicle_start <= collision_end).then(|| {
                    Rect::new(
                        vehicle.render_x(),
                        self.player.render_y(),
                        SPRITE_WIDTH,
                        SPRITE_HEIGHT,
                    )
                })
            })
            .collect();

        // Bound steering with the same target velocity and response used by
        // update_player. One render frame is removed from the reaction time to
        // cover fixed-step and cell-rounding error.
        let reaction_time = (collision_start - 1.0 / 30.0).max(0.0);
        let maximum_velocity = STEERING_SPEED * steering_speed_scale(self.speed);
        let left_position = self.player.x
            + displacement_toward(
                self.player.lateral_velocity,
                -maximum_velocity,
                STEERING_RESPONSE,
                reaction_time,
            );
        let right_position = self.player.x
            + displacement_toward(
                self.player.lateral_velocity,
                maximum_velocity,
                STEERING_RESPONSE,
                reaction_time,
            );
        let road_min = self.road.min_x_for(SPRITE_WIDTH);
        let road_max = self.road.max_x_for(SPRITE_WIDTH);
        let reachable_min = (left_position.min(right_position).max(road_min as f32)).ceil() as i32;
        let reachable_max = (left_position.max(right_position).min(road_max as f32)).floor() as i32;

        (reachable_min..=reachable_max).any(|x| {
            let player_rect = Rect::new(x, self.player.render_y(), SPRITE_WIDTH, SPRITE_HEIGHT);
            blockers
                .iter()
                .all(|blocker| !player_rect.overlaps_horizontally(*blocker))
        })
    }
}

fn centered_player(road: &Road) -> Player {
    Player::new(
        road.centered_x_for(SPRITE_WIDTH) as f32,
        road.player_y(SPRITE_HEIGHT) as f32,
    )
}

fn approach(current: f32, target: f32, maximum_change: f32) -> f32 {
    if current < target {
        (current + maximum_change).min(target)
    } else {
        (current - maximum_change).max(target)
    }
}

fn displacement_toward(
    initial_velocity: f32,
    target_velocity: f32,
    acceleration: f32,
    time: f32,
) -> f32 {
    let velocity_change = target_velocity - initial_velocity;
    let direction = velocity_change.signum();
    let acceleration_time = (velocity_change.abs() / acceleration.max(f32::EPSILON)).min(time);
    let accelerating_distance = initial_velocity * acceleration_time
        + 0.5 * direction * acceleration * acceleration_time * acceleration_time;
    let cruising_time = (time - acceleration_time).max(0.0);
    accelerating_distance + target_velocity * cruising_time
}

fn horizontal_clearance(first: Rect, second: Rect) -> i32 {
    if first.right() <= second.left() {
        second.left() - first.right()
    } else if second.right() <= first.left() {
        first.left() - second.right()
    } else {
        0
    }
}

fn speed_ratio(speed: f32) -> f32 {
    ((speed - MIN_SPEED) / (MAX_SPEED - MIN_SPEED)).clamp(0.0, 1.0)
}

fn boost_ratio(speed: f32) -> f32 {
    ((speed - MAX_SPEED) / (BOOST_MAX_SPEED - MAX_SPEED)).clamp(0.0, 1.0)
}

fn steering_speed_scale(speed: f32) -> f32 {
    0.86 + 0.24 * speed_ratio(speed)
}

#[must_use]
fn score_from_distance(distance: f32) -> u64 {
    distance.max(0.0).floor() as u64
}

#[must_use]
fn difficulty_progress(distance: f32) -> f32 {
    let maximum_distance = DISTANCE_PER_LEVEL * f32::from(MAX_LEVEL - 1);
    (distance.max(0.0) / maximum_distance).clamp(0.0, 1.0)
}

#[must_use]
fn difficulty_for_distance(distance: f32) -> u8 {
    let gained_levels = (distance.max(0.0) / DISTANCE_PER_LEVEL).floor() as u8;
    1_u8.saturating_add(gained_levels).min(MAX_LEVEL)
}

#[must_use]
fn max_traffic_for_level(level: u8) -> usize {
    2 + usize::from(level.saturating_sub(1).min(MAX_LEVEL - 1)) * 6 / usize::from(MAX_LEVEL - 1)
}

#[must_use]
fn spawn_interval(distance: f32, speed: f32) -> f32 {
    let difficulty = difficulty_progress(distance);
    let base_interval = 1.85 + (0.62 - 1.85) * difficulty;
    base_interval * (1.08 + (0.90 - 1.08) * speed_ratio(speed))
}

#[must_use]
fn enemy_speed_range(distance: f32) -> (f32, f32) {
    let difficulty = difficulty_progress(distance);
    (
        0.92 + (0.78 - 0.92) * difficulty,
        1.06 + (1.22 - 1.06) * difficulty,
    )
}

/// Validates road bounds, exact overlap, same-path headway, and a free lane in
/// the spawn band. The predictive reachable-gap check is applied by `Game`
/// because it also depends on the player's current position and road speed.
#[must_use]
pub(crate) fn is_valid_enemy_spawn(road: &Road, existing: &[Vehicle], candidate: &Vehicle) -> bool {
    let candidate_rect = candidate.rect();
    let minimum_spawn_top = road.top.saturating_sub(i32::from(SPRITE_HEIGHT));
    if candidate_rect.left() < road.min_x_for(SPRITE_WIDTH)
        || candidate_rect.right() > road.right
        || candidate_rect.top() < minimum_spawn_top
    {
        return false;
    }

    if existing.iter().any(|vehicle| {
        let rect = vehicle.rect();
        if candidate_rect.overlaps(rect) {
            return true;
        }

        if !candidate_rect.overlaps_horizontally(rect) {
            return false;
        }

        let gap = if candidate_rect.bottom() <= rect.top() {
            rect.top() - candidate_rect.bottom()
        } else if rect.bottom() <= candidate_rect.top() {
            candidate_rect.top() - rect.bottom()
        } else {
            0
        };
        gap < SPAWN_HEADWAY_CELLS
    }) {
        return false;
    }

    let spawn_band = Rect::new(
        road.left.saturating_add(1),
        candidate_rect.top().saturating_sub(2),
        u16::try_from(road.inner_width()).unwrap_or(u16::MAX),
        SPRITE_HEIGHT.saturating_add(5),
    );
    let every_lane_blocked = (0..road.lane_count).all(|lane| {
        let lane_rect = Rect::new(
            road.lane_x_for(lane, SPRITE_WIDTH),
            spawn_band.top(),
            SPRITE_WIDTH,
            spawn_band.height,
        );
        existing
            .iter()
            .chain(std::iter::once(candidate))
            .any(|vehicle| lane_rect.overlaps(vehicle.rect()))
    });

    !every_lane_blocked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started_game(seed: u64) -> Game {
        let mut game = Game::new_seeded(80, 30, seed);
        game.handle_action(GameAction::Start);
        game
    }

    fn add_vehicle_ready_to_pass(game: &mut Game, clearance: Option<i32>) {
        let player_rect = game.player.rect();
        let x = clearance.map_or_else(
            || game.road.min_x_for(SPRITE_WIDTH),
            |gap| player_rect.right().saturating_add(gap),
        );
        game.vehicles.push(Vehicle::new(
            x as f32,
            player_rect.bottom() as f32,
            0.0,
            VehicleKind::Sports,
        ));
    }

    #[test]
    fn player_is_clamped_to_both_road_boundaries() {
        let mut game = started_game(1);
        for _ in 0..600 {
            game.update(
                1.0 / 60.0,
                ControlInput {
                    left: true,
                    ..ControlInput::default()
                },
            );
        }
        assert_eq!(game.player.render_x(), game.road.min_x_for(SPRITE_WIDTH));

        for _ in 0..1_200 {
            game.update(
                1.0 / 60.0,
                ControlInput {
                    right: true,
                    ..ControlInput::default()
                },
            );
            if game.state != GameState::Playing {
                game.state = GameState::Playing;
                game.vehicles.clear();
            }
        }
        assert_eq!(game.player.render_x(), game.road.max_x_for(SPRITE_WIDTH));
    }

    #[test]
    fn multi_character_vehicle_collision_ends_round() {
        let mut game = started_game(2);
        game.vehicles.push(Vehicle::new(
            game.player.x + 4.0,
            game.player.y + 2.0,
            0.0,
            VehicleKind::Truck,
        ));

        game.update(1.0 / 60.0, ControlInput::default());
        assert_eq!(game.state, GameState::GameOver);
    }

    #[test]
    fn valid_spawn_is_inside_road_and_overlap_is_rejected() {
        let road = Road::new(80, 30);
        let candidate = Vehicle::new(
            road.lane_x_for(0, SPRITE_WIDTH) as f32,
            (road.top + 1) as f32,
            1.0,
            VehicleKind::Sedan,
        );
        assert!(is_valid_enemy_spawn(&road, &[], &candidate));
        assert!(!is_valid_enemy_spawn(
            &road,
            std::slice::from_ref(&candidate),
            &candidate
        ));
    }

    #[test]
    fn traffic_may_spawn_just_above_the_road_but_not_above_its_entry_band() {
        let road = Road::new(60, 24);
        let minimum_top = road.top - i32::from(SPRITE_HEIGHT);
        let x = road.lane_x_for(0, SPRITE_WIDTH) as f32;
        let entering = Vehicle::new(x, minimum_top as f32, 1.0, VehicleKind::Sedan);
        let too_high = Vehicle::new(x, (minimum_top - 1) as f32, 1.0, VehicleKind::Sedan);

        assert!(is_valid_enemy_spawn(&road, &[], &entering));
        assert!(!is_valid_enemy_spawn(&road, &[], &too_high));
    }

    #[test]
    fn spawn_headway_rejects_same_path_traffic() {
        let road = Road::new(80, 30);
        let x = road.lane_x_for(1, SPRITE_WIDTH) as f32;
        let first = Vehicle::new(x, 4.0, 1.0, VehicleKind::Sedan);
        let too_close = Vehicle::new(x, 10.0, 1.0, VehicleKind::Sports);
        let far_enough = Vehicle::new(
            x,
            4.0 + f32::from(SPRITE_HEIGHT) + SPAWN_HEADWAY_CELLS as f32,
            1.0,
            VehicleKind::Truck,
        );

        assert!(!is_valid_enemy_spawn(
            &road,
            std::slice::from_ref(&first),
            &too_close
        ));
        assert!(is_valid_enemy_spawn(&road, &[first], &far_enough));
    }

    #[test]
    fn spawn_rejects_a_full_width_barrier() {
        let road = Road::new(50, 30);
        let existing: Vec<_> = (0..road.lane_count.saturating_sub(1))
            .map(|lane| {
                Vehicle::new(
                    road.lane_x_for(lane, SPRITE_WIDTH) as f32,
                    4.0,
                    1.0,
                    VehicleKind::ALL[usize::from(lane) % VehicleKind::ALL.len()],
                )
            })
            .collect();
        let final_lane = Vehicle::new(
            road.lane_x_for(road.lane_count - 1, SPRITE_WIDTH) as f32,
            4.0,
            1.0,
            VehicleKind::Truck,
        );

        assert!(!is_valid_enemy_spawn(&road, &existing, &final_lane));
    }

    #[test]
    fn score_uses_distance_floor_and_accumulated_traffic_score() {
        assert_eq!(score_from_distance(19.99), 19);
        assert_eq!(score_from_distance(20.0), 20);

        let mut game = started_game(3);
        game.distance = 20.9;
        game.traffic_score = 600;
        game.refresh_score();
        assert_eq!(game.score, 620);
    }

    #[test]
    fn passing_bonus_is_awarded_once() {
        let mut game = started_game(4);
        let already_below = (game.player.rect().bottom() + 1) as f32;
        game.vehicles.push(Vehicle::new(
            game.player.x,
            already_below,
            0.0,
            VehicleKind::Sports,
        ));

        game.update(1.0 / 60.0, ControlInput::default());
        assert_eq!(game.passed, 1);
        assert_eq!(game.traffic_score, PASS_BASE_SCORE);
        let score_after_pass = game.score;
        game.update(1.0 / 60.0, ControlInput::default());
        assert_eq!(game.passed, 1);
        assert_eq!(game.traffic_score, PASS_BASE_SCORE);
        assert!(game.score < score_after_pass + PASS_BASE_SCORE);
    }

    #[test]
    fn timely_passes_build_a_capped_streak_and_variable_score() {
        let mut game = started_game(40);
        game.spawn_timer = 100.0;

        for expected_streak in 1..=7 {
            add_vehicle_ready_to_pass(&mut game, None);
            game.update(1.0 / 60.0, ControlInput::default());

            let capped_streak = expected_streak.min(MAX_STREAK);
            assert_eq!(game.streak(), capped_streak);
            assert_eq!(
                game.last_event(),
                Some(RaceEvent::Pass {
                    points: PASS_BASE_SCORE * u64::from(capped_streak),
                    streak: capped_streak,
                })
            );
        }

        assert_eq!(game.passed(), 7);
        assert_eq!(game.best_streak(), MAX_STREAK);
        assert_eq!(game.traffic_score, 5_000);
    }

    #[test]
    fn expired_streak_makes_the_next_pass_start_over() {
        let mut game = started_game(41);
        game.spawn_timer = 100.0;
        add_vehicle_ready_to_pass(&mut game, None);
        game.update(1.0 / 60.0, ControlInput::default());
        assert_eq!(game.streak(), 1);

        game.update_feedback_timers(STREAK_WINDOW);
        assert_eq!(game.streak(), 0);

        add_vehicle_ready_to_pass(&mut game, None);
        game.update(1.0 / 60.0, ControlInput::default());
        assert_eq!(game.streak(), 1);
        assert_eq!(game.best_streak(), 1);
        assert_eq!(game.traffic_score, PASS_BASE_SCORE * 2);
    }

    #[test]
    fn one_and_two_cell_clearances_are_near_misses_but_other_gaps_are_not() {
        for (gap, expected_near_misses, expected_points) in [
            (0, 0, PASS_BASE_SCORE),
            (1, 1, PASS_BASE_SCORE + NEAR_MISS_BASE_SCORE),
            (2, 1, PASS_BASE_SCORE + NEAR_MISS_BASE_SCORE),
            (3, 0, PASS_BASE_SCORE),
        ] {
            let mut game = started_game(50 + gap as u64);
            game.nitro = 0.0;
            game.spawn_timer = 100.0;
            add_vehicle_ready_to_pass(&mut game, Some(gap));
            game.update(
                1.0 / 60.0,
                ControlInput {
                    boost: true,
                    ..ControlInput::default()
                },
            );

            assert_eq!(game.near_misses(), expected_near_misses);
            assert_eq!(game.traffic_score, expected_points);
            assert_eq!(
                game.nitro(),
                if expected_near_misses == 1 {
                    NITRO_NEAR_MISS_GAIN
                } else {
                    0.0
                }
            );
            assert_eq!(
                game.last_event(),
                Some(if expected_near_misses == 1 {
                    RaceEvent::NearMiss {
                        points: expected_points,
                        streak: 1,
                    }
                } else {
                    RaceEvent::Pass {
                        points: expected_points,
                        streak: 1,
                    }
                })
            );
        }
    }

    #[test]
    fn near_miss_is_awarded_only_once_and_refills_nitro_to_its_cap() {
        let mut game = started_game(55);
        game.nitro = 95.0;
        game.spawn_timer = 100.0;
        add_vehicle_ready_to_pass(&mut game, Some(1));

        game.update(
            1.0 / 60.0,
            ControlInput {
                boost: true,
                ..ControlInput::default()
            },
        );
        assert_eq!(game.near_misses(), 1);
        assert_eq!(game.nitro(), NITRO_MAX);
        let traffic_score = game.traffic_score;

        game.update(1.0 / 60.0, ControlInput::default());
        assert_eq!(game.near_misses(), 1);
        assert_eq!(game.traffic_score, traffic_score);
    }

    #[test]
    fn simultaneous_mixed_passes_score_independently_of_vehicle_order() {
        fn result(near_miss_first: bool) -> (u64, u32, u32, u8, Option<RaceEvent>) {
            let mut game = started_game(57);
            game.nitro = 0.0;
            game.spawn_timer = 100.0;
            if near_miss_first {
                add_vehicle_ready_to_pass(&mut game, Some(1));
                add_vehicle_ready_to_pass(&mut game, None);
            } else {
                add_vehicle_ready_to_pass(&mut game, None);
                add_vehicle_ready_to_pass(&mut game, Some(1));
            }

            game.update(1.0 / 60.0, ControlInput::default());
            (
                game.traffic_score,
                game.passed(),
                game.near_misses(),
                game.streak(),
                game.last_event(),
            )
        }

        let clean_first = result(false);
        let near_miss_first = result(true);

        assert_eq!(clean_first, near_miss_first);
        assert_eq!(clean_first.0, 1_200);
        assert_eq!(clean_first.1, 2);
        assert_eq!(clean_first.2, 1);
        assert_eq!(clean_first.3, 2);
        assert_eq!(
            clean_first.4,
            Some(RaceEvent::NearMiss {
                points: 1_000,
                streak: 2,
            })
        );
    }

    #[test]
    fn feedback_events_expire_without_erasing_best_streak() {
        let mut game = started_game(56);
        add_vehicle_ready_to_pass(&mut game, None);
        game.update(1.0 / 60.0, ControlInput::default());
        assert!(game.event_time_remaining() > 0.0);
        assert_eq!(game.best_streak(), 1);

        game.update_feedback_timers(EVENT_DISPLAY_SECONDS);
        assert_eq!(game.last_event(), None);
        assert_eq!(game.event_time_remaining(), 0.0);
        assert_eq!(game.best_streak(), 1);
    }

    #[test]
    fn safe_pass_is_scored_when_another_vehicle_crashes_same_tick() {
        let mut game = started_game(14);
        let safely_below = game.player.rect().bottom() as f32;
        game.vehicles.push(Vehicle::new(
            game.road.min_x_for(SPRITE_WIDTH) as f32,
            safely_below,
            0.0,
            VehicleKind::Sports,
        ));
        game.vehicles.push(Vehicle::new(
            game.player.x,
            game.player.y,
            0.0,
            VehicleKind::Truck,
        ));

        game.update(1.0 / 60.0, ControlInput::default());

        assert_eq!(game.state, GameState::GameOver);
        assert_eq!(game.passed, 1);
        assert!(game.score >= PASS_BASE_SCORE);
        assert_eq!(game.high_score, game.score);
    }

    #[test]
    fn difficulty_progresses_and_caps() {
        assert_eq!(difficulty_for_distance(0.0), 1);
        assert_eq!(difficulty_for_distance(499.99), 1);
        assert_eq!(difficulty_for_distance(500.0), 2);
        assert_eq!(difficulty_for_distance(4_500.0), 10);
        assert_eq!(difficulty_for_distance(99_999.0), 10);

        assert_eq!(difficulty_progress(-1.0), 0.0);
        assert_eq!(difficulty_progress(0.0), 0.0);
        assert_eq!(difficulty_progress(2_250.0), 0.5);
        assert_eq!(difficulty_progress(4_500.0), 1.0);
        assert_eq!(difficulty_progress(99_999.0), 1.0);

        let expected_caps = [2, 2, 3, 4, 4, 5, 6, 6, 7, 8];
        let actual_caps = std::array::from_fn(|index| max_traffic_for_level(index as u8 + 1));
        assert_eq!(actual_caps, expected_caps);

        let easiest_interval = spawn_interval(0.0, MIN_SPEED);
        let hardest_interval = spawn_interval(4_500.0, MAX_SPEED);
        assert!((easiest_interval - 1.998).abs() < 0.000_1);
        assert!((hardest_interval - 0.558).abs() < 0.000_1);
        assert!(hardest_interval < easiest_interval);

        let mut previous = easiest_interval;
        for distance in (500..=4_500).step_by(500) {
            let interval = spawn_interval(distance as f32, MIN_SPEED);
            assert!(interval < previous);
            previous = interval;
        }

        assert_eq!(enemy_speed_range(0.0), (0.92, 1.06));
        assert_eq!(enemy_speed_range(4_500.0), (0.78, 1.22));
    }

    #[test]
    fn restart_resets_round_and_preserves_high_score() {
        let mut game = started_game(5);
        game.distance = 900.0;
        game.run_time = 42.5;
        game.traffic_score = 1_200;
        game.passed = 3;
        game.streak = 3;
        game.best_streak = 4;
        game.streak_timer = 2.0;
        game.near_misses = 2;
        game.nitro = 7.0;
        game.boosting = true;
        game.last_event = Some(RaceEvent::NearMiss {
            points: 1_500,
            streak: 3,
        });
        game.event_timer = 0.5;
        game.refresh_score();
        let high_score = game.high_score;
        game.state = GameState::GameOver;
        game.handle_action(GameAction::Restart);

        assert_eq!(game.state, GameState::Playing);
        assert_eq!(game.distance, 0.0);
        assert_eq!(game.run_time(), 0.0);
        assert_eq!(game.score, 0);
        assert_eq!(game.traffic_score, 0);
        assert_eq!(game.passed, 0);
        assert_eq!(game.streak(), 0);
        assert_eq!(game.best_streak(), 0);
        assert_eq!(game.near_misses(), 0);
        assert_eq!(game.nitro(), NITRO_START);
        assert!(!game.boosting());
        assert_eq!(game.last_event(), None);
        assert_eq!(game.event_time_remaining(), 0.0);
        assert_eq!(game.level, 1);
        assert_eq!(game.spawn_timer, INITIAL_SPAWN_DELAY);
        assert!(game.vehicles.is_empty());
        assert_eq!(game.high_score, high_score);
        assert_eq!(game.player, centered_player(&game.road));
    }

    #[test]
    fn pause_prevents_all_simulation_updates() {
        let mut game = started_game(6);
        game.vehicles.push(Vehicle::new(
            game.road.lane_x_for(0, SPRITE_WIDTH) as f32,
            5.0,
            1.0,
            VehicleKind::Truck,
        ));
        game.handle_action(GameAction::PauseToggle);
        let simulation_snapshot = (
            game.player.clone(),
            game.vehicles.clone(),
            game.speed,
            game.distance,
            game.run_time,
            game.score,
            game.road_phase,
            game.spawn_timer,
        );
        let engagement_snapshot = (
            game.traffic_score,
            game.streak,
            game.best_streak,
            game.streak_timer,
            game.near_misses,
            game.nitro,
            game.boosting,
            game.last_event,
            game.event_timer,
        );

        game.update(
            10.0,
            ControlInput {
                right: true,
                accelerate: true,
                ..ControlInput::default()
            },
        );
        assert_eq!(
            simulation_snapshot,
            (
                game.player.clone(),
                game.vehicles.clone(),
                game.speed,
                game.distance,
                game.run_time,
                game.score,
                game.road_phase,
                game.spawn_timer,
            )
        );
        assert_eq!(
            engagement_snapshot,
            (
                game.traffic_score,
                game.streak,
                game.best_streak,
                game.streak_timer,
                game.near_misses,
                game.nitro,
                game.boosting,
                game.last_event,
                game.event_timer,
            )
        );
    }

    #[test]
    fn run_time_uses_clamped_playing_delta_only() {
        let mut game = Game::new_seeded(80, 30, 66);

        game.update(0.08, ControlInput::default());
        assert_eq!(game.run_time(), 0.0);

        game.handle_action(GameAction::Start);
        game.spawn_timer = 100.0;
        game.update(10.0, ControlInput::default());
        assert!((game.run_time() - 0.1).abs() < f32::EPSILON);

        game.update(-1.0, ControlInput::default());
        game.update(f32::NAN, ControlInput::default());
        assert!((game.run_time() - 0.1).abs() < f32::EPSILON);

        game.handle_action(GameAction::PauseToggle);
        game.update(0.1, ControlInput::default());
        assert!((game.run_time() - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn seeded_spawning_stays_valid_across_supported_sizes() {
        for (width, height) in [(60, 24), (80, 30), (160, 50)] {
            for seed in 0..32 {
                let mut game = started_game(seed);
                game.resize(width, height);
                game.spawn_timer = 0.0;
                game.update(1.0 / 60.0, ControlInput::default());

                for vehicle in &game.vehicles {
                    assert!(vehicle.rect().left() >= game.road.min_x_for(SPRITE_WIDTH));
                    assert!(vehicle.rect().right() <= game.road.right);
                }
                for (index, vehicle) in game.vehicles.iter().enumerate() {
                    assert!(
                        game.vehicles[..index]
                            .iter()
                            .all(|other| !vehicle.rect().overlaps(other.rect()))
                    );
                }
            }
        }
    }

    #[test]
    fn acceleration_braking_and_speed_caps_are_gradual() {
        let mut game = started_game(9);
        game.spawn_timer = 100.0;
        let initial = game.speed;
        game.update(
            0.1,
            ControlInput {
                accelerate: true,
                ..ControlInput::default()
            },
        );
        assert!(game.speed > initial && game.speed < MAX_SPEED);

        game.speed = MAX_SPEED;
        game.update(
            0.1,
            ControlInput {
                accelerate: true,
                ..ControlInput::default()
            },
        );
        assert_eq!(game.speed, MAX_SPEED);

        game.speed = MIN_SPEED;
        game.update(
            0.1,
            ControlInput {
                brake: true,
                accelerate: true,
                ..ControlInput::default()
            },
        );
        assert_eq!(game.speed, MIN_SPEED);
    }

    #[test]
    fn nitro_drains_recharges_caps_and_brake_takes_precedence() {
        let mut game = started_game(60);
        game.spawn_timer = 100.0;
        game.speed = 100.0;
        game.nitro = 50.0;

        game.update(
            0.1,
            ControlInput {
                boost: true,
                ..ControlInput::default()
            },
        );
        assert!(game.boosting());
        assert!((game.speed() - 109.5).abs() < 0.000_1);
        assert!((game.nitro() - 46.5).abs() < 0.000_1);

        game.speed = 100.0;
        game.nitro = 50.0;
        game.update(
            0.1,
            ControlInput {
                brake: true,
                boost: true,
                ..ControlInput::default()
            },
        );
        assert!(!game.boosting());
        assert!((game.speed() - 92.0).abs() < 0.000_1);
        assert_eq!(game.nitro(), 50.0);

        game.nitro = 0.0;
        game.update(
            0.1,
            ControlInput {
                boost: true,
                ..ControlInput::default()
            },
        );
        assert!(!game.boosting());
        assert_eq!(game.nitro(), 0.0);

        game.update(0.1, ControlInput::default());
        assert!((game.nitro() - 0.6).abs() < 0.000_1);

        game.speed = BOOST_MAX_SPEED - 1.0;
        game.nitro = NITRO_MAX;
        game.update(
            0.1,
            ControlInput {
                boost: true,
                ..ControlInput::default()
            },
        );
        assert_eq!(game.speed(), BOOST_MAX_SPEED);
        assert!(game.nitro() < NITRO_MAX);
    }

    #[test]
    fn nearly_empty_nitro_only_boosts_for_the_available_fraction_of_a_tick() {
        let mut game = started_game(63);
        game.spawn_timer = 100.0;
        game.speed = 100.0;
        game.nitro = 0.35; // One hundredth of a second at the configured drain rate.

        game.update(
            0.1,
            ControlInput {
                boost: true,
                ..ControlInput::default()
            },
        );

        // 0.01 s of boost (+0.95) followed by 0.09 s of coasting (-0.72).
        assert!((game.speed() - 100.23).abs() < 0.000_1);
        assert!(game.nitro() < 0.000_1);
        assert!(game.boosting());

        game.update(
            0.1,
            ControlInput {
                boost: true,
                ..ControlInput::default()
            },
        );
        assert!(!game.boosting());
    }

    #[test]
    fn pausing_clears_active_boost_feedback() {
        let mut game = started_game(64);
        game.boosting = true;

        game.handle_action(GameAction::PauseToggle);

        assert_eq!(game.state(), GameState::Paused);
        assert!(!game.boosting());
    }

    #[test]
    fn overspeed_falls_smoothly_to_the_normal_cap_without_boost() {
        let mut game = started_game(61);
        game.spawn_timer = 100.0;
        game.speed = 200.0;

        game.update(
            0.1,
            ControlInput {
                accelerate: true,
                ..ControlInput::default()
            },
        );
        assert!((game.speed() - 196.5).abs() < 0.000_1);

        for _ in 0..100 {
            game.update(
                0.1,
                ControlInput {
                    accelerate: true,
                    ..ControlInput::default()
                },
            );
        }
        assert_eq!(game.speed(), MAX_SPEED);
    }

    #[test]
    fn crossing_a_distance_threshold_emits_a_level_event() {
        let mut game = started_game(62);
        game.spawn_timer = 100.0;
        game.distance = DISTANCE_PER_LEVEL - 0.1;

        game.update(0.1, ControlInput::default());

        assert_eq!(game.level(), 2);
        assert_eq!(game.last_event(), Some(RaceEvent::LevelUp { level: 2 }));
        assert_eq!(game.event_time_remaining(), LEVEL_EVENT_SECONDS);
    }

    #[test]
    fn resize_rebuilds_layout_and_removes_unsafe_traffic() {
        let mut game = started_game(10);
        game.vehicles.push(Vehicle::new(
            game.player.x,
            game.player.y,
            1.0,
            VehicleKind::Sedan,
        ));
        game.resize(60, 24);

        assert!(game.vehicles.is_empty());
        assert_eq!(game.spawn_timer, RESIZE_SPAWN_GRACE);
        assert!(game.player.rect().left() >= game.road.min_x_for(SPRITE_WIDTH));
        assert!(game.player.rect().right() <= game.road.right);
        assert_eq!(game.player.render_y(), game.road.player_y(SPRITE_HEIGHT));
    }

    #[test]
    fn spawn_rejects_gap_that_current_steering_momentum_cannot_reach() {
        let mut game = started_game(12);
        game.resize(60, 24);
        game.speed = MAX_SPEED;
        game.distance = DISTANCE_PER_LEVEL * f32::from(MAX_LEVEL - 1);
        game.level = MAX_LEVEL;
        game.player.x = 13.0;
        game.player.lateral_velocity = -26.4;
        game.vehicles
            .push(Vehicle::new(12.0, 7.0, 0.82, VehicleKind::Sedan));
        let candidate = Vehicle::new(21.0, 4.0, 1.18, VehicleKind::Truck);

        assert!(is_valid_enemy_spawn(&game.road, &game.vehicles, &candidate));
        assert!(!game.spawn_keeps_reachable_gap(&candidate, game.scroll_speed()));
    }

    #[test]
    fn normal_top_speed_still_spawns_rookie_traffic_at_minimum_height() {
        let mut game = started_game(21);
        game.resize(60, 24);
        game.speed = MAX_SPEED;
        game.distance = 0.0;
        game.level = 1;

        assert!(game.try_spawn_vehicle(game.scroll_speed()));
        assert_eq!(game.vehicles.len(), 1);
        assert_eq!(
            game.vehicles[0].render_y(),
            game.road.top - i32::from(SPRITE_HEIGHT)
        );
    }
}
