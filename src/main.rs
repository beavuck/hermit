use clap::Parser;
use hermit::{cli, constants, router, spec};

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();
    let spec = spec::load(&args.specs);
    let routes = spec::extract_routes(&spec);
    let app = router::build(routes);

    let addr = (constants::BIND_ADDR, args.port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
