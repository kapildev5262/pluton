use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::solsniffer::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolCreationEvent {
    pub dex_name: String,
    pub pool_address: Option<String>,
    pub token_a: TokenData,
    pub token_b: TokenData,
    pub timestamp: String,
    pub transaction_signature: String,
    pub liquidity_amounts: LiquidityInfo,
    pub coin_type: Coin,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiquidityInfo {
    pub token_a_amount: Option<String>,
    pub token_b_amount: Option<String>,
    pub sol_amount: Option<String>,

    pub token_a_amount_formatted: Option<String>,
    pub token_b_amount_formatted: Option<String>,
    pub sol_amount_formatted: Option<String>,
}

pub fn format_token_amount(raw_amount: &str, decimals: u8) -> String {
    if let Ok(amount) = raw_amount.parse::<u64>() {
        let decimal_places = 10_u64.pow(decimals as u32);
        let formatted = amount as f64 / decimal_places as f64;

        if formatted >= 1_000_000_000.0 {
            format!("{:.2}B", formatted / 1_000_000_000.0)
        } else if formatted >= 1_000_000.0 {
            format!("{:.2}M", formatted / 1_000_000.0)
        } else if formatted >= 1_000.0 {
            format!("{:.2}K", formatted / 1_000.0)
        } else {
            format!("{:.6}", formatted)
        }
    } else {
        raw_amount.to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Coin {
    ImmediateCoin,
    ShortTermCoin,
    GenuineCoin,
    MegaCoin,
}

impl Coin {
    pub fn coin_class(liquidity: f32) -> Self {
        match liquidity {
            x if x < 120.0 => Coin::ImmediateCoin,
            x if x >= 120.0 && x < 200.0 => Coin::ShortTermCoin,
            x if x >= 200.0 && x <= 4000.0 => Coin::GenuineCoin,
            x if x > 4000.0 => Coin::MegaCoin,
            _ => {
                eprintln!(
                    "Warning: Unhandled or invalid liquidity value encountered: {}",
                    liquidity
                );
                Coin::ImmediateCoin
            }
        }
    }
}

// Configuration for each DEX
#[derive(Debug)]
struct DexConfig {
    name: String,
    method_name: String,
    token_a_account: String,
    token_b_account: String,
    pool_account: String,
    token_a_amount_arg: String,
    token_b_amount_arg: String,
    token_a_key: String,
    token_b_key: String,
}

impl DexConfig {
    fn get_config(dex_name: &str) -> Option<Self> {
        match dex_name {
            "raydium" => Some(DexConfig {
                name: "Raydium".to_string(),
                method_name: "initialize2".to_string(),
                token_a_account: "pcMint".to_string(),
                token_b_account: "coinMint".to_string(),
                pool_account: "amm".to_string(),
                token_a_amount_arg: "initPcAmount".to_string(),
                token_b_amount_arg: "initCoinAmount".to_string(),
                token_a_key: "pc_mint".to_string(),
                token_b_key: "coin_mint".to_string(),
            }),
            "pumpswap" => Some(DexConfig {
                name: "PumpSwap".to_string(),
                method_name: "create_pool".to_string(),
                token_a_account: "base_mint".to_string(),
                token_b_account: "quote_mint".to_string(),
                pool_account: "pool".to_string(),
                token_a_amount_arg: "base_amount_in".to_string(),
                token_b_amount_arg: "quote_amount_in".to_string(),
                token_a_key: "base_mint".to_string(),
                token_b_key: "quote_mint".to_string(),
            }),
            "meteora" => Some(DexConfig {
                name: "Meteora".to_string(),
                method_name: "initializePermissionlessConstantProductPoolWithConfig2".to_string(),
                token_a_account: "tokenAMint".to_string(),
                token_b_account: "tokenBMint".to_string(),
                pool_account: "pool".to_string(),
                token_a_amount_arg: "tokenAAmount".to_string(),
                token_b_amount_arg: "tokenBAmount".to_string(),
                token_a_key: "token_a_mint".to_string(),
                token_b_key: "token_b_mint".to_string(),
            }),
            _ => None,
        }
    }
}

// Made this function async since it calls parse_dex_event which is async
pub async fn parse_pool_creation_event(
    subscription_name: &str,
    data: &Value,
) -> Option<PoolCreationEvent> {
    println!("🔍 Parsing data for {}", subscription_name);

    if let Some(msg_type) = data.get("type").and_then(|t| t.as_str()) {
        match msg_type {
            "pong" | "connection_ack" | "connection_error" | "ka" => {
                println!(
                    "⏭️  Skipping control message '{}' for {} \n",
                    msg_type, subscription_name
                );
                return None;
            }
            "next" => {
                println!("✅ Processing data message for {}", subscription_name);
            }
            _ => {
                println!(
                    "🔍 Unknown message type '{}' for {}",
                    msg_type, subscription_name
                );
            }
        }
    }

    let payload = if let Some(payload) = data.get("payload") {
        println!("Got Payload");
        payload
    } else if let Some(data_field) = data.get("data") {
        println!("Got data_field");
        data_field
    } else {
        println!(
            "❌ No payload or data field found for {}",
            subscription_name
        );
        return None;
    };

    let instructions = if let Some(solana_data) = payload.get("data").and_then(|d| d.get("Solana"))
    {
        solana_data.get("Instructions")?.as_array()?
    } else if let Some(instructions) = payload.get("Solana").and_then(|s| s.get("Instructions")) {
        instructions.as_array()?
    } else {
        println!(
            "❌ No Instructions found in payload for {}",
            subscription_name
        );
        return None;
    };

    if instructions.is_empty() {
        println!("❌ Empty instructions array for {}", subscription_name);
        return None;
    }

    let instruction = &instructions[0];
    let block = instruction.get("Block")?;
    let transaction = instruction.get("Transaction")?;
    let instruction_data = instruction.get("Instruction")?;

    let timestamp = block.get("Time")?.as_str()?.to_string();
    let signature = transaction.get("Signature")?.as_str()?.to_string();

    println!(
        "✅ Found instruction data for {}: timestamp={}, signature={}",
        subscription_name, timestamp, signature
    );

    // Now properly await the async function
    let result = parse_dex_event(subscription_name, instruction_data, timestamp, signature).await;

    if result.is_some() {
        println!(
            "✅ Successfully parsed pool creation event for {}",
            subscription_name
        );
    } else {
        println!(
            "❌ Failed to parse pool creation event for {}",
            subscription_name
        );
    }

    result
}

async fn parse_dex_event(
    dex_name: &str,
    instruction: &Value,
    timestamp: String,
    signature: String,
) -> Option<PoolCreationEvent> {
    // println!("Got data for parsing: {}",serde_json::to_string_pretty(instruction).unwrap_or_default());

    let accounts = instruction.get("Accounts")?.as_array()?;
    let program = instruction.get("Program")?;

    let program_name = program.get("Name")?.as_str()?;
    let method_name = program.get("Method")?.as_str().unwrap_or("");
    println!("method name:{}", method_name);

    // Handle combined case by matching method name to appropriate config
    let config = if dex_name == "combined" {
        // Try to find a config that matches the method name
        let dex_names = ["raydium", "pumpswap", "meteora"];
        let mut matching_config = None;

        for &dex in &dex_names {
            if let Some(test_config) = DexConfig::get_config(dex) {
                println!("config method name:{}", method_name);
                if test_config.method_name == method_name {
                    matching_config = Some(test_config);
                    break;
                }
            }
        }

        if let Some(config) = matching_config {
            println!(
                "🎯 Combined mode: matched method '{}' to {} config",
                method_name, config.name
            );
            config
        } else {
            println!(
                "❌ Combined mode: no config found for method '{}'",
                method_name
            );
            return None;
        }
    } else {
        DexConfig::get_config(dex_name)?
    };

    println!(
        "📋 {} program name: {}, method: {}",
        config.name, program_name, method_name
    );

    if method_name != config.method_name {
        println!(
            "⏭️  Not a {} pool creation instruction: method = {}",
            config.name, method_name
        );
        return None;
    }

    // Extract liquidity amounts from arguments
    let (token_a_amount, token_b_amount) = extract_liquidity_amounts(program, &config);
    println!(
        "💰 Extracted amounts - Token A: {:?}, Token B: {:?}",
        token_a_amount, token_b_amount
    );

    // Extract token addresses and pool address from accounts
    let (token_addresses, pool_address) =
        extract_addresses_from_accounts(accounts, program, &config);

    println!(
        "🔢 Found {} token addresses: {:?}",
        token_addresses.len(),
        token_addresses
    );
    println!("🏊 Pool address: {:?}", pool_address);

    // Ensure we have exactly 2 tokens for a pair
    if token_addresses.len() != 2 {
        println!(
            "❌ Expected exactly 2 tokens, found: {}",
            token_addresses.len()
        );
        return None;
    }

    // Extract token addresses from HashMap
    let token_a_address = token_addresses.get(&config.token_a_key)?.clone();
    let token_b_address = token_addresses.get(&config.token_b_key)?.clone();

    let token_data_a: TokenData;
    let token_data_b: TokenData;

    // Properly handle the Result from sniffer function
    if token_a_address == "So11111111111111111111111111111111111111112" {
        token_data_a = create_fallback_for_wsol(&token_a_address)
    } else {
        token_data_a = match sniffer(&token_a_address).await {
            Ok(token_data) => {
                println!("✅ Analysis complete for token A: {}", token_data.address);
                token_data
            }
            Err(e) => {
                eprintln!(
                    "❌ Failed to get token A data for {}: {}",
                    token_a_address, e
                );
                create_fallback_token_data(&token_a_address)
            }
        };
    }

    if token_b_address == "So11111111111111111111111111111111111111112" {
        token_data_b = create_fallback_for_wsol(&token_b_address)
    } else {
        token_data_b = match sniffer(&token_b_address).await {
            Ok(token_data) => {
                println!("✅ Analysis complete for token B: {}", token_data.address);
                token_data
            }
            Err(e) => {
                eprintln!(
                    "❌ Failed to get token B data for {}: {}",
                    token_b_address, e
                );
                create_fallback_token_data(&token_b_address)
            }
        };
    }

    // Determine SOL amount and convert to f32 for classification
    let sol_amount = determine_sol_amount(
        &token_a_address,
        &token_b_address,
        &token_a_amount,
        &token_b_amount,
    );

    let sol_amount_f32: f32 = if let Some(ref sol_str) = sol_amount {
        // Convert from raw amount to SOL (divide by 10^9)
        match sol_str.parse::<u64>() {
            Ok(raw_amount) => {
                let sol_formatted = raw_amount as f64 / 1_000_000_000.0;
                sol_formatted as f32
            }
            Err(e) => {
                eprintln!("Failed to parse SOL amount '{}' to u64: {}", sol_str, e);
                0.0
            }
        }
    } else {
        eprintln!("No SOL amount found in liquidity pair");
        0.0
    };

    println!("💰 SOL amount for classification: {}", sol_amount_f32);

    let coin_type = Coin::coin_class(sol_amount_f32);

    // Format amounts for display
    let token_a_amount_formatted = token_a_amount
        .as_ref()
        .map(|amount| format_token_amount(amount, token_data_a.decimals));

    let token_b_amount_formatted = token_b_amount
        .as_ref()
        .map(|amount| format_token_amount(amount, token_data_b.decimals));

    let sol_amount_formatted = sol_amount.as_ref().map(|amount| {
        format_token_amount(amount, 9) // SOL has 9 decimals
    });

    Some(PoolCreationEvent {
        dex_name: config.name,
        pool_address,
        token_a: token_data_a,
        token_b: token_data_b,
        timestamp,
        transaction_signature: signature,
        liquidity_amounts: LiquidityInfo {
            token_a_amount: token_a_amount.clone(),
            token_b_amount: token_b_amount.clone(),
            sol_amount,
            token_a_amount_formatted,
            token_b_amount_formatted,
            sol_amount_formatted,
        },
        coin_type,
    })
}

// Helper function to create fallback TokenData when API fails
fn create_fallback_token_data(address: &str) -> TokenData {
    TokenData {
        address: address.to_string(),
        token_name: "Unknown".to_string(),
        token_symbol: "UNKNOWN".to_string(),
        decimals: 9,
        market_cap: 0.0,
        score: 0,
        risk_level: RiskLevel::Critical, // Mark as critical when we can't analyze
        price: 0.0,
        supply_amount: 0.0,
        liquidity_total: 0.0,
        top_10_percentage: 0.0,
        holder_count: 0,
        is_honeypot: true, // Mark as honeypot when we can't verify
        audit_risks: vec!["Unable to analyze - API error".to_string()],
        deploy_time: "Unknown".to_string(),
        mint_disabled: false,
        freeze_disabled: false,
        lp_burned: false,
    }
}

fn create_fallback_for_wsol(address: &str) -> TokenData {
    TokenData {
        address: address.to_string(),
        token_name: "Wrapped SOL".to_string(),
        token_symbol: "WSOL".to_string(),
        decimals: 9,
        market_cap: 0.0,
        score: 100,
        risk_level: RiskLevel::Low,
        price: 0.0,
        supply_amount: 0.0,
        liquidity_total: 0.0,
        top_10_percentage: 0.0,
        holder_count: 0,
        is_honeypot: false,
        audit_risks: vec!["No risk".to_string()],
        deploy_time: "Unknown".to_string(),
        mint_disabled: false,
        freeze_disabled: false,
        lp_burned: false,
    }
}

fn extract_liquidity_amounts(
    program: &Value,
    config: &DexConfig,
) -> (Option<String>, Option<String>) {
    let mut token_a_amount = None;
    let mut token_b_amount = None;

    if let Some(arguments) = program.get("Arguments").and_then(|a| a.as_array()) {
        for arg in arguments {
            if let Some(name) = arg.get("Name").and_then(|n| n.as_str()) {
                match name {
                    name if name == config.token_a_amount_arg => {
                        if let Some(value) = arg.get("Value") {
                            if let Some(big_int) =
                                value.get("bigInteger").and_then(|bi| bi.as_str())
                            {
                                token_a_amount = Some(big_int.to_string());
                            }
                        }
                    }
                    name if name == config.token_b_amount_arg => {
                        if let Some(value) = arg.get("Value") {
                            if let Some(big_int) =
                                value.get("bigInteger").and_then(|bi| bi.as_str())
                            {
                                token_b_amount = Some(big_int.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (token_a_amount, token_b_amount)
}

fn extract_addresses_from_accounts(
    accounts: &[Value],
    program: &Value,
    config: &DexConfig,
) -> (HashMap<String, String>, Option<String>) {
    let mut token_addresses: HashMap<String, String> = HashMap::new();
    let mut pool_address = None;

    let account_names = program
        .get("AccountNames")
        .and_then(|names| names.as_array());

    for (index, account) in accounts.iter().enumerate() {
        if let Some(address) = account.get("Address").and_then(|a| a.as_str()) {
            // Get account name from the mapping if available
            let account_name = account_names
                .and_then(|names| names.get(index))
                .and_then(|name| name.as_str())
                .unwrap_or("");

            println!("🔍 Account {}: {} ({})", index, address, account_name);

            // Skip system programs and long token addresses
            if should_skip_address(address) {
                continue;
            }

            // Extract addresses based on account names
            match account_name {
                name if name == config.token_a_account => {
                    token_addresses.insert(config.token_a_key.clone(), address.to_string());
                    println!("🪙 Found token A mint: {}", address);
                }
                name if name == config.token_b_account => {
                    token_addresses.insert(config.token_b_key.clone(), address.to_string());
                    println!("🪙 Found token B mint: {}", address);
                }
                name if name == config.pool_account => {
                    pool_address = Some(address.to_string());
                    println!("🏊 Found pool address: {}", address);
                }
                _ => {}
            }
        }
    }

    (token_addresses, pool_address)
}

fn should_skip_address(address: &str) -> bool {
    address == "11111111111111111111111111111111"
        || (address.contains("Token") && address.len() > 50)
        || address == "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"
}

fn determine_sol_amount(
    token_a_address: &str,
    token_b_address: &str,
    token_a_amount: &Option<String>,
    token_b_amount: &Option<String>,
) -> Option<String> {
    let sol_address = "So11111111111111111111111111111111111111112";

    if token_a_address == sol_address {
        token_a_amount.clone()
    } else if token_b_address == sol_address {
        token_b_amount.clone()
    } else {
        None
    }
}
