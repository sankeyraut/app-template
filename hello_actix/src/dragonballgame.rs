use serde::{Deserialize, Serialize};
use rand::Rng; // Kept as it is used
use std::time::Instant;



const CANVAS_WIDTH: f64 = 800.0;
const CANVAS_HEIGHT: f64 = 600.0;
const PLAYER_X_OFFSET: f64 = 50.0; // Distance from right edge
const DRAGON_X_OFFSET: f64 = 50.0; // Distance from left edge
const FIREBALL_SPEED_BASE: f64 = 3.0; // Reduced speed for better playability
const FIREBALL_RADIUS: f64 = 20.0;
const WATER_SPRAY_RANGE: f64 = 300.0; // Range of the water spray
const WATER_SPRAY_ANGLE: f64 = 0.5; // Spread of spray in radians

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FireballState {
    Active,
    Extinguishing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fireball {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub state: FireballState,
    pub extinguish_timer: f64, // Normalized 1.0 to 0.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub y: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    pub score: u32,
    pub game_over: bool,
    pub fireballs: Vec<Fireball>,
    pub player: Player,
    #[serde(skip)]
    pub last_update: Option<Instant>,
    #[serde(skip)]
    pub fireball_id_counter: u64,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            score: 0,
            game_over: false,
            fireballs: Vec::new(),
            player: Player { y: CANVAS_HEIGHT / 2.0 },
            last_update: None,
            fireball_id_counter: 0,
        }
    }

    pub fn update_player_pos(&mut self, y: f64) {
        if !self.game_over {
            self.player.y = y.clamp(0.0, CANVAS_HEIGHT);
        }
    }

    pub fn tick(&mut self) {
        if self.game_over {
            return;
        }

        // Assume 60 FPS tick rate roughly, or use delta time if we tracked it strictly per tick
        // For simplicity in this loop, we'll just move by velocity
        
        // 1. Spawn Fireballs (Random chance)
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.02) { // 2% chance per tick to spawn
            self.fireball_id_counter += 1;
            let target_y = rng.gen_range(50.0..CANVAS_HEIGHT-50.0);
            let start_y = rng.gen_range(100.0..CANVAS_HEIGHT-100.0);
            
            // Calculate velocity vector towards a random point on the right side
            let dx = CANVAS_WIDTH - DRAGON_X_OFFSET;
            let dy = target_y - start_y;
            let distance = (dx*dx + dy*dy).sqrt();
            
            let speed = FIREBALL_SPEED_BASE + (self.score as f64 * 0.1); // Increase difficulty
            
            self.fireballs.push(Fireball {
                id: self.fireball_id_counter,
                x: DRAGON_X_OFFSET,
                y: start_y,
                vx: (dx / distance) * speed,
                vy: (dy / distance) * speed,
                state: FireballState::Active,
                extinguish_timer: 1.0,
            });
        }

        // 2. Update Fireballs & Check Collisions
        let mut game_over_triggered = false;

        for fireball in &mut self.fireballs {
            if fireball.state == FireballState::Extinguishing {
                fireball.extinguish_timer -= 0.05; // Fade out speed
                continue; // Skip movement/collision for extinguishing fireballs
            }

            fireball.x += fireball.vx;
            fireball.y += fireball.vy;

            // Check Collision with Red Line (Right side)
            if fireball.x > CANVAS_WIDTH - PLAYER_X_OFFSET {
                game_over_triggered = true;
            }

            // Check Collision with Water Spray
            // Simple logic: If fireball is within range and angle of player
            let dx = fireball.x - (CANVAS_WIDTH - PLAYER_X_OFFSET);
            let dy = fireball.y - self.player.y;
            let dist = (dx*dx + dy*dy).sqrt();
            
            // In our game, player shoots LEFT. 
            // So valid spray area is to the LEFT of the player.
            // dx should be negative.
            
            if dist < WATER_SPRAY_RANGE && dx < 0.0 {
                 // Check angle. 
                 // At (dx, dy), atan2(dy, dx) gives angle. 
                 // Player aims directly LEFT (Pi or -Pi).
                 // We check if angle is close to Pi.
                 let angle = dy.atan2(dx).abs(); 
                 // Target angle is PI (180 deg). We accept PI +/- spray_angle
                 if (std::f64::consts::PI - angle).abs() < WATER_SPRAY_ANGLE {
                     // Extinguished!
                     fireball.state = FireballState::Extinguishing;
                     self.score += 10;
                 }
            }
        }
        
        if game_over_triggered {
            self.game_over = true;
        }

        // Remove fireballs that are done extinguishing
        self.fireballs.retain(|f| f.state == FireballState::Active || f.extinguish_timer > 0.0);
    }
}
