use std::str::FromStr;

use anyhow::Context;
use chrono::{DateTime, TimeZone, Utc};

#[derive(Clone)]
pub struct Schedule(String);

impl FromStr for Schedule {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let _ = cron_parser::parse(s, &Utc::now()).context("failed to parse schedule");
        Ok(Self(s.to_string()))
    }
}

impl Schedule {
    pub fn after<Tz: TimeZone>(&self, dt: &DateTime<Tz>) -> Option<DateTime<Tz>> {
        cron_parser::parse(&self.0, dt).ok()
    }
}
