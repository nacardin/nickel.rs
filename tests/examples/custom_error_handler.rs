use util::*;

use hyper::StatusCode;
use hyper::client::Response;

fn with_path<F>(path: &str, f: F) where F: FnOnce(Response) {
    run_example("custom_error_handler", |port| {
        let url = format!("http://localhost:{}{}", port, path);
        let res = response_for(&url);
        f(res)
    })
}

#[test]
fn accepts_some_inputs() {
    with_path("/user/42", |res| {
        assert_eq!(res.status(), StatusCode::Ok);
        let s = read_body_to_string(res);
        assert_eq!(s, "User 42 was found!");
    })
}

#[test]
fn has_custom_message_for_custom_error() {
    with_path("/user/19", |res| {
        assert_eq!(res.status(), StatusCode::ImATeapot);
        let s = read_body_to_string(res);
        assert_eq!(s, "Teapot activated!");
    });
}

#[test]
fn has_custom_message_for_fallthrough() {
    with_path("/not_a_handled_path", |res| {
        assert_eq!(res.status(), StatusCode::NotFound);
        let s = read_body_to_string(res);
        assert_eq!(s, "<h1>404 - Not Found</h1>");
    })
}
