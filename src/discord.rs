pub mod mainwhitelist;
pub mod admin;
pub mod checks;
pub mod common;
pub mod closedwhitelist;

use fs4::tokio::AsyncFileExt;
use serde::{Serialize, Deserialize};

use poise::serenity_prelude as serenity;

use checks::*;
use mainwhitelist::*;
use admin::*;
use closedwhitelist::*;
use common::GeneralData;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
struct PlayerList {
    list: Vec<Player>
}

#[derive(Serialize, Deserialize, Debug, Clone)]

struct Player {
    username: String,
    userid: String,
    jointime: i64,
    pfp: Option<String>,
}

/// Get overall headless status
#[poise::command(slash_command, check = "channel_check")]
pub async fn status(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let (_file_path, file, data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), true).await?;
    file.unlock()?;

    let (_file_path, genfile, gendata) = load_json::<GeneralData>(Some(&ctx),"data.json".to_string(), true).await?;
    genfile.unlock()?;

    let mut num_players: Option<usize> = None;
    if let Some(url) = gendata.info_api {
        let response = reqwest::get(format!("{url}/list")).await?.json::<PlayerList>().await?;
        num_players = Some(response.list.len());
    }

    let next_close = data.next_close_event();
    let next_open = data.next_open_event();
    let close_status = match data.is_closed {
        ClosedStatus::Open => "Manually open",
        ClosedStatus::Closed => "Manually closed",
        ClosedStatus::Automatic => if data.is_currently_closed() {"Automatically closed"} else {"Automatically open"},
    };

    let next_open_str =  if next_open == i64::MAX {"No scheduled openings".to_string()} else {format!("<t:{next_open}:f>")};

    let next_closed_str =  if next_close == i64::MAX {"No scheduled closings".to_string()} else {format!("<t:{next_close}:f>")};

    ctx.send(|b| b.embed(|embed| {
        embed.color(serenity::colours::branding::BLURPLE);
        embed.title("Headless status");
        embed.field("Current whitelist status", close_status, false);
        embed.field("Next scheduled whitelist closing", next_closed_str, false);
        embed.field("Next scheduled whitelist opening", next_open_str, false);
        if let Some(players) = num_players {
            embed.field("Number of players online", format!("{players}"), false);
        }
        embed
    })).await?;

    Ok(())
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
            commands: vec![register(),userid(),status(),help(),setchannel(),addrole(),removerole(),adduser(),removeuser(),adduserclosed(),removeuserclosed(),setclosed(),addcloseevent(),removecloseevent(),addopenevent(),removeopenevent(),listevents(),setinfourl()],
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