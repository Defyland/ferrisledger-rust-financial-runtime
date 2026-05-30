//! FerrisLedger command-line adapter.

use std::{net::SocketAddr, path::PathBuf};

use anyhow::Context as _;
use clap::{Parser, Subcommand, ValueEnum};
use ferrisledger_api::{ApiConfig, router};
use ferrisledger_domain::{
    AccountId, CorrelationId, EventId, IdempotencyKey, LedgerEntryId, Money, SettlementId, TenantId,
};
use ferrisledger_events::LedgerDirection;
use ferrisledger_rules::RuntimeCommand;
use ferrisledger_runtime::RuntimeService;

#[derive(Debug, Parser)]
#[command(name = "ferrisledger")]
#[command(about = "Append-only Rust financial event runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start the HTTP API.
    Serve {
        /// Address to bind.
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: SocketAddr,
        /// JSONL event store path.
        #[arg(
            long,
            env = "FERRISLEDGER_STORE_PATH",
            default_value = "data/events.jsonl"
        )]
        store_path: PathBuf,
        /// API key expected in x-api-key.
        #[arg(long, env = "FERRISLEDGER_API_KEY", default_value = "dev-secret")]
        api_key: String,
        /// Authenticated requests allowed per API key per rolling minute.
        #[arg(
            long,
            env = "FERRISLEDGER_RATE_LIMIT_PER_MINUTE",
            default_value_t = 120
        )]
        rate_limit_per_minute: u32,
    },
    /// Open an account.
    OpenAccount(AccountArgs),
    /// Deposit money.
    Deposit(MovementArgs),
    /// Request Pix transfer.
    PixTransfer(PixArgs),
    /// Execute settlement for a reserved transfer.
    Settle(SettlementArgs),
    /// Create an accounting ledger entry.
    LedgerEntry(LedgerArgs),
    /// Replay one account stream and print the current snapshot.
    Replay(StreamArgs),
    /// Verify append-only store checksums.
    Verify {
        /// JSONL event store path.
        #[arg(long, default_value = "data/events.jsonl")]
        store_path: PathBuf,
    },
}

#[derive(Debug, Parser)]
struct StoreArg {
    /// JSONL event store path.
    #[arg(long, default_value = "data/events.jsonl")]
    store_path: PathBuf,
}

#[derive(Debug, Parser)]
struct AccountArgs {
    #[command(flatten)]
    store: StoreArg,
    /// Tenant ID.
    #[arg(long)]
    tenant_id: String,
    /// Account ID.
    #[arg(long)]
    account_id: String,
    /// Account currency.
    #[arg(long, default_value = "BRL")]
    currency: String,
    /// Account holder name.
    #[arg(long)]
    account_holder_name: String,
    /// Correlation ID.
    #[arg(long, default_value = "cli")]
    correlation_id: String,
}

#[derive(Debug, Parser)]
struct MovementArgs {
    #[command(flatten)]
    stream: StreamArgs,
    /// Amount in minor units.
    #[arg(long)]
    amount_cents: i64,
    /// ISO-4217 currency.
    #[arg(long, default_value = "BRL")]
    currency: String,
    /// Idempotency key.
    #[arg(long)]
    idempotency_key: String,
}

#[derive(Debug, Parser)]
struct PixArgs {
    #[command(flatten)]
    movement: MovementArgs,
    /// Beneficiary Pix key.
    #[arg(long)]
    beneficiary_pix_key: String,
}

#[derive(Debug, Parser)]
struct SettlementArgs {
    #[command(flatten)]
    movement: MovementArgs,
    /// Settlement ID.
    #[arg(long)]
    settlement_id: String,
}

