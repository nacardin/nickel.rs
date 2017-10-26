// This attribute being conditional is an implementation detail of the nickel
// test setup for testing examples. Usually, it should just require `#[macro_use]`.
#[cfg_attr(not(test), macro_use)]
extern crate nickel;
extern crate hyper;
extern crate rustc_serialize;
extern crate futures;

use nickel::{Nickel, ListeningServer, HttpRouter, JsonBody};
use nickel::status::StatusCode;

use rustc_serialize::json::Json;

use std::error::Error as StdError;
use std::{env, str};
use std::sync::atomic::{self, AtomicUsize};

// Example trait to allow custom databases to be used within the integration tests
// to test different behaviours without touching a real database.
trait Database : Send + Sync + 'static {
    fn get_users(&self) -> Vec<String>;
}

impl Database for Vec<String> {
    fn get_users(&self) -> Vec<String> {
        self.clone()
    }
}

struct ServerData {
    hits: AtomicUsize,

    // FIXME: The `middleware` macro typehinting doesn't support hinting with
    // typeparams, i.e. the hint can't be `< ServerData<T> >` where T is a
    // typeparam from the enclosing function, e.g. `start_server`.
    //
    // To counter this limitation, we've boxed the trait so that specifying the
    // typeparam is unnecessary.
    database: Box<Database>
}

impl ServerData {
    fn hitcount(&self) -> usize {
        self.hits.load(atomic::Ordering::Relaxed)
    }

    fn log_hit(&self) -> usize {
        self.hits.fetch_add(1, atomic::Ordering::Relaxed)
    }

    fn get_users(&self) -> Vec<String> {
        self.database.get_users()
    }
}

fn main() {
    let port = env::var("PORT").map(|s| s.parse().unwrap()).unwrap_or(3000);
    let address = &format!("0.0.0.0:{}", port);
    let database = vec![];

    start_server(address, database).unwrap();
}

