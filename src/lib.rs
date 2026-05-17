#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, token, Address, Env, String,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobStatus {
    Locked,
    Disputed,
    Released,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Job {
    pub id: u64,
    pub client: Address,
    pub artisan: Address,
    pub token: Address,
    pub amount: i128,
    pub metadata: String,
    pub status: JobStatus,
}

#[contracttype]
pub enum DataKey {
    Admin,
    NextJobId,
    Job(u64),
    Arbitrator(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    JobNotFound = 4,
    NotParticipant = 5,
    NotClient = 6,
    NotArbitrator = 7,
    InvalidStatus = 8,
}

#[contractevent(topics = ["job", "locked"])]
pub struct JobLocked {
    #[topic]
    pub job_id: u64,
}

#[contractevent(topics = ["job", "released"])]
pub struct JobReleased {
    #[topic]
    pub job_id: u64,
}

#[contractevent(topics = ["job", "disputed"])]
pub struct JobDisputed {
    #[topic]
    pub job_id: u64,
}

#[contractevent(topics = ["job", "resolved"])]
pub struct JobResolved {
    #[topic]
    pub job_id: u64,
    pub paid_artisan: bool,
}

#[contract]
pub struct WorkProofEscrow;

#[contractimpl]
impl WorkProofEscrow {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextJobId, &1u64);
        Ok(())
    }

    pub fn set_arbitrator(env: Env, arbitrator: Address, enabled: bool) -> Result<(), Error> {
        let admin = read_admin(&env)?;
        admin.require_auth();

        env.storage()
            .persistent()
            .set(&DataKey::Arbitrator(arbitrator), &enabled);
        Ok(())
    }

    pub fn create_job(
        env: Env,
        client: Address,
        artisan: Address,
        token: Address,
        amount: i128,
        metadata: String,
    ) -> Result<u64, Error> {
        read_admin(&env)?;
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        client.require_auth();

        let id = next_job_id(&env);
        let escrow_address = env.current_contract_address();
        token::Client::new(&env, &token).transfer(&client, &escrow_address, &amount);

        let job = Job {
            id,
            client,
            artisan,
            token,
            amount,
            metadata,
            status: JobStatus::Locked,
        };

        env.storage().persistent().set(&DataKey::Job(id), &job);
        env.storage().instance().set(&DataKey::NextJobId, &(id + 1));
        JobLocked { job_id: id }.publish(&env);
        Ok(id)
    }

    pub fn release(env: Env, job_id: u64) -> Result<(), Error> {
        let mut job = read_job(&env, job_id)?;
        job.client.require_auth();

        if job.status != JobStatus::Locked && job.status != JobStatus::Disputed {
            return Err(Error::InvalidStatus);
        }

        let escrow_address = env.current_contract_address();
        token::Client::new(&env, &job.token).transfer(&escrow_address, &job.artisan, &job.amount);
        job.status = JobStatus::Released;

        env.storage().persistent().set(&DataKey::Job(job_id), &job);
        JobReleased { job_id }.publish(&env);
        Ok(())
    }

    pub fn dispute(env: Env, job_id: u64, caller: Address) -> Result<(), Error> {
        let mut job = read_job(&env, job_id)?;
        caller.require_auth();

        if caller != job.client && caller != job.artisan {
            return Err(Error::NotParticipant);
        }

        if job.status != JobStatus::Locked {
            return Err(Error::InvalidStatus);
        }

        job.status = JobStatus::Disputed;
        env.storage().persistent().set(&DataKey::Job(job_id), &job);
        JobDisputed { job_id }.publish(&env);
        Ok(())
    }

    pub fn resolve_dispute(env: Env, job_id: u64, arbitrator: Address, pay_artisan: bool) -> Result<(), Error> {
        let mut job = read_job(&env, job_id)?;
        arbitrator.require_auth();

        let allowed = env
            .storage()
            .persistent()
            .get::<DataKey, bool>(&DataKey::Arbitrator(arbitrator.clone()))
            .unwrap_or(false);
        if !allowed {
            return Err(Error::NotArbitrator);
        }

        if job.status != JobStatus::Disputed {
            return Err(Error::InvalidStatus);
        }

        let escrow_address = env.current_contract_address();
        if pay_artisan {
            token::Client::new(&env, &job.token).transfer(&escrow_address, &job.artisan, &job.amount);
            job.status = JobStatus::Released;
        } else {
            token::Client::new(&env, &job.token).transfer(&escrow_address, &job.client, &job.amount);
            job.status = JobStatus::Refunded;
        }

        JobResolved {
            job_id,
            paid_artisan: pay_artisan,
        }
        .publish(&env);
        env.storage().persistent().set(&DataKey::Job(job_id), &job);
        Ok(())
    }

    pub fn get_job(env: Env, job_id: u64) -> Result<Job, Error> {
        read_job(&env, job_id)
    }
}

fn read_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

fn next_job_id(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::NextJobId)
        .unwrap_or(1u64)
}

fn read_job(env: &Env, job_id: u64) -> Result<Job, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Job(job_id))
        .ok_or(Error::JobNotFound)
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use soroban_sdk::{
        testutils::Address as _,
        token::{StellarAssetClient, TokenClient},
        Address,
    };

    fn setup() -> (
        Env,
        WorkProofEscrowClient<'static>,
        Address,
        Address,
        Address,
        Address,
        TokenClient<'static>,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let client = Address::generate(&env);
        let artisan = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token = TokenClient::new(&env, &token_id.address());
        let asset = StellarAssetClient::new(&env, &token_id.address());
        asset.mint(&client, &100_000);

        let contract_id = env.register(WorkProofEscrow, ());
        let contract = WorkProofEscrowClient::new(&env, &contract_id);
        contract.initialize(&admin);

        (env, contract, contract_id, client, artisan, admin, token)
    }

    #[test]
    fn locks_and_releases_payment() {
        let (env, contract, contract_id, client, artisan, _admin, token) = setup();
        let metadata = String::from_str(&env, "Tiling job, Ajah, receipt #014");

        let job_id = contract.create_job(&client, &artisan, &token.address, &40_000, &metadata);
        assert_eq!(job_id, 1);
        assert_eq!(token.balance(&client), 60_000);
        assert_eq!(token.balance(&contract_id), 40_000);

        let job = contract.get_job(&job_id);
        assert_eq!(job.status, JobStatus::Locked);

        contract.release(&job_id);
        assert_eq!(token.balance(&artisan), 40_000);

        let job = contract.get_job(&job_id);
        assert_eq!(job.status, JobStatus::Released);
    }

    #[test]
    fn arbitrator_can_refund_disputed_job() {
        let (env, contract, _contract_id, client, artisan, _admin, token) = setup();
        let arbitrator = Address::generate(&env);
        let metadata = String::from_str(&env, "Electrical repair, Kumasi");

        contract.set_arbitrator(&arbitrator, &true);
        let job_id = contract.create_job(&client, &artisan, &token.address, &25_000, &metadata);
        contract.dispute(&job_id, &artisan);
        contract.resolve_dispute(&job_id, &arbitrator, &false);

        assert_eq!(token.balance(&client), 100_000);
        let job = contract.get_job(&job_id);
        assert_eq!(job.status, JobStatus::Refunded);
    }
}
