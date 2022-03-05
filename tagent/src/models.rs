use crate::schema::*;
use diesel::sql_types::Text;
use diesel::Queryable;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AclAction {
    Read,
    Execute,
    Write,
}

impl fmt::Display for AclAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Read => write!(f, "Read"),
            Self::Execute => write!(f, "Execute"),
            Self::Write => write!(f, "Write"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum AclDecision {
    Allow,
    Deny,
}

impl fmt::Display for AclDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Allow => write!(f, "Allow"),
            Self::Deny => write!(f, "Deny"),
        }
    }
}

// struct representing a database record retrieved from sqlite; the id attribute is included
#[derive(Debug, Serialize, Deserialize, Queryable, PartialEq)]
pub struct DbAcl {
    pub id: i32,
    pub subject: String,
    pub action: String,
    pub path: String,
    pub user: String,
    pub create_by: String,
    pub create_time: String,
    pub decision: String,
}

impl DbAcl {
    // determines whether a DbAcl represents a lower action than a given action
    pub fn is_leq_action(&self, action: &str) -> bool {
        if self.action == "Read" {
            // Read is less than every action
            return true;
        } else if self.action == "Execute" {
            if (action == "Execute") || (action == "Write") {
                return true;
            }
        } else {
            // acl action is Write, so only Write is leq..
            if action == "Write" {
                return true;
            }
        }
        false
    }
}

// struct representing an ACL row to insert into sqlite; the id attribute is not included
#[derive(Debug, Insertable)]
#[table_name = "acls"]
pub struct NewAcl<'a> {
    pub subject: &'a str,
    pub action: &'a str,
    pub path: &'a str,
    pub user: &'a str,
    pub create_by: &'a str,
    pub create_time: &'a str,
    pub decision: &'a str,
}

// struct representing a user-supplied JSON object describing a new ACL to be created
#[derive(Debug, Serialize, Deserialize, Insertable)]
#[table_name = "acls"]
pub struct NewAclJson {
    pub subject: String,
    pub action: AclAction,
    pub decision: AclDecision,
    pub path: String,
    pub user: String,
}

impl diesel::Expression for AclAction {
    type SqlType = Text;
}

impl diesel::Expression for AclDecision {
    type SqlType = Text;
}
