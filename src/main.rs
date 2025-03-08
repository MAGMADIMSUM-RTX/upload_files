use actix_web::{web, App, HttpResponse, HttpServer};
use futures::{StreamExt, TryStreamExt};
use std::io::Write;
use actix_multipart::Multipart;
use std::fs::File;

async fn upload(mut payload: Multipart) -> HttpResponse {
    let mut password = None;
    let mut files = Vec::new();

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
            let filename = content_disposition.get_filename().unwrap_or("unknown").to_string();
            let filepath = format!("/home/lckfb/Downloads/uploads/{}", filename);
            let filepath_clone = filepath.clone();
            let mut f = match web::block(move || File::create(&filepath_clone)).await {
                Ok(file) => file.unwrap(),
                Err(_) => return HttpResponse::InternalServerError().body("Failed to create file"),
            };
            while let Some(chunk) = field.next().await {
                let data = chunk.unwrap();
                f = match web::block(move || f.write_all(&data).map(|_| f)).await {
                    Ok(file) => file.unwrap(),
                    Err(_) => return HttpResponse::InternalServerError().body("Failed to write file"),
                };
            }
            files.push(filename);
        }
    }

    // 修改密码验证逻辑，将 "correct_password" 改为 "lc"
    if password != Some("lc".to_string()) {
        return HttpResponse::Forbidden().body("Incorrect password");
    }

    HttpResponse::Ok().body(format!("Uploaded files: {:?}", files))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 确保上传目录存在
    std::fs::create_dir_all("/home/linaro/Downloads/uploads")?;

    HttpServer::new(|| {
        App::new()
            .service(web::resource("/upload").route(web::post().to(upload)))
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
