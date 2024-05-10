use diesel::prelude::*;

#[derive(Queryable, Selectable, PartialEq)]
#[diesel(table_name = crate::schema::videos)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Video {
    pub id: i64,
    pub uid: String,
    pub link: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub duration: Option<i64>,
    pub description: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schema::videos)]
pub struct NewVideo {
    pub uid: String,
    pub link: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub duration: Option<i64>,
    pub description: Option<String>,
}
