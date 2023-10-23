mod schedule;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::path::{Path, PathBuf};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    select,
    signal::ctrl_c,
    time::sleep,
};

use anyhow::Context;
use clap::Parser;
use schedule::Schedule;

/// A simple app that runs the given command in the background on a cron like schedule.
/// Refer to https://en.wikipedia.org/wiki/Cron for more information on cron schedules.
#[derive(Parser)]
struct CliArgs {
    /// The schedule when the command should be run.
    #[clap(short, long)]
    schedule: Schedule,

    /// The file to store the last run time in.
    #[clap(long, env = "CRON_RUN_STATE_FILE")]
    state_file: Option<PathBuf>,

    /// Whether to run the command on first time.
    #[clap(short, long, default_value_t = true)]
    run_on_first: bool,

    /// The program to run.
    program: PathBuf,

    /// The args to run the program with.
    #[clap(default_value = "[]")]
    args: Vec<String>,

    /// The timezone to use when running the command.
    #[clap(env, long)]
    tz: Tz,
}

fn main() -> anyhow::Result<()> {
    let CliArgs {
        schedule,
        tz,
        program,
        args,
        run_on_first,
        state_file,
    } = CliArgs::parse();

    let state_file = state_file.or_else(|| dirs::cache_dir().map(|p| p.join("cron-run.state")));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("A tokio runtime");

    runtime.block_on(async move {
        select! {
            r = run_schedule(state_file, tz, run_on_first, schedule, program, args) => r,
            _ = ctrl_c() => {
                println!("Exiting");
                Ok(())
            },
        }
    })
}

async fn run_schedule(
    state_file: Option<PathBuf>,
    tz: Tz,
    run_on_first: bool,
    schedule: Schedule,
    program: impl AsRef<Path>,
    args: Vec<String>,
) -> anyhow::Result<()> {
    match &state_file {
        Some(s) => println!("Using {} to store state", s.display()),
        None => {
            eprintln!("Warning: couldn't find a writable directory, state will not be saved.")
        }
    };

    let mut last_run = if let Some(f) = &state_file {
        read_last_run(f, &tz).await
    } else {
        None
    };

    loop {
        let now = Utc::now().with_timezone(&tz);

        let (should_sleep, next_run) = match (last_run, run_on_first) {
            (None, true) => (false, now),
            (last, _) => (
                true,
                schedule
                    .after(&last.unwrap_or(now))
                    .context("failed to get next run")?,
            ),
        };

        if should_sleep && next_run > now {
            let sleep_duration = next_run - now;
            println!("Next run at {next_run}");
            sleep(sleep_duration.to_std().unwrap()).await;
        }

        run_command(&program, &args).await?;

        let now = Utc::now().with_timezone(&tz);
        last_run = Some(now);

        if let Some(f) = &state_file {
            let _ = write_last_run(f, &now).await;
        }
    }
}

async fn run_command(program: impl AsRef<Path>, args: &[String]) -> anyhow::Result<()> {
    println!("Running command");
    let mut child = Command::new(program.as_ref())
        .args(args)
        .kill_on_drop(true)
        .spawn()
        .context("failed to execute command")?;

    let status = child.wait().await?;

    if !status.success() {
        eprintln!("Command failed with status: {}", status);
    }

    Ok(())
}

async fn read_last_run(file: impl AsRef<Path>, tz: &Tz) -> Option<DateTime<Tz>> {
    let mut buf = BufReader::new(File::open(file).await.ok()?);

    let mut text = Default::default();
    buf.read_to_string(&mut text).await.ok()?;

    Some(DateTime::parse_from_rfc3339(&text).ok()?.with_timezone(tz))
}

async fn write_last_run(file: impl AsRef<Path>, now: &DateTime<Tz>) -> anyhow::Result<()> {
    let mut file = File::create(file).await.context("Writing last run time")?;
    file.write_all(&now.to_rfc3339().as_bytes())
        .await
        .context("Writing last run time")
}
