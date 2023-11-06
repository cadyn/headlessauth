use std::collections::HashMap;
use std::path::PathBuf;

use directories::ProjectDirs;

use fs4::tokio::AsyncFileExt;

use tokio::io::{AsyncWriteExt, BufReader, AsyncReadExt};
use tokio::fs::{OpenOptions,File};

use serde::{Serialize, Deserialize};

use chrono::{DateTime, Utc, Datelike};

#[derive(poise::ChoiceParameter, Clone, Copy)]
pub enum RepeatType {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

#[derive(Serialize, Deserialize, Debug,poise::ChoiceParameter, Clone, Copy)]
pub enum ClosedStatus {
    Automatic,
    Open,
    Closed,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClosedData {
    pub is_closed: ClosedStatus,
    pub close_events: HashMap<usize,RepeatingEvent>,
    pub open_events: HashMap<usize,RepeatingEvent>,
}

impl Default for ClosedData {
    fn default() -> Self {ClosedData{is_closed: ClosedStatus::Automatic, close_events: HashMap::new(), open_events: HashMap::new()}}
}

impl ClosedData {
    pub fn is_currently_closed(&self) -> bool {
        match self.is_closed {
            ClosedStatus::Open => return false,
            ClosedStatus::Closed => return true,
            ClosedStatus::Automatic => (),
        }

        if self.close_events.is_empty() || self.open_events.is_empty() {return false;}

        let mut close_elapsed = i64::MAX;
        for event in self.close_events.values() {
            let elapsed = event.elapsed();
            if close_elapsed > elapsed {
                close_elapsed = elapsed;
            }
        }

        let mut open_elapsed = i64::MAX;
        for event in self.open_events.values() {
            let elapsed = event.elapsed();
            if open_elapsed > elapsed {
                open_elapsed = elapsed;
            }
        }

        return close_elapsed < open_elapsed;
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RepeatingEvent {
    pub id: usize,
    pub initial: i64,
    pub repeating: RepeatingDate,
}

impl RepeatingEvent {
    pub fn nth(&self, n: i64) -> i64 {
        match self.repeating {
            RepeatingDate::Seconds(x) => self.initial + (x * n),
            RepeatingDate::Minutes(x) => self.initial + (x * 60 * n),
            RepeatingDate::Hours(x) => self.initial + (x * 3600 * n),
            RepeatingDate::Days(x) => self.initial + (x * 86400 * n),
            RepeatingDate::Weeks(x) => self.initial + (x * 604800 * n),
            RepeatingDate::Months(x) => {
                let initial_dt: DateTime<Utc> = DateTime::from_timestamp(self.initial, 0).unwrap();
                let pre_month = initial_dt.month() as i64 + (x * n);
                let year = (initial_dt.year() as i64 + (pre_month / 12)) as i32;
                let month = (pre_month % 12) as u32;
                let new_dt = initial_dt.with_month(month).unwrap().with_year(year).unwrap();
                return new_dt.timestamp();
            },
            RepeatingDate::Years(x) => {
                let initial_dt: DateTime<Utc> = DateTime::from_timestamp(self.initial, 0).unwrap();
                let year = (initial_dt.year() as i64 + (x * n)) as i32;
                let new_dt = initial_dt.with_year(year).unwrap();
                return new_dt.timestamp();
            }
        }
    }
    pub fn most_recent(&self) -> i64 {
        let now: i64 = Utc::now().timestamp();
        //let most_recent: DateTime<Utc> = DateTime::from_timestamp(self.initial, 0).unwrap();
        let mut i = 0;
        while self.nth(i+1) < now {
            i+=1;
        }
        let most_recent: DateTime<Utc> = DateTime::from_timestamp(self.nth(i),0).unwrap();
        return most_recent.timestamp();
    }
    pub fn elapsed(&self) -> i64 {
        let now: i64 = Utc::now().timestamp();
        return now - self.most_recent();
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RepeatingDate {
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
    Weeks(i64),
    Months(i64),
    Years(i64),
}

impl RepeatingDate {
    pub fn from_input(t: RepeatType, num: i64) -> Self {
        match t {
            RepeatType::Seconds => RepeatingDate::Seconds(num),
            RepeatType::Minutes => RepeatingDate::Minutes(num),
            RepeatType::Hours => RepeatingDate::Hours(num),
            RepeatType::Days => RepeatingDate::Days(num),
            RepeatType::Weeks => RepeatingDate::Weeks(num),
            RepeatType::Months => RepeatingDate::Months(num),
            RepeatType::Years => RepeatingDate::Years(num),
        }
    }
    pub fn decompose(&self) -> (RepeatType, i64) {
        match self {
            RepeatingDate::Seconds(num) => (RepeatType::Seconds, *num),
            RepeatingDate::Minutes(num) => (RepeatType::Minutes, *num),
            RepeatingDate::Hours(num) => (RepeatType::Hours, *num),
            RepeatingDate::Days(num) => (RepeatType::Days, *num),
            RepeatingDate::Weeks(num) => (RepeatType::Weeks, *num),
            RepeatingDate::Months(num) => (RepeatType::Months, *num),
            RepeatingDate::Years(num) => (RepeatType::Years, *num),
        }
    }
}

pub struct Data {} // User data, which is stored and accessible in all command invocations
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub fn get_dir() -> Result<PathBuf,Error> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "cadyn",  "headlessauth") {
        let dir = proj_dirs.data_dir();

        if !dir.exists() {
            let res = std::fs::create_dir_all(dir);
            match res {
                Err(e) => {
                    println!("create_dir:{e:?}");
                    return Err(Box::new(e));
                },
                Ok(_) => ()
            }
        }
        
        return Ok(dir.to_path_buf());
    }
    Err(Error::from("Unable to get project directories"))
}

pub async fn try_get_file(ctx: Option<&Context<'_>>, file_path: &PathBuf) -> Result<File, Error> {
    match OpenOptions::new().read(true).write(true).create(true).open(file_path).await {
        Err(e) => {
            println!("openoptions:{e:?}");
            if let Some(x) = ctx {
                x.say("Encountered error accessing files").await?;
            }
            Err(Box::new(e))
        },
        Ok(res) => Ok(res)
    }
}

pub async fn write_tmp_and_copy(ctx: &Context<'_>, file_path: &PathBuf, file:File, data: &str) -> Result<(), Error> {
    let tmp_file_path = file_path.with_extension("tmp");

    let mut tmp_file = match OpenOptions::new().read(true).write(true).create(true).truncate(true).open(&tmp_file_path).await {
        Err(e) => {
            println!("openoptions:{e:?}");
            ctx.say("Encountered error accessing files").await?;
            return Err(Box::new(e));
        },
        Ok(res) => res
    };
    
    tmp_file.lock_exclusive()?;
    tmp_file.write_all(data.as_bytes()).await?;
    tmp_file.flush().await?;
    tmp_file.unlock()?;
    file.unlock()?;
    tokio::fs::copy(&tmp_file_path, &file_path).await?;
    Ok(())
}

pub async fn load_json<T: Default + for<'a> Deserialize<'a>>(ctx: Option<&Context<'_>>, file_name: String, read_only: bool) -> Result<(PathBuf, File, T), Error>{
    let dir = get_dir()?;
    let file_path = dir.join(file_name);

    let mut file = try_get_file(ctx, &file_path).await?;

    if read_only {
        file.lock_shared()?;
    } else {
        file.lock_exclusive()?;
    }
    
    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await?;
    let data: T =  if data_string == "" {
        T::default()
    } else {
        serde_json::from_str(&data_string)?
    };
    return Ok((file_path,file,data));
}