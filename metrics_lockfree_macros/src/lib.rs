extern crate proc_macro;

use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream};
use std::env;
use syn::{Data, DeriveInput};

fn debug_print_generated(ast: &DeriveInput, toks: &TokenStream) {
    let debug = env::var("METRICS_MACROS_DEBUG");
    if let Ok(s) = debug {
        if s == "1" {
            println!("{}", toks);
        }

        if ast.ident == s {
            println!("{}", toks);
        }
    }
}

//use crate::helpers::{case_style::snakify, non_enum_error, HasStrumVariantProperties};
use quote::{format_ident, quote, ToTokens};

fn non_struct_error() -> syn::Error {
    syn::Error::new(Span::call_site(), "This macro only supports structs.")
}

/// heck doesn't treat numbers as new words, but this function does.
/// E.g. for input `Hello2You`, heck would output `hello2_you`, and snakify would output `hello_2_you`.
fn snakify(s: &str) -> String {
    let mut output: Vec<char> = s.to_string().to_snake_case().chars().collect();
    let mut num_starts = vec![];
    for (pos, c) in output.iter().enumerate() {
        if c.is_digit(10) && pos != 0 && !output[pos - 1].is_digit(10) {
            num_starts.push(pos);
        }
    }
    // need to do in reverse, because after inserting, all chars after the point of insertion are off
    for i in num_starts.into_iter().rev() {
        output.insert(i, '_')
    }
    output.into_iter().collect()
}

fn generate_metrics(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let fields = match &ast.data {
        Data::Struct(v) => &v.fields,
        _ => return Err(non_struct_error()),
    };

    let static_factory_name = format_ident!("{}", format!("{}Factory", &ast.ident).to_uppercase());
    let factory_name = format_ident!("{}Factory", &ast.ident);
    let values_name = format_ident!("{}Values", &ast.ident);
    let struct_name = ast.ident.clone();

    let mut count = 0;

    let mut field_names = vec![];

    let fields: Vec<_> = fields
        .iter()
        .filter_map(|field| match &field.ident {
            Some(ident) => {
                let ident = ident.to_string();
                let fn_name = format_ident!("add_{}", snakify(&ident));
                field_names.push(ident.to_token_stream());
                let idx: usize = count;
                count += 1;
                Some(quote! {
                    pub fn #fn_name(&mut self, inc: u64) {
                        self.add(#idx, 1)
                    }
                })
            }
            None => None,
        })
        .collect();

    Ok(quote! {
        impl #struct_name {
            pub fn new() -> #values_name {
                let mut factory = #static_factory_name.write().unwrap();
                factory.build()
            }

            pub fn read_lock<'a>() -> std::sync::LockResult<std::sync::RwLockReadGuard<'a, #factory_name>> {
                #static_factory_name.read()
            }
        }

        pub struct #values_name {
            ptr: *mut u64,
            size: usize,
        }

        impl #values_name {

            fn new(list: &mut [u64]) -> Self {
                Self {
                    ptr: list.as_mut_ptr(),
                    size: list.len(),
                }
            }

            fn add(&mut self, idx: usize, inc: u64) {
                if idx >= self.size {
                    panic!("idx overflow");
                }

                // c'est safe, parce que metric list ne peut pas etre dans deux threads à la fois
                // il ne faut jamais que cet objet puisse etre cloné
                // rust interdit un utilisateur de le faire parce que l'objet contient un pointeur
                unsafe {
                    let ptr = self.ptr.add(idx);
                    *ptr += inc;
                }
            }

            #(#fields)*
        }

        unsafe impl Send for #values_name {}

        struct #factory_name {
            metrics: Vec<String>,
            per_thread_metrics: Vec<Vec<u64>>,
        }

        impl #factory_name {
            pub fn new(array: &[&str]) -> Self {
                let metrics = array.iter().map(|s| s.to_string()).collect();
                Self {
                    metrics,
                    per_thread_metrics: vec![],
                }
            }

            pub fn build(&mut self) -> #values_name {
                self.per_thread_metrics.push(vec![0; self.metrics.len()]);
                let last = self.per_thread_metrics.last_mut().unwrap();
                #values_name ::new(last)
            }

            pub fn thread(&self) -> &Vec<Vec<u64>> {
                &self.per_thread_metrics
            }

            pub fn metrics(&self) -> &Vec<String> {
                &self.metrics
            }
        }

        static #static_factory_name : std::sync::LazyLock<std::sync::RwLock<#factory_name>> =
            std::sync::LazyLock::new(|| std::sync::RwLock::new(#factory_name ::new(&[ #(#field_names),* ])));

    })
}

#[proc_macro_derive(Metrics)]
pub fn enum_try_as(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);

    let toks = generate_metrics(&ast).unwrap_or_else(|err| err.to_compile_error());
    debug_print_generated(&ast, &toks);
    toks.into()
}
