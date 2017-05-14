use std::fs;
use std::path::{Path, PathBuf};

use cfg::CargonautsConfig;
use heck::KebabCase;
use quote::{ToTokens, Tokens, Ident};

use ast::*;

pub struct Routes {
    routes: Vec<Route>,
    assets: Tokens,
}

impl Routes {
    pub fn new(assets: Tokens, resources: &[RouteItem], cfg: &CargonautsConfig) -> Routes {
        Routes {
            routes: Route::build(resources, cfg),
            assets
        }
    }
}

impl ToTokens for Routes {
    fn to_tokens(&self, tokens: &mut Tokens) {
        let routes = &self.routes;
        let assets = &self.assets;
        tokens.append(quote! ({
            use ::cargonauts::server::NewService;
            let mut routes = ::cargonauts::routing::RouteBuilder::default();
            #(#routes)*
            #assets
            routes.build(env)
        }));
    }
}

#[derive(Clone)]
struct Route {
    resource: String,
    endpoint: String,
    method: String,
    format: String,
    rel: Option<String>,
    rel_endpoint: Option<String>,
    middleware: Option<String>,
    template_root: PathBuf,
}

impl Route {
    fn build(routes: &[RouteItem], cfg: &CargonautsConfig) -> Vec<Route> {
        fn routes_for_resource(vec: &mut Vec<Route>, resource: &Resource, module: Option<&str>, template_root: &Path) {
            let endpoint = match module {
                Some(module)    => {
                    match resource.header.endpoint.as_ref() {
                        Some(endpoint)  => [module, endpoint].join("/"),
                        None            => {
                            let endpoint = resource.header.ty.to_kebab_case();
                            [module, &endpoint].join("/")
                        }
                    }
                }
                None            => {
                    resource.header.endpoint.clone().unwrap_or_else(|| {
                        resource.header.ty.to_kebab_case()
                    })
                }
            };

            let resource_ty = &resource.header.ty;

            for (method, attrs) in resource.members.iter().filter_map(|m| m.as_method()) {
                let middleware = attrs.iter().filter_map(|attr| attr.as_middleware()).next();
                for m in &method.methods {
                    vec.push(Route {
                        resource: resource_ty.clone(),
                        endpoint: endpoint.clone(),
                        method: m.clone(),
                        format: method.format.clone(),
                        rel: None,
                        rel_endpoint: None,
                        middleware: middleware.clone(),
                        template_root: template_root.to_owned(),
                    })
                }
            }

            for relation in resource.members.iter().filter_map(|m| m.as_relation()) {
                for (method, attrs) in relation.members.iter().filter_map(|m| m.as_method()) {
                    let middleware = attrs.iter().filter_map(|attr| attr.as_middleware()).next();
                    for m in &method.methods {
                        vec.push(Route {
                            resource: resource_ty.clone(),
                            endpoint: endpoint.clone(),
                            method: m.clone(),
                            format: method.format.clone(),
                            rel: Some(relation.rel.clone()),
                            rel_endpoint: relation.endpoint.clone().or_else(|| Some(relation.rel.to_kebab_case())),
                            middleware: middleware.clone(),
                            template_root: template_root.to_owned(),
                        })
                    }
                }
            }
        }

        fn routes_for_item(vec: &mut Vec<Route>, item: &RouteItem, module: Option<&str>, template_root: &Path) {
            match *item {
                RouteItem::Resource(ref resource) => {
                    routes_for_resource(vec, resource, module, template_root);
                }
                RouteItem::Module(ref inner, ref items) => {
                    let module = match module {
                        Some(module) => [module, inner].join("/"),
                        None         => inner.to_owned()
                    };
                    
                    for item in items {
                        routes_for_item(vec, item, Some(&module), template_root)
                    }
                }
            }
        }

        let mut vec = vec![];
        let root = cfg.templates();

        for item in routes {
            routes_for_item(&mut vec, item, None, &root);
        }

        vec
    }
}

