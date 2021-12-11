pub mod sqlite_instance;

#[cfg(test)]
mod tests {
    use crate::db_instances::sqlite_instance::SQLiteInstance;
    use crate::db::*;
    #[test]
    fn test_db_creation() {
        let db = SQLiteInstance::create("./test.db").unwrap();
        db.close();
    }
    #[test]
    fn test_db_raffle_creation() {
        let mut db = SQLiteInstance::create("./test.db").unwrap();
        let new_raffle = Raffle::create("Test Raffle", "Test Description");
        db.create_raffle(new_raffle);

        let created_raffle = db.get_ongoing_raffle();
        assert!(created_raffle.is_some());
        
        db.close();
    }
}