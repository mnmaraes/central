use std::collections::HashMap;
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

#[derive(Clone, Debug)]
pub struct Router {
    pub(crate) router_type: Ident,

    pub(crate) interface: ServerInterface,
}

#[derive(Clone, Debug)]
pub struct Client {
    pub(crate) request_type: Ident,
    pub(crate) client_name: Ident,

    pub(crate) actions: Vec<ClientAction>,
    pub(crate) response_mapping: Vec<ResponseMapping>,
}

impl Client {
    pub(crate) fn get_client_type_name(&self) -> Ident {
        Ident::new(
            format!("{}Client", self.client_name).as_str(),
            self.client_name.span(),
        )
    }

    pub(crate) fn get_request_type_name(&self) -> Ident {
        Ident::new(
            format!("{}Request", self.request_type).as_str(),
            self.request_type.span(),
        )
    }

    pub(crate) fn get_response_type_name(&self) -> Ident {
        Ident::new(
            format!("{}Response", self.request_type).as_str(),
            self.request_type.span(),
        )
    }
}

#[derive(Clone, Debug)]
pub enum ClientFields {
    Actions(Vec<ClientAction>),
    ResponseMapping(Vec<ResponseMapping>),
}

#[derive(Clone, Debug)]
pub struct ActionDeclaration {
    pub(crate) name: Ident,
    pub(crate) fields: Vec<CaseField>,

    pub(crate) result_type: Option<Type>,
}

impl From<&ClientAction> for ActionDeclaration {
    fn from(action: &ClientAction) -> Self {
        let ActionType { name, fields } = action.action_type.clone();
        let result_type = action
            .response
            .clone()
            .map(|res| match res {
                ClientResponse::Wait(res) => res.ty,
            })
            .flatten();

        ActionDeclaration {
            name,
            fields,
            result_type,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FutureDescriptor {
    pub(crate) ref_types: Vec<Ident>,

    pub(crate) result_type: Option<Type>,
}

impl FutureDescriptor {
    pub(crate) fn get_descriptors(client: &Client) -> Vec<FutureDescriptor> {
        client
            .actions
            .iter()
            .map(|action| {
                (
                    action.action_type.name.clone(),
                    action.response.clone().and_then(|res| match res {
                        ClientResponse::Wait(WaitResponse { ty }) => Some(ty),
                    }),
                )
            })
            .filter_map(|(ref_type, result_type)| match result_type {
                None => None,
                Some(ty) => Some((ref_type, ty)),
            })
            .fold(
                HashMap::<Option<Type>, Vec<Ident>>::new(),
                |mut hash, (ref_type, result_type)| {
                    if let Some(v) = hash.get_mut(&result_type) {
                        v.push(ref_type);
                    } else {
                        hash.insert(result_type, vec![ref_type]);
                    };

                    hash
                },
            )
            .iter()
            .map(|(result_type, ref_types)| FutureDescriptor {
                ref_types: ref_types.clone(),
                result_type: result_type.clone(),
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct ClientAction {
    pub(crate) action_type: ActionType,
    pub(crate) mapped_request: Option<ActionMapping>,

    pub(crate) response: Option<ClientResponse>,
}

#[derive(Clone, Debug)]
pub enum FutureRequestMapping {
    None,
    Single,
    Indexed(usize),
}

impl FutureRequestMapping {
    pub(crate) fn get_mapping(
        type_name: &Ident,
        descriptors: &[FutureDescriptor],
    ) -> FutureRequestMapping {
        match descriptors.iter().enumerate().find(|(_idx, descriptor)| {
            descriptor
                .ref_types
                .iter()
                .cloned()
                .any(|ty| ty == *type_name)
        }) {
            Some((_, _)) if descriptors.len() == 1 => FutureRequestMapping::Single,
            Some((idx, _)) => FutureRequestMapping::Indexed(idx),
            None => FutureRequestMapping::None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndexMapping {
    pub(crate) index: usize,
    pub(crate) action_mapping: TypedActionMapping,
}

#[derive(Clone, Debug)]
pub enum FutureResponseMapping {
    Single {
        mapping_case: ResponseMappingCase,
        action_mapping: TypedActionMapping,
    },
    Indexed {
        mapping_case: ResponseMappingCase,
        indexed_mappings: Vec<IndexMapping>,
    },
}

impl FutureResponseMapping {
    pub(crate) fn wrap_mapping(
        mapping: &ResponseMapping,
        descriptors: &[FutureDescriptor],
    ) -> FutureResponseMapping {
        if descriptors.len() == 1 {
            FutureResponseMapping::Single {
                mapping_case: mapping.response_case.clone(),
                action_mapping: mapping.action_mapping[0].clone(),
            }
        } else {
            FutureResponseMapping::Indexed {
                mapping_case: mapping.response_case.clone(),
                indexed_mappings: descriptors
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, descriptor)| {
                        match mapping.action_mapping.iter().find(|action_mapping| {
                            action_mapping.get_type() == descriptor.result_type
                        }) {
                            Some(action_mapping) => Some(IndexMapping {
                                index: idx,
                                action_mapping: action_mapping.clone(),
                            }),
                            None => None,
                        }
                    })
                    .collect(),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct HandlerDeclaration {
    pub(crate) client_name: Ident,
    pub(crate) request_name: Ident,
    pub(crate) future_mapping: FutureRequestMapping,

    pub(crate) action: ClientAction,
}

impl HandlerDeclaration {
    pub(crate) fn get_declarations(
        client: &Client,
        descriptors: &[FutureDescriptor],
    ) -> Vec<HandlerDeclaration> {
        let request_name = client.get_request_type_name();
        let client_name = client.get_client_type_name();

        client
            .actions
            .iter()
            .map(|action| {
                let future_mapping =
                    FutureRequestMapping::get_mapping(&action.action_type.name, descriptors);

                HandlerDeclaration {
                    client_name: client_name.clone(),
                    request_name: request_name.clone(),
                    future_mapping,
                    action: action.clone(),
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct ActionType {
    pub(crate) name: Ident,
    pub(crate) fields: Vec<CaseField>,
}

#[derive(Clone, Debug)]
pub enum ActionMapping {
    BaseMapping {
        name: Ident,
        field_values: Vec<CaseFieldValue>,
    },
    BlockMapping(Block),
}

#[derive(Clone, Debug)]
pub enum ClientResponse {
    Wait(WaitResponse),
}

#[derive(Clone, Debug)]
pub struct WaitResponse {
    pub(crate) ty: Option<Type>,
}

#[derive(Clone, Debug)]
pub enum TypedActionMapping {
    UnitMapping { block: Option<Block> },
    ExprMapping { ty: Box<Type>, expr: Box<Expr> },
    BlockMapping { ty: Box<Type>, block: Block },
}

impl TypedActionMapping {
    fn get_type(&self) -> Option<Type> {
        match self {
            TypedActionMapping::UnitMapping { block: _ } => None,
            TypedActionMapping::ExprMapping { ty, expr: _ } => Some(ty.as_ref().clone()),
            TypedActionMapping::BlockMapping { ty, block: _ } => Some(ty.as_ref().clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ResponseMappingCase {
    Empty {
        name: Ident,
    },
    Structured {
        name: Ident,
        build: Punctuated<CaseFieldValue, Token![,]>,
    },
}

#[derive(Clone, Debug)]
pub struct ResponseMapping {
    pub(crate) response_case: ResponseMappingCase,
    pub(crate) action_mapping: Vec<TypedActionMapping>,
}
