use clap::Parser;
use hermit::{cli, constants, router, spec_parser};

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();
    args.validate().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });
    let spec = spec_parser::load(&args.specs);
    let routes = spec_parser::extract_routes(&spec);
    let app = router::build_with_bounds(routes, args.min_items, args.max_items);

    let addr = (constants::BIND_ADDR, args.port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
