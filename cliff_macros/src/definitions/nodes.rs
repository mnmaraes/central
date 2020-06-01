use std::convert::{From, TryFrom};

use syn::punctuated::Punctuated;
use syn::{Block, Expr, Ident, Token, Type};

#[derive(Clone, Debug)]
pub enum ServerInterface {
    Single(ServerMessage),
    Multiple(Punctuated<ServerMessage, Token![,]>),
}

#[derive(Clone, Debug)]
pub struct CaseField {
    pub(crate) name: Ident,
    pub(crate) ty: Type,
}

#[derive(Clone, Debug)]
pub struct RequestCase {
    pub(crate) ident: Ident,
    pub(crate) fields: Vec<CaseField>,
}

#[derive(Clone, Debug)]
pub struct CaseFieldValue {
    pub(crate) name: Ident,
    pub(crate) value: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct CaseDeclaration {
    pub(crate) name: Ident,
    pub(crate) fields: Vec<CaseField>,
}

impl From<&RequestCase> for CaseDeclaration {
    fn from(case: &RequestCase) -> Self {
        CaseDeclaration {
            name: case.ident.clone(),
            fields: case.fields.to_vec(),
        }
    }
}

impl TryFrom<&ResponseCase> for CaseDeclaration {
    type Error = String;

    fn try_from(case: &ResponseCase) -> core::result::Result<Self, Self::Error> {
        use ResponseCase::*;

        let declaration = match case {
            Empty { name } => CaseDeclaration {
                name: name.clone(),
                fields: vec![],
            },
            Typed { name, types, build } => CaseDeclaration {
                name: name.clone(),
                fields: types
                    .iter()
                    .zip(build)
                    .map(|(ty, c_value)| CaseField {
                        name: c_value.name.clone(),
                        ty: ty.clone(),
                    })
                    .collect(),
            },
            Structured { name: _, build: _ } => {
                return Err("Can't convert Structured variant into Case Declaration".into());
            }
        };

        Ok(declaration)
    }
}

#[derive(Clone, Debug)]
pub enum ResponseCase {
    Empty {
        name: Ident,
    },
    Structured {
        name: Ident,
        build: Punctuated<CaseFieldValue, Token![,]>,
    },
    Typed {
        name: Ident,
        types: Vec<Type>,
        build: Punctuated<CaseFieldValue, Token![,]>,
    },
}

#[derive(Clone, Debug)]
pub struct ConditionalResponse {
    pub(crate) cond: Option<Expr>,
    pub(crate) response: ResponseCase,
}

#[derive(Clone, Debug)]
pub enum Response {
    Base { case: ResponseCase },
    Conditional { cases: Vec<ConditionalResponse> },
}

#[derive(Clone, Debug)]
pub struct RequestHandler {
    pub(crate) request_case: RequestCase,
    pub(crate) block: Option<Block>,
    pub(crate) response: Response,
}

#[derive(Clone, Debug)]
pub struct ServerMessage {
    pub(crate) interface_name: Ident,
    pub(crate) handlers: Vec<RequestHandler>,
}

#[derive(Clone)]
pub struct Router {
    pub(crate) router_type: Ident,

    pub(crate) interface: ServerInterface,
}
