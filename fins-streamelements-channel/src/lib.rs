use rocket::http::Status;
use rocket::request::Outcome;
use rocket::request::{self, FromRequest, Request};
use std::fmt::Display;
use thiserror::Error;

const STREAMELEMENTS_HEADER: &str = "x-streamelements-channel";
const STREAMELEMENTS_HEADER_LEN: usize = 24; // 24 bytes (not 24 characters), but since they are all ASCII it works

#[derive(Debug, PartialEq)]
pub struct Channel<'a>(&'a str);

impl<'a> AsRef<str> for Channel<'a> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> Channel<'a> {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl Display for Channel<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ChannelError {
    #[error("expected header `{STREAMELEMENTS_HEADER}`")]
    Missing,
    #[error(transparent)]
    Parsing(#[from] ChannelParseError),
}

#[derive(Error, Debug, PartialEq)]
pub enum ChannelParseError {
    #[error("invalid channel length (expected {STREAMELEMENTS_HEADER_LEN}, found {0})")]
    Length(usize),
    #[error("invalid characters")]
    InvalidCharacters,
}

fn is_lowercase_ascii_hexdigit(c: char) -> bool {
    c.is_ascii_hexdigit() && (c.is_ascii_lowercase() || c.is_ascii_digit())
}

impl<'a> TryFrom<&'a str> for Channel<'a> {
    type Error = ChannelParseError;

    fn try_from(channel: &'a str) -> Result<Self, Self::Error> {
        if channel.len() != STREAMELEMENTS_HEADER_LEN {
            Err(ChannelParseError::Length(channel.len()))
        } else if !channel.chars().all(is_lowercase_ascii_hexdigit) {
            Err(ChannelParseError::InvalidCharacters)
        } else {
            Ok(Channel(channel))
        }
    }
}

impl<'a> From<&'a Channel<'a>> for rocket::http::Header<'a> {
    fn from(channel: &'a Channel<'a>) -> Self {
        rocket::http::Header::new(STREAMELEMENTS_HEADER, channel.as_ref())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Channel<'r> {
    type Error = ChannelError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match req.headers().get_one(STREAMELEMENTS_HEADER) {
            None => Outcome::Failure((Status::BadRequest, ChannelError::Missing)),
            Some(channel) => match channel.try_into() {
                Ok(channel) => Outcome::Success(channel),
                Err(e) => Outcome::Failure((Status::BadRequest, e.into())),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Channel, ChannelParseError};
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[test]
    fn valid() {
        let result: Result<Channel, _> = "0123456789abcdef00000000".try_into();
        assert_eq!(result, Ok(Channel("0123456789abcdef00000000")));
    }

    #[test]
    fn invalid_characters() {
        let result: Result<Channel, _> = "A00000000000000000000000".try_into();
        assert_eq!(result, Err(ChannelParseError::InvalidCharacters));
        let result: Result<Channel, _> = "z00000000000000000000000".try_into();
        assert_eq!(result, Err(ChannelParseError::InvalidCharacters));
        let result: Result<Channel, _> = "Ä0000000000000000000000".try_into();
        assert_eq!(result, Err(ChannelParseError::InvalidCharacters));
    }

    #[test]
    fn invalid_length() {
        // too short
        let result: Result<Channel, _> = "a".try_into();
        assert_eq!(result, Err(ChannelParseError::Length(1)));
        // too long
        let result: Result<Channel, _> = "a00000000000000000000000a".try_into();
        assert_eq!(result, Err(ChannelParseError::Length(25)));
        // size is in bytes and not characters
        let result: Result<Channel, _> = "Ä00000000000000000000000".try_into();
        assert_eq!(result, Err(ChannelParseError::Length(25)));
    }

    #[test]
    fn request() {
        #[get("/channel")]
        fn channel(channel: Channel) -> String {
            channel.to_string()
        }

        let rocket = rocket::build().mount("/", routes![channel]);
        let client = Client::tracked(rocket).unwrap();

        // no header
        let req = client.get("/channel");
        let response = req.dispatch();
        assert_eq!(response.status(), Status::BadRequest);

        // invalid header
        let req = client.get("/channel").header(&Channel("abc"));
        let response = req.dispatch();
        assert_eq!(response.status(), Status::BadRequest);

        // valid header
        let req = client
            .get("/channel")
            .header(&Channel("0123456789abcdef00000000"));
        let response = req.dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "0123456789abcdef00000000");
    }
}
