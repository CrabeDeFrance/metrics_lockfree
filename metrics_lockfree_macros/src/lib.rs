extern crate proc_macro;

use heck::ToSnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use std::env;
use syn::{Data, DeriveInput, Fields};
//use syn::{Lit, LitStr};

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

/*
fn parse_field_doc_comment(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        let meta = attr.parse_args().unwrap();
        if let syn::Meta::NameValue(meta) = meta {
            match meta.value {
                syn::Expr::Lit(doc) => match doc.lit {
                    syn::Lit::Str(doc) => return Some(doc.value().trim().to_string()),
                    _ => (),
                },
                _ => (),
            }
        }
    }

    None
}
*/

fn generate_impl_user_struct(user_struct_name: &Ident, static_factory_name: &Ident) -> TokenStream {
    quote! {
        unsafe impl Send for #user_struct_name {}

        impl MyMetrics {
            pub fn new() -> Option<#user_struct_name> {
                if let Ok(mut factory) = #static_factory_name.write() {
                    Some(factory.build())
                } else {
                    None
                }
            }
        }
    }
}

fn generate_tags_global_fn(user_struct_name: &Ident, field_name: &Ident) -> Ident {
    format_ident!(
        "{}_{}_tags_get",
        snakify(user_struct_name.to_string().as_str()),
        field_name
    )
}

fn generate_factory(
    fields: &Fields,
    user_struct_name: &Ident,
    values_struct_name: &Ident,
    factory_struct_name: &Ident,
    static_factory_name: &Ident,
) -> TokenStream {
    let mut metrics = vec![];
    let mut metrics_tags_hashmap = vec![];

    let metrics_tags_static_prefix = format_ident!(
        "{}",
        format!("{}_TAGS_HASHMAP_", user_struct_name.to_string()).to_uppercase()
    );

    for field in fields {
        let ident = if let Some(ident) = &field.ident {
            ident
        } else {
            continue;
        };
        let ident_str = ident.to_string();

        let ty = MacroFieldType::from(&field.ty);

        // fill types
        match ty {
            MacroFieldType::Counter => {
                metrics.push(quote! {
                    let mut value_sum = 0;
                    factory.threads().iter().for_each(|f| {
                        value_sum += f.#ident.get();
                    });

                    metrics.push(metrics_lockfree::prometheus::prometheus_metric_family_build(
                        metrics_lockfree::types::MetricType::Counter,
                        #ident_str,
                        value_sum,
                        None,
                    ));
                });
            }
            MacroFieldType::Gauge => {
                metrics.push(quote! {
                    let mut value_sum = 0;
                    factory.threads().iter().for_each(|f| {
                        value_sum += f.#ident.get();
                    });

                    metrics.push(metrics_lockfree::prometheus::prometheus_metric_family_build(
                        metrics_lockfree::types::MetricType::Gauge,
                        #ident_str,
                        value_sum,
                        None,
                    ));
                });
            }
            MacroFieldType::CounterWithTags(max_tags) => {
                let static_hashmap_name = format_ident!(
                    "{}_{}",
                    metrics_tags_static_prefix,
                    ident.to_string().to_uppercase()
                );

                metrics.push(quote! {
                    let mut value_sum = 0;
                    factory.threads().iter().for_each(|f| {
                        value_sum += f.#ident.get(0);
                    });

                    metrics.push(metrics_lockfree::prometheus::prometheus_metric_family_build(
                        metrics_lockfree::types::MetricType::CounterWithTags,
                        #ident_str,
                        value_sum,
                        None,
                    ));

                    // then other tags
                    #static_hashmap_name
                        .read()
                        .unwrap()
                        .tags()
                        .iter()
                        .for_each(|(key_value, id)| {
                            let mut value_sum_tag = 0;
                            factory.threads().iter().for_each(|f| {
                                value_sum_tag += f.#ident.get(*id);
                            });

                            metrics.push(metrics_lockfree::prometheus::prometheus_metric_family_build(
                                metrics_lockfree::types::MetricType::CounterWithTags,
                                #ident_str,
                                value_sum_tag,
                                Some(key_value),
                            ));
                        });

                });

                let fn_name = generate_tags_global_fn(user_struct_name, ident);

                metrics_tags_hashmap.push(quote! {
                    static #static_hashmap_name: std::sync::LazyLock<std::sync::RwLock<metrics_lockfree::types::Tags>> =
                        std::sync::LazyLock::new(|| std::sync::RwLock::new(metrics_lockfree::types::Tags::new(#max_tags)));

                    pub fn #fn_name(tags: &[(String, String)]) -> Option<usize> {
                        if let Some(id) = #static_hashmap_name.read().unwrap().get(tags) {
                            return Some(id);
                        }
                        #static_hashmap_name.write().unwrap().insert(tags)
                    }
                });
            }
            MacroFieldType::Unknown(s) => panic!(
                "Error: field '{}' has invalid type: '{s}'. It must be 'Counter' or 'Gauge'",
                ident
            ),
        };
    }

    quote! {

        struct #factory_struct_name {
            per_thread_metrics: Vec<#values_struct_name>,
        }

        impl #factory_struct_name {
            pub fn new() -> Self {
                metrics_lockfree::Exporter::register(#factory_struct_name::metrics);
                Self {
                    per_thread_metrics: vec![],
                }
            }

            pub fn build(&mut self) -> #user_struct_name {
                // todo push per type

                self.per_thread_metrics.push(#values_struct_name::default());
                let last = self.per_thread_metrics.last_mut().unwrap();
                #user_struct_name::from(last)
            }

            pub fn threads(&self) -> &Vec<#values_struct_name> {
                &self.per_thread_metrics
            }

            pub fn metrics() -> Vec<prometheus::proto::MetricFamily> {
                let mut metrics = vec![];

                if let Ok(factory) = #static_factory_name.read() {

                    #(#metrics)*
                }

                metrics
            }

        }

        static #static_factory_name : std::sync::LazyLock<std::sync::RwLock<#factory_struct_name>> =
            std::sync::LazyLock::new(|| std::sync::RwLock::new(#factory_struct_name ::new()));

        #(#metrics_tags_hashmap)*
    }
}

