//! rkyv `ArchiveWith` wrappers for types that don't natively support rkyv.
//!
//! - [`AsJson`] serializes a `serde_json::Value` (and collections thereof) as a
//!   JSON-encoded `String` in the archive.
//! - [`AsPath`] serializes a `std::path::PathBuf` as a UTF-8 `String`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use rkyv::{
    Archive, Archived, Place, Resolver,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
};

// ── AsPath ────────────────────────────────────────────────────────────────────

/// Wrapper that archives a `PathBuf` as a UTF-8 `String`.
///
/// # Limitations
///
/// Non-UTF-8 path components are silently replaced with `U+FFFD` via
/// `to_string_lossy`. A round-trip through this wrapper will produce a
/// different (non-existent) path on such systems, causing a cache miss.
/// Template files are developer-named HCL files and are virtually always
/// UTF-8, so this is acceptable in practice.
pub struct AsPath;

impl ArchiveWith<PathBuf> for AsPath {
    type Archived = Archived<String>;
    type Resolver = Resolver<String>;

    fn resolve_with(field: &PathBuf, resolver: Self::Resolver, out: Place<Self::Archived>) {
        let s = field.to_string_lossy().into_owned();
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized> SerializeWith<PathBuf, S> for AsPath
where
    S::Error: Source,
{
    fn serialize_with(field: &PathBuf, s: &mut S) -> Result<Self::Resolver, S::Error> {
        let path_str = field.to_string_lossy().into_owned();
        rkyv::Serialize::serialize(&path_str, s)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<String>, PathBuf, D> for AsPath {
    fn deserialize_with(field: &Archived<String>, _d: &mut D) -> Result<PathBuf, D::Error> {
        Ok(PathBuf::from(field.as_str()))
    }
}

// ── AsJson ────────────────────────────────────────────────────────────────────

/// Wrapper that archives a value by JSON-encoding it into a `String`.
///
/// Use this with `#[rkyv(with = AsJson)]` on fields whose types contain
/// `serde_json::Value` (which does not implement rkyv's `Archive` trait).
///
/// Supported field types:
/// - `serde_json::Value`
/// - `BTreeMap<String, serde_json::Value>`
/// - `Vec<serde_json::Value>`
/// - `Option<serde_json::Value>`
/// - `Option<BTreeMap<String, serde_json::Value>>`
/// - `Option<Vec<serde_json::Value>>`
/// - `Vec<(PathBuf, u64)>` (used for cache fingerprints)
///
/// # Limitations
///
/// The `Vec<(PathBuf, u64)>` impl uses `to_string_lossy` for path conversion.
/// Non-UTF-8 path components are silently replaced with `U+FFFD`, causing
/// fingerprint mismatches and perpetual cache misses on such paths.
pub struct AsJson;

// ── serde_json::Value ─────────────────────────────────────────────────────────

impl ArchiveWith<serde_json::Value> for AsJson {
    type Archived = Archived<String>;
    type Resolver = Resolver<String>;

    fn resolve_with(
        field: &serde_json::Value,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let s = serde_json::to_string(field).unwrap_or_default();
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized> SerializeWith<serde_json::Value, S> for AsJson
where
    S::Error: Source,
{
    fn serialize_with(field: &serde_json::Value, s: &mut S) -> Result<Self::Resolver, S::Error> {
        let json = serde_json::to_string(field).unwrap_or_default();
        rkyv::Serialize::serialize(&json, s)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<String>, serde_json::Value, D> for AsJson {
    fn deserialize_with(
        field: &Archived<String>,
        _d: &mut D,
    ) -> Result<serde_json::Value, D::Error> {
        Ok(serde_json::from_str(field.as_str()).unwrap_or(serde_json::Value::Null))
    }
}

// ── BTreeMap<String, serde_json::Value> ──────────────────────────────────────

impl ArchiveWith<BTreeMap<String, serde_json::Value>> for AsJson {
    type Archived = Archived<String>;
    type Resolver = Resolver<String>;

    fn resolve_with(
        field: &BTreeMap<String, serde_json::Value>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let s = serde_json::to_string(field).unwrap_or_default();
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized>
    SerializeWith<BTreeMap<String, serde_json::Value>, S> for AsJson
where
    S::Error: Source,
{
    fn serialize_with(
        field: &BTreeMap<String, serde_json::Value>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let json = serde_json::to_string(field).unwrap_or_default();
        rkyv::Serialize::serialize(&json, s)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<String>, BTreeMap<String, serde_json::Value>, D>
    for AsJson
{
    fn deserialize_with(
        field: &Archived<String>,
        _d: &mut D,
    ) -> Result<BTreeMap<String, serde_json::Value>, D::Error> {
        Ok(serde_json::from_str(field.as_str()).unwrap_or_default())
    }
}

// ── Vec<serde_json::Value> ────────────────────────────────────────────────────

impl ArchiveWith<Vec<serde_json::Value>> for AsJson {
    type Archived = Archived<String>;
    type Resolver = Resolver<String>;

    fn resolve_with(
        field: &Vec<serde_json::Value>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let s = serde_json::to_string(field).unwrap_or_default();
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized> SerializeWith<Vec<serde_json::Value>, S> for AsJson
where
    S::Error: Source,
{
    fn serialize_with(
        field: &Vec<serde_json::Value>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let json = serde_json::to_string(field).unwrap_or_default();
        rkyv::Serialize::serialize(&json, s)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<String>, Vec<serde_json::Value>, D> for AsJson {
    fn deserialize_with(
        field: &Archived<String>,
        _d: &mut D,
    ) -> Result<Vec<serde_json::Value>, D::Error> {
        Ok(serde_json::from_str(field.as_str()).unwrap_or_default())
    }
}

// ── Option<serde_json::Value> ─────────────────────────────────────────────────

impl ArchiveWith<Option<serde_json::Value>> for AsJson {
    type Archived = Archived<Option<String>>;
    type Resolver = Resolver<Option<String>>;

    fn resolve_with(
        field: &Option<serde_json::Value>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let s: Option<String> = field
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized> SerializeWith<Option<serde_json::Value>, S>
    for AsJson
where
    S::Error: Source,
{
    fn serialize_with(
        field: &Option<serde_json::Value>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let opt: Option<String> = field
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        rkyv::Serialize::serialize(&opt, s)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<Option<String>>, Option<serde_json::Value>, D>
    for AsJson
{
    fn deserialize_with(
        field: &Archived<Option<String>>,
        _d: &mut D,
    ) -> Result<Option<serde_json::Value>, D::Error> {
        match field.as_ref() {
            None => Ok(None),
            Some(s) => Ok(Some(
                serde_json::from_str(s.as_str()).unwrap_or(serde_json::Value::Null),
            )),
        }
    }
}

// ── Option<BTreeMap<String, serde_json::Value>> ───────────────────────────────

impl ArchiveWith<Option<BTreeMap<String, serde_json::Value>>> for AsJson {
    type Archived = Archived<Option<String>>;
    type Resolver = Resolver<Option<String>>;

    fn resolve_with(
        field: &Option<BTreeMap<String, serde_json::Value>>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let s: Option<String> = field
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized>
    SerializeWith<Option<BTreeMap<String, serde_json::Value>>, S> for AsJson
where
    S::Error: Source,
{
    fn serialize_with(
        field: &Option<BTreeMap<String, serde_json::Value>>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let opt: Option<String> = field
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());
        rkyv::Serialize::serialize(&opt, s)
    }
}

impl<D: Fallible + ?Sized>
    DeserializeWith<Archived<Option<String>>, Option<BTreeMap<String, serde_json::Value>>, D>
    for AsJson
{
    fn deserialize_with(
        field: &Archived<Option<String>>,
        _d: &mut D,
    ) -> Result<Option<BTreeMap<String, serde_json::Value>>, D::Error> {
        match field.as_ref() {
            None => Ok(None),
            Some(s) => Ok(Some(serde_json::from_str(s.as_str()).unwrap_or_default())),
        }
    }
}

// ── Option<Vec<serde_json::Value>> ────────────────────────────────────────────

impl ArchiveWith<Option<Vec<serde_json::Value>>> for AsJson {
    type Archived = Archived<Option<String>>;
    type Resolver = Resolver<Option<String>>;

    fn resolve_with(
        field: &Option<Vec<serde_json::Value>>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let s: Option<String> = field
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized> SerializeWith<Option<Vec<serde_json::Value>>, S>
    for AsJson
where
    S::Error: Source,
{
    fn serialize_with(
        field: &Option<Vec<serde_json::Value>>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let opt: Option<String> = field
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        rkyv::Serialize::serialize(&opt, s)
    }
}

impl<D: Fallible + ?Sized>
    DeserializeWith<Archived<Option<String>>, Option<Vec<serde_json::Value>>, D> for AsJson
{
    fn deserialize_with(
        field: &Archived<Option<String>>,
        _d: &mut D,
    ) -> Result<Option<Vec<serde_json::Value>>, D::Error> {
        match field.as_ref() {
            None => Ok(None),
            Some(s) => Ok(Some(serde_json::from_str(s.as_str()).unwrap_or_default())),
        }
    }
}

// ── Vec<(PathBuf, u64)> ───────────────────────────────────────────────────────
//
// Used for the cache fingerprint list. Archived as a JSON string since
// (PathBuf, u64) tuples are not Archive-able without wrapper indirection.

impl ArchiveWith<Vec<(PathBuf, u64)>> for AsJson {
    type Archived = Archived<String>;
    type Resolver = Resolver<String>;

    fn resolve_with(
        field: &Vec<(PathBuf, u64)>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let pairs: Vec<(String, u64)> = field
            .iter()
            .map(|(p, t)| (p.to_string_lossy().into_owned(), *t))
            .collect();
        let s = serde_json::to_string(&pairs).unwrap_or_default();
        Archive::resolve(&s, resolver, out);
    }
}

impl<S: Fallible + Writer + Allocator + ?Sized> SerializeWith<Vec<(PathBuf, u64)>, S> for AsJson
where
    S::Error: Source,
{
    fn serialize_with(field: &Vec<(PathBuf, u64)>, s: &mut S) -> Result<Self::Resolver, S::Error> {
        let pairs: Vec<(String, u64)> = field
            .iter()
            .map(|(p, t)| (p.to_string_lossy().into_owned(), *t))
            .collect();
        let json = serde_json::to_string(&pairs).unwrap_or_default();
        rkyv::Serialize::serialize(&json, s)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<String>, Vec<(PathBuf, u64)>, D> for AsJson {
    fn deserialize_with(
        field: &Archived<String>,
        _d: &mut D,
    ) -> Result<Vec<(PathBuf, u64)>, D::Error> {
        let pairs: Vec<(String, u64)> = serde_json::from_str(field.as_str()).unwrap_or_default();
        Ok(pairs
            .into_iter()
            .map(|(s, t)| (PathBuf::from(s), t))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use rkyv::rancor::Error as RkyvError;
    use serde_json::{Value, json};

    use super::*;

    /// Helper: roundtrip a wrapper struct through rkyv serialize → deserialize.
    macro_rules! roundtrip {
        ($wrapper:ty, $field_ty:ty, $value:expr) => {{
            #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
            struct Wrapper {
                #[rkyv(with = $wrapper)]
                field: $field_ty,
            }
            let original = Wrapper { field: $value };
            let bytes = rkyv::to_bytes::<RkyvError>(&original).expect("serialize");
            let decoded: Wrapper =
                rkyv::from_bytes::<Wrapper, RkyvError>(&bytes).expect("deserialize");
            decoded.field
        }};
    }

    #[test]
    fn as_path_roundtrips_utf8_path() {
        let path = PathBuf::from("/home/user/templates/github.hcl");
        let decoded = roundtrip!(AsPath, PathBuf, path.clone());
        assert_eq!(decoded, path);
    }

    #[test]
    fn as_json_value_roundtrips_object() {
        let v: Value = json!({"key": "value", "num": 42});
        let decoded = roundtrip!(AsJson, Value, v.clone());
        assert_eq!(decoded, v);
    }

    #[test]
    fn as_json_value_roundtrips_null() {
        let decoded = roundtrip!(AsJson, Value, Value::Null);
        assert_eq!(decoded, Value::Null);
    }

    #[test]
    fn as_json_btreemap_roundtrips() {
        let mut m = BTreeMap::new();
        m.insert("x".to_string(), json!(1));
        m.insert("y".to_string(), json!("hello"));
        let decoded = roundtrip!(AsJson, BTreeMap<String, Value>, m.clone());
        assert_eq!(decoded, m);
    }

    #[test]
    fn as_json_vec_value_roundtrips() {
        let v = vec![json!(1), json!("two"), json!(null)];
        let decoded = roundtrip!(AsJson, Vec<Value>, v.clone());
        assert_eq!(decoded, v);
    }

    #[test]
    fn as_json_option_value_some_roundtrips() {
        let v: Option<Value> = Some(json!({"a": true}));
        let decoded = roundtrip!(AsJson, Option<Value>, v.clone());
        assert_eq!(decoded, v);
    }

    #[test]
    fn as_json_option_value_none_roundtrips() {
        let v: Option<Value> = None;
        let decoded = roundtrip!(AsJson, Option<Value>, v);
        assert_eq!(decoded, None);
    }

    #[test]
    fn as_json_option_btreemap_some_roundtrips() {
        let mut m = BTreeMap::new();
        m.insert("k".to_string(), json!(99));
        let v: Option<BTreeMap<String, Value>> = Some(m.clone());
        let decoded = roundtrip!(AsJson, Option<BTreeMap<String, Value>>, v);
        assert_eq!(decoded, Some(m));
    }

    #[test]
    fn as_json_option_btreemap_none_roundtrips() {
        let v: Option<BTreeMap<String, Value>> = None;
        let decoded = roundtrip!(AsJson, Option<BTreeMap<String, Value>>, v);
        assert_eq!(decoded, None);
    }

    #[test]
    fn as_json_option_vec_value_some_roundtrips() {
        let v: Option<Vec<Value>> = Some(vec![json!(1), json!(2)]);
        let decoded = roundtrip!(AsJson, Option<Vec<Value>>, v.clone());
        assert_eq!(decoded, v);
    }

    #[test]
    fn as_json_option_vec_value_none_roundtrips() {
        let v: Option<Vec<Value>> = None;
        let decoded = roundtrip!(AsJson, Option<Vec<Value>>, v);
        assert_eq!(decoded, None);
    }

    #[test]
    fn as_json_fingerprint_roundtrips() {
        let fp = vec![
            (PathBuf::from("/tmp/a.hcl"), 1_700_000_000u64),
            (PathBuf::from("/home/user/b.hcl"), 999u64),
        ];
        let decoded = roundtrip!(AsJson, Vec<(PathBuf, u64)>, fp.clone());
        assert_eq!(decoded, fp);
    }
}
