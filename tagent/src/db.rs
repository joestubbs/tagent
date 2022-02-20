use diesel::prelude::*;
use dotenv::dotenv;
use chrono::prelude::{DateTime, Utc};
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
    let dt: DateTime<Utc> = st.clone().into();
    format!("{}", dt.format("%+"))
    // formats like "2001-07-08T00:34:60.026490+09:30"
}

pub fn create_acl(conn: &mut SqliteConnection, subject: &str, action: &str, path: &str, user: &str, create_by: &str) -> Result<usize, diesel::result::Error> {
    let now = SystemTime::now();
    let new_acl = NewAcl {subject, action, path, user, create_by, create_time: &iso8601(&now)};
    diesel::insert_into(acls::table).values(&new_acl).execute(conn)
}