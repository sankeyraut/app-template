use actix_web::{web, HttpResponse, Responder};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde::{Deserialize, Serialize};
use rand::Rng;
use redis::AsyncCommands;
use crate::{AppState, validate_token};

/// Request structure from the frontend for playing a move
#[derive(Serialize, Deserialize)]
pub struct XandZeroRequest {
    pub board: Vec<String>,     // Length 9 array representing the 3x3 grid
    pub move_history: Option<Vec<usize>>, // List of move indices in order (for Sudden Death)
    pub game_mode: String,      // "normal" or "sudden_death"
    pub used_power_up: bool,    // If true, user wants to use "Erase" power-up
    pub erase_index: Option<usize>, // Index to erase if using power-up
}

/// Response returned to the frontend after processing a turn
#[derive(Serialize, Deserialize)]
pub struct XandZeroResponse {
    pub board: Vec<String>,
    pub move_history: Vec<usize>,
    pub winner: Option<String>,
    pub score_increment: i32,
    pub power_up_used_by_ai: bool,
    pub ai_erase_index: Option<usize>,
}

/// Utility function to check if the current board state has a winner
pub fn check_winner(board: &[String]) -> Option<String> {
    let lines = [
        [0, 1, 2], [3, 4, 5], [6, 7, 8], // Rows
        [0, 3, 6], [1, 4, 7], [2, 5, 8], // Cols
        [0, 4, 8], [2, 4, 6],           // Diagonals
    ];

    for line in lines {
        if !board[line[0]].is_empty() && board[line[0]] == board[line[1]] && board[line[0]] == board[line[2]] {
            return Some(board[line[0]].clone());
        }
    }

    if board.iter().all(|s| !s.is_empty()) {
        return Some("Draw".to_string());
    }

    None
}

/// Simple AI logic for the computer move (O)
/// Priority: 1. Win if possible, 2. Block user if they are winning, 3. Take center, 4. Random available move
pub fn get_computer_move(board: &[String]) -> Option<usize> {
    // 1. Try to win
    for i in 0..9 {
        if board[i].is_empty() {
            let mut test_board = board.to_vec();
            test_board[i] = "O".to_string();
            if check_winner(&test_board) == Some("O".to_string()) {
                return Some(i);
            }
        }
    }

    // 2. Block player
    for i in 0..9 {
        if board[i].is_empty() {
            let mut test_board = board.to_vec();
            test_board[i] = "X".to_string();
            if check_winner(&test_board) == Some("X".to_string()) {
                return Some(i);
            }
        }
    }

    // 3. Take center
    if board[4].is_empty() {
        return Some(4);
    }

    // 4. Random available
    let available: Vec<usize> = board.iter().enumerate()
        .filter(|(_, s)| s.is_empty())
        .map(|(i, _)| i)
        .collect();
    
    if available.is_empty() {
        return None;
    }

    let mut rng = rand::thread_rng();
    Some(available[rng.gen_range(0..available.len())])
}

