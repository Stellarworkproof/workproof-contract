# WorkProof Contract

Soroban escrow contract for informal skilled-trade jobs.

## Flow

1. `initialize(admin)` sets the contract admin.
2. `create_job(client, artisan, token, amount, metadata)` transfers stablecoin from the client into the contract and marks the job `Locked`.
3. `release(job_id)` lets the client release locked funds to the artisan.
4. `dispute(job_id, caller)` lets either participant mark a locked job as disputed.
5. `resolve_dispute(job_id, arbitrator, pay_artisan)` lets an approved arbitrator pay the artisan or refund the client.

## Commands

```bash
cargo test
cargo build --target wasm32v1-none --release
```
