pub mod logger_writer;

#[macro_export]
macro_rules! migrations {
    (@impl, $MIGRATIONS:expr) => {
        ::rocket::fairing::AdHoc::on_liftoff("Migrate Database", |rocket| {
            Box::pin(async move {
                let conn = Self::get_one(rocket).await.unwrap();
                let mut lines =
                    ::fins_migrations_fairing::logger_writer::LoggerWriter::lines("migrate_database", ::log::Level::Info);
                use diesel_migrations::MigrationHarness;
                conn.run(move |c| diesel_migrations::HarnessWithOutput::new(c, &mut lines).run_pending_migrations($MIGRATIONS).map(|_|())) // 
                    .await
                    .unwrap();
            })
        })
    };

    ($ty:ty) => {
        impl $ty {
            pub fn migrate() -> impl ::rocket::fairing::Fairing {
                pub const MIGRATIONS: diesel_migrations::EmbeddedMigrations = embed_migrations!();
                migrations!(@impl, MIGRATIONS)
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
