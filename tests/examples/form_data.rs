use util::*;
use hyper::Client;
use tokio_core::reactor::Core;

use hyper::Method;
use hyper::Request;
use hyper::Uri;
use std::str::FromStr;
use futures::future::Future;
use futures::Stream;

use hyper::header::ContentType;
use hyper::StatusCode;
use util::{read_body_to_string, read_url, run_example};

#[test]
fn display_form() {
    run_example("form_data", |port| {
        let url = format!("http://localhost:{}/", port);
        let s = read_url(&url);
        assert!(s.contains(r#"<form>"#), "response didn't have a form");
    })
}

#[test]
fn post_with_data() {
    run_example("form_data", |port| {
        let url = format!("http://localhost:{}/confirmation", port);

        let mut core = Core::new().unwrap();

        let mut request = Request::new(Method::Post, Uri::from_str(&url).unwrap());
        request.set_body(r#"firstname=John&lastname=Doe&phone=911&email=john@doe.com"#.to_owned());
        request.headers_mut().set(ContentType::form_url_encoded());

        let res = Client::new(&core.handle())
            .request(request)
            .wait()
            .unwrap();

        let s = read_body_to_string(&mut res);
        assert!(s.contains(r#"John Doe 911 john@doe.com"#), "response didn't have an expected data");
    })
}

#[test]
fn post_without_data() {
    run_example("form_data", |port| {
        let url = format!("http://localhost:{}/confirmation", port);

        let mut core = Core::new().unwrap();

        let mut request = Request::new(Method::Post, Uri::from_str(&url).unwrap());
        request.set_body(r#"firstname=John&lastname=Doe&phone=911&email=john@doe.com"#.to_owned());
        request.headers_mut().set(ContentType::form_url_encoded());

        let res = Client::new(&core.handle())
            .request(request)
            .wait()
            .unwrap();

        let s = read_body_to_string(&mut res);
        assert!(s.contains(r#"First name? Last name? Phone? Email?"#), "response didn't have an expected data");
    })
}

#[test]
fn post_without_content_type() {
    run_example("form_data", |port| {
        let url = format!("http://localhost:{}/confirmation", port);
        let res = response_for_method(Method::Get, &url);
        assert_eq!(res.status(), StatusCode::BadRequest);
    })
}
