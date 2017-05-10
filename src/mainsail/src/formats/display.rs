use std::fmt::{self, Debug, Display};

use futures::{Future, future};

use rigging::resource::ResourceEndpoint;
use rigging::Error;
use rigging::environment::Environment;
use rigging::http;
use rigging::format::*;
use rigging::method::Method;

const MIME: &'static str = "text/plain; charset=utf-8";

#[derive(Default, Clone)]
pub struct SimpleDisplay {
    _private: (),
}

impl<T, R, M> Format<T, R, M> for SimpleDisplay
where
    T: ResourceEndpoint,
    M: ?Sized + Method<T, Request = ()>,
    R: Display,
{
    type ReqFuture = future::FutureResult<(), Error>;

    fn receive_request(_: http::Request, _: &mut Environment) -> Self::ReqFuture {
        future::ok(())
    }

    fn present_unit(future: M::Future, _: Option<Template>, _: &mut Environment) -> http::BoxFuture
        where M: Method<T, Response = ()>
    {
        Box::new(future.then(|result| match result {
            Ok(())  => Ok(http::Response::new().with_status(http::StatusCode::NoContent)),
            Err(e)  => Ok(debug_response(e, http::StatusCode::InternalServerError)),
        }))
    }

    fn present_resource(future: M::Future, _: Option<Template>, _: &mut Environment) -> http::BoxFuture
        where M: Method<T, Response = R>, R: ResourceEndpoint
    {
        Box::new(future.then(|result| match result {
            Ok(resource)    => Ok(display_response(resource, http::StatusCode::Ok)),
            Err(e)          => Ok(debug_response(e, http::StatusCode::InternalServerError)),
        }))
    }

    fn present_collection(future: M::Future, _: Option<Template>, _: &mut Environment) -> http::BoxFuture
        where M: Method<T, Response = Vec<R>>, R: ResourceEndpoint
    {
        Box::new(future.then(|result| match result {
            Ok(resources)   => Ok(display_response(Newline(resources), http::StatusCode::Ok)),
            Err(e)          => Ok(debug_response(e, http::StatusCode::InternalServerError)),
        }))
    }

    fn present_error(error: Error, _: &mut Environment) -> http::BoxFuture {
        Box::new(future::ok(debug_response(error, http::StatusCode::InternalServerError)))
    }
}


fn display_response<T: Display>(t: T, status: http::StatusCode) -> http::Response {
    let data = format!("{}", t);
    http::Response::new().with_status(status)
                         .with_header(http::headers::ContentLength(data.len() as u64))
                         .with_header(http::headers::ContentType(MIME.parse().unwrap()))
                         .with_body(data)
}

fn debug_response<T: Debug>(t: T, status: http::StatusCode) -> http::Response {
    let data = format!("{:?}", t);
    http::Response::new().with_status(status)
                         .with_header(http::headers::ContentLength(data.len() as u64))
                         .with_header(http::headers::ContentType(MIME.parse().unwrap()))
                         .with_body(data)
}

struct Newline<T>(Vec<T>);

impl<T: Display> Display for Newline<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for elem in &self.0 {
            writeln!(f, "{}", elem)?;
        }
        Ok(())
    }
}
