use std::io;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
};

use crate::{
    entities::{Road, SPRITE_HEIGHT, SPRITE_WIDTH, Sprite, VehicleKind},
    game::{Game, GameState, RaceEvent},
};

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 24;

const CANVAS: Color = Color::Rgb { r: 5, g: 7, b: 7 };
const ASPHALT: Color = Color::Rgb { r: 7, g: 10, b: 11 };
const ASPHALT_GRAIN: Color = Color::Rgb {
    r: 16,
    g: 23,
    b: 25,
};
const PRIMARY: Color = Color::Rgb {
    r: 232,
    g: 236,
    b: 235,
};
const LANE: Color = Color::Rgb {
    r: 215,
    g: 221,
    b: 220,
};
const MUTED: Color = Color::Rgb {
    r: 92,
    g: 102,
    b: 104,
};
const CYAN: Color = Color::Rgb {
    r: 103,
    g: 220,
    b: 232,
};
const RED: Color = Color::Rgb {
    r: 255,
    g: 59,
    b: 48,
};
const AMBER: Color = Color::Rgb {
    r: 246,
    g: 185,
    b: 74,
};
const CITY_NEAR: Color = Color::Rgb {
    r: 167,
    g: 155,
    b: 131,
};
const CITY_FAR: Color = Color::Rgb {
    r: 80,
    g: 76,
    b: 67,
};
const GARDEN: Color = Color::Rgb {
    r: 92,
    g: 138,
    b: 112,
};

const DEFAULT_STYLE: Style = Style::new(PRIMARY, CANVAS, false);
const ROAD_STYLE: Style = Style::new(ASPHALT_GRAIN, ASPHALT, false);
const ROAD_TEXTURE_STYLE: Style = Style::new(ASPHALT_GRAIN, ASPHALT, false);
const BOUNDARY_STYLE: Style = Style::new(LANE, ASPHALT, false);
const MARKER_STYLE: Style = Style::new(LANE, ASPHALT, false);
const HUD_STYLE: Style = Style::new(CYAN, CANVAS, false);
const MUTED_STYLE: Style = Style::new(MUTED, CANVAS, false);
const PLAYER_STYLE: Style = Style::new(RED, ASPHALT, true);
const BOOST_STYLE: Style = Style::new(CYAN, ASPHALT, true);
const AMBER_STYLE: Style = Style::new(AMBER, CANVAS, true);
const VERGE_STYLE: Style = Style::new(CITY_FAR, CANVAS, false);
const CITY_DOT_STYLE: Style = Style::new(CITY_FAR, CANVAS, false);
const SIDEWALK_STYLE: Style = Style::new(MUTED, CANVAS, false);

const SCENERY_WIDTH: i32 = 6;
const SCENERY_HEIGHT: i32 = 4;
const SCENERY_SLOT_HEIGHT: i32 = 7;

type ScenerySprite = [&'static str; SCENERY_HEIGHT as usize];

const EMPTY_SCENERY: ScenerySprite = ["      ", "      ", "      ", "      "];
const HOUSE_SCENERY: ScenerySprite = [" /\\   ", "/--\\  ", "|[]|  ", "|__|  "];
const SHOP_SCENERY: ScenerySprite = [".----.", "|.[] |", "| ===|", "'----'"];
const GARDEN_SCENERY: ScenerySprite = [" . * .", " .|. .", "  |   ", "......"];
const APARTMENT_SCENERY: ScenerySprite = ["+----+", "|.[] |", "| [] |", "|_||_|"];
const SPECTATOR_SCENERY: ScenerySprite = [" o o  ", "/|\\|\\ ", "/\\/\\  ", "..... "];
const TREE_SCENERY: ScenerySprite = [" /\\   ", "/  \\  ", " ||   ", " ..   "];
const SIGN_SCENERY: ScenerySprite = [".2KM .", "'----'", "  ||  ", "  ||  "];
const PARK_SCENERY: ScenerySprite = ["  __  ", " /__\\ ", "   |  ", ".. |.."];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SceneryKind {
    Empty,
    House,
    Shop,
    Garden,
    Apartment,
    Spectators,
    Trees,
    Sign,
    Park,
}

impl SceneryKind {
    const fn sprite(self) -> &'static ScenerySprite {
        match self {
            Self::Empty => &EMPTY_SCENERY,
            Self::House => &HOUSE_SCENERY,
            Self::Shop => &SHOP_SCENERY,
            Self::Garden => &GARDEN_SCENERY,
            Self::Apartment => &APARTMENT_SCENERY,
            Self::Spectators => &SPECTATOR_SCENERY,
            Self::Trees => &TREE_SCENERY,
            Self::Sign => &SIGN_SCENERY,
            Self::Park => &PARK_SCENERY,
        }
    }
}

const LEFT_SCENERY: [SceneryKind; 12] = [
    SceneryKind::House,
    SceneryKind::Empty,
    SceneryKind::Trees,
    SceneryKind::Empty,
    SceneryKind::Garden,
    SceneryKind::Spectators,
    SceneryKind::Empty,
    SceneryKind::Apartment,
    SceneryKind::Empty,
    SceneryKind::Sign,
    SceneryKind::Trees,
    SceneryKind::Empty,
];

const RIGHT_SCENERY: [SceneryKind; 12] = [
    SceneryKind::Empty,
    SceneryKind::Trees,
    SceneryKind::Shop,
    SceneryKind::Empty,
    SceneryKind::Park,
    SceneryKind::Empty,
    SceneryKind::House,
    SceneryKind::Trees,
    SceneryKind::Empty,
    SceneryKind::Garden,
    SceneryKind::Sign,
    SceneryKind::Empty,
];

