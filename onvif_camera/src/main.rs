use onvif_camera::run;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut verbose = false;
    for arg in args {
        if arg == "-v" || arg == "--verbose" {
            verbose = true;
        }
    }
    // Also support VERBOSE env var
    if env::var("VERBOSE").is_ok() {
        verbose = true;
    }

    run(8080, verbose).await;
}
