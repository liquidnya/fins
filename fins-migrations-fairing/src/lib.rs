pub mod logger_writer;

#[macro_export]
macro_rules! migrations {
    (@impl) => {
        ::rocket::fairing::AdHoc::on_liftoff("Migrate Database", |rocket| {
            Box::pin(async move {
                let conn = Self::get_one(rocket).await.unwrap();
                let mut lines =
                    ::fins_migrations_fairing::logger_writer::LoggerWriter::lines("migrate_database", ::log::Level::Info);
                conn.run(move |c| embedded_migrations::run_with_output(c, &mut lines))
                    .await
                    .unwrap();
            })
        })
    };

    ($ty:ty) => {
        impl $ty {
            pub fn migrate() -> impl ::rocket::fairing::Fairing {
                ::diesel_migrations::embed_migrations!();
                migrations!(@impl)
            }
        }
    };

    ($ty:ty, $migrations_path:expr) => {
        impl $ty {
            pub fn migrate() -> impl ::rocket::fairing::Fairing {
                ::diesel_migrations::embed_migrations!($migrations_path);
                migrations!(@impl)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
