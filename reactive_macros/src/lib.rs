extern crate proc_macro;
extern crate proc_macro2;
extern crate proc_macro_hack;
#[macro_use]
extern crate lazy_static;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate reactive_streams;

use proc_macro_hack::proc_macro_hack;
use std::collections::HashSet;

lazy_static! {
    static ref EXCEMPT_IDENTIFIERS: HashSet<&'static str> = {
        let mut m = HashSet::new();
        m.insert("i8");
        m.insert("u8");
        m.insert("i16");
        m.insert("u16");
        m.insert("i32");
        m.insert("u32");
        m.insert("i64");
        m.insert("u64");
        m.insert("i128");
        m.insert("u128");
        m.insert("isize");
        m.insert("usize");
        m.insert("f32");
        m.insert("f64");
        m.insert("bool");
        m.insert("char");
        m.insert("str");
        m
    };
}

#[proc_macro_hack]
pub fn computed(block_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let wrapped_block_stream = proc_macro::TokenStream::from(proc_macro::TokenTree::from(
        proc_macro::Group::new(proc_macro::Delimiter::Brace, block_stream),
    ));
    let compute_fn_body: syn::Block = syn::parse(wrapped_block_stream).unwrap();

    let mut external_vars = ExternalVarVisitor::new();
    syn::visit::visit_block(&mut external_vars, &compute_fn_body);

    // NOTE: I'm not proud of this code but I haven't figured out a better way to work with quote!()
    let value_tokens_1 = external_vars
        .idents
        .difference(&external_vars.local_idents)
        .into_iter()
        .filter(|ident| !EXCEMPT_IDENTIFIERS.contains(&*format!("{}", ident)));

    let value_tokens_2 = value_tokens_1.clone();
    let value_tokens_3 = value_tokens_1.clone();
    let value_tokens_4 = value_tokens_1.clone();
    let value_tokens_5 = value_tokens_1.clone();
    let value_tokens_6 = value_tokens_1.clone();
    let value_tokens_7 = value_tokens_1.clone();

    proc_macro::TokenStream::from(quote! {
        {
            #(let #value_tokens_1 = #value_tokens_2.clone();
            )*

            let initial = {
                #(let #value_tokens_3 = &*#value_tokens_4.get();
                )*
                #compute_fn_body
            };

            reactive_streams::merge(vec![
                #(#value_tokens_5.as_stream().map(|_| ())),*
            ]).map(move |_| {
                #(let #value_tokens_6 = &*#value_tokens_7.get();
                )*
                #compute_fn_body
            }).to_reactive_value_with_default(initial)
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
                syn::Pat::Ident(pat_ident) => self.local_idents.insert(pat_ident.ident.clone()),
                _ => false,
            };
        }
    }
}
