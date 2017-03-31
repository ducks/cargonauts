use std::marker::PhantomData;

use mainsail::{Resource, Error, Environment, Get, Index};
use tokio::NewService;
use tokio::stream::NewStreamService;

use http;
use service::{GetService, IndexService};
use receive::Receive;

pub trait Request<T: Resource>: Sized {
    type Service: Default;
    fn receive<R: Receive<T>>(rec: &R, req: http::Request) -> Result<Self, Error>;
}

pub trait ResourceRequest<T: Resource>: Request<T>
where
    <Self as Request<T>>::Service: NewService<Response = T, Error = Error>,
{ }

pub trait CollectionRequest<T: Resource>: Request<T>
where
    <Self as Request<T>>::Service: NewStreamService<Response = T, Error = Error>,
{ }

pub struct GetRequest<T: Resource> {
    pub identifier: T::Identifier,
    pub env: Environment,
}

impl<T: Get> Request<T> for GetRequest<T> {
    type Service = GetService<T>;

    fn receive<R: Receive<T>>(rec: &R, req: http::Request) -> Result<Self, Error> {
        let id = req.path().rsplit('/').next().ok_or(Error).and_then(|s| {
            s.parse().or(Err(Error))
        })?;
        rec.get(req, id)
    }
}

impl<T: Get> ResourceRequest<T> for GetRequest<T> { }

pub struct IndexRequest<T: Resource> {
    pub env: Environment,
    pub _spoopy: PhantomData<T>,
}

impl<T: Index> Request<T> for IndexRequest<T> {
    type Service = IndexService<T>;

    fn receive<R: Receive<T>>(rec: &R, req: http::Request) -> Result<Self, Error> {
        rec.index(req)
        
    }
}

impl<T: Index> CollectionRequest<T> for IndexRequest<T> { }
