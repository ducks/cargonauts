use cfg::CargonautsConfig;
use heck::KebabCase;
use quote::{Tokens, Ident};

use ast::*;

pub fn setup(setup: &Setup, config: Option<&CargonautsConfig>) -> Tokens {
    let clients: Vec<_> = setup.members.iter().map(|setup| match *setup {
        SetupMember::Client(ref c) => client(c, config),
        SetupMember::Connection(ref c) => conn(c, config),
    }).collect();

    if clients.is_empty() {
        quote! {
            ::cargonauts::futures::future::ok(::cargonauts::routing::EnvBuilder::new().build())
        }
    } else {
        quote!({
            let env_b = ::cargonauts::routing::EnvBuilder::new();
            let mut vec = vec![];
            #({ let env_b = env_b.clone(); vec.push(#clients) })*
            ::cargonauts::futures::stream::futures_unordered(vec).for_each(|_| Ok(()))
                .map(|_| env_b.build())
        })
    }
}

fn client(client: &Client, config: Option<&CargonautsConfig>) -> Tokens {
    let ident = Ident::new(&client.client[..]);
    let service = quote!(::cargonauts::routing::ClientConnector<#ident>);
    let client = client.client.to_kebab_case();
    let cfg = pool_cfg(&client, config);
    let member_cfg = member_cfg(&client, &service, config);
    quote!({env_b.new_pool::<#service>(handle.clone(), #cfg, #member_cfg)})
}

fn conn(conn: &Connection, config: Option<&CargonautsConfig>) -> Tokens {
    let ident = Ident::new(&conn.conn[..]);
    let conn = conn.conn.to_kebab_case();
    let cfg = pool_cfg(&conn, config);
    let member_cfg = member_cfg(&conn, &quote!(#ident), config);
    quote!({env_b.new_pool::<#ident>(handle.clone(), #cfg, #member_cfg)})
}

fn pool_cfg(client: &str, config: Option<&CargonautsConfig>) -> Tokens {
    match config.and_then(|cfg| cfg.client_cfg(client)) {
        Some(cfg)   => panic!(),
        None        => quote!(::cargonauts::server::pool::Config::default()),
    }
}

fn member_cfg(client: &str, service: &Tokens, config: Option<&CargonautsConfig>) -> Tokens {
    match config.and_then(|cfg| cfg.client_cfg(client)) {
        Some(cfg)   => panic!(),
        None        => {
            quote!(<#service as ::cargonauts::server::pool::Configure>::Config::default())
        }
    }
}
