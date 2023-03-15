use lightning::util::logger::{Logger, Record};

pub struct MyLogger {
    // data_dir: String,
}

impl Logger for MyLogger {
    fn log(&self, record: &Record) {
        let raw_log = record.args.to_string();
        let log = format!(
            "{:<5} [{}:{}] {}\n",
            // OffsetDateTime::now_utc().format("%F %T"),
            record.level.to_string(),
            record.module_path,
            record.line,
            raw_log
        );
        dbg!(log);
    }
}
