#[allow(resolve_trait_on_defaulted_unit)] // https://github.com/nickel-org/nickel.rs/issues/399
extern crate rustc_serialize;
extern crate hyper;
// HACK: integration_testing example refers to `nickel::foo`
// and this import helps that resolve rather than requiring `self::nickel::foo`
// which is an oddity due to the include method, which is used as tests in examples
// don't get executed otherwise.
#[macro_use]
extern crate nickel;

#[macro_use] extern crate lazy_static;

extern crate unicase;
extern crate tokio_core;
extern crate futures;

mod util;

mod examples {
    mod hello_world;
    // mod mount;
    mod route_data;
    mod routing;
    // mod template;
    mod moved_ownership;
    mod chaining;
    mod json;
    mod query_string;
    mod regex_route;
    mod custom_error_handler;
    mod static_files;
    mod enable_cors;
    mod form_data;
    mod integration_testing;

    // #[cfg(feature = "ssl")]
    // mod https;
}
