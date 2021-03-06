use futures::Future;

use rigging::{Error, http};
use rigging::resource::Resource;
use rigging::environment::Environment;
use rigging::method::{Method, ResourceMethod};
use rigging::routes::{Route, Kind};

/// Update an instance of this resource.
///
/// This method corresponds to `PATCH /$resource-type/$identifier`.
pub trait Patch: Resource {
    /// The representation of this resource received with the Patch request.
    type Patch;
    fn patch(id: Self::Identifier, patch: Self::Patch, env: Environment) -> Box<Future<Item = Self, Error = Error>> where Self: Sized;
}

impl<T: Patch> Method<T> for Patch<Identifier = T::Identifier, Patch = T::Patch> {
    const ROUTE: Route = Route {
        kind: Kind::Resource,
        method: http::Method::Patch,
    };

    type Request = T::Patch;
    type Response = T;
    type Future = Box<Future<Item = T, Error = Error>>;
}

impl<T: Patch> ResourceMethod<T> for Patch<Identifier = T::Identifier, Patch = T::Patch> {
    fn call(id: T::Identifier, req: Self::Request, env: Environment) -> Self::Future {
        T::patch(id, req, env)
    }
}
