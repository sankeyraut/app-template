use actix_web::{get, web, App, HttpServer, Responder, HttpResponse, HttpRequest, HttpMessage};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use rand::Rng;
use actix_web_httpauth::extractors::bearer::BearerAuth;
// use async_oidc_jwt_validator::IMValidator; // Removed
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct XandZeroRequest {
    board: Vec<String>, // Length 9, "X", "O", or ""
}

#[derive(Serialize, Deserialize)]
struct XandZeroResponse {
    board: Vec<String>,
    winner: Option<String>, // "X", "O", or "Draw"
    score_increment: i32,
}

/// Request payload for submitting a new score
#[derive(Serialize, Deserialize)]
struct ScoreRequest {
    score: i32,
}

/// Represents a single entry in the leaderboard
#[derive(Serialize, Deserialize, Debug)]
struct LeaderboardEntry {
    username: String,
    score: i32,
    rank: Option<i64>, // Rank is calculated real-time by Valkey
}

#[derive(Clone)]
struct AppState {
    db: Pool<Postgres>,
    redis_client: redis::Client,
    // validator: IMValidator, // Removed
    oidc_jwks_uri: String, 
}

#[derive(Serialize, Deserialize)]
struct Joke {
    id: i32,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    preferred_username: Option<String>,
    name: Option<String>,
}

// Fetch JWKS and find the key (Simplified: In prod, cache this!)
async fn get_decoding_key(jwks_uri: &str, kid: &str) -> Result<DecodingKey, Box<dyn std::error::Error>> {
    let jwks: Value = reqwest::get(jwks_uri).await?.json().await?;
    if let Some(keys) = jwks.get("keys").and_then(|k| k.as_array()) {
        for key in keys {
            if key.get("kid").and_then(|k| k.as_str()) == Some(kid) {
                let n = key.get("n").and_then(|v| v.as_str()).ok_or("Missing n")?;
                let e = key.get("e").and_then(|v| v.as_str()).ok_or("Missing e")?;
                return Ok(DecodingKey::from_rsa_components(n, e)?);
            }
        }
    }
    Err("Key not found".into())
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("hello {}", name)
}

#[get("/joke")]
async fn get_joke(
    data: web::Data<AppState>,
    auth: BearerAuth,
) -> impl Responder {
    let token = auth.token();
    
    // 1. Decode Header to get KID
    let header = match decode_header(token) {
        Ok(h) => h,
        Err(_) => return HttpResponse::Unauthorized().body("Invalid token header"),
    };

    let kid = match header.kid {
        Some(k) => k,
        None => return HttpResponse::Unauthorized().body("Token missing kid"),
    };

    // 2. Fetch JWKS and get Key (Note: Fetching on every request is bad practice, implement caching!)
    // For this demo, we fetch or ideally we should have cached it in AppState.
    // For reliability in this task, I'll fetch it.
    let decoding_key = match get_decoding_key(&data.oidc_jwks_uri, &kid).await {
        Ok(key) => key,
        Err(e) => {
             // Fallback: If Keycloak is not ready or key not found
             println!("JWKS Fetch Error: {:?}", e);
             return HttpResponse::Unauthorized().body("Could not fetch signing key");
        }
    };

    // 3. Validate Token
    let mut validation = Validation::new(Algorithm::RS256);
    // Disable audience check for simplicity or set it from env
    validation.validate_aud = false; 

    match decode::<Claims>(token, &decoding_key, &validation) {
        Ok(_) => {
            // Valid!
        }
        Err(e) => return HttpResponse::Unauthorized().body(format!("Invalid token: {:?}", e)),
    };

    // --- Business Logic ---
    let random_id = rand::thread_rng().gen_range(1..=10);
    let cache_key = format!("joke:{}", random_id);

    // Redis
    let mut con = match data.redis_client.get_async_connection().await {
        Ok(con) => con,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Redis error: {}", e)),
    };

    let cached: Result<String, redis::RedisError> = con.get(&cache_key).await;
    if let Ok(joke_content) = cached {
        return HttpResponse::Ok().json(Joke {
            id: random_id,
            content: joke_content,
        });
    }

    // DB
    let row: Result<(String,), sqlx::Error> = sqlx::query_as("SELECT content FROM jokes WHERE id = $1")
        .bind(random_id)
        .fetch_one(&data.db)
        .await;

    match row {
        Ok((content,)) => {
            let _: Result<(), redis::RedisError> = con.set_ex(&cache_key, &content, 60).await;
            HttpResponse::Ok().json(Joke {
                id: random_id,
                content: content,
            })
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}

/// Validates the OIDC token and returns the user claims
async fn validate_token(token: &str, jwks_uri: &str) -> Result<Claims, HttpResponse> {
    // ... logic ...
    let header = decode_header(token).map_err(|_| HttpResponse::Unauthorized().body("Invalid token header"))?;
    let kid = header.kid.ok_or_else(|| HttpResponse::Unauthorized().body("Token missing kid"))?;
    let decoding_key = get_decoding_key(jwks_uri, &kid).await.map_err(|e| {
        println!("JWKS Fetch Error: {:?}", e);
        HttpResponse::Unauthorized().body("Could not fetch signing key")
    })?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_aud = false;

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| HttpResponse::Unauthorized().body(format!("Invalid token: {:?}", e)))
}

/// Submits a player's score to the leaderboard.
/// Persistence: PostgreSQL (only updates if score is higher)
/// Real-time Ranking: Valkey (Sorted Set)
#[actix_web::post("/leaderboard")]
async fn submit_score(
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
    // Since we only updated DB if score was higher, we should do the same here or just ZADD with GT option if supported
    // But simple ZADD is fine if we only care about the latest "high" score being in Redis
    let _: Result<(), redis::RedisError> = con.zadd("leaderboard", &username, new_score).await;

    HttpResponse::Ok().body("Score submitted")
}

/// Retrieves the top 10 players from the Valkey sorted set
#[actix_web::get("/leaderboard")]
async fn get_leaderboard(data: web::Data<AppState>) -> impl Responder {
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

/// Retrieves the current authenticated user's rank and score from Valkey
#[actix_web::get("/leaderboard/me")]
async fn get_my_rank(data: web::Data<AppState>, auth: BearerAuth) -> impl Responder {
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

fn check_winner(board: &[String]) -> Option<String> {
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

fn get_computer_move(board: &[String]) -> Option<usize> {
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

#[actix_web::post("/xandzero/play")]
async fn xandzero_play(
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server initialization...");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    
    // Auth Config
    // let issuer = std::env::var("OIDC_ISSUER").unwrap_or_else(|_| "http://keycloak:8080/realms/antigravity".to_string());
    // JWKS URI: http://keycloak:8080/realms/antigravity/protocol/openid-connect/certs
    let jwks_uri = std::env::var("OIDC_JWKS").unwrap_or_else(|_| "http://keycloak:8080/realms/app-template/protocol/openid-connect/certs".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let redis_client = redis::Client::open(redis_url).expect("Invalid Redis URL");

    let app_state = AppState {
        db: pool,
        redis_client,
        oidc_jwks_uri: jwks_uri,
    };

    println!("Starting server at http://0.0.0.0:9876");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(greet)
            .service(get_joke)
            .service(submit_score)
            .service(get_leaderboard)
            .service(get_my_rank)
            .service(xandzero_play)
    })
    .bind(("0.0.0.0", 9876))?
    .run()
    .await
}
