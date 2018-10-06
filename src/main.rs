#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
#[macro_use] extern crate rocket_contrib;
extern crate serde_json;

use rocket::Request;
use rocket::http::{Cookie, Status};
use rocket::local::Client;
use rocket::outcome::IntoOutcome;
use rocket::response::Responder;
use rocket::response::status;
use rocket_contrib::{Json};

#[derive(Debug)]
struct UserId(i32);

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for UserId {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> rocket::request::Outcome<UserId, ()>{
        println!("Cookies: {:?}", request.cookies());
        println!("Private cookie: {:?}", request.cookies().get_private("user_id"));
        request.cookies()
            .get_private("user_id")
            .ok_or(())
            .and_then(|cookie| cookie.value().parse().map_err(|_| ()))
            .map(|id| UserId(id))
            .into_outcome(Status::Unauthorized)
    }
}

#[derive(Debug)]
struct User {
    id: i32,
    name: String,
}

#[derive(Debug)]
enum SessionResponse {
    LoginFailed,
    LoginSucceeded(User),
}

impl<'r> Responder<'r> for SessionResponse {
    fn respond_to(self, req: &Request) -> rocket::response::Result<'r> {
        match self {
            SessionResponse::LoginFailed =>
                status::Custom(Status::Unauthorized, Json(json!({}))).respond_to(req),
            SessionResponse::LoginSucceeded(user) => {
                let mut cookies = req.cookies();
                cookies.remove_private(Cookie::named("user_id"));
                cookies.add_private(Cookie::new("user_id", format!("{}", user.id)));
                Json(json!({"user":user.name})).respond_to(req)
            }
        }
    }
}

#[post("/sessions")]
fn create_session() -> SessionResponse {
    SessionResponse::LoginSucceeded(User { id: 2, name: "Ethan".to_string() })
}

#[get("/whoami")]
fn whoami(user_id: UserId) -> String {
    format!("Got user id: {}", user_id.0)
}

fn main() {
    let my_rocket = rocket::ignite()
        .mount("/", routes![create_session])
        .mount("/", routes![whoami]);
    let client = Client::new(my_rocket).expect("rocket hrngh");
    let mut response = client
        .post("/sessions")
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.body_string(), Some("{\"user\":\"Ethan\"}".into()));
    let raw_cookie = response.headers().get_one("Set-Cookie").expect("login should return a cookie");
    let cookie = Cookie::parse(raw_cookie).expect("couldn't parse cookie");
    let mut response = client
        .get("/whoami")
        .cookie(cookie.into_owned())
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    let body = response.body_string().expect("response did not have a body");
    assert_eq!(body, "Got user id: 2");
}
