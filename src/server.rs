use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std;
use hyper::Result as HttpResult;
use hyper::server::{Http, Service, NewService, Request, Response, Server as HyperServer};

use middleware::MiddlewareStack;
use request;
use response;

use futures;
use futures::future::FutureResult;
use futures::IntoFuture;

use std::time::Duration;

use hyper;
use hyper::Body;
use tokio_core::reactor::Handle;
use futures::Future;

pub struct Server<D> {
    middleware_stack: MiddlewareStack<D>,
    templates: response::TemplateCache,
    shared_data: D,
}


// FIXME: Any better coherence solutions?
pub struct ArcServer<D>(Arc<Server<D>>);

impl<D: Sync + Send + 'static> Service for ArcServer<D> {


    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=hyper::Response, Error=hyper::Error>>;

    fn call<'a, 'k>(&'a self, req: Request) -> Self::Future {
        
        let i = self.0.clone();

        Box::new(request::RequestOrigin::from_internal(req).and_then(move |req_origin| {

            let mut res = hyper::Response::new();
            
            {
                let nickel_req = request::Request::from_internal(&req_origin,
                                                                &i.shared_data);
            

                let nickel_res = response::Response::from_internal(&mut res,
                                                                &i.templates,
                                                                &i.shared_data);

                i.middleware_stack.invoke(nickel_req, nickel_res);
            }

            futures::future::ok(res)

        }))

        // Box::new(futures::future::ok(hyper::Response::new()))


    }
}

impl<D: Sync + Send + 'static> NewService for ArcServer<D> {

    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = ArcServer<D>;
    
    fn new_service(&self) -> Result<Self::Instance, std::io::Error> {
        
        let cl: ArcServer<D> = ArcServer(self.0.clone());
        Ok(cl)
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
                                   keep_alive: bool,
                                   shutdown_timeout: Option<Duration>)
                                    -> Result<ListeningServer, hyper::Error> {
        let arc = ArcServer(Arc::new(self));

        let mut http = Http::new();

        http.keep_alive(keep_alive);
        
        let addr2 = addr.to_socket_addrs().unwrap().next().unwrap();

        match http.bind(&addr2, arc) {
            Ok(mut server) => {
                let local_addr = server.local_addr();
                if shutdown_timeout.is_some() {
                    server.shutdown_timeout(shutdown_timeout.unwrap());
                }
                let listening_server = ListeningServer {
                    local_addr: local_addr.unwrap(),
                    handle: server.handle()
                };
                server.run().unwrap();
                Ok(listening_server)
            },
            Err(err) => {
                Err(err)
            }
        }
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

/// A server listening on a socket
pub struct ListeningServer {
    pub local_addr: SocketAddr,
    pub handle: Handle
}