/// Main game move endpoint. Handles user move, computer move, win detection, and leaderboard updates.
#[actix_web::post("/xandzero/play")]
pub async fn xandzero_play(
    data: web::Data<AppState>,
    auth: BearerAuth,
    req: web::Json<XandZeroRequest>,
) -> impl Responder {
    let claims = match validate_token(auth.token(), &data.oidc_jwks_uri).await {
        Ok(c) => c,
        Err(r) => return r,
    };

    let user_id = claims.sub;
    let username = claims.preferred_username.or(claims.name).unwrap_or_else(|| "Anonymous".to_string());
    let mut board = req.board.clone();
    let mut history = req.move_history.clone().unwrap_or_default();
    let mut ai_used_power_up = false;
    let mut ai_erase_idx = None;
    // --- POWER-UP HANDLING (USER) ---
    if req.used_power_up {
        if let Some(idx) = req.erase_index {
            // Check points (Cost: 50)
            let user_points: i32 = sqlx::query_scalar("SELECT score FROM leaderboard WHERE user_id = $1 AND game_name = 'xandzero'")
                .bind(&user_id)
                .fetch_one(&data.db)
                .await
                .unwrap_or(0);

            if user_points >= 50 {
                // Erase mark and deduct points
                if !board[idx].is_empty() {
                    board[idx] = "".to_string();
                    history.retain(|&i| i != idx);

                    // Update DB
                    let new_total = user_points - 50;
                    let _ = sqlx::query("UPDATE leaderboard SET score = $1 WHERE user_id = $2 AND game_name = 'xandzero'")
                        .bind(new_total)
                        .bind(&user_id)
                        .execute(&data.db)
                        .await;
                    
                    // Update Redis (Valkey) immediately so leaderboard UI reflects deduction
                    if let Ok(mut con) = data.redis_client.get_async_connection().await {
                        let _: Result<(), redis::RedisError> = con.zadd("leaderboard:xandzero", &username, new_total).await;
                    }
                }
            } else {
                return HttpResponse::BadRequest().body("Insufficient points for power-up");
            }

            // Return immediately after power-up usage so user sees the board change
            // and the AI doesn't immediately counter-move into the same spot.
            let current_winner = check_winner(&board);
            return HttpResponse::Ok().json(XandZeroResponse {
                board,
                move_history: history,
                winner: current_winner,
                score_increment: 0,
                power_up_used_by_ai: false,
                ai_erase_index: None,
            });
        }
    }

    // --- SUDDEN DEATH: MOVE REMOVAL (USER) ---
    // If Sudden Death and User had > 3 moves just now
    if req.game_mode == "sudden_death" {
        let x_moves: Vec<usize> = history.iter().cloned().filter(|&i| board[i] == "X").collect();
        if x_moves.len() > 3 {
            let oldest = x_moves[0];
            board[oldest] = "".to_string();
            history.retain(|&i| i != oldest);
        }
    }

    // --- AI TURN ---
    let mut winner = check_winner(&board);
    let mut score_increment = 0;

    if winner.is_none() {
        // AI Power-up check (20% chance if player has > 2 marks)
        let player_marks = board.iter().filter(|&s| s == "X").count();
        if player_marks >= 2 && rand::thread_rng().gen_bool(0.2) {
            // AI erases a random player mark
            let x_moves: Vec<usize> = board.iter().enumerate()
                .filter(|(_, s)| s == &"X")
                .map(|(i, _)| i)
                .collect();
            
            if !x_moves.is_empty() {
                let erase_idx = x_moves[rand::thread_rng().gen_range(0..x_moves.len())];
                board[erase_idx] = "".to_string();
                history.retain(|&i| i != erase_idx);
                ai_used_power_up = true;
                ai_erase_idx = Some(erase_idx);
            }
        }

        // Computer move (O)
        let computer_move = get_computer_move(&board);

        if let Some(mv) = computer_move {
            board[mv] = "O".to_string();
            history.push(mv);
            
            // Sudden Death: Move removal (AI)
            if req.game_mode == "sudden_death" {
                let o_moves: Vec<usize> = history.iter().cloned().filter(|&i| board[i] == "O").collect();
                if o_moves.len() > 3 {
                    let oldest = o_moves[0];
                    board[oldest] = "".to_string();
                    history.retain(|&i| i != oldest);
                }
            }
            winner = check_winner(&board);
        }
    }

    // --- SCORING & FINALIZATION ---
    if let Some(w) = &winner {
        if w == "X" {
            score_increment = 100;
        } else if w == "O" {
            score_increment = 10;
        } else if w == "Draw" {
            score_increment = 20;
        }

        // Update Leaderboard (Increment)
        let _ = sqlx::query(
            "INSERT INTO leaderboard (user_id, game_name, username, score) 
             VALUES ($1, 'xandzero', $2, $3) 
             ON CONFLICT (user_id, game_name) DO UPDATE SET score = leaderboard.score + $3"
        )
        .bind(&user_id)
        .bind(&username)
        .bind(score_increment)
        .execute(&data.db)
        .await;

        // Update Valkey
        let total_score: i32 = sqlx::query_scalar("SELECT score FROM leaderboard WHERE user_id = $1 AND game_name = 'xandzero'")
            .bind(&user_id)
            .fetch_one(&data.db)
            .await
            .unwrap_or(0);

        if let Ok(mut con) = data.redis_client.get_async_connection().await {
            let _: Result<(), redis::RedisError> = con.zadd("leaderboard:xandzero", &username, total_score).await;
        }
    }

    HttpResponse::Ok().json(XandZeroResponse {
        board,
        move_history: history,
        winner,
        score_increment,
        power_up_used_by_ai: ai_used_power_up,
        ai_erase_index: ai_erase_idx,
    })
}
