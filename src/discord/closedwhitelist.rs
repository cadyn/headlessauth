use fs4::tokio::AsyncFileExt;
use tokio::io::{BufReader, AsyncBufReadExt};

use crate::commonio::*;
use crate::repeat::*;
use super::checks::*;
use super::common::check_userid;

/// Admin only command to add users to the closed whitelist
#[poise::command(slash_command, check = "admin_check")]
pub async fn adduserclosed(
    ctx: Context<'_>,
    #[rest]
    #[description = "Resonite UserID"]
    uid: String,
) -> Result<(),Error> {
    if !check_userid(&ctx, &uid).await? {
        return Ok(());
    }

    let dir = get_dir()?;
    let file_path = dir.join("usersclosed.txt");

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

/// Admin only command to remove users from the closed whitelist
#[poise::command(slash_command, check = "admin_check")]
pub async fn removeuserclosed(
    ctx: Context<'_>,
    #[rest]
    #[description = "Resonite UserID"]
    uid: String,
) -> Result<(),Error> {
    let dir = get_dir()?;
    let file_path = dir.join("usersclosed.txt");

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

/// Admin only command to set the headless to closed mode.
#[poise::command(slash_command, check = "admin_check")]
pub async fn setclosed(
    ctx: Context<'_>,
    #[rest]
    #[description = "Closed"]
    closed: ClosedStatus,
) -> Result<(),Error> {
    let (file_path, file, mut data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), false).await?;
    
    data.is_closed = closed;

    write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;
    
    let to_say = match closed {
        ClosedStatus::Open => "Open",
        ClosedStatus::Closed => "Closed",
        ClosedStatus::Automatic => "Automatic",
    };

    ctx.say(format!("Set headless to {to_say}")).await?;
    Ok(())
}



/// Admin only command to add a repeating time to automatically set the headless to closed
#[poise::command(slash_command, check = "admin_check")]
pub async fn addcloseevent(
    ctx: Context<'_>,
    #[rest]
    #[description = "Starting timestamp"]
    timestamp: i64,
    #[description = "Repeat every N"]
    n: i64,
    #[description = "type"]
    t: RepeatType,
) -> Result<(),Error> {
    let (file_path, file, mut data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), false).await?;
    
    let current_id: usize = if data.close_events.is_empty() {0} else {
        *data.close_events.keys().max().unwrap() + 1
    };

    data.close_events.insert(current_id, RepeatingEvent{id: current_id, initial: timestamp, repeating: RepeatInterval{t,n}});

    write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;
    
    let type_s = t.to_string() + if n > 1 {"s"} else {""};

    let to_say = format!("Added event to close every {n} {type_s} starting on <t:{timestamp}:f>");
    ctx.say(to_say).await?;
    Ok(())
}

/// Admin only command to add a repeating time to automatically set the headless to open
#[poise::command(slash_command, check = "admin_check")]
pub async fn addopenevent(
    ctx: Context<'_>,
    #[rest]
    #[description = "Starting timestamp"]
    timestamp: i64,
    #[description = "Repeat every N"]
    n: i64,
    #[description = "type"]
    t: RepeatType,
) -> Result<(),Error> {
    let (file_path, file, mut data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), false).await?;
    
    let current_id: usize = if data.close_events.is_empty() {0} else {
        *data.open_events.keys().max().unwrap() + 1
    };

    data.open_events.insert(current_id, RepeatingEvent{id: current_id, initial: timestamp, repeating: RepeatInterval{t,n}});
    

    write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;
    
    let type_s = t.to_string() + if n > 1 {"s"} else {""};

    let to_say = format!("Added event to open every {n} {type_s} starting on <t:{timestamp}:f>");
    ctx.say(to_say).await?;
    Ok(())
}

/// Admin only command to remove an opening event
#[poise::command(slash_command, check = "admin_check")]
pub async fn removeopenevent(
    ctx: Context<'_>,
    #[rest]
    #[description = "id of event"]
    id: usize,
) -> Result<(),Error> {
    let (file_path, file, mut data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), false).await?;

    if let Some(value) = data.open_events.remove(&id) {

        let (t, n) = (value.repeating.t, value.repeating.n);
        let type_s = t.to_string() + if n > 1 {"s"} else {""};
        let most_recent = value.most_recent();

        write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;

        let to_say = format!("Removed event {id} to open every {n} {type_s} with most recent at <t:{most_recent}:f>");
        ctx.say(to_say).await?;
        return Ok(());
    }
    ctx.say(format!("No such open event exists with id {id}")).await?;
    file.unlock()?;

    Ok(())
}

/// Admin only command to remove a closing event
#[poise::command(slash_command, check = "admin_check")]
pub async fn removecloseevent(
    ctx: Context<'_>,
    #[rest]
    #[description = "id of event"]
    id: usize,
) -> Result<(),Error> {
    let (file_path, file, mut data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), false).await?;

    if let Some(value) = data.close_events.remove(&id) {

        let (t, n) = (value.repeating.t, value.repeating.n);
        let type_s = t.to_string() + if n > 1 {"s"} else {""};
        let most_recent = value.most_recent();

        write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;

        let to_say = format!("Removed event {id} to close every {n} {type_s} with most recent at <t:{most_recent}:f>");
        ctx.say(to_say).await?;
        return Ok(());
    }
    ctx.say(format!("No such close event exists with id {id}")).await?;
    file.unlock()?;

    Ok(())
}


/// Admin only command to show all events for opening and closing the headless
#[poise::command(slash_command, check = "admin_check")]
pub async fn listevents(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let (_file_path, _file, data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), true).await?;
    _file.unlock()?;
    
    ctx.send(|b| b.content("").embed(|embed| {
        embed.color(poise::serenity_prelude::colours::branding::GREEN)
        .title("Open events");

        for event in data.open_events.values() {
            let most_recent = event.most_recent();
            let (t,n) = (event.repeating.t, event.repeating.n);
            let type_s = t.to_string() + if n > 1 {"s"} else {""};
            let val = format!("<t:{most_recent}:f> every {n} {type_s}");
            embed.field(event.id, val, false);
        }
        embed
    })).await?;

    ctx.send(|b| b.content("").embed(|embed| {
        embed.color(poise::serenity_prelude::colours::branding::RED)
        .title("Close events");

        for event in data.close_events.values() {
            let most_recent = event.most_recent();
            let (t,n) = (event.repeating.t, event.repeating.n);
            let type_s = t.to_string() + if n > 1 {"s"} else {""};
            let val = format!("<t:{most_recent}:f> every {n} {type_s}");
            embed.field(event.id, val, false);
        }
        embed
    })).await?;
    Ok(())
}

/// Check whether the headless is in open or closed mode
#[poise::command(slash_command, check = "channel_check")]
pub async fn status(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let (_file_path, _file, data) = load_json::<ClosedData>(Some(&ctx),"closed.json".to_string(), true).await?;
    _file.unlock()?;

    let next_close = data.next_close_event();
    let next_open = data.next_open_event();
    let close_status = match data.is_closed {
        ClosedStatus::Open => "Manually open",
        ClosedStatus::Closed => "Manually closed",
        ClosedStatus::Automatic => if data.is_currently_closed() {"Automatically closed"} else {"Automatically open"},
    };

    ctx.send(|b| b.embed(|embed| {
        embed.title("Headless whitelist status");
        embed.field("Current status", close_status, false);
        embed.field("Next scheduled closing", format!("<t:{next_close}:f>"), false);
        embed.field("Next scheduled opening", format!("<t:{next_open}:f>"), false);
        embed
    })).await?;

    Ok(())
}