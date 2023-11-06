use fs4::tokio::AsyncFileExt;

use poise::serenity_prelude as serenity;

use super::common::GeneralData;
use super::checks::admin_check;
use crate::commonio::*;

/// Admin only command to set channel for bot to be used in.
#[poise::command(slash_command, check = "admin_check")]
pub async fn setchannel(
    ctx: Context<'_>,
    #[rest]
    #[description = "Channel"]
    channel: serenity::GuildChannel,
) -> Result<(),Error> {
    let channelid = channel.id.0;
    let (file_path, file, mut data) = load_json::<GeneralData>(Some(&ctx),"data.json".to_string(), false).await?;
    
    data.channel_id = Some(channelid);

    write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;

    ctx.say(format!("Changed channel to <#{}>",channelid)).await?;
    Ok(())
}

/// Admin only command to add roles to admin access to the bot
#[poise::command(slash_command, check = "admin_check")]
pub async fn addrole (
    ctx: Context<'_>,
    #[rest]
    #[description = "Role"]
    role: serenity::Role,
) -> Result<(),Error> {
    let newroleid = role.id.0;
    let (file_path, file, mut data) = load_json::<GeneralData>(Some(&ctx),"data.json".to_string(), false).await?;

    if let Some(roleids) = &mut data.admin_roles {
        if roleids.contains(&newroleid) {
            ctx.say("Role is already assigned as admin").await?;
            file.unlock()?;
            return Ok(());
        }
        roleids.push(newroleid);
        write_tmp_and_copy(&ctx, &file_path, file,&serde_json::to_string(&data)?).await?;
        
        //file.unlock()?;
        ctx.say("Role has been added to admin roles.").await?;
        return Ok(());
    }
    data.admin_roles = Some(vec![newroleid]);
    write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;
    //file.unlock()?;
    ctx.say("Role has been added to admin roles.").await?;
    Ok(())
}

/// Admin only command to remove roles from admin access to the bot
#[poise::command(slash_command, check = "admin_check")]
pub async fn removerole (
    ctx: Context<'_>,
    #[rest]
    #[description = "Role"]
    role: serenity::Role,
) -> Result<(),Error> {
    let newroleid = role.id.0;
    let (file_path, file, mut data) = load_json::<GeneralData>(Some(&ctx),"data.json".to_string(), false).await?;

    if let Some(roleids) = &mut data.admin_roles {
        if roleids.contains(&newroleid) {
            let index = roleids.iter().position(|x| *x == newroleid).unwrap();
            roleids.remove(index);
            write_tmp_and_copy(&ctx, &file_path, file, &serde_json::to_string(&data)?).await?;
            ctx.say("Role has been removed from admins").await?;
            return Ok(());
        }
        
        ctx.say("Role is not in admins.").await?;
        return Ok(());
    }

    ctx.say("Role is not in admins.").await?;
    Ok(())
}