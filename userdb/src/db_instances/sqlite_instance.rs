use std::collections::HashSet;
use std::ops::Add;
use rand::Rng;
use rusqlite::{Connection, CachedStatement, Result, params};
use crate::db::Result as RaffleResult;
use crate::db::*;

pub struct SQLiteInstance {    
    connection: rusqlite::Connection,
}

fn timestamp_now() -> Timestamp {
    std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs()
}

fn setup_connection(connection: &mut Connection) {
    connection.execute("
    CREATE TABLE IF NOT EXISTS PARTECIPANTS (
        user_id INTEGER NOT NULL PRIMARY KEY,
        joined_when INTEGER NOT NULL
    );
    ", params!()).unwrap();
    
    connection.execute("
    CREATE TABLE IF NOT EXISTS REDEEMABLE_CODES (
        code_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        code TEXT NOT NULL UNIQUE,
        remaining_uses INTEGER NOT NULL,
        generated_when INTEGER NOT NULL
    );
    ", params!()).unwrap();
    
    connection.execute("
    CREATE TABLE IF NOT EXISTS USED_CODES (
        user_id INTEGER NOT NULL,
        code_id INTEGER NOT NULL,
        used_when INTEGER NOT NULL,
        FOREIGN KEY (user_id) REFERENCES PARTECIPANTS(user_id),
        FOREIGN KEY (code_id) REFERENCES REDEEMABLE_CODES(code_id)
    );
    ", params!()).unwrap();
    
    // The referrer is the user that invited the referee in the raffle
    connection.execute("
    CREATE TABLE IF NOT EXISTS REFERRALS (
        referrer_id INTEGER NOT NULL,
        referee_id INTEGER NOT NULL,
        FOREIGN KEY(referrer_id) REFERENCES PARTECIPANTS(user_id),
        FOREIGN KEY(referee_id) REFERENCES PARTECIPANTS(user_id)
    );
    ", params!()).unwrap();
    
    connection.execute("
    CREATE TABLE IF NOT EXISTS RAFFLE (
        raffle_id INTEGER PRIMARY KEY AUTOINCREMENT,
        raffle_name TEXT NOT NULL,
        raffle_message BLOB NOT NULL,
        started_when INTEGER NOT NULL,
        ended_when INTEGER
    );
    ", params!()).unwrap();
    
    connection.execute("
    CREATE TABLE IF NOT EXISTS RAFFLE_WINNERS (
        raffle_id INTEGER,
        winner_id INTEGER,
        position INTEGER,
        FOREIGN KEY (raffle_id) REFERENCES RAFFLE(raffle_id)
    );
    ", params!()).unwrap();
}
fn raffle_from_row(row: &rusqlite::Row) -> Raffle {
    Raffle {
        raffle_id: row.get(0).unwrap(),
        raffle_name : row.get(1).unwrap(),
        raffle_description : row.get(2).unwrap(),
        started_when : row.get(3).unwrap(),
    }
}
fn raffle_code_from_row(row: &rusqlite::Row) -> RedeemableCode {
    RedeemableCode {
        unique_id: row.get(0).unwrap(),
        code : row.get(1).unwrap(),
        remaining_uses : row.get(2).unwrap(),
        generated_when: row.get(3).unwrap()
    }
}

fn partecipant_from_row(row: &rusqlite::Row, db: &SQLiteInstance) -> Partecipant {
    let user_id = row.get_unwrap(0);
    let referees = db.get_referees_of_user(user_id).len();
    let codes_used = db.get_raffle_codes_used_by_user(user_id).len();
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
    fn close(self) -> () {}
    
    // raffle functions
    fn create_raffle(&mut self, name: &str, description: &str) -> RaffleCreationResult {
        let ongoing_raffle = self.get_ongoing_raffle();
        if let Some(existing_raffle) = ongoing_raffle {
            RaffleCreationResult::OngoingRaffleExists(existing_raffle)
        } else {
            let time_since_epoch = timestamp_now();
            let insertion = self.connection.execute("
            INSERT INTO RAFFLE (raffle_name, raffle_message, started_when)
            VALUES (?1, ?2, ?3)
            ", params!(name, description, time_since_epoch));
            if let Err(o) = insertion {
                RaffleCreationResult::Other(format!("{}", o))
            } else {
                RaffleCreationResult::Success(self.get_ongoing_raffle().unwrap())
            }
        }
    }
    fn get_ongoing_raffle(&self) -> Option<Raffle> {
        let running_raffle= self.connection.query_row("
        SELECT * FROM RAFFLE WHERE RAFFLE.ended_when IS NULL
        ", params!(), |row| Ok(raffle_from_row(&row)));

        if let Ok(raffle) = running_raffle {
            Some(raffle)
        } else {
            None
        }
    }
    fn stop_raffle(&mut self, num_winners: usize) -> RaffleResult<Vec<Partecipant>> {
        let ongoing_raffle = self.get_ongoing_raffle();
        if let Some(raffle) = ongoing_raffle {
            let mut statement = self.connection.prepare_cached("
            UPDATE RAFFLE
            SET ended_when = ?1
            WHERE
                raffle_id == ?2
            ").unwrap();
            let result = statement.execute(params!(timestamp_now(), raffle.raffle_id));
            match result {
            Err(e) => Err(format!("{}", e)),
            Ok(rows) =>
                if rows > 0 {
                    let mut partecipants = Vec::from_iter(self.get_partecipants().into_iter());
                    partecipants
                        .sort_by(|a, b|  b.priority.cmp(&a.priority));
                    Ok(Vec::from_iter(partecipants
                        .iter()
                        .take(num_winners)
                        .map(|p| p.clone())
                    ))
                } else {
                    Err("No running raffle with this id".to_owned())
                }
            }   
        } else {
            Err("No ongoing raffle".to_owned())
        }
    }

    // user functions
    fn get_partecipants(&self) -> HashSet<Partecipant> {
        let mut partecipants_statement = self.connection.prepare_cached(
            "SELECT * FROM PARTECIPANTS"
        ).unwrap();
        let partecipants_from_db = partecipants_statement.query_map([], |row| Ok(partecipant_from_row(&row, &self))).unwrap();
        HashSet::from_iter(partecipants_from_db.map(|row| row.unwrap()))
    }
    fn is_partecipant(&self, user_id: UserID) -> bool {
        let mut partecipant_query = self.connection.prepare_cached(
            "SELECT COUNT(*) FROM PARTECIPANTS
            WHERE
                user_id == ?1").unwrap();
        let partecipant_count = partecipant_query.query_row(params!(user_id), |row| Ok(row.get_unwrap::<usize, u64>(0))).unwrap();
        partecipant_count > 0
    }
    fn get_partecipant(&self, user_id: UserID) -> Option<Partecipant> {
        let mut partecipant_query = self.connection.prepare_cached(
            "SELECT * FROM PARTECIPANTS
            WHERE
                user_id == ?1").unwrap();
        let partecipant_maybe = partecipant_query.query_row(params!(user_id), |row| Ok(partecipant_from_row(&row, &self)));
        if let Ok(partecipant) = partecipant_maybe {
            Some(partecipant)
        } else {
            None
        }
    }
    fn register_partecipant(&mut self, user_id: UserID, referrer: Option<UserID>) -> RegistrationStatus{
        let mut register_query = self.connection.prepare_cached(
            "INSERT INTO PARTECIPANTS (user_id, joined_when) 
            VALUES (?1, ?2)").unwrap();
        let now = timestamp_now();
        let insertion_result = register_query.execute(params!(user_id, now));
        if let Err(_) = insertion_result {
            RegistrationStatus::NotRegistered
        } else {
            if let Some(referrer_id) = referrer {
                if self.is_partecipant(referrer_id) {
                    let mut referral_query = self.connection.prepare_cached(
                        "INSERT INTO REFERRALS (referrer_id, referee_id)
                        VALUES (?1, ?2)"
                    ).unwrap();
                    let insertion_result = referral_query.execute(params!(referrer_id, user_id));
                    if let Err(e) = insertion_result {
                        todo!();
                    }
                }
            }
            RegistrationStatus::Registered(self.get_partecipant(user_id).unwrap())
        }
        
    }
    fn remove_partecipant(&mut self, user_id: UserID) -> RaffleResult<bool> {
        let mut remove_query = self.connection.prepare_cached(
            "DELETE FROM PARTECIPANTS
            WHERE user_id == ?1").unwrap();
        let result = remove_query.execute(params!(user_id));
        match result {
            Ok(number) => Ok(number > 0),
            Err(e) => Err(format!("{}", e))
        }
    }
    fn get_registration_status(&self, user_id: UserID) -> RegistrationStatus{
        if let Some(partecipant) = self.get_partecipant(user_id) {
            RegistrationStatus::Registered(partecipant)
        } else {
            RegistrationStatus::NotRegistered
        }
    }
    fn get_referees_of_user(&self, user_id: UserID) -> Vec<UserID> {
        let mut referees_query = self.connection.prepare_cached(
            "SELECT referee_id FROM REFERRALS
            WHERE referrer_id == ?1").unwrap();
        let resulting_rows = referees_query.query_map(params!(user_id), 
        |row| row.get(0)).unwrap();
        Vec::from_iter(resulting_rows.into_iter().map(|row| row.unwrap()))
    }
    fn get_referrer_of_user(&self, user_id: UserID) -> Option<UserID> {
        let mut referees_query = self.connection.prepare_cached(
            "SELECT referrer_id FROM REFERRALS
            WHERE referee_id == ?1").unwrap();
        let resulting_rows = referees_query.query_row(params!(user_id), 
        |row| Ok(row.get(0).unwrap()));
        if let Ok(referrer_id) = resulting_rows {
            Some(referrer_id)
        } else {
            None
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
        match query.execute(params!(new_code, numeric_usages, timestamp_now())) {
            Ok(_) => Ok(self.get_raffle_code_by_name(new_code.as_str()).unwrap()),
            Err(e) => Err(format!("{}", e))
        }
        
    }
    fn delete_raffle_code(&mut self, code: RedeemableCodeId) -> bool {
        let mut query = self.connection
        .prepare_cached(
            "DELETE FROM REDEEMABLE_CODES
            WHERE code_id == ?1").unwrap();
        let result = query.execute(params!(code));
        match result {
            Ok(1) => true,
            _ => false
        }
    }

    fn validate_code(&self, code: &str) -> CodeValidation{
        if let Some(redeemable_code) = self.get_raffle_code_by_name(code) {
            CodeValidation::Valid(redeemable_code.unique_id)
        } else {
            CodeValidation::NotValid("Code not found".to_owned())
        }
    }
    fn redeem_code(&mut self, user_id: UserID, code_id: RedeemableCodeId) -> CodeRedeemalResult {
        let user = self.get_partecipant(user_id);
        if user == None {
            return CodeRedeemalResult::NonExistingUser;
        }

        let code = self.get_raffle_code_by_id(code_id);
        if let Some(_existing_code) = code {
            if self.partecipant_has_redeemed_code(user_id, code_id) {
                CodeRedeemalResult::AlreadyRedeemed
            } else {
                let redeem_transaction = self.connection
                    .transaction()
                    .unwrap();
                {
                    let mut insert_query =
                    redeem_transaction.prepare_cached("INSERT INTO USED_CODES (user_id, code_id, used_when)
                        VALUES (?1, ?2, ?3)").unwrap();
                    insert_query.execute(params!(user_id, code_id, timestamp_now())).unwrap();
                    
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
                    let n = update_codes_query.execute(params!(code_id)).unwrap();
                }
                {
                    let mut delete_expired_codes_query = redeem_transaction.prepare_cached("
                    DELETE FROM REDEEMABLE_CODES
                    WHERE remaining_uses = 0
                    ").unwrap();
                    let n = delete_expired_codes_query.execute(params!()).unwrap();
                }
                redeem_transaction.commit().unwrap();
                CodeRedeemalResult::Redeemed
            }
        } else {
            CodeRedeemalResult::NonExistingCode
        }
    }

    fn get_raffle_codes(&self) -> HashSet<RedeemableCode> {
        let mut raffle_code_query = self.connection.prepare_cached(
        "SELECT * FROM REDEEMABLE_CODES").unwrap();
        let found_codes = raffle_code_query.query_map(
        params!(),
        |row| Ok(raffle_code_from_row(row))).unwrap();
        HashSet::from_iter(found_codes.into_iter().map(|row| row.unwrap()))
    }

    fn get_raffle_codes_used_by_user(&self, user_id: UserID) -> HashSet<RedeemableCodeId> {
        let mut raffle_code_query = self.connection.prepare_cached(
        "SELECT code_id FROM USED_CODES
            WHERE
                user_id = ?1").unwrap();
        let found_codes = raffle_code_query.query_map(
        params!(user_id),
        |row| Ok(row.get_unwrap(0))).unwrap();
        HashSet::from_iter(found_codes.into_iter().map(|row| row.unwrap()))
    }
    fn get_raffle_code_by_id(&self, code: RedeemableCodeId) -> Option<RedeemableCode> {
        let mut raffle_code_query = self.connection.prepare_cached(
            "SELECT * FROM REDEEMABLE_CODES
                WHERE code_id == ?1").unwrap();
        let found_codes = raffle_code_query.query_row(
            params!(code),
            |row| Ok(raffle_code_from_row(row)));
        if let Ok(redeemable_code) = found_codes {
            Some(redeemable_code)
        } else {
            None
        }
    }
    fn get_raffle_code_by_name(&self, name: &str) -> Option<RedeemableCode> {
        let mut raffle_code_query = self.connection.prepare_cached(
            "SELECT * FROM REDEEMABLE_CODES
                WHERE code == ?1").unwrap();
        let found_codes = raffle_code_query.query_row(
            params!(name),
            |row| Ok(raffle_code_from_row(row)));
        if let Ok(redeemable_code) = found_codes {
            Some(redeemable_code)
        } else {
            None
        }
    }

    fn partecipant_has_redeemed_code(&self, partecipant_id: UserID, code_id: RedeemableCodeId) -> bool {
        let mut redeem_query = self.connection.prepare_cached(
            "SELECT COUNT(*) FROM USED_CODES
            WHERE
                user_id == ?1 AND code_id == ?2").unwrap();
        let result : u8 = redeem_query.query_row(
            params!(partecipant_id, code_id),
            |row| Ok(row.get(0).unwrap()))
            .unwrap();
        result > 0
    }
}