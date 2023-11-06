use fs4::tokio::AsyncFileExt;

use tokio::io::{BufReader, AsyncBufReadExt};

use crate::commonio::*;
use super::checks::*;
use super::common::check_userid;

/// Admin only command to add users to the admin whitelist
#[poise::command(slash_command, check = "admin_check")]
pub async fn adduser(
    ctx: Context<'_>,
    #[rest]
    #[description = "Resonite UserID"]
    uid: String,
) -> Result<(),Error> {
    if !check_userid(&ctx, &uid).await? {
        return Ok(());
    }

    let dir = get_dir()?;
    let file_path = dir.join("usersadmin.txt");

    let mut file = try_get_file(Some(&ctx), &file_path).await?;

    file.lock_exclusive()?;
    
    let buf = BufReader::new(&mut file);
    let mut lines_reader = buf.lines();
    let mut lines: Vec<String> = Vec::new();

    while let Some(next_line) = lines_reader.next_line().await? {
        lines.push(next_line);
    }

    if lines.contains(&uid) {
        ctx.say("User is already in admin whitelist").await?;
        file.unlock()?;
        return Ok(());
    }

    lines.push(uid.clone());

    

    let new_contents = lines.join("\n");
    match write_tmp_and_copy(&ctx, &file_path, file, &new_contents).await {
        Ok(_) => {
            ctx.say(format!("Successfully added record for {uid}!")).await?;
            return Ok(());
        }
        Err(e) => {
            ctx.say("Error writing to file, please try again later.").await?;
            return Err(e);
        }
    }
}

/// Admin only command to remove users from the admin whitelist
#[poise::command(slash_command, check = "admin_check")]
pub async fn removeuser(
    ctx: Context<'_>,
    #[rest]
    #[description = "Resonite UserID"]
    uid: String,
) -> Result<(),Error> {
    let dir = get_dir()?;
    let file_path = dir.join("usersadmin.txt");

    let mut file = try_get_file(Some(&ctx), &file_path).await?;

    file.lock_exclusive()?;
    
    let buf = BufReader::new(&mut file);
    let mut lines_reader = buf.lines();
    let mut lines: Vec<String> = Vec::new();

    while let Some(next_line) = lines_reader.next_line().await? {
        lines.push(next_line);
    }

    if !lines.contains(&uid) {
        ctx.say("UserID was not in admin whitelist").await?;
        return Ok(());
    }

    let index = lines.iter().position(|x| *x == uid).unwrap();
    lines.remove(index);

    let new_contents = lines.join("\n");
    match write_tmp_and_copy(&ctx, &file_path, file, &new_contents).await {
        Ok(_) => {
            ctx.say(format!("Successfully added record for {uid}!")).await?;
            return Ok(());
        }
        Err(e) => {
            ctx.say("Error writing to file, please try again later.").await?;
            return Err(e);
        }
    }
}

/// Register your UserID with the headless whitelist or change your registered UserID.
#[poise::command(slash_command, check = "channel_check")]
pub async fn register(
    ctx: Context<'_>,
    #[rest]
    #[description = "Resonite UserID"]
    uid: String,
) -> Result<(), Error> {
    if !check_userid(&ctx, &uid).await? {
        return Ok(());
    }

    let dir = get_dir()?;
    let file_path = dir.join("usersauth.txt");

    let mut file = try_get_file(Some(&ctx), &file_path).await?;

    file.lock_exclusive()?;
    let userid = ctx.author().id.0.to_string();
    
    let buf = BufReader::new(&mut file);
    let mut lines_reader = buf.lines();
    let mut lines: Vec<String> = Vec::new();

    while let Some(next_line) = lines_reader.next_line().await? {
        lines.push(next_line);
    }

    let mut existing_record = false;
    for line_i in &mut lines {
        if line_i.starts_with(&userid){
            existing_record = true;
            *line_i = format!("{userid}={uid}");
        }
    }

    let mut operation = "changed";
    if !existing_record {
        operation = "created";
        lines.push(format!("{userid}={uid}"))
    }

    let new_contents = lines.join("\n");
    match write_tmp_and_copy(&ctx, &file_path, file, &new_contents).await {
        Ok(_) => {
            ctx.say(format!("Successfully {operation} your record!")).await?;
            return Ok(());
        }
        Err(e) => {
            ctx.say("Error writing to file, please try again later.").await?;
            return Err(e);
        }
    }
}