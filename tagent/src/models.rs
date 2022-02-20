use crate::schema::*;
use diesel::Queryable;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Acl {
    pub id: i32,
    pub subject: String,
    pub action: String,
    pub path: String,
    pub user: String,
    pub create_by: String,
    pub create_time: String,
}

#[derive(Debug, Insertable)]
#[table_name = "acls"]
pub struct NewAcl<'a> {
    pub subject: &'a str,
    pub action: &'a str,
    pub path: &'a str,
    pub user: &'a str,
    pub create_by: &'a str,
    pub create_time: &'a str,
}


