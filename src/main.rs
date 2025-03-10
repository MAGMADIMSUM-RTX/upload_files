use actix_multipart::Multipart;
use actix_web::{App, HttpResponse, HttpServer, web};
use futures::{StreamExt, TryStreamExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

struct AppState {
    cancel_flags: Arc<Mutex<HashMap<String, bool>>>,
}

async fn upload(mut payload: Multipart, data: web::Data<AppState>) -> HttpResponse {
    let mut password = None;
    let mut files = Vec::new();
    let cancel_flags = data.cancel_flags.clone();
    let upload_id = Uuid::new_v4().to_string(); // 修改此行

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let name = content_disposition.get_name().unwrap_or("");

        if name == "password" {
            let mut data = Vec::new();
            while let Some(chunk) = field.next().await {
                data.extend_from_slice(&chunk.unwrap());
            }
            password = Some(String::from_utf8(data).unwrap());
        } else if name == "file" {
            let filename = content_disposition
                .get_filename()
                .unwrap_or("unknown")
                .to_string();
            let filepath = format!("/home/lxzs/Downloads/{}", filename);
            let filepath_clone = filepath.clone();
            let mut f = match web::block(move || File::create(&filepath_clone)).await {
                Ok(file) => file.unwrap(),
                Err(_) => return HttpResponse::InternalServerError().body("Failed to create file"),
            };
            while let Some(chunk) = field.next().await {
                if *cancel_flags
                    .lock()
                    .unwrap()
                    .get(&upload_id)
                    .unwrap_or(&false)
                {
                    std::fs::remove_file(&filepath).ok();
                    return HttpResponse::Ok().body("Upload cancelled");
                }
                let data = chunk.unwrap();
                f = match web::block(move || f.write_all(&data).map(|_| f)).await {
                    Ok(file) => file.unwrap(),
                    Err(_) => {
                        return HttpResponse::InternalServerError().body("Failed to write file");
                    }
                };
            }
            files.push(filename);
        }
    }

    if password != Some("lc".to_string()) {
        return HttpResponse::Forbidden().body("Incorrect password");
    }

    HttpResponse::Ok().body(format!("Uploaded files: {:?}", files))
}

async fn cancel_upload(data: web::Data<AppState>, upload_id: web::Path<String>) -> HttpResponse {
    data.cancel_flags
        .lock()
        .unwrap()
        .insert(upload_id.into_inner(), true);
    HttpResponse::Ok().body("Upload cancelled")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("/home/lxzs/Downloads/uploads")?;

    let cancel_flags = Arc::new(Mutex::new(HashMap::new()));

    let port = if cfg!(debug_assertions) { 8081 } else { 8080 };
    println!("Listening on port :{}", port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                cancel_flags: cancel_flags.clone(),
            }))
            .service(web::resource("/upload").route(web::post().to(upload)))
            .service(web::resource("/cancel/{upload_id}").route(web::post().to(cancel_upload)))
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
