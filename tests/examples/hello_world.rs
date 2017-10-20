use util::{run_example, read_url};
use futures::future::Future;
use hyper;

fn t(file: &str) {
    run_example(file, |port| {
        let paths = ["", "foo", "bar.html", "foo-barrrr/baz"];

        for path in &paths {
            let url = format!("http://localhost:{}/{}", port, path);

            type asd = Box<Future<Item = String, Error = hyper::Error>>;

            let rurl: asd = read_url(&url);
            
            rurl.then(|s| -> Result<(),()> {

                assert_eq!(s.unwrap(), "Hello World");
                Ok(())

            });
        }
    })
}

#[test] fn non_macro() { t("hello_world") }
#[test] fn _macro() { t("hello_world_macro") }
