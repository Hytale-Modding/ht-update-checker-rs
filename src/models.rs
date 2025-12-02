use chrono::{DateTime, Utc};
use diesel::prelude::*;

#[allow(unused)]
type DB = diesel::sqlite::Sqlite;

#[derive(Queryable, Selectable, Identifiable, Debug, PartialEq)]
#[diesel(check_for_backend(DB))]
#[diesel(primary_key(version))]
#[diesel(table_name = crate::schema::launcher_versions)]
pub struct LauncherVersion {
    pub version: String,

    pub created_at: DateTime<Utc>
}

#[derive(Insertable, Identifiable)]
#[diesel(check_for_backend(DB))]
#[diesel(primary_key(version))]
#[diesel(table_name = crate::schema::launcher_versions)]
pub struct NewLauncherVersion<'a> {
    pub version: &'a str,
}

#[derive(Queryable, Selectable, Identifiable, Debug, PartialEq)]
#[diesel(primary_key(version))]
#[diesel(check_for_backend(DB))]
#[diesel(table_name = crate::schema::java_versions)]
pub struct JavaVersion {
    pub version: String,

    #[diesel(serialize_as = diesel::sql_types::Timestamp)]
    pub created_at: DateTime<Utc>
}

#[derive(Insertable, Identifiable)]
#[diesel(primary_key(version))]
#[diesel(table_name = crate::schema::java_versions)]
pub struct NewJavaVersion<'a> {
    pub version: &'a str,
}

#[derive(Queryable, Selectable, Identifiable, Debug, PartialEq)]
#[diesel(check_for_backend(DB))]
#[diesel(primary_key(build_number))]
#[diesel(table_name = crate::schema::game_versions)]
pub struct GameVersion {
    pub build_number: i32,
    pub version: String,

    pub created_at: DateTime<Utc>
}

#[derive(Insertable, Identifiable)]
#[diesel(check_for_backend(DB))]
#[diesel(primary_key(build_number))]
#[diesel(table_name = crate::schema::game_versions)]
pub struct NewGameVersion<'a> {
    pub build_number: i32,
    pub version: &'a str
}

#[derive(Queryable, Selectable, Identifiable, Debug, PartialEq)]
#[diesel(check_for_backend(DB))]
#[diesel(primary_key(channel))]
#[diesel(belongs_to(LauncherVersion))]
#[diesel(belongs_to(JavaVersion))]
#[diesel(belongs_to(GameVersion))]
#[diesel(table_name = crate::schema::update_channels)]
pub struct UpdateChannel {
    pub channel: String,

    pub launcher_version_id: String,
    pub java_version_id: String,
    pub game_version_id: i32,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>
}

#[derive(Insertable, Identifiable, Default)]
#[diesel(check_for_backend(DB))]
#[diesel(primary_key(channel))]
#[diesel(table_name = crate::schema::update_channels)]
pub struct NewUpdateChannel<'a> {
    pub channel: &'a str,

    pub launcher_version_id: &'a str,
    pub java_version_id: &'a str,
    pub game_version_id: i32,

    pub updated_at: Option<DateTime<Utc>>
}

impl NewUpdateChannel<'_> {
    pub fn get_as_changeset(value: &Self, updated_at: Option<DateTime<Utc>>) -> SetUpdateChannel {
        SetUpdateChannel {
            channel: value.channel.to_string(),

            launcher_version_id: value.launcher_version_id.to_string(),
            java_version_id: value.java_version_id.to_string(),
            game_version_id: value.game_version_id,

            updated_at: updated_at.unwrap_or_else(Utc::now),
        }
    }
}

#[derive(AsChangeset)]
#[diesel(check_for_backend(DB))]
#[diesel(primary_key(channel))]
#[diesel(table_name = crate::schema::update_channels)]
pub struct SetUpdateChannel {
    pub channel: String,

    pub launcher_version_id: String,
    pub java_version_id: String,
    pub game_version_id: i32,

    pub updated_at: DateTime<Utc>
}

impl SetUpdateChannel {
    pub fn convert(value: &NewUpdateChannel, updated_at: &DateTime<Utc>) -> Self {
        Self {
            channel: value.channel.to_string(),
            launcher_version_id: value.launcher_version_id.to_string(),
            java_version_id: value.java_version_id.to_string(),
            game_version_id: value.game_version_id,
            updated_at: updated_at.clone(),
        }
    }
}
