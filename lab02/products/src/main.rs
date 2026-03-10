mod media;
mod product;
mod upload;

use std::sync::Mutex;

use actix_form_data::Multipart;
use actix_web::{
    App, HttpResponse, HttpServer, delete, get, post, put,
    web::{self, Data},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    media::Images,
    product::{PartialProduct, Products},
    upload::UploadedIcon,
};

fn to_response(raw: anyhow::Result<HttpResponse>) -> HttpResponse {
    let (Ok(resp) | Err(resp)) = raw.map_err(http_error);
    resp
}

fn http_error(err: anyhow::Error) -> HttpResponse {
    HttpResponse::InternalServerError().body(err.to_string())
}

fn missing_image(icon: Option<Uuid>, images: web::Data<Mutex<Images>>) -> anyhow::Result<()> {
    match icon {
        Some(id) => images.lock().unwrap().contains(id),
        None => Ok(()),
    }
}

#[derive(Deserialize)]
pub struct CreateProductRequest {
    name: String,
    description: String,
    icon: Option<media::Id>,
}

async fn try_create_product(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    request: CreateProductRequest,
) -> anyhow::Result<HttpResponse> {
    missing_image(request.icon, images)?;
    Ok(HttpResponse::Ok().json(products.lock().unwrap().create(
        request.name,
        request.description,
        request.icon,
    )))
}

#[post("/product")]
async fn create_product(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    body: web::Json<CreateProductRequest>,
) -> HttpResponse {
    to_response(try_create_product(products, images, body.into_inner()).await)
}

async fn try_product_by_id(
    products: web::Data<Mutex<Products>>,
    id: product::Id,
) -> anyhow::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(products.lock().unwrap().find(id)?))
}

#[get("/product/{id}")]
async fn product_by_id(products: web::Data<Mutex<Products>>, id: web::Path<i64>) -> HttpResponse {
    to_response(try_product_by_id(products, id.into_inner()).await)
}

async fn try_update_product(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    id: product::Id,
    request: PartialProduct,
) -> anyhow::Result<HttpResponse> {
    missing_image(request.icon(), images)?;
    Ok(HttpResponse::Ok().json(products.lock().unwrap().update(id, request)?))
}

#[put("/product/{id}")]
async fn update_product(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    id: web::Path<product::Id>,
    request: web::Json<PartialProduct>,
) -> HttpResponse {
    to_response(try_update_product(products, images, id.into_inner(), request.into_inner()).await)
}

async fn try_delete_product(
    products: web::Data<Mutex<Products>>,
    id: product::Id,
) -> anyhow::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(products.lock().unwrap().delete(id)?))
}

#[delete("/product/{id}")]
async fn delete_product(
    products: web::Data<Mutex<Products>>,
    id: web::Path<product::Id>,
) -> HttpResponse {
    to_response(try_delete_product(products, id.into_inner()).await)
}

async fn try_products(products: web::Data<Mutex<Products>>) -> anyhow::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(products.lock().unwrap().all()))
}

#[get("/products")]
async fn all_products(products: web::Data<Mutex<Products>>) -> HttpResponse {
    to_response(try_products(products).await)
}

async fn try_upload_icon(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    id: product::Id,
    uploaded: UploadedIcon,
) -> anyhow::Result<HttpResponse> {
    let icon = images.lock().unwrap().create(
        uploaded
            .bytes()
            .ok_or_else(|| anyhow::anyhow!("Wrong form-data layout"))?,
    );
    Ok(HttpResponse::Ok().json(products.lock().unwrap().set_icon(id, icon)?))
}

#[post("/product/{id}/image")]
async fn upload_icon(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    id: web::Path<product::Id>,
    Multipart(uploaded): Multipart<UploadedIcon>,
) -> HttpResponse {
    to_response(try_upload_icon(products, images, id.into_inner(), uploaded).await)
}

async fn try_download_icon(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    id: product::Id,
) -> anyhow::Result<HttpResponse> {
    let icon = products.lock().unwrap().icon(id)?;
    Ok(HttpResponse::Ok().body(images.lock().unwrap().find(icon)?.bytes()))
}

#[get("/product/{id}/image")]
async fn download_icon(
    products: web::Data<Mutex<Products>>,
    images: web::Data<Mutex<Images>>,
    id: web::Path<product::Id>,
) -> HttpResponse {
    to_response(try_download_icon(products, images, id.into_inner()).await)
}

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json("ok")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let products = Data::new(Mutex::new(Products::new()));
    let images = Data::new(Mutex::new(Images::new()));
    HttpServer::new(move || {
        App::new()
            .app_data(products.clone())
            .app_data(images.clone())
            .service(create_product)
            .service(product_by_id)
            .service(update_product)
            .service(delete_product)
            .service(all_products)
            .service(upload_icon)
            .service(download_icon)
            .service(health)
    })
    .bind(("0.0.0.0", 8100))?
    .run()
    .await
}
