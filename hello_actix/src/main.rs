use actix_web::{get, web, App, HttpServer, Responder, HttpResponse};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use rand::Rng;
use std::sync::Mutex;

#[derive(Clone)]
struct AppState {
    db: Pool<Postgres>,
    redis_client: redis::Client,
}

#[derive(Serialize, Deserialize)]
struct Joke {
    id: i32,
    content: String,
}

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("hello {}", name)
}

#[get("/joke")]
async fn get_joke(data: web::Data<AppState>) -> impl Responder {
    // 1. Generate random ID (1-10)
    let random_id = rand::thread_rng().gen_range(1..=10);
    let cache_key = format!("joke:{}", random_id);

    // 2. Try to get from Redis
    let mut con = match data.redis_client.get_async_connection().await {
        Ok(con) => con,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Redis error: {}", e)),
    };

    let cached: Result<String, redis::RedisError> = con.get(&cache_key).await;
    if let Ok(joke_content) = cached {
        println!("Cache HIT for joke {}", random_id);
        return HttpResponse::Ok().json(Joke {
            id: random_id,
            content: joke_content,
        });
    }

    // 3. If cache miss, fetch from DB
    println!("Cache MISS for joke {}", random_id);
    let row: Result<(String,), sqlx::Error> = sqlx::query_as("SELECT content FROM jokes WHERE id = $1")
        .bind(random_id)
        .fetch_one(&data.db)
        .await;

    match row {
        Ok((content,)) => {
            // 4. Store in Redis with TTL (e.g., 60 seconds)
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

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    let redis_client = redis::Client::open(redis_url).expect("Invalid Redis URL");

    let app_state = AppState {
        db: pool,
        redis_client,
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
