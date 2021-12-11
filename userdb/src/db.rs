use std::collections::HashSet;

pub type UserID = i64;
pub type Timestamp = u64;
pub type RedeemableCodeId = u64;
pub type Result<T> = std::result::Result<T, String>;

pub struct Partecipant {
    user_id: UserID,
    referred_by: Option<UserID>,
    joined_when: Timestamp,
    priority: u32, // 1 + #codes used + 1 for each referred user
}

pub struct RedeemableCode {
    code: String,
    unique_id: RedeemableCodeId,
    remaining_uses: i32,
    generated_when: Timestamp,
}

pub struct UsedCode {
    partecpiant_user_id: UserID,
    code: String,
    used_when: Timestamp,
}

pub struct Raffle {
    raffle_name: String,
    raffle_description: String, // something else
    started_when: Timestamp,
}

impl Raffle {
    pub fn create(name: &str, description: &str) -> Raffle {
        let time_since_epoch = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();
        Raffle {
            raffle_name : name.to_owned(),
            raffle_description: description.to_owned(),
            started_when: time_since_epoch.as_secs()
        }
    }
}

pub enum RegistrationStatus {
    Registered(Partecipant),
    NotRegistered
}

pub enum CodeUseCount {
    Counted(i32), // Code can be used a max of n times,
    Once, // = Counted(1),
    Illimited
}

pub enum CodeValidation {
    Valid,
    NotValid(String) // The String is the reason why the code is not valid
}

pub enum RaffleCreationResult {
    Success,
    OngoingRaffleExists,
    Other(String) // The String is the reason why the raffle failed
}

pub trait RaffleDB {
    fn close(self) -> ();
    
    // raffle functions
    fn create_raffle(&mut self, raffle: Raffle) -> RaffleCreationResult;
    fn get_ongoing_raffle(&self) -> Option<Raffle>;
    fn stop_raffle(&mut self);
    fn pick_winners(&mut self, num_winners: u8) -> Vec<Partecipant>;

    // user functions
    fn get_partecipants(&self) -> HashSet<Partecipant>;
    fn register_partecipant(&mut self, user_id: UserID) -> RegistrationStatus;
    fn get_registration_status(&self, user_id: UserID) -> RegistrationStatus;
    
    // raffle codes functions
    fn generate_raffle_code(&mut self, use_count: CodeUseCount) -> Result<RedeemableCode>;
    fn delete_raffle_code(&mut self, code: RedeemableCodeId);

    fn validate_code(&self, code: String) -> CodeValidation;
    fn redeem_code(&mut self, user_id: UserID, code_id: RedeemableCodeId);
    
}