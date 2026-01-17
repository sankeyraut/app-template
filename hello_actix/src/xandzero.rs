use actix_web::{web, HttpResponse, Responder};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde::{Deserialize, Serialize};
use rand::Rng;
use redis::AsyncCommands;
use crate::{AppState, validate_token};

/// Request structure from the frontend for playing a move
#[derive(Serialize, Deserialize)]
pub struct XandZeroRequest {
    pub board: Vec<String>, // Length 9 array representing the 3x3 grid
}

/// Response returned to the frontend after processing a turn
#[derive(Serialize, Deserialize)]
pub struct XandZeroResponse {
    pub board: Vec<String>,     // The updated board state
    pub winner: Option<String>, // The winner if the game has ended
    pub score_increment: i32,   // Points earned in this turn
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

    let mut board = req.board.clone();
    if board.len() != 9 {
        return HttpResponse::BadRequest().body("Invalid board size");
    }

    // User move (X) - Handled by UI, but we check if game already over
    let mut winner = check_winner(&board);
    let mut score_increment = 0;

    if winner.is_none() {
        // Computer move (O)
        if let Some(mv) = get_computer_move(&board) {
            board[mv] = "O".to_string();
            winner = check_winner(&board);
        }
    }

    if let Some(w) = &winner {
        if w == "X" {
            score_increment = 100;
        } else if w == "O" {
            score_increment = 10;
        } else if w == "Draw" {
            score_increment = 20;
        }

        // Update Leaderboard
        let user_id = claims.sub;
        let username = claims.preferred_username.or(claims.name).unwrap_or_else(|| "Anonymous".to_string());

        // Increment score in DB
        let _ = sqlx::query(
            "INSERT INTO leaderboard (user_id, username, score) 
             VALUES ($1, $2, $3) 
             ON CONFLICT (user_id) DO UPDATE SET score = leaderboard.score + $3"
        )
        .bind(&user_id)
        .bind(&username)
        .bind(score_increment)
        .execute(&data.db)
        .await;

        // Update Valkey (get new total score first for ZADD)
        let total_score: i32 = sqlx::query_scalar("SELECT score FROM leaderboard WHERE user_id = $1")
            .bind(&user_id)
            .fetch_one(&data.db)
            .await
            .unwrap_or(0);

        if let Ok(mut con) = data.redis_client.get_async_connection().await {
            let _: Result<(), redis::RedisError> = con.zadd("leaderboard", &username, total_score).await;
        }
    }

    HttpResponse::Ok().json(XandZeroResponse {
        board,
        winner,
        score_increment,
    })
}
