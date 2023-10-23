mod schedule;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread::sleep,
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

    match &state_file {
        Some(s) => println!("Using {} to store state", s.display()),
        None => eprintln!("Warning: couldn't find a writable directory, state will not be saved."),
    };

    let mut last_run = state_file.as_ref().and_then(|p| read_last_run(p, &tz));

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
            sleep(sleep_duration.to_std().unwrap());
        }

        run_command(&program, &args)?;

        let now = Utc::now().with_timezone(&tz);
        last_run = Some(now);

        if let Some(f) = &state_file {
            write_last_run(f, &now);
        }
    }
}

fn run_command(program: impl AsRef<Path>, args: &[String]) -> anyhow::Result<()> {
    println!("Running command");
    let status = Command::new(program.as_ref())
        .args(args)
        .status()
        .context("failed to execute command")?;

    if !status.success() {
        eprintln!("Command failed with status: {}", status);
    }

    Ok(())
}

fn read_last_run(file: impl AsRef<Path>, tz: &Tz) -> Option<DateTime<Tz>> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .map(|t| t.with_timezone(tz))
}

fn write_last_run(file: impl AsRef<Path>, now: &DateTime<Tz>) {
    std::fs::write(file, now.to_rfc3339()).ok();
}
