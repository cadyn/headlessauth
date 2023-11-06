use reqwest::StatusCode;
use serde::{Serialize, Deserialize};

use crate::commonio::*;

#[derive(Serialize,Deserialize,Debug)]
pub struct GeneralData {
    pub channel_id: Option<u64>,
    pub admin_roles: Option<Vec<u64>>,
}

impl Default for GeneralData {
    fn default() -> Self {GeneralData{channel_id: None, admin_roles: None}}
}

pub async fn check_userid(ctx: &Context<'_>, uid: &str) -> Result<bool,Error>{
    let response = reqwest::get(format!("https://api.resonite.com/users/{uid}")).await?.status();
    let code = response.as_u16();

    match response {
        StatusCode::NOT_FOUND => {
            ctx.say("UserID not found, make sure capitalizations are correct and try checking with `/userid <username>`").await?;
            return Ok(false);
        },
        StatusCode::BAD_REQUEST => {
            ctx.say("UserID invalid format. UserID should look like `U-xxxx`. To get your UserID, try using `/userid <username`").await?;
            return Ok(false);
        },
        StatusCode::OK => Ok(true),
        _ => {
            ctx.say(format!("Error validating UserID with Resonite API, error code {code}. Please try again later or report this to Cadyn.")).await?;
            return Ok(false);
        }
    }
}