
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use rand::Rng;
use actix_web_httpauth::extractors::bearer::BearerAuth;
// use async_oidc_jwt_validator::IMValidator; // Removed
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use serde_json::Value;
mod dragonballgame;

use actix_web::{get, web, App, HttpRequest, HttpServer, Responder, HttpResponse, Error};
use actix_ws::Message;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use tokio::time::interval;

mod xandzero;
mod leaderboard;

/// Represents a single entry in the leaderboard (used in response JSON)
#[derive(Serialize, Deserialize, Debug)]
pub struct LeaderboardEntry {
    pub username: String,
    pub score: i32,
    pub rank: Option<i64>, // Real-time rank from Valkey/Redis
}

/// Shared application state injected into Actix handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,          // PostgreSQL connection pool
    pub redis_client: redis::Client, // Valkey/Redis client
    pub oidc_jwks_uri: String,       // Keycloak JWKS endpoint
}

#[derive(Serialize, Deserialize)]
struct Joke {
    id: i32,
    content: String,
}

/// Claims structure for JWT token decoding
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,                      // User ID from Keycloak
    pub exp: usize,                       // Expiration timestamp
    pub preferred_username: Option<String>, // Keycloak username
    pub name: Option<String>,             // User's full name
}

/// Fetches the JSON Web Key Set (JWKS) and returns the decoding key for a specific kid
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
pub async fn validate_token(token: &str, jwks_uri: &str) -> Result<Claims, HttpResponse> {
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


#[derive(Deserialize)]
struct WsQuery {
    token: String,
}

async fn dragon_socket(
    req: HttpRequest, 
    stream: web::Payload,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    // 1. Manually Parse Query
    let query_string = req.query_string();
    let token = match serde_urlencoded::from_str::<WsQuery>(query_string) {
        Ok(q) => q.token,
        Err(_) => return Ok(HttpResponse::Unauthorized().body("Missing or invalid token")),
    };

    // 2. Validate Token
    let claims = match validate_token(&token, &data.oidc_jwks_uri).await {
        Ok(c) => c,
        Err(_) => return Ok(HttpResponse::Unauthorized().finish()),
    };

    let user_id = claims.sub.clone();
    let username = claims.preferred_username.or(claims.name).unwrap_or_else(|| "Anonymous".to_string());
    
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let game_state = Arc::new(Mutex::new(dragonballgame::GameState::new()));
    let game_state_clone = game_state.clone();
    let mut session_clone = session.clone();
    let db_pool = data.db.clone();
    let redis_client = data.redis_client.clone();

    // 2. Input Handler Task (Client -> Server)
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = futures_util::StreamExt::next(&mut msg_stream).await {
            match msg {
                Message::Text(text) => {
                    if let Ok(player_input) = serde_json::from_str::<dragonballgame::Player>(&text) {
                        if let Ok(mut state) = game_state.lock() {
                            state.update_player_pos(player_input.y);
                        }
                    }
                }
                Message::Ping(bytes) => {
                    if session.pong(&bytes).await.is_err() {
                        break;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // 3. Game Loop Task (Server -> Client)
    actix_web::rt::spawn(async move {
        let mut ticker = interval(Duration::from_millis(16)); // ~60 FPS
        let mut score_saved = false;

        loop {
            ticker.tick().await;

            let (state_json, game_over, score) = {
                let mut state = game_state_clone.lock().unwrap();
                state.tick();
                (serde_json::to_string(&*state).unwrap(), state.game_over, state.score)
            };

            // Check for Game Over Persistence
            if game_over && !score_saved {
                // Save Score
                score_saved = true;
                
                // Update DB
                let _ = sqlx::query(
                    "INSERT INTO leaderboard (user_id, game_name, username, score) 
                     VALUES ($1, 'dragonball', $2, $3) 
                     ON CONFLICT (user_id, game_name) DO UPDATE SET score = GREATEST(leaderboard.score, $3)"
                )
                .bind(&user_id)
                .bind(&username)
                .bind(score as i32)
                .execute(&db_pool)
                .await; // Note: In a real app, handle error logging

                // Update Redis
                if let Ok(mut con) = redis_client.get_async_connection().await {
                   // We want to store the BEST score in leaderboard, usually. 
                   // But ZADD just updates. So we should probably check if new score is higher?
                   // Redis ZADD updates if score is different.
                   // Logic: Fetch best from DB or Redis? 
                   // For this simple game, let's just push the latest score if it depends on "high score" semantics.
                   // But if I play twice and get lower score, I shouldn't overwrite high score in Redis?
                   // The SQL query used GREATEST.
                   // Let's use ZADD GT (Greater Than) if supported, or just trust the DB flow?
                   // Let's re-fetch the max score from DB to be safe and consistent.
                    let best_score: i32 = sqlx::query_scalar("SELECT score FROM leaderboard WHERE user_id = $1 AND game_name = 'dragonball'")
                        .bind(&user_id)
                        .fetch_one(&db_pool)
                        .await
                        .unwrap_or(score as i32);

                    let _: Result<(), redis::RedisError> = con.zadd("leaderboard:dragonball", &username, best_score).await;
                }
            }

            if session_clone.text(state_json).await.is_err() {
                break;
            }
            
            if game_over && score_saved {
                 // Game is over and saved. We can stop the loop or keep sending end state?
                 // Usually keep sending so client sees "Game Over" screen.
                 // But we don't need to tick logic anymore.
                 // Just continue to render static state or break if we want to close connection?
                 // Let's continue so client can see the screen.
            }
        }
    });

    Ok(res)
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
            .service(leaderboard::submit_score)
            .service(leaderboard::get_leaderboard)
            .service(leaderboard::get_my_rank)
            .service(xandzero::xandzero_play)
            .route("/dragon_ws", web::get().to(dragon_socket))
    })
    .bind(("0.0.0.0", 9876))?
    .run()
    .await
}
