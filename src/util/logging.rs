use log::LevelFilter;
use simple_logger::SimpleLogger;

pub fn setup_logging() {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();
}
