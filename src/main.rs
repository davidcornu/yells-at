mod github;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use image::{imageops, ImageFormat};
use lazy_static::lazy_static;
use std::convert::Infallible;
use std::error::Error;
use std::io::Cursor;

pub(crate) type BoxedError = Box<dyn Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxedError> {
    println!("Starting");

    let addr = ([0, 0, 0, 0], 3000).into();

    let service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(endpoint)) });

    let server = Server::bind(&addr).serve(service);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}

async fn endpoint(req: Request<Body>) -> Result<Response<Body>, BoxedError> {
    let path_parts = req
        .uri()
        .path()
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    let result = match (req.method(), path_parts.as_slice()) {
        (&Method::GET, ["favicon.ico"]) => not_found(),
        (&Method::GET, [username]) => serve_image(username).await,
        _ => not_found(),
    };

    if let Err(error) = result {
        println!(
            "{method} {uri} - {error:?}",
            method = req.method(),
            uri = req.uri(),
            error = error
        );

        return internal_error();
    }

    result
}

async fn serve_image(username: &str) -> Result<Response<Body>, BoxedError> {
    let yells = generate(username).await?;

    match yells {
        Some(img) => {
            let mut bytes = Cursor::new(vec![]);
            img.write_to(&mut bytes, ImageFormat::Png)?;

            Ok(Response::builder()
                .header("Content-Type", "image/png")
                .header("Cache-Control", "public, max-age=31557600, immutable")
                .body(Body::from(bytes.into_inner()))
                .unwrap())
        }
        None => not_found(),
    }
}

fn not_found() -> Result<Response<Body>, BoxedError> {
    Ok(Response::builder()
        .status(404)
        .header("Content-Type", "text/plain")
        .body(Body::from("NOT FOUND"))
        .unwrap())
}

fn internal_error() -> Result<Response<Body>, BoxedError> {
    Ok(Response::builder()
        .status(400)
        .header("Content-Type", "text/plain")
        .body(Body::from("INTERNAL SERVER ERROR"))
        .unwrap())
}

const TEMPLATE_BYTES: &[u8] = include_bytes!("../assets/yells_at.png");

lazy_static! {
    static ref HTTP_CLIENT: github::HttpClient = github::build_http_client();
    static ref TEMPLATE: image::DynamicImage = image::load_from_memory_with_format(TEMPLATE_BYTES, ImageFormat::Png).unwrap();
}

async fn generate(username: &str) -> Result<Option<image::DynamicImage>, BoxedError> {
    let client = github::Client::new(&HTTP_CLIENT);

    let avatar = match client.fetch_avatar(username).await? {
        Some(img) => img,
        None => {
            return Ok(None);
        }
    };

    let combined: Result<image::DynamicImage, BoxedError> = tokio::task::block_in_place(|| {
        let resized_avatar = avatar.thumbnail(60, 60);

        let mut yells = TEMPLATE.clone(); 

        imageops::overlay(&mut yells, &resized_avatar, 0, 0);

        Ok(yells)
    });

    Ok(Some(combined?))
}
