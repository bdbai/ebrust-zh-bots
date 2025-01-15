use std::{
    fmt::{self, Debug, Display, Formatter},
    marker::PhantomData,
    num::NonZeroI64,
};

use rusqlite::{types::FromSql, ToSql};
use serde::Serialize;

#[derive(Serialize)]
#[serde(transparent)]
pub struct Id<T: ?Sized>(pub NonZeroI64, PhantomData<T>);

impl<T: ?Sized> Debug for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Id({})", self.0)
    }
}

impl<T: ?Sized> Display for Id<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<T: ?Sized> Clone for Id<T> {
    fn clone(&self) -> Id<T> {
        *self
    }
}
impl<T: ?Sized> Copy for Id<T> {}
impl<T: ?Sized> PartialEq for Id<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.0.eq(&rhs.0)
    }
}
impl<T: ?Sized> Eq for Id<T> {}
impl<T: ?Sized> From<NonZeroI64> for Id<T> {
    fn from(id: NonZeroI64) -> Self {
        Self(id, PhantomData)
    }
}
impl<T: ?Sized> TryFrom<i64> for Id<T> {
    type Error = <NonZeroI64 as TryFrom<i64>>::Error;

    fn try_from(id: i64) -> Result<Self, Self::Error> {
        NonZeroI64::try_from(id).map(Self::from)
    }
}
impl<T: ?Sized> Default for Id<T> {
    fn default() -> Self {
        Self(NonZeroI64::new(1).unwrap(), PhantomData)
    }
}
impl<T: ?Sized> ToSql for Id<T> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}
impl<T: ?Sized> FromSql for Id<T> {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        NonZeroI64::column_result(value).map(Self::from)
    }
}
