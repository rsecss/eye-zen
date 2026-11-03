use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono_tz::Tz;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::error::{AppError, Result};

use super::SCHEMA_VERSION;

static EXPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn temporary_export_path(target: &Path) -> Result<PathBuf> {
    let parent = target.parent().ok_or_else(|| AppError::ConfigInvalid {
        field: "target_path".to_string(),
        reason: format!("must have a parent directory, got {}", target.display()),
    })?;
    let file_name = target.file_name().ok_or_else(|| AppError::ConfigInvalid {
        field: "target_path".to_string(),
        reason: format!("must have a file name, got {}", target.display()),
    })?;
    let sequence = EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut temporary_name = OsString::from(".");
    temporary_name.push(file_name);
    temporary_name.push(format!(
        ".eyezen-export-{}-{sequence}.tmp",
        std::process::id()
    ));
    Ok(parent.join(temporary_name))
}

pub(super) async fn validate_exported_database(path: &Path) -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(path).read_only(true))
        .await?;
    let validation = async {
        let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await?;
        if user_version != SCHEMA_VERSION {
            return Err(AppError::IoError {
                message: format!(
                    "exported database schema version {user_version} does not match {SCHEMA_VERSION}"
                ),
            });
        }

        let quick_check: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&pool)
            .await?;
        if quick_check != "ok" {
            return Err(AppError::IoError {
                message: format!("exported database quick_check failed: {quick_check}"),
            });
        }
        Ok(())
    }
    .await;
    pool.close().await;
    validation
}

