use std::net::SocketAddr;
use std::path::PathBuf;
use std::thread;

use directories::ProjectDirs;

use fs4::tokio::AsyncFileExt;

use reqwest::StatusCode;
use tokio::io::{AsyncWriteExt,BufReader,AsyncBufReadExt, AsyncReadExt};
use tokio::fs::{OpenOptions,File};
use serde::{Serialize, Deserialize};

use axum::{
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;

use poise::serenity_prelude as serenity;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn get_dir() -> Result<PathBuf,Error> {
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

async fn try_get_file(ctx: &Context<'_>, file_path: &PathBuf) -> Result<File, Error> {
    match OpenOptions::new().read(true).write(true).create(true).open(file_path).await {
        Err(e) => {
            println!("openoptions:{e:?}");
            ctx.say("Encountered error accessing files").await?;
            Err(Box::new(e))
        },
        Ok(res) => Ok(res)
    }
}

async fn write_tmp_and_copy(ctx: &Context<'_>, file_path: &PathBuf, file:File, data: &str) -> Result<(), Error> {
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

#[derive(Serialize,Deserialize,Debug)]
struct GeneralData {
    channel_id: Option<u64>,
    admin_roles: Option<Vec<u64>>,
}

async fn channel_check(ctx: Context<'_>) -> Result<bool, Error> {
    let dir = get_dir()?;
    let file_path = dir.join("data.json");

    let mut file = try_get_file(&ctx, &file_path).await?;
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

async fn admin_check(ctx: Context<'_>) -> Result<bool, Error> {
    let dir = get_dir()?;
    let file_path = dir.join("data.json");

    let mut file = try_get_file(&ctx, &file_path).await?;
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

/// Admin only command to set channel for bot to be used in.
#[poise::command(slash_command, check = "admin_check")]
pub async fn setchannel(
    ctx: Context<'_>,
    #[rest]
    #[description = "Channel"]
    channel: serenity::GuildChannel,
) -> Result<(),Error> {
    let channelid = channel.id.0;
    let dir = get_dir()?;
    let file_path = dir.join("data.json");

    let mut file = try_get_file(&ctx, &file_path).await?;

    file.lock_exclusive()?;
    
    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await?;
    let mut data: GeneralData =  if data_string == "" {
        GeneralData{channel_id: None, admin_roles: None}
    } else {
        serde_json::from_str(&data_string)?
    };
    
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
    let dir = get_dir()?;

    let file_path = dir.join("data.json");
    let mut file = try_get_file(&ctx, &file_path).await?;

    file.lock_exclusive()?;
    
    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await?;
    let mut data: GeneralData =  if data_string == "" {
        GeneralData{channel_id: None, admin_roles: None}
    } else {
        serde_json::from_str(&data_string)?
    };

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
    let dir = get_dir()?;
    let file_path = dir.join("data.json");

    let mut file = try_get_file(&ctx, &file_path).await?;

    file.lock_exclusive()?;
    
    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await?;
    let mut data: GeneralData =  if data_string == "" {
        GeneralData{channel_id: None, admin_roles: None}
    } else {
        serde_json::from_str(&data_string)?
    };

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

/// Admin only command to add users from the admin whitelist
#[poise::command(slash_command, check = "admin_check")]
pub async fn adduser(
    ctx: Context<'_>,
    #[rest]
    #[description = "Resonite UserID"]
    uid: String,
) -> Result<(),Error> {
    let dir = get_dir()?;
    let file_path = dir.join("usersadmin.txt");

    let mut file = try_get_file(&ctx, &file_path).await?;

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

    let mut file = try_get_file(&ctx, &file_path).await?;

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
    let response = reqwest::get(format!("https://api.resonite.com/users/{uid}")).await?.status();
    let code = response.as_u16();

    match response {
        StatusCode::NOT_FOUND => {
            ctx.say("UserID not found, make sure capitalizations are correct and try checking with `/userid <username>`").await?;
            return Ok(());
        },
        StatusCode::BAD_REQUEST => {
            ctx.say("UserID invalid format. UserID should look like `U-xxxx`. To get your UserID, try using `/userid <username`").await?;
            return Ok(());
        },
        StatusCode::OK => (),
        _ => {
            ctx.say(format!("Error validating UserID with Resonite API, error code {code}. Please try again later or report this to Cadyn.")).await?;
            return Ok(())
        }
    }

    let dir = get_dir()?;
    let file_path = dir.join("usersauth.txt");

    let mut file = try_get_file(&ctx, &file_path).await?;

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

#[poise::command(slash_command, prefix_command)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<(), Error> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}

fn main() {
    let t1 = thread::spawn(|| {
        discord();
    });
    let t2 = thread::spawn(|| {
        web();
    });

    t1.join().unwrap();
    t2.join().unwrap();
}

#[tokio::main]
async fn discord() {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![register(),userid(),setchannel(),addrole(),removerole(),adduser(),removeuser(),help()],
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

#[tokio::main]
async fn web() {
    // initialize tracing
    //tracing_subscriber::fmt::init();

    let config = RustlsConfig::from_pem_file(
        PathBuf::from(std::env::var("SERVER_SSL_CERT").expect("No SSL Cert provided")),
        PathBuf::from(std::env::var("SERVER_SSL_KEY").expect("No SSL Key provided"))
    )
    .await
    .unwrap();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));


    let addr = SocketAddr::from(([0, 0, 0, 0], 2096));
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> String {
    let dir = get_dir().unwrap();
    //First read from the admin file as it's already in the format we need
    let file_path = dir.join("usersadmin.txt");

    let mut file = match OpenOptions::new().read(true).write(true).create(true).open(file_path).await {
        Err(e) => {
            println!("openoptions:{e:?}");
            panic!("{e:?}");
        },
        Ok(res) => res
    };
    file.lock_shared().unwrap();

    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await.unwrap();

    file.unlock().unwrap();
    
    //Next read from the auth file and do it by line so we only grab the bits we want.
    let dir = get_dir().unwrap();
    let file_path = dir.join("usersauth.txt");

    let mut file = match OpenOptions::new().read(true).write(true).create(true).open(file_path).await {
        Err(e) => {
            println!("openoptions:{e:?}");
            panic!("{e:?}");
        },
        Ok(res) => res
    };
    file.lock_shared().unwrap();

    let buf = BufReader::new(&mut file);
    let mut lines_reader = buf.lines();

    while let Some(next_line) = lines_reader.next_line().await.unwrap() {
        let resuid: &str = next_line.split('=').collect::<Vec<&str>>()[1];
        data_string +=  &format!("\n{resuid}");
    }
    
    file.unlock().unwrap();

    return data_string.trim().to_string();
}