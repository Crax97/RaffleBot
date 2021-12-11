use std::collections::HashSet;
use rusqlite::{Connection, Result};
use crate::db::Result as RaffleResult;
use crate::db::*;

pub struct SQLiteInstance {    
    connection: rusqlite::Connection,
}

impl SQLiteInstance {
    pub fn create(file: &str) -> Result<SQLiteInstance, ()> {
        let conn = Connection::open(file);
        if let Ok(connection) = conn {
            Ok(SQLiteInstance {
                connection : connection
            })
        } else {
            Err(())
        }
    }
}

impl RaffleDB for SQLiteInstance {
    fn close(self) -> () {}
    
    // raffle functions
    fn create_raffle(&mut self, raffle: Raffle) -> RaffleCreationResult {
        todo!();
    }
    fn get_ongoing_raffle(&self) -> Option<Raffle> {
        todo!();
    }
    fn stop_raffle(&mut self){
        todo!();
    }
    fn pick_winners(&mut self, num_winners: u8) -> Vec<Partecipant> {
        todo!();
    }

    // user functions
    fn get_partecipants(&self) -> HashSet<Partecipant> {
        todo!();
    }
    fn register_partecipant(&mut self, user_id: UserID) -> RegistrationStatus{
        todo!();
    }
    fn get_registration_status(&self, user_id: UserID) -> RegistrationStatus{
        todo!();
    }
    
    // raffle codes functions
    fn generate_raffle_code(&mut self, use_count: CodeUseCount) -> RaffleResult<RedeemableCode>{
        todo!();
    }
    fn delete_raffle_code(&mut self, code: RedeemableCodeId) {
        todo!();
    }

    fn validate_code(&self, code: String) -> CodeValidation{
        todo!();
    }
    fn redeem_code(&mut self, user_id: UserID, code_id: RedeemableCodeId) {
        todo!();
    }
}