fn start_server<D>(address: &str, database: D) -> Result<ListeningServer, Box<StdError>>
where D: Database {
    let server_data = ServerData {
        hits: AtomicUsize::new(0),
        database: Box::new(database),
    };

    let mut server = Nickel::with_data(server_data);

    // Track all hits to the server
    server.utilize(middleware! { |_req, res| <ServerData>
        res.data().log_hit();
        return res.next_middleware()
    });

    // Core server
    server.get("/", middleware!( "Hello World" ));

    server.get("/users", middleware! { |_, res| <ServerData>
        let users = res.data().get_users();

        Json::from_str(&format!(r#"{{ "users": {:?} }}"#, users)).unwrap()
    });

    // Json example
    server.post("/", middleware! { |req, res|
        #[derive(RustcEncodable, RustcDecodable)]
        struct Data { name: String, age: Option<u32> }

        let client = try_with!(res, req.json_as::<Data>().map_err(|e| (StatusCode::BadRequest, e)));

        match client.age {
            Some(age) => {
                Json::from_str(&format!(
                    r#"{{ "message": "Hello {}, your age is {}" }}"#,
                    client.name,
                    age
                )).unwrap()
            }
            None => {
                Json::from_str(&format!(
                    r#"{{ "message": "Hello {}, I don't know your age" }}"#,
                    client.name
                )).unwrap()
            }
        }
    });

    // Get the hitcount
    server.get("/hits", middleware!{ |_req, res| <ServerData>
        res.data().hitcount().to_string()
    });

    server.listen(address)
}

#[cfg(test)]
mod tests {
    use self::support::{Server, STATIC_SERVER, get, post};

    use hyper::header;
    use nickel::status::StatusCode;
    use rustc_serialize::json::Json;

    use futures::{Future, Stream};

    use std::{thread, time};

    fn get_hits_after_delay(server: &Server) -> u32 {
        // let other tests hit the server
        thread::sleep(time::Duration::from_secs(1));

        let response = server.get("/hits");
        response.body();
        0
    }

    #[test]
    fn root_responds_with_hello_world() {
        let response = get("/");

        // assert_eq!(response.body(), "Hello World");
        assert_eq!(response.status(), StatusCode::Ok);
    }

    #[test]
    // FIXME: This will probably fail if tests are run without parallelism
    fn server_is_shared_with_other_tests() {
        assert!(get_hits_after_delay(&STATIC_SERVER) > 1);
    }

    #[test]
    fn root_responds_with_modified_json() {
        let response = post("/", r#"{ "name": "Rust", "age": 1 }"#);

        let chunk = response.body().concat2().wait().unwrap();
        let s = String::from_utf8(chunk.to_vec()).unwrap();
        let json = Json::from_str(&s).unwrap();

        assert_eq!(json["message"].as_string(), Some("Hello Rust, your age is 1"));
        // assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(
            response.headers().get::<header::ContentType>(),
            Some(&header::ContentType::json())
        );
    }

    #[test]
    fn accepts_json_with_missing_fields() {
        let response = post("/", r#"{ "name": "Rust" }"#);

        let chunk = response.body().concat2().wait().unwrap();
        let s = String::from_utf8(chunk.to_vec()).unwrap();
        let json = Json::from_str(&s).unwrap();

        assert_eq!(json["message"].as_string(), Some("Hello Rust, I don't know your age"));
        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(
            response.headers().get::<header::ContentType>(),
            Some(&header::ContentType::json())
        );
    }

    #[test]
    fn doesnt_accept_bad_inputs() {
        let response = post("/", r#"{ }"#);
        assert_eq!(response.status(), StatusCode::BadRequest);
    }

    /// This test has a Server instance all to itself.
    #[test]
    fn non_shared_server() {
        let test_local_server = {
            let server = super::start_server("127.0.0.1:0", vec![]).unwrap();
            Server::new(server)
        };

        assert_eq!(get_hits_after_delay(&test_local_server), 1);
    }

    #[test]
    fn has_no_users_by_default() {
        let response = get("/users");

        let chunk = response.body().concat2().wait().unwrap();
        let s = String::from_utf8(chunk.to_vec()).unwrap();
        let json = Json::from_str(&s).unwrap();

        assert_eq!(json["users"].as_array().unwrap().len(), 0);
        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(
            response.headers().get::<header::ContentType>(),
            Some(&header::ContentType::json())
        );
    }

    #[test]
    fn non_shared_server_with_different_database() {
        let server = {
            let bots = vec!["bors".into(), "homu".into(), "highfive".into()];
            let server = super::start_server("127.0.0.1:0", bots).unwrap();
            Server::new(server)
        };

        let response = server.get("/users");

        let chunk = response.body().concat2().wait().unwrap();
        let s = String::from_utf8(chunk.to_vec()).unwrap();
        let json = Json::from_str(&s).unwrap();

        assert_eq!(json["users"].as_array().unwrap().len(), 3);
        assert_eq!(response.status(), StatusCode::Ok);
        assert_eq!(
            response.headers().get::<header::ContentType>(),
            Some(&header::ContentType::json())
        );
    }

    mod support {
        use hyper::client::{Client, Response as HyperResponse, Request as HyperRequest};
        use hyper::{Method, Uri};
        use tokio_core::reactor::Core;
        use nickel::ListeningServer;

        use std::net::SocketAddr;

        use futures::{Future, Stream};
        use std::str::FromStr;

        /// An example wrapper type to make testing more readable
        pub struct Server(SocketAddr);
        impl Server {
            pub fn new(server: ListeningServer) -> Server {
                let wrapped = Server(server.socket());

                // detaching is important otherwise it would block the test threads.
                server.detach();

                wrapped
            }

            pub fn get(&self, path: &str) -> HyperResponse {
                let url = self.url_for(path);

                let mut core = Core::new().unwrap();

                Client::new(&core.handle())
                    .get(Uri::from_str(&url).unwrap())
                    .wait()
                    .unwrap()
            }

            pub fn post(&self, path: &str, body: &str) -> HyperResponse {
                let url = self.url_for(path);

                let mut core = Core::new().unwrap();

                let mut request = HyperRequest::new(Method::Post, Uri::from_str(&url).unwrap());
                request.set_body(body.to_owned());

                Client::new(&core.handle())
                    .request(request)
                    .wait()
                    .unwrap()
            }

            pub fn url_for(&self, path: &str) -> String {
                format!("http://{}{}", self.0, path)
            }
        }

        lazy_static! {
            /// This is a shared instance of the server between all the tests
            pub static ref STATIC_SERVER: Server = {
                let server = super::super::start_server("127.0.0.1:0", vec![]).unwrap();
                Server::new(server)
            };
        }

        /// Example of a free function version of `get` which uses the shared server
        pub fn get(path: &str) -> HyperResponse {
            STATIC_SERVER.get(path)
        }

        /// Example of a free function version of `post` which uses the shared server
        pub fn post(path: &str, body: &str) -> HyperResponse {
            STATIC_SERVER.post(path, body)
        }
    }
}
