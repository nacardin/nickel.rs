use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::Duration;
use hyper::Result as HttpResult;
use hyper::server::{Http, Service, Request, Response, Server as HyperServer};

use middleware::MiddlewareStack;
use request;
use response;

pub struct Server<D> {
    middleware_stack: MiddlewareStack<D>,
    templates: response::TemplateCache,
    shared_data: D,
}


// FIXME: Any better coherence solutions?
struct ArcServer<D>(Arc<Server<D>>);

impl<D: Sync + Send + 'static> Service for ArcServer<D> {
    fn call<'a, 'k>(&'a self, req: Request<'a, 'k>, res: Response<'a>) {
        let nickel_req = request::Request::from_internal(req,
                                                         &self.0.shared_data);

        let nickel_res = response::Response::from_internal(res,
                                                           &self.0.templates,
                                                           &self.0.shared_data);

        self.0.middleware_stack.invoke(nickel_req, nickel_res);
    }
}

impl<D: Sync + Send + 'static> Server<D> {
    pub fn new(middleware_stack: MiddlewareStack<D>, data: D) -> Server<D> {
        Server {
            middleware_stack: middleware_stack,
            templates: RwLock::new(HashMap::new()),
            shared_data: data
        }
    }

    pub fn serve<A: ToSocketAddrs>(self,
                                   addr: A,
                                   keep_alive_timeout: Option<Duration>,
                                   thread_count: Option<usize>)
                                    -> HttpResult<ListeningServer> {
        let arc = ArcServer(Arc::new(self));

        let server = Http::new().bind(&addr, arc).unwrap();

        server.keep_alive(keep_alive_timeout);

        // let listening = match thread_count {
        //     Some(threads) => server.handle_threads(arc, threads),
        //     None => server.handle(arc),
        // };

        ListeningServer::<D>(server)
    }

    //TODO: SSL
    // pub fn serve_https<A,S>(self,
    //                         addr: A,
    //                         keep_alive_timeout: Option<Duration>,
    //                         thread_count: Option<usize>,
    //                         ssl: S)
    //                         -> HttpResult<ListeningServer>
    //     where A: ToSocketAddrs,
    //           S: SslServer + Clone + Send + 'static {
    //     let arc = ArcServer(Arc::new(self));
    //     let mut server = try!(HyperServer::https(addr, ssl));

    //     server.keep_alive(keep_alive_timeout);

    //     let listening = match thread_count {
    //         Some(threads) => server.handle_threads(arc, threads),
    //         None => server.handle(arc),
    //     };

    //     listening.map(ListeningServer)
    // }
}
use std::any::Any;
/// A server listeing on a socket
pub struct ListeningServer<D>(HyperServer<ArcServer<D>, Any>);

impl ListeningServer<Any> {
    /// Gets the `SocketAddr` which the server is currently listening on.
    pub fn socket(&self) -> SocketAddr {
        self.0.socket
    }

    /// Detaches the server thread.
    ///
    /// This doesn't actually kill the server, it just stops the current thread from
    /// blocking due to the server running. In the case where `main` returns due to
    /// this unblocking, then the server will be killed due to process death.
    ///
    /// The required use of this is when writing unit tests which spawn servers and do
    /// not want to block the test-runner by waiting on the server to stop because
    /// it probably never will.
    ///
    /// See [this hyper issue](https://github.com/hyperium/hyper/issues/338) for more
    /// information.
    pub fn detach(self) {
        // We want this handle to be dropped without joining.
        let _ = ::std::thread::spawn(move || {
            // This will hang the spawned thread.
            // See: https://github.com/hyperium/hyper/issues/338
            let _ = self.0;
        });
    }
}
