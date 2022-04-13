use diesel::prelude::*;
// use diesel::{Connection};
use crate::models::{AclAction, AclDecision, DbAcl};
use crate::representations::AuthAnswer;
use chrono::prelude::{DateTime, Utc};
use log::debug;
use std::env;
use std::fs::File;

use std::time::SystemTime;

use super::models::{NewAcl, NewAclJson};
use super::representations::AuthCheckError;
use super::schema::acls;

use diesel::r2d2::ConnectionManager;
use r2d2::{Pool, PooledConnection};

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

/// Create a sqlite connection pool; the db_name attribute is only used in testing; should be an absolute path
/// to a file to use for the connection pool. This allows for the creation of different connection pools for each test
/// or set of tests.
pub fn get_db_pool(db_name: Option<String>) -> DbPool {
    if cfg!(test) {
        let db_path = &db_name.expect("db_name must be provided");
        dbg!(&db_path);
        let _f = File::create(db_path).expect("could not create file at path {}");
        let mem_db = format!("/{}", db_path);
        // let mem_db = ":memory:";
        dbg!(&mem_db);
        let manager = ConnectionManager::<SqliteConnection>::new(mem_db);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create DB pool.");
        let _result = diesel_migrations::run_pending_migrations(&pool.get().unwrap());
        pool
    } else {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let manager = ConnectionManager::<SqliteConnection>::new(&database_url);
        r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create DB pool.")
    }
}

// convert current system time to iso8601
// cf., https://stackoverflow.com/questions/64146345/how-do-i-convert-a-systemtime-to-iso-8601-in-rust
fn iso8601(st: &SystemTime) -> String {
    let dt: DateTime<Utc> = (*st).into();
    format!("{}", dt.format("%+"))
    // formats like "2001-07-08T00:34:60.026490+09:30"
}

pub fn save_acl(
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
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

pub fn retrieve_all_acls(
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
) -> Result<Vec<DbAcl>, diesel::result::Error> {
    acls::dsl::acls.load::<DbAcl>(conn)
}

pub fn retrieve_acls_for_subject(
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
    sub: &str,
) -> Result<Vec<DbAcl>, diesel::result::Error> {
    use crate::schema::acls::subject;
    acls::dsl::acls.filter(subject.eq(sub)).load::<DbAcl>(conn)
}

pub fn retrieve_acls_for_subject_user(
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
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
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
    id: i32,
) -> Result<DbAcl, diesel::result::Error> {
    acls::dsl::acls.find(id).first(conn)
}

pub fn delete_acl_from_db_by_id(
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
    acl_id: i32,
) -> Result<usize, diesel::result::Error> {
    use crate::schema::acls::id;
    diesel::delete(acls::table.filter(id.eq(&acl_id))).execute(conn)
}

pub fn update_acl_in_db_by_id(
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
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

/// Checks whether a field with a wildcard character matches another field value
pub fn check_acl_glob_for_match(acl_field: &str, field: &str) -> Result<bool, glob::PatternError> {
    let options = glob::MatchOptions {
        case_sensitive: false,
        // Whether or not to require that the path separator (/) be  matched explicitly by a literal / in the
        // user-supplied pattern. Note that the consequence of this configuration is whether or not a pattern
        // with a star, such as /foo/bar/*, matches on all subdirectories.
        // That is, /foo/bar/* will always match /foo/bar/<any_file>, but with require_literal_separator set to false,
        // it will also match /foo/bar/baz/bop/<any_file>, etc.
        require_literal_separator: false,
        // set to true to prevent matching on subdirectories.
        // require_literal_separator: true,
        require_literal_leading_dot: false,
    };
    let gb = glob::Pattern::new(acl_field)?;
    Ok(gb.matches_with(field, options))
}

// checks if a DB ACL matches a set of criteria
pub fn check_acl_for_match(
    sub: &str,
    usr: &str,
    pth: &str,
    act: &str,
    acl: &DbAcl,
) -> Result<bool, glob::PatternError> {
    debug!("top of check_acl_for_match for acl: {}", acl.id);
    // subject must be an exact match
    if sub != acl.subject {
        debug!("subject didn't match; returning false");
        return Ok(false);
    };
    // user field allowed to have wild cards
    if usr != acl.user {
        debug!("user isn't exact match");
        if !(check_acl_glob_for_match(&acl.user, usr)?) {
            debug!("acl user glob didn't match; returning false");
            return Ok(false);
        };
    };
    // path field allowed to have wild cards
    if pth != acl.path {
        debug!("path isn't exact match");
        // special check for acl with wildcard
        if !(check_acl_glob_for_match(&acl.path, pth)?) {
            debug!("acl path glob didn't match; returning false");
            return Ok(false);
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
                return Ok(false);
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
                return Ok(false);
            }
        }
    };
    debug!("db_acl with id {} matched request", acl.id);
    Ok(true)
}

/// check whether a subject is authroized for an action on a path as a user.
pub fn is_authz_db(
    conn: &PooledConnection<ConnectionManager<diesel::SqliteConnection>>,
    sub: &str,
    usr: &str,
    pth: &str,
    act: &AclAction,
) -> Result<AuthAnswer, AuthCheckError> {
    use crate::schema::acls::decision;
    use crate::schema::acls::subject;

    // first check for a matching ACL with a Deny decision
    let deny_str = AclDecision::Deny.to_string();
    let deny_acls = acls::dsl::acls
        .filter(subject.eq(&sub))
        .filter(decision.eq(&deny_str))
        .load::<DbAcl>(conn)?;
    for acl in deny_acls {
        if check_acl_for_match(sub, usr, pth, &act.to_string(), &acl)? {
            return Ok(AuthAnswer {
                allowed: false,
                acl_id: Some(acl.id),
            });
        }
    }
    // check for any matching ACL with an Allow decision
    let allow_str = AclDecision::Allow.to_string();
    let allow_acls = acls::dsl::acls
        .filter(subject.eq(&sub))
        .filter(decision.eq(&allow_str))
        .load::<DbAcl>(conn)?;
    for acl in allow_acls {
        if check_acl_for_match(sub, usr, pth, &act.to_string(), &acl)? {
            return Ok(AuthAnswer {
                allowed: true,
                acl_id: Some(acl.id),
            });
        }
    }
    debug!("no ACL matched; returning default decision (false)");
    // if no ACL matched then the action is not authorized by default
    Ok(AuthAnswer {
        allowed: false,
        acl_id: None,
    })
}
