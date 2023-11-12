use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config<'a> {
    /// Connection URL specified in the Rocket configuration.
    pub url: ::rocket::http::uri::Absolute<'a>,
    /*
    /// Initial pool size. Defaults to the number of Rocket workers * 4.
    pub pool_size: u32,
    /// How long to wait, in seconds, for a new connection before timing out.
    /// Defaults to `5`.
    // FIXME: Use `time`.
    pub timeout: u8,
    */
}

pub fn create_client(config: Config) -> (::reqwest::Client, ::rocket::http::uri::Absolute) {
    (::reqwest::Client::new(), config.url)
}

#[macro_export]
macro_rules! client {
    (#[client($client_name:literal)] $struct_vis:vis struct $client:ident { $(#[$method:ident($($args:tt)*)] $fn_vis:vis fn $fn:ident( $($ident:ident:$ty:ty),* ) -> $ret:ty;)* }) => {
        $struct_vis struct $client { #[allow(dead_code)] client: ::reqwest::Client, #[allow(dead_code)] url: ::rocket::http::uri::Absolute<'static> }

        impl $client {
            #[allow(dead_code)]
            pub fn new(url: ::rocket::http::uri::Absolute<'static>) -> Self {
                Self { client: ::reqwest::Client::new(), url }
            }

            #[allow(dead_code)]
            pub fn with_client(client: ::reqwest::Client, url: ::rocket::http::uri::Absolute<'static>) -> Self {
                Self { client, url }
            }

            pub fn fairing() -> impl ::rocket::fairing::Fairing {
                ::rocket::fairing::AdHoc::try_on_ignite(concat!("'", $client_name, "' HTTP Client"), move |rocket| async move {
                    match ::tokio::task::spawn_blocking(move || {
                        let config = match rocket.figment().extract_inner(concat!("clients.", $client_name)) {
                            Ok(config) => config,
                            Err(e) => {
                                ::log::error!("client config error for client named `{}`", $client_name);
                                ::log::error!("{}", e);
                                return Err(rocket);
                            },
                        };

                        let (client, url) = ::fins_rest_client::create_client(config);

                        Ok(rocket.manage($client::with_client(client, url)))
                    }).await {
                        Ok(result) => result,
                        Err(e) => {
                            ::std::panic::resume_unwind(e.try_into_panic().unwrap())
                        },
                    }
                })
            }

            $(
                $fn_vis async fn $fn(&self, $($ident:$ty),*) -> Result<$ret, ::reqwest::Error> {
                    #[allow(dead_code, unused_variables)]
                    #[$method($($args)*)]
                    fn inner($($ident:$ty),*) { unimplemented!("inner rest api call") }
                    // TODO: why is prefix consumed and not passed by reference?
                    // TODO: header support etc.
                    // TODO: does not have to be json
                    self.client
                        .$method({ uri!(self.url.clone(), inner($($ident),*)) }
                        .to_string())
                        .send()
                        .await?
                        .json()
                        .await
                }
            )*
        }

        impl ::rocket::Sentinel for $client {
            fn abort(rocket: &::rocket::Rocket<::rocket::Ignite>) -> bool {
                if rocket.state::<$client>().is_none() {
                    ::log::error!("requesting `{0}` HTTP client without attaching `{0}::fairing()`.", ::std::any::type_name::<Self>());
                    ::log::info!("Attach `{}::fairing()` to use the HTTP client.", ::std::any::type_name::<Self>());
                    return true;
                }

                false
            }
        }

        #[rocket::async_trait]
        impl<'r> ::rocket::request::FromRequest<'r> for &'r $client {
            type Error = ();

            async fn from_request(
                req: &'r ::rocket::request::Request<'_>,
            ) -> ::rocket::request::Outcome<Self, Self::Error> {
                match req.rocket().state::<$client>() {
                    Some(c) => ::rocket::request::Outcome::Success(c),
                    None => {
                        ::log::error!("Missing client fairing for `{}`", ::std::any::type_name::<Self>());
                        ::rocket::request::Outcome::Error((::rocket::http::Status::InternalServerError, ()))
                    }
                }
            }
        }



    };
}
