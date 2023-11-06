use fs4::tokio::AsyncFileExt;

use tokio::io::{BufReader, AsyncReadExt};

use crate::commonio::*;
use super::common::GeneralData;

pub async fn channel_check(ctx: Context<'_>) -> Result<bool, Error> {
    let dir = get_dir()?;
    let file_path = dir.join("data.json");

    let mut file = try_get_file(Some(&ctx), &file_path).await?;
    file.lock_shared()?;
    
    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();
    buf.read_to_string(&mut data_string).await?;
    if data_string == "" {
        return Ok(true);
    }
    let data: GeneralData = serde_json::from_str(&data_string)?;
    file.unlock()?;
    
    if let Some(channelid) = data.channel_id {
        let is_correct_channel = channelid == ctx.channel_id().0;
        
        return Ok(is_correct_channel);
    }
    return Ok(true);
}

async fn has_admin_perm(ctx: &Context<'_>) -> Result<bool, Error> {
    let permissions = ctx.author_member().await.expect("Only for use in guilds").permissions(ctx)?;
    Ok(permissions.administrator())
}

pub async fn admin_check(ctx: Context<'_>) -> Result<bool, Error> {
    let dir = get_dir()?;
    let file_path = dir.join("data.json");

    let mut file = try_get_file(Some(&ctx), &file_path).await?;
    file.lock_shared()?;
    
    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await?;
    if data_string == "" {
        return has_admin_perm(&ctx).await;
    }
    let data: GeneralData = serde_json::from_str(&data_string)?;
    file.unlock()?;

    if let Some(roleids) = data.admin_roles {

        let mut is_correct_channel = false;
        for roleid in roleids {
            if ctx.author().has_role(ctx,ctx.guild_id().expect("should be in guild"),roleid).await? {
                is_correct_channel = true;
                break;
            }
        }

        return Ok(is_correct_channel);
    }
    return has_admin_perm(&ctx).await;
}