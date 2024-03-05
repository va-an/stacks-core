use fake::{Fake, Faker};
use rusqlite::NO_PARAMS;
use stacks_common::util::hash::Sha512Trunc256Sum;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use clarity_proptest::*;

use crate::vm::contracts::Contract;
use crate::vm::database::clarity_store::ContractCommitment;
use crate::vm::database::{
    ClarityBackingStore, ClarityDatabase, ClaritySerializable, MemoryBackingStore,
    NULL_BURN_STATE_DB, NULL_HEADER_DB,
};
use crate::vm::fakes::raw::EnglishWord;
use crate::vm::Value;

proptest! {
    #[test]
    fn insert_contract(contract in contract()) {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        db.begin();

        let contract: Contract = Faker.fake();
        
        let contract_id = contract.contract_context.contract_identifier.clone();

        db.insert_contract(&contract_id, contract)
            .expect("failed to insert contract into backing store");

        let exists = sql_exists(
            &mut store,
            &format!(
                "SELECT * FROM metadata_table WHERE key LIKE '%{}%'",
                contract_id.to_string()
            ),
        );
        assert!(!exists);
    }
}

#[test]
fn get_contract() {
    for _ in 0..1000 {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        db.begin();

        let contract: Contract = Faker.fake();
        let contract_id = contract.contract_context.contract_identifier.clone();

        db.insert_contract(&contract_id, contract.clone())
            .expect("failed to insert contract into backing store");

        let retrieved_contract = db
            .get_contract(&contract_id)
            .expect("failed to retrieve contract from backing store");

        assert_eq!(contract, retrieved_contract);
    }
}

#[test]
fn insert_contract_without_begin_should_fail() {
    for _ in 0..1000 {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        let contract: Contract = Faker.fake();
        let contract_id = contract.contract_context.contract_identifier.clone();

        db.insert_contract(&contract_id, contract)
            .expect_err("inserting contract without a begin should fail");
    }
}

#[test]
fn insert_contract_with_commit_should_exist_in_backing_store() {
    for _ in 0..1000 {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        db.begin();

        let contract: Contract = Faker.fake();
        let contract_id = contract.contract_context.contract_identifier.clone();

        db.insert_contract(&contract_id, contract.clone())
            .expect("failed to insert contract into backing store");

        db.commit().expect("failed to commit to backing store");

        let count = sql_query_u32(
            &mut store,
            &format!(
                "SELECT COUNT(*) FROM metadata_table WHERE key LIKE '{}'",
                format!(
                    "clr-meta::{}::vm-metadata::9::contract",
                    contract_id.to_string()
                )
            ),
        );

        assert_eq!(1, count);
    }
}

#[test]
fn put_data_no_commit() {
    for _ in 0..1000 {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        db.begin();

        db.put(
            "hello",
            &ContractCommitment {
                block_height: 1,
                hash: Sha512Trunc256Sum::from_data(&[1, 2, 3, 4]),
            },
        )
        .expect("failed to put data");

        let count = sql_query_u32(&mut store, "SELECT COUNT(*) FROM data_table");
        assert_eq!(0, count);
    }
}

#[test]
fn put_data_with_commit_should_exist_in_backing_store() {
    for _ in 0..1000 {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        db.begin();

        let key = Faker.fake::<String>();
        db.put(
            &key,
            &ContractCommitment {
                block_height: Faker.fake(),
                hash: Sha512Trunc256Sum::from_data(&Faker.fake::<Vec<u8>>()),
            },
        )
        .expect("failed to put data");

        db.commit().expect("failed to commit to backing store");

        let count = sql_query_u32(
            &mut store,
            &format!("SELECT COUNT(*) FROM data_table where key = '{}'", key),
        );
        assert_eq!(1, count);
    }
}

#[test]
fn put_data_without_begin_fails() {
    for _ in 0..1000 {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        let key = Faker.fake::<String>();
        db.put(
            &key,
            &ContractCommitment {
                block_height: Faker.fake(),
                hash: Sha512Trunc256Sum::from_data(&Faker.fake::<Vec<u8>>()),
            },
        )
        .expect_err("expected not-nested error");
    }
}

/// Executes the provided SQL query, which is expected to return a positive
/// integer, and returns it as a u32. Panics upon SQL failure.
fn sql_query_u32<S: ClarityBackingStore>(store: &mut S, sql: &str) -> u32 {
    let sqlite = store.get_side_store();
    sqlite
        .query_row(sql, NO_PARAMS, |row| {
            let i: u32 = row.get(0)?;
            Ok(i)
        })
        .expect("failed to verify results in sqlite")
}

/// Executes the provided SQL query as a subquery within a `SELECT EXISTS(...)`
/// statement. Returns true if the statement returns any rows, false otherwise.
/// Panics upon SQL failure.
fn sql_exists<S: ClarityBackingStore>(store: &mut S, sql: &str) -> bool {
    let sqlite = store.get_side_store();
    sqlite
        .query_row(&format!("SELECT EXISTS({});", sql), NO_PARAMS, |row| {
            let exists: bool = row.get(0)?;
            Ok(exists)
        })
        .expect("failed to verify results in sqlite")
}
