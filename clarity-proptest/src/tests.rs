use proptest::prelude::*;
use clarity::vm::database::{ClarityBackingStore, ClarityDatabase, MemoryBackingStore, NULL_HEADER_DB, NULL_BURN_STATE_DB};
use rusqlite::NO_PARAMS;

use crate::contract;

proptest! {
    #[test]
    fn insert_contract(contract in contract()) {
        let mut store = MemoryBackingStore::new();
        let mut db = ClarityDatabase::new(&mut store, &NULL_HEADER_DB, &NULL_BURN_STATE_DB);

        db.begin();

        //let contract: Contract = Faker.fake();
        
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
