//! Conversões entre tipos do domínio e valores do SQLite.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rusqlite::types::Type;

use crate::Error;

/// Mapeia erros do rusqlite para o erro do núcleo.
pub(crate) trait DbResult<T> {
    fn db(self) -> crate::Result<T>;
}

impl<T> DbResult<T> for rusqlite::Result<T> {
    fn db(self) -> crate::Result<T> {
        self.map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound("registro".into()),
            other => Error::Store(other.to_string()),
        })
    }
}

/// Timestamp em ISO-8601 UTC com microssegundos e sufixo `Z` (ordenável como texto).
pub(crate) fn ts(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Micros, true)
}

pub(crate) fn ts_opt(t: Option<DateTime<Utc>>) -> Option<String> {
    t.map(ts)
}

pub(crate) fn date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub(crate) fn date_opt(d: Option<NaiveDate>) -> Option<String> {
    d.map(date)
}

fn conv_err(idx: usize, e: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(idx, Type::Text, Box::new(e))
}

pub(crate) fn get_ts(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<DateTime<Utc>> {
    let s: String = row.get(idx)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| conv_err(idx, e))
}

pub(crate) fn get_ts_opt(
    row: &rusqlite::Row<'_>,
    idx: usize,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
    match row.get::<_, Option<String>>(idx)? {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|t| Some(t.with_timezone(&Utc)))
            .map_err(|e| conv_err(idx, e)),
    }
}

pub(crate) fn get_date(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<NaiveDate> {
    let s: String = row.get(idx)?;
    NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|e| conv_err(idx, e))
}

pub(crate) fn get_date_opt(
    row: &rusqlite::Row<'_>,
    idx: usize,
) -> rusqlite::Result<Option<NaiveDate>> {
    match row.get::<_, Option<String>>(idx)? {
        None => Ok(None),
        Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map(Some)
            .map_err(|e| conv_err(idx, e)),
    }
}

/// Lê um enum textual (`str_enum!` do `model`).
pub(crate) fn get_enum<T>(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<T>
where
    T: std::str::FromStr<Err = Error>,
{
    let s: String = row.get(idx)?;
    s.parse::<T>().map_err(|e| conv_err(idx, e))
}
