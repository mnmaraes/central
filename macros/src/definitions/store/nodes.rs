use syn::{Block, Expr, FieldsNamed, Ident, Type};

#[derive(Clone, Debug)]
pub struct Ipc {
    pub(crate) model_name: Ident,

    pub(crate) query_actions: Vec<QueryAction>,
    pub(crate) command_actions: Vec<CommandAction>,
}

#[derive(Clone, Debug)]
pub enum StoreField {
    Command { actions: Vec<CommandAction> },
    Query { actions: Vec<QueryAction> },
}

#[derive(Clone, Debug)]
pub struct CommandAction {
    pub(crate) action_name: Ident,
    pub(crate) fields: Option<FieldsNamed>,
    pub(crate) block: Block,
}

#[derive(Clone, Debug)]
pub struct QueryAction {
    pub(crate) action_name: Ident,
    pub(crate) action_fields: Option<FieldsNamed>,

    pub(crate) run_block: Block,

    pub(crate) response_name: Ident,
    pub(crate) response_fields: Vec<QueryResponseField>,

    pub(crate) result_block: Block,
    pub(crate) result_type: Type,
}

pub struct ResultMapping {
    pub(crate) response_name: Ident,
    pub(crate) field_names: proc_macro2::TokenStream,
}

#[derive(Clone, Debug)]
pub struct QueryResponseField {
    pub(crate) field_name: Ident,
    pub(crate) ty: Option<Type>,
    pub(crate) value: Expr,
}
