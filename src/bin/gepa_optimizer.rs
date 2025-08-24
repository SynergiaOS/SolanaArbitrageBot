use anyhow::{anyhow, Result};
use clap::Parser;
use log::{info, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    input: String, // trades_last7.csv lub .json
    #[arg(long)]
    output: String, // optimized.yaml
    #[arg(long, default_value = "csv")]
    format: String, // csv|json
    #[arg(long)]
    api_url: Option<String>, // http://127.0.0.1:3001
    #[arg(long)]
    auth_token: Option<String>,
    #[arg(long, default_value_t = true)]
    dry_run: bool,
    #[arg(long, default_value_t = 30)]
    generations: usize,
    #[arg(long, default_value_t = 30)]
    population: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
struct RowCsv {
    timestamp: String,
    profit_usd: f64,
    amount_sol: f64,
    spread_percent: f64,
    raydium_price: f64,
    orca_price: f64,
}

#[derive(Clone, Debug)]
struct Gene {
    min_profit_percent: f64,   // 0.05 .. 1.0
    max_slippage_percent: f64, // 0.1 .. 1.5
    max_position_sol: f64,     // 0.01 .. 2.0
    fitness: f64,
}

#[derive(Serialize)]
struct OutputYaml {
    limits: LimitsOut,
    execution: ExecOut,
}
#[derive(Serialize)]
struct LimitsOut {
    min_profit_percent: f64,
    max_slippage_percent: f64,
    max_position_sol: f64,
    max_daily_loss_usd: f64,
}
#[derive(Serialize)]
struct ExecOut {
    priority_fee_lamports: u64,
    max_retries: u32,
}

fn eval_fitness(g: &Gene, rows: &[RowCsv]) -> f64 {
    let mut profit = 0.0;
    for r in rows {
        if r.spread_percent >= g.min_profit_percent {
            // proxy: kara za zbyt duże pozycje i zbyt wysoką slippage
            let penalty = (g.max_position_sol / 2.0).max(0.0) + (g.max_slippage_percent / 2.0);
            profit += r.profit_usd - penalty;
        }
    }
    profit
}

fn evolve(mut pop: Vec<Gene>, rows: &[RowCsv], generations: usize) -> Gene {
    let mut rng = rand::thread_rng();
    for _ in 0..generations {
        for g in pop.iter_mut() {
            g.fitness = eval_fitness(g, rows);
        }
        pop.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        let elite = pop[0].clone();
        // mutacje lekkie
        for g in pop.iter_mut().skip(1).take(5) {
            g.min_profit_percent =
                (g.min_profit_percent + rng.gen_range(-0.05..0.05)).clamp(0.05, 1.0);
            g.max_slippage_percent =
                (g.max_slippage_percent + rng.gen_range(-0.1..0.1)).clamp(0.1, 1.5);
            g.max_position_sol = (g.max_position_sol + rng.gen_range(-0.05..0.05)).clamp(0.01, 2.0);
        }
        // reszta – krzyżowanie z elitą
        for g in pop.iter_mut().skip(6) {
            g.min_profit_percent = (elite.min_profit_percent + g.min_profit_percent) / 2.0;
        }
    }
    pop.into_iter()
        .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
        .unwrap()
}

fn parse_csv(path: &str) -> Result<Vec<RowCsv>> {
    let mut rdr = csv::Reader::from_path(path)?;
    let mut out = Vec::new();
    for r in rdr.records() {
        let rec = r?;
        out.push(RowCsv {
            timestamp: rec.get(0).unwrap_or_default().to_string(),
            profit_usd: rec.get(1).unwrap_or("0").parse().unwrap_or(0.0),
            amount_sol: rec.get(2).unwrap_or("0").parse().unwrap_or(0.0),
            spread_percent: rec.get(3).unwrap_or("0").parse().unwrap_or(0.0),
            raydium_price: rec.get(4).unwrap_or("0").parse().unwrap_or(0.0),
            orca_price: rec.get(5).unwrap_or("0").parse().unwrap_or(0.0),
        });
    }
    Ok(out)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let rows = match args.format.as_str() {
        "csv" => parse_csv(&args.input)?,
        "json" => serde_json::from_str::<Vec<RowCsv>>(&fs::read_to_string(&args.input)?)?,
        _ => return Err(anyhow!("Unsupported format")),
    };
    if rows.is_empty() {
        return Err(anyhow!("No data rows"));
    }

    // inicjalna populacja
    let mut rng = rand::thread_rng();
    let pop = (0..args.population)
        .map(|_| Gene {
            min_profit_percent: rng.gen_range(0.05..0.80),
            max_slippage_percent: rng.gen_range(0.10..1.00),
            max_position_sol: rng.gen_range(0.02..0.50),
            fitness: 0.0,
        })
        .collect::<Vec<_>>();

    let best = evolve(pop, &rows, args.generations);
    let out = OutputYaml {
        limits: LimitsOut {
            min_profit_percent: (best.min_profit_percent * 100.0).round() / 100.0,
            max_slippage_percent: (best.max_slippage_percent * 100.0).round() / 100.0,
            max_position_sol: (best.max_position_sol * 1000.0).round() / 1000.0,
            max_daily_loss_usd: f64::max(5.0, rows.len() as f64 * 0.05),
        },
        execution: ExecOut {
            priority_fee_lamports: 5000,
            max_retries: 3,
        },
    };
    let yaml = serde_yaml::to_string(&out)?;
    fs::write(&args.output, &yaml)?;
    info!("Written {}", &args.output);

    // opcjonalny live update obsługiwanych pól przez /api/config
    if !args.dry_run {
        if let Some(api) = args.api_url {
            let payload = serde_json::json!({
              "min_profit_usd": 0.02, // proxy; można policzyć z aktualnej ceny
              "max_position_sol": out.limits.max_position_sol,
              "max_daily_trades": 50,
              "max_daily_loss_usd": out.limits.max_daily_loss_usd,
              "enabled": true
            });
            let client = reqwest::Client::new();
            let mut req = client.post(format!("{}/api/config", api)).json(&payload);
            if let Some(tok) = args.auth_token {
                req = req.header("Authorization", format!("Bearer {}", tok));
            }
            let resp = req.send().await?;
            if !resp.status().is_success() {
                warn!("POST /api/config failed: {}", resp.status());
            }
        }
    }
    Ok(())
}
