# WorkProof Contract

Soroban escrow contract for WorkProof, an on-chain labour completion flow for informal skilled trades.

This contract holds stablecoin funds while a job is in progress, lets the client release payment when work is complete, and exposes a dispute path for community arbitration.

## Why this repo exists

In informal trade markets, the core trust problem is simple:

- clients do not want to pay before work is done
- artisans do not want to show up without proof that money is available

This contract turns that handshake into an on-chain state machine with low enough fees to work for small real-world jobs.

## Current contract scope

- escrow funding with `create_job`
- client release with `release`
- dispute trigger with `dispute`
- arbitrator resolution with `resolve_dispute`
- arbitrator allowlist management with `set_arbitrator`
- typed contract events for job lifecycle tracking

## Job lifecycle

1. `initialize(admin)` sets the contract admin and bootstraps the instance.
2. `set_arbitrator(arbitrator, enabled)` manages approved dispute resolvers.
3. `create_job(client, artisan, token, amount, metadata)` transfers funds into escrow and creates a `Locked` job.
4. `release(job_id)` pays the artisan and marks the job `Released`.
5. `dispute(job_id, caller)` moves a locked job into `Disputed`.
6. `resolve_dispute(job_id, arbitrator, pay_artisan)` pays the artisan or refunds the client.

## Data model

- `JobStatus`: `Locked`, `Disputed`, `Released`, `Refunded`
- `Job`: stores the client, artisan, token, amount, metadata, and current status
- `DataKey`: stores the admin, next job id, jobs, and arbitrator allowlist

## Local development

Requirements:

- Rust with the `wasm32v1-none` target installed
- a recent toolchain compatible with `soroban-sdk = 26.0.0`

Commands:

```bash
cargo test
cargo build --target wasm32v1-none --release
```

## Test coverage

The current tests cover:

- locking and releasing funds
- disputing a job and refunding through an approved arbitrator

## Known MVP limitations

- arbitrators are allowlisted by admin rather than randomly selected on-chain
- fees for arbitrators are not yet distributed by the contract
- metadata is stored as a simple string and is not yet normalized into structured job fields
- there is no deployment script or network configuration committed yet

## Related repos

- `Stellarworkproof/workproof-web`: mobile-first product interface
- `Stellarworkproof/workproof-docs`: product notes, demo flow, and project context
