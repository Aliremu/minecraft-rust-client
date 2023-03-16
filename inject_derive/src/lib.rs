#![allow(warnings)]
extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;
extern crate proc_macro2;

use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, Ident, Span};
use quote::{__private::ext::RepToTokensExt, ToTokens};
use syn::{spanned::Spanned, DataStruct, DeriveInput, Meta, parse_macro_input, AttributeArgs, Attribute, Field, ext::IdentExt, punctuated::Punctuated, Token, MetaNameValue, Lit, parse::Parser, token::{Comma, Box}, Expr, TypeBareFn, TypePath, BareFnArg, ReturnType, Type};

mod generate;
use crate::generate::{generate_field, generate_method};

fn get_jvalue(ty: String) -> String {
    match ty.as_str() {
        "bool" => "JValue::Bool(val as u8)".to_string(),
        "char" => "JValue::Char(val)".to_string(),
        "i32" => "JValue::Int(val)".to_string(),
        "f32" => "JValue::Float(val)".to_string(),
        "f64" => "JValue::Double(val)".to_string(),
        _ => "JValue::Object(&(val.instance.as_ref().unwrap()))".to_string()
    }
}

fn get_args(attr: &Attribute) -> HashMap<String, String> {
    let mut map = HashMap::new();

    let args = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated).unwrap();
        
    for meta in args.into_iter() {
        if let Some(Meta::NameValue(MetaNameValue { lit: Lit::Str(ref s), path, ..})) = Some(meta) {
            // println!("{} = {}", path.get_ident().unwrap(), s.value());
            map.insert(path.get_ident().unwrap().to_string(), s.value());
        }
    }

    map
}

fn generate(f: &Field) -> TokenStream2 {
    let field_name = f.clone().ident.unwrap_or_else(|| panic!("Expected a field name!"));
    let field_type = f.clone().ty;

    if f.attrs.len() < 1 {
        return quote! { };
    }

    let attr = f.clone().attrs[0].clone();
    let attr_name = attr.path.get_ident().unwrap();

    let args = get_args(&attr);

    match attr_name.to_string().as_str() {
        "class" => {   
            let class = args.get("name").unwrap();
            quote! {
                pub unsafe fn new(app: &'a App) -> Result<Self, jni::errors::Error> {
                    let mut env = app.get_env()?;
                    let system = env.find_class(#class)?;

                    Ok(Self {
                        instance: None,
                        class: system,
                        app: app
                    })
                }
            }
        }
        "field" => {
            let attr_name = args.get("name").unwrap();
            let attr_ty = args.get("ty").unwrap();

            generate_field(&field_name, &field_type, attr_name, attr_ty, args.contains_key("static"))
        } 
        "method" => {
            if let syn::Type::BareFn(TypeBareFn { inputs, output, .. }) = &field_type {
                let method_args = inputs.iter().map(|x| (x.clone().name.unwrap().0, x.clone().ty)).collect::<Vec<(Ident, Type)>>();
                let attr_name = args.get("name").unwrap();
                let attr_sig = args.get("sig").unwrap();

                generate_method(&field_name, &output, method_args, attr_name, attr_sig, args.contains_key("static"))
            } else {
                quote! {}
            }
        }
        &_ => { quote! {} }
    }
}

fn produce(ast: &syn::DeriveInput) -> TokenStream2 {    
    if let syn::Data::Struct(DataStruct { ref fields, .. }) = ast.data {
        let generated: Vec<_> = fields.iter().map(|f| generate(f)).collect();

        let name = &ast.ident;

        quote! {
            impl<'a> #name<'a> {
                pub unsafe fn set_instance(&mut self, instance: JObject<'a>) {
                    self.instance.insert(instance);
                }

                #(#generated)*
            }
        }
    } else {
        quote! {

        }
    }
}

#[proc_macro_derive(Inject, attributes(class, field, method, lol))]
pub fn inject_derive(input: TokenStream) -> TokenStream {
    use syn::{punctuated::Punctuated, Token};

    let ast: DeriveInput = syn::parse(input).unwrap();

    let gen = produce(&ast); 

    gen.into()
}


#[proc_macro_attribute]
pub fn inject(_args: TokenStream, input: TokenStream) -> TokenStream  {
    let mut ast = parse_macro_input!(input as DeriveInput);
    match &mut ast.data {
        syn::Data::Struct(ref mut struct_data) => {           
            match &mut struct_data.fields {
                syn::Fields::Named(fields) => {
                    let copy = fields.clone();
                    let copy = copy.named.iter().filter(|f| ["app", "class"].contains(&f.clone().ident.clone().unwrap().to_string().as_str())).collect::<Punctuated<&Field, Comma>>();
                    fields.named.clear();
                    for a in copy.iter() {
                        fields.named.push((**a).clone());
                    }

                    fields.named.push(syn::Field::parse_named.parse2(quote! { instance: Option<JObject<'a>> }).unwrap());
                }   
                _ => {
                    ()
                }
            }             
            
            return quote! {
                #ast
            }.into();
        }
        _ => panic!("`add_field` has to be used with structs "),
    }
}