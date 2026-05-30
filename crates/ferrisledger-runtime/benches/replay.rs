//! Criterion benchmark for deterministic account replay.
#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use ferrisledger_domain::{AccountId, CorrelationId, IdempotencyKey, Money, TenantId};
use ferrisledger_rules::RuntimeCommand;
use ferrisledger_runtime::RuntimeService;

fn replay_snapshot(c: &mut Criterion) {
    c.bench_function("replay_100_deposits", |b| {
        b.iter_batched(
            seed_runtime,
            |(runtime, tenant_id, account_id)| {
                let snapshot = runtime
                    .account_snapshot(&tenant_id, &account_id)
                    .expect("snapshot")
                    .expect("account");
                assert_eq!(snapshot.balance.cents(), 100_000);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn seed_runtime() -> (
    RuntimeService<ferrisledger_store::FileEventStore>,
    TenantId,
    AccountId,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.keep().join("events.jsonl");
    let runtime = RuntimeService::file(path);
    let tenant_id = TenantId::new("tenant_001").expect("tenant");
    let account_id = AccountId::new("account_001").expect("account");
    runtime
        .execute(
            RuntimeCommand::OpenAccount {
                tenant_id: tenant_id.clone(),
                account_id: account_id.clone(),
                currency: "BRL".to_string(),
                account_holder_name: "Ada Lovelace".to_string(),
            },
            CorrelationId::new("corr_open").expect("correlation"),
        )
        .expect("open");

    for index in 0..100 {
        runtime
            .execute(
                RuntimeCommand::DepositMoney {
                    tenant_id: tenant_id.clone(),
                    account_id: account_id.clone(),
                    amount: Money::new(1_000, "BRL").expect("money"),
                    idempotency_key: IdempotencyKey::new(format!("deposit_{index}"))
                        .expect("idempotency"),
                },
                CorrelationId::new(format!("corr_{index}")).expect("correlation"),
            )
            .expect("deposit");
    }

    (runtime, tenant_id, account_id)
}

criterion_group!(benches, replay_snapshot);
criterion_main!(benches);
