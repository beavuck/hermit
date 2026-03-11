use clap::Parser;
use hermit::{cli, constants, router, spec_parser};

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();
    args.validate().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });
    hermit::resource_generator::set_ignore_examples(args.ignore_examples);
    let routes = spec_parser::load_all(&args.specs);
    let app = router::build_with_bounds(routes, args.min_items, args.max_items)
        .layer(axum::middleware::from_fn(log_request));

    let addr = (constants::BIND_ADDR, args.port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    print_banner(args.port);
    axum::serve(listener, app).await.unwrap();
}

async fn log_request(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let now = chrono::Local::now();
    let response = next.run(req).await;
    println!(
        "{} {} {} --> {}",
        now.format("%Y-%m-%d %H:%M:%S%.3f"),
        method,
        path,
        response.status()
    );
    response
}

fn print_banner(port: u16) {
    println!(
        r#"
| |                               | |                                          ,=.
| |__   ___  __ ___   ___   _  ___| | __                        ,=""""==.__.="  o".___
| '_ \ / _ \/ _` \ \ / / | | |/ __| |/ /                  ,=.=="                  ___/
| |_) |  __/ (_| |\ V /| |_| | (__|   <             ,==.,"    ,          , \,===""
|_.__/ \___|\__,_| \_/  \__,_|\___|_|\_\          <     ,==)  \"'"=._.==)  \
                                                    `==''    `"           `"
                   __
                .-(  )-.
               (  (  )  )
              (   (  )   )
              ..//(o o)\\..                       hermit

 Ready on port {port}
"#
    );
}
