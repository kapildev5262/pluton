use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, anyhow};
use tokio::time::{Duration};use dotenv::dotenv;
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenData {
    pub address: String,
    pub token_name: String,
    pub token_symbol: String,
    pub decimals: u8,
    pub market_cap: f64,
    pub score: u32,
    pub risk_level: RiskLevel,
    pub price: f64,
    pub supply_amount: f64,
    pub liquidity_total: f64,
    pub top_10_percentage: f64,
    pub holder_count: u32,
    pub is_honeypot: bool,
    pub audit_risks: Vec<String>,
    pub deploy_time: String,
    pub mint_disabled: bool,
    pub freeze_disabled: bool,
    pub lp_burned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium, 
    High,
    Critical,
}

impl RiskLevel {
    fn as_emoji(&self) -> &str {
        match self {
            RiskLevel::Low => "🟢",
            RiskLevel::Medium => "🟡", 
            RiskLevel::High => "🟠",
            RiskLevel::Critical => "🔴",
        }
    }

    fn from_score(score: u32) -> Self {
        match score {
            0..=25 => RiskLevel::Critical,
            26..=50 => RiskLevel::High,
            51..=75 => RiskLevel::Medium,
            76..=100 => RiskLevel::Low,
            _ => RiskLevel::Medium,
        }
    }
}


pub struct SolSnifferClient {
    client: Client,
    api_key: String,
}

