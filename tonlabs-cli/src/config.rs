
pub struct Config {
    pub url: String,
    pub wc: i8,
}

impl Config {
    pub fn new() -> Self {
        Config {
            url: "https://net.ton.dev".to_string(),
            wc: 0,
        }
    }
}