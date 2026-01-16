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
    // Add other fields if needed
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server initialization...");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    
    // Auth Config
    // let issuer = std::env::var("OIDC_ISSUER").unwrap_or_else(|_| "http://keycloak:8080/realms/antigravity".to_string());
    // JWKS URI: http://keycloak:8080/realms/antigravity/protocol/openid-connect/certs
    let jwks_uri = std::env::var("OIDC_JWKS").unwrap_or_else(|_| "http://keycloak:8080/realms/antigravity/protocol/openid-connect/certs".to_string());

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
    })
    .bind(("0.0.0.0", 9876))?
    .run()
    .await
}
