//! Terminal-independent game entities and responsive road geometry.

use crate::collision::Rect;

pub const SPRITE_WIDTH: u16 = 7;
pub const SPRITE_HEIGHT: u16 = 5;
const COMPACT_LAYOUT_WIDTH: u16 = 80;
const STANDARD_LAYOUT_HEIGHT: u16 = 28;

pub type Sprite = [&'static str; SPRITE_HEIGHT as usize];

// Restrained seven-by-five line art leaves the negative space that makes a
// top-down vehicle read as machinery instead of a filled icon.
const PLAYER_SPRITE: Sprite = ["/--^--\\", "/o_|_o\\", "|/---\\|", "O\\_|_/O", "'\\v*v/'"];
const SEDAN_SPRITE: Sprite = [".-----.", "/o___o\\", "|/---\\|", "O|   |O", "'\\v_v/'"];
const SPORTS_SPRITE: Sprite = [".--^--.", "/o___o\\", "|/---\\|", "O\\   /O", "'\\v-v/'"];
const TRUCK_SPRITE: Sprite = ["+-----+", "|o___o|", "O|---|O", "O|===|O", "|v___v|"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehicleKind {
    Sedan,
    Sports,
    Truck,
}

impl VehicleKind {
    pub const ALL: [Self; 3] = [Self::Sedan, Self::Sports, Self::Truck];

    #[must_use]
    pub const fn sprite(self) -> &'static Sprite {
        match self {
            Self::Sedan => &SEDAN_SPRITE,
            Self::Sports => &SPORTS_SPRITE,
            Self::Truck => &TRUCK_SPRITE,
        }
    }
}

/// The responsive road layout, expressed in terminal cells.
///
/// `left` and `right` are the visible boundary columns. Drivable cells lie
/// strictly between them. The top and bottom margins reserve responsive
/// telemetry rails outside the active playfield.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Road {
    pub left: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub lane_count: u8,
}

impl Road {
    #[must_use]
    pub fn new(width: u16, height: u16) -> Self {
        let terminal_width = i32::from(width);
        let target_inner_width = if width < COMPACT_LAYOUT_WIDTH {
            40
        } else if width < 100 {
            48
        } else {
            56
        };
        let available_width = terminal_width.saturating_sub(2).max(3);
        let road_width = available_width.min(target_inner_width + 2);
        let left = terminal_width.saturating_sub(road_width).max(0) / 2;
        let right = if terminal_width == 0 {
            -1
        } else {
            (left + road_width - 1).min(terminal_width - 1)
        };
        let inner_width = (right - left - 1).max(0);
        let lane_count = if inner_width >= 36 {
            4
        } else if inner_width >= 18 {
            3
        } else if inner_width >= 10 {
            2
        } else {
            1
        };

        let top = if width < COMPACT_LAYOUT_WIDTH { 3 } else { 2 };
        let footer_rows = if height < STANDARD_LAYOUT_HEIGHT {
            2
        } else {
            3
        };
        let bottom = i32::from(height).saturating_sub(footer_rows + 1).max(top);

        Self {
            left,
            right,
            top,
            bottom,
            lane_count,
        }
    }

    #[must_use]
    pub fn inner_width(&self) -> i32 {
        (self.right - self.left - 1).max(0)
    }

    #[must_use]
    pub const fn min_x_for(&self, _entity_width: u16) -> i32 {
        self.left.saturating_add(1)
    }

    #[must_use]
    pub fn max_x_for(&self, entity_width: u16) -> i32 {
        let minimum = self.min_x_for(entity_width);
        self.right
            .saturating_sub(i32::from(entity_width))
            .max(minimum)
    }

    /// Returns the center cell for a zero-based lane index.
    #[must_use]
    pub fn lane_center_x(&self, lane: u8) -> i32 {
        let lanes = i32::from(self.lane_count.max(1));
        let lane = i32::from(lane.min(self.lane_count.saturating_sub(1)));
        let doubled_lane = lane.saturating_mul(2).saturating_add(1);

        self.left.saturating_add(1).saturating_add(
            self.inner_width()
                .saturating_mul(doubled_lane)
                .saturating_div(lanes.saturating_mul(2)),
        )
    }

    /// Returns the divider column at `index` (normally `1..lane_count`).
    #[must_use]
    pub fn lane_marker_x(&self, index: u8) -> i32 {
        let lanes = i32::from(self.lane_count.max(1));
        let index = i32::from(index.min(self.lane_count));

        self.left.saturating_add(1).saturating_add(
            self.inner_width()
                .saturating_mul(index)
                .saturating_div(lanes),
        )
    }

    #[must_use]
    pub fn lane_x_for(&self, lane: u8, entity_width: u16) -> i32 {
        let centered = self
            .lane_center_x(lane)
            .saturating_sub(i32::from(entity_width) / 2);
        centered.clamp(self.min_x_for(entity_width), self.max_x_for(entity_width))
    }

