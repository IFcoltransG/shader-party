use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, about, long_about = None)]
pub(crate) struct Config {
    #[clap(short, long, value_parser, default_value = "./src/shader.wgsl")]
    pub path: String,
}
