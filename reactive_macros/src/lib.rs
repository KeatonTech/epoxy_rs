extern crate proc_macro;
extern crate proc_macro2;
extern crate proc_macro_hack;
#[macro_use]
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro_hack::proc_macro_hack;
use std::collections::HashSet;

#[proc_macro_hack]
pub fn computed(block_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let wrapped_block_stream = proc_macro::TokenStream::from(proc_macro::TokenTree::from(
        proc_macro::Group::new(proc_macro::Delimiter::Brace, block_stream),
    ));
    let compute_fn_body: syn::Block = syn::parse(wrapped_block_stream).unwrap();

    let mut external_vars = ExternalVarVisitor::new();
    syn::visit::visit_block(&mut external_vars, &compute_fn_body);
    let output = format!("{:?}", external_vars.idents);
    proc_macro::TokenStream::from(quote! {
        {
            let call = #compute_fn_body;
            let consts = #output;
            1
        }
    })
}

struct ExternalVarVisitor {
    pub idents: HashSet<syn::Ident>,
    pub local_idents: HashSet<syn::Ident>,
}

impl ExternalVarVisitor {
    pub fn new() -> ExternalVarVisitor {
        ExternalVarVisitor {
            idents: HashSet::new(),
            local_idents: HashSet::new(),
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for ExternalVarVisitor {
    fn visit_ident(&mut self, ident: &'ast syn::Ident) {
        self.idents.insert(ident.clone());
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        for pat in local.pats.iter() {
            match pat {
                syn::Pat::Ident(patIdent) => self.local_idents.insert(patIdent.ident.clone()),
                _ => false,
            };
        }
    }
}
