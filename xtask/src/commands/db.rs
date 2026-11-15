// =============================================================================
// DATABASE SCHEMA & SEED COMMAND (`cargo xtask db`)
// =============================================================================
//
// Applies SQL schemas (`database/schema/*.sql`), triggers
// (`database/triggers/*.sql`), and idempotent seeds (`database/seed/*.sql`)
// deterministically using SeaORM's pure-Rust PostgreSQL driver (zero local
// `psql` binary required).
//
// ## Key Concepts
//
// - Sorts `.sql` files lexicographically by filename (`01_users.sql`, ...).
// - Accepts `--database-url` override or reads `APP__DATABASE__URL` from `.env`.
//
// ## Architecture Notes
//
// Purely non-interactive; never prompts for confirmation or DB passwords.

// SECTION: IMPORTS

// `anyhow` - Descriptive errors when database connections or SQL files fail.
use anyhow::{Context, Result};

// `clap` - CLI argument and subcommand definitions for `db`.
use clap::{Args, Subcommand};

// `sea_orm` - Pure-Rust async PostgreSQL client for executing raw DDL/trigger/seed SQL.
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};

// `std` - Directory traversal, sorting, and filesystem reads.
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{print_step, resolve_database_url};

// SECTION: CLI ARGUMENTS

/// Arguments for `cargo xtask db`.
#[derive(Debug, Args)]
pub struct DbArgs {
    #[command(subcommand)]
    pub action: DbAction,
}

/// Supported database subcommands.
#[derive(Debug, Subcommand)]
pub enum DbAction {
    /// Apply `database/schema/*.sql`, `database/triggers/*.sql`, and `database/seed/*.sql`.
    Schema {
        /// Optional PostgreSQL URI override (defaults to `APP__DATABASE__URL`).
        #[arg(long)]
        database_url: Option<String>,

        /// Skip applying `database/seed/*.sql` after schemas and triggers.
        #[arg(long)]
        skip_seed: bool,
    },

    /// Apply only `database/seed/*.sql`.
    Seed {
        /// Optional PostgreSQL URI override (defaults to `APP__DATABASE__URL`).
        #[arg(long)]
        database_url: Option<String>,
    },
}

// SECTION: EXECUTION

/// Executes database schema, trigger, or seed SQL files via SeaORM PostgreSQL.
pub fn run_db(root: &Path, args: DbArgs) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed initializing Tokio runtime for xtask db")?;

    runtime.block_on(run_db_async(root, args))
}

async fn run_db_async(root: &Path, args: DbArgs) -> Result<()> {
    match args.action {
        DbAction::Schema {
            database_url,
            skip_seed,
        } => {
            let db_url = resolve_database_url(database_url)?;
            let conn = connect_postgres(&db_url).await?;

            print_step("Applying database schemas (`database/schema/*.sql`)");
            apply_sql_dir(&conn, &root.join("database/schema")).await?;

            print_step("Applying database triggers (`database/triggers/*.sql`)");
            apply_sql_dir(&conn, &root.join("database/triggers")).await?;

            if !skip_seed {
                print_step("Applying database seed scripts (`database/seed/*.sql`)");
                apply_sql_dir(&conn, &root.join("database/seed")).await?;
            }

            println!("\n[xtask] Database schema setup completed successfully.");
            Ok(())
        }
        DbAction::Seed { database_url } => {
            let db_url = resolve_database_url(database_url)?;
            let conn = connect_postgres(&db_url).await?;

            print_step("Applying database seed scripts (`database/seed/*.sql`)");
            apply_sql_dir(&conn, &root.join("database/seed")).await?;

            println!("\n[xtask] Database seed completed successfully.");
            Ok(())
        }
    }
}

async fn connect_postgres(db_url: &str) -> Result<DatabaseConnection> {
    let mut opts = ConnectOptions::new(db_url);
    opts.max_connections(2)
        .connect_timeout(Duration::from_secs(10));

    Database::connect(opts)
        .await
        .context("Failed connecting to PostgreSQL database")
}

/// Collects and executes all `.sql` files in `dir` sorted alphabetically.
async fn apply_sql_dir(conn: &DatabaseConnection, dir: &Path) -> Result<()> {
    if !dir.exists() {
        println!("[xtask] Directory {} does not exist (skipping).", dir.display());
        return Ok(());
    }

    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read SQL directory {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("sql"))
        .collect();

    files.sort();

    if files.is_empty() {
        println!("[xtask] No `.sql` files in {} (skipping).", dir.display());
        return Ok(());
    }

    for file in files {
        let sql = std::fs::read_to_string(&file)
            .with_context(|| format!("Failed reading SQL file {}", file.display()))?;
        conn.execute_unprepared(&sql)
            .await
            .with_context(|| format!("SQL execution failed in {}", file.display()))?;
        println!("[xtask] Applied {}", file.file_name().unwrap_or_default().to_string_lossy());
    }

    Ok(())
}
