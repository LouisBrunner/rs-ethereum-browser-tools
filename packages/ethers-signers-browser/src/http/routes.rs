use super::{comm::CommServer, session::WSFlow};
use actix::Addr;
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};
use actix_web_actors::ws;
use mime_guess::from_path;
use rust_embed::RustEmbed;
use serde::Deserialize;

#[derive(RustEmbed)]
#[folder = "$OUT_DIR/frontend"]
struct Asset;

fn handle_embedded_file(path: &str) -> HttpResponse {
    match Asset::get(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[derive(Deserialize)]
pub(super) struct IndexQuery {
    nonce: String,
}

#[actix_web::get("/")]
pub(super) async fn index(
    info: web::Query<IndexQuery>,
    nonce: web::Data<String>,
) -> impl Responder {
    if info.nonce != **nonce {
        return HttpResponse::NotFound().body("404 Not Found")
    }
    handle_embedded_file("index.html")
}

#[actix_web::get("/ws/")]
pub(super) async fn ws_open(
    req: HttpRequest,
    stream: web::Payload,
    comm: web::Data<Addr<CommServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(WSFlow::new(comm.get_ref().clone()), &req, stream)
}

#[actix_web::get("/dist/{_:.*}")]
pub(super) async fn dist(path: web::Path<String>) -> impl Responder {
    handle_embedded_file(path.as_str())
}
