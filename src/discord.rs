pub mod mainwhitelist;
pub mod admin;
pub mod checks;
pub mod common;
pub mod closedwhitelist;

use serde::{Serialize, Deserialize};

use poise::serenity_prelude as serenity;

use checks::*;
use mainwhitelist::*;
use admin::*;
use closedwhitelist::*;
use crate::commonio::*;


#[derive(Serialize,Deserialize,Debug)]
#[serde(transparent)]
struct UserResponse {
    users: Vec<UserData>,
}

#[derive(Serialize,Deserialize,Debug)]
struct UserData {
    id: String,
}

/// Gets the Resonite UserID from a given username.
#[poise::command(slash_command, check = "channel_check")]
pub async fn userid(
    ctx: Context<'_>,
    #[rest]
    #[description = "Resonite Username"]
    username: String,
) -> Result<(), Error> {
    let response = reqwest::get(format!("https://api.resonite.com/users?name={username}")).await?.json::<UserResponse>().await?;
    match response.users.get(0) {
        Some(userdata) => {
            let userid = &userdata.id;
            ctx.say(format!("The UserID for {username} is {userid}")).await?;
        },
        None => {
            ctx.say(format!("No such user, {username}, exists")).await?;
        }
    };
    
    Ok(())
}

/// Provides a list of all commands and some basic information about them.
#[poise::command(slash_command, prefix_command)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<(), Error> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}


#[tokio::main]
pub async fn discord() {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![register(),userid(),status(),help(),setchannel(),addrole(),removerole(),adduser(),removeuser(),adduserclosed(),removeuserclosed(),setclosed(),addcloseevent(),removecloseevent(),addopenevent(),removeopenevent(),listevents()],
            ..Default::default()
        })
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        });

    framework.run().await.unwrap();
}