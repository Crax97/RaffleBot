use std::{collections::HashSet, hash::Hash};

pub type UserID = i64;
pub type RaffleID = u64;
pub type Timestamp = u64;
pub type RedeemableCodeId = u64;
pub type RaffleResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Eq, Clone)]
pub struct Partecipant {
    pub user_id: UserID,
    pub joined_when: Timestamp,
    pub priority: usize, // 1 + #codes used + 1 for each referred user
}

impl PartialEq for Partecipant {
    fn eq(&self, other: &Self) -> bool {
        return self.user_id == other.user_id;
    } 
}
impl Hash for Partecipant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.user_id.hash(state);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Referral {
    pub referrer: UserID,
    pub referee: UserID,
}

#[derive(Debug, Eq)]
pub struct RedeemableCode {
    pub code: String,
    pub unique_id: RedeemableCodeId,
    pub remaining_uses: i32,
    pub generated_when: Timestamp,
}

impl PartialEq for RedeemableCode {
    fn eq(&self, other: &Self) -> bool {
        self.unique_id == other.unique_id 
    }
}

impl Hash for RedeemableCode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.code.hash(state);
        self.unique_id.hash(state);
        self.remaining_uses.hash(state);
        self.generated_when.hash(state);
    }
}

#[derive(Debug)]
pub struct UsedCode {
    pub partecpiant_user_id: UserID,
    pub code: String,
    pub used_when: Timestamp,
}

#[derive(Debug)]
pub struct Raffle {
    pub raffle_id: RaffleID,
    pub raffle_name: String,
    pub raffle_description: String, // something else
    pub started_when: Timestamp,
}
impl PartialEq for Raffle {
    fn eq(&self, other: &Self) -> bool {
        return self.raffle_id == other.raffle_id;
    } 
}

#[derive(Debug, PartialEq)]
pub enum RegistrationStatus {
    Registered(Partecipant),
    NotRegistered,
    GenericError(String)
}

#[derive(Debug, PartialEq)]
pub enum CodeUseCount {
    Counted(i32), // Code can be used a max of n times,
    Once, // = Counted(1),
    Expired,
    Illimited,
    CodeNotValid,
}
#[derive(Debug, PartialEq)]
pub enum CodeValidation {
    Valid(RedeemableCodeId),
    NotValid(String) // The String is the reason why the code is not valid
}

#[derive(Debug, PartialEq)]
pub enum CodeRedeemalResult {
    Redeemed,
    AlreadyRedeemed,
    NonExistingUser,
    NonExistingCode
}
#[derive(Debug, PartialEq)]
pub enum RaffleCreationResult {
    Success(Raffle),
    OngoingRaffleExists(Raffle),
    Generic(String) // The String is the reason why the raffle failed
}

impl RaffleCreationResult {
    pub fn is_success(&self) -> bool {
        match self {
            RaffleCreationResult::Success(_) => true,
            _ => false
        }
    }
}

pub trait RaffleDB {
    fn close(self) -> RaffleResult<()>;
    
    // raffle functions
    fn create_raffle(&mut self, name: &str, description: &str) -> RaffleResult<RaffleCreationResult>;
    fn get_ongoing_raffle(&self) -> RaffleResult<Option<Raffle>>;
    fn stop_raffle(&mut self, num_winners: usize) -> RaffleResult<Vec<Partecipant>>;

    // user functions
    fn get_partecipants(&self) -> RaffleResult<HashSet<Partecipant>>;
    fn get_partecipant(&self, user_id: UserID) -> RaffleResult<Option<Partecipant>>;
    fn is_partecipant(&self, user_id: UserID) -> RaffleResult<bool>;
    fn register_partecipant(&mut self, user_id: UserID, referrer: Option<UserID>) -> RaffleResult<RegistrationStatus>;
    fn remove_partecipant(&mut self, user_id: UserID) -> RaffleResult<bool>;
    fn get_registration_status(&self, user_id: UserID) -> RaffleResult<RegistrationStatus>;
    fn get_referees_of_user(&self, user_id: UserID) -> RaffleResult<Vec<UserID>>;
    fn get_referrer_of_user(&self, user_id: UserID) -> RaffleResult<Option<UserID>>;

    // raffle codes functions
    fn generate_raffle_code(&mut self, use_count: CodeUseCount) -> RaffleResult<RedeemableCode>;
    fn get_raffle_codes(&self) -> RaffleResult<HashSet<RedeemableCode>>;
    fn get_raffle_codes_used_by_user(&self, user_id: UserID) -> RaffleResult<HashSet<RedeemableCodeId>>;
    fn get_raffle_code_by_name(&self, name: &str) -> RaffleResult<Option<RedeemableCode>>;
    fn get_raffle_code_by_id(&self, code: RedeemableCodeId) -> RaffleResult<Option<RedeemableCode>>;
    fn partecipant_has_redeemed_code(&self, partecipant_id: UserID, code_id: RedeemableCodeId) -> RaffleResult<bool>;
    fn delete_raffle_code(&mut self, code: RedeemableCodeId) -> RaffleResult<bool>;

    fn validate_code(&self, code: &str) -> RaffleResult<CodeValidation>;
    fn redeem_code(&mut self, user_id: UserID, code_id: RedeemableCodeId) -> RaffleResult<CodeRedeemalResult>;
    
}