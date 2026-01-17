use actix_web::{web, HttpResponse, Responder};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde::{Deserialize, Serialize};
use redis::AsyncCommands;
use crate::{AppState, LeaderboardEntry, validate_token};

/// Request payload for submitting a new score
#[derive(Serialize, Deserialize)]
pub struct ScoreRequest {
    pub score: i32,
}

/// API endpoint to submit a player's latest score.
/// Persistence is handled in PostgreSQL, while real-time rankings are managed in Valkey/Redis.
#[actix_web::post("/leaderboard")]
pub async fn submit_score(
    data: web::Data<AppState>,
    auth: BearerAuth,
    score_req: web::Json<ScoreRequest>,
) -> impl Responder {
    let claims = match validate_token(auth.token(), &data.oidc_jwks_uri).await {
        Ok(c) => c,
        Err(r) => return r,
    };

    let user_id = claims.sub;
    let username = claims.preferred_username.or(claims.name).unwrap_or_else(|| "Anonymous".to_string());
    let new_score = score_req.score;

    // 1. Update PostgreSQL (High Score Logic)
    let result = sqlx::query(
        "INSERT INTO leaderboard (user_id, username, score) 
         VALUES ($1, $2, $3) 
         ON CONFLICT (user_id) DO UPDATE 
         SET score = EXCLUDED.score, username = EXCLUDED.username, updated_at = NOW() 
         WHERE leaderboard.score < EXCLUDED.score"
    )
    .bind(&user_id)
    .bind(&username)
    .bind(new_score)
    .execute(&data.db)
    .await;

    if let Err(e) = result {
        return HttpResponse::InternalServerError().body(format!("DB error: {}", e));
    }

    // 2. Update Valkey (Redis)
    let mut con = match data.redis_client.get_async_connection().await {
        Ok(con) => con,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Redis error: {}", e)),
    };

    // ZADD will update if score is different. 
    let _: Result<(), redis::RedisError> = con.zadd("leaderboard", &username, new_score).await;

    HttpResponse::Ok().body("Score submitted")
}

/// API endpoint to retrieve the current top 10 global leaderboard.
#[actix_web::get("/leaderboard")]
pub async fn get_leaderboard(data: web::Data<AppState>) -> impl Responder {
    let mut con = match data.redis_client.get_async_connection().await {
        Ok(con) => con,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Redis error: {}", e)),
    };

    // Get top 10
    let results: Vec<(String, i32)> = match con.zrevrange_withscores("leaderboard", 0, 9).await {
        Ok(res) => res,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Redis error: {}", e)),
    };

    let leaderboard: Vec<LeaderboardEntry> = results
        .into_iter()
        .enumerate()
        .map(|(rank, (username, score))| LeaderboardEntry {
            username,
            score,
            rank: Some((rank + 1) as i64),
        })
        .collect();

    HttpResponse::Ok().json(leaderboard)
}

/// API endpoint to retrieve the rank and score of the currently authenticated player.
#[actix_web::get("/leaderboard/me")]
pub async fn get_my_rank(data: web::Data<AppState>, auth: BearerAuth) -> impl Responder {
    let claims = match validate_token(auth.token(), &data.oidc_jwks_uri).await {
        Ok(c) => c,
        Err(r) => return r,
    };

    let username = claims.preferred_username.or(claims.name).unwrap_or_else(|| "Anonymous".to_string());

    let mut con = match data.redis_client.get_async_connection().await {
        Ok(con) => con,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Redis error: {}", e)),
    };

    let score: Option<i32> = con.zscore("leaderboard", &username).await.ok();
    let rank: Option<i64> = con.zrevrank("leaderboard", &username).await.ok();

    match (score, rank) {
        (Some(s), Some(r)) => HttpResponse::Ok().json(LeaderboardEntry {
            username,
            score: s,
            rank: Some(r + 1),
        }),
        _ => HttpResponse::NotFound().body("User not found on leaderboard"),
    }
}