enum MacroFieldType {
    Counter,
    Gauge,
    CounterWithTags(usize),
    Unknown(String),
}

impl From<&syn::Type> for MacroFieldType {
    fn from(value: &syn::Type) -> Self {
        match value {
            syn::Type::Path(tp) => {
                if let Some(p) = tp.path.get_ident() {
                    match p.to_string().as_str() {
                        "Counter" => MacroFieldType::Counter,
                        "Gauge" => MacroFieldType::Gauge,
                        s @ _ => MacroFieldType::Unknown(s.to_string()),
                    }
                } else {
                    // faut passer en raw quand il y a une generic, pour l'instant on ignore
                    let mut ty = vec![];

                    tp.path
                        .to_token_stream()
                        .into_token_stream()
                        .into_iter()
                        .for_each(|t| ty.push(t.to_string()));

                    if ty.len() == 4 && ty[1] == "<" && ty[3] == ">" {
                        if let Ok(max_tags) = ty[2].parse::<usize>() {
                            return MacroFieldType::CounterWithTags(max_tags);
                        }
                    }

                    MacroFieldType::Unknown(ty.join(""))
                }
            }
            _ => MacroFieldType::Unknown(value.to_token_stream().to_string()),
        }
    }
}

fn generate_struct_values(
    fields: &Fields,
    user_struct_name: &Ident,
    values_struct_name: &Ident,
) -> TokenStream {
    let mut field_types = vec![];
    let mut field_init = vec![];

    for field in fields {
        let ident = if let Some(ident) = &field.ident {
            ident
        } else {
            continue;
        };

        let ty = MacroFieldType::from(&field.ty);

        // fill types
        match ty {
            MacroFieldType::Counter => {
                field_types.push(quote!(#ident: metrics_lockfree::counter::CounterPin));
                field_init.push(
                    quote!(#ident: metrics_lockfree::counter::Counter::from(&mut value.#ident)),
                );
            }
            MacroFieldType::Gauge => {
                field_types.push(quote!(#ident: metrics_lockfree::gauge::GaugePin));
                field_init
                    .push(quote!(#ident: metrics_lockfree::gauge::Gauge::from(&mut value.#ident)));
            }
            MacroFieldType::CounterWithTags(max_tags) => {
                let fn_name = generate_tags_global_fn(user_struct_name, ident);

                field_types.push(
                    quote!(#ident: metrics_lockfree::counter_with_tags::CounterWithTagsPin<#max_tags>),
                );
                field_init.push(
                    quote!(#ident: metrics_lockfree::counter_with_tags::CounterWithTags::from(&mut value.#ident).set_fn(#fn_name)),
                );
            }
            MacroFieldType::Unknown(s) => panic!(
                "Error: field '{}' has invalid type: '{s}'. It must be 'Counter' or 'Gauge'",
                ident
            ),
        };
    }

    quote! {
        #[derive(Default)]
        pub struct MyMetricsValues {
            #(#field_types),*
        }

        impl From<&mut #values_struct_name> for #user_struct_name {
            fn from(value: &mut #values_struct_name) -> Self {
                Self {
                    #(#field_init),*
                }
            }
        }
    }
}

fn generate_metrics(ast: &DeriveInput) -> syn::Result<TokenStream> {
    let fields = match &ast.data {
        Data::Struct(v) => &v.fields,
        _ => return Err(non_struct_error()),
    };

    let static_factory_name = format_ident!("{}", format!("{}Factory", &ast.ident).to_uppercase());
    let factory_struct_name = format_ident!("{}Factory", &ast.ident);
    let values_struct_name = format_ident!("{}Values", &ast.ident);
    let user_struct_name = ast.ident.clone();

    let impl_user_struct = generate_impl_user_struct(&user_struct_name, &static_factory_name);

    let struct_values = generate_struct_values(fields, &user_struct_name, &values_struct_name);

    let factory = generate_factory(
        fields,
        &user_struct_name,
        &values_struct_name,
        &factory_struct_name,
        &static_factory_name,
    );

    Ok(quote! {
        #impl_user_struct

        #struct_values

        #factory
    })
}

#[proc_macro_derive(Metrics)]
pub fn enum_try_as(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);

    let toks = generate_metrics(&ast).unwrap_or_else(|err| err.to_compile_error());
    debug_print_generated(&ast, &toks);
    toks.into()
}