impl ToTokens for Route {
    fn to_tokens(&self, tokens: &mut Tokens) {
        let endpoint = &self.endpoint;
        let template = load_template(self.template_root.clone(),
                                     &self.resource,
                                     &self.method,
                                     self.rel.as_ref());
        let resource = Ident::new(&self.resource[..]);
        let format = Ident::new(&self.format[..]);
        let method = method_for(&self.method, self.rel.as_ref(), &resource);

        let http_method = quote!(<#method as ::cargonauts::method::Method<#resource>>::ROUTE.method);

        let path = if let Some(ref rel) = self.rel_endpoint {
            quote!(::cargonauts::routing::path(<#method as ::cargonauts::method::Method<#resource>>::ROUTE.kind, #endpoint, Some(#rel)))
        } else {
            quote!(::cargonauts::routing::path(<#method as ::cargonauts::method::Method<#resource>>::ROUTE.kind, #endpoint, None))
        };

        if let Some(ref middleware) = self.middleware {
            let middleware = Ident::new(&middleware[..]);
            tokens.append(quote!({
                let service = ::cargonauts::routing::EndpointService::<_, _, (#resource, #format, #method)>::new(#template);
                let service = ::cargonauts::middleware::Middleware::wrap(<#middleware as Default>::default(), service);
                routes.add(#http_method, #path, Box::new(service) as ::cargonauts::routing::Handler);
            }));
        } else {
            tokens.append(quote!({
                let service = ::cargonauts::routing::EndpointService::<_, _, (#resource, #format, #method)>::new(#template);
                routes.add(#http_method, #path, Box::new(service) as ::cargonauts::routing::Handler);
            }));
        }

    }
}

fn method_for(method: &str, rel: Option<&String>, resource: &Ident) -> Tokens {
    // Special cases to allow mainsail methods to have associated types
    match method {
        "Post"  => {
            quote!(Post<Identifier = <#resource as ::cargonauts::api::Resource>::Identifier, Post = <#resource as ::cargonauts::api::Post>::Post>)
        }
        "Patch" => {
            quote!(Patch<Identifier = <#resource as ::cargonauts::api::Resource>::Identifier, Patch = <#resource as ::cargonauts::api::Patch>::Patch>)
        }
        "PostRelated" => {
            let rel = Ident::new(&rel.unwrap()[..]);
            quote!(PostRelated<#rel, Identifier = <#resource as ::cargonauts::api::Resource>::Identifier, Post = <#resource as ::cargonauts::api::PostRelated<#rel>::Post>)
        }
        "UpdateRelated" => {
            let rel = Ident::new(&rel.unwrap()[..]);
            quote!(UpdateRelated<#rel, Identifier = <#resource as ::cargonauts::api::Resource>::Identifier, Update = <#resource as ::cargonauts::api::UpdateRelated<#rel>::Update>)
        }
        _       => {
            let method = Ident::new(method);
            match rel {
                Some(rel)   => {
                    let rel = Ident::new(&rel[..]);
                    quote!(#method<#rel, Identifier = <#resource as ::cargonauts::api::Resource>::Identifier>)
                }
                None        => {
                    quote!(#method<Identifier = <#resource as ::cargonauts::api::Resource>::Identifier>)
                }
            }
        }
    }
}

fn load_template(mut path: PathBuf, resource: &str, method: &str, rel: Option<&String>) -> Tokens {
    path.push(resource);
    if let Some(rel) = rel { path.push(rel); }

    let match_method = |entry: &fs::DirEntry| {
        entry.file_name().into_string().unwrap().starts_with(&method.to_kebab_case())
    };

    let mut walk_dir = fs::read_dir(path).into_iter().flat_map(|ls| ls.into_iter().map(|e| e.unwrap()));

    if let Some(entry) = walk_dir.find(match_method) {
        let path = entry.path();
        let path = path.to_string_lossy();
        quote!(Some(::cargonauts::format::Template::static_prepare(include_str!(#path))))
    } else {
        quote!(None)
    }
}
