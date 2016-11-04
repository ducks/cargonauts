#[macro_use]
mod relation;

/// The entry point for the routes DSL, which defines the endpoints of your API.
#[macro_export]
macro_rules! routes {
    {$(resource $resource:ident : [$($method:ident),*] { $(has $count:ident $rel:ident : [$($rel_method:ident),*];)*})*} => {
        pub fn attach_routes<T: $crate::router::Router>(router: &mut T, linker: T::Linker) {
            let mut router = $crate::_internal::_Router::new(router, linker);
            $({ _resource!(router, $resource : [$($method),*] {$($count $rel: [$($rel_method),*];)*}); })*
        }
    }
}

/// Do not call this macro, it is an implementation detail of the routes! macro.
#[macro_export]
macro_rules! _resource {
    ($router:expr, $resource:ty: [] $ignore:tt) => { };
    ($router:expr, $resource:ty: [$($method:ident),*] {}) => {
        impl $crate::api::raw::RawFetch for $resource {
            type Relationships = $crate::api::raw::NoRelationships;
        }

        impl $crate::api::raw::RawUpdate for $resource {
            type Relationships = $crate::api::raw::NoRelationships;
        }

        impl $crate::_internal::_FetchRels for $resource {
            fn rels<S: $crate::Serializer>(_: &$crate::api::Entity<Self>, _: &[$crate::router::IncludeQuery]) -> $crate::api::Result<(Self::Relationships, Vec<$crate::api::raw::Include<S>>)> {
                Ok(($crate::api::raw::NoRelationships, vec![]))
            }
        }

        impl $crate::_internal::_UpdateRels for $resource {
            fn update_rels(_: &$crate::api::Entity<Self>, rels: $crate::api::raw::NoRelationships) -> $crate::api::Result<$crate::api::raw::NoRelationships> {
                Ok($crate::api::raw::NoRelationships)
            }
        }
        _methods!($router, $resource, [$($method),*]);

    };
    ($router:expr, $resource:ty: [$($method:ident),*] { $($count:ident $rel:ident : [$($rel_method:ident),*];)* }) => {
        impl $crate::api::raw::RawFetch for $resource {
            type Relationships = Relationships;
        }

        impl $crate::api::raw::RawUpdate for $resource {
            type Relationships = UpdateRelationships;
        }

        impl $crate::_internal::_FetchRels for $resource {
            fn rels<S: $crate::Serializer>(id: &$crate::api::Entity<Self>, includes: &[$crate::router::IncludeQuery]) -> Result<(Self::Relationships, Vec<$crate::api::raw::Include<S>>), $crate::api::Error> {
                let mut include_objects = vec![];
                let rels = Relationships {
                    $(
                        $rel: {
                            _fetch_rel!(id, includes, include_objects, $resource, $rel, $count)
                        },
                    )*
                };
                Ok((rels, include_objects))
            }
        }

        impl $crate::_internal::_UpdateRels for $resource {
            fn update_rels(id: &$crate::api::Entity<Self>, rels: UpdateRelationships) -> Result<Relationships, $crate::api::Error> {
                $(
                    if let Some(rel) = rels.$rel {
                        _link_rel!(id, rel, $resource, $rel, $count);
                    };
                )*
                Ok(Relationships::default())
            }
        }

        #[allow(non_snake_case)]
        #[derive(Default)]
        struct Relationships {
            $(
                $rel: $crate::api::raw::RelationshipLinkage,
            )*
        }

        impl IntoIterator for Relationships {
            type Item = (&'static str, $crate::api::raw::RelationshipLinkage);
            type IntoIter = RelationshipsIntoIter;
            fn into_iter(self) -> RelationshipsIntoIter {
                RelationshipsIntoIter {
                    $(
                        $rel: Some(self.$rel),
                    )*
                    state: RelationshipsEnum::Init,
                }
            }
        }

        impl<'a> $crate::api::raw::FetchRelationships<'a> for Relationships {
            type Iter = RelationshipsIter<'a>;

            fn iter(&'a self) -> RelationshipsIter<'a> {
                RelationshipsIter {
                    rels: self,
                    state: RelationshipsEnum::Init,
                }
            }

            fn count(&self) -> usize {
                _count_rels!($($rel),*)
            }
        }

        #[allow(non_snake_case)]
        struct RelationshipsIntoIter {
            $(
                $rel: Option<$crate::api::raw::RelationshipLinkage>,
            )*
            state: RelationshipsEnum,
        }

        impl Iterator for RelationshipsIntoIter {
            type Item = (&'static str, $crate::api::raw::RelationshipLinkage);
            fn next(&mut self) -> Option<Self::Item> {
                match self.state {
                    RelationshipsEnum::Init => $( {
                        self.state = RelationshipsEnum::$rel;
                        self.$rel.take().map(|rel| (_name_rel!($rel, $count), rel))
                    }
                    RelationshipsEnum::$rel => )* { None }
                }
            }
        }

        struct RelationshipsIter<'a> {
            rels: &'a Relationships,
            state: RelationshipsEnum,
        }

        impl<'a> Iterator for RelationshipsIter<'a> {
            type Item = (&'static str, &'a $crate::api::raw::RelationshipLinkage);
            fn next(&mut self) -> Option<Self::Item> {
                match self.state {
                    RelationshipsEnum::Init => $( {
                        self.state = RelationshipsEnum::$rel;
                        Some((_name_rel!($rel, $count), &self.rels.$rel))
                    }
                    RelationshipsEnum::$rel => )* { None }
                }
            }
        }

        #[derive(Copy, Clone, Eq, PartialEq, Debug)]
        enum RelationshipsEnum {
            Init,
            $($rel,)*
        }

        #[derive(Clone, Eq, PartialEq, Debug, Default)]
        #[allow(non_snake_case)]
        struct UpdateRelationships {
            $(
                $rel: Option<$crate::api::raw::Relationship>,
            )*
        }

        impl $crate::api::raw::UpdateRelationships for UpdateRelationships {
            fn from_iter<I>(iter: I) -> Result<Self, $crate::api::Error> where I: Iterator<Item = (String, $crate::api::raw::Relationship)> {
                let mut rels = UpdateRelationships::default();
                for (name, value) in iter {
                    $( if &name[..] == _name_rel!($rel, $count) {
                        rels.$rel = Some(value);
                    } else )* {
                        return Err($crate::api::Error::BadRequest)
                    }
                }
                Ok(rels)
            }
        }

        _methods!($router, $resource, [$($method),*]);
        $(_rel_methods!($router, $resource, $count $rel [$($rel_method),*]);)*
    };
}

/// Do not call this macro, it is an implementation detail of the routes! macro.
#[macro_export]
macro_rules! _methods {
    ($router:expr, $resource:ty, [delete, $($method:ident),*]) => {
        $router.attach_delete::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _methods!($router, $resource, [$($method),*])
    };
    ($router:expr, $resource:ty, [delete]) => {
        $router.attach_delete::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, [get, $($method:ident),*]) => {
        $router.attach_get::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _methods!($router, $resource, [$($method),*]);
    };
    ($router:expr, $resource:ty, [get]) => {
        $router.attach_get::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, [index, $($method:ident),*]) => {
        $router.attach_index::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _methods!($router, $resource, [$($method),*])
    };
    ($router:expr, $resource:ty, [index]) => {
        $router.attach_index::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty,  [patch, $($method:ident),*]) => {
        $router.attach_patch::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _methods!($router, $resource, [$($method),*]);
    };
    ($router:expr, $resource:ty, [patch]) => {
        $router.attach_patch::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty,  [patch-async, $($method:ident),*]) => {
        $router.attach_patch_async::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _methods!($router, $resource, [$($method),*]);
    };
    ($router:expr, $resource:ty, [patch-async]) => {
        $router.attach_patch_async::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, [post, $($method:ident),*]) => {
        $router.attach_post::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _methods!($router, $resource, [$($method),*])
    };
    ($router:expr, $resource:ty, [post]) => {
        $router.attach_post::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, [post-async, $($method:ident),*]) => {
        $router.attach_post_async::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _methods!($router, $resource, [$($method),*])
    };
    ($router:expr, $resource:ty, [post-async]) => {
        $router.attach_post_async::<$resource, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, []) => {
    };
}

#[macro_export]
macro_rules! _rel_methods {
    ($router:expr, $resource:ty, one $rel:ident [fetch, $($method:ident),*])  => {
        $router.attach_fetch_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, one $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, one $rel:ident [fetch])  => {
        $router.attach_fetch_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, one $rel:ident [post, $($method:ident),*])  => {
        $router.attach_post_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, one $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, one $rel:ident [post])  => {
        $router.attach_post_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, one $rel:ident [patch, $($method:ident),*])  => {
        $router.attach_patch_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, one $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, one $rel:ident [patch])  => {
        $router.attach_patch_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, one $rel:ident [delete, $($method:ident),*])  => {
        $router.attach_delete_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, one $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, one $rel:ident [delete])  => {
        $router.attach_delete_one::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, many $rel:ident [fetch, $($method:ident),*])  => {
        $router.attach_fetch_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, many $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, many $rel:ident [fetch])  => {
        $router.attach_fetch_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, many $rel:ident [append, $($method:ident),*])  => {
        $router.attach_append_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, many $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, many $rel:ident [append])  => {
        $router.attach_append_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, many $rel:ident [replace, $($method:ident),*])  => {
        $router.attach_replace_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, many $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, many $rel:ident [replace])  => {
        $router.attach_replace_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, many $rel:ident [remove, $($method:ident),*])  => {
        $router.attach_remove_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, many $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, many $rel:ident [remove])  => {
        $router.attach_remove_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, many $rel:ident [clear, $($method:ident),*])  => {
        $router.attach_clear_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
        _rel_methods!($router, $resource, many $rel [$($method),*]);
    };
    ($router:expr, $resource:ty, many $rel:ident [clear])  => {
        $router.attach_clear_many::<$resource, $rel, $crate::presenter::JsonApi<T::Response, T::Linker>>();
    };
    ($router:expr, $resource:ty, $count:ident $rel:ident [])  => {};
}

#[macro_export]
macro_rules! _fetch_rel {
    ($id:expr, $includes:expr, $includes_out:expr, $resource:ty, $rel:ty, one) => {
        if let Some(include) = $includes.iter().find(|include| include.is_of(_name_rel!($rel, one))) {
            match <$resource as $crate::api::rel::raw::FetchOne<$rel>>::fetch_one($id, &include.transitive)? {
                Some(response)  => {
                    let identifier = $crate::api::raw::Identifier::from(&response.resource);
                    $includes_out.push(response.resource.into());
                    $includes_out.extend(response.includes);
                    $crate::api::raw::RelationshipLinkage {
                        linkage: Some($crate::api::raw::Relationship::One(Some(identifier))),
                    }
                }
                None            => {
                    $crate::api::raw::RelationshipLinkage {
                        linkage: Some($crate::api::raw::Relationship::One(None)),
                    }
                }
            }
        } else {
            $crate::api::raw::RelationshipLinkage {
                linkage: None,
            }
        };
    };
    ($id:expr, $includes:expr, $includes_out:expr, $resource:ty, $rel:ty, many) => {
        if let Some(include) = $includes.iter().find(|include| include.is_of(_name_rel!($rel, many))) {
            let response = <$resource as $crate::api::rel::raw::FetchMany<$rel>>::fetch_many($id, &include.transitive)?;
            let identifiers = response.resources.iter().map($crate::api::raw::Identifier::from).collect();
            $includes_out.extend(response.resources.into_iter().map(Into::into));
            $includes_out.extend(response.includes);
            $crate::api::raw::RelationshipLinkage {
                linkage: Some($crate::api::raw::Relationship::Many(identifiers)),
            }
        } else {
            $crate::api::raw::RelationshipLinkage {
                linkage: None,
            }
        }
    };
}

#[macro_export]
macro_rules! _link_rel {
    ($id:expr, $rel_obj:expr, $resource:ty, $rel:ty, one)   => {
        match $rel_obj {
            $crate::api::raw::Relationship::One(Some(ref identifier)) => {
                let rel_id = identifier.id.parse().or(Err($crate::api::Error::BadRequest))?;
                <$resource as $crate::_internal::_MaybeLinkOne<$rel>>::link_one($id, &rel_id)?;

            }
            $crate::api::raw::Relationship::One(None)           => {
                <$resource as $crate::_internal::_MaybeUnlinkOne<$rel>>::unlink_one($id)?;
            }
            $crate::api::raw::Relationship::Many(_)             => {
                return Err($crate::api::Error::BadRequest);
            }
        }
    };
    ($id:expr, $rel_obj:expr, $resource:ty, $rel:ty, many)   => {
        match $rel_obj {
            $crate::api::raw::Relationship::One(_)                  => {
                return Err($crate::api::Error::BadRequest);
            }
            $crate::api::raw::Relationship::Many(ref identifiers)   => {
                let rel_ids = identifiers.iter().map(|identifier| identifier.id.parse().or(Err($crate::api::Error::BadRequest))).collect::<Result<Vec<_>, _>>()?;
                <$resource as $crate::_internal::_MaybeReplaceLinks<$rel>>::replace_links($id, &rel_ids)?;
            }
        }
    };
}

#[macro_export]
macro_rules! _name_rel {
    ($rel:ty, one) => {
        <$rel as $crate::api::rel::Relation>::to_one()
    };
    ($rel:ty, many) => {
        <$rel as $crate::api::rel::Relation>::to_many()
    };
}

#[macro_export]
macro_rules! _count_rels {
    ($head:ident, $($rest:ident),*) => (1 + _count_rels!($($rest),*));
    ($last:ident)                   => (1);
}