const DISTANT_SCENERY: [SceneryKind; 6] = [
    SceneryKind::Apartment,
    SceneryKind::Empty,
    SceneryKind::Empty,
    SceneryKind::House,
    SceneryKind::Empty,
    SceneryKind::Empty,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Style {
    foreground: Color,
    background: Color,
    bold: bool,
}

impl Style {
    const fn new(foreground: Color, background: Color, bold: bool) -> Self {
        Self {
            foreground,
            background,
            bold,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Cell {
    character: char,
    style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            style: DEFAULT_STYLE,
        }
    }
}

#[derive(Default)]
struct FrameBuffer {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl FrameBuffer {
    fn resize(&mut self, width: u16, height: u16) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;
        self.cells = vec![Cell::default(); usize::from(width) * usize::from(height)];
    }

    fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    fn put(&mut self, x: i32, y: i32, character: char, style: Style) {
        if x < 0 || y < 0 {
            return;
        }

        let Ok(x) = u16::try_from(x) else {
            return;
        };
        let Ok(y) = u16::try_from(y) else {
            return;
        };

        if x >= self.width || y >= self.height {
            return;
        }

        let index = usize::from(y) * usize::from(self.width) + usize::from(x);
        self.cells[index] = Cell { character, style };
    }

    fn text(&mut self, x: i32, y: i32, text: &str, style: Style) {
        for (offset, character) in text.chars().enumerate() {
            let Ok(offset) = i32::try_from(offset) else {
                break;
            };
            self.put(x.saturating_add(offset), y, character, style);
        }
    }

    fn centered_text(&mut self, y: i32, text: &str, style: Style) {
        let length = i32::try_from(text.chars().count()).unwrap_or(i32::MAX);
        let x = (i32::from(self.width) - length).max(0) / 2;
        self.text(x, y, text, style);
    }

    fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, style: Style) {
        for row in 0..height.max(0) {
            for column in 0..width.max(0) {
                self.put(x.saturating_add(column), y.saturating_add(row), ' ', style);
            }
        }
    }

    fn draw_box(&mut self, x: i32, y: i32, width: i32, height: i32, style: Style) {
        if width < 2 || height < 2 {
            return;
        }

        self.fill_rect(x, y, width, height, Style::new(PRIMARY, CANVAS, false));
        self.put(x, y, '+', style);
        self.put(x + width - 1, y, '+', style);
        self.put(x, y + height - 1, '+', style);
        self.put(x + width - 1, y + height - 1, '+', style);

        for column in 1..(width - 1) {
            self.put(x + column, y, '-', style);
            self.put(x + column, y + height - 1, '-', style);
        }
        for row in 1..(height - 1) {
            self.put(x, y + row, '|', style);
            self.put(x + width - 1, y + row, '|', style);
        }
    }
}

/// Converts immutable game state into a complete, in-memory terminal frame.
pub struct Renderer {
    frame: FrameBuffer,
    output: Vec<u8>,
    text_run: String,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            frame: FrameBuffer::default(),
            output: Vec::with_capacity(64 * 1024),
            text_run: String::with_capacity(256),
        }
    }

    pub fn render(&mut self, game: &Game, width: u16, height: u16) -> io::Result<&[u8]> {
        self.frame.resize(width, height);
        self.frame.clear();

        if width < MIN_WIDTH || height < MIN_HEIGHT {
            self.draw_size_warning(width, height);
        } else {
            match game.state() {
                GameState::Title => self.draw_title(game),
                GameState::Playing | GameState::Paused | GameState::GameOver => {
                    self.draw_race(game);
                    match game.state() {
                        GameState::Paused => self.draw_pause_overlay(),
                        GameState::GameOver => self.draw_game_over_overlay(game),
                        _ => {}
                    }
                }
                GameState::Quit => {}
            }
        }

        self.serialize()?;
        Ok(&self.output)
    }

    fn draw_size_warning(&mut self, width: u16, height: u16) {
        let center_y = i32::from(height) / 2;
        self.frame.centered_text(
            center_y.saturating_sub(1),
            "TERMINAL TOO SMALL",
            Style::new(RED, CANVAS, true),
        );
        self.frame.centered_text(
            center_y,
            &format!("Need at least {MIN_WIDTH} x {MIN_HEIGHT}"),
            DEFAULT_STYLE,
        );
        self.frame.centered_text(
            center_y.saturating_add(1),
            &format!("Current size: {width} x {height}"),
            MUTED_STYLE,
        );
        self.frame.centered_text(
            center_y.saturating_add(3),
            "Resize the terminal, or press Q / Esc to quit",
            DEFAULT_STYLE,
        );
    }

    fn draw_title(&mut self, game: &Game) {
        let road = game.road();
        self.draw_roadside(road, 0);
        self.draw_track(road, 0, false, 0.0);

        self.frame
            .text(1, 0, "ASCII APEX", Style::new(RED, CANVAS, true));
        let right_label = "NIGHT // 01";
        self.frame.text(
            i32::from(self.frame.width) - i32::try_from(right_label.len()).unwrap_or(0) - 1,
            0,
            right_label,
            MUTED_STYLE,
        );
        self.draw_rule(road.top - 1);

        let preview_y = road.top + 1;
        self.draw_vehicle_sprite(
            road.centered_x_for(SPRITE_WIDTH),
            preview_y,
            game.player().sprite(),
            PLAYER_STYLE,
            road,
        );

        self.frame.centered_text(
            preview_y + i32::from(SPRITE_HEIGHT) + 1,
            "N I G H T   E N D U R A N C E",
            Style::new(PRIMARY, ASPHALT, false),
        );
        self.frame.centered_text(
            preview_y + i32::from(SPRITE_HEIGHT) + 3,
            &format!("SESSION BEST  {:07}", game.high_score()),
            Style::new(CYAN, ASPHALT, false),
        );
        self.frame.centered_text(
            preview_y + i32::from(SPRITE_HEIGHT) + 5,
            "[ ENTER ]  START ENGINE",
            Style::new(RED, ASPHALT, true),
        );

        let controls_y = road.bottom - 3;
        self.frame.centered_text(
            controls_y,
            "A / D  STEER    W / S  SPEED    SPACE  NITRO",
            Style::new(PRIMARY, ASPHALT, false),
        );
        self.frame.centered_text(
            controls_y + 1,
            "P  PAUSE    Q / ESC  EXIT",
            Style::new(MUTED, ASPHALT, false),
        );
        self.frame.centered_text(
            road.bottom,
            "CLOSE CALLS REFILL NITRO",
            Style::new(AMBER, ASPHALT, false),
        );

        self.draw_rule(road.bottom + 1);
        self.frame.centered_text(
            road.bottom + 2,
            "ENDLESS HIGHWAY // ONE MORE RUN",
            MUTED_STYLE,
        );
    }

    fn draw_race(&mut self, game: &Game) {
        let road = game.road();
        let phase = game.road_phase().floor() as i32;
        self.draw_roadside(road, phase);
        self.draw_track(road, phase, game.boosting(), game.speed());

        for vehicle in game.vehicles() {
            let style = match vehicle.kind {
                VehicleKind::Sedan => Style::new(CITY_NEAR, ASPHALT, false),
                VehicleKind::Sports => Style::new(PRIMARY, ASPHALT, false),
                VehicleKind::Truck => Style::new(MUTED, ASPHALT, false),
            };
            self.draw_vehicle_sprite(
                vehicle.render_x(),
                vehicle.render_y(),
                vehicle.sprite(),
                style,
                road,
            );
        }

        let player = game.player();
        if game.boosting() {
            let flame = Style::new(CYAN, ASPHALT, true);
            let exhaust_y = player.render_y() + i32::from(SPRITE_HEIGHT);
            self.frame.put(player.render_x() + 2, exhaust_y, ':', flame);
            self.frame.put(player.render_x() + 4, exhaust_y, ':', flame);
            self.frame
                .put(player.render_x() + 2, exhaust_y + 1, '|', BOOST_STYLE);
            self.frame
                .put(player.render_x() + 4, exhaust_y + 1, '|', BOOST_STYLE);
            self.frame
                .put(player.render_x() + 3, exhaust_y + 2, '.', BOOST_STYLE);
        }
        self.draw_vehicle_sprite(
            player.render_x(),
            player.render_y(),
            player.sprite(),
            PLAYER_STYLE,
            road,
        );

        if game.state() == GameState::GameOver {
            self.draw_crash_effect(game);
        } else if game.state() == GameState::Playing {
            self.draw_race_event(game);
        }

        self.draw_telemetry(game);
    }

    fn draw_track(&mut self, road: &Road, phase: i32, boosting: bool, speed: f32) {
        let dash_length = if boosting || speed >= 150.0 { 4 } else { 3 };
        for y in road.top..=road.bottom {
            let world_y = y - phase;
            for x in (road.left + 1)..road.right {
                self.frame.put(x, y, ' ', ROAD_STYLE);
                if (x * 5 + world_y * 7).rem_euclid(28) == 0 {
                    self.frame.put(x, y, '.', ROAD_TEXTURE_STYLE);
                }
            }

            self.frame.put(road.left, y, '|', BOUNDARY_STYLE);
            self.frame.put(road.right, y, '|', BOUNDARY_STYLE);

            for lane in 1..road.lane_count {
                if world_y.rem_euclid(7) < dash_length {
                    self.frame
                        .put(road.lane_marker_x(lane), y, '|', MARKER_STYLE);
                }
            }

            if boosting && world_y.rem_euclid(3) == 1 {
                self.frame.put(road.left + 2, y, ':', BOOST_STYLE);
                self.frame.put(road.right - 2, y, ':', BOOST_STYLE);
            }
        }
    }

    fn draw_telemetry(&mut self, game: &Game) {
        let road = game.road();
        let compact = road.top == 3;
        let speed = format!("{:03.0}", game.speed());
        let score = format!("{:07}", game.score());
        let high = format!("{:07}", game.high_score());
        let nitro = format!("{:03.0}", game.nitro());
        let filled = ((game.nitro() / 10.0).round() as usize).min(10);
        let tier = difficulty_name(game.level());

        if compact {
            self.frame
                .text(1, 0, "ASCII APEX", Style::new(RED, CANVAS, true));
            self.frame.text(16, 0, &speed, DEFAULT_STYLE);
            self.frame.text(20, 0, "KM/H", HUD_STYLE);
            self.frame.text(27, 0, "SCORE", DEFAULT_STYLE);
            self.frame.text(33, 0, &score, HUD_STYLE);
            self.frame.text(52, 0, "P", Style::new(RED, CANVAS, true));
            self.frame.text(54, 0, "PAUSE", DEFAULT_STYLE);

            self.frame.text(1, 1, "N2O", DEFAULT_STYLE);
            self.frame.text(5, 1, &nitro, HUD_STYLE);
            self.draw_meter(9, 1, filled);
            self.frame.text(23, 1, "STREAK", DEFAULT_STYLE);
            self.frame.text(
                30,
                1,
                &format!("x{:02}", game.streak()),
                Style::new(RED, CANVAS, true),
            );
            self.frame
                .text(35, 1, &format!("L{:02}", game.level()), DEFAULT_STYLE);
            self.frame.text(39, 1, tier, DEFAULT_STYLE);
            self.frame.text(47, 1, "D", MUTED_STYLE);
            self.frame
                .text(49, 1, &format_distance(game.distance()), HUD_STYLE);
            self.draw_rule(2);
        } else if self.frame.width < 100 {
            self.frame
                .text(1, 0, "ASCII APEX", Style::new(RED, CANVAS, true));
            self.frame.text(17, 0, &speed, DEFAULT_STYLE);
            self.frame.text(21, 0, "KM/H", HUD_STYLE);
            self.frame.text(28, 0, "SCORE", DEFAULT_STYLE);
            self.frame.text(34, 0, &score, HUD_STYLE);
            self.frame.text(44, 0, "N2O", DEFAULT_STYLE);
            self.frame.text(48, 0, &nitro, HUD_STYLE);
            self.draw_meter(52, 0, filled);
            self.frame.text(65, 0, "STK", DEFAULT_STYLE);
            self.frame.text(
                69,
                0,
                &format!("x{:02}", game.streak()),
                Style::new(RED, CANVAS, true),
            );
            self.frame.text(73, 0, "P", Style::new(RED, CANVAS, true));
            self.frame.text(75, 0, "PAUSE", DEFAULT_STYLE);
            self.draw_rule(1);
        } else {
            self.frame
                .text(2, 0, "ASCII APEX", Style::new(RED, CANVAS, true));
            self.frame.text(28, 0, &speed, DEFAULT_STYLE);
            self.frame.text(32, 0, "KM/H", HUD_STYLE);
            self.frame.text(44, 0, "SCORE", DEFAULT_STYLE);
            self.frame.text(50, 0, &score, HUD_STYLE);
            self.frame.text(62, 0, "NITRO", DEFAULT_STYLE);
            self.frame.text(68, 0, &nitro, HUD_STYLE);
            self.draw_meter(72, 0, filled);
            self.frame.text(91, 0, "STREAK", DEFAULT_STYLE);
            self.frame.text(
                98,
                0,
                &format!("x{:02}", game.streak()),
                Style::new(RED, CANVAS, true),
            );
            self.frame.text(109, 0, "P", Style::new(RED, CANVAS, true));
            self.frame.text(111, 0, "PAUSE", DEFAULT_STYLE);
            self.draw_rule(1);
        }

        let rule_y = road.bottom + 1;
        let data_y = road.bottom + 2;
        self.draw_rule(rule_y);
        let run_time = format_run_time(game.run_time());
        let distance = format_distance(game.distance());
        if compact {
            self.frame.text(1, data_y, "T", MUTED_STYLE);
            self.frame.text(3, data_y, &run_time, HUD_STYLE);
            self.frame.text(12, data_y, "D", MUTED_STYLE);
            self.frame.text(14, data_y, &distance, HUD_STYLE);
            self.frame.text(22, data_y, "HI", MUTED_STYLE);
            self.frame.text(25, data_y, &high, HUD_STYLE);
            self.frame.text(34, data_y, "P", MUTED_STYLE);
            self.frame
                .text(36, data_y, &format!("{:03}", game.passed()), DEFAULT_STYLE);
            self.frame.text(41, data_y, "N", MUTED_STYLE);
            self.frame.text(
                43,
                data_y,
                &format!("{:02}", game.near_misses()),
                DEFAULT_STYLE,
            );
            self.frame.text(47, data_y, "B", MUTED_STYLE);
            self.frame.text(
                49,
                data_y,
                &format!("x{:02}", game.best_streak()),
                DEFAULT_STYLE,
            );
            self.frame.text(54, data_y, "L", MUTED_STYLE);
            self.frame
                .text(56, data_y, &format!("{:02}", game.level()), AMBER_STYLE);
        } else if self.frame.width < 100 {
            self.frame.text(1, data_y, "TIME", MUTED_STYLE);
            self.frame.text(6, data_y, &run_time, HUD_STYLE);
            self.frame.text(14, data_y, "DIST", MUTED_STYLE);
            self.frame.text(19, data_y, &distance, HUD_STYLE);
            self.frame.text(27, data_y, "HIGH", MUTED_STYLE);
            self.frame.text(32, data_y, &high, HUD_STYLE);
            self.frame.text(
                41,
                data_y,
                &format!("L{:02} {tier}", game.level()),
                DEFAULT_STYLE,
            );
            self.frame.text(53, data_y, "P", MUTED_STYLE);
            self.frame
                .text(55, data_y, &format!("{:03}", game.passed()), DEFAULT_STYLE);
            self.frame.text(60, data_y, "N", MUTED_STYLE);
            self.frame.text(
                62,
                data_y,
                &format!("{:02}", game.near_misses()),
                DEFAULT_STYLE,
            );
            self.frame.text(66, data_y, "BEST", MUTED_STYLE);
            self.frame.text(
                71,
                data_y,
                &format!("x{:02}", game.best_streak()),
                DEFAULT_STYLE,
            );
        } else {
            self.frame.text(2, data_y, "TIME", MUTED_STYLE);
            self.frame.text(7, data_y, &run_time, HUD_STYLE);
            self.frame.text(18, data_y, "DIST", MUTED_STYLE);
            self.frame.text(23, data_y, &distance, HUD_STYLE);
            self.frame.text(34, data_y, "HIGH", MUTED_STYLE);
            self.frame.text(39, data_y, &high, HUD_STYLE);
            self.frame.text(50, data_y, "LEVEL", MUTED_STYLE);
            self.frame.text(
                56,
                data_y,
                &format!("{:02} {tier}", game.level()),
                AMBER_STYLE,
            );
            self.frame.text(72, data_y, "PASS", MUTED_STYLE);
            self.frame
                .text(77, data_y, &format!("{:03}", game.passed()), DEFAULT_STYLE);
            self.frame.text(85, data_y, "NEAR", MUTED_STYLE);
            self.frame.text(
                90,
                data_y,
                &format!("{:02}", game.near_misses()),
                DEFAULT_STYLE,
            );
            self.frame.text(98, data_y, "BEST STREAK", MUTED_STYLE);
            self.frame.text(
                110,
                data_y,
                &format!("x{:02}", game.best_streak()),
                DEFAULT_STYLE,
            );
        }

        let ruler_y = road.bottom + 3;
        if ruler_y < i32::from(self.frame.height) {
            for x in 0..i32::from(self.frame.width) {
                self.frame.put(
                    x,
                    ruler_y,
                    if x.rem_euclid(10) == 0 { '+' } else { '-' },
                    MUTED_STYLE,
                );
            }
        }
    }

    fn draw_race_event(&mut self, game: &Game) {
        if game.event_time_remaining() <= 0.0 {
            return;
        }
        let Some(event) = game.last_event() else {
            return;
        };
        let (message, foreground) = match event {
            RaceEvent::Pass { points, streak } => {
                (format!("CLEAN PASS +{points:04} // x{streak}"), CYAN)
            }
            RaceEvent::NearMiss { points, streak } => {
                (format!(">>> NEAR MISS +{points:04} // x{streak} <<<"), RED)
            }
            RaceEvent::LevelUp { level } => (
                format!("LEVEL {level:02} // {} TRAFFIC", difficulty_name(level)),
                AMBER,
            ),
        };
        let road = game.road();
        let message_width = i32::try_from(message.chars().count()).unwrap_or(i32::MAX);
        let x = road.left + 1 + (road.inner_width() - message_width).max(0) / 2;
        let y = road.top + (road.bottom - road.top) / 2;
        self.frame
            .text(x, y, &message, Style::new(foreground, ASPHALT, true));
    }

    fn draw_meter(&mut self, x: i32, y: i32, filled: usize) {
        self.frame.put(x, y, '[', MUTED_STYLE);
        for index in 0..10 {
            self.frame.put(
                x + 1 + i32::try_from(index).unwrap_or(0),
                y,
                if index < filled { '|' } else { '-' },
                if index < filled {
                    HUD_STYLE
                } else {
                    MUTED_STYLE
                },
            );
        }
        self.frame.put(x + 11, y, ']', MUTED_STYLE);
    }

    fn draw_rule(&mut self, y: i32) {
        for x in 0..i32::from(self.frame.width) {
            self.frame.put(x, y, '-', MUTED_STYLE);
        }
    }

    fn draw_roadside(&mut self, road: &Road, phase: i32) {
        let frame_width = i32::from(self.frame.width);
        let walkway_width = if road.left >= 25 {
            6
        } else if road.left >= 13 {
            4
        } else {
            2
        };

        for y in road.top..=road.bottom {
            let world_y = y - phase;
            for x in 0..road.left.max(0) {
                self.frame.put(x, y, ' ', VERGE_STYLE);
                if x < road.left - walkway_width && (x * 5 + world_y * 7).rem_euclid(28) == 0 {
                    self.frame.put(x, y, '.', CITY_DOT_STYLE);
                }
            }
            for x in road.right.saturating_add(1).max(0)..frame_width {
                self.frame.put(x, y, ' ', VERGE_STYLE);
                if x > road.right + walkway_width && (x * 5 + world_y * 7).rem_euclid(28) == 0 {
                    self.frame.put(x, y, '.', CITY_DOT_STYLE);
                }
            }

            for offset in 1..=walkway_width {
                let paving = if (world_y + offset).rem_euclid(7) == 0 {
                    '.'
                } else {
                    ' '
                };
                self.put_roadside(road, road.left - offset, y, paving, SIDEWALK_STYLE);
                self.put_roadside(road, road.right + offset, y, paving, SIDEWALK_STYLE);
            }

            let lamp = match world_y.rem_euclid(12) {
                0 => Some(('o', AMBER_STYLE)),
                1 | 2 => Some(('|', VERGE_STYLE)),
                _ => None,
            };
            if let Some((character, style)) = lamp {
                self.put_roadside(road, road.left - 1, y, character, style);
                self.put_roadside(road, road.right + 1, y, character, style);
            }
        }

        // Near-side modules sit beyond the quiet sidewalk rather than touching
        // the road edge. Empty sequence slots preserve the reference design's
        // deliberate negative space.
        let left_near_x = road.left - walkway_width - SCENERY_WIDTH;
        if left_near_x >= 0 {
            self.draw_scrolling_scenery(road, left_near_x, phase, 0, &LEFT_SCENERY, false);
        }
        let right_near_x = road.right + walkway_width + 1;
        if right_near_x + SCENERY_WIDTH <= frame_width {
            self.draw_scrolling_scenery(road, right_near_x, phase, 3, &RIGHT_SCENERY, false);
        }

        // Wider terminals gain a slower outer city layer. Its 42-row loop is
        // exactly half the road's 84-row animation period, so parallax wraps
        // without a visible jump.
        let distant_phase = phase.div_euclid(2);
        let mut layer = 0_i32;
        let mut left_outer_x = 1;
        while left_outer_x + SCENERY_WIDTH + 2 <= left_near_x {
            self.draw_scrolling_scenery(
                road,
                left_outer_x,
                distant_phase,
                (layer * 2 + 1).rem_euclid(SCENERY_SLOT_HEIGHT),
                &DISTANT_SCENERY,
                true,
            );
            layer += 1;
            left_outer_x += SCENERY_WIDTH + 2;
        }

        layer = 0;
        let mut right_outer_x = right_near_x + SCENERY_WIDTH + 2;
        while right_outer_x + SCENERY_WIDTH < frame_width {
            self.draw_scrolling_scenery(
                road,
                right_outer_x,
                distant_phase,
                (layer * 2 + 2).rem_euclid(SCENERY_SLOT_HEIGHT),
                &DISTANT_SCENERY,
                true,
            );
            layer += 1;
            right_outer_x += SCENERY_WIDTH + 2;
        }
    }

    fn draw_scrolling_scenery(
        &mut self,
        road: &Road,
        x: i32,
        phase: i32,
        stagger: i32,
        sequence: &[SceneryKind],
        distant: bool,
    ) {
        let first_anchor = road.top - (SCENERY_HEIGHT - 1);
        for anchor_y in first_anchor..=road.bottom {
            let world_y = anchor_y - phase - stagger;
            if world_y.rem_euclid(SCENERY_SLOT_HEIGHT) != 0 {
                continue;
            }

            let slot = usize::try_from(
                world_y
                    .div_euclid(SCENERY_SLOT_HEIGHT)
                    .rem_euclid(i32::try_from(sequence.len()).unwrap_or(1)),
            )
            .unwrap_or(0);
            self.draw_scenery_sprite(road, x, anchor_y, sequence[slot], distant);
        }
    }

    fn draw_scenery_sprite(
        &mut self,
        road: &Road,
        x: i32,
        y: i32,
        kind: SceneryKind,
        distant: bool,
    ) {
        for (row_index, row) in kind.sprite().iter().enumerate() {
            let Ok(row_index) = i32::try_from(row_index) else {
                break;
            };
            for (column, character) in row.chars().enumerate() {
                if character == ' ' {
                    continue;
                }
                let Ok(column) = i32::try_from(column) else {
                    break;
                };
                self.put_roadside(
                    road,
                    x + column,
                    y + row_index,
                    character,
                    if distant {
                        distant_scenery_style(character)
                    } else {
                        scenery_style(kind, character)
                    },
                );
            }
        }
    }

    fn put_roadside(&mut self, road: &Road, x: i32, y: i32, character: char, style: Style) {
        let is_gutter =
            (x >= 0 && x < road.left) || (x > road.right && x < i32::from(self.frame.width));
        if is_gutter && y >= road.top && y <= road.bottom {
            self.frame.put(x, y, character, style);
        }
    }

    fn draw_vehicle_sprite(
        &mut self,
        x: i32,
        y: i32,
        sprite: &'static Sprite,
        body_style: Style,
        road: &Road,
    ) {
        for (row_index, row) in sprite.iter().enumerate() {
            let Ok(draw_row) = i32::try_from(row_index) else {
                break;
            };
            let draw_y = y + draw_row;
            if draw_y < road.top || draw_y > road.bottom {
                continue;
            }

            for (column, character) in row.chars().enumerate() {
                if character == ' ' {
                    continue;
                }
                let Ok(column) = i32::try_from(column) else {
                    break;
                };
                let draw_x = x + column;
                if draw_x > road.left && draw_x < road.right {
                    self.frame.put(
                        draw_x,
                        draw_y,
                        character,
                        vehicle_detail_style(character, row_index, body_style),
                    );
                }
            }
        }
    }

    fn draw_crash_effect(&mut self, game: &Game) {
        let player = game.player();
        let x = player.render_x();
        let y = player.render_y();
        let right = x + i32::from(SPRITE_WIDTH);
        let bottom = y + i32::from(SPRITE_HEIGHT) - 1;
        let red = Style::new(RED, ASPHALT, true);
        let amber = Style::new(AMBER, ASPHALT, true);
        self.frame.text(x - 1, y - 2, "COLLISION", red);
        self.frame.put(x - 2, y, '*', amber);
        self.frame.put(x - 1, y - 1, '\\', red);
        self.frame.put(right, y - 1, '/', red);
        self.frame.put(right + 1, y, '*', amber);
        self.frame.put(x - 1, bottom, '/', amber);
        self.frame.put(right, bottom, '\\', amber);
    }

    fn draw_pause_overlay(&mut self) {
        let box_width = 40;
        let box_height = 7;
        let x = (i32::from(self.frame.width) - box_width).max(0) / 2;
        let y = (i32::from(self.frame.height) - box_height).max(0) / 2;
        self.frame.draw_box(
            x,
            y,
            box_width,
            box_height,
            Style::new(MUTED, CANVAS, false),
        );
        self.frame
            .centered_text(y + 1, "PAUSED", Style::new(RED, CANVAS, true));
        self.frame
            .centered_text(y + 3, "RACE SUSPENDED", DEFAULT_STYLE);
        self.frame
            .centered_text(y + 5, "[P] RESUME    [Q / ESC] EXIT", MUTED_STYLE);
    }

    fn draw_game_over_overlay(&mut self, game: &Game) {
        let box_width = 50.min(i32::from(self.frame.width).saturating_sub(4));
        let box_height = 11;
        let x = (i32::from(self.frame.width) - box_width).max(0) / 2;
        let y = (i32::from(self.frame.height) - box_height).max(0) / 2;
        self.frame
            .draw_box(x, y, box_width, box_height, Style::new(RED, CANVAS, true));
        self.frame
            .centered_text(y + 1, "RACE OVER", Style::new(RED, CANVAS, true));
        self.frame.centered_text(
            y + 3,
            &format!(
                "FINAL {:07}    SESSION BEST {:07}",
                game.score(),
                game.high_score()
            ),
            DEFAULT_STYLE,
        );
        self.frame.centered_text(
            y + 5,
            &format!(
                "TIME {}    DIST {}    PASS {:03}",
                format_run_time(game.run_time()),
                format_distance(game.distance()),
                game.passed(),
            ),
            HUD_STYLE,
        );
        self.frame.centered_text(
            y + 6,
            &format!(
                "NEAR {:02}    BEST STREAK x{:02}",
                game.near_misses(),
                game.best_streak()
            ),
            DEFAULT_STYLE,
        );
        self.frame
            .centered_text(y + 8, "[ENTER / R] RETRY", Style::new(CYAN, CANVAS, true));
        self.frame
            .centered_text(y + 9, "[Q / ESC] EXIT", MUTED_STYLE);
    }

    fn serialize(&mut self) -> io::Result<()> {
        self.output.clear();
        // Line wrapping is disabled for the lifetime of the terminal session,
        // so painting the final column is safe and prevents a default-color
        // stripe on terminals whose background is not black.
        let usable_width = self.frame.width;

        for y in 0..self.frame.height {
            queue!(&mut self.output, MoveTo(0, y))?;
            let row_start = usize::from(y) * usize::from(self.frame.width);
            let mut x = 0_u16;

            while x < usable_width {
                let cell = self.frame.cells[row_start + usize::from(x)];
                let run_start = x;
                x += 1;
                while x < usable_width
                    && self.frame.cells[row_start + usize::from(x)].style == cell.style
                {
                    x += 1;
                }

                self.text_run.clear();
                self.text_run.extend(
                    (run_start..x)
                        .map(|column| self.frame.cells[row_start + usize::from(column)].character),
                );
                queue!(
                    &mut self.output,
                    SetForegroundColor(cell.style.foreground),
                    SetBackgroundColor(cell.style.background),
                    SetAttribute(if cell.style.bold {
                        Attribute::Bold
                    } else {
                        Attribute::NormalIntensity
                    }),
                    Print(&self.text_run)
                )?;
            }
        }

        queue!(&mut self.output, ResetColor, SetAttribute(Attribute::Reset))?;
        Ok(())
    }
}