pub(super) async fn sync_export_file(path: &Path) -> Result<()> {
    tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .await?
        .sync_all()
        .await?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(super) async fn replace_file(temporary: PathBuf, target: PathBuf) -> Result<()> {
    tokio::fs::rename(temporary, target).await?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub(super) async fn replace_file(temporary: PathBuf, target: PathBuf) -> Result<()> {
    tokio::task::spawn_blocking(move || replace_file_windows(&temporary, &target))
        .await
        .map_err(|err| AppError::IoError {
            message: format!("atomic file replace task failed: {err}"),
        })?
}

#[cfg(target_os = "windows")]
fn replace_file_windows(temporary: &Path, target: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let temporary_wide: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let target_wide: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();

    // The buffers are NUL-terminated and remain alive for the duration of the
    // Win32 call. Both paths share a directory, so no copy fallback is used.
    unsafe {
        MoveFileExW(
            PCWSTR(temporary_wide.as_ptr()),
            PCWSTR(target_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|err| AppError::IoError {
        message: format!("atomic file replace failed: {err}"),
    })
}

/// Boundary validation for `export_statistics`. Reject anything that is not
/// a normal sqlite-shaped path the user could plausibly select via the OS
/// save dialog:
///
/// - MUST be absolute. Relative paths would resolve against the working
///   directory of the helper process and could escape into the app data
///   directory.
/// - MUST NOT contain a `..` component. `SQLite`'s `VACUUM INTO` will follow
///   it, and the save dialog has no business producing one.
/// - MUST have a `.db` or `.sqlite` extension (case-insensitive). Anything
///   else (`.log`, `.toml`, no extension) is a sign of misuse and would
///   confuse the file as a "valid sqlite backup".
/// - MUST NOT point at the live source database. Without this check
///   `VACUUM INTO` would refuse with an opaque `SQLite` error after
///   `remove_file` already destroyed it; far worse, when the source path
///   has been canonicalized differently the destructive `remove_file`
///   could still hit a symlink to the live DB.
/// - The parent directory MUST exist. `tokio::fs::create_dir_all` is
///   allowed to create the final leaf, but we do not silently create
///   arbitrary deep ancestors on behalf of the user.
pub(super) fn validate_export_path(
    target: &std::path::Path,
    source_db: &std::path::Path,
) -> Result<()> {
    if !target.is_absolute() {
        return Err(AppError::ConfigInvalid {
            field: "target_path".to_string(),
            reason: format!("must be an absolute path, got {}", target.display()),
        });
    }

    if target
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(AppError::ConfigInvalid {
            field: "target_path".to_string(),
            reason: format!("must not contain `..` segments, got {}", target.display()),
        });
    }

    let extension_ok = target
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            let lower = ext.to_ascii_lowercase();
            lower == "db" || lower == "sqlite"
        });
    if !extension_ok {
        return Err(AppError::ConfigInvalid {
            field: "target_path".to_string(),
            reason: format!("extension must be .db or .sqlite, got {}", target.display()),
        });
    }

    let Some(parent) = target.parent() else {
        return Err(AppError::ConfigInvalid {
            field: "target_path".to_string(),
            reason: format!("must have a parent directory, got {}", target.display()),
        });
    };
    if !parent.as_os_str().is_empty() && !parent.is_dir() {
        return Err(AppError::ConfigInvalid {
            field: "target_path".to_string(),
            reason: format!("parent directory does not exist: {}", parent.display()),
        });
    }

    // Same-file check: prefer canonicalize (resolves symlinks, normalizes
    // case on Windows) but fall back to a lexical compare so a non-yet-
    // existing target still gets the basic equality guard.
    let same_file = match (target.canonicalize(), source_db.canonicalize()) {
        (Ok(t), Ok(s)) => t == s,
        _ => target == source_db,
    };
    if same_file {
        return Err(AppError::ConfigInvalid {
            field: "target_path".to_string(),
            reason: "must not overwrite the live statistics database".to_string(),
        });
    }

    Ok(())
}

pub(super) fn resolve_timezone(timezone: Option<&str>) -> Result<Tz> {
    let requested = timezone
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| iana_time_zone::get_timezone().ok())
        .unwrap_or_else(|| "UTC".to_string());

    requested
        .parse::<Tz>()
        .map_err(|err| AppError::ConfigInvalid {
            field: "timezone".to_string(),
            reason: format!("invalid IANA timezone \"{requested}\": {err}"),
        })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::tempdir;

    use crate::error::AppError;
    use crate::services::Service;
    use crate::services::ServiceContext;

    use super::super::writer::session_at;
    use super::super::{StatService, SCHEMA_VERSION};
    use super::*;

    #[tokio::test]
    async fn export_to_writes_a_valid_sqlite_file_preserving_rows() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        for offset in 0..3 {
            service
                .record_rest_session(session_at(
                    Utc.with_ymd_and_hms(2026, 5, 20, 8, 0, 0)
                        .single()
                        .expect("valid UTC datetime")
                        + chrono::Duration::minutes(offset * 20),
                    20,
                ))
                .await
                .expect("record should persist");
        }

        let target = dir.path().join("backup.db");
        service
            .export_to(target.clone())
            .await
            .expect("export should succeed");
        assert!(target.exists(), "backup file should be created");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&target))
            .await
            .expect("backup should open as sqlite");
        let rows: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM activity_segments WHERE state = 'resting'")
                .fetch_one(&pool)
                .await
                .expect("backup should expose activity_segments");
        assert_eq!(rows, 3);

        let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(user_version, SCHEMA_VERSION);
        pool.close().await;
    }

    #[tokio::test]
    async fn export_to_succeeds_on_empty_database() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        let target = dir.path().join("empty-backup.db");
        service
            .export_to(target.clone())
            .await
            .expect("empty export should succeed");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&target))
            .await
            .expect("empty backup should open as sqlite");
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_segments")
            .fetch_one(&pool)
            .await
            .expect("schema should still exist");
        assert_eq!(rows, 0);
        pool.close().await;
    }

    #[tokio::test]
    async fn export_to_overwrites_existing_target_file() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        let target = dir.path().join("backup.db");
        std::fs::write(&target, b"old contents that VACUUM INTO would reject")
            .expect("seed file should write");
        assert!(target.exists());

        service
            .export_to(target.clone())
            .await
            .expect("export should overwrite existing target");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(SqliteConnectOptions::new().filename(&target))
            .await
            .expect("overwritten file should be a valid sqlite db");
        let user_version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .expect("user_version should query");
        assert_eq!(user_version, SCHEMA_VERSION);
        pool.close().await;
    }

    #[tokio::test]
    async fn export_replace_failure_preserves_existing_target_and_cleans_temp() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        let target = dir.path().join("backup.db");
        let old_bytes = b"existing backup must survive";
        std::fs::write(&target, old_bytes).expect("seed backup should write");

        let err = service
            .export_to_with(target.clone(), |_temporary, _target| async {
                Err(AppError::IoError {
                    message: "injected replace failure".to_string(),
                })
            })
            .await
            .expect_err("injected replace failure should surface");

        assert!(matches!(
            err,
            AppError::IoError { ref message } if message.contains("injected replace failure")
        ));
        assert_eq!(
            std::fs::read(&target).expect("old backup should remain readable"),
            old_bytes
        );
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("tempdir should be readable")
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().contains("eyezen-export"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary exports remain: {leftovers:?}"
        );
    }

    #[tokio::test]
    async fn exported_database_validation_rejects_corrupt_file() {
        let dir = tempdir().expect("tempdir should be created");
        let corrupt = dir.path().join("corrupt.db");
        std::fs::write(&corrupt, b"not a sqlite database").expect("corrupt file should write");

        let err = validate_exported_database(&corrupt)
            .await
            .expect_err("corrupt database must be rejected");

        assert!(matches!(err, AppError::IoError { .. }));
    }

    #[tokio::test]
    async fn concurrent_exports_both_produce_valid_databases() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        let first = dir.path().join("first.db");
        let second = dir.path().join("second.db");
        let (first_outcome, second_outcome) = tokio::join!(
            service.export_to(first.clone()),
            service.export_to(second.clone())
        );

        first_outcome.expect("first export should succeed");
        second_outcome.expect("second export should succeed");
        validate_exported_database(&first)
            .await
            .expect("first export should validate");
        validate_exported_database(&second)
            .await
            .expect("second export should validate");
    }

    #[tokio::test]
    async fn export_to_returns_error_for_unwritable_target_directory() {
        let dir = tempdir().expect("tempdir should be created");
        let db_path = dir.path().join("data.db");
        let service = StatService::new(db_path);
        service
            .init(&ServiceContext::default())
            .await
            .expect("stat service should init");

        // The parent of this target is the source DB file itself, which is
        // not a directory; validate_export_path rejects it before touching
        // the filesystem.
        let bogus_target = service.db_path.join("nested").join("backup.db");
        let err = service
            .export_to(bogus_target)
            .await
            .expect_err("export to invalid path should fail");
        assert!(matches!(err, AppError::ConfigInvalid { .. }));
    }

    #[tokio::test]
    async fn export_path_rejects_relative_path() {
        let target = std::path::PathBuf::from("relative/backup.db");
        let source = std::path::PathBuf::from("/tmp/data.db");
        let Err(AppError::ConfigInvalid { field, reason }) = validate_export_path(&target, &source)
        else {
            panic!("relative path should be rejected");
        };
        assert_eq!(field, "target_path");
        assert!(reason.contains("absolute"), "reason was {reason}");
    }

    #[tokio::test]
    async fn export_path_rejects_traversal() {
        let dir = tempdir().expect("tempdir should be created");
        let target = dir.path().join("..").join("backup.db");
        let source = dir.path().join("data.db");
        let Err(AppError::ConfigInvalid { field, reason }) = validate_export_path(&target, &source)
        else {
            panic!("`..` traversal should be rejected");
        };
        assert_eq!(field, "target_path");
        assert!(reason.contains(".."), "reason was {reason}");
    }

    #[tokio::test]
    async fn export_path_rejects_non_db_extension() {
        let dir = tempdir().expect("tempdir should be created");
        let target = dir.path().join("backup.log");
        let source = dir.path().join("data.db");
        let Err(AppError::ConfigInvalid { field, reason }) = validate_export_path(&target, &source)
        else {
            panic!("non-.db extension should be rejected");
        };
        assert_eq!(field, "target_path");
        assert!(reason.contains(".db"), "reason was {reason}");
    }

    #[tokio::test]
    async fn export_path_rejects_missing_parent_directory() {
        let dir = tempdir().expect("tempdir should be created");
        let target = dir.path().join("missing").join("backup.db");
        let source = dir.path().join("data.db");
        let Err(AppError::ConfigInvalid { field, reason }) = validate_export_path(&target, &source)
        else {
            panic!("missing parent should be rejected");
        };
        assert_eq!(field, "target_path");
        assert!(reason.contains("parent directory"), "reason was {reason}");
    }

    #[tokio::test]
    async fn export_path_rejects_source_db_self() {
        let dir = tempdir().expect("tempdir should be created");
        let source = dir.path().join("data.db");
        // Create the source file so canonicalize sees it.
        std::fs::write(&source, b"sqlite3").expect("seed source db");
        let Err(AppError::ConfigInvalid { field, reason }) = validate_export_path(&source, &source)
        else {
            panic!("source-db-self should be rejected");
        };
        assert_eq!(field, "target_path");
        assert!(
            reason.contains("live statistics database"),
            "reason was {reason}"
        );
    }

    #[tokio::test]
    async fn export_path_accepts_valid_absolute_db() {
        let dir = tempdir().expect("tempdir should be created");
        let target = dir.path().join("backup.db");
        let source = dir.path().join("data.db");
        validate_export_path(&target, &source).expect("absolute .db in tempdir should pass");
    }

    #[tokio::test]
    async fn export_path_accepts_sqlite_extension_case_insensitive() {
        let dir = tempdir().expect("tempdir should be created");
        let target = dir.path().join("backup.SQLITE");
        let source = dir.path().join("data.db");
        validate_export_path(&target, &source).expect(".SQLITE should pass");
    }
}