    #[must_use]
    pub fn centered_x_for(&self, entity_width: u16) -> i32 {
        let centered = self
            .left
            .saturating_add(self.right)
            .saturating_sub(i32::from(entity_width))
            .saturating_add(1)
            / 2;
        centered.clamp(self.min_x_for(entity_width), self.max_x_for(entity_width))
    }

    #[must_use]
    pub fn clamp_x(&self, x: f32, entity_width: u16) -> f32 {
        x.clamp(
            self.min_x_for(entity_width) as f32,
            self.max_x_for(entity_width) as f32,
        )
    }

    #[must_use]
    pub fn player_y(&self, entity_height: u16) -> i32 {
        self.bottom
            .saturating_sub(i32::from(entity_height))
            .saturating_sub(2)
            .max(self.top)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) lateral_velocity: f32,
}

impl Player {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            lateral_velocity: 0.0,
        }
    }

    #[must_use]
    pub fn render_x(&self) -> i32 {
        rounded_cell(self.x)
    }

    #[must_use]
    pub fn render_y(&self) -> i32 {
        rounded_cell(self.y)
    }

    #[must_use]
    pub const fn sprite(&self) -> &'static Sprite {
        &PLAYER_SPRITE
    }

    #[must_use]
    pub fn rect(&self) -> Rect {
        Rect::new(
            self.render_x(),
            self.render_y(),
            SPRITE_WIDTH,
            SPRITE_HEIGHT,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Vehicle {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) downward_speed: f32,
    pub(crate) kind: VehicleKind,
    pub(crate) passed: bool,
}

impl Vehicle {
    #[must_use]
    pub const fn new(x: f32, y: f32, downward_speed: f32, kind: VehicleKind) -> Self {
        Self {
            x,
            y,
            downward_speed,
            kind,
            passed: false,
        }
    }

    #[must_use]
    pub fn render_x(&self) -> i32 {
        rounded_cell(self.x)
    }

    #[must_use]
    pub fn render_y(&self) -> i32 {
        rounded_cell(self.y)
    }

    #[must_use]
    pub const fn sprite(&self) -> &'static Sprite {
        self.kind.sprite()
    }

    #[must_use]
    pub fn rect(&self) -> Rect {
        Rect::new(
            self.render_x(),
            self.render_y(),
            SPRITE_WIDTH,
            SPRITE_HEIGHT,
        )
    }
}

#[must_use]
fn rounded_cell(value: f32) -> i32 {
    value.round() as i32
}

#[cfg(test)]
mod tests {
    use super::{Player, Road, SPRITE_HEIGHT, SPRITE_WIDTH, Sprite, Vehicle, VehicleKind};

    fn assert_fixed_size(sprite: &Sprite) {
        assert_eq!(sprite.len(), usize::from(SPRITE_HEIGHT));
        for row in sprite {
            assert!(row.is_ascii());
            assert_eq!(row.len(), usize::from(SPRITE_WIDTH));
        }
    }

    #[test]
    fn every_sprite_has_the_declared_fixed_dimensions() {
        assert_fixed_size(Player::new(0.0, 0.0).sprite());
        for kind in VehicleKind::ALL {
            assert_fixed_size(Vehicle::new(0.0, 0.0, 1.0, kind).sprite());
        }
    }

    #[test]
    fn every_vehicle_has_a_complete_recognizable_silhouette() {
        let player = Player::new(0.0, 0.0);
        for sprite in std::iter::once(player.sprite())
            .chain(VehicleKind::ALL.iter().map(|kind| kind.sprite()))
        {
            assert!(
                sprite
                    .iter()
                    .all(|row| { !row.starts_with(' ') && !row.ends_with(' ') })
            );
            let art: String = sprite.concat();
            assert_eq!(art.matches('o').count(), 2);
            assert_eq!(art.matches('v').count(), 2);
            assert!(art.matches('O').count() >= 2);
        }
    }

    #[test]
    fn vehicle_variants_and_player_are_visually_distinct() {
        let player = Player::new(0.0, 0.0);
        assert_ne!(player.sprite(), VehicleKind::Sedan.sprite());
        assert_ne!(VehicleKind::Sedan.sprite(), VehicleKind::Sports.sprite());
        assert_ne!(VehicleKind::Sports.sprite(), VehicleKind::Truck.sprite());
        assert_ne!(VehicleKind::Truck.sprite(), VehicleKind::Sedan.sprite());
    }