fn vehicle_detail_style(character: char, row: usize, body: Style) -> Style {
    match character {
        'O' => Style::new(CITY_FAR, body.background, false),
        'o' => Style::new(AMBER, body.background, true),
        'v' => Style::new(RED, body.background, true),
        '=' | '*' => Style::new(CYAN, body.background, true),
        _ if row == 2 && character != '|' => Style::new(CYAN, body.background, false),
        _ => body,
    }
}

fn scenery_style(kind: SceneryKind, character: char) -> Style {
    let foreground = match kind {
        SceneryKind::Empty => MUTED,
        SceneryKind::House | SceneryKind::Shop | SceneryKind::Apartment => {
            if matches!(character, '[' | ']') {
                AMBER
            } else {
                CITY_NEAR
            }
        }
        SceneryKind::Garden | SceneryKind::Trees => {
            if character == '*' {
                AMBER
            } else {
                GARDEN
            }
        }
        SceneryKind::Spectators => {
            if character == 'o' {
                PRIMARY
            } else {
                CITY_NEAR
            }
        }
        SceneryKind::Sign => {
            if character.is_ascii_digit() {
                CYAN
            } else {
                AMBER
            }
        }
        SceneryKind::Park => {
            if character == '|' {
                GARDEN
            } else {
                MUTED
            }
        }
    };
    Style::new(foreground, CANVAS, false)
}

