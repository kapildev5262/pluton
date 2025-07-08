use anyhow::{Result};

use async_tungstenite::tungstenite::{
    client::IntoClientRequest,
    http::{header, HeaderValue},
    Message,
};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use prettytable::{Table, Row, Cell, format};

use actix_web::{get, App, HttpServer, HttpResponse, web::Bytes, Result as ActixResult};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use actix_cors::Cors;

mod subscriptions;
mod parsing;
mod solsniffer;

use subscriptions::*;
use parsing::*;



// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     println!("Server starting on http://127.0.0.1:8080");
//     println!("Access the SSE stream at http://127.0.0.1:8080/stream");

//     HttpServer::new(|| {
//         App::new().service(subscribe_to_bitquery)
//     })
//     .bind(("127.0.0.1", 8080))?
//     .run()
//     .await
// }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server starting on http://127.0.0.1:8080");
    println!("Access the SSE stream at http://127.0.0.1:8080/stream");

    HttpServer::new(|| {
        // Configure CORS middleware
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000") // Allow your Next.js frontend
            .allowed_methods(vec!["GET"]) // SSE uses GET requests
            .allowed_headers(vec![header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION]) // Include headers your client might send
            .max_age(3600); // Cache preflight requests for 1 hour

        App::new()
            .wrap(cors) // Apply the CORS middleware to your application
            .service(subscribe_to_bitquery)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}




#[get("/stream")]
pub async fn subscribe_to_bitquery() -> ActixResult<HttpResponse> {

    dotenv::dotenv().ok();
    let oauth_token = std::env::var("BITQUERY_TOKEN").unwrap_or_else(|_| {
        std::env::var("BITQUERY_TOKEN").unwrap_or_else(|_| {
            eprintln!("❌ BITQUERY_TOKEN not found");
            "".to_string()
        })
    });

    println!("🎯 Setting up multiple concurrent subscriptions...");

    let subscriptions = vec![
        // ("raydium", RAYDIUM_SUBSCRIPTION),
        // ("pumpswap", PUMPSWAP_SUBSCRIPTION),
        // ("meteora", METEORA_SUBSCRIPTION),
        ("combined", COMBINED),
    ];

    let (tx, rx) = mpsc::channel::<String>(50);
    
    for (sub_name, query) in subscriptions {

        let oauth_token_clone = oauth_token.clone();
        let tx_clone = tx.clone();
        let sub_name = sub_name.to_string();
        let query = query.to_string();
        
        tokio::spawn(async move {
            if let Err(e) = handle_subscription(oauth_token_clone, sub_name.clone(), query, tx_clone).await {
                println!("❌ Subscription '{}' failed: {}", sub_name, e);
            }
        });
    }

    println!("🎧 Listening for real-time data from all subscriptions...\n");
    
    let event_stream = ReceiverStream::new(rx)
        .map(|data| {
            Ok::<Bytes, actix_web::Error>(data.into())
        });

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(event_stream))
}


async fn handle_subscription(
    oauth_token: String,
    subscription_name: String,
    query: String,
    tx: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔌 Connecting to Bitquery WebSocket for '{}'...", subscription_name);
    
    let mut request = "wss://streaming.bitquery.io/eap".into_client_request()?;

    request.headers_mut().insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_str("graphql-transport-ws")?,
    );
    request.headers_mut().insert(
        "Authorization", 
        HeaderValue::from_str(&format!("Bearer {}", oauth_token))?,
    );

    let (ws_stream, _) = async_tungstenite::tokio::connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();

    println!("✅ Connected '{}'! Initializing connection...", subscription_name);

    let init_msg = json!({
        "type": "connection_init"
    });
    write.send(Message::Text(init_msg.to_string())).await?;

    let mut connection_acked = false;
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            let parsed: serde_json::Value = serde_json::from_str(&text)?;
            match parsed["type"].as_str() {
                Some("connection_ack") => {
                    println!("🤝 Connection acknowledged for '{}'!", subscription_name);
                    connection_acked = true;
                    break;
                }
                Some("connection_error") => {
                    return Err(format!("Connection error for '{}': {}", subscription_name, parsed["payload"]).into());
                }
                _ => {
                    println!("📥 Received init message for '{}': {}", subscription_name, text);
                }
            }
        }
    }

    if !connection_acked {
        return Err(format!("Failed to get connection acknowledgment for '{}'", subscription_name).into());
    }

    let subscription_msg = json!({
        "id": format!("{}_{}", subscription_name, chrono::Utc::now().timestamp()),
        "type": "start",
        "payload": {
            "query": query
        }
    });
    
    println!("📡 Sending subscription for '{}'...", subscription_name);
    write.send(Message::Text(subscription_msg.to_string())).await?;

    while let Some(msg) = read.next().await {
        if let Ok(msg) = msg {
            if let Message::Text(text) = msg {
                let raw_data: Value = serde_json::from_str(&text).unwrap_or(json!({"raw": text}));
                
                // Parse and transform the data - now properly await the async function
                if let Some(mut pool_event) = parse_pool_creation_event(&subscription_name, &raw_data).await {

                    pool_event = format_pool_event_amounts(pool_event).await;

                    print_pool_event_table(&pool_event);

                    let formatted_message = json!({
                        "event_type": "pool_creation",
                        "subscription": pool_event.dex_name,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "data": pool_event
                    });
                    
                    println!("🎯 New pool created on {}: {} - {}", 
                        pool_event.dex_name, 
                        pool_event.token_a.address, 
                        pool_event.token_b.address
                    );
                    
                    // Send the formatted message
                    let formatted_json = serde_json::to_string(&formatted_message)?;
                    if tx.send(format!("data: {}\n\n", formatted_json)).await.is_err() {
                        println!("Server: Client disconnected, stopping '{}' stream.", subscription_name);
                        break;
                    }
                } else {
                    // For debugging - send raw data with cleaner format
                    let tagged_message = json!({
                        "event_type": "raw",
                        "subscription": subscription_name,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "data": raw_data
                    });
                    
                    let raw_json = serde_json::to_string(&tagged_message)?;
                    if tx.send(format!("data: {}\n\n", raw_json)).await.is_err() {
                        println!("Server: Client disconnected, stopping '{}' stream.", subscription_name);
                        break;
                    }
                }
            }
        }
    }
    
    Ok(())
}



