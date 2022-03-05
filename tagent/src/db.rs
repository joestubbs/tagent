use diesel::prelude::*;
// use diesel::{Connection};
use crate::models::{AclAction, AclDecision, DbAcl};
use chrono::prelude::{DateTime, Utc};
use dotenv::dotenv;
use log::{debug, info};
use regex::Regex;
use std::env;
use std::time::SystemTime;

use super::models::{NewAcl, NewAclJson};
use super::schema::acls;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    // TODO -- do not panic on error
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

    let mut new_path = String::from("/");

    // every path must start with a slash
    if !(path.starts_with('/')) {
        new_path.push_str(path);
    } else {
        new_path = path.to_string();
    }

    let new_acl = NewAcl {
        subject,
        action: &action.to_string(),
        path: &new_path,
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

pub fn retrieve_acls_for_subject(
    conn: &mut SqliteConnection,
    sub: &str,
) -> Result<Vec<DbAcl>, diesel::result::Error> {
    use crate::schema::acls::subject;
    acls::dsl::acls.filter(subject.eq(sub)).load::<DbAcl>(conn)
}

pub fn retrieve_acls_for_subject_user(
    conn: &mut SqliteConnection,
    sub: &str,
    usr: &str,
) -> Result<Vec<DbAcl>, diesel::result::Error> {
    use crate::schema::acls::subject;
    use crate::schema::acls::user;
    acls::dsl::acls
        .filter(subject.eq(sub))
        .filter(user.eq(usr))
        .load::<DbAcl>(conn)
}

pub fn retrieve_acl_by_id(
    conn: &mut SqliteConnection,
    id: i32,
) -> Result<DbAcl, diesel::result::Error> {
    acls::dsl::acls.find(id).first(conn)
}

pub fn delete_acl_from_db_by_id(
    conn: &mut SqliteConnection,
    acl_id: i32,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::acls::id;
    diesel::delete(acls::table.filter(id.eq(&acl_id))).execute(conn)
}

pub fn update_acl_in_db_by_id(
    conn: &mut SqliteConnection,
    acl_id: i32,
    new_acl: &NewAclJson,
    new_subject: &str,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::acls::action;
    use crate::schema::acls::create_by;
    use crate::schema::acls::decision;
    use crate::schema::acls::id;
    use crate::schema::acls::path;
    use crate::schema::acls::subject;
    use crate::schema::acls::user;

    diesel::update(acls::table.filter(id.eq(&acl_id)))
        .set((
            action.eq(new_acl.action.to_string()),
            subject.eq(new_acl.subject.clone()),
            path.eq(new_acl.path.clone()),
            user.eq(new_acl.user.clone()),
            decision.eq(new_acl.decision.to_string()),
            create_by.eq(new_subject),
        ))
        .execute(conn)
}

// checks whether a field with a wildcard character matches another field value
pub fn check_acl_field_with_wildcard_for_match(acl_field: &str, field: &str) -> bool {
    let re = Regex::new(acl_field);
    let re = match re {
        Ok(re) => re,
        Err(e) => {
            let msg = format!("acl was not a valid regex; details: {}", e);
            info!("{}", msg);
            return false;
        }
    };
    re.is_match(field)
}

// checks if a DB ACL matches a set of criteria
pub fn check_acl_for_match(sub: &str, usr: &str, pth: &str, act: &str, acl: &DbAcl) -> bool {
    debug!("top of check_acl_for_match for acl: {}", acl.id);
    // subject must be an exact match
    if sub != acl.subject {
        debug!("subject didn't match; returning false");
        return false;
    };
    // user field allowed to have wild cards
    if usr != acl.user {
        debug!("user isn't exact match");
        // if it wasn't an exact match and the acl doesn't contain wildcards, it's not a match
        if !acl.user.contains('*') {
            debug!("acl user was not a regex; returning false");
            return false;
        };
        debug!("acl user WAS a regex; checking for wildcard match");
        // special check for acl with wildcard
        if !(check_acl_field_with_wildcard_for_match(&acl.user, usr)) {
            debug!("acl user regex didn't match; returning false");
            return false;
        };
    };
    // path field allowed to have wild cards
    if pth != acl.path {
        debug!("patch isn't exact match");
        // if it wasn't an exact match and the acl doesn't contain wildcards, it's not a match
        if !acl.path.contains('*') {
            debug!("acl path was not a regex; returning false");
            return false;
        };
        debug!("acl path WAS a regex; checking for wildcard match");
        // special check for acl with wildcard
        if !(check_acl_field_with_wildcard_for_match(&acl.path, pth)) {
            debug!("acl path regex didn't match; returning false");
            return false;
        };
    };

    if acl.action != act {
        // actions have a hierarchy, with "higher" values implying lower values
        // Read < Execute < Write
        // whether the ACL matches depends on the decision associated with the ACL.
        // in case Deny, lower ACL values match because a Deny of a lower action implies deny for higher actions.
        // in case Allow, higher ACL values match because an Allow of a higher action implies allow for lower actions
        if acl.decision == "Allow" {
            if acl.is_leq_action(act) {
                return false;
            }
        }
        // acl decision is "Deny" so it is only a match if the acl action is greater than
        else {
            debug!(
                "checking Deny ACL action ({}) against action ({})",
                acl.action, act
            );
            if !(acl.is_leq_action(act)) {
                debug!("Deny ACL action was not less than action.. returning false");
                return false;
            }
        }
    };
    debug!("db_acl with id {} matched request", acl.id);
    true
}

pub fn is_authz_db(
    conn: &mut SqliteConnection,
    sub: &str,
    usr: &str,
    pth: &str,
    act: &AclAction,
) -> Result<bool, diesel::result::Error> {
    use crate::schema::acls::decision;
    use crate::schema::acls::subject;

    // first check for a matching ACL with a Deny decision
    let deny_str = AclDecision::Deny.to_string();
    let deny_acls = acls::dsl::acls
        .filter(subject.eq(&sub))
        .filter(decision.eq(&deny_str))
        .load::<DbAcl>(conn)?;
    for acl in deny_acls {
        if check_acl_for_match(sub, usr, pth, &act.to_string(), &acl) {
            return Ok(false);
        }
    }
    // check for any matching ACL with an Allow decision
    let allow_str = AclDecision::Allow.to_string();
    let allow_acls = acls::dsl::acls
        .filter(subject.eq(&sub))
        .filter(decision.eq(&allow_str))
        .load::<DbAcl>(conn)?;
    for acl in allow_acls {
        if check_acl_for_match(sub, usr, pth, &act.to_string(), &acl) {
            return Ok(true);
        }
    }
    debug!("no ACL matched; returning default decision (false)");
    // if no ACL matched then the action is not authorized by default
    Ok(false)
}
