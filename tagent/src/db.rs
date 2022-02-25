use diesel::prelude::*;
// use diesel::{Connection};
use crate::models::{AclAction, DbAcl, AclDecision};
use chrono::prelude::{DateTime, Utc};
use dotenv::dotenv;
use std::env;
use std::time::SystemTime;

use super::models::NewAcl;
use super::schema::acls;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

// convert current system time to iso8601
// cf., https://stackoverflow.com/questions/64146345/how-do-i-convert-a-systemtime-to-iso-8601-in-rust
fn iso8601(st: &SystemTime) -> String {
    let dt: DateTime<Utc> = (*st).into();
    format!("{}", dt.format("%+"))
    // formats like "2001-07-08T00:34:60.026490+09:30"
}

pub fn save_acl(
    conn: &mut SqliteConnection,
    subject: &str,
    action: &AclAction,
    path: &str,
    user: &str,
    decision: &AclDecision,
    create_by: &str,
) -> Result<usize, diesel::result::Error> {
    let now = SystemTime::now();
    let new_acl = NewAcl {
        subject,
        action: &action.to_string(),
        path,
        user,
        decision: &decision.to_string(),
        create_by,
        create_time: &iso8601(&now),
    };
    diesel::insert_into(acls::table)
        .values(&new_acl)
        .execute(conn)
}

pub fn retrieve_all_acls(conn: &mut SqliteConnection) -> Result<Vec<DbAcl>, diesel::result::Error> {
    acls::dsl::acls.load::<DbAcl>(conn)
}

pub fn retrieve_acl_by_id(conn: &mut SqliteConnection, id: i32) -> Result<DbAcl, diesel::result::Error> {
    acls::dsl::acls.find(id).first(conn)
}

pub fn delete_acl_from_db_by_id(conn: &mut SqliteConnection, acl_id: i32) -> Result<usize, diesel::result::Error> {
    use crate::schema::acls::id;
    diesel::delete(acls::table.filter(id.eq(&acl_id))).execute(conn)
}

pub fn update_acl_in_db_by_id(conn: &mut SqliteConnection, acl_id: i32, new_subject: &str,
    new_action: &AclAction,
    new_path: &str,
    new_user: &str,
    new_decision: &AclDecision,
    new_create_by: &str) -> Result<usize, diesel::result::Error>  {
        use crate::schema::acls::id;
        use crate::schema::acls::subject;
        use crate::schema::acls::action;
        use crate::schema::acls::path;
        use crate::schema::acls::user;
        use crate::schema::acls::create_by;
        use crate::schema::acls::decision;

        diesel::update(acls::table.filter(id.eq(&acl_id)))
        .set((action.eq(new_action.to_string()), 
        subject.eq(new_subject),
        path.eq(new_path), 
        user.eq(new_user), 
        decision.eq(new_decision.to_string()), create_by.eq(new_create_by)))
        .execute(conn)

    }