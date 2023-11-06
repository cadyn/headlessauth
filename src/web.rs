use std::net::SocketAddr;
use std::path::PathBuf;

use fs4::tokio::AsyncFileExt;

use tokio::io::{BufReader,AsyncBufReadExt, AsyncReadExt};

use crate::commonio::*;

use axum::{
    extract::Path,
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;

#[tokio::main]
pub async fn web() {
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
        .route("/", get(root))
        .route("/check/:userid", get(is_whitelisted))
        .route("/open", get(open))
        .route("/open/check/:userid", get(open_is_whitelisted))
        .route("/closed", get(closed))
        .route("/closed/check/:userid", get(closed_is_whitelisted));
        

    let addr = SocketAddr::from(([0, 0, 0, 0], 2096));
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn root() -> String {
    let (_file_path, _file, data) = load_json::<ClosedData>(None,"closed.json".to_string(),true).await.unwrap();
    if data.is_currently_closed() {
        return closed().await;
    } else {
        return open().await;
    }
}

async fn is_whitelisted(Path(userid): Path<String>) -> String {
    let (_file_path, _file, data) = load_json::<ClosedData>(None,"closed.json".to_string(),true).await.unwrap();
    if data.is_currently_closed() {
        return closed_is_whitelisted(Path::<String>(userid)).await;
    } else {
        return open_is_whitelisted(Path::<String>(userid)).await;
    }
}

async fn open() -> String {
    let dir = get_dir().unwrap();
    //First read from the admin file as it's already in the format we need
    let file_path = dir.join("usersadmin.txt");

    let mut file = try_get_file(None, &file_path).await.unwrap();
    file.lock_shared().unwrap();

    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await.unwrap();

    file.unlock().unwrap();
    
    //Next read from the auth file and do it by line so we only grab the bits we want.
    let dir = get_dir().unwrap();
    let file_path = dir.join("usersauth.txt");

    let mut file = try_get_file(None, &file_path).await.unwrap();
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

async fn open_is_whitelisted(Path(userid): Path<String>) -> String {
    let dir = get_dir().unwrap();
    //First read from the admin file as it's already in the format we need
    let file_path = dir.join("usersadmin.txt");

    let mut file = try_get_file(None, &file_path).await.unwrap();
    file.lock_shared().unwrap();

    let buf = BufReader::new(&mut file);

    let mut lines: Vec<String> = Vec::new();
    let mut lines_reader = buf.lines();

    while let Some(next_line) = lines_reader.next_line().await.unwrap() {
        lines.push(next_line.clone());
    }

    file.unlock().unwrap();
    
    //Next read from the auth file and do it by line so we only grab the bits we want.
    let dir = get_dir().unwrap();
    let file_path = dir.join("usersauth.txt");

    let mut file = try_get_file(None, &file_path).await.unwrap();
    file.lock_shared().unwrap();

    let buf = BufReader::new(&mut file);
    let mut lines_reader = buf.lines();

    while let Some(next_line) = lines_reader.next_line().await.unwrap() {
        let resuid: &str = next_line.split('=').collect::<Vec<&str>>()[1];
        lines.push(resuid.to_string());
    }
    
    file.unlock().unwrap();

    if lines.contains(&userid) {
        return "TRUE".to_string()
    }
    return "FALSE".to_string();
}

async fn closed() -> String {
    let dir = get_dir().unwrap();
    //Closed whitelist is all in one file so we just read the file basically.
    let file_path = dir.join("usersclosed.txt");

    let mut file = try_get_file(None, &file_path).await.unwrap();
    file.lock_shared().unwrap();

    let mut buf = BufReader::new(&mut file);
    let mut data_string = String::new();

    buf.read_to_string(&mut data_string).await.unwrap();

    file.unlock().unwrap();
    return data_string.trim().to_string();
}

async fn closed_is_whitelisted(Path(userid): Path<String>) -> String {
    let dir = get_dir().unwrap();
    //Read closed whitelist to lines
    let file_path = dir.join("usersclosed.txt");

    let mut file = try_get_file(None, &file_path).await.unwrap();
    file.lock_shared().unwrap();

    let buf = BufReader::new(&mut file);

    let mut lines: Vec<String> = Vec::new();
    let mut lines_reader = buf.lines();

    while let Some(next_line) = lines_reader.next_line().await.unwrap() {
        lines.push(next_line.clone());
    }

    file.unlock().unwrap();

    if lines.contains(&userid) {
        return "TRUE".to_string()
    }
    return "FALSE".to_string();
}