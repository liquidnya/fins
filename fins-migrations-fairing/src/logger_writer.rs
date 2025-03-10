use log::{logger, Level, Metadata, Record};
use std::fmt;
use std::io;
use std::io::{LineWriter, Write};

pub struct LoggerWriter<'a> {
    metadata: Metadata<'a>,
}

impl Write for LoggerWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        std::str::from_utf8(buf)
            .map(|str| {
                let bytes = str.len();
                // trim newline at the end
                let str = str.strip_suffix('\n').unwrap_or(str);
                logger().log(
                    &Record::builder()
                        .args(format_args!("{}", str))
                        .metadata(self.metadata.clone())
                        .build(),
                );
                bytes
            })
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }
    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        if let Some(str) = fmt.as_str() {
            return self.write(str.as_bytes()).map(|_| ());
        }
        // TODO: trim newline at the end
        logger().log(
            &Record::builder()
                .args(fmt)
                .metadata(self.metadata.clone())
                .build(),
        );
        Ok(())
    }
    fn flush(&mut self) -> io::Result<()> {
        logger().flush();
        Ok(())
    }
}

pub type LoggerLineWriter<'a> = LineWriter<LoggerWriter<'a>>;

impl<'a> LoggerWriter<'a> {
    pub fn lines(target: &'a str, level: Level) -> LoggerLineWriter<'a> {
        LineWriter::new(LoggerWriter {
            metadata: Metadata::builder().level(level).target(target).build(),
        })
    }
}
