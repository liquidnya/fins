use rocket::catch;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::Request;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct JsonError {
    code: u16,
    message: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: JsonError,
}

#[catch(default)]
pub fn api_catcher(status: Status, _req: &Request) -> Json<ErrorResponse> {
    Json(ErrorResponse {
        error: JsonError {
            code: status.code,
            message: status.reason().unwrap_or("Something went wrong"),
        },
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
