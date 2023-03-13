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
use syn::{spanned::Spanned, DataStruct, DeriveInput, Meta, parse_macro_input, AttributeArgs, Attribute, Field, ext::IdentExt, punctuated::Punctuated, Token, MetaNameValue, Lit, parse::Parser, token::Comma, Expr};


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
    let field_name = f.clone().ident;
    let field_type = f.clone().ty;

    if f.attrs.len() < 1 {
        return quote! { };
    }

    let attr = f.clone().attrs[0].clone();
    let attr_name = attr.path.get_ident().unwrap();

    let args = get_args(&attr);

    for (a, b) in &args {
        println!("{} {}", a, b);
    }

    // let TYPE_MAP = HashMap::from([
    //     ("I", Ident::new("i", Span::call_site())),
    //     ("Denmark", 24),
    //     ("Iceland", 12),
    // ]);

    match attr_name.to_string().as_str() {
        "class" => {
            
            let class = args.get("name").unwrap();
            quote! {
                pub unsafe fn new(app: App) -> Result<Self, jni::errors::Error> {
                    let mut env = JNIEnv::from_raw(app.env)?;
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
            let obf_name = args.get("name").unwrap();
            let obf_ty = args.get("ty").unwrap();

            let dirty = quote! {#field_type}.to_string();
            let hacky: proc_macro2::TokenStream = get_jvalue(dirty).parse().unwrap();

            let mut o = quote!{ self.instance.as_ref().unwrap() };

            let mut get_fn_name = Ident::new(&format!("{}{}", "get_", field_name.as_ref().unwrap()), Span::call_site());
            let mut get_method_call = quote! {get_field};

            let mut set_fn_name = Ident::new(&format!("{}{}", "set_", field_name.as_ref().unwrap()), Span::call_site());
            let mut set_method_call = quote! {
                let out = env.set_field(
                    &(#o), 
                    #obf_name, 
                    #obf_ty,
                    #hacky
                )?
            };

            let mut ret_ty = Ident::new(&obf_ty[0..1].to_lowercase(), Span::call_site());
            let mut import = quote! {};
            
            if obf_ty.starts_with("L") {
                import = quote! {
                    let mut object = #field_type::new(self.app)?;
                    object.set_instance(out);

                    let out = object;
                };

                ret_ty = Ident::new("l", Span::call_site());
            }

            if args.contains_key("static") {
                o = quote! { self.class };

                get_fn_name = Ident::new(&format!("{}{}_static", "get_", field_name.as_ref().unwrap()), Span::call_site());
                get_method_call = quote! {get_static_field};

                set_fn_name = Ident::new(&format!("{}{}_static", "set_", field_name.as_ref().unwrap()), Span::call_site());
                set_method_call = quote! {
                    let id = env.get_static_field_id(&(#o), #obf_name, #obf_ty)?;
                    let out = env.set_static_field(
                        &(#o), 
                        id,
                        #hacky
                    )?
                };
            }



            quote! {
                pub unsafe fn #get_fn_name(&mut self) -> Result<#field_type, jni::errors::Error> {
                    let mut env = JNIEnv::from_raw(self.app.env)?;
                    
                    let out = env
                    .#get_method_call(
                        &(#o), 
                        #obf_name, 
                        #obf_ty
                    )?.#ret_ty()?;

                    #import

                    Ok(out)
                }

                pub unsafe fn #set_fn_name(&mut self, val: #field_type) -> Result<(), jni::errors::Error> {
                    let mut env = JNIEnv::from_raw(self.app.env)?;
                    
                    #set_method_call;

                    Ok(())
                }
            }
        } 
        "method" => {
            let mut test_args: proc_macro2::TokenStream =  quote! {};
            
            if args.contains_key("args") {
                test_args = format!(", {}", args.get("args").unwrap()).to_string().parse().unwrap();
            }

            let mut import = quote! {};
            let mut o = quote!{ self.instance.as_ref().unwrap() };
            
            let mut ret_ty = quote! {};
            let dirty = quote! {#field_type}.to_string();

            if ["bool", "char", "i32", "f32"].contains(&dirty.as_str()) {
                let cum = Ident::new(&dirty[0..1].to_lowercase(), Span::call_site());
                ret_ty = quote! {
                    let out = out.#cum()?;
                };
            } else if dirty != "()" {
                ret_ty = quote! {
                    let out = out.l()?;
                    let mut object = #field_type::new(self.app)?;
                    object.set_instance(out);

                    let out = object;
                };
            } else {
                ret_ty = quote! {
                    let out = ();
                }
            }

            if args.contains_key("static") {
                let fn_name = Ident::new(&format!("{}{}", field_name.unwrap(), "_static"), Span::call_site());
                let obf_name = args.get("name").unwrap();
                let obf_sig = args.get("sig").unwrap();

                quote! {
                    pub unsafe fn #fn_name(&mut self #test_args) -> Result<#field_type, jni::errors::Error> {
                        let mut env = JNIEnv::from_raw(self.app.env)?;
                        // let system = env.find_class("java/lang/System")?;
                        // let print_stream = env.find_class("java/io/PrintStream")?;
                        let out = env
                        .call_static_method(
                            &self.class,
                            #obf_name,
                            #obf_sig,
                            &[]
                        )?;

                        #ret_ty
                        // let out = env.get_static_field(system, "out", "Ljava/io/PrintStream;")?.l()?;
                        // let message = env.new_string("asdsaddsa World2")?;
    
                        // let msg = env.new_string(text.to_string())?;
                        /*env
                        .call_method(
                            &out, 
                            "println", 
                            "(Ljava/lang/String;)V", 
                            &[JValue::from(&message)]
                        )?;*/

                        // let mut object = #field_type::new(self.app)?;
                        // object.set_instance(out);
                        
                        Ok(out)
                    }
                }
            } else {
                let obf_name = args.get("name").unwrap();
                let obf_sig = args.get("sig").unwrap();
                quote! {
                    pub unsafe fn #field_name(&mut self #test_args) -> Result<#field_type, jni::errors::Error> {
                        if self.instance.is_none() {
                            return Err(jni::errors::Error::NullPtr("A"));
                        }

                        let mut env = JNIEnv::from_raw(self.app.env)?;
                        let message = env.new_string("SKLDLSKDL")?;

                        let out = env
                        .call_method(
                            &(self.instance.as_ref().unwrap()),
                            #obf_name,
                            #obf_sig,
                            &[JValue::Bool(1)]
                        )?;

                        #ret_ty
                        
                        Ok(out)
                    }
                }
            }
        }
        &_ => { quote! {} }
    }
}

fn produce(ast: &syn::DeriveInput) -> TokenStream2 {    
    if let syn::Data::Struct(DataStruct { ref fields, .. }) = ast.data {
        let generated: Vec<_> = fields.iter().map(|f| generate(f)).collect();

        let name = &ast.ident;

        // println!("asdasdasdsadad-========\n{:?}", generated);
        
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
pub fn inject(input: TokenStream) -> TokenStream {
    use syn::{punctuated::Punctuated, Token};

    let ast: DeriveInput = syn::parse(input).unwrap();
    // Build the impl
    let gen = produce(&ast); //produce(&ast);
    println!("{}", gen);
    // Return the generated impl
    gen.into()
}


#[proc_macro_attribute]
pub fn hack(_args: TokenStream, input: TokenStream) -> TokenStream  {
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
                        // .push(syn::Field::parse_named.parse2(quote! { pub a: String }).unwrap());
                }   
                _ => {
                    ()
                }
            }             

            println!("{:?}", ast); 
            
            return quote! {
                #ast
            }.into();
        }
        _ => panic!("`add_field` has to be used with structs "),
    }
}