impl SolSnifferClient {
    pub fn new() -> Result<Self> {
        dotenv().ok();
    
        let api_key = env::var("SOLSNIFER_KEY")
        .map_err(|_| anyhow!("SOLSNIFER_KEY should be set in .env file"))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("SolSniffer-Integration/1.0.0")
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self { client, api_key })
    }

    pub async fn analyze_token(&self, token_address: &str) -> Result<TokenData> {
        let url = format!("https://solsniffer.com/api/v2/token/{}", token_address);
        
        let response = self.client
            .get(&url)
            .header("accept", "application/json")
            .header("X-API-KEY", &self.api_key)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow!("API request failed: {} - {}", status, error_body));
        }

        let body = response.text().await
            .map_err(|e| anyhow!("Failed to read response body: {}", e))?;
        
        let json: Value = serde_json::from_str(&body)
            .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

        self.parse_token_data(&json)
    }

    fn parse_token_data(&self, json: &Value) -> Result<TokenData> {
        let token_data = json.get("tokenData")
            .ok_or_else(|| anyhow!("Missing tokenData in response"))?;
        
        let token_info = json.get("tokenInfo")
            .ok_or_else(|| anyhow!("Missing tokenInfo in response"))?;

        // Extract basic token information
        let address = token_data.get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let token_name = token_data.get("tokenName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let token_symbol = token_data.get("tokenSymbol")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();

        let decimals = token_data.get("decimals")
            .and_then(|v| v.as_u64())
            .unwrap_or(9) as u8;

        let market_cap = token_data.get("marketCap")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let score = token_data.get("score")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let price = token_info.get("price")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let supply_amount = token_info.get("supplyAmount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let deploy_time = token_data.get("deployTime")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Extract audit risk information
        let audit_risk = token_data.get("auditRisk");
        let mint_disabled = audit_risk
            .and_then(|ar| ar.get("mintDisabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let freeze_disabled = audit_risk
            .and_then(|ar| ar.get("freezeDisabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let lp_burned = audit_risk
            .and_then(|ar| ar.get("lpBurned"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Calculate liquidity total
        let liquidity_total = self.calculate_total_liquidity(token_data)?;

        // Calculate top 10 holders percentage
        let (top_10_percentage, holder_count) = self.calculate_holder_metrics(token_data)?;

        // Determine risk level
        let risk_level = RiskLevel::from_score(score);

        // Extract audit risks
        let audit_risks = self.extract_audit_risks(token_data)?;

        // Honeypot detection
        let is_honeypot = self.detect_honeypot(token_data, &audit_risks)?;

        Ok(TokenData {
            address,
            token_name,
            token_symbol,
            decimals,
            market_cap,
            score,
            risk_level,
            price,
            supply_amount,
            liquidity_total,
            top_10_percentage,
            holder_count,
            is_honeypot,
            audit_risks,
            deploy_time,
            mint_disabled,
            freeze_disabled,
            lp_burned,
        })
    }

    fn calculate_total_liquidity(&self, token_data: &Value) -> Result<f64> {
        let liquidity_list = token_data.get("liquidityList")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing or invalid liquidityList"))?;

        let mut total_liquidity = 0.0;
        for liquidity_item in liquidity_list {
            if let Some(obj) = liquidity_item.as_object() {
                for (_, platform_data) in obj {
                    if let Some(amount) = platform_data.get("amount").and_then(|v| v.as_f64()) {
                        total_liquidity += amount;
                    }
                }
            }
        }

        Ok(total_liquidity)
    }

    fn calculate_holder_metrics(&self, token_data: &Value) -> Result<(f64, u32)> {
        let owners_list = token_data.get("ownersList")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Missing or invalid ownersList"))?;

        let holder_count = owners_list.len() as u32;
        
        // Calculate top 10 percentage
        let top_10_percentage: f64 = owners_list
            .iter()
            .take(10)
            .filter_map(|owner| {
                owner.get("percentage")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
            })
            .sum();

        Ok((top_10_percentage, holder_count))
    }

    fn extract_audit_risks(&self, token_data: &Value) -> Result<Vec<String>> {
        let mut risks = Vec::new();

        if let Some(indicator_data) = token_data.get("indicatorData") {
            if let Some(high_risks) = indicator_data.get("high") {
                if let Some(count) = high_risks.get("count").and_then(|v| v.as_u64()) {
                    if count > 0 {
                        risks.push(format!("High risk indicators: {}", count));
                    }
                }
            }

            if let Some(moderate_risks) = indicator_data.get("moderate") {
                if let Some(count) = moderate_risks.get("count").and_then(|v| v.as_u64()) {
                    if count > 0 {
                        risks.push(format!("Moderate risk indicators: {}", count));
                    }
                }
            }
        }

        Ok(risks)
    }

    fn detect_honeypot(&self, token_data: &Value, _audit_risks: &[String]) -> Result<bool> {
        let mut honeypot_indicators = 0;

        // Check mint authority (not disabled = potential honeypot)
        if let Some(audit_risk) = token_data.get("auditRisk") {
            if let Some(mint_disabled) = audit_risk.get("mintDisabled").and_then(|v| v.as_bool()) {
                if !mint_disabled {
                    honeypot_indicators += 1;
                }
            }

            if let Some(freeze_disabled) = audit_risk.get("freezeDisabled").and_then(|v| v.as_bool()) {
                if !freeze_disabled {
                    honeypot_indicators += 1;
                }
            }

            if let Some(lp_burned) = audit_risk.get("lpBurned").and_then(|v| v.as_bool()) {
                if !lp_burned {
                    honeypot_indicators += 1;
                }
            }
        }

        // Check high concentration of top holders
        let (top_10_percentage, _) = self.calculate_holder_metrics(token_data)?;
        if top_10_percentage > 80.0 {
            honeypot_indicators += 1;
        }

        // Check for high risk count
        if let Some(indicator_data) = token_data.get("indicatorData") {
            if let Some(high_risks) = indicator_data.get("high") {
                if let Some(count) = high_risks.get("count").and_then(|v| v.as_u64()) {
                    if count >= 3 {
                        honeypot_indicators += 1;
                    }
                }
            }
        }

        // If 3 or more honeypot indicators, classify as honeypot
        Ok(honeypot_indicators >= 3)
    }

    pub fn print_analysis(&self, token_data: &TokenData) {
        println!("\n{} ===== TOKEN SECURITY ANALYSIS =====", token_data.risk_level.as_emoji());
        println!("📍 Token: {} ({})", token_data.token_name, token_data.token_symbol);
        println!("🔗 Address: {}", token_data.address);
        println!("📊 Risk Score: {}/100 ({})", token_data.score, format!("{:?}", token_data.risk_level));
        println!("🍯 Honeypot Risk: {}", if token_data.is_honeypot { "⚠️  HIGH" } else { "✅ LOW" });
        
        println!("\n💰 FINANCIAL METRICS:");
        println!("  • Market Cap: ${:.2}", token_data.market_cap);
        println!("  • Price: ${:.6}", token_data.price);
        println!("  • Supply: {:.2}", token_data.supply_amount);
        println!("  • Total Liquidity: {:.2} SOL", token_data.liquidity_total);
        
        println!("\n👥 HOLDER ANALYSIS:");
        println!("  • Total Holders: {}", token_data.holder_count);
        println!("  • Top 10 Hold: {:.2}%", token_data.top_10_percentage);
        
        println!("\n🛡️  SECURITY STATUS:");
        println!("  • Mint Authority: {}", if token_data.mint_disabled { "✅ Disabled" } else { "⚠️  Active" });
        println!("  • Freeze Authority: {}", if token_data.freeze_disabled { "✅ Disabled" } else { "⚠️  Active" });
        println!("  • LP Burned: {}", if token_data.lp_burned { "✅ Yes" } else { "⚠️  No" });
        
        if !token_data.audit_risks.is_empty() {
            println!("\n⚠️  AUDIT RISKS:");
            for risk in &token_data.audit_risks {
                println!("  • {}", risk);
            }
        }
        
        println!("\n📅 Deploy Time: {}", token_data.deploy_time);
        println!("=====================================\n");
    }
}

pub async fn sniffer(token_address: &str ) ->Result<TokenData> {
    println!("🚀 SolSniffer Token Analyzer v1.0");
    let client = SolSnifferClient::new()?;    

    let token_data = client.analyze_token(token_address).await?;

    client.print_analysis(&token_data);

    if token_data.is_honeypot {
        println!("🚨 HONEYPOT ALERT: This token shows signs of being a honeypot!");
    }
    if token_data.risk_level == RiskLevel::Critical {
        println!("🚨 CRITICAL RISK: Avoid this token!");
    }

    Ok(token_data)

}