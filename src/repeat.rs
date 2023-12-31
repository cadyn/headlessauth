use chrono::{DateTime, Utc, Datelike};
use serde::{Serialize, Deserialize};

#[derive(poise::ChoiceParameter, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum RepeatType {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

impl RepeatType {
    pub fn with_plurality(&self,n: i64) -> String {
        let singular = (n <= 1) as usize;
        let t_string = self.to_string();
        
        let type_s = &t_string[0..t_string.len()-singular];

        return type_s.to_string().to_lowercase();
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct RepeatingEvent {
    pub id: usize,
    pub initial: i64,
    pub repeating: RepeatInterval,
}

impl RepeatingEvent {
    pub fn nth(&self, n: i64) -> i64 {
        match self.repeating.t {
            RepeatType::Months => {
                let initial_dt: DateTime<Utc> = DateTime::from_timestamp(self.initial, 0).unwrap();
                let pre_month = initial_dt.month() as i64 + (self.repeating.n * n);
                let year = (initial_dt.year() as i64 + (pre_month / 12)) as i32;
                let month = (pre_month % 12) as u32;
                let new_dt = initial_dt.with_month(month).unwrap().with_year(year).unwrap();
                return new_dt.timestamp();
            },
            RepeatType::Years => {
                let initial_dt: DateTime<Utc> = DateTime::from_timestamp(self.initial, 0).unwrap();
                let year = (initial_dt.year() as i64 + (self.repeating.n * n)) as i32;
                let new_dt = initial_dt.with_year(year).unwrap();
                return new_dt.timestamp();
            }
            _ => return self.initial + (average_seconds(self.repeating)* n),
        }
    }
    
    pub fn most_recent(&self) -> i64 {
        let now: i64 = Utc::now().timestamp();
        
        //Use average number of seconds per repeat to get a good starting point, then iterate until nth(i + 1) is in the future.
        let diff = now - self.initial;
        let approx_n = diff / average_seconds(self.repeating);
        let mut i = approx_n - 1;
        
        while self.nth(i+1) < now {
            i+=1;
        }

        return self.nth(i);
    }

    pub fn elapsed(&self) -> i64 {
        let now: i64 = Utc::now().timestamp();
        return now - self.most_recent();
    }

    pub fn next(&self) -> i64 {
        let now: i64 = Utc::now().timestamp();
        
        //Use average number of seconds per repeat to get a good starting point, then iterate until nth(i - 1) is in the past.
        let diff = now - self.initial;
        let approx_n = diff / average_seconds(self.repeating);
        let mut i = approx_n + 1;
        
        while self.nth(i-1) > now {
            i-=1;
        }

        return self.nth(i);
    }
}

fn average_seconds(interval: RepeatInterval) -> i64 {
    let t_sec = match interval.t {
        RepeatType::Seconds => 1,
        RepeatType::Minutes => 60,
        RepeatType::Hours => 3600,
        RepeatType::Days => 86400,
        RepeatType::Weeks => 604800,
        RepeatType::Months => 2628288,
        RepeatType::Years => 31556952,
    };
    return t_sec * interval.n;
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct RepeatInterval {
    pub t: RepeatType,
    pub n: i64,
}