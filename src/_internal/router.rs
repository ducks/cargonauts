use api;
use from_value;
use Serialize;
use router::Router as RouterTrait;
use router::{PostObject, Status, Response};
use _internal::{Resource, Wrapper};
use _internal::document::{CollectionDocument, ResourceDocument};

pub struct Router<'a, R: RouterTrait + 'a> {
    router: &'a mut R,
    base_url: &'static str,
}

impl<'a, R: RouterTrait> Router<'a, R> {
    pub fn new(router: &'a mut R) -> Router<'a, R> {
        let base_url = router.base_url();
        Router {
            router: router,
            base_url: base_url,
        }
    }

    pub fn attach_get<T: api::Get>(&mut self) where Resource<T>: Wrapper<T> {
        let Router { ref mut router, base_url } = *self;
        router.attach_get(T::resource(), |get_object| {
            let mut response = R::Response::default();
            let id = match get_object.id.parse() {
                Ok(id)  => id,
                Err(_)  => {
                    response.set_status(Status::BadRequest);
                    return response
                }
            };
            if let Some(resource) = T::get(id) {
                let document = ResourceDocument::new(resource, &get_object.includes, base_url);
                respond_with(document, response)
            } else {
                // TODO write the error to the body in the error case
                response.set_status(Status::NotFound);
                response
            }
        });
    }

    pub fn attach_index<T: api::Index>(&mut self) where Resource<T>: Wrapper<T> {
        let Router { ref mut router, base_url } = *self;
        router.attach_index(T::resource(), |index_object| {
            let mut response = R::Response::default();
            let resources = T::index();
            let document = CollectionDocument::new(resources, &index_object.includes, base_url);
            match document.serialize(response.serializer()) {
                Ok(_)   => response.set_status(Status::Ok),
                // TODO write the error to the body in the error case
                Err(_)  => response.set_status(Status::InternalError),
            }
            response
        });
    }

    pub fn attach_post<T: api::Post>(&mut self) where Resource<T>: Wrapper<T> {
        let Router { ref mut router, base_url } = *self;
        router.attach_post(T::resource(), |post_object| {
            let PostObject { attributes, relationships, resource_type, .. } = post_object;
            let mut response = R::Response::default();
            if &resource_type[..] != T::resource() {
                response.set_status(Status::Conflict);
                // TODO write the error to the body in the error case
                return response
            }
            let attributes = match from_value::<T>(attributes) {
                Ok(attributes)  => attributes,
                Err(_)          => {
                    response.set_status(Status::Conflict);
                    return response
                }
            };
            match attributes.post(relationships) {
                Ok(Some(resource))  => {
                    let document = ResourceDocument::new(resource, &[], base_url);
                    respond_with(document, response)
                }
                Ok(None)            => {
                    response.set_status(Status::NoContent);
                    response
                }
                Err(err)            => {
                    // TODO write the error to the body in the error case
                    match err {
                        api::PostError::Forbidden       => response.set_status(Status::Forbidden),
                        api::PostError::Conflict        => response.set_status(Status::Conflict),
                        api::PostError::BadRequest      => response.set_status(Status::BadRequest),
                        api::PostError::InternalError   => response.set_status(Status::InternalError),
                    }
                    response
                }
            }
        })
    }
}

fn respond_with<T: Serialize, R: Response>(document: T, mut response: R) -> R {
    match document.serialize(response.serializer()) {
        Ok(_)   => response.set_status(Status::Ok),
        // TODO write the error to the body in the error case
        Err(_)  => response.set_status(Status::InternalError),
    };
    response
}