#[derive(Debug, Parser)]
struct LedgerArgs {
    #[command(flatten)]
    movement: MovementArgs,
    /// Ledger entry ID.
    #[arg(long)]
    ledger_entry_id: String,
    /// Ledger direction.
    #[arg(long)]
    direction: DirectionArg,
    /// Business reason.
    #[arg(long)]
    reason: String,
    /// Related event ID.
    #[arg(long)]
    related_event_id: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DirectionArg {
    /// Credit entry.
    Credit,
    /// Debit entry.
    Debit,
}

#[derive(Debug, Parser)]
struct StreamArgs {
    #[command(flatten)]
    store: StoreArg,
    /// Tenant ID.
    #[arg(long)]
    tenant_id: String,
    /// Account ID.
    #[arg(long)]
    account_id: String,
    /// Correlation ID.
    #[arg(long, default_value = "cli")]
    correlation_id: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ferrisledger_telemetry::init_tracing();
    match Cli::parse().command {
        Commands::Serve {
            bind,
            store_path,
            api_key,
            rate_limit_per_minute,
        } => {
            let app = router(
                ApiConfig::new(store_path, api_key)
                    .with_rate_limit_per_minute(rate_limit_per_minute),
            )?;
            tracing::info!(%bind, "starting ferrisledger api");
            let listener = tokio::net::TcpListener::bind(bind).await?;
            axum::serve(listener, app).await?;
        }
        Commands::OpenAccount(args) => {
            let runtime = RuntimeService::file(args.store.store_path);
            let outcome = runtime.execute(
                RuntimeCommand::OpenAccount {
                    tenant_id: tenant(args.tenant_id)?,
                    account_id: account(args.account_id)?,
                    currency: args.currency,
                    account_holder_name: args.account_holder_name,
                },
                correlation(args.correlation_id)?,
            )?;
            print_json(&outcome)?;
        }
        Commands::Deposit(args) => {
            let runtime = RuntimeService::file(args.stream.store.store_path);
            let outcome = runtime.execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant(args.stream.tenant_id)?,
                    account_id: account(args.stream.account_id)?,
                    amount: Money::new(args.amount_cents, args.currency)?,
                    idempotency_key: idempotency(args.idempotency_key)?,
                },
                correlation(args.stream.correlation_id)?,
            )?;
            print_json(&outcome)?;
        }
        Commands::PixTransfer(args) => {
            let runtime = RuntimeService::file(args.movement.stream.store.store_path);
            let outcome = runtime.execute(
                RuntimeCommand::RequestPixTransfer {
                    tenant_id: tenant(args.movement.stream.tenant_id)?,
                    account_id: account(args.movement.stream.account_id)?,
                    amount: Money::new(args.movement.amount_cents, args.movement.currency)?,
                    beneficiary_pix_key: args.beneficiary_pix_key,
                    idempotency_key: idempotency(args.movement.idempotency_key)?,
                },
                correlation(args.movement.stream.correlation_id)?,
            )?;
            print_json(&outcome)?;
        }
        Commands::Settle(args) => {
            let runtime = RuntimeService::file(args.movement.stream.store.store_path);
            let outcome = runtime.execute(
                RuntimeCommand::ExecuteSettlement {
                    tenant_id: tenant(args.movement.stream.tenant_id)?,
                    account_id: account(args.movement.stream.account_id)?,
                    amount: Money::new(args.movement.amount_cents, args.movement.currency)?,
                    settlement_id: SettlementId::new(args.settlement_id)
                        .map_err(anyhow::Error::msg)?,
                    idempotency_key: idempotency(args.movement.idempotency_key)?,
                },
                correlation(args.movement.stream.correlation_id)?,
            )?;
            print_json(&outcome)?;
        }
        Commands::LedgerEntry(args) => {
            let runtime = RuntimeService::file(args.movement.stream.store.store_path);
            let outcome = runtime.execute(
                RuntimeCommand::CreateLedgerEntry {
                    tenant_id: tenant(args.movement.stream.tenant_id)?,
                    account_id: account(args.movement.stream.account_id)?,
                    ledger_entry_id: ledger_entry(args.ledger_entry_id)?,
                    direction: direction(args.direction),
                    amount: Money::new(args.movement.amount_cents, args.movement.currency)?,
                    reason: args.reason,
                    idempotency_key: idempotency(args.movement.idempotency_key)?,
                    related_event_id: event_id(args.related_event_id)?,
                },
                correlation(args.movement.stream.correlation_id)?,
            )?;
            print_json(&outcome)?;
        }
        Commands::Replay(args) => {
            let runtime = RuntimeService::file(args.store.store_path);
            let snapshot = runtime
                .account_snapshot(&tenant(args.tenant_id)?, &account(args.account_id)?)
                .context("failed to replay account stream")?;
            print_json(&snapshot)?;
        }
        Commands::Verify { store_path } => {
            let runtime = RuntimeService::file(store_path);
            print_json(&runtime.verify_store()?)?;
        }
    }
    Ok(())
}

fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn tenant(value: String) -> anyhow::Result<TenantId> {
    TenantId::new(value).map_err(anyhow::Error::msg)
}

fn account(value: String) -> anyhow::Result<AccountId> {
    AccountId::new(value).map_err(anyhow::Error::msg)
}

fn correlation(value: String) -> anyhow::Result<CorrelationId> {
    CorrelationId::new(value).map_err(anyhow::Error::msg)
}

fn idempotency(value: String) -> anyhow::Result<IdempotencyKey> {
    IdempotencyKey::new(value).map_err(anyhow::Error::msg)
}

fn ledger_entry(value: String) -> anyhow::Result<LedgerEntryId> {
    LedgerEntryId::new(value).map_err(anyhow::Error::msg)
}

fn direction(value: DirectionArg) -> LedgerDirection {
    match value {
        DirectionArg::Credit => LedgerDirection::Credit,
        DirectionArg::Debit => LedgerDirection::Debit,
    }
}

fn event_id(value: Option<String>) -> anyhow::Result<Option<EventId>> {
    value
        .map(EventId::new)
        .transpose()
        .map_err(anyhow::Error::msg)
}