    #[test]
    fn render_position_and_hitbox_use_identical_rounding() {
        let player = Player::new(10.5, 7.49);
        let vehicle = Vehicle::new(12.49, 9.5, 8.0, VehicleKind::Sedan);

        assert_eq!((player.render_x(), player.render_y()), (11, 7));
        assert_eq!((player.rect().x, player.rect().y), (11, 7));
        assert_eq!((vehicle.render_x(), vehicle.render_y()), (12, 10));
        assert_eq!((vehicle.rect().x, vehicle.rect().y), (12, 10));
    }

    #[test]
    fn multi_character_sprite_hitboxes_collide_as_rendered() {
        let player = Player::new(10.0, 12.0);
        let overlapping = Vehicle::new(
            10.0 + f32::from(SPRITE_WIDTH - 1),
            12.0 + f32::from(SPRITE_HEIGHT - 1),
            7.0,
            VehicleKind::Sports,
        );
        let touching = Vehicle::new(
            10.0 + f32::from(SPRITE_WIDTH),
            12.0,
            7.0,
            VehicleKind::Truck,
        );

        assert!(player.rect().overlaps(overlapping.rect()));
        assert!(!player.rect().overlaps(touching.rect()));
    }

    #[test]
    fn responsive_road_reserves_telemetry_rails() {
        let compact = Road::new(60, 24);
        assert_eq!((compact.left, compact.right), (9, 50));
        assert_eq!((compact.top, compact.bottom), (3, 21));
        assert_eq!(compact.inner_width(), 40);

        let standard = Road::new(80, 30);
        assert_eq!((standard.left, standard.right), (15, 64));
        assert_eq!((standard.top, standard.bottom), (2, 26));
        assert_eq!(standard.inner_width(), 48);
        assert_eq!(standard.lane_count, 4);

        let wide = Road::new(120, 40);
        assert_eq!((wide.left, wide.right), (31, 88));
        assert_eq!((wide.top, wide.bottom), (2, 36));
        assert_eq!(wide.inner_width(), 56);
    }

    #[test]
    fn lane_positions_fit_wholly_inside_the_road() {
        for width in [36, 50, 60, 80, 120] {
            let road = Road::new(width, 24);
            for lane in 0..road.lane_count {
                let x = road.lane_x_for(lane, SPRITE_WIDTH);
                assert!(x >= road.min_x_for(SPRITE_WIDTH));
                assert!(x <= road.max_x_for(SPRITE_WIDTH));
                assert!(x + i32::from(SPRITE_WIDTH) <= road.right);
            }
        }
    }

    #[test]
    fn lane_markers_are_ordered_and_inside_boundaries() {
        let road = Road::new(80, 24);
        let mut previous = road.left;
        for index in 1..road.lane_count {
            let marker = road.lane_marker_x(index);
            assert!(marker > previous);
            assert!(marker > road.left && marker < road.right);
            previous = marker;
        }
    }

    #[test]
    fn road_clamps_player_to_both_boundaries() {
        let road = Road::new(50, 24);
        let minimum = road.min_x_for(SPRITE_WIDTH) as f32;
        let maximum = road.max_x_for(SPRITE_WIDTH) as f32;

        assert_eq!(road.clamp_x(-1_000.0, SPRITE_WIDTH), minimum);
        assert_eq!(road.clamp_x(1_000.0, SPRITE_WIDTH), maximum);
        assert_eq!(road.clamp_x(minimum + 2.0, SPRITE_WIDTH), minimum + 2.0);
    }

    #[test]
    fn centered_player_position_is_valid_on_supported_sizes() {
        for (width, height) in [(36, 22), (80, 24), (160, 50)] {
            let road = Road::new(width, height);
            let x = road.centered_x_for(SPRITE_WIDTH);
            let y = road.player_y(SPRITE_HEIGHT);

            assert!(x >= road.min_x_for(SPRITE_WIDTH));
            assert!(x <= road.max_x_for(SPRITE_WIDTH));
            assert!(y >= road.top);
            assert!(y + i32::from(SPRITE_HEIGHT) <= road.bottom);
        }
    }

    #[test]
    fn supported_layouts_leave_room_for_a_three_row_boost_trail() {
        for (width, height) in [(60, 24), (80, 30), (120, 40)] {
            let road = Road::new(width, height);
            let y = road.player_y(SPRITE_HEIGHT);
            assert!(y + i32::from(SPRITE_HEIGHT) + 2 <= road.bottom);
        }
    }

    #[test]
    fn tiny_terminal_geometry_never_panics_or_reverses_clamp_range() {
        for width in 0..10 {
            let road = Road::new(width, 4);
            let minimum = road.min_x_for(SPRITE_WIDTH);
            let maximum = road.max_x_for(SPRITE_WIDTH);

            assert!(maximum >= minimum);
            let _ = road.clamp_x(0.0, SPRITE_WIDTH);
            let _ = road.lane_center_x(200);
            let _ = road.lane_marker_x(200);
        }
    }
}
