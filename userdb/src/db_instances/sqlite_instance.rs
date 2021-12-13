use std::collections::HashSet;
use std::ops::Add;
use rand::Rng;
use rusqlite::{Connection, CachedStatement, Result, params};
use crate::db::RaffleResult;
use crate::db::*;

pub struct SQLiteInstance {    
    connection: rusqlite::Connection,
}

fn timestamp_now() -> Timestamp {
    std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs()
}

fn setup_connection(connection: &mut Connection) {
    connection.execute_batch("
    CREATE TABLE IF NOT EXISTS PARTECIPANTS (
        user_id INTEGER NOT NULL PRIMARY KEY,
        joined_when INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS REDEEMABLE_CODES (
        code_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE,
        remaining_uses INTEGER NOT NULL,
        generated_when INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS USED_CODES (
        user_id INTEGER NOT NULL,
        code_id INTEGER NOT NULL,
        used_when INTEGER NOT NULL,
        FOREIGN KEY (user_id) REFERENCES PARTECIPANTS(user_id),
        FOREIGN KEY (code_id) REFERENCES REDEEMABLE_CODES(code_id)
    );
    --The referrer is the user that invited the referee in the raffle
    CREATE TABLE IF NOT EXISTS REFERRALS (
        referrer_id INTEGER NOT NULL,
        referee_id INTEGER NOT NULL,
        FOREIGN KEY(referrer_id) REFERENCES PARTECIPANTS(user_id),
        FOREIGN KEY(referee_id) REFERENCES PARTECIPANTS(user_id)
    );
    CREATE TABLE IF NOT EXISTS RAFFLE (
        raffle_id INTEGER PRIMARY KEY AUTOINCREMENT,
        raffle_name TEXT NOT NULL,
        raffle_message BLOB NOT NULL,
        started_when INTEGER NOT NULL,
        ended_when INTEGER
    );
    CREATE TABLE IF NOT EXISTS RAFFLE_WINNERS (
        raffle_id INTEGER,
        winner_id INTEGER,
        position INTEGER,
        FOREIGN KEY (raffle_id) REFERENCES RAFFLE(raffle_id)
    );
    ").expect("Failed to create or intialize the database")
}
fn raffle_from_row(row: &rusqlite::Row) -> Raffle {
    Raffle {
        raffle_id: row.get_unwrap(0),
        raffle_name : row.get_unwrap(1),
        raffle_description : row.get_unwrap(2),
        started_when : row.get_unwrap(3)
    }
}
fn raffle_code_from_row(row: &rusqlite::Row) -> RedeemableCode {
    RedeemableCode {
        unique_id: row.get_unwrap(0),
        code : row.get_unwrap(1),
        remaining_uses : row.get_unwrap(2),
        generated_when: row.get_unwrap(3),
    }
}

fn partecipant_from_row(row: &rusqlite::Row, db: &SQLiteInstance) -> Partecipant {
    let user_id = row.get_unwrap(0);
    let referees = db.get_referees_of_user(user_id).unwrap().len();
    let codes_used = db.get_raffle_codes_used_by_user(user_id).unwrap().len();
    Partecipant {
        user_id,
        joined_when: row.get_unwrap(1),
        priority: 1 + referees + codes_used
    }
}

impl SQLiteInstance {
    
    pub fn create(file: &str) -> Result<SQLiteInstance, ()> {
        let  conn = Connection::open(file);
        if let Ok(mut connection) = conn {
            setup_connection(&mut connection);
            Ok(SQLiteInstance {
                connection
            })
        } else {
            Err(())
        }
    }
}

impl RaffleDB for SQLiteInstance {
    fn close(self) -> RaffleResult<()> {
        match self.connection.close() {
            Ok(_) => Ok(()),
            Err((_c, e)) => Err(Box::new(e))
        }
    }
    
    // raffle functions
    fn create_raffle(&mut self, name: &str, description: &str) -> RaffleResult<RaffleCreationResult> {
        let ongoing_raffle = self.get_ongoing_raffle()?;
        if let Some(existing_raffle) = ongoing_raffle {
            Ok(RaffleCreationResult::OngoingRaffleExists(existing_raffle))
        } else {
            let time_since_epoch = timestamp_now();
            let insertion = self.connection.execute("
            INSERT INTO RAFFLE (raffle_name, raffle_message, started_when)
            VALUES (?1, ?2, ?3)
            ", params!(name, description, time_since_epoch));
            if let Err(e) = insertion {
                Err(Box::new(e))
            } else {
                Ok(RaffleCreationResult::Success(self.get_ongoing_raffle()?
                                            .expect("Raffle was created but was not correctly inserted in db")))
            }
        }
    }
    fn get_ongoing_raffle(&self) -> RaffleResult<Option<Raffle>> {
        let running_raffle= self.connection.query_row("
        SELECT * FROM RAFFLE WHERE RAFFLE.ended_when IS NULL
        ", params!(), |row| Ok(raffle_from_row(&row)));
        
        match running_raffle {
            Ok(raffle) => Ok(Some(raffle)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e))
        }
    }
    fn stop_raffle(&mut self, num_winners: usize) -> RaffleResult<Vec<Partecipant>> {
        let ongoing_raffle = self.get_ongoing_raffle()?;
        if let Some(raffle) = ongoing_raffle {
            let partecipants_set = self.get_partecipants()?;
            let transaction = self.connection.transaction()
                .expect("stop_raffle: failed to begin SQL transaction");
            let _ = transaction.execute_batch("
                DELETE FROM REFERRALS;
                DELETE FROM USED_CODES;
                DELETE FROM REDEEMABLE_CODES;
                DELETE FROM PARTECIPANTS;
            ")
            .unwrap();

            let closed_raffles = {
                let mut statement = transaction.prepare_cached("
                UPDATE RAFFLE
                SET ended_when = ?1
                WHERE
                    raffle_id == ?2
                ").unwrap();
                statement.execute(params!(timestamp_now(), raffle.raffle_id)).unwrap()
            };
            if closed_raffles > 0 {
                let mut partecipants = Vec::from_iter(partecipants_set.into_iter());
                partecipants
                    .sort_by(|a, b|  b.priority.cmp(&a.priority));
                let winners = Vec::from_iter(partecipants
                    .iter()
                    .take(num_winners)
                    .map(|p| p.clone())
                );
                for (pos, winner) in winners.iter().enumerate() {
                    let mut winner_statement = transaction.
                        prepare_cached("
                            INSERT INTO RAFFLE_WINNERS(raffle_id, winner_id, position)
                            VALUES (?1, ?2, ?3)")
                            .unwrap();
                    winner_statement
                    .execute(params!(raffle.raffle_id, winner.user_id, pos))?;
                }
                transaction.commit().unwrap();
                Ok(winners)
            } else {
                transaction.rollback().unwrap();
                todo!()
            }
        } else {
            todo!()
        }
    }

    // user functions
    fn get_partecipants(&self) -> RaffleResult<HashSet<Partecipant>> {
        let mut partecipants_statement = self.connection.prepare_cached(
            "SELECT * FROM PARTECIPANTS"
        ).unwrap();
        let partecipants_from_db = partecipants_statement.
            query_map([], 
                |row| Ok(partecipant_from_row(&row, &self))
            )?;
        Ok(HashSet::from_iter(partecipants_from_db.map(|row| row.unwrap())))
    }
    fn is_partecipant(&self, user_id: UserID) -> RaffleResult<bool> {
        let mut partecipant_query = self.connection.prepare_cached(
            "SELECT COUNT(*) FROM PARTECIPANTS
            WHERE
                user_id == ?1").unwrap();
        let partecipant_count = partecipant_query
            .query_row(
                params!(user_id), 
                |row| Ok(row.get_unwrap::<usize, u64>(0))
            )?;
        Ok(partecipant_count > 0)
    }
    fn get_partecipant(&self, user_id: UserID) -> RaffleResult<Option<Partecipant>> {
        let mut partecipant_query = self.connection.prepare_cached(
            "SELECT * FROM PARTECIPANTS
            WHERE
                user_id == ?1").unwrap();
        let partecipant_maybe = partecipant_query.query_row(params!(user_id), |row| Ok(partecipant_from_row(&row, &self)));
        Ok(if let Ok(partecipant) = partecipant_maybe {
            Some(partecipant)
        } else {
            None
        })
    }
    fn register_partecipant(&mut self, user_id: UserID, referrer: Option<UserID>) -> RaffleResult<RegistrationStatus>{
        let mut register_query = self.connection.prepare_cached(
            "INSERT INTO PARTECIPANTS (user_id, joined_when) 
            VALUES (?1, ?2)").unwrap();
        let now = timestamp_now();
        let inserted_rows = register_query.execute(params!(user_id, now))?;
        if inserted_rows == 0 {
            Ok(RegistrationStatus::NotRegistered)
        } else {
            // We did insert the partecipant in the raffle, now let's check if it has a referrer
            if let Some(referrer_id) = referrer {
                if self.is_partecipant(referrer_id)? {
                    let mut referral_query = self.connection.prepare_cached(
                        "INSERT INTO REFERRALS (referrer_id, referee_id)
                        VALUES (?1, ?2)"
                    ).unwrap();
                    referral_query.execute(params!(referrer_id, user_id))?;
                }
            }
            Ok(RegistrationStatus::Registered(self.get_partecipant(user_id)?.unwrap()))
        }
        
    }
    fn remove_partecipant(&mut self, user_id: UserID) -> RaffleResult<bool> {
        let mut remove_query = self.connection.prepare_cached(
            "DELETE FROM PARTECIPANTS
            WHERE user_id == ?1").unwrap();
        let result = remove_query.execute(params!(user_id))?;
        Ok(result > 0)
    }
    fn get_registration_status(&self, user_id: UserID) -> RaffleResult<RegistrationStatus> {
        Ok(if let Some(partecipant) = self.get_partecipant(user_id)? {
            RegistrationStatus::Registered(partecipant)
        } else {
            RegistrationStatus::NotRegistered
        })
    }
    fn get_referees_of_user(&self, user_id: UserID) -> RaffleResult<Vec<UserID>> {
        let mut referees_query = self.connection.prepare_cached(
            "SELECT referee_id FROM REFERRALS
            WHERE referrer_id == ?1").unwrap();
        let resulting_rows = referees_query.query_map(params!(user_id), 
        |row| row.get(0))?;
        Ok(Vec::from_iter(resulting_rows.into_iter().map(|row| row.unwrap())))
    }
    fn get_referrer_of_user(&self, user_id: UserID) -> RaffleResult<Option<UserID>> {
        let mut referees_query = self.connection.prepare_cached(
            "SELECT referrer_id FROM REFERRALS
            WHERE referee_id == ?1").unwrap();
        let resulting_rows = referees_query.query_row(params!(user_id), 
        |row| Ok(row.get_unwrap(0)));
        match resulting_rows {
            Ok(referrer_id) => Ok(Some(referrer_id)),
            Err(_) => Ok(None),
        }
    }
    // raffle codes functions
    fn generate_raffle_code(&mut self, use_count: CodeUseCount) -> RaffleResult<RedeemableCode>{
        let numeric_usages = match use_count {
            CodeUseCount::Counted(n) => n,
            CodeUseCount::Once => 1,
            CodeUseCount::Illimited => -1,
            _ => panic!("Passed an invalid use count")
        };
        let mut generator = rand::thread_rng();
        const CODE_LENGTH : usize = 8;
        let new_code = std::iter::repeat(0)
            .take(CODE_LENGTH)
            .fold(String::from(""), 
                |s, _| 
                    s.add(generator.gen_range('0'..'Z')
                    .to_string()
                    .as_str()
            )
        );
        let mut query = self.connection
            .prepare_cached(
                "INSERT INTO REDEEMABLE_CODES (code, remaining_uses, generated_when)
                VALUES (?1, ?2, ?3)").unwrap();
        query.execute(params!(new_code, numeric_usages, timestamp_now()))?;
        Ok(self.get_raffle_code_by_name(new_code.as_str())?.unwrap())
    }
    fn delete_raffle_code(&mut self, code: RedeemableCodeId) -> RaffleResult<bool> {
        let mut query = self.connection
        .prepare_cached(
            "DELETE FROM REDEEMABLE_CODES
            WHERE code_id == ?1").unwrap();
        let result = query.execute(params!(code))?;
        Ok(result > 0)
    }

    fn validate_code(&self, code: &str) -> RaffleResult<CodeValidation> {
        Ok(if let Some(redeemable_code) = self.get_raffle_code_by_name(code)? {
            CodeValidation::Valid(redeemable_code.unique_id)
        } else {
            CodeValidation::NotValid("Code not found".to_owned())
        })
    }
    fn redeem_code(&mut self, user_id: UserID, code_id: RedeemableCodeId) -> RaffleResult<CodeRedeemalResult> {
        let user = self.get_partecipant(user_id)?;
        if user == None {
            return Ok(CodeRedeemalResult::NonExistingUser);
        }

        let code = self.get_raffle_code_by_id(code_id)?;
        if let Some(_existing_code) = code {
            if self.partecipant_has_redeemed_code(user_id, code_id)? {
                Ok(CodeRedeemalResult::AlreadyRedeemed)
            } else {
                let redeem_transaction = self.connection
                    .transaction()?;
                {
                    let mut insert_query =
                    redeem_transaction.prepare_cached("INSERT INTO USED_CODES (user_id, code_id, used_when)
                        VALUES (?1, ?2, ?3)").unwrap();
                    insert_query
                    .execute(
                        params!(user_id, code_id, timestamp_now())
                    ).unwrap();
                    
                }
                {
                    let mut update_codes_query = redeem_transaction.prepare_cached("
                        UPDATE REDEEMABLE_CODES
                        SET remaining_uses = remaining_uses - 1
                        WHERE
                            code_id == ?1
                            AND
                            remaining_uses > 0")
                        .unwrap();
                    update_codes_query.execute(params!(code_id)).unwrap();
                }
                {
                    let mut delete_expired_codes_query = redeem_transaction.prepare_cached("
                    DELETE FROM REDEEMABLE_CODES
                    WHERE remaining_uses = 0
                    ").unwrap();
                    delete_expired_codes_query.execute(params!()).unwrap();
                }
                redeem_transaction.commit().unwrap();
                Ok(CodeRedeemalResult::Redeemed)
            }
        } else {
            Ok(CodeRedeemalResult::NonExistingCode)
        }
    }

    fn get_raffle_codes(&self) -> RaffleResult<HashSet<RedeemableCode>> {
        let mut raffle_code_query = self.connection.prepare_cached(
        "SELECT * FROM REDEEMABLE_CODES").unwrap();
        let found_codes = raffle_code_query.query_map(
        params!(),
        |row| Ok(raffle_code_from_row(row)))?;
        Ok(HashSet::from_iter(
            found_codes
            .into_iter()
            .map(
                |row| row.unwrap()
            )))
    }

    fn get_raffle_codes_used_by_user(&self, user_id: UserID) -> RaffleResult<HashSet<RedeemableCodeId>> {
        let mut raffle_code_query = self.connection.prepare_cached(
        "SELECT code_id FROM USED_CODES
            WHERE
                user_id = ?1").unwrap();
        let found_codes = raffle_code_query.query_map(
        params!(user_id),
        |row| Ok(row.get_unwrap(0)))?;
        Ok(HashSet::from_iter(
            found_codes
            .into_iter()
            .map(|
                row| row.unwrap()
            )))
    }
    fn get_raffle_code_by_id(&self, code: RedeemableCodeId) -> RaffleResult<Option<RedeemableCode>> {
        let mut raffle_code_query = self.connection.prepare_cached(
            "SELECT * FROM REDEEMABLE_CODES
                WHERE code_id == ?1").unwrap();
        let found_codes = raffle_code_query.query_row(
            params!(code),
            |row| Ok(raffle_code_from_row(row)));
        match found_codes {
            Ok(code) => Ok(Some(code)),
            Err(_) => Ok(None),
        }
    }
    fn get_raffle_code_by_name(&self, name: &str) -> RaffleResult<Option<RedeemableCode>> {
        let mut raffle_code_query = self.connection.prepare_cached(
            "SELECT * FROM REDEEMABLE_CODES
                WHERE code == ?1").unwrap();
        let found_codes = raffle_code_query.query_row(
            params!(name),
            |row| Ok(raffle_code_from_row(row)));
        match found_codes {
            Ok(code) => Ok(Some(code)),
            Err(_) => Ok(None)
        }
    }

    fn partecipant_has_redeemed_code(&self, partecipant_id: UserID, code_id: RedeemableCodeId) -> RaffleResult<bool> {
        let mut redeem_query = self.connection.prepare_cached(
            "SELECT COUNT(*) FROM USED_CODES
            WHERE
                user_id == ?1 AND code_id == ?2").unwrap();
        let result : u8 = redeem_query.query_row(
            params!(partecipant_id, code_id),
            |row| Ok(row.get(0).unwrap()))?;
        Ok(result > 0)
    }
}