async fn format_pool_event_amounts(
    mut pool_event: PoolCreationEvent
) -> PoolCreationEvent {
    // Get decimals for both tokens
    let token_a_decimals = pool_event.token_a.decimals;
    let token_b_decimals = pool_event.token_b.decimals;

    // Format amounts if they exist
    if let Some(ref amount) = pool_event.liquidity_amounts.token_a_amount {
        pool_event.liquidity_amounts.token_a_amount_formatted = 
            Some(format_token_amount(amount, token_a_decimals));
    }

    if let Some(ref amount) = pool_event.liquidity_amounts.token_b_amount {
        pool_event.liquidity_amounts.token_b_amount_formatted = 
            Some(format_token_amount(amount, token_b_decimals));
    }

    if let Some(ref amount) = pool_event.liquidity_amounts.sol_amount {
        pool_event.liquidity_amounts.sol_amount_formatted = 
            Some(format_token_amount(amount, 9)); 
    }

    pool_event
}

fn print_pool_event_table(pool_event: &PoolCreationEvent) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_BOX_CHARS);
    
    // Header
    table.add_row(Row::new(vec![
        Cell::new("🎯 NEW POOL CREATED").style_spec("bFg"),
    ]));
    
    // DEX Information
    table.add_row(Row::new(vec![
        Cell::new("DEX"),
        Cell::new(&pool_event.dex_name).style_spec("bFc"),
    ]));
    
    // Pool Address
    if let Some(pool_addr) = &pool_event.pool_address {
        table.add_row(Row::new(vec![
            Cell::new("Pool Address"),
            Cell::new(&format!("{}...{}", &pool_addr[..8], &pool_addr[pool_addr.len()-8..])).style_spec("Fc"),
        ]));
    }
    
    // Token Information
    table.add_row(Row::new(vec![
        Cell::new("Token A"),
        Cell::new(&format!("{}...{}", &pool_event.token_a.address[..8], &pool_event.token_a.address[pool_event.token_a.address.len()-8..])).style_spec("Fy"),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Token B"),
        Cell::new(&format!("{}...{}", &pool_event.token_b.address[..8], &pool_event.token_b.address[pool_event.token_b.address.len()-8..])).style_spec("Fy"),
    ]));
    
    // Liquidity Information
    if let Some(token_a_amount) = &pool_event.liquidity_amounts.token_a_amount_formatted {
        table.add_row(Row::new(vec![
            Cell::new("Token A Amount"),
            Cell::new(&(token_a_amount)).style_spec("Fg"),
        ]));
    }
    
    if let Some(token_b_amount) = &pool_event.liquidity_amounts.token_b_amount_formatted {
        table.add_row(Row::new(vec![
            Cell::new("Token B Amount"),
            Cell::new(&(token_b_amount)).style_spec("Fg"),
        ]));
    }
    
    if let Some(sol_amount) = &pool_event.liquidity_amounts.sol_amount_formatted {
        table.add_row(Row::new(vec![
            Cell::new("SOL Amount"),
            Cell::new(&format!("{} SOL", sol_amount)).style_spec("bFg"),
        ]));
    }
    
    // Transaction Information
    table.add_row(Row::new(vec![
        Cell::new("Transaction"),
        Cell::new(&format!("{}...{}", &pool_event.transaction_signature[..8], &pool_event.transaction_signature[pool_event.transaction_signature.len()-8..])).style_spec("Fb"),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Timestamp"),
        Cell::new(&pool_event.timestamp).style_spec("Fd"),
    ]));
    
    println!("\n{}", table);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
