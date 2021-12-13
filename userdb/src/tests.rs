use crate::db_instances::sqlite_instance::SQLiteInstance;
use crate::db::*;

#[test]
fn test_db_raffle_execution() {
    let mut db = SQLiteInstance::create("./test.db").unwrap();
    let new_raffle = db.create_raffle("Test Raffle 2", "Test Description");
    assert!(new_raffle.is_success());
    db.register_partecipant(0, None);
    assert_eq!(db.get_referrer_of_user(0), None);
    db.register_partecipant(1, None);
    assert_eq!(db.get_referrer_of_user(1), None);
    db.register_partecipant(2, Some(1));
    assert_eq!(db.get_referees_of_user(1).len(), 1);
    assert_eq!(db.get_referrer_of_user(2).unwrap(), 1);

    let current_partecipants = db.get_partecipants();
    assert_eq!(current_partecipants.len(), 3);

    assert!(db.remove_partecipant(0).unwrap());
    let current_partecipants = db.get_partecipants();
    assert_eq!(current_partecipants.len(), 2);
    assert!(!current_partecipants.iter().fold(false, |v, p| v || p.user_id == 0));
    assert_eq!(db.get_registration_status(0), RegistrationStatus::NotRegistered);
    

    let new_code = db.generate_raffle_code(CodeUseCount::Once).unwrap();
    assert!(match db.validate_code(new_code.code.as_ref()) {
        CodeValidation::Valid(_) => true,
        _ => false
    });
    db.redeem_code(1, new_code.unique_id);
    let validation_now = db.validate_code(new_code.code.as_ref());
    assert!(match validation_now {
        CodeValidation::Valid(_) => false,
        _ => true
    });

    let new_code = db.generate_raffle_code(CodeUseCount::Counted(10)).unwrap();
    assert_eq!(db.redeem_code(1, new_code.unique_id), CodeRedeemalResult::Redeemed);
    assert_eq!(db.redeem_code(1, new_code.unique_id), CodeRedeemalResult::AlreadyRedeemed);
    assert_eq!(db.redeem_code(2, new_code.unique_id), CodeRedeemalResult::Redeemed);
    
    let usage_count = db.get_raffle_code_by_id(new_code.unique_id).unwrap().remaining_uses;
    assert_eq!(usage_count, 8);
    assert!(db.delete_raffle_code(new_code.unique_id));
    assert!(!db.delete_raffle_code(new_code.unique_id));
    let usage_count = db.get_raffle_code_by_id(new_code.unique_id);
    assert_eq!(usage_count, None);
    

    for i in 10..20 {
        db.register_partecipant(i, Some(2));
    }
    for i in 11..21 {
        db.register_partecipant(i, None);
    }

    let winners = db.stop_raffle(3).unwrap();
    println!("Winners: {:?}", winners);
    assert_eq!(winners.into_iter().next().unwrap().user_id, 2);

    db.close();
}