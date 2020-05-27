use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::future;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::service::Service;
use lazy_static::lazy_static;
use regex::Regex;
use slab::Slab;

type UserId = u64;
type UserDb = Arc<Mutex<Slab<UserData>>>;

const INDEX: &str = r#"
<!doctype html>
<html>
    <head>
        <title>Rust Microservice</title>
    </head>
    <body>
        <h3>Rust Microservice</h3>
    </body>
</html>
"#;

lazy_static! {
    static ref INDEX_PATH: Regex = Regex::new("^/(index\\.html?)?$").unwrap();
    static ref USER_PATH: Regex = Regex::new("^/user/((?P<user_id>\\d+?)/?)?$").unwrap();
    static ref USERS_PATH: Regex = Regex::new("^/users/?$").unwrap();
}

#[derive(Debug)]
struct UserData;

#[derive(Debug)]
pub struct MicroService {
    user_db: UserDb
}

pub struct MakeMicroService {
    user_db: UserDb
}

impl MicroService {
    fn new(user_db: UserDb) -> Self {
        MicroService {
            user_db
        }
    }
}

impl MakeMicroService {
    fn new(user_db: UserDb) -> Self {
        MakeMicroService {
            user_db
        }
    }
}

impl fmt::Display for UserData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("{}")
    }
}

impl Service<Request<Body>> for MicroService {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let response = {
            let method = req.method();
            let path = req.uri().path();
            let mut users = self.user_db.lock().unwrap();

            if INDEX_PATH.is_match(path) {
                if method == &Method::GET {
                    Response::new(INDEX.into())
                } else {
                    response_with_code(StatusCode::METHOD_NOT_ALLOWED)
                }
            } else if USERS_PATH.is_match(path) {
                if method == &Method::GET {
                    let list = users.iter()
                                            .map(|(id, _)| id.to_string())
                                            .collect::<Vec<String>>()
                                            .join(",");

                    Response::new(list.into())
                } else {
                    response_with_code(StatusCode::METHOD_NOT_ALLOWED)
                }
            } else if let Some(cap) = USER_PATH.captures(path) {
                let user_id = cap.name("user_id").and_then(|m| {
                    m.as_str()
                        .parse::<UserId>()
                        .ok()
                        .map(|x| x as usize)
                });

                match (method, user_id) {
                    (&Method::POST, None) => {
                        let id = users.insert(UserData);
                        Response::new(id.to_string().into())
                    },
                    (&Method::POST, Some(_)) => {
                        response_with_code(StatusCode::BAD_REQUEST)
                    },
                    (&Method::GET, Some(id)) => {
                        if let Some(data) = users.get(id) {
                            Response::new(data.to_string().into())
                        } else {
                            response_with_code(StatusCode::NOT_FOUND)
                        }
                    },
                    (&Method::PUT, Some(id)) => {
                        if let Some(data) = users.get_mut(id){
                            *data = UserData;
                            response_with_code(StatusCode::OK)
                        } else {
                            response_with_code(StatusCode::NOT_FOUND)
                        }
                    },
                    (&Method::DELETE, Some(id)) => {
                        if users.contains(id) {
                            users.remove(id);
                            response_with_code(StatusCode::OK)
                        } else {
                            response_with_code(StatusCode::NOT_FOUND)
                        }
                    },
                    _ => {
                        response_with_code(StatusCode::METHOD_NOT_ALLOWED)
                    }
                }
            } else {
                response_with_code(StatusCode::NOT_FOUND)
            }
        };

        future::ok(response)
    }
}

impl<T> Service<T> for MakeMicroService {
    type Response = MicroService;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(MicroService::new(self.user_db.clone()))
    }
}

#[tokio::main]
async fn main() {
    let user_db: UserDb = Arc::new(Mutex::new(Slab::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let server = Server::bind(&addr).serve(MakeMicroService::new(user_db));

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn response_with_code(status_code: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status_code)
        .body(Body::empty())
        .unwrap()
}