fn distant_scenery_style(character: char) -> Style {
    let foreground = if matches!(character, '[' | ']') {
        CITY_NEAR
    } else {
        CITY_FAR
    };
    Style::new(foreground, CANVAS, false)
}

fn format_run_time(seconds: f32) -> String {
    let safe_seconds = if seconds.is_finite() {
        seconds.max(0.0)
    } else {
        0.0
    };
    let total_tenths = (safe_seconds * 10.0).floor() as u64;
    let minutes = total_tenths / 600;
    let seconds = (total_tenths % 600) as f32 / 10.0;
    format!("{minutes:02}:{seconds:04.1}")
}

fn format_distance(distance: f32) -> String {
    let safe_distance = if distance.is_finite() {
        distance.max(0.0)
    } else {
        0.0
    };
    if safe_distance >= 10_000.0 {
        format!("{:04.1}K", safe_distance / 1_000.0)
    } else {
        format!("{safe_distance:05.0}m")
    }
}

fn difficulty_name(level: u8) -> &'static str {
    match level {
        1..=2 => "ROOKIE",
        3..=4 => "STREET",
        5..=6 => "TURBO",
        7..=8 => "EXPERT",
        _ => "MAYHEM",
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{ControlInput, GameAction};

    fn contains(frame: &[u8], needle: &[u8]) -> bool {
        frame.windows(needle.len()).any(|window| window == needle)
    }

    fn row_text(frame: &FrameBuffer, y: u16) -> String {
        let start = usize::from(y) * usize::from(frame.width);
        frame.cells[start..start + usize::from(frame.width)]
            .iter()
            .map(|cell| cell.character)
            .collect()
    }

    fn frame_text(frame: &FrameBuffer) -> String {
        (0..frame.height)
            .map(|y| row_text(frame, y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn tiny_terminal_warning_is_clipped_safely() {
        let game = Game::new_seeded(80, 30, 7);
        let mut renderer = Renderer::new();
        let frame = renderer.render(&game, 8, 3).expect("render should succeed");
        assert!(!frame.is_empty());
    }

    #[test]
    fn serialized_frame_uses_cursor_moves_instead_of_newlines() {
        let game = Game::new_seeded(80, 30, 11);
        let mut renderer = Renderer::new();
        let frame = renderer
            .render(&game, 80, 30)
            .expect("render should succeed");
        assert!(!frame.contains(&b'\n'));
        assert!(!frame.contains(&b'\r'));
    }

    #[test]
    fn serialization_paints_the_last_column() {
        let mut renderer = Renderer::new();
        renderer.frame.resize(3, 2);
        renderer.frame.clear();
        renderer.frame.put(2, 0, 'X', DEFAULT_STYLE);
        renderer.frame.put(2, 1, 'Y', DEFAULT_STYLE);

        renderer.serialize().expect("serialization should succeed");

        assert!(contains(&renderer.output, b"X"));
        assert!(contains(&renderer.output, b"Y"));
    }

    #[test]
    fn every_roadside_sprite_is_fixed_width_ascii() {
        for kind in [
            SceneryKind::Empty,
            SceneryKind::House,
            SceneryKind::Shop,
            SceneryKind::Garden,
            SceneryKind::Apartment,
            SceneryKind::Spectators,
            SceneryKind::Trees,
            SceneryKind::Sign,
            SceneryKind::Park,
        ] {
            assert_eq!(kind.sprite().len(), SCENERY_HEIGHT as usize);
            for row in kind.sprite() {
                assert!(row.is_ascii());
                assert_eq!(row.len(), SCENERY_WIDTH as usize);
            }
        }
    }

    #[test]
    fn minimum_size_roadside_is_populated_without_touching_the_road() {
        let road = Road::new(60, 24);
        let mut renderer = Renderer::new();
        renderer.frame.resize(60, 24);
        renderer.frame.clear();
        for y in road.top..=road.bottom {
            for x in road.left..=road.right {
                renderer.frame.put(x, y, '@', ROAD_STYLE);
            }
        }

        renderer.draw_roadside(&road, 0);

        for y in road.top..=road.bottom {
            for x in road.left..=road.right {
                let index = usize::try_from(y).unwrap() * usize::from(renderer.frame.width)
                    + usize::try_from(x).unwrap();
                assert_eq!(renderer.frame.cells[index].character, '@');
            }
        }

        let visible = renderer
            .frame
            .cells
            .iter()
            .enumerate()
            .filter(|(index, cell)| {
                let x = i32::try_from(index % usize::from(renderer.frame.width)).unwrap();
                let y = i32::try_from(index / usize::from(renderer.frame.width)).unwrap();
                y >= road.top
                    && y <= road.bottom
                    && (x < road.left || x > road.right)
                    && cell.character != ' '
            })
            .count();
        let gutter_cells = usize::try_from(road.bottom - road.top + 1).unwrap()
            * usize::try_from(road.left + (i32::from(renderer.frame.width) - road.right - 1))
                .unwrap();
        assert!(visible >= 35, "only {visible} visible roadside cells");
        assert!(
            visible * 2 < gutter_cells,
            "roadside lost its negative space"
        );
        assert!(
            renderer
                .frame
                .cells
                .iter()
                .any(|cell| cell.character == '[')
        );
        assert!(
            renderer
                .frame
                .cells
                .iter()
                .any(|cell| cell.character == 'o')
        );
    }

    #[test]
    fn roadside_loop_is_seamless_at_the_road_phase_wrap() {
        let road = Road::new(80, 30);
        let mut renderer = Renderer::new();
        renderer.frame.resize(80, 30);
        renderer.frame.clear();
        renderer.draw_roadside(&road, 0);
        let at_zero = renderer.frame.cells.clone();

        renderer.frame.clear();
        renderer.draw_roadside(&road, 84);

        assert_eq!(renderer.frame.cells, at_zero);
    }

    #[test]
    fn partially_entering_vehicle_cannot_overwrite_the_hud() {
        let road = Road::new(60, 24);
        let mut renderer = Renderer::new();
        renderer.frame.resize(60, 24);
        renderer.frame.clear();
        for y in 0..road.top {
            for x in 0..i32::from(renderer.frame.width) {
                renderer.frame.put(x, y, 'H', HUD_STYLE);
            }
        }

        renderer.draw_vehicle_sprite(
            road.lane_x_for(0, SPRITE_WIDTH),
            -1,
            VehicleKind::Sedan.sprite(),
            Style::new(RED, ASPHALT, true),
            &road,
        );

        for y in 0..road.top {
            for x in 0..i32::from(renderer.frame.width) {
                let index = usize::try_from(y).unwrap() * usize::from(renderer.frame.width)
                    + usize::try_from(x).unwrap();
                assert_eq!(renderer.frame.cells[index].character, 'H');
            }
        }
        assert!((road.left + 1..road.right).any(|x| {
            let index = usize::try_from(road.top).unwrap() * usize::from(renderer.frame.width)
                + usize::try_from(x).unwrap();
            renderer.frame.cells[index].character != ' '
        }));
    }

    #[test]
    fn minimum_size_gameplay_has_complete_telemetry_and_scenery() {
        let mut game = Game::new_seeded(60, 24, 19);
        game.handle_action(GameAction::Start);
        let mut renderer = Renderer::new();
        renderer
            .render(&game, 60, 24)
            .expect("render should succeed");
        let frame = frame_text(&renderer.frame);

        assert!(frame.contains("ASCII APEX"));
        assert!(frame.contains("N2O"));
        assert!(frame.contains("ROOKIE"));
        assert!(frame.contains("SCORE"));
        assert!(frame.contains("["));
    }

    #[test]
    fn title_screen_uses_the_selected_endurance_visual_language() {
        let game = Game::new_seeded(80, 30, 23);
        let mut renderer = Renderer::new();
        renderer
            .render(&game, 80, 30)
            .expect("render should succeed");
        let frame = frame_text(&renderer.frame);

        assert!(frame.contains("ASCII APEX"));
        assert!(frame.contains("N I G H T   E N D U R A N C E"));
        assert!(frame.contains("[ ENTER ]  START ENGINE"));
        assert!(frame.contains("CLOSE CALLS REFILL NITRO"));
        assert!(!frame.contains("CAFE"));
    }

    #[test]
    fn minimum_size_overlays_are_compact_and_actionable() {
        let game = Game::new_seeded(60, 24, 29);
        let mut renderer = Renderer::new();
        renderer.frame.resize(60, 24);
        renderer.frame.clear();
        renderer.draw_pause_overlay();
        let pause = frame_text(&renderer.frame);
        assert!(pause.contains("PAUSED"));
        assert!(pause.contains("[P] RESUME"));
        assert!(pause.contains("[Q / ESC] EXIT"));
        assert!(!pause.contains("simulation is frozen"));

        renderer.frame.clear();
        renderer.draw_game_over_overlay(&game);
        let result = frame_text(&renderer.frame);
        assert!(result.contains("RACE OVER"));
        assert!(result.contains("FINAL 0000000"));
        assert!(result.contains("TIME 00:00.0"));
        assert!(result.contains("[ENTER / R] RETRY"));
        assert!(result.contains("[Q / ESC] EXIT"));
    }

    #[test]
    fn telemetry_rails_adapt_at_compact_standard_and_wide_sizes() {
        for (width, height) in [(60, 24), (72, 30), (80, 30), (120, 40)] {
            let mut game = Game::new_seeded(width, height, u64::from(width));
            game.handle_action(GameAction::Start);
            game.update(0.1, Default::default());
            let mut renderer = Renderer::new();
            renderer
                .render(&game, width, height)
                .expect("render should succeed");

            let road = game.road();
            let header = row_text(&renderer.frame, 0);
            let footer = row_text(&renderer.frame, u16::try_from(road.bottom + 2).unwrap());
            assert_eq!(header.len(), usize::from(width));
            assert_eq!(footer.len(), usize::from(width));
            assert!(header.contains("ASCII APEX"));
            assert!(header.contains("SCORE"));
            assert!(header.contains("PAUSE"));
            assert!(footer.contains("00:00.1"));

            if width < 80 {
                let second_header = row_text(&renderer.frame, 1);
                assert!(second_header.contains("N2O"));
                assert!(second_header.contains("STREAK"));
                assert!(second_header.contains("ROOKIE"));
            } else {
                assert!(header.contains("N2O") || header.contains("NITRO"));
                assert!(footer.contains("TIME"));
                assert!(footer.contains("HIGH"));
            }
        }
    }

    #[test]
    fn vehicle_details_are_semantic_and_not_rainbow_body_fills() {
        let body = Style::new(PRIMARY, ASPHALT, false);
        assert_eq!(vehicle_detail_style('-', 1, body), body);
        assert_eq!(vehicle_detail_style('o', 1, body).foreground, AMBER);
        assert_eq!(vehicle_detail_style('v', 4, body).foreground, RED);
        assert_eq!(vehicle_detail_style('-', 2, body).foreground, CYAN);
        assert_eq!(vehicle_detail_style('O', 3, body).foreground, CITY_FAR);
    }

    #[test]
    fn active_nitro_draws_a_three_row_exhaust_trail() {
        let mut game = Game::new_seeded(60, 24, 31);
        game.handle_action(GameAction::Start);
        game.update(
            0.1,
            ControlInput {
                boost: true,
                ..ControlInput::default()
            },
        );
        let mut renderer = Renderer::new();
        renderer
            .render(&game, 60, 24)
            .expect("render should succeed");

        let x = game.player().render_x();
        let exhaust_y = game.player().render_y() + i32::from(SPRITE_HEIGHT);
        let cell = |x: i32, y: i32| {
            renderer.frame.cells[usize::try_from(y).unwrap() * usize::from(renderer.frame.width)
                + usize::try_from(x).unwrap()]
        };
        assert_eq!(cell(x + 2, exhaust_y).character, ':');
        assert_eq!(cell(x + 4, exhaust_y + 1).character, '|');
        assert_eq!(cell(x + 3, exhaust_y + 2).character, '.');
        assert_eq!(cell(x + 3, exhaust_y + 2).style.foreground, CYAN);
    }

    #[test]
    fn telemetry_formatters_are_stable_at_boundaries() {
        assert_eq!(format_run_time(0.0), "00:00.0");
        assert_eq!(format_run_time(59.99), "00:59.9");
        assert_eq!(format_run_time(60.0), "01:00.0");
        assert_eq!(format_run_time(f32::NAN), "00:00.0");
        assert_eq!(format_distance(0.0), "00000m");
        assert_eq!(format_distance(9_999.0), "09999m");
        assert_eq!(format_distance(10_000.0), "10.0K");